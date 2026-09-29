use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use crate::dsn::{Config, Endpoint, sql_literal};
use crate::error::{Error, Result, client};
use crate::handshake::{self, handshake_collation};
use crate::io::{PacketIo, Socket};
use crate::lb;
use crate::packet::{Reader, Writer};
use crate::protocol::{Column, Ok, cap, cmd, is_eof, parse_eof, parse_err, status};
use crate::value::{Param, Row, decode_binary_row, decode_text_row};

const LONG_DATA_CHUNK: usize = 8 * 1024 * 1024;
const INFILE_CHUNK: usize = 1024 * 1024;
pub const REFRESH_GRANT: u8 = 0x01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Read every row at execute time; the connection is free afterwards.
    Buffered,
    /// Read rows on demand; no other command may run until the result is consumed.
    Streaming,
    /// Server-side cursor (prepared statements only): rows arrive in batches of this size via COM_STMT_FETCH.
    Cursor(u32),
}

enum Source {
    Done,
    Streaming,
    Cursor { stmt_id: u32, fetch_size: u32, exhausted: bool },
}

pub struct ResultSet {
    pub columns: Arc<[Column]>,
    pub binary: bool,
    pub ok: Ok,
    pub more_results: bool,
    pub current: Row,
    pub fetched: u64,
    pub buffered_rows: Option<u64>,
    mode: Mode,
    rows: VecDeque<Row>,
    source: Source,
}

impl ResultSet {
    fn empty(ok: Ok, binary: bool, mode: Mode) -> Self {
        let more_results = ok.status & status::MORE_RESULTS_EXISTS != 0;
        Self { columns: Arc::from(Vec::new()), binary, ok, more_results, current: Row::default(), fetched: 0, buffered_rows: None, mode, rows: VecDeque::new(), source: Source::Done }
    }

    pub fn columns_only(columns: Vec<Column>) -> Self {
        let mut rs = Self::empty(Ok::default(), false, Mode::Buffered);
        rs.columns = Arc::from(columns);
        rs.buffered_rows = Some(0);
        rs
    }

    pub fn is_cursor(&self) -> bool {
        matches!(self.source, Source::Cursor { .. })
    }
}

pub struct Stmt {
    pub id: u32,
    pub params: Vec<Column>,
    pub columns: Arc<[Column]>,
}

pub struct Conn {
    io: PacketIo,
    pub cfg: Config,
    pub server_version: String,
    pub connection_id: u32,
    pub capabilities: u32,
    pub status: u16,
    pub last: Ok,
    pub endpoint: String,
    pub auth_plugin: String,
    pub created: Instant,
    pub last_used: Instant,
    pub discovery_error: Option<String>,
    scramble: Vec<u8>,
    tcp_endpoint: Option<Endpoint>,
    pending: bool,
    pub broken: bool,
    /// Data PHP supplied for the next LOAD DATA LOCAL request, whatever file name the server asks for.
    pub infile_data: Option<Vec<u8>>,
}

impl Conn {
    pub fn connect(cfg: Config) -> Result<Self> {
        #[cfg(unix)]
        if let Some(path) = cfg.unix_socket.clone() {
            let io = PacketIo::new(Socket::unix(&path)?, cfg.read_timeout, cfg.write_timeout)?;
            return Self::establish(io, cfg, path, "localhost".into(), None);
        }
        let mut last = Error::connect("no TiDB server to connect to");
        for endpoint in lb::candidates(&cfg) {
            let socket = match Socket::tcp(&endpoint.host, endpoint.port, cfg.connect_timeout) {
                Ok(s) => s,
                Err(e) => {
                    lb::mark_down(&endpoint);
                    last = e;
                    continue;
                }
            };
            lb::mark_up(&endpoint);
            let io = PacketIo::new(socket, cfg.connect_timeout.or(cfg.read_timeout), cfg.write_timeout)?;
            let label = format!("{}:{}", endpoint.host, endpoint.port);
            return Self::establish(io, cfg, label, endpoint.host.clone(), Some(endpoint));
        }
        Err(last)
    }

