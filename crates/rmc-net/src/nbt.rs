//! Bounded, uncompressed protocol NBT inspection. Wire bytes remain the round-trip authority.
use crate::buffer::{BufferError, PacketReader};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Tag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Bytes(Vec<u8>),
    String(Vec<u16>),
    List { kind: u8, values: Vec<Tag> },
    Compound(BTreeMap<Vec<u16>, Tag>),
    Ints(Vec<i32>),
    Longs(Vec<i64>),
}
impl Tag {
    pub fn get(&self, name: &str) -> Option<&Tag> {
        if let Self::Compound(values) = self {
            values.get(&name.encode_utf16().collect::<Vec<_>>())
        } else {
            None
        }
    }
    pub fn short(&self) -> Option<i16> {
        Some(match self {
            Self::Byte(value) => *value as i16,
            Self::Short(value) => *value,
            Self::Int(value) => *value as i16,
            Self::Long(value) => *value as i16,
            Self::Float(value) => *value as i32 as i16,
            Self::Double(value) => *value as i32 as i16,
            _ => return None,
        })
    }
    pub fn string_equals(&self, value: &str) -> bool {
        matches!(self,Self::String(units) if units.iter().copied().eq(value.encode_utf16()))
    }
    pub fn list(&self) -> Option<&[Tag]> {
        if let Self::List { values, .. } = self {
            Some(values)
        } else {
            None
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum NbtError {
    Buffer(BufferError),
    Limit,
    InvalidString,
}
impl From<BufferError> for NbtError {
    fn from(value: BufferError) -> Self {
        Self::Buffer(value)
    }
}

pub fn parse(bytes: &[u8]) -> Result<Tag, NbtError> {
    if bytes.len() > 2_097_152 {
        return Err(NbtError::Limit);
    }
    let mut reader = PacketReader::new(bytes);
    let kind = reader.read_u8()?;
    let _name = string(&mut reader)?;
    let mut budget = 131_072;
    let tag = payload(&mut reader, kind, 0, &mut budget)?;
    reader.finish()?;
    Ok(tag)
}

fn string(reader: &mut PacketReader<'_>) -> Result<Vec<u16>, NbtError> {
    let length = reader.read_u16()? as usize;
    let bytes = reader.read_bytes(length)?;
    let mut units = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        let (count, value) = if first < 0x80 {
            (1, first as u16)
        } else if first & 0xe0 == 0xc0 {
            let second = *bytes.get(index + 1).ok_or(NbtError::InvalidString)?;
            if second & 0xc0 != 0x80 {
                return Err(NbtError::InvalidString);
            }
            (2, ((first & 31) as u16) << 6 | (second & 63) as u16)
        } else if first & 0xf0 == 0xe0 {
            let second = *bytes.get(index + 1).ok_or(NbtError::InvalidString)?;
            let third = *bytes.get(index + 2).ok_or(NbtError::InvalidString)?;
            if second & 0xc0 != 0x80 || third & 0xc0 != 0x80 {
                return Err(NbtError::InvalidString);
            }
            (
                3,
                ((first & 15) as u16) << 12 | ((second & 63) as u16) << 6 | (third & 63) as u16,
            )
        } else {
            return Err(NbtError::InvalidString);
        };
        units.push(value);
        index += count;
    }
    Ok(units)
}
fn count(reader: &mut PacketReader<'_>, budget: usize) -> Result<usize, NbtError> {
    let count = reader.read_i32()?;
    if count < 0 {
        return Err(BufferError::NegativeLength(count).into());
    }
    if count as usize > budget {
        return Err(NbtError::Limit);
    }
    Ok(count as usize)
}
fn payload(
    reader: &mut PacketReader<'_>,
    kind: u8,
    depth: usize,
    budget: &mut usize,
) -> Result<Tag, NbtError> {
    if depth > 512 || *budget == 0 {
        return Err(NbtError::Limit);
    }
    *budget -= 1;
    Ok(match kind {
        1 => Tag::Byte(reader.read_i8()?),
        2 => Tag::Short(reader.read_i16()?),
        3 => Tag::Int(reader.read_i32()?),
        4 => Tag::Long(reader.read_i64()?),
        5 => Tag::Float(reader.read_f32()?),
        6 => Tag::Double(reader.read_f64()?),
        7 => {
            let size = count(reader, 2_097_152)?;
            Tag::Bytes(reader.read_bytes(size)?)
        }
        8 => Tag::String(string(reader)?),
        9 => {
            let child = reader.read_u8()?;
            let size = count(reader, *budget)?;
            if size > 0 && child == 0 {
                return Err(BufferError::InvalidNbtTag(child).into());
            }
            let mut list = Vec::new();
            for _ in 0..size {
                list.push(payload(reader, child, depth + 1, budget)?);
            }
            Tag::List {
                kind: child,
                values: list,
            }
        }
        10 => {
            let mut values = BTreeMap::new();
            loop {
                let child = reader.read_u8()?;
                if child == 0 {
                    break;
                }
                let name = string(reader)?;
                values.insert(name, payload(reader, child, depth + 1, budget)?);
            }
            Tag::Compound(values)
        }
        11 => {
            let size = count(reader, *budget)?;
            let mut values = Vec::new();
            for _ in 0..size {
                values.push(reader.read_i32()?);
            }
            *budget -= size;
            Tag::Ints(values)
        }
        12 => {
            let size = count(reader, *budget)?;
            let mut values = Vec::new();
            for _ in 0..size {
                values.push(reader.read_i64()?);
            }
            *budget -= size;
            Tag::Longs(values)
        }
        other => return Err(BufferError::InvalidNbtTag(other).into()),
    })
}
