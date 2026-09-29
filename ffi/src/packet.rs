use crate::error::{Error, Result};

/// Cursor over one packet payload.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    pub fn peek(&self) -> Option<u8> {
        self.buf.get(self.pos).copied()
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.remaining() < n {
            return Err(Error::malformed("unexpected end of packet"));
        }
        let out = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(out)
    }

    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.bytes(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u24(&mut self) -> Result<u32> {
        let b = self.bytes(3)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], 0]))
    }

    pub fn u32(&mut self) -> Result<u32> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u64(&mut self) -> Result<u64> {
        let b = self.bytes(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    pub fn uint(&mut self, n: usize) -> Result<u64> {
        let b = self.bytes(n)?;
        Ok(b.iter().rev().fold(0u64, |acc, &x| (acc << 8) | u64::from(x)))
    }

    /// Length-encoded integer; `None` is the 0xFB NULL marker used in text rows.
    pub fn lenenc(&mut self) -> Result<Option<u64>> {
        match self.u8()? {
            0xFB => Ok(None),
            0xFC => Ok(Some(u64::from(self.u16()?))),
            0xFD => Ok(Some(u64::from(self.u24()?))),
            0xFE => self.u64().map(Some),
            0xFF => Err(Error::malformed("invalid length-encoded integer")),
            n => Ok(Some(u64::from(n))),
        }
    }

    pub fn lenenc_int(&mut self) -> Result<u64> {
        self.lenenc()?.ok_or_else(|| Error::malformed("unexpected NULL length"))
    }

    pub fn lenenc_bytes(&mut self) -> Result<Option<&'a [u8]>> {
        match self.lenenc()? {
            None => Ok(None),
            Some(n) => self.bytes(usize::try_from(n).map_err(|_| Error::malformed("length overflow"))?).map(Some),
        }
    }

    pub fn lenenc_str(&mut self) -> Result<&'a [u8]> {
        self.lenenc_bytes()?.ok_or_else(|| Error::malformed("unexpected NULL string"))
    }

    pub fn null_str(&mut self) -> Result<&'a [u8]> {
        let rest = &self.buf[self.pos..];
        let end = rest.iter().position(|&b| b == 0).ok_or_else(|| Error::malformed("unterminated string"))?;
        self.pos += end + 1;
        Ok(&rest[..end])
    }

    /// A NUL-terminated string that may also run to the end of the packet without a terminator.
    pub fn null_str_or_rest(&mut self) -> &'a [u8] {
        let rest = &self.buf[self.pos..];
        match rest.iter().position(|&b| b == 0) {
            Some(end) => {
                self.pos += end + 1;
                &rest[..end]
            }
            None => self.rest(),
        }
    }

    pub fn rest(&mut self) -> &'a [u8] {
        let out = &self.buf[self.pos..];
        self.pos = self.buf.len();
        out
    }
}

pub trait Writer {
    fn put_u16(&mut self, v: u16);
    fn put_u24(&mut self, v: u32);
    fn put_u32(&mut self, v: u32);
    fn put_u64(&mut self, v: u64);
    fn put_lenenc(&mut self, v: u64);
    fn put_lenenc_bytes(&mut self, v: &[u8]);
    fn put_null_str(&mut self, v: &[u8]);
}

impl Writer for Vec<u8> {
    fn put_u16(&mut self, v: u16) {
        self.extend_from_slice(&v.to_le_bytes());
    }

    fn put_u24(&mut self, v: u32) {
        self.extend_from_slice(&v.to_le_bytes()[..3]);
    }

    fn put_u32(&mut self, v: u32) {
        self.extend_from_slice(&v.to_le_bytes());
    }

    fn put_u64(&mut self, v: u64) {
        self.extend_from_slice(&v.to_le_bytes());
    }

    fn put_lenenc(&mut self, v: u64) {
        match v {
            0..=250 => self.push(v as u8),
            251..=0xFFFF => {
                self.push(0xFC);
                self.put_u16(v as u16);
            }
            0x1_0000..=0xFF_FFFF => {
                self.push(0xFD);
                self.put_u24(v as u32);
            }
            _ => {
                self.push(0xFE);
                self.put_u64(v);
            }
        }
    }

    fn put_lenenc_bytes(&mut self, v: &[u8]) {
        self.put_lenenc(v.len() as u64);
        self.extend_from_slice(v);
    }

    fn put_null_str(&mut self, v: &[u8]) {
        self.extend_from_slice(v);
        self.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lenenc_round_trip() {
        for v in [0u64, 250, 251, 65535, 65536, 0xFF_FFFF, 0x100_0000, u64::MAX] {
            let mut buf = Vec::new();
            buf.put_lenenc(v);
            assert_eq!(Reader::new(&buf).lenenc_int().unwrap(), v);
        }
        assert_eq!(Reader::new(&[0xFB]).lenenc().unwrap(), None);
    }

    #[test]
    fn null_strings() {
        let mut r = Reader::new(b"abc\0def");
        assert_eq!(r.null_str().unwrap(), b"abc");
        assert!(r.null_str().is_err());
        assert_eq!(Reader::new(b"def").null_str_or_rest(), b"def");
    }
}
