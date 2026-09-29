//! C ABI used by the PHP extension (declared in src/tidb_ffi.h). Handles and out-parameters cross as
//! `Option<&T>` / `Option<&mut T>` / `Option<Box<T>>`, which Rust lays out as nullable C pointers.

use std::ffi::c_char;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Once;

use crate::conn::{Conn, Mode, ResultSet, Stmt};
use crate::dsn::{Compression, Config};
use crate::error::{Error, ErrorClass, Result, client};
use crate::protocol::{Column, status, ty};
use crate::value::{Cell, Param};

#[repr(C)]
pub struct TidbStr {
    pub ptr: *const u8,
    pub len: usize,
}

impl TidbStr {
    const NONE: Self = Self { ptr: std::ptr::null(), len: 0 };

    fn of(bytes: &[u8]) -> Self {
        Self { ptr: bytes.as_ptr(), len: bytes.len() }
    }

    /// # Safety
    /// `ptr` must be null or point to `len` bytes that stay readable for `'a`.
    unsafe fn bytes<'a>(&self) -> Option<&'a [u8]> {
        if self.ptr.is_null() { None } else { Some(unsafe { std::slice::from_raw_parts(self.ptr, self.len) }) }
    }

    /// # Safety
    /// Same as [`TidbStr::bytes`].
    unsafe fn text<'a>(&self, what: &str) -> Result<Option<&'a str>> {
        match unsafe { self.bytes() } {
            None => Ok(None),
            Some(b) => std::str::from_utf8(b).map(Some).map_err(|_| Error::invalid(format!("{what} is not valid UTF-8"))),
        }
    }
}

/// # Safety
/// `ptr` must be null or point to `len` readable items that stay valid for `'a`.
unsafe fn slice<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if ptr.is_null() || len == 0 { &[] } else { unsafe { std::slice::from_raw_parts(ptr, len) } }
}

/// NUL-terminated bytes allocated here; released with `tidb_owned_free`.
#[repr(C)]
pub struct TidbOwned {
    pub ptr: *mut u8,
    pub len: usize,
}

impl TidbOwned {
    fn from_bytes(mut bytes: Vec<u8>) -> Self {
        let len = bytes.len();
        bytes.push(0);
        Self { ptr: Box::into_raw(bytes.into_boxed_slice()).cast::<u8>(), len }
    }

    fn release(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: a non-null `ptr` only comes from `from_bytes`, which leaked a boxed slice of `len + 1` bytes.
            drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(self.ptr, self.len + 1)) });
        }
        self.ptr = std::ptr::null_mut();
        self.len = 0;
    }
}

