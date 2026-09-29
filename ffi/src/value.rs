use std::fmt::Write as _;

use crate::error::{Error, Result};
use crate::packet::{Reader, Writer};
use crate::protocol::{Column, ty};

/// One decoded cell. `Bytes` points into the owning row buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    Null,
    Int(i64),
    Float(f64),
    Bytes { start: usize, len: usize },
}

/// A row: the cells plus the bytes they reference (the packet payload, then any formatted temporal values).
#[derive(Debug, Default, Clone)]
pub struct Row {
    pub buf: Vec<u8>,
    pub cells: Vec<Cell>,
}

impl Row {
    pub fn bytes(&self, cell: Cell) -> Option<&[u8]> {
        match cell {
            Cell::Bytes { start, len } => Some(&self.buf[start..start + len]),
            _ => None,
        }
    }
}

fn is_integer(t: u8) -> bool {
    matches!(t, ty::TINY | ty::SHORT | ty::LONG | ty::LONGLONG | ty::INT24 | ty::YEAR)
}

/// Text-protocol values become ints and floats for numeric columns, as mysqlnd does; everything else stays bytes.
fn typed_text(col: &Column, bytes: &[u8], start: usize) -> Cell {
    let text = || std::str::from_utf8(bytes).ok();
    if is_integer(col.ty) {
        if let Some(v) = text().and_then(|s| s.parse::<i64>().ok()) {
            return Cell::Int(v);
        }
    } else if matches!(col.ty, ty::FLOAT | ty::DOUBLE)
        && let Some(v) = text().and_then(|s| s.parse::<f64>().ok())
    {
        return Cell::Float(v);
    }
    Cell::Bytes { start, len: bytes.len() }
}

pub fn decode_text_row(payload: Vec<u8>, columns: &[Column]) -> Result<Row> {
    let mut cells = Vec::with_capacity(columns.len());
    {
        let mut r = Reader::new(&payload);
        for col in columns {
            match r.lenenc_bytes()? {
                None => cells.push(Cell::Null),
                Some(bytes) => {
                    let start = payload.len() - r.remaining() - bytes.len();
                    cells.push(typed_text(col, bytes, start));
                }
            }
        }
    }
    Ok(Row { buf: payload, cells })
}

pub fn decode_binary_row(payload: Vec<u8>, columns: &[Column]) -> Result<Row> {
    let mut cells = Vec::with_capacity(columns.len());
    let mut extra = String::new();
    let mut formatted: Vec<(usize, usize, usize)> = Vec::new();
    {
        let mut r = Reader::new(&payload);
        if r.u8()? != 0x00 {
            return Err(Error::malformed("binary row header"));
        }
        let bitmap = r.bytes((columns.len() + 9) / 8)?;
        for (i, col) in columns.iter().enumerate() {
            let bit = i + 2;
            if bitmap[bit / 8] & (1 << (bit % 8)) != 0 {
                cells.push(Cell::Null);
                continue;
            }
            let unsigned = col.is_unsigned();
            let cell = match col.ty {
                ty::TINY => Cell::Int(if unsigned { i64::from(r.u8()?) } else { i64::from(r.u8()? as i8) }),
                ty::SHORT | ty::YEAR => Cell::Int(if unsigned { i64::from(r.u16()?) } else { i64::from(r.u16()? as i16) }),
                ty::LONG | ty::INT24 => Cell::Int(if unsigned { i64::from(r.u32()?) } else { i64::from(r.u32()? as i32) }),
                ty::LONGLONG => {
                    let v = r.u64()?;
                    if unsigned && v > i64::MAX as u64 {
                        formatted.push((i, extra.len(), 0));
                        write!(extra, "{v}").ok();
                        Cell::Null
                    } else {
                        Cell::Int(v as i64)
                    }
                }
                ty::FLOAT => Cell::Float(format!("{}", f32::from_bits(r.u32()?)).parse().unwrap_or(0.0)),
                ty::DOUBLE => Cell::Float(f64::from_bits(r.u64()?)),
                ty::DATE | ty::DATETIME | ty::TIMESTAMP => {
                    formatted.push((i, extra.len(), 0));
                    format_datetime(&mut r, col, &mut extra)?;
                    Cell::Null
                }
                ty::TIME => {
                    formatted.push((i, extra.len(), 0));
                    format_time(&mut r, col, &mut extra)?;
                    Cell::Null
                }
                _ => {
                    let bytes = r.lenenc_str()?;
                    let start = payload.len() - r.remaining() - bytes.len();
                    Cell::Bytes { start, len: bytes.len() }
                }
            };
            cells.push(cell);
        }
    }
    let mut buf = payload;
    let base = buf.len();
    buf.extend_from_slice(extra.as_bytes());
    for (k, &(index, offset, _)) in formatted.iter().enumerate() {
        let end = formatted.get(k + 1).map_or(extra.len(), |next| next.1);
        cells[index] = Cell::Bytes { start: base + offset, len: end - offset };
    }
    Ok(Row { buf, cells })
}

