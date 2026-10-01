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

/// Encode an unnamed root tag for protocol item NBT. Existing untouched wire bytes need not be reencoded.
pub fn encode(tag: &Tag) -> Result<Vec<u8>, NbtError> {
    let mut output = vec![tag_kind(tag), 0, 0];
    let mut budget = 131_072;
    encode_payload(tag, &mut output, 0, &mut budget)?;
    Ok(output)
}
fn tag_kind(tag: &Tag) -> u8 {
    match tag {
        Tag::Byte(_) => 1,
        Tag::Short(_) => 2,
        Tag::Int(_) => 3,
        Tag::Long(_) => 4,
        Tag::Float(_) => 5,
        Tag::Double(_) => 6,
        Tag::Bytes(_) => 7,
        Tag::String(_) => 8,
        Tag::List { .. } => 9,
        Tag::Compound(_) => 10,
        Tag::Ints(_) => 11,
        Tag::Longs(_) => 12,
    }
}
fn encode_string(units: &[u16], output: &mut Vec<u8>) -> Result<(), NbtError> {
    if units.len() > 65535 {
        return Err(NbtError::Limit);
    }
    let mut bytes = Vec::new();
    for &unit in units {
        if (1..=127).contains(&unit) {
            bytes.push(unit as u8);
        } else if unit < 2048 {
            bytes.extend_from_slice(&[(0xc0 | unit >> 6) as u8, (0x80 | unit & 63) as u8]);
        } else {
            bytes.extend_from_slice(&[
                (0xe0 | unit >> 12) as u8,
                (0x80 | (unit >> 6) & 63) as u8,
                (0x80 | unit & 63) as u8,
            ]);
        }
        if bytes.len() > 65535 {
            return Err(NbtError::Limit);
        }
    }
    output.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    output.extend_from_slice(&bytes);
    Ok(())
}
fn encode_payload(
    tag: &Tag,
    output: &mut Vec<u8>,
    depth: usize,
    budget: &mut usize,
) -> Result<(), NbtError> {
    if depth > 512 || *budget == 0 || output.len() > 2_097_152 {
        return Err(NbtError::Limit);
    }
    *budget -= 1;
    match tag {
        Tag::Byte(v) => output.push(*v as u8),
        Tag::Short(v) => output.extend_from_slice(&v.to_be_bytes()),
        Tag::Int(v) => output.extend_from_slice(&v.to_be_bytes()),
        Tag::Long(v) => output.extend_from_slice(&v.to_be_bytes()),
        Tag::Float(v) => output.extend_from_slice(&v.to_be_bytes()),
        Tag::Double(v) => output.extend_from_slice(&v.to_be_bytes()),
        Tag::Bytes(v) => {
            if v.len() > 2_097_152 {
                return Err(NbtError::Limit);
            }
            output.extend_from_slice(&(v.len() as i32).to_be_bytes());
            output.extend_from_slice(v);
        }
        Tag::String(v) => encode_string(v, output)?,
        Tag::List { kind, values } => {
            if values.len() > *budget {
                return Err(NbtError::Limit);
            }
            if *kind > 12
                || (!values.is_empty()
                    && (*kind == 0 || values.iter().any(|v| tag_kind(v) != *kind)))
            {
                return Err(BufferError::InvalidNbtTag(*kind).into());
            }
            output.push(*kind);
            output.extend_from_slice(&(values.len() as i32).to_be_bytes());
            for v in values {
                encode_payload(v, output, depth + 1, budget)?;
            }
        }
        Tag::Compound(v) => {
            if v.len() > *budget {
                return Err(NbtError::Limit);
            }
            for (name, value) in v {
                output.push(tag_kind(value));
                encode_string(name, output)?;
                encode_payload(value, output, depth + 1, budget)?;
            }
            output.push(0);
        }
        Tag::Ints(v) => {
            if v.len() > *budget {
                return Err(NbtError::Limit);
            }
            *budget -= v.len();
            output.extend_from_slice(&(v.len() as i32).to_be_bytes());
            for item in v {
                output.extend_from_slice(&item.to_be_bytes());
            }
        }
        Tag::Longs(v) => {
            if v.len() > *budget {
                return Err(NbtError::Limit);
            }
            *budget -= v.len();
            output.extend_from_slice(&(v.len() as i32).to_be_bytes());
            for item in v {
                output.extend_from_slice(&item.to_be_bytes());
            }
        }
    }
    if output.len() > 2_097_152 {
        return Err(NbtError::Limit);
    }
    Ok(())
}

#[cfg(test)]
mod encoding_tests {
    use super::*;
    #[test]
    fn modified_utf8_preserves_null_surrogates_and_japanese() {
        let tag = Tag::String(vec![0, 0x41, 0xd83d, 0xde00, 0x65e5]);
        let bytes = encode(&tag).unwrap();
        assert_eq!(
            &bytes[3..],
            &[0, 12, 0xc0, 0x80, 0x41, 0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80, 0xe6, 0x97, 0xa5]
        );
        assert_eq!(parse(&bytes).unwrap(), tag);
    }
    #[test]
    fn compound_roundtrip_retains_numeric_arrays_and_lists() {
        let values = vec![
            Tag::Byte(-1),
            Tag::Short(-200),
            Tag::Int(300000),
            Tag::Long(i64::MIN),
            Tag::Float(1.25),
            Tag::Double(-3.5),
            Tag::Bytes(vec![0, 255]),
            Tag::Ints(vec![-1, 2]),
            Tag::Longs(vec![i64::MAX]),
            Tag::List {
                kind: 8,
                values: vec![Tag::String(vec![0xd800])],
            },
            Tag::List {
                kind: 0,
                values: vec![],
            },
        ];
        let tag = Tag::Compound(
            values
                .into_iter()
                .enumerate()
                .map(|(i, v)| (i.to_string().encode_utf16().collect(), v))
                .collect(),
        );
        assert_eq!(parse(&encode(&tag).unwrap()).unwrap(), tag);
    }
    #[test]
    fn invalid_lists_and_oversized_values_are_rejected() {
        assert!(encode(&Tag::List {
            kind: 1,
            values: vec![Tag::Int(1)]
        })
        .is_err());
        assert!(encode(&Tag::String(vec![0x800; 21846])).is_err());
        assert!(encode(&Tag::Bytes(vec![0; 2097152])).is_err());
        assert!(encode(&Tag::Ints(vec![0; 131072])).is_err());
    }
}
