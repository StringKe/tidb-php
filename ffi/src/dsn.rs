use std::time::Duration;

use crate::error::{Error, Result};

pub const DEFAULT_PORT: u16 = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SslMode {
    Disable,
    Preferred,
    Required,
    VerifyCa,
    VerifyIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Zlib,
    Zstd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub endpoints: Vec<Endpoint>,
    pub unix_socket: Option<String>,
    pub user: String,
    pub password: Option<String>,
    pub database: Option<String>,
    pub charset: String,
    pub collation: Option<String>,
    pub ssl_mode: SslMode,
    pub ssl_ca: Option<String>,
    pub ssl_capath: Option<String>,
    pub ssl_cert: Option<String>,
    pub ssl_key: Option<String>,
    pub tls_min_version: Option<String>,
    pub compression: Compression,
    pub zstd_level: u8,
    pub connect_timeout: Option<Duration>,
    pub read_timeout: Option<Duration>,
    pub write_timeout: Option<Duration>,
    pub multi_statements: bool,
    pub found_rows: bool,
    pub local_infile: bool,
    pub local_infile_directory: Option<String>,
    pub discover: bool,
    pub discover_ttl: Duration,
    pub max_lifetime: Option<Duration>,
    pub idle_ping: Option<Duration>,
    pub init_commands: Vec<String>,
    /// Session variables set right after the handshake, in DSN order.
    pub session: Vec<(String, String)>,
    pub connect_attrs: Vec<(String, String)>,
    pub auth_plugin: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            endpoints: vec![Endpoint { host: "127.0.0.1".into(), port: DEFAULT_PORT }],
            unix_socket: None,
            user: String::new(),
            password: None,
            database: None,
            charset: "utf8mb4".into(),
            collation: None,
            ssl_mode: SslMode::Preferred,
            ssl_ca: None,
            ssl_capath: None,
            ssl_cert: None,
            ssl_key: None,
            tls_min_version: None,
            compression: Compression::None,
            zstd_level: 3,
            connect_timeout: None,
            read_timeout: None,
            write_timeout: None,
            multi_statements: true,
            found_rows: false,
            local_infile: false,
            local_infile_directory: None,
            discover: false,
            discover_ttl: Duration::from_secs(60),
            max_lifetime: None,
            idle_ping: None,
            init_commands: Vec::new(),
            session: Vec::new(),
            connect_attrs: Vec::new(),
            auth_plugin: None,
        }
    }
}