    fn establish(mut io: PacketIo, cfg: Config, label: String, host: String, tcp: Option<Endpoint>) -> Result<Self> {
        let negotiated = handshake::run(&mut io, &cfg, &host)?;
        let now = Instant::now();
        let mut conn = Self {
            io,
            server_version: negotiated.handshake.server_version.clone(),
            connection_id: negotiated.handshake.connection_id,
            capabilities: negotiated.capabilities,
            status: negotiated.ok.status,
            last: negotiated.ok,
            endpoint: label,
            auth_plugin: negotiated.auth_plugin,
            created: now,
            last_used: now,
            discovery_error: None,
            scramble: negotiated.handshake.scramble,
            tcp_endpoint: tcp,
            pending: false,
            broken: false,
            infile_data: None,
            cfg,
        };
        conn.io.set_timeouts(conn.cfg.read_timeout, conn.cfg.write_timeout)?;
        conn.after_connect()?;
        Ok(conn)
    }

    fn after_connect(&mut self) -> Result<()> {
        let (_, set_names) = handshake_collation(&self.cfg);
        let mut statements = Vec::new();
        if set_names {
            let mut s = format!("SET NAMES {}", sql_literal(&self.cfg.charset));
            if let Some(c) = &self.cfg.collation {
                s.push_str(&format!(" COLLATE {}", sql_literal(c)));
            }
            statements.push(s);
        }
        statements.extend(self.cfg.session_statement());
        statements.extend(self.cfg.init_commands.clone());
        if statements.is_empty() {
            // TiDB keeps no process info for a connection that has not run a command yet, and KILL against it panics server side (1105).
            self.ping()?;
        }
        for sql in statements {
            self.execute_discard(&sql)?;
        }
        if self.tcp_endpoint.is_some() && lb::needs_discovery(&self.cfg) {
            self.discover();
        }
        Ok(())
    }

    fn discover(&mut self) {
        let found = (|| -> Result<Vec<Endpoint>> {
            let mut rs = self.query(b"SELECT IP, PORT FROM INFORMATION_SCHEMA.TIDB_SERVERS_INFO", Mode::Buffered)?;
            let mut out = Vec::new();
            while self.next_row(&mut rs)? {
                let host = rs.current.bytes(rs.current.cells[0]).map(|b| String::from_utf8_lossy(b).into_owned());
                let port = match rs.current.cells[1] {
                    crate::value::Cell::Int(p) => u16::try_from(p).ok(),
                    _ => None,
                };
                if let (Some(host), Some(port)) = (host, port) {
                    out.push(Endpoint { host, port });
                }
            }
            Ok(out)
        })();
        match found {
            Ok(list) if !list.is_empty() => lb::store(&self.cfg, list),
            Ok(_) => lb::store(&self.cfg, self.cfg.endpoints.clone()),
            Err(e) => {
                self.discovery_error = Some(e.message);
                lb::store(&self.cfg, self.cfg.endpoints.clone());
            }
        }
    }

    pub fn is_tls(&self) -> bool {
        self.io.is_tls()
    }

