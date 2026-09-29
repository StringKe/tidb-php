use crate::auth;
use crate::dsn::{Compression, Config, SslMode};
use crate::error::{Error, Result};
use crate::io::PacketIo;
use crate::packet::{Reader, Writer};
use crate::protocol::{Handshake, Ok, cap, parse_err};
use crate::tls;

const MAX_PACKET: u32 = 1 << 30;

pub struct Negotiated {
    pub handshake: Handshake,
    pub capabilities: u32,
    pub ok: Ok,
    pub auth_plugin: String,
}

/// Collation id sent in the handshake; other charsets are applied with SET NAMES afterwards.
pub fn handshake_collation(cfg: &Config) -> (u8, bool) {
    match (cfg.charset.to_ascii_lowercase().as_str(), cfg.collation.as_deref()) {
        ("utf8mb4", None) => (46, false),
        ("utf8mb4", Some("utf8mb4_bin")) => (46, false),
        ("utf8mb4", Some("utf8mb4_general_ci")) => (45, false),
        ("utf8mb4", Some("utf8mb4_unicode_ci")) => (224, false),
        ("utf8mb4", Some("utf8mb4_0900_ai_ci")) => (255, false),
        ("binary", None) => (63, false),
        _ => (46, true),
    }
}

fn connect_attrs(cfg: &Config) -> Vec<u8> {
    let mut attrs: Vec<(String, String)> = vec![("_client_name".into(), "pdo_tidb".into()), ("_client_version".into(), env!("CARGO_PKG_VERSION").into()), ("_os".into(), std::env::consts::OS.into()), ("_platform".into(), std::env::consts::ARCH.into()), ("_pid".into(), std::process::id().to_string())];
    for (k, v) in &cfg.connect_attrs {
        attrs.retain(|(existing, _)| existing != k);
        attrs.push((k.clone(), v.clone()));
    }
    let mut body = Vec::new();
    for (k, v) in attrs {
        body.put_lenenc_bytes(k.as_bytes());
        body.put_lenenc_bytes(v.as_bytes());
    }
    body
}

pub fn run(io: &mut PacketIo, cfg: &Config, host: &str) -> Result<Negotiated> {
    let handshake = Handshake::parse(&io.read_packet()?)?;
    let server = handshake.capabilities;

    // LOCAL_FILES is always on: in-memory data from loadDataLocal() needs it, and file requests are checked client side in Conn::stream_file.
    let mut caps = cap::LONG_PASSWORD | cap::LONG_FLAG | cap::PROTOCOL_41 | cap::TRANSACTIONS | cap::SECURE_CONNECTION | cap::MULTI_RESULTS | cap::PS_MULTI_RESULTS | cap::PLUGIN_AUTH | cap::CONNECT_ATTRS | cap::DEPRECATE_EOF | cap::LOCAL_FILES;
    if cfg.multi_statements {
        caps |= cap::MULTI_STATEMENTS;
    }
    if cfg.found_rows {
        caps |= cap::FOUND_ROWS;
    }
    if cfg.database.is_some() {
        caps |= cap::CONNECT_WITH_DB;
    }
    match cfg.compression {
        Compression::Zlib => caps |= cap::COMPRESS,
        Compression::Zstd => caps |= cap::ZSTD_COMPRESSION_ALGORITHM,
        Compression::None => {}
    }
    caps &= server;
    if cfg.compression != Compression::None && caps & (cap::COMPRESS | cap::ZSTD_COMPRESSION_ALGORITHM) == 0 {
        return Err(Error::connect(format!("The server does not support {:?} compression", cfg.compression)));
    }

    let (collation, _) = handshake_collation(cfg);
    let use_tls = match cfg.ssl_mode {
        SslMode::Disable => false,
        SslMode::Preferred => server & cap::SSL != 0 && !io.is_unix(),
        _ => {
            if server & cap::SSL == 0 {
                return Err(Error { sqlstate: *b"08001", code: crate::error::client::SSL_CONNECTION, message: "TLS was required (sslmode) but the TiDB server has no TLS certificate configured".into(), fatal: true });
            }
            true
        }
    };
    if use_tls {
        caps |= cap::SSL;
        let mut req = Vec::with_capacity(32);
        req.put_u32(caps);
        req.put_u32(MAX_PACKET);
        req.push(collation);
        req.extend_from_slice(&[0; 23]);
        io.write_packet(&req)?;
        io.start_tls(tls::client_config(cfg)?, tls::server_name(host)?)?;
    }

    let password = cfg.password.as_deref().unwrap_or("").as_bytes();
    let first_plugin = cfg.auth_plugin.clone().unwrap_or_else(|| handshake.auth_plugin.clone());
    let started = auth::start(&first_plugin, &handshake.scramble, &cfg.user, password, io.is_secure())?;
    let mut plugin = started.plugin;
    let mut plugin_name = first_plugin;

    let mut resp = Vec::with_capacity(128);
    resp.put_u32(caps);
    resp.put_u32(MAX_PACKET);
    resp.push(collation);
    resp.extend_from_slice(&[0; 23]);
    resp.put_null_str(cfg.user.as_bytes());
    if started.response.len() > 255 {
        return Err(Error::auth("authentication response is longer than 255 bytes"));
    }
    resp.push(started.response.len() as u8);
    resp.extend_from_slice(&started.response);
    if let Some(db) = &cfg.database {
        resp.put_null_str(db.as_bytes());
    }
    resp.put_null_str(plugin_name.as_bytes());
    resp.put_lenenc_bytes(&connect_attrs(cfg));
    if caps & cap::ZSTD_COMPRESSION_ALGORITHM != 0 {
        resp.push(cfg.zstd_level);
    }
    io.write_packet(&resp)?;

    loop {
        let packet = io.read_packet()?;
        match packet.first() {
            Some(0x00) => {
                let ok = Ok::parse(&packet)?;
                match cfg.compression {
                    Compression::None => {}
                    c => io.enable_compression(c, cfg.zstd_level),
                }
                return Ok(Negotiated { handshake, capabilities: caps, ok, auth_plugin: plugin_name });
            }
            Some(0xFF) => return Err(parse_err(&packet).fatal()),
            Some(0xFE) => {
                let mut r = Reader::new(&packet[1..]);
                plugin_name = String::from_utf8_lossy(r.null_str_or_rest()).into_owned();
                let data = r.rest();
                let next = auth::start(&plugin_name, data, &cfg.user, password, io.is_secure())?;
                plugin = next.plugin;
                io.write_packet(&next.response)?;
            }
            Some(0x01) => {
                if let Some(reply) = plugin.respond(&packet[1..], password, io.is_secure())? {
                    io.write_packet(&reply)?;
                }
            }
            _ => return Err(Error::malformed("unexpected packet during authentication")),
        }
    }
}