impl Config {
    /// Parses the part of the DSN after `tidb:`, then applies `overrides` (driver options) in order.
    pub fn parse(dsn: &str, user: Option<&str>, password: Option<&str>, overrides: &[(String, String)]) -> Result<Self> {
        let mut cfg = Config::default();
        let mut ssl_mode_set = false;
        let mut hosts: Option<String> = None;
        let mut port: Option<u16> = None;

        let pairs = dsn.split(';').filter(|p| !p.trim().is_empty()).map(|p| match p.split_once('=') {
            Some((k, v)) => Ok((k.trim().to_string(), v.trim().to_string())),
            None => Err(Error::invalid(format!("invalid DSN part \"{p}\": expected key=value"))),
        });
        let pairs: Vec<(String, String)> = pairs.chain(overrides.iter().cloned().map(Ok)).collect::<Result<_>>()?;

        for (key, value) in pairs {
            match key.as_str() {
                "host" => hosts = Some(value),
                "port" => port = Some(parse_num(&key, &value)?),
                "dbname" | "database" => cfg.database = non_empty(value),
                "unix_socket" => cfg.unix_socket = non_empty(value),
                "user" => cfg.user = value,
                "password" => cfg.password = Some(value),
                "charset" => cfg.charset = value,
                "collation" => cfg.collation = non_empty(value),
                "sslmode" | "ssl_mode" => {
                    cfg.ssl_mode = parse_ssl_mode(&value)?;
                    ssl_mode_set = true;
                }
                "ssl_ca" => cfg.ssl_ca = non_empty(value),
                "ssl_capath" => cfg.ssl_capath = non_empty(value),
                "ssl_cert" => cfg.ssl_cert = non_empty(value),
                "ssl_key" => cfg.ssl_key = non_empty(value),
                "tls_version" => cfg.tls_min_version = non_empty(value),
                "compression" => cfg.compression = parse_compression(&value)?,
                "zstd_level" => cfg.zstd_level = parse_num::<u8>(&key, &value).and_then(|l| if (1..=22).contains(&l) { Ok(l) } else { Err(Error::invalid("zstd_level must be between 1 and 22")) })?,
                "connect_timeout" => cfg.connect_timeout = parse_secs(&key, &value)?,
                "read_timeout" => cfg.read_timeout = parse_secs(&key, &value)?,
                "write_timeout" => cfg.write_timeout = parse_secs(&key, &value)?,
                "multi_statements" => cfg.multi_statements = parse_bool(&key, &value)?,
                "found_rows" => cfg.found_rows = parse_bool(&key, &value)?,
                "local_infile" => cfg.local_infile = parse_bool(&key, &value)?,
                "local_infile_directory" => cfg.local_infile_directory = non_empty(value),
                "discover" => cfg.discover = parse_bool(&key, &value)?,
                "discover_ttl" => cfg.discover_ttl = parse_secs(&key, &value)?.unwrap_or(Duration::ZERO),
                "max_lifetime" => cfg.max_lifetime = parse_secs(&key, &value)?,
                "idle_ping" => cfg.idle_ping = parse_secs(&key, &value)?,
                "init_command" => cfg.init_commands.extend(non_empty(value)),
                "time_zone" => cfg.session.push(("time_zone".into(), value)),
                "max_execution_time" => cfg.session.push(("max_execution_time".into(), value)),
                "idle_transaction_timeout" => cfg.session.push(("tidb_idle_transaction_timeout".into(), value)),
                "auth_plugin" => cfg.auth_plugin = non_empty(value),
                k if k.starts_with("session.") => cfg.session.push((k["session.".len()..].to_string(), value)),
                k if k.starts_with("attr.") => cfg.connect_attrs.push((k["attr.".len()..].to_string(), value)),
                _ => return Err(Error::invalid(format!("unknown DSN key \"{key}\""))),
            }
        }

        if let Some(u) = user.filter(|u| !u.is_empty()) {
            cfg.user = u.to_string();
        }
        if let Some(p) = password {
            cfg.password = Some(p.to_string());
        }
        if let Some(h) = hosts {
            cfg.endpoints = parse_hosts(&h, port.unwrap_or(DEFAULT_PORT))?;
        } else if let Some(p) = port {
            cfg.endpoints[0].port = p;
        }
        for (name, _) in &cfg.session {
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(Error::invalid(format!("invalid session variable name \"{name}\"")));
            }
        }
        if !ssl_mode_set && cfg.unix_socket.is_none() && cfg.endpoints.iter().any(|e| is_tidb_cloud(&e.host)) {
            cfg.ssl_mode = SslMode::VerifyIdentity;
        }
        if cfg.unix_socket.is_some() && !ssl_mode_set {
            cfg.ssl_mode = SslMode::Disable;
        }
        Ok(cfg)
    }

    /// `SET` statement for the configured session variables; numbers stay bare, everything else is quoted.
    pub fn session_statement(&self) -> Option<String> {
        if self.session.is_empty() {
            return None;
        }
        let parts: Vec<String> = self.session.iter().map(|(k, v)| format!("@@SESSION.{k} = {}", sql_literal(v))).collect();
        Some(format!("SET {}", parts.join(", ")))
    }
}

pub fn is_tidb_cloud(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host.ends_with(".tidbcloud.com") || host.ends_with(".tidbcloud.com.")
}

pub fn sql_literal(v: &str) -> String {
    let numeric = !v.is_empty() && v.trim_start_matches('-').bytes().all(|b| b.is_ascii_digit() || b == b'.') && v.bytes().filter(|&b| b == b'.').count() <= 1 && v != "-";
    if numeric || matches!(v.to_ascii_uppercase().as_str(), "ON" | "OFF" | "DEFAULT" | "TRUE" | "FALSE") {
        return v.to_string();
    }
    let mut out = String::with_capacity(v.len() + 2);
    out.push('\'');
    for c in v.chars() {
        match c {
            '\'' => out.push_str("''"),
            '\\' => out.push_str("\\\\"),
            '\0' => out.push_str("\\0"),
            c => out.push(c),
        }
    }
    out.push('\'');
    out
}

fn parse_hosts(value: &str, default_port: u16) -> Result<Vec<Endpoint>> {
    let mut out = Vec::new();
    for item in value.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let (host, port) = if let Some(rest) = item.strip_prefix('[') {
            let (h, tail) = rest.split_once(']').ok_or_else(|| Error::invalid(format!("invalid host \"{item}\"")))?;
            match tail.strip_prefix(':') {
                Some(p) => (h, parse_num("port", p)?),
                None if tail.is_empty() => (h, default_port),
                None => return Err(Error::invalid(format!("invalid host \"{item}\""))),
            }
        } else if item.matches(':').count() == 1 {
            let (h, p) = item.split_once(':').unwrap_or((item, ""));
            (h, parse_num("port", p)?)
        } else {
            (item, default_port)
        };
        out.push(Endpoint { host: host.to_string(), port });
    }
    if out.is_empty() {
        return Err(Error::invalid("host is empty"));
    }
    Ok(out)
}

