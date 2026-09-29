use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;

use flate2::Compression as ZlibLevel;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use rustls::{ClientConfig, ClientConnection, StreamOwned};

use crate::dsn::Compression;
use crate::error::{Error, Result, client};

pub const MAX_PAYLOAD: usize = 0xFF_FFFF;
/// TiDB sends payloads up to this size uncompressed; the client does the same.
const MIN_COMPRESS_LENGTH: usize = 50;

pub enum Socket {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(UnixStream),
}

impl Socket {
    pub fn tcp(host: &str, port: u16, timeout: Option<Duration>) -> Result<Self> {
        let addrs = (host, port).to_socket_addrs().map_err(|e| Error { sqlstate: *b"08001", code: client::UNKNOWN_HOST, message: format!("Unknown TiDB server host '{host}' ({e})"), fatal: true })?;
        let mut last = None;
        for addr in addrs {
            let attempt = match timeout {
                Some(t) => TcpStream::connect_timeout(&addr, t),
                None => TcpStream::connect(addr),
            };
            match attempt {
                Ok(stream) => {
                    let _ = stream.set_nodelay(true);
                    return Ok(Socket::Tcp(stream));
                }
                Err(e) => last = Some(e),
            }
        }
        let reason = last.map(|e| e.to_string()).unwrap_or_else(|| "no address".into());
        Err(Error { sqlstate: *b"08001", code: client::CONN_HOST, message: format!("Can't connect to TiDB server on '{host}:{port}' ({reason})"), fatal: true })
    }

    #[cfg(unix)]
    pub fn unix(path: &str) -> Result<Self> {
        UnixStream::connect(path).map(Socket::Unix).map_err(|e| Error { sqlstate: *b"08001", code: client::CONNECTION, message: format!("Can't connect to local TiDB server through socket '{path}' ({e})"), fatal: true })
    }

    pub fn is_unix(&self) -> bool {
        !matches!(self, Socket::Tcp(_))
    }

    fn set_timeouts(&self, read: Option<Duration>, write: Option<Duration>) -> io::Result<()> {
        match self {
            Socket::Tcp(s) => {
                s.set_read_timeout(read)?;
                s.set_write_timeout(write)
            }
            #[cfg(unix)]
            Socket::Unix(s) => {
                s.set_read_timeout(read)?;
                s.set_write_timeout(write)
            }
        }
    }

    fn shutdown(&self) {
        let _ = match self {
            Socket::Tcp(s) => s.shutdown(Shutdown::Both),
            #[cfg(unix)]
            Socket::Unix(s) => s.shutdown(Shutdown::Both),
        };
    }
}

impl Read for Socket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Socket::Tcp(s) => s.read(buf),
            #[cfg(unix)]
            Socket::Unix(s) => s.read(buf),
        }
    }
}

impl Write for Socket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Socket::Tcp(s) => s.write(buf),
            #[cfg(unix)]
            Socket::Unix(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Socket::Tcp(s) => s.flush(),
            #[cfg(unix)]
            Socket::Unix(s) => s.flush(),
        }
    }
}

enum Stream {
    Plain(Socket),
    Tls(Box<StreamOwned<ClientConnection, Socket>>),
    Closed,
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(s) => s.read(buf),
            Stream::Tls(s) => s.read(buf),
            Stream::Closed => Err(io::ErrorKind::NotConnected.into()),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(s) => s.write(buf),
            Stream::Tls(s) => s.write(buf),
            Stream::Closed => Err(io::ErrorKind::NotConnected.into()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Plain(s) => s.flush(),
            Stream::Tls(s) => s.flush(),
            Stream::Closed => Err(io::ErrorKind::NotConnected.into()),
        }
    }
}

/// Packet framing over one connection: sequence ids, 16 MiB splitting and the zlib / zstd compressed protocol.
pub struct PacketIo {
    stream: Stream,
    rbuf: Vec<u8>,
    rpos: usize,
    seq: u8,
    compressed_seq: u8,
    compression: Compression,
    zstd_level: i32,
    /// Uncompressed protocol bytes waiting to be read when compression is on.
    inflated: Vec<u8>,
    inflated_pos: usize,
    unix: bool,
}

impl PacketIo {
    pub fn new(socket: Socket, read_timeout: Option<Duration>, write_timeout: Option<Duration>) -> Result<Self> {
        socket.set_timeouts(read_timeout, write_timeout).map_err(|e| Error::connect(format!("cannot set socket timeouts: {e}")))?;
        let unix = socket.is_unix();
        Ok(Self { stream: Stream::Plain(socket), rbuf: Vec::with_capacity(16 * 1024), rpos: 0, seq: 0, compressed_seq: 0, compression: Compression::None, zstd_level: 3, inflated: Vec::new(), inflated_pos: 0, unix })
    }

    pub fn set_timeouts(&self, read: Option<Duration>, write: Option<Duration>) -> Result<()> {
        let socket = match &self.stream {
            Stream::Plain(s) => s,
            Stream::Tls(t) => &t.sock,
            Stream::Closed => return Ok(()),
        };
        socket.set_timeouts(read, write).map_err(|e| Error::connect(format!("cannot set socket timeouts: {e}")))
    }