fn fraction(out: &mut String, micros: u32, decimals: u8) {
    let digits = usize::from(if decimals <= 6 { decimals } else { 0 });
    if digits > 0 {
        let s = format!("{micros:06}");
        out.push('.');
        out.push_str(&s[..digits]);
    }
}

fn format_datetime(r: &mut Reader<'_>, col: &Column, out: &mut String) -> Result<()> {
    let len = r.u8()?;
    let (mut y, mut mo, mut d, mut h, mut mi, mut s, mut us) = (0u16, 0u8, 0u8, 0u8, 0u8, 0u8, 0u32);
    if len >= 4 {
        y = r.u16()?;
        mo = r.u8()?;
        d = r.u8()?;
    }
    if len >= 7 {
        h = r.u8()?;
        mi = r.u8()?;
        s = r.u8()?;
    }
    if len >= 11 {
        us = r.u32()?;
    }
    write!(out, "{y:04}-{mo:02}-{d:02}").ok();
    if col.ty != ty::DATE {
        write!(out, " {h:02}:{mi:02}:{s:02}").ok();
        fraction(out, us, col.decimals);
    }
    Ok(())
}

fn format_time(r: &mut Reader<'_>, col: &Column, out: &mut String) -> Result<()> {
    let len = r.u8()?;
    let (mut neg, mut days, mut h, mut mi, mut s, mut us) = (false, 0u32, 0u8, 0u8, 0u8, 0u32);
    if len >= 8 {
        neg = r.u8()? == 1;
        days = r.u32()?;
        h = r.u8()?;
        mi = r.u8()?;
        s = r.u8()?;
    }
    if len >= 12 {
        us = r.u32()?;
    }
    let hours = u64::from(days) * 24 + u64::from(h);
    write!(out, "{}{hours:02}:{mi:02}:{s:02}", if neg { "-" } else { "" }).ok();
    fraction(out, us, col.decimals);
    Ok(())
}

/// A bound statement parameter.
#[derive(Debug, Clone)]
pub enum Param<'a> {
    Null,
    Int(i64),
    Float(f64),
    Bytes(&'a [u8]),
    /// Sent before execution with COM_STMT_SEND_LONG_DATA; only the type goes into the execute packet.
    LongData,
}

