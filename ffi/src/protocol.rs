use crate::error::{Error, Result};
use crate::packet::Reader;

pub mod cap {
    pub const LONG_PASSWORD: u32 = 1;
    pub const FOUND_ROWS: u32 = 1 << 1;
    pub const LONG_FLAG: u32 = 1 << 2;
    pub const CONNECT_WITH_DB: u32 = 1 << 3;
    pub const COMPRESS: u32 = 1 << 5;
    pub const LOCAL_FILES: u32 = 1 << 7;
    pub const PROTOCOL_41: u32 = 1 << 9;
    pub const INTERACTIVE: u32 = 1 << 10;
    pub const SSL: u32 = 1 << 11;
    pub const TRANSACTIONS: u32 = 1 << 13;
    pub const SECURE_CONNECTION: u32 = 1 << 15;
    pub const MULTI_STATEMENTS: u32 = 1 << 16;
    pub const MULTI_RESULTS: u32 = 1 << 17;
    pub const PS_MULTI_RESULTS: u32 = 1 << 18;
    pub const PLUGIN_AUTH: u32 = 1 << 19;
    pub const CONNECT_ATTRS: u32 = 1 << 20;
    pub const PLUGIN_AUTH_LENENC_CLIENT_DATA: u32 = 1 << 21;
    pub const SESSION_TRACK: u32 = 1 << 23;
    pub const DEPRECATE_EOF: u32 = 1 << 24;
    pub const ZSTD_COMPRESSION_ALGORITHM: u32 = 1 << 26;
    pub const QUERY_ATTRIBUTES: u32 = 1 << 27;
}

pub mod status {
    pub const IN_TRANS: u16 = 1;
    pub const AUTOCOMMIT: u16 = 1 << 1;
    pub const MORE_RESULTS_EXISTS: u16 = 1 << 3;
    pub const CURSOR_EXISTS: u16 = 1 << 6;
    pub const LAST_ROW_SENT: u16 = 1 << 7;
    pub const NO_BACKSLASH_ESCAPES: u16 = 1 << 9;
    pub const IN_TRANS_READONLY: u16 = 1 << 13;
}

pub mod cmd {
    pub const QUIT: u8 = 0x01;
    pub const INIT_DB: u8 = 0x02;
    pub const QUERY: u8 = 0x03;
    pub const FIELD_LIST: u8 = 0x04;
    pub const REFRESH: u8 = 0x07;
    pub const STATISTICS: u8 = 0x09;
    pub const PING: u8 = 0x0E;
    pub const CHANGE_USER: u8 = 0x11;
    pub const STMT_PREPARE: u8 = 0x16;
    pub const STMT_EXECUTE: u8 = 0x17;
    pub const STMT_SEND_LONG_DATA: u8 = 0x18;
    pub const STMT_CLOSE: u8 = 0x19;
    pub const STMT_RESET: u8 = 0x1A;
    pub const SET_OPTION: u8 = 0x1B;
    pub const STMT_FETCH: u8 = 0x1C;
    pub const RESET_CONNECTION: u8 = 0x1F;
}

/// Column types from the binary protocol.
pub mod ty {
    pub const DECIMAL: u8 = 0x00;
    pub const TINY: u8 = 0x01;
    pub const SHORT: u8 = 0x02;
    pub const LONG: u8 = 0x03;
    pub const FLOAT: u8 = 0x04;
    pub const DOUBLE: u8 = 0x05;
    pub const NULL: u8 = 0x06;
    pub const TIMESTAMP: u8 = 0x07;
    pub const LONGLONG: u8 = 0x08;
    pub const INT24: u8 = 0x09;
    pub const DATE: u8 = 0x0A;
    pub const TIME: u8 = 0x0B;
    pub const DATETIME: u8 = 0x0C;
    pub const YEAR: u8 = 0x0D;
    pub const VARCHAR: u8 = 0x0F;
    pub const BIT: u8 = 0x10;
    pub const VECTOR: u8 = 0xE1;
    pub const JSON: u8 = 0xF5;
    pub const NEWDECIMAL: u8 = 0xF6;
    pub const ENUM: u8 = 0xF7;
    pub const SET: u8 = 0xF8;
    pub const TINY_BLOB: u8 = 0xF9;
    pub const MEDIUM_BLOB: u8 = 0xFA;
    pub const LONG_BLOB: u8 = 0xFB;
    pub const BLOB: u8 = 0xFC;
    pub const VAR_STRING: u8 = 0xFD;
    pub const STRING: u8 = 0xFE;
    pub const GEOMETRY: u8 = 0xFF;