fn write_owned(out: Option<&mut TidbOwned>, value: impl Into<Vec<u8>>) -> bool {
    match out {
        Some(out) => {
            out.release();
            *out = TidbOwned::from_bytes(value.into());
            true
        }
        None => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_owned_free(owned: Option<&mut TidbOwned>) {
    if let Some(owned) = owned {
        owned.release();
    }
}

#[repr(C)]
pub struct TidbError {
    pub sqlstate: [c_char; 6],
    pub code: i32,
    pub error_class: i32,
    pub fatal: bool,
    pub message: TidbOwned,
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_error_clear(error: Option<&mut TidbError>) {
    if let Some(e) = error {
        e.message.release();
        e.sqlstate = [0; 6];
        e.code = 0;
        e.error_class = 0;
        e.fatal = false;
    }
}

fn set_error(error: Option<&mut TidbError>, failure: &Error) {
    let Some(e) = error else { return };
    e.message.release();
    let mut sqlstate = [0 as c_char; 6];
    for (slot, byte) in sqlstate.iter_mut().zip(failure.sqlstate) {
        *slot = byte as c_char;
    }
    e.sqlstate = sqlstate;
    e.code = i32::from(failure.code);
    e.error_class = failure.class().code();
    e.fatal = failure.fatal;
    e.message = TidbOwned::from_bytes(failure.message.clone().into_bytes());
}

/// Runs `body`, turning errors and panics into a filled `TidbError` and the `fallback` value.
fn guard<T>(error: Option<&mut TidbError>, fallback: T, body: impl FnOnce() -> Result<T>) -> T {
    static QUIET_PANICS: Once = Once::new();
    QUIET_PANICS.call_once(|| panic::set_hook(Box::new(|_| {})));
    match panic::catch_unwind(AssertUnwindSafe(body)) {
        Ok(Ok(v)) => v,
        Ok(Err(failure)) => {
            set_error(error, &failure);
            fallback
        }
        Err(payload) => {
            let detail = payload.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| payload.downcast_ref::<String>().cloned()).unwrap_or_default();
            set_error(error, &Error::client(client::UNKNOWN, format!("pdo_tidb internal error: {detail}")));
            fallback
        }
    }
}

fn quietly(body: impl FnOnce()) {
    let _ = panic::catch_unwind(AssertUnwindSafe(body));
}

fn need<'a, T>(handle: Option<&'a mut T>, what: &str) -> Result<&'a mut T> {
    handle.ok_or_else(|| Error::invalid(format!("{what} handle is null")))
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_driver_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_error_class_name(class: i32) -> *const c_char {
    let name: &'static [u8] = match class {
        1 => b"retry_transaction\0",
        2 => b"backoff\0",
        3 => b"busy\0",
        4 => b"retry_after_unlock\0",
        5 => b"fatal\0",
        6 => b"reconnect\0",
        _ => b"none\0",
    };
    name.as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_error_class_of(code: i32, connection_lost: bool) -> i32 {
    u16::try_from(code).map(|c| ErrorClass::of(c, connection_lost).code()).unwrap_or(0)
}

/// Pairs of driver options that override DSN keys.
#[repr(C)]
pub struct TidbOption {
    pub key: TidbStr,
    pub value: TidbStr,
}

/// # Safety
/// Every `TidbStr` must satisfy [`TidbStr::bytes`]; `options` must point to `count` items or be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_connect(dsn: TidbStr, user: TidbStr, password: TidbStr, options: *const TidbOption, count: usize, error: Option<&mut TidbError>) -> Option<Box<Conn>> {
    guard(error, None, || {
        let dsn = unsafe { dsn.text("DSN") }?.unwrap_or("");
        let user = unsafe { user.text("user name") }?;
        let password = unsafe { password.text("password") }?;
        let mut overrides = Vec::new();
        for o in unsafe { slice(options, count) } {
            let key = unsafe { o.key.text("option name") }?.unwrap_or("").to_string();
            let value = unsafe { o.value.text("option value") }?.unwrap_or("").to_string();
            overrides.push((key, value));
        }
        let cfg = Config::parse(dsn, user, password, &overrides)?;
        Conn::connect(cfg).map(Box::new).map(Some)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_conn_free(conn: Option<Box<Conn>>) {
    if let Some(mut conn) = conn {
        quietly(|| conn.close());
    }
}

fn mode_of(mode: i32, fetch_size: u32) -> Mode {
    match mode {
        1 => Mode::Streaming,
        2 => Mode::Cursor(fetch_size.max(1)),
        _ => Mode::Buffered,
    }
}

/// # Safety
/// `sql` must satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_query(conn: Option<&mut Conn>, sql: TidbStr, mode: i32, fetch_size: u32, error: Option<&mut TidbError>) -> Option<Box<ResultSet>> {
    guard(error, None, || {
        let conn = need(conn, "connection")?;
        let sql = unsafe { sql.bytes() }.unwrap_or_default();
        conn.query(sql, mode_of(mode, fetch_size)).map(Box::new).map(Some)
    })
}

/// Runs a statement and discards every result; the affected rows and insert id stay readable on the connection.
///
/// # Safety
/// `sql` must satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_exec(conn: Option<&mut Conn>, sql: TidbStr, affected: Option<&mut u64>, error: Option<&mut TidbError>) -> bool {
    guard(error, false, || {
        let conn = need(conn, "connection")?;
        let sql = unsafe { sql.bytes() }.unwrap_or_default();
        let mut rs = conn.query(sql, Mode::Streaming)?;
        let first = rs.ok.affected_rows;
        let first_id = rs.ok.last_insert_id;
        conn.finish(&mut rs)?;
        conn.last.affected_rows = first;
        conn.last.last_insert_id = first_id;
        if let Some(a) = affected {
            *a = first;
        }
        Ok(true)
    })
}

/// # Safety
/// `sql` must satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_prepare(conn: Option<&mut Conn>, sql: TidbStr, error: Option<&mut TidbError>) -> Option<Box<Stmt>> {
    guard(error, None, || {
        let conn = need(conn, "connection")?;
        let sql = unsafe { sql.bytes() }.unwrap_or_default();
        conn.prepare(sql).map(Box::new).map(Some)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_stmt_param_count(stmt: Option<&Stmt>) -> usize {
    stmt.map_or(0, |s| s.params.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_stmt_column_count(stmt: Option<&Stmt>) -> usize {
    stmt.map_or(0, |s| s.columns.len())
}

pub const PARAM_NULL: i32 = 0;
pub const PARAM_INT: i32 = 1;
pub const PARAM_FLOAT: i32 = 2;
pub const PARAM_BYTES: i32 = 3;
pub const PARAM_LONG_DATA: i32 = 4;

#[repr(C)]
pub struct TidbParam {
    pub kind: i32,
    pub int_value: i64,
    pub float_value: f64,
    pub data: TidbStr,
}

/// # Safety
/// `params` must point to `count` items (or be null with `count == 0`) whose `data` satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_stmt_execute(conn: Option<&mut Conn>, stmt: Option<&Stmt>, params: *const TidbParam, count: usize, mode: i32, fetch_size: u32, error: Option<&mut TidbError>) -> Option<Box<ResultSet>> {
    guard(error, None, || {
        let conn = need(conn, "connection")?;
        let stmt = stmt.ok_or_else(|| Error::invalid("statement handle is null"))?;
        let mut bound = Vec::with_capacity(count);
        let mut long = Vec::new();
        for (i, p) in unsafe { slice(params, count) }.iter().enumerate() {
            let data = unsafe { p.data.bytes() }.unwrap_or(&[]);
            bound.push(match p.kind {
                PARAM_INT => Param::Int(p.int_value),
                PARAM_FLOAT => Param::Float(p.float_value),
                PARAM_BYTES => Param::Bytes(data),
                PARAM_LONG_DATA => {
                    let index = u16::try_from(i).map_err(|_| Error::invalid("too many parameters"))?;
                    long.push((index, data));
                    Param::LongData
                }
                _ => Param::Null,
            });
        }
        conn.execute(stmt, &bound, &long, mode_of(mode, fetch_size)).map(Box::new).map(Some)
    })
}

/// Frees the statement and, when `conn` is given, closes it on the server.
#[unsafe(no_mangle)]
pub extern "C" fn tidb_stmt_free(conn: Option<&mut Conn>, stmt: Option<Box<Stmt>>) {
    if let (Some(conn), Some(stmt)) = (conn, stmt) {
        quietly(|| {
            let _ = conn.close_stmt(stmt.id);
        });
    }
}

#[repr(C)]
pub struct TidbColumn {
    pub name: TidbStr,
    pub org_name: TidbStr,
    pub table: TidbStr,
    pub org_table: TidbStr,
    pub schema: TidbStr,
    pub native_type: TidbStr,
    pub type_id: u8,
    pub decimals: u8,
    pub flags: u16,
    pub charset: u16,
    pub length: u32,
}

fn column_out(c: &Column) -> TidbColumn {
    TidbColumn {
        name: TidbStr::of(c.name.as_bytes()),
        org_name: TidbStr::of(c.org_name.as_bytes()),
        table: TidbStr::of(c.table.as_bytes()),
        org_table: TidbStr::of(c.org_table.as_bytes()),
        schema: TidbStr::of(c.schema.as_bytes()),
        native_type: TidbStr::of(ty::name(c.ty).as_bytes()),
        type_id: c.ty,
        decimals: c.decimals,
        flags: c.flags,
        charset: c.charset,
        length: c.length,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_column_count(result: Option<&ResultSet>) -> usize {
    result.map_or(0, |r| r.columns.len())
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_column(result: Option<&ResultSet>, index: usize, out: Option<&mut TidbColumn>) -> bool {
    match (result.and_then(|r| r.columns.get(index)), out) {
        (Some(c), Some(out)) => {
            *out = column_out(c);
            true
        }
        _ => false,
    }
}

/// 1 when a row was read, 0 at the end of the current result set, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_next(conn: Option<&mut Conn>, result: Option<&mut ResultSet>, error: Option<&mut TidbError>) -> i32 {
    guard(error, -1, || {
        let conn = need(conn, "connection")?;
        let rs = need(result, "result")?;
        conn.next_row(rs).map(i32::from)
    })
}

pub const CELL_NULL: i32 = 0;
pub const CELL_INT: i32 = 1;
pub const CELL_FLOAT: i32 = 2;
pub const CELL_BYTES: i32 = 3;

/// `data` stays valid until the next `tidb_result_next` or `tidb_result_free`.
#[repr(C)]
pub struct TidbCell {
    pub kind: i32,
    pub int_value: i64,
    pub float_value: f64,
    pub data: TidbStr,
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_cell(result: Option<&ResultSet>, index: usize, out: Option<&mut TidbCell>) -> bool {
    let (Some(rs), Some(out)) = (result, out) else { return false };
    let Some(&cell) = rs.current.cells.get(index) else { return false };
    *out = match cell {
        Cell::Null => TidbCell { kind: CELL_NULL, int_value: 0, float_value: 0.0, data: TidbStr::NONE },
        Cell::Int(v) => TidbCell { kind: CELL_INT, int_value: v, float_value: 0.0, data: TidbStr::NONE },
        Cell::Float(v) => TidbCell { kind: CELL_FLOAT, int_value: 0, float_value: v, data: TidbStr::NONE },
        Cell::Bytes { .. } => TidbCell { kind: CELL_BYTES, int_value: 0, float_value: 0.0, data: TidbStr::of(rs.current.bytes(cell).unwrap_or(&[])) },
    };
    true
}

/// 1 when the next result set is now current, 0 when there is none, -1 on error.
#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_next_rowset(conn: Option<&mut Conn>, result: Option<&mut ResultSet>, error: Option<&mut TidbError>) -> i32 {
    guard(error, -1, || {
        let conn = need(conn, "connection")?;
        let rs = need(result, "result")?;
        conn.next_result(rs).map(i32::from)
    })
}

/// Reads and discards everything the server still sends for this command.
#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_close(conn: Option<&mut Conn>, result: Option<&mut ResultSet>, error: Option<&mut TidbError>) -> bool {
    guard(error, false, || {
        let conn = need(conn, "connection")?;
        let rs = need(result, "result")?;
        conn.finish(rs).map(|()| true)
    })
}

#[repr(C)]
pub struct TidbResultInfo {
    pub affected_rows: u64,
    pub last_insert_id: u64,
    pub warnings: u16,
    pub status: u16,
    pub more_results: bool,
    pub is_cursor: bool,
    pub has_row_count: bool,
    pub row_count: u64,
    pub fetched: u64,
    pub info: TidbStr,
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_get_info(result: Option<&ResultSet>, out: Option<&mut TidbResultInfo>) -> bool {
    let (Some(rs), Some(out)) = (result, out) else { return false };
    *out = TidbResultInfo {
        affected_rows: rs.ok.affected_rows,
        last_insert_id: rs.ok.last_insert_id,
        warnings: rs.ok.warnings,
        status: rs.ok.status,
        more_results: rs.more_results,
        is_cursor: rs.is_cursor(),
        has_row_count: rs.buffered_rows.is_some(),
        row_count: rs.buffered_rows.unwrap_or(0),
        fetched: rs.fetched,
        info: TidbStr::of(rs.ok.info.as_bytes()),
    };
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_result_free(result: Option<Box<ResultSet>>) {
    drop(result);
}

pub const ACTION_PING: i32 = 0;
pub const ACTION_RESET_CONNECTION: i32 = 1;
pub const ACTION_KILL_QUERY: i32 = 2;
pub const ACTION_MULTI_STATEMENTS_ON: i32 = 3;
pub const ACTION_MULTI_STATEMENTS_OFF: i32 = 4;

#[unsafe(no_mangle)]
pub extern "C" fn tidb_conn_action(conn: Option<&mut Conn>, action: i32, error: Option<&mut TidbError>) -> bool {
    guard(error, false, || {
        let conn = need(conn, "connection")?;
        match action {
            ACTION_PING => conn.ping()?,
            ACTION_RESET_CONNECTION => conn.reset_connection()?,
            ACTION_KILL_QUERY => conn.kill_query()?,
            ACTION_MULTI_STATEMENTS_ON => conn.set_multi_statements(true)?,
            ACTION_MULTI_STATEMENTS_OFF => conn.set_multi_statements(false)?,
            _ => return Err(Error::invalid(format!("unknown connection action {action}"))),
        }
        Ok(true)
    })
}

/// # Safety
/// `db` must satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_init_db(conn: Option<&mut Conn>, db: TidbStr, error: Option<&mut TidbError>) -> bool {
    guard(error, false, || {
        let conn = need(conn, "connection")?;
        conn.init_db(unsafe { db.text("database") }?.unwrap_or("")).map(|()| true)
    })
}

/// # Safety
/// Every `TidbStr` must satisfy [`TidbStr::bytes`]; a null `password` or `database` means none.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_change_user(conn: Option<&mut Conn>, user: TidbStr, password: TidbStr, database: TidbStr, error: Option<&mut TidbError>) -> bool {
    guard(error, false, || {
        let conn = need(conn, "connection")?;
        let user = unsafe { user.text("user name") }?.unwrap_or("");
        let password = unsafe { password.text("password") }?;
        let database = unsafe { database.text("database") }?;
        conn.change_user(user, password, database).map(|()| true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_conn_statistics(conn: Option<&mut Conn>, out: Option<&mut TidbOwned>, error: Option<&mut TidbError>) -> bool {
    guard(error, false, || {
        let conn = need(conn, "connection")?;
        let text = conn.statistics()?;
        Ok(write_owned(out, text))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tidb_conn_refresh(conn: Option<&mut Conn>, options: u8, error: Option<&mut TidbError>) -> bool {
    guard(error, false, || need(conn, "connection")?.refresh(options).map(|()| true))
}

/// COM_FIELD_LIST: a result with the table's column definitions and no rows.
///
/// # Safety
/// Both strings must satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_field_list(conn: Option<&mut Conn>, table: TidbStr, wildcard: TidbStr, error: Option<&mut TidbError>) -> Option<Box<ResultSet>> {
    guard(error, None, || {
        let conn = need(conn, "connection")?;
        let table = unsafe { table.text("table") }?.unwrap_or("");
        let wildcard = unsafe { wildcard.text("wildcard") }?.unwrap_or("");
        let columns = conn.field_list(table, wildcard)?;
        Ok(Some(Box::new(ResultSet::columns_only(columns))))
    })
}

/// Data for the next LOAD DATA LOCAL INFILE, used whatever file name the statement names.
///
/// # Safety
/// `data` must satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_set_infile_data(conn: Option<&mut Conn>, data: TidbStr) {
    if let Some(conn) = conn {
        conn.infile_data = unsafe { data.bytes() }.map(<[u8]>::to_vec);
    }
}

/// Quotes a value for emulated prepares, honouring NO_BACKSLASH_ESCAPES; binary values get the `_binary` introducer.
///
/// # Safety
/// `value` must satisfy [`TidbStr::bytes`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tidb_conn_quote(conn: Option<&Conn>, value: TidbStr, binary: bool, out: Option<&mut TidbOwned>) -> bool {
    let no_backslash = conn.is_some_and(|c| c.status & status::NO_BACKSLASH_ESCAPES != 0);
    let bytes = unsafe { value.bytes() }.unwrap_or(&[]);
    write_owned(out, quote(bytes, binary, no_backslash))
}

pub fn quote(bytes: &[u8], binary: bool, no_backslash: bool) -> Vec<u8> {
    let mut q = Vec::with_capacity(bytes.len() + 10);
    if binary {
        q.extend_from_slice(b"_binary");
    }
    q.push(b'\'');
    for &b in bytes {
        match b {
            b'\'' if no_backslash => q.extend_from_slice(b"''"),
            _ if no_backslash => q.push(b),
            b'\'' => q.extend_from_slice(b"\\'"),
            b'"' => q.extend_from_slice(b"\\\""),
            b'\\' => q.extend_from_slice(b"\\\\"),
            0 => q.extend_from_slice(b"\\0"),
            b'\n' => q.extend_from_slice(b"\\n"),
            b'\r' => q.extend_from_slice(b"\\r"),
            0x1A => q.extend_from_slice(b"\\Z"),
            _ => q.push(b),
        }
    }
    q.push(b'\'');
    q
}

pub const TEXT_SERVER_VERSION: i32 = 0;
pub const TEXT_ENDPOINT: i32 = 1;
pub const TEXT_AUTH_PLUGIN: i32 = 2;
pub const TEXT_TLS_VERSION: i32 = 3;
pub const TEXT_TLS_CIPHER: i32 = 4;
pub const TEXT_COMPRESSION: i32 = 5;
pub const TEXT_LAST_INFO: i32 = 6;
pub const TEXT_DISCOVERY_ERROR: i32 = 7;
pub const TEXT_DATABASE: i32 = 8;

/// Writes a text fact about the connection; false when it has no value (for example no TLS).
#[unsafe(no_mangle)]
pub extern "C" fn tidb_conn_text(conn: Option<&Conn>, what: i32, out: Option<&mut TidbOwned>) -> bool {
    let Some(c) = conn else { return false };
    let value: Option<String> = match what {
        TEXT_SERVER_VERSION => Some(c.server_version.clone()),
        TEXT_ENDPOINT => Some(c.endpoint.clone()),
        TEXT_AUTH_PLUGIN => Some(c.auth_plugin.clone()),
        TEXT_TLS_VERSION => c.tls_version().map(str::to_string),
        TEXT_TLS_CIPHER => c.tls_cipher(),
        TEXT_COMPRESSION => match c.compression() {
            Compression::None => None,
            Compression::Zlib => Some("zlib".into()),
            Compression::Zstd => Some("zstd".into()),
        },
        TEXT_LAST_INFO => Some(c.last.info.clone()).filter(|s| !s.is_empty()),
        TEXT_DISCOVERY_ERROR => c.discovery_error.clone(),
        TEXT_DATABASE => c.cfg.database.clone(),
        _ => None,
    };
    let Some(v) = value else { return false };
    write_owned(out, v);
    true
}

pub const NUM_CONNECTION_ID: i32 = 0;
pub const NUM_STATUS: i32 = 1;
pub const NUM_WARNINGS: i32 = 2;
pub const NUM_AFFECTED_ROWS: i32 = 3;
pub const NUM_LAST_INSERT_ID: i32 = 4;
pub const NUM_CAPABILITIES: i32 = 5;
pub const NUM_IN_TRANSACTION: i32 = 6;
pub const NUM_HAS_PENDING_RESULT: i32 = 7;
pub const NUM_BROKEN: i32 = 8;

#[unsafe(no_mangle)]
pub extern "C" fn tidb_conn_number(conn: Option<&Conn>, what: i32) -> u64 {
    let Some(c) = conn else { return 0 };
    match what {
        NUM_CONNECTION_ID => u64::from(c.connection_id),
        NUM_STATUS => u64::from(c.status),
        NUM_WARNINGS => u64::from(c.last.warnings),
        NUM_AFFECTED_ROWS => c.last.affected_rows,
        NUM_LAST_INSERT_ID => c.last.last_insert_id,
        NUM_CAPABILITIES => u64::from(c.capabilities),
        NUM_IN_TRANSACTION => u64::from(c.in_transaction()),
        NUM_HAS_PENDING_RESULT => u64::from(c.has_pending_result()),
        NUM_BROKEN => u64::from(c.broken),
        _ => 0,
    }
}

/// Persistent-connection check: false when the connection is broken, older than `max_lifetime`, or fails a ping after `idle_ping` idle time.
#[unsafe(no_mangle)]
pub extern "C" fn tidb_conn_alive(conn: Option<&mut Conn>) -> bool {
    let Some(c) = conn else { return false };
    let mut alive = false;
    quietly(|| alive = c.is_alive());
    alive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_like_libmysqlclient() {
        assert_eq!(quote(b"a'b\\c\0\n\x1a", false, false), b"'a\\'b\\\\c\\0\\n\\Z'".to_vec());
        assert_eq!(quote(b"a'b\\c", false, true), b"'a''b\\c'".to_vec());
        assert_eq!(quote(b"\xff", true, false), b"_binary'\xff'".to_vec());
    }

    #[test]
    fn error_class_names_match_codes() {
        let name = |c| unsafe { std::ffi::CStr::from_ptr(tidb_error_class_name(c)) }.to_str().unwrap().to_string();
        assert_eq!(name(ErrorClass::RetryTransaction.code()), ErrorClass::RetryTransaction.as_str());
        assert_eq!(name(ErrorClass::Reconnect.code()), ErrorClass::Reconnect.as_str());
        assert_eq!(tidb_error_class_of(9007, false), ErrorClass::RetryTransaction.code());
    }
}