/// Body of COM_STMT_EXECUTE after the command byte.
pub fn execute_body(stmt_id: u32, cursor: bool, params: &[Param<'_>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + params.len() * 12);
    out.put_u32(stmt_id);
    out.push(u8::from(cursor));
    out.put_u32(1);
    if params.is_empty() {
        return out;
    }
    let mut bitmap = vec![0u8; params.len().div_ceil(8)];
    for (i, p) in params.iter().enumerate() {
        if matches!(p, Param::Null) {
            bitmap[i / 8] |= 1 << (i % 8);
        }
    }
    out.extend_from_slice(&bitmap);
    out.push(1);
    for p in params {
        let t = match p {
            Param::Null => ty::NULL,
            Param::Int(_) => ty::LONGLONG,
            Param::Float(_) => ty::DOUBLE,
            Param::Bytes(_) => ty::VAR_STRING,
            Param::LongData => ty::BLOB,
        };
        out.push(t);
        out.push(0);
    }
    for p in params {
        match p {
            Param::Int(v) => out.put_u64(*v as u64),
            Param::Float(v) => out.put_u64(v.to_bits()),
            Param::Bytes(b) => out.put_lenenc_bytes(b),
            Param::Null | Param::LongData => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(t: u8, flags: u16, decimals: u8) -> Column {
        Column { schema: String::new(), table: String::new(), org_table: String::new(), name: "c".into(), org_name: "c".into(), charset: 33, length: 0, ty: t, flags, decimals }
    }

    #[test]
    fn text_rows_type_numbers() {
        let cols = [col(ty::LONGLONG, 0, 0), col(ty::DOUBLE, 0, 31), col(ty::VAR_STRING, 0, 0), col(ty::LONGLONG, crate::protocol::flag::UNSIGNED, 0)];
        let mut p = Vec::new();
        p.put_lenenc_bytes(b"-42");
        p.put_lenenc_bytes(b"1.5");
        p.push(0xFB);
        p.put_lenenc_bytes(b"18446744073709551615");
        let row = decode_text_row(p, &cols).unwrap();
        assert_eq!(row.cells[0], Cell::Int(-42));
        assert_eq!(row.cells[1], Cell::Float(1.5));
        assert_eq!(row.cells[2], Cell::Null);
        assert_eq!(row.bytes(row.cells[3]).unwrap(), b"18446744073709551615");
    }

    #[test]
    fn binary_rows_format_temporals() {
        let cols = [col(ty::DATETIME, 0, 3), col(ty::DATE, 0, 0), col(ty::TIME, 0, 0), col(ty::LONGLONG, crate::protocol::flag::UNSIGNED, 0), col(ty::VAR_STRING, 0, 0), col(ty::FLOAT, 0, 31), col(ty::LONG, 0, 0)];
        let mut p = vec![0x00, 0b0000_0000, 0b0000_0001];
        p.extend_from_slice(&[11, 0xEA, 0x07, 9, 29, 10, 28, 26]);
        p.extend_from_slice(&123_456u32.to_le_bytes());
        p.extend_from_slice(&[4, 0xEA, 0x07, 1, 2]);
        p.extend_from_slice(&[8, 1, 1, 0, 0, 0, 2, 3, 4]);
        p.extend_from_slice(&u64::MAX.to_le_bytes());
        p.put_lenenc_bytes(b"hi");
        p.extend_from_slice(&1.1f32.to_bits().to_le_bytes());
        let row = decode_binary_row(p, &cols).unwrap();
        let text = |i: usize| String::from_utf8(row.bytes(row.cells[i]).unwrap().to_vec()).unwrap();
        assert_eq!(text(0), "2026-09-29 10:28:26.123");
        assert_eq!(text(1), "2026-01-02");
        assert_eq!(text(2), "-26:03:04");
        assert_eq!(text(3), "18446744073709551615");
        assert_eq!(text(4), "hi");
        assert_eq!(row.cells[5], Cell::Float(1.1));
        assert_eq!(row.cells[6], Cell::Null);
    }

    #[test]
    fn execute_packet_layout() {
        let body = execute_body(7, true, &[Param::Int(1), Param::Null, Param::Bytes(b"x")]);
        assert_eq!(&body[..9], &[7, 0, 0, 0, 1, 1, 0, 0, 0]);
        assert_eq!(body[9], 0b010);
        assert_eq!(body[10], 1);
        assert_eq!(&body[11..17], &[ty::LONGLONG, 0, ty::NULL, 0, ty::VAR_STRING, 0]);
    }
}