    pub fn name(t: u8) -> &'static str {
        match t {
            DECIMAL | NEWDECIMAL => "NEWDECIMAL",
            TINY => "TINY",
            SHORT => "SHORT",
            LONG => "LONG",
            FLOAT => "FLOAT",
            DOUBLE => "DOUBLE",
            NULL => "NULL",
            TIMESTAMP => "TIMESTAMP",
            LONGLONG => "LONGLONG",
            INT24 => "INT24",
            DATE => "DATE",
            TIME => "TIME",
            DATETIME => "DATETIME",
            YEAR => "YEAR",
            VARCHAR => "VARCHAR",
            BIT => "BIT",
            VECTOR => "VECTOR",
            JSON => "JSON",
            ENUM => "ENUM",
            SET => "SET",
            TINY_BLOB => "TINY_BLOB",
            MEDIUM_BLOB => "MEDIUM_BLOB",
            LONG_BLOB => "LONG_BLOB",
            BLOB => "BLOB",
            VAR_STRING => "VAR_STRING",
            STRING => "STRING",
            GEOMETRY => "GEOMETRY",
            _ => "UNKNOWN",
        }
    }
}

pub mod flag {
    pub const NOT_NULL: u16 = 1;
    pub const PRI_KEY: u16 = 1 << 1;
    pub const UNIQUE_KEY: u16 = 1 << 2;
    pub const MULTIPLE_KEY: u16 = 1 << 3;
    pub const BLOB: u16 = 1 << 4;
    pub const UNSIGNED: u16 = 1 << 5;
    pub const ZEROFILL: u16 = 1 << 6;
    pub const BINARY: u16 = 1 << 7;
    pub const ENUM: u16 = 1 << 8;
    pub const AUTO_INCREMENT: u16 = 1 << 9;
    pub const TIMESTAMP: u16 = 1 << 10;
    pub const SET: u16 = 1 << 11;

    pub const NAMES: [(u16, &str); 12] =
        [(NOT_NULL, "not_null"), (PRI_KEY, "primary_key"), (UNIQUE_KEY, "unique_key"), (MULTIPLE_KEY, "multiple_key"), (BLOB, "blob"), (UNSIGNED, "unsigned"), (ZEROFILL, "zerofill"), (BINARY, "binary"), (ENUM, "enum"), (AUTO_INCREMENT, "auto_increment"), (TIMESTAMP, "timestamp"), (SET, "set")];
}

pub const BINARY_CHARSET: u16 = 63;

pub struct Handshake {
    pub server_version: String,
    pub connection_id: u32,
    pub scramble: Vec<u8>,
    pub capabilities: u32,
    pub collation: u8,
    pub status: u16,
    pub auth_plugin: String,
}

impl Handshake {
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.first() == Some(&0xFF) {
            return Err(parse_err(payload));
        }
        let mut r = Reader::new(payload);
        let protocol = r.u8()?;
        if protocol != 10 {
            return Err(Error::malformed(&format!("unsupported handshake protocol version {protocol}")));
        }
        let server_version = String::from_utf8_lossy(r.null_str()?).into_owned();
        let connection_id = r.u32()?;
        let mut scramble = r.bytes(8)?.to_vec();
        r.skip(1)?;
        let mut capabilities = u32::from(r.u16()?);
        let mut collation = 0;
        let mut status = 0;
        let mut auth_plugin = String::from("mysql_native_password");
        if !r.is_empty() {
            collation = r.u8()?;
            status = r.u16()?;
            capabilities |= u32::from(r.u16()?) << 16;
            let auth_len = r.u8()?;
            r.skip(10)?;
            if capabilities & cap::SECURE_CONNECTION != 0 {
                let len = (usize::from(auth_len).saturating_sub(8)).max(13);
                let part = r.bytes(len.min(r.remaining()))?;
                scramble.extend_from_slice(part.strip_suffix(&[0]).unwrap_or(part));
            }
            if capabilities & cap::PLUGIN_AUTH != 0 {
                auth_plugin = String::from_utf8_lossy(r.null_str_or_rest()).into_owned();
            }
        }
        Ok(Self { server_version, connection_id, scramble, capabilities, collation, status, auth_plugin })
    }
}

#[derive(Debug, Clone, Default)]
pub struct Ok {
    pub affected_rows: u64,
    pub last_insert_id: u64,
    pub status: u16,
    pub warnings: u16,
    pub info: String,
}