    pub fn tls_version(&self) -> Option<&'static str> {
        self.io.tls_version()
    }

    pub fn tls_cipher(&self) -> Option<String> {
        self.io.tls_cipher()
    }

    pub fn compression(&self) -> crate::dsn::Compression {
        self.io.compression()
    }

    pub fn in_transaction(&self) -> bool {
        self.status & status::IN_TRANS != 0
    }

    pub fn has_pending_result(&self) -> bool {
        self.pending
    }

    pub fn is_alive(&mut self) -> bool {
        if self.broken || self.pending {
            return false;
        }
        if self.cfg.max_lifetime.is_some_and(|max| self.created.elapsed() >= max) {
            return false;
        }
        if self.cfg.idle_ping.is_none_or(|idle| self.last_used.elapsed() >= idle) {
            return self.ping().is_ok();
        }
        true
    }

    fn guard<T>(&mut self, r: Result<T>) -> Result<T> {
        if let Err(e) = &r
            && e.fatal
        {
            self.broken = true;
            self.pending = false;
        }
        r
    }

    fn send(&mut self, command: u8, body: &[u8]) -> Result<()> {
        if self.broken {
            return Err(Error { sqlstate: *b"08S01", code: client::SERVER_GONE, message: "TiDB server has gone away (the connection was closed after an earlier error)".into(), fatal: true });
        }
        if self.pending {
            return Err(Error::client(client::OUT_OF_SYNC, "Cannot execute queries while other unbuffered queries are active. Consider using PDO::fetchAll() or PDOStatement::closeCursor()"));
        }
        self.last_used = Instant::now();
        self.io.reset_seq();
        let mut packet = Vec::with_capacity(body.len() + 1);
        packet.push(command);
        packet.extend_from_slice(body);
        let r = self.io.write_packet(&packet);
        self.guard(r)
    }

    fn read(&mut self) -> Result<Vec<u8>> {
        let r = self.io.read_packet();
        self.guard(r)
    }

    fn terminator(&self, p: &[u8]) -> Result<Ok> {
        terminator(p, self.capabilities & cap::DEPRECATE_EOF != 0)
    }

    fn read_ok(&mut self) -> Result<Ok> {
        let p = self.read()?;
        match p.first() {
            Some(0x00) | Some(0xFE) => {
                let ok = if p[0] == 0xFE { self.terminator(&p)? } else { Ok::parse(&p)? };
                self.status = ok.status;
                self.last = ok.clone();
                Ok(ok)
            }
            Some(0xFF) => Err(parse_err(&p)),
            _ => self.guard(Err(Error::malformed("expected OK packet"))),
        }
    }

    /// Runs a statement and throws its result away (session setup, transaction control).
    pub fn execute_discard(&mut self, sql: &str) -> Result<Ok> {
        let mut rs = self.query(sql.as_bytes(), Mode::Streaming)?;
        self.finish(&mut rs)?;
        Ok(rs.ok)
    }

    /// SQL is bytes: `_binary'...'` literals from `quote()` are not UTF-8.
    pub fn query(&mut self, sql: &[u8], mode: Mode) -> Result<ResultSet> {
        self.send(cmd::QUERY, sql)?;
        let mode = if matches!(mode, Mode::Cursor(_)) { Mode::Streaming } else { mode };
        self.read_result(false, mode, None)
    }

    fn read_result(&mut self, binary: bool, mode: Mode, stmt_id: Option<u32>) -> Result<ResultSet> {
        loop {
            let p = self.read()?;
            match p.first() {
                Some(0x00) => {
                    let ok = Ok::parse(&p)?;
                    self.status = ok.status;
                    self.last = ok.clone();
                    let rs = ResultSet::empty(ok, binary, mode);
                    self.pending = rs.more_results;
                    return Ok(rs);
                }
                Some(0xFF) => {
                    self.pending = false;
                    return Err(parse_err(&p));
                }
                Some(0xFB) => {
                    let name = String::from_utf8_lossy(&p[1..]).into_owned();
                    self.send_infile(&name)?;
                }
                _ => return self.read_rows_header(&p, binary, mode, stmt_id),
            }
        }
    }

    fn read_rows_header(&mut self, first: &[u8], binary: bool, mode: Mode, stmt_id: Option<u32>) -> Result<ResultSet> {
        let count = Reader::new(first).lenenc_int()?;
        let mut columns = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let p = self.read()?;
            columns.push(Column::parse(&p)?);
        }
        let mut rs = ResultSet::empty(Ok::default(), binary, mode);
        rs.columns = Arc::from(columns);
        let deprecate_eof = self.capabilities & cap::DEPRECATE_EOF != 0;
        if let (Mode::Cursor(fetch_size), Some(id)) = (mode, stmt_id) {
            let p = self.read()?;
            let ok = self.terminator(&p)?;
            self.status = ok.status;
            if ok.status & status::CURSOR_EXISTS != 0 {
                rs.source = Source::Cursor { stmt_id: id, fetch_size: fetch_size.max(1), exhausted: false };
                rs.ok = ok;
                return Ok(rs);
            }
            rs.source = Source::Streaming;
            self.pending = true;
        } else {
            if !deprecate_eof {
                let p = self.read()?;
                self.terminator(&p)?;
            }
            rs.source = Source::Streaming;
            self.pending = true;
        }
        if mode == Mode::Buffered {
            let mut rows = VecDeque::new();
            while let Some(row) = self.read_stream_row(&mut rs)? {
                rows.push_back(row);
            }
            rs.buffered_rows = Some(rows.len() as u64);
            rs.rows = rows;
        }
        Ok(rs)
    }

    /// Next row of a streaming result, or `None` after the terminator (which updates status and more_results).
    fn read_stream_row(&mut self, rs: &mut ResultSet) -> Result<Option<Row>> {
        let p = self.read()?;
        match p.first() {
            Some(0xFF) => {
                rs.source = Source::Done;
                rs.more_results = false;
                self.pending = false;
                Err(parse_err(&p))
            }
            Some(0xFE) if p.len() < crate::io::MAX_PAYLOAD => {
                let ok = self.terminator(&p)?;
                self.status = ok.status;
                self.last = Ok { affected_rows: 0, last_insert_id: 0, ..ok.clone() };
                rs.more_results = ok.status & status::MORE_RESULTS_EXISTS != 0;
                rs.ok = ok;
                rs.source = Source::Done;
                self.pending = rs.more_results;
                Ok(None)
            }
            _ => {
                let row = if rs.binary { decode_binary_row(p, &rs.columns) } else { decode_text_row(p, &rs.columns) };
                self.guard(row).map(Some)
            }
        }
    }

    pub fn next_row(&mut self, rs: &mut ResultSet) -> Result<bool> {
        loop {
            if let Some(row) = rs.rows.pop_front() {
                rs.current = row;
                rs.fetched += 1;
                return Ok(true);
            }
            match rs.source {
                Source::Done => return Ok(false),
                Source::Streaming => {
                    return match self.read_stream_row(rs)? {
                        Some(row) => {
                            rs.current = row;
                            rs.fetched += 1;
                            Ok(true)
                        }
                        None => Ok(false),
                    };
                }
                Source::Cursor { stmt_id, fetch_size, exhausted } => {
                    if exhausted {
                        rs.source = Source::Done;
                        return Ok(false);
                    }
                    self.fetch_cursor(rs, stmt_id, fetch_size)?;
                }
            }
        }
    }

    fn fetch_cursor(&mut self, rs: &mut ResultSet, stmt_id: u32, fetch_size: u32) -> Result<()> {
        let mut body = Vec::with_capacity(8);
        body.put_u32(stmt_id);
        body.put_u32(fetch_size);
        self.send(cmd::STMT_FETCH, &body)?;
        loop {
            let p = self.read()?;
            match p.first() {
                Some(0xFF) => {
                    rs.source = Source::Done;
                    return Err(parse_err(&p));
                }
                Some(0xFE) if p.len() < crate::io::MAX_PAYLOAD => {
                    let ok = self.terminator(&p)?;
                    self.status = ok.status;
                    let exhausted = ok.status & status::LAST_ROW_SENT != 0 || ok.status & status::CURSOR_EXISTS == 0;
                    rs.ok = ok;
                    rs.source = Source::Cursor { stmt_id, fetch_size, exhausted };
                    return Ok(());
                }
                _ => {
                    let row = decode_binary_row(p, &rs.columns);
                    rs.rows.push_back(self.guard(row)?);
                }
            }
        }
    }

    /// Skips the rest of the current result and moves to the next one; false when there is none.
    pub fn next_result(&mut self, rs: &mut ResultSet) -> Result<bool> {
        self.drain_rows(rs)?;
        if !rs.more_results {
            return Ok(false);
        }
        self.pending = false;
        let next = self.read_result(rs.binary, rs.mode, None)?;
        *rs = next;
        Ok(true)
    }

    fn drain_rows(&mut self, rs: &mut ResultSet) -> Result<()> {
        rs.rows.clear();
        while matches!(rs.source, Source::Streaming) {
            self.read_stream_row(rs)?;
        }
        if let Source::Cursor { stmt_id, exhausted: false, .. } = rs.source {
            rs.source = Source::Done;
            self.reset_stmt(stmt_id)?;
        }
        rs.source = Source::Done;
        Ok(())
    }

    /// Consumes everything the server still has for this command, including further result sets.
    pub fn finish(&mut self, rs: &mut ResultSet) -> Result<()> {
        while self.next_result(rs)? {}
        Ok(())
    }

    fn send_infile(&mut self, name: &str) -> Result<()> {
        let data = self.infile_data.take();
        let outcome = match data {
            Some(bytes) => {
                for chunk in bytes.chunks(INFILE_CHUNK) {
                    self.io.write_packet(chunk)?;
                }
                Ok(())
            }
            None => self.stream_file(name),
        };
        let done = self.io.write_packet(&[]);
        self.guard(done)?;
        if let Err(e) = outcome {
            let p = self.read()?;
            if p.first() == Some(&0xFF) {
                let _ = parse_err(&p);
            }
            self.pending = false;
            return Err(e);
        }
        Ok(())
    }

    fn stream_file(&mut self, name: &str) -> Result<()> {
        let reject = |why: String| Error::client(client::LOCAL_INFILE_REJECTED, format!("LOAD DATA LOCAL INFILE file request rejected due to restrictions on access: {why}"));
        let path = match &self.cfg.local_infile_directory {
            Some(dir) => {
                let base = Path::new(dir).canonicalize().map_err(|e| reject(format!("local_infile_directory {dir}: {e}")))?;
                let file = Path::new(name).canonicalize().map_err(|e| reject(format!("{name}: {e}")))?;
                if !file.starts_with(&base) {
                    return Err(reject(format!("{name} is outside {dir}")));
                }
                file
            }
            None if self.cfg.local_infile => Path::new(name).to_path_buf(),
            None => return Err(reject("local_infile is disabled".into())),
        };
        let mut file = File::open(&path).map_err(|e| reject(format!("{}: {e}", path.display())))?;
        let mut buf = vec![0u8; INFILE_CHUNK];
        loop {
            let n = file.read(&mut buf).map_err(|e| reject(format!("{}: {e}", path.display())))?;
            if n == 0 {
                return Ok(());
            }
            let r = self.io.write_packet(&buf[..n]);
            self.guard(r)?;
        }
    }

    pub fn prepare(&mut self, sql: &[u8]) -> Result<Stmt> {
        self.send(cmd::STMT_PREPARE, sql)?;
        let p = self.read()?;
        if p.first() == Some(&0xFF) {
            return Err(parse_err(&p));
        }
        let mut r = Reader::new(&p);
        r.u8()?;
        let id = r.u32()?;
        let ncols = r.u16()?;
        let nparams = r.u16()?;
        let deprecate_eof = self.capabilities & cap::DEPRECATE_EOF != 0;
        let read_defs = |n: u16, this: &mut Self| -> Result<Vec<Column>> {
            let mut defs = Vec::with_capacity(usize::from(n));
            for _ in 0..n {
                defs.push(Column::parse(&this.read()?)?);
            }
            if n > 0 && !deprecate_eof {
                this.read()?;
            }
            Ok(defs)
        };
        let params = read_defs(nparams, self)?;
        let columns = read_defs(ncols, self)?;
        Ok(Stmt { id, params, columns: Arc::from(columns) })
    }

    /// `long_data` is sent with COM_STMT_SEND_LONG_DATA for the parameters marked `Param::LongData`.
    pub fn execute(&mut self, stmt: &Stmt, params: &[Param<'_>], long_data: &[(u16, &[u8])], mode: Mode) -> Result<ResultSet> {
        if params.len() != stmt.params.len() {
            return Err(Error::server(1210, *b"HY000", format!("Incorrect arguments to EXECUTE: the statement has {} parameters, {} given", stmt.params.len(), params.len())));
        }
        for (index, data) in long_data {
            let chunks: Vec<&[u8]> = if data.is_empty() { vec![&[][..]] } else { data.chunks(LONG_DATA_CHUNK).collect() };
            for chunk in chunks {
                let mut body = Vec::with_capacity(chunk.len() + 6);
                body.put_u32(stmt.id);
                body.put_u16(*index);
                body.extend_from_slice(chunk);
                self.send(cmd::STMT_SEND_LONG_DATA, &body)?;
            }
        }
        let cursor = matches!(mode, Mode::Cursor(_)) && !stmt.columns.is_empty();
        let mode = if matches!(mode, Mode::Cursor(_)) && !cursor { Mode::Buffered } else { mode };
        self.send(cmd::STMT_EXECUTE, &crate::value::execute_body(stmt.id, cursor, params))?;
        self.read_result(true, mode, Some(stmt.id))
    }

    pub fn close_stmt(&mut self, id: u32) -> Result<()> {
        if self.broken {
            return Ok(());
        }
        self.send(cmd::STMT_CLOSE, &id.to_le_bytes())
    }

    pub fn reset_stmt(&mut self, id: u32) -> Result<()> {
        self.send(cmd::STMT_RESET, &id.to_le_bytes())?;
        self.read_ok().map(|_| ())
    }

    pub fn ping(&mut self) -> Result<()> {
        self.send(cmd::PING, &[])?;
        self.read_ok().map(|_| ())
    }

    pub fn init_db(&mut self, db: &str) -> Result<()> {
        self.send(cmd::INIT_DB, db.as_bytes())?;
        self.read_ok()?;
        self.cfg.database = Some(db.to_string());
        Ok(())
    }

    /// COM_RESET_CONNECTION: TiDB rebuilds the session (variables, temporary tables, prepared statements) and keeps the user and database.
    pub fn reset_connection(&mut self) -> Result<()> {
        self.send(cmd::RESET_CONNECTION, &[])?;
        self.read_ok()?;
        self.after_connect()
    }

    pub fn change_user(&mut self, user: &str, password: Option<&str>, database: Option<&str>) -> Result<()> {
        if self.pending {
            return self.send(cmd::CHANGE_USER, &[]);
        }
        let mut cfg = self.cfg.clone();
        cfg.user = user.to_string();
        cfg.password = password.map(str::to_string);
        cfg.database = database.map(str::to_string);
        let r = handshake::change_user(&mut self.io, &cfg, &self.scramble, &self.auth_plugin);
        // A rejected COM_CHANGE_USER leaves TiDB on the previous session, so the previous credentials stay in effect.
        let (ok, plugin) = self.guard(r)?;
        self.cfg = cfg;
        self.status = ok.status;
        self.last = ok;
        self.auth_plugin = plugin;
        self.after_connect()
    }

    pub fn set_multi_statements(&mut self, on: bool) -> Result<()> {
        self.send(cmd::SET_OPTION, &u16::from(!on).to_le_bytes())?;
        self.read_ok()?;
        self.cfg.multi_statements = on;
        Ok(())
    }

    pub fn statistics(&mut self) -> Result<String> {
        self.send(cmd::STATISTICS, &[])?;
        let p = self.read()?;
        if p.first() == Some(&0xFF) {
            return Err(parse_err(&p));
        }
        Ok(String::from_utf8_lossy(&p).into_owned())
    }

    pub fn refresh(&mut self, options: u8) -> Result<()> {
        self.send(cmd::REFRESH, &[options])?;
        self.read_ok()?;
        // TiDB answers REFRESH_GRANT with the OK of its internal FLUSH PRIVILEGES and then a second OK (server/conn.go handleRefresh).
        if options == REFRESH_GRANT {
            self.read_ok()?;
        }
        Ok(())
    }

    pub fn field_list(&mut self, table: &str, wildcard: &str) -> Result<Vec<Column>> {
        let mut body = Vec::with_capacity(table.len() + wildcard.len() + 1);
        body.put_null_str(table.as_bytes());
        body.extend_from_slice(wildcard.as_bytes());
        self.send(cmd::FIELD_LIST, &body)?;
        let mut out = Vec::new();
        loop {
            let p = self.read()?;
            match p.first() {
                Some(0xFF) => return Err(parse_err(&p)),
                Some(0xFE) if p.len() < crate::io::MAX_PAYLOAD => {
                    self.status = self.terminator(&p)?.status;
                    return Ok(out);
                }
                // TiDB ignores the wildcard of COM_FIELD_LIST (server/conn.go handleFieldList), so it is applied here.
                _ => {
                    let column = Column::parse(&p)?;
                    if wildcard.is_empty() || like_ci(wildcard, &column.name) {
                        out.push(column);
                    }
                }
            }
        }
    }

    /// Stops the statement running on this connection from a second connection to the same TiDB server.
    pub fn kill_query(&self) -> Result<()> {
        let mut cfg = self.cfg.clone();
        cfg.discover = false;
        cfg.init_commands.clear();
        cfg.session.clear();
        if let Some(ep) = &self.tcp_endpoint {
            cfg.endpoints = vec![ep.clone()];
        }
        let mut other = Conn::connect(cfg)?;
        other.execute_discard(&format!("KILL QUERY {}", self.connection_id))?;
        other.close();
        Ok(())
    }

    pub fn close(&mut self) {
        if !self.broken && !self.pending {
            self.io.reset_seq();
            let _ = self.io.write_packet(&[cmd::QUIT]);
        }
        self.io.close();
        self.broken = true;
    }
}