fn non_empty(v: String) -> Option<String> {
    if v.is_empty() { None } else { Some(v) }
}

fn parse_num<T: std::str::FromStr>(key: &str, v: &str) -> Result<T> {
    v.parse().map_err(|_| Error::invalid(format!("invalid value \"{v}\" for {key}")))
}

fn parse_secs(key: &str, v: &str) -> Result<Option<Duration>> {
    let secs: f64 = parse_num(key, v)?;
    if !secs.is_finite() || secs < 0.0 {
        return Err(Error::invalid(format!("invalid value \"{v}\" for {key}")));
    }
    Ok(if secs == 0.0 { None } else { Some(Duration::from_secs_f64(secs)) })
}

fn parse_bool(key: &str, v: &str) -> Result<bool> {
    match v.to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Ok(true),
        "0" | "false" | "off" | "no" | "" => Ok(false),
        _ => Err(Error::invalid(format!("invalid value \"{v}\" for {key}"))),
    }
}

fn parse_ssl_mode(v: &str) -> Result<SslMode> {
    Ok(match v.to_ascii_lowercase().replace('-', "_").as_str() {
        "disable" | "disabled" => SslMode::Disable,
        "prefer" | "preferred" => SslMode::Preferred,
        "require" | "required" => SslMode::Required,
        "verify_ca" => SslMode::VerifyCa,
        "verify_identity" | "verify_full" => SslMode::VerifyIdentity,
        _ => return Err(Error::invalid(format!("invalid sslmode \"{v}\""))),
    })
}

fn parse_compression(v: &str) -> Result<Compression> {
    Ok(match v.to_ascii_lowercase().as_str() {
        "" | "none" | "off" | "0" | "false" => Compression::None,
        "zlib" | "1" | "true" | "on" => Compression::Zlib,
        "zstd" => Compression::Zstd,
        _ => return Err(Error::invalid(format!("invalid compression \"{v}\""))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hosts_and_options() {
        let cfg = Config::parse("host=a:4001,[::1]:4002,b;port=4003;dbname=app;compression=zstd;zstd_level=7;time_zone=+00:00;session.tidb_mem_quota_query=1073741824", Some("root"), None, &[]).unwrap();
        assert_eq!(cfg.endpoints, vec![Endpoint { host: "a".into(), port: 4001 }, Endpoint { host: "::1".into(), port: 4002 }, Endpoint { host: "b".into(), port: 4003 }]);
        assert_eq!(cfg.database.as_deref(), Some("app"));
        assert_eq!(cfg.compression, Compression::Zstd);
        assert_eq!(cfg.zstd_level, 7);
        assert_eq!(cfg.session_statement().unwrap(), "SET @@SESSION.time_zone = '+00:00', @@SESSION.tidb_mem_quota_query = 1073741824");
    }

    #[test]
    fn tidb_cloud_hosts_verify_identity_by_default() {
        let cfg = Config::parse("host=gateway01.ap-southeast-1.prod.aws.tidbcloud.com", None, None, &[]).unwrap();
        assert_eq!(cfg.ssl_mode, SslMode::VerifyIdentity);
        let cfg = Config::parse("host=gateway01.ap-southeast-1.prod.aws.tidbcloud.com;sslmode=required", None, None, &[]).unwrap();
        assert_eq!(cfg.ssl_mode, SslMode::Required);
    }

    #[test]
    fn overrides_win_and_bad_input_is_rejected() {
        let cfg = Config::parse("dbname=a", None, None, &[("dbname".into(), "b".into())]).unwrap();
        assert_eq!(cfg.database.as_deref(), Some("b"));
        assert!(Config::parse("nope=1", None, None, &[]).is_err());
        assert!(Config::parse("session.a b=1", None, None, &[]).is_err());
        assert!(Config::parse("zstd_level=30", None, None, &[]).is_err());
    }

    #[test]
    fn quotes_session_values() {
        assert_eq!(sql_literal("it's"), "'it''s'");
        assert_eq!(sql_literal("-12.5"), "-12.5");
        assert_eq!(sql_literal("ON"), "ON");
        assert_eq!(sql_literal("1.2.3"), "'1.2.3'");
    }
}