    pub fn is_tls(&self) -> bool {
        matches!(self.stream, Stream::Tls(_))
    }

    pub fn is_unix(&self) -> bool {
        self.unix
    }

    /// True when a password may be sent in clear text: over TLS or a local socket.
    pub fn is_secure(&self) -> bool {
        self.is_tls() || self.unix
    }

    pub fn start_tls(&mut self, config: Arc<ClientConfig>, name: rustls::pki_types::ServerName<'static>) -> Result<()> {
        let socket = match std::mem::replace(&mut self.stream, Stream::Closed) {
            Stream::Plain(s) => s,
            other => {
                self.stream = other;
                return Err(Error::client(client::SSL_CONNECTION, "TLS is already active").fatal());
            }
        };
        let conn = ClientConnection::new(config, name).map_err(|e| Error { sqlstate: *b"08001", code: client::SSL_CONNECTION, message: format!("TLS setup failed: {e}"), fatal: true })?;
        let mut tls = StreamOwned::new(conn, socket);
        while tls.conn.is_handshaking() {
            tls.conn.complete_io(&mut tls.sock).map_err(|e| Error { sqlstate: *b"08001", code: client::SSL_CONNECTION, message: format!("TLS handshake failed: {e}"), fatal: true })?;
        }
        self.stream = Stream::Tls(Box::new(tls));
        Ok(())
    }