/// COM_CHANGE_USER; TiDB re-runs the authentication exchange on the same connection.
pub fn change_user(io: &mut PacketIo, cfg: &Config, scramble: &[u8], plugin_name: &str) -> Result<(Ok, String)> {
    let password = cfg.password.as_deref().unwrap_or("").as_bytes();
    // Without TLS the clear-text plugins cannot open the exchange; a native scramble lets the server switch to whatever the new user needs.
    let plugin_name = if !io.is_secure() && matches!(plugin_name, auth::CACHING_SHA2 | auth::SM3) { auth::NATIVE } else { plugin_name };
    let started = auth::start_change_user(plugin_name, scramble, &cfg.user, password, io.is_secure())?;
    let mut plugin = started.plugin;
    let mut name = plugin_name.to_string();
    let (collation, _) = handshake_collation(cfg);
    let mut body = vec![crate::protocol::cmd::CHANGE_USER];
    body.put_null_str(cfg.user.as_bytes());
    body.push(started.response.len() as u8);
    body.extend_from_slice(&started.response);
    body.put_null_str(cfg.database.as_deref().unwrap_or("").as_bytes());
    body.put_u16(u16::from(collation));
    body.put_null_str(name.as_bytes());
    body.put_lenenc_bytes(&connect_attrs(cfg));
    io.reset_seq();
    io.write_packet(&body)?;
    loop {
        let packet = io.read_packet()?;
        match packet.first() {
            Some(0x00) => return Ok((Ok::parse(&packet)?, name)),
            Some(0xFF) => return Err(parse_err(&packet)),
            Some(0xFE) => {
                let mut r = Reader::new(&packet[1..]);
                name = String::from_utf8_lossy(r.null_str_or_rest()).into_owned();
                let next = auth::start_change_user(&name, r.rest(), &cfg.user, password, io.is_secure())?;
                plugin = next.plugin;
                io.write_packet(&next.response)?;
            }
            Some(0x01) => {
                if let Some(reply) = plugin.respond(&packet[1..], password, io.is_secure())? {
                    io.write_packet(&reply)?;
                }
            }
            _ => return Err(Error::malformed("unexpected packet during COM_CHANGE_USER").fatal()),
        }
    }
}
