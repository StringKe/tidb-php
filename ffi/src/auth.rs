use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use hmac::{Hmac, Mac};
use rand::RngCore;
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

pub const NATIVE: &str = "mysql_native_password";
pub const CACHING_SHA2: &str = "caching_sha2_password";
pub const SM3: &str = "tidb_sm3_password";
pub const CLEAR: &str = "mysql_clear_password";
pub const SOCKET: &str = "auth_socket";
pub const LDAP_SASL: &str = "authentication_ldap_sasl_client";

/// One authentication exchange; `respond` is fed every AuthMoreData payload (without the 0x01 marker).
pub enum Plugin {
    Native,
    /// caching_sha2_password and tidb_sm3_password: TiDB always asks for full authentication, which sends the password in clear text.
    FullAuth {
        name: &'static str,
    },
    Clear,
    Socket,
    Scram(Scram),
}

pub struct Start {
    pub plugin: Plugin,
    pub response: Vec<u8>,
}

/// Picks the plugin named by the server and computes the first response. `secure` is true on TLS or a local socket.
pub fn start(name: &str, data: &[u8], user: &str, password: &[u8], secure: bool) -> Result<Start> {
    let scramble = data.strip_suffix(&[0]).unwrap_or(data);
    match name {
        NATIVE => Ok(Start { plugin: Plugin::Native, response: native_scramble(password, scramble) }),
        CACHING_SHA2 => Ok(Start { plugin: Plugin::FullAuth { name: CACHING_SHA2 }, response: sha256_scramble(password, scramble) }),
        SM3 => Ok(Start { plugin: Plugin::FullAuth { name: SM3 }, response: sha256_scramble(password, scramble) }),
        CLEAR => {
            require_secure(CLEAR, secure)?;
            Ok(Start { plugin: Plugin::Clear, response: with_nul(password) })
        }
        SOCKET => Ok(Start { plugin: Plugin::Socket, response: Vec::new() }),
        LDAP_SASL => {
            let method = String::from_utf8_lossy(scramble).trim().to_ascii_uppercase();
            let scram = Scram::new(&method, user, password)?;
            let response = scram.client_first();
            Ok(Start { plugin: Plugin::Scram(scram), response })
        }
        other => Err(Error::auth(format!("Authentication plugin '{other}' is not supported by pdo_tidb"))),
    }
}

/// Same as [`start`] for COM_CHANGE_USER. TiDB skips the full-authentication round there (server/conn.go handleChangeUser) and checks the
/// caching_sha2_password / tidb_sm3_password response as the clear-text password, without a trailing NUL.
pub fn start_change_user(name: &str, data: &[u8], user: &str, password: &[u8], secure: bool) -> Result<Start> {
    match name {
        CACHING_SHA2 | SM3 => {
            require_secure(name, secure)?;
            let name = if name == SM3 { SM3 } else { CACHING_SHA2 };
            Ok(Start { plugin: Plugin::FullAuth { name }, response: password.to_vec() })
        }
        _ => start(name, data, user, password, secure),
    }
}

impl Plugin {
    /// Answer to an AuthMoreData packet; `None` means read the next packet without sending anything.
    pub fn respond(&mut self, data: &[u8], password: &[u8], secure: bool) -> Result<Option<Vec<u8>>> {
        match self {
            Plugin::FullAuth { name } => match data.first() {
                Some(3) => Ok(None),
                Some(4) => {
                    require_secure(name, secure)?;
                    Ok(Some(with_nul(password)))
                }
                _ => Err(Error::auth(format!("unexpected {name} auth data"))),
            },
            Plugin::Scram(scram) => scram.step(data),
            _ => Err(Error::auth("unexpected extra authentication data")),
        }
    }
}

fn require_secure(plugin: &str, secure: bool) -> Result<()> {
    if secure { Ok(()) } else { Err(Error::auth(format!("Authentication plugin '{plugin}' sends the password in clear text and requires TLS or a Unix socket; enable TLS (sslmode=required or stronger)"))) }
}

fn with_nul(password: &[u8]) -> Vec<u8> {
    let mut out = password.to_vec();
    out.push(0);
    out
}

pub fn native_scramble(password: &[u8], scramble: &[u8]) -> Vec<u8> {
    if password.is_empty() {
        return Vec::new();
    }
    let stage1 = Sha1::digest(password);
    let stage2 = Sha1::digest(stage1);
    let mut h = Sha1::new();
    h.update(scramble);
    h.update(stage2);
    let mix = h.finalize();
    stage1.iter().zip(mix.iter()).map(|(a, b)| a ^ b).collect()
}

pub fn sha256_scramble(password: &[u8], scramble: &[u8]) -> Vec<u8> {
    if password.is_empty() {
        return Vec::new();
    }
    let p1 = Sha256::digest(password);
    let p2 = Sha256::digest(p1);
    let mut h = Sha256::new();
    h.update(p2);
    h.update(scramble);
    let p3 = h.finalize();
    p1.iter().zip(p3.iter()).map(|(a, b)| a ^ b).collect()
}

#[derive(Clone, Copy)]
enum ScramHash {
    Sha1,
    Sha256,
}

/// SCRAM (RFC 5802) client for authentication_ldap_sasl_client; TiDB relays each step to the LDAP server.
pub struct Scram {
    hash: ScramHash,
    password: Vec<u8>,
    client_first_bare: String,
    nonce: String,
    server_signature: Option<Vec<u8>>,
}

macro_rules! hmac {
    ($d:ty, $key:expr, $($part:expr),+) => {{
        let mut mac = <Hmac<$d> as Mac>::new_from_slice($key).expect("HMAC accepts any key length");
        $(mac.update($part);)+
        mac.finalize().into_bytes().to_vec()
    }};
}