impl Ok {
    /// Parses an OK packet, or an EOF-style OK (0xFE header) when DEPRECATE_EOF is on.
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let mut r = Reader::new(payload);
        let header = r.u8()?;
        if header != 0x00 && header != 0xFE {
            return Err(Error::malformed("expected OK packet"));
        }
        let affected_rows = r.lenenc_int()?;
        let last_insert_id = r.lenenc_int()?;
        let status = r.u16()?;
        let warnings = r.u16()?;
        let info = if r.is_empty() { String::new() } else { String::from_utf8_lossy(r.lenenc_str()?).into_owned() };
        Ok(Self { affected_rows, last_insert_id, status, warnings, info })
    }
}

/// Legacy EOF packet: `0xFE`, warnings, status; shorter than 9 bytes.
pub fn parse_eof(payload: &[u8]) -> Result<(u16, u16)> {
    let mut r = Reader::new(payload);
    r.u8()?;
    if r.remaining() < 4 {
        return Ok((0, 0));
    }
    let warnings = r.u16()?;
    let status = r.u16()?;
    Ok((warnings, status))
}

pub fn is_eof(payload: &[u8]) -> bool {
    payload.first() == Some(&0xFE) && payload.len() < 9
}

pub fn parse_err(payload: &[u8]) -> Error {
    let mut r = Reader::new(payload);
    let parsed = (|| -> Result<Error> {
        r.u8()?;
        let code = r.u16()?;
        let mut state = *b"HY000";
        if r.peek() == Some(b'#') {
            r.skip(1)?;
            state.copy_from_slice(r.bytes(5)?);
        }
        Ok(Error::server(code, state, String::from_utf8_lossy(r.rest()).into_owned()))
    })();
    parsed.unwrap_or_else(|e| e)
}

#[derive(Debug, Clone)]
pub struct Column {
    pub schema: String,
    pub table: String,
    pub org_table: String,
    pub name: String,
    pub org_name: String,
    pub charset: u16,
    pub length: u32,
    pub ty: u8,
    pub flags: u16,
    pub decimals: u8,
}

impl Column {
    pub fn parse(payload: &[u8]) -> Result<Self> {
        let mut r = Reader::new(payload);
        let text = |b: &[u8]| String::from_utf8_lossy(b).into_owned();
        let _catalog = r.lenenc_str()?;
        let schema = text(r.lenenc_str()?);
        let table = text(r.lenenc_str()?);
        let org_table = text(r.lenenc_str()?);
        let name = text(r.lenenc_str()?);
        let org_name = text(r.lenenc_str()?);
        r.lenenc_int()?;
        let charset = r.u16()?;
        let length = r.u32()?;
        let ty = r.u8()?;
        let flags = r.u16()?;
        let decimals = r.u8()?;
        Ok(Self { schema, table, org_table, name, org_name, charset, length, ty, flags, decimals })
    }

    pub fn is_unsigned(&self) -> bool {
        self.flags & flag::UNSIGNED != 0
    }

    pub fn is_binary(&self) -> bool {
        self.charset == BINARY_CHARSET
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tidb_handshake() {
        let mut p = vec![10];
        p.extend_from_slice(b"8.0.11-TiDB-v8.5.8\0");
        p.extend_from_slice(&7u32.to_le_bytes());
        p.extend_from_slice(b"abcdefgh\0");
        p.extend_from_slice(&0xF7FFu16.to_le_bytes());
        p.push(46);
        p.extend_from_slice(&2u16.to_le_bytes());
        p.extend_from_slice(&0x05BFu16.to_le_bytes());
        p.push(21);
        p.extend_from_slice(&[0; 10]);
        p.extend_from_slice(b"ijklmnopqrst\0");
        p.extend_from_slice(b"mysql_native_password\0");
        let h = Handshake::parse(&p).unwrap();
        assert_eq!(h.server_version, "8.0.11-TiDB-v8.5.8");
        assert_eq!(h.connection_id, 7);
        assert_eq!(h.scramble, b"abcdefghijklmnopqrst");
        assert_eq!(h.auth_plugin, "mysql_native_password");
        assert!(h.capabilities & cap::PLUGIN_AUTH != 0);
    }

    #[test]
    fn parses_ok_and_err() {
        let ok = Ok::parse(&[0, 3, 0xFC, 0x10, 0x27, 2, 0, 1, 0, 2, b'h', b'i']).unwrap();
        assert_eq!((ok.affected_rows, ok.last_insert_id, ok.status, ok.warnings, ok.info.as_str()), (3, 10000, 2, 1, "hi"));
        let mut e = vec![0xFF, 0x26, 0x04, b'#'];
        e.extend_from_slice(b"23000Duplicate entry '1' for key 't.PRIMARY'");
        let err = parse_err(&e);
        assert_eq!((err.code, &err.sqlstate, err.message.as_str()), (1062, b"23000", "Duplicate entry '1' for key 't.PRIMARY'"));
    }
}