    pub fn tls_version(&self) -> Option<&'static str> {
        match &self.stream {
            Stream::Tls(s) => s.conn.protocol_version().map(|v| match v {
                rustls::ProtocolVersion::TLSv1_3 => "TLSv1.3",
                rustls::ProtocolVersion::TLSv1_2 => "TLSv1.2",
                _ => "TLS",
            }),
            _ => None,
        }
    }

    pub fn tls_cipher(&self) -> Option<String> {
        match &self.stream {
            Stream::Tls(s) => s.conn.negotiated_cipher_suite().map(|c| format!("{:?}", c.suite())),
            _ => None,
        }
    }

    pub fn enable_compression(&mut self, compression: Compression, zstd_level: u8) {
        self.compression = compression;
        self.zstd_level = i32::from(zstd_level);
    }

    pub fn compression(&self) -> Compression {
        self.compression
    }

    pub fn reset_seq(&mut self) {
        self.seq = 0;
        self.compressed_seq = 0;
    }

    pub fn close(&mut self) {
        if let Stream::Tls(tls) = &mut self.stream {
            tls.conn.send_close_notify();
            let _ = tls.conn.complete_io(&mut tls.sock);
        }
        match std::mem::replace(&mut self.stream, Stream::Closed) {
            Stream::Plain(s) => s.shutdown(),
            Stream::Tls(t) => t.sock.shutdown(),
            Stream::Closed => {}
        }
    }

    fn fill(&mut self, n: usize) -> io::Result<()> {
        if self.rpos > 0 && self.rpos == self.rbuf.len() {
            self.rbuf.clear();
            self.rpos = 0;
        }
        while self.rbuf.len() - self.rpos < n {
            if self.rpos > 64 * 1024 {
                self.rbuf.drain(..self.rpos);
                self.rpos = 0;
            }
            let start = self.rbuf.len();
            let want = (n - (start - self.rpos)).max(16 * 1024);
            self.rbuf.resize(start + want, 0);
            let got = self.stream.read(&mut self.rbuf[start..]);
            match got {
                Ok(0) => {
                    self.rbuf.truncate(start);
                    return Err(io::ErrorKind::UnexpectedEof.into());
                }
                Ok(k) => self.rbuf.truncate(start + k),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => self.rbuf.truncate(start),
                Err(e) => {
                    self.rbuf.truncate(start);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn take_wire(&mut self, n: usize) -> io::Result<&[u8]> {
        self.fill(n)?;
        let out = &self.rbuf[self.rpos..self.rpos + n];
        self.rpos += n;
        Ok(out)
    }

    /// Reads `n` bytes of uncompressed protocol stream, inflating compressed frames as needed.
    fn take(&mut self, n: usize, out: &mut Vec<u8>) -> Result<()> {
        if self.compression == Compression::None {
            let bytes = self.take_wire(n).map_err(|e| Error::io(&e, true))?;
            out.extend_from_slice(bytes);
            return Ok(());
        }
        let mut need = n;
        while need > 0 {
            if self.inflated_pos == self.inflated.len() {
                self.read_compressed_frame()?;
            }
            let k = need.min(self.inflated.len() - self.inflated_pos);
            out.extend_from_slice(&self.inflated[self.inflated_pos..self.inflated_pos + k]);
            self.inflated_pos += k;
            need -= k;
        }
        Ok(())
    }

    fn read_compressed_frame(&mut self) -> Result<()> {
        let header = self.take_wire(7).map_err(|e| Error::io(&e, true))?;
        let clen = usize::from(header[0]) | usize::from(header[1]) << 8 | usize::from(header[2]) << 16;
        let seq = header[3];
        let ulen = usize::from(header[4]) | usize::from(header[5]) << 8 | usize::from(header[6]) << 16;
        if seq != self.compressed_seq {
            return Err(Error::client(client::OUT_OF_SYNC, format!("Commands out of sync: compressed packet sequence {seq}, expected {}", self.compressed_seq)).fatal());
        }
        self.compressed_seq = self.compressed_seq.wrapping_add(1);
        let compression = self.compression;
        let payload = self.take_wire(clen).map_err(|e| Error::io(&e, true))?.to_vec();
        self.inflated.clear();
        self.inflated_pos = 0;
        if ulen == 0 {
            self.inflated.extend_from_slice(&payload);
            return Ok(());
        }
        self.inflated.reserve(ulen);
        let result = match compression {
            Compression::Zlib => ZlibDecoder::new(payload.as_slice()).read_to_end(&mut self.inflated).map(|_| ()),
            Compression::Zstd => zstd::stream::copy_decode(payload.as_slice(), &mut self.inflated),
            Compression::None => unreachable!(),
        };
        result.map_err(|e| Error::malformed(&format!("cannot decompress packet: {e}")))?;
        if self.inflated.len() != ulen {
            return Err(Error::malformed("decompressed length mismatch"));
        }
        Ok(())
    }

    /// Reads one logical packet, joining 16 MiB continuation packets.
    pub fn read_packet(&mut self) -> Result<Vec<u8>> {
        let mut payload = Vec::new();
        loop {
            let mut header = Vec::with_capacity(4);
            self.take(4, &mut header)?;
            let len = usize::from(header[0]) | usize::from(header[1]) << 8 | usize::from(header[2]) << 16;
            // Each server flush under compression restarts the inner sequence at the compressed one (server/internal/packetio.go Flush),
            // so only the compressed sequence is checked there.
            if matches!(self.compression, Compression::None) && header[3] != self.seq {
                return Err(Error::client(client::OUT_OF_SYNC, format!("Commands out of sync: packet sequence {}, expected {}", header[3], self.seq)).fatal());
            }
            self.seq = header[3].wrapping_add(1);
            self.take(len, &mut payload)?;
            if len < MAX_PAYLOAD {
                return Ok(payload);
            }
        }
    }

    /// Writes one logical packet, splitting it into 16 MiB pieces.
    pub fn write_packet(&mut self, payload: &[u8]) -> Result<()> {
        let mut framed = Vec::with_capacity(payload.len() + 4 * (payload.len() / MAX_PAYLOAD + 1));
        let mut chunks = payload.chunks(MAX_PAYLOAD).peekable();
        if payload.is_empty() {
            self.frame(&mut framed, &[]);
        }
        while let Some(chunk) = chunks.next() {
            self.frame(&mut framed, chunk);
            if chunks.peek().is_none() && chunk.len() == MAX_PAYLOAD {
                self.frame(&mut framed, &[]);
            }
        }
        self.write_wire(&framed)
    }

    fn frame(&mut self, out: &mut Vec<u8>, chunk: &[u8]) {
        out.extend_from_slice(&(chunk.len() as u32).to_le_bytes()[..3]);
        out.push(self.seq);
        self.seq = self.seq.wrapping_add(1);
        out.extend_from_slice(chunk);
    }

    fn write_wire(&mut self, data: &[u8]) -> Result<()> {
        if self.compression == Compression::None {
            self.stream.write_all(data).and_then(|()| self.stream.flush()).map_err(|e| Error::io(&e, false))?;
            return Ok(());
        }
        let mut out = Vec::with_capacity(data.len() / 2 + 64);
        for chunk in data.chunks(MAX_PAYLOAD) {
            let (body, ulen) = if chunk.len() <= MIN_COMPRESS_LENGTH { (chunk.to_vec(), 0) } else { (self.deflate(chunk)?, chunk.len()) };
            out.extend_from_slice(&(body.len() as u32).to_le_bytes()[..3]);
            out.push(self.compressed_seq);
            self.compressed_seq = self.compressed_seq.wrapping_add(1);
            out.extend_from_slice(&(ulen as u32).to_le_bytes()[..3]);
            out.extend_from_slice(&body);
        }
        self.stream.write_all(&out).and_then(|()| self.stream.flush()).map_err(|e| Error::io(&e, false))
    }

    fn deflate(&self, chunk: &[u8]) -> Result<Vec<u8>> {
        match self.compression {
            Compression::Zlib => {
                let mut enc = ZlibEncoder::new(Vec::with_capacity(chunk.len() / 2), ZlibLevel::default());
                enc.write_all(chunk).map_err(|e| Error::client(client::UNKNOWN, format!("zlib compression failed: {e}")))?;
                enc.finish().map_err(|e| Error::client(client::UNKNOWN, format!("zlib compression failed: {e}")))
            }
            Compression::Zstd => zstd::bulk::compress(chunk, self.zstd_level).map_err(|e| Error::client(client::UNKNOWN, format!("zstd compression failed: {e}"))),
            Compression::None => unreachable!(),
        }
    }
}

impl Drop for PacketIo {
    fn drop(&mut self) {
        self.close();
    }
}