impl Scram {
    fn new(method: &str, user: &str, password: &[u8]) -> Result<Self> {
        let hash = match method {
            "SCRAM-SHA-1" => ScramHash::Sha1,
            "SCRAM-SHA-256" => ScramHash::Sha256,
            other => return Err(Error::auth(format!("LDAP SASL method '{other}' is not supported by pdo_tidb; use SCRAM-SHA-1 or SCRAM-SHA-256"))),
        };
        let mut raw = [0u8; 24];
        rand::rng().fill_bytes(&mut raw);
        let nonce = B64.encode(raw);
        let name = user.replace('=', "=3D").replace(',', "=2C");
        Ok(Self { hash, password: password.to_vec(), client_first_bare: format!("n={name},r={nonce}"), nonce, server_signature: None })
    }

    fn client_first(&self) -> Vec<u8> {
        format!("n,,{}", self.client_first_bare).into_bytes()
    }

    fn step(&mut self, data: &[u8]) -> Result<Option<Vec<u8>>> {
        let message = std::str::from_utf8(data).map_err(|_| Error::auth("SCRAM message is not UTF-8"))?;
        if let Some(expected) = &self.server_signature {
            let v = attr(message, 'v').ok_or_else(|| Error::auth(format!("SCRAM server final message rejected: {message}")))?;
            if B64.decode(v).ok().as_deref() != Some(expected.as_slice()) {
                return Err(Error::auth("SCRAM server signature does not match"));
            }
            return Ok(None);
        }
        let nonce = attr(message, 'r').ok_or_else(|| Error::auth("SCRAM server first message has no nonce"))?;
        let salt = attr(message, 's').and_then(|s| B64.decode(s).ok()).ok_or_else(|| Error::auth("SCRAM server first message has no salt"))?;
        let iterations: u32 = attr(message, 'i').and_then(|i| i.parse().ok()).filter(|&i| i > 0).ok_or_else(|| Error::auth("SCRAM server first message has no iteration count"))?;
        if !nonce.starts_with(&self.nonce) {
            return Err(Error::auth("SCRAM server nonce does not extend the client nonce"));
        }
        let without_proof = format!("c=biws,r={nonce}");
        let auth_message = format!("{},{message},{without_proof}", self.client_first_bare);
        let (proof, server_signature) = match self.hash {
            ScramHash::Sha1 => {
                let mut salted = [0u8; 20];
                pbkdf2::pbkdf2_hmac::<Sha1>(&self.password, &salt, iterations, &mut salted);
                let client_key = hmac!(Sha1, &salted, b"Client Key");
                let stored = Sha1::digest(&client_key);
                let signature = hmac!(Sha1, &stored, auth_message.as_bytes());
                let server_key = hmac!(Sha1, &salted, b"Server Key");
                (xor(&client_key, &signature), hmac!(Sha1, &server_key, auth_message.as_bytes()))
            }
            ScramHash::Sha256 => {
                let mut salted = [0u8; 32];
                pbkdf2::pbkdf2_hmac::<Sha256>(&self.password, &salt, iterations, &mut salted);
                let client_key = hmac!(Sha256, &salted, b"Client Key");
                let stored = Sha256::digest(&client_key);
                let signature = hmac!(Sha256, &stored, auth_message.as_bytes());
                let server_key = hmac!(Sha256, &salted, b"Server Key");
                (xor(&client_key, &signature), hmac!(Sha256, &server_key, auth_message.as_bytes()))
            }
        };
        self.server_signature = Some(server_signature);
        Ok(Some(format!("{without_proof},p={}", B64.encode(proof)).into_bytes()))
    }
}

fn attr(message: &str, key: char) -> Option<&str> {
    message.split(',').find_map(|part| part.strip_prefix(key).and_then(|rest| rest.strip_prefix('=')))
}

fn xor(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b).map(|(x, y)| x ^ y).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_scramble_matches_mysql() {
        let scramble = b"\x3a\x2c\x6e\x0f\x21\x33\x55\x0a\x42\x61\x3f\x1b\x5e\x70\x21\x4c\x09\x3d\x5b\x10";
        assert_eq!(native_scramble(b"", scramble), Vec::<u8>::new());
        assert_eq!(native_scramble(b"secret", scramble).len(), 20);
        assert_ne!(native_scramble(b"secret", scramble), native_scramble(b"other", scramble));
    }

    #[test]
    fn full_auth_needs_a_secure_channel() {
        let mut start = start(SM3, b"01234567890123456789\0", "u", b"pw", false).unwrap();
        assert!(start.plugin.respond(&[3], b"pw", false).unwrap().is_none());
        assert!(start.plugin.respond(&[4], b"pw", false).is_err());
        assert_eq!(start.plugin.respond(&[4], b"pw", true).unwrap().unwrap(), b"pw\0");
        assert!(super::start(CLEAR, b"", "u", b"pw", false).is_err());
    }

    #[test]
    fn scram_sha1_rfc5802_vector() {
        let mut scram = Scram::new("SCRAM-SHA-1", "user", b"pencil").unwrap();
        scram.nonce = "fyko+d2lbbFgONRv9qkxdawL".into();
        scram.client_first_bare = "n=user,r=fyko+d2lbbFgONRv9qkxdawL".into();
        let final_msg = scram.step(b"r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,s=QSXCR+Q6sek8bf92,i=4096").unwrap().unwrap();
        assert_eq!(String::from_utf8(final_msg).unwrap(), "c=biws,r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,p=v0X8v3Bz2T0CJGbJQyF0X+HI4Ts=");
        assert!(scram.step(b"v=rmF9pqV8S7suAoZWja4dJRkFsKQ=").unwrap().is_none());
    }
}