/// SQL `LIKE` over column names: `%`, `_`, backslash escape, case-insensitive like MySQL identifiers.
fn like_ci(pattern: &str, text: &str) -> bool {
    fn go(p: &[char], t: &[char]) -> bool {
        match p.split_first() {
            None => t.is_empty(),
            Some(('%', rest)) => (0..=t.len()).any(|i| go(rest, &t[i..])),
            Some(('_', rest)) => !t.is_empty() && go(rest, &t[1..]),
            Some(('\\', rest)) if !rest.is_empty() => t.first() == Some(&rest[0]) && go(&rest[1..], &t[1..]),
            Some((c, rest)) => t.first() == Some(c) && go(rest, &t[1..]),
        }
    }
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let t: Vec<char> = text.to_lowercase().chars().collect();
    go(&p, &t)
}

fn eof_as_ok(p: &[u8]) -> Ok {
    let (warnings, status) = parse_eof(p).unwrap_or((0, 0));
    Ok { status, warnings, ..Ok::default() }
}

/// With DEPRECATE_EOF every terminator is an OK packet under a 0xFE header, and a short one is indistinguishable from a legacy EOF by length.
fn terminator(p: &[u8], deprecate_eof: bool) -> Result<Ok> {
    match p.first() {
        Some(0xFE) if !deprecate_eof && is_eof(p) => Ok(eof_as_ok(p)),
        Some(0xFE) => Ok::parse(p),
        Some(0xFF) => Err(parse_err(p)),
        _ => Err(Error::malformed("expected EOF packet").fatal()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_ok_terminator_keeps_warnings() {
        let p = [0xFE, 0x00, 0x00, 0x22, 0x00, 0x01, 0x00];
        let ok = terminator(&p, true).unwrap();
        assert_eq!((ok.status, ok.warnings), (0x22, 1));
        let legacy = terminator(&[0xFE, 0x03, 0x00, 0x02, 0x00], false).unwrap();
        assert_eq!((legacy.status, legacy.warnings), (2, 3));
    }

    #[test]
    fn like_matches_mysql_wildcards() {
        assert!(like_ci("na%", "Name"));
        assert!(like_ci("_d", "id"));
        assert!(like_ci("a\\_b", "a_b"));
        assert!(!like_ci("a\\_b", "axb"));
        assert!(!like_ci("na%", "amount"));
        assert!(like_ci("%", ""));
    }
}
