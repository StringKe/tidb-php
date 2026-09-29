use std::fmt;
use std::io;

/// Client error codes, numbered like libmysqlclient's `CR_*` so tools that match on them keep working.
pub mod client {
    pub const UNKNOWN: u16 = 2000;
    pub const CONNECTION: u16 = 2002;
    pub const CONN_HOST: u16 = 2003;
    pub const UNKNOWN_HOST: u16 = 2005;
    pub const SERVER_GONE: u16 = 2006;
    pub const SERVER_LOST: u16 = 2013;
    pub const OUT_OF_SYNC: u16 = 2014;
    pub const NET_PACKET_TOO_LARGE: u16 = 2020;
    pub const SSL_CONNECTION: u16 = 2026;
    pub const MALFORMED_PACKET: u16 = 2027;
    pub const INVALID_PARAMETER: u16 = 2034;
    pub const NO_DATA: u16 = 2051;
    pub const AUTH_PLUGIN: u16 = 2061;
    pub const LOCAL_INFILE_REJECTED: u16 = 2068;
}

#[derive(Debug, Clone)]
pub struct Error {
    pub sqlstate: [u8; 5],
    pub code: u16,
    pub message: String,
    /// Set when the connection can no longer be used: the stream is broken or out of sync.
    pub fatal: bool,
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn server(code: u16, sqlstate: [u8; 5], message: String) -> Self {
        Self { sqlstate, code, message, fatal: false }
    }

    pub fn client(code: u16, message: impl Into<String>) -> Self {
        Self { sqlstate: *b"HY000", code, message: message.into(), fatal: false }
    }

    pub fn connect(message: impl Into<String>) -> Self {
        Self { sqlstate: *b"08001", code: client::CONNECTION, message: message.into(), fatal: true }
    }

    pub fn auth(message: impl Into<String>) -> Self {
        Self { sqlstate: *b"28000", code: client::AUTH_PLUGIN, message: message.into(), fatal: true }
    }

    pub fn malformed(what: &str) -> Self {
        Self { sqlstate: *b"HY000", code: client::MALFORMED_PACKET, message: format!("Malformed packet: {what}"), fatal: true }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::client(client::INVALID_PARAMETER, message)
    }

    pub fn fatal(mut self) -> Self {
        self.fatal = true;
        self
    }

    /// I/O failures after the handshake: the wording matches libmysqlclient because frameworks detect lost connections by message.
    pub fn io(err: &io::Error, during_query: bool) -> Self {
        let (code, message) = match err.kind() {
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => (client::SERVER_LOST, format!("Lost connection to TiDB server during query: read timeout ({err})")),
            _ if during_query => (client::SERVER_LOST, format!("Lost connection to TiDB server during query ({err})")),
            _ => (client::SERVER_GONE, format!("TiDB server has gone away ({err})")),
        };
        Self { sqlstate: *b"08S01", code, message, fatal: true }
    }

    pub fn class(&self) -> ErrorClass {
        ErrorClass::of(self.code, self.fatal)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

/// What a caller should do about an error, from TiDB's error code reference and the MySQL codes TiDB reuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorClass {
    /// Nothing to retry: fix the statement or the data.
    None,
    /// Roll back and run the whole transaction again: write conflicts, schema changes during the transaction, deadlocks.
    RetryTransaction,
    /// A TiKV, PD or TiFlash node is busy or unreachable; retry after a backoff.
    Backoff,
    /// `NOWAIT` found the row locked.
    Busy,
    /// The transaction TTL or lock was cleaned up by another transaction; retry once the lock is released.
    RetryAfterUnlock,
    /// The transaction or statement exceeded a hard limit and retrying cannot succeed.
    Fatal,
    /// The connection is gone; reconnect before retrying.
    Reconnect,
}

impl ErrorClass {
    pub fn of(code: u16, connection_lost: bool) -> Self {
        match code {
            8002 | 8005 | 8022 | 8028 | 9007 | 1213 | 1205 => Self::RetryTransaction,
            9001..=9005 | 9008 | 9010 | 8252 | 8027 => Self::Backoff,
            3572 => Self::Busy,
            8020 => Self::RetryAfterUnlock,
            8004 | 8025 | 9006 | 8141 | 8147 | 8200 | 8216 => Self::Fatal,
            client::SERVER_GONE | client::SERVER_LOST | client::CONNECTION | client::CONN_HOST => Self::Reconnect,
            _ if connection_lost => Self::Reconnect,
            _ => Self::None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::RetryTransaction => "retry_transaction",
            Self::Backoff => "backoff",
            Self::Busy => "busy",
            Self::RetryAfterUnlock => "retry_after_unlock",
            Self::Fatal => "fatal",
            Self::Reconnect => "reconnect",
        }
    }

    pub fn code(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_tidb_codes() {
        assert_eq!(ErrorClass::of(9007, false), ErrorClass::RetryTransaction);
        assert_eq!(ErrorClass::of(8022, false), ErrorClass::RetryTransaction);
        assert_eq!(ErrorClass::of(9005, false), ErrorClass::Backoff);
        assert_eq!(ErrorClass::of(3572, false), ErrorClass::Busy);
        assert_eq!(ErrorClass::of(8004, false), ErrorClass::Fatal);
        assert_eq!(ErrorClass::of(1062, false), ErrorClass::None);
        assert_eq!(ErrorClass::of(1062, true), ErrorClass::Reconnect);
        assert_eq!(ErrorClass::of(client::SERVER_GONE, false), ErrorClass::Reconnect);
    }

    #[test]
    fn io_errors_use_libmysqlclient_wording() {
        let lost = Error::io(&io::Error::from(io::ErrorKind::UnexpectedEof), true);
        let gone = Error::io(&io::Error::from(io::ErrorKind::BrokenPipe), false);
        assert!(lost.message.starts_with("Lost connection"));
        assert!(gone.message.contains("server has gone away"));
        assert!(lost.fatal && gone.fatal);
    }
}
