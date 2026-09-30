//! Minimal PacketBuffer-compatible read/write helpers for protocol 47 packets.

use crate::varint::{decode_i32, encode_i32, VarIntError};
use std::str;

pub const MAX_MINECRAFT_STRING_BYTES: usize = 32_767;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BufferError {
    UnexpectedEof { needed: usize, remaining: usize },
    VarInt(VarIntError),
    NegativeLength(i32),
    EncodedStringTooLong { max_bytes: usize, actual: usize },
    DecodedStringTooLong { max_chars: usize, actual: usize },
    ByteArrayTooLong { max_len: usize, actual: usize },
    InvalidDataWatcherType(u8),
    InvalidNbtTag(u8),
    InvalidUtf8,
    TrailingBytes { remaining: usize },
}

impl From<VarIntError> for BufferError {
    fn from(value: VarIntError) -> Self {
        Self::VarInt(value)
    }
}

pub struct PacketReader<'a> {
    input: &'a [u8],
    offset: usize,
}

impl<'a> PacketReader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, offset: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.input.len().saturating_sub(self.offset)
    }

    pub fn read_u8(&mut self) -> Result<u8, BufferError> {
        self.ensure_remaining(1)?;
        let value = self.input[self.offset];
        self.offset += 1;
        Ok(value)
    }

    pub fn read_i8(&mut self) -> Result<i8, BufferError> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_bool(&mut self) -> Result<bool, BufferError> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_i16(&mut self) -> Result<i16, BufferError> {
        self.ensure_remaining(2)?;
        let bytes = [self.input[self.offset], self.input[self.offset + 1]];
        self.offset += 2;
        Ok(i16::from_be_bytes(bytes))
    }

    pub fn read_u16(&mut self) -> Result<u16, BufferError> {
        self.ensure_remaining(2)?;
        let bytes = [self.input[self.offset], self.input[self.offset + 1]];
        self.offset += 2;
        Ok(u16::from_be_bytes(bytes))
    }

    pub fn read_i32(&mut self) -> Result<i32, BufferError> {
        self.ensure_remaining(4)?;
        let bytes = [
            self.input[self.offset],
            self.input[self.offset + 1],
            self.input[self.offset + 2],
            self.input[self.offset + 3],
        ];
        self.offset += 4;
        Ok(i32::from_be_bytes(bytes))
    }

    pub fn read_i64(&mut self) -> Result<i64, BufferError> {
        self.ensure_remaining(8)?;
        let bytes = [
            self.input[self.offset],
            self.input[self.offset + 1],
            self.input[self.offset + 2],
            self.input[self.offset + 3],
            self.input[self.offset + 4],
            self.input[self.offset + 5],
            self.input[self.offset + 6],
            self.input[self.offset + 7],
        ];
        self.offset += 8;
        Ok(i64::from_be_bytes(bytes))
    }

    pub fn read_f32(&mut self) -> Result<f32, BufferError> {
        self.ensure_remaining(4)?;
        let bytes = [
            self.input[self.offset],
            self.input[self.offset + 1],
            self.input[self.offset + 2],
            self.input[self.offset + 3],
        ];
        self.offset += 4;
        Ok(f32::from_be_bytes(bytes))
    }

    pub fn read_f64(&mut self) -> Result<f64, BufferError> {
        self.ensure_remaining(8)?;
        let bytes = [
            self.input[self.offset],
            self.input[self.offset + 1],
            self.input[self.offset + 2],
            self.input[self.offset + 3],
            self.input[self.offset + 4],
            self.input[self.offset + 5],
            self.input[self.offset + 6],
            self.input[self.offset + 7],
        ];
        self.offset += 8;
        Ok(f64::from_be_bytes(bytes))
    }

    pub fn read_var_i32(&mut self) -> Result<i32, BufferError> {
        let (value, read) = decode_i32(&self.input[self.offset..])?;
        self.offset += read;
        Ok(value)
    }

    pub fn read_string(&mut self, max_chars: usize) -> Result<String, BufferError> {
        let len = self.read_var_i32()?;

        if len < 0 {
            return Err(BufferError::NegativeLength(len));
        }

        let len = len as usize;
        let max_bytes = max_chars.saturating_mul(4);

        if len > max_bytes {
            return Err(BufferError::EncodedStringTooLong {
                max_bytes,
                actual: len,
            });
        }

        self.ensure_remaining(len)?;
        let bytes = &self.input[self.offset..self.offset + len];
        self.offset += len;

        let string = str::from_utf8(bytes)
            .map_err(|_| BufferError::InvalidUtf8)?
            .to_owned();

        if string.chars().count() > max_chars {
            return Err(BufferError::DecodedStringTooLong {
                max_chars,
                actual: string.chars().count(),
            });
        }

        Ok(string)
    }

    pub fn read_chat(&mut self) -> Result<String, BufferError> {
        self.read_string(32_767)
    }

    pub fn read_byte_array(&mut self, max_len: usize) -> Result<Vec<u8>, BufferError> {
        let len = self.read_var_i32()?;

        if len < 0 {
            return Err(BufferError::NegativeLength(len));
        }

        let len = len as usize;

        if len > max_len {
            return Err(BufferError::ByteArrayTooLong {
                max_len,
                actual: len,
            });
        }

        self.ensure_remaining(len)?;
        let bytes = self.input[self.offset..self.offset + len].to_vec();
        self.offset += len;
        Ok(bytes)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>, BufferError> {
        self.ensure_remaining(len)?;
        let bytes = self.input[self.offset..self.offset + len].to_vec();
        self.offset += len;
        Ok(bytes)
    }

    pub fn read_remaining_bytes(&mut self, max_len: usize) -> Result<Vec<u8>, BufferError> {
        let len = self.remaining();

        if len > max_len {
            return Err(BufferError::ByteArrayTooLong {
                max_len,
                actual: len,
            });
        }

        let bytes = self.input[self.offset..].to_vec();
        self.offset = self.input.len();
        Ok(bytes)
    }

    pub fn read_uuid_bytes(&mut self) -> Result<[u8; 16], BufferError> {
        self.ensure_remaining(16)?;
        let mut uuid = [0; 16];
        uuid.copy_from_slice(&self.input[self.offset..self.offset + 16]);
        self.offset += 16;
        Ok(uuid)
    }

    pub fn read_nbt_blob(&mut self) -> Result<Option<Vec<u8>>, BufferError> {
        self.ensure_remaining(1)?;

        if self.input[self.offset] == 0 {
            self.offset += 1;
            return Ok(None);
        }

        let start = self.offset;
        let tag_id = self.read_u8()?;
        self.skip_nbt_string()?;
        self.skip_nbt_payload(tag_id)?;
        Ok(Some(self.input[start..self.offset].to_vec()))
    }

    pub fn read_data_watcher_blob(&mut self) -> Result<Vec<u8>, BufferError> {
        let start = self.offset;

        loop {
            let header = self.read_u8()?;

            if header == 0x7f {
                break;
            }

            match (header & 0xe0) >> 5 {
                0 => {
                    self.ensure_remaining(1)?;
                    self.offset += 1;
                }
                1 => {
                    self.ensure_remaining(2)?;
                    self.offset += 2;
                }
                2 | 3 => {
                    self.ensure_remaining(4)?;
                    self.offset += 4;
                }
                4 => {
                    let _ = self.read_string(32_767)?;
                }
                5 => {
                    self.skip_slot()?;
                }
                6 | 7 => {
                    self.ensure_remaining(12)?;
                    self.offset += 12;
                }
                other => return Err(BufferError::InvalidDataWatcherType(other)),
            }
        }

        Ok(self.input[start..self.offset].to_vec())
    }

    pub fn finish(self) -> Result<(), BufferError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(BufferError::TrailingBytes {
                remaining: self.remaining(),
            })
        }
    }

    fn ensure_remaining(&self, needed: usize) -> Result<(), BufferError> {
        let remaining = self.remaining();

        if remaining < needed {
            Err(BufferError::UnexpectedEof { needed, remaining })
        } else {
            Ok(())
        }
    }

    fn skip_nbt_string(&mut self) -> Result<(), BufferError> {
        let len = self.read_u16()? as usize;
        self.ensure_remaining(len)?;
        self.offset += len;
        Ok(())
    }

    fn skip_slot(&mut self) -> Result<(), BufferError> {
        let item_id = self.read_i16()?;

        if item_id < 0 {
            return Ok(());
        }

        self.ensure_remaining(3)?;
        self.offset += 3;
        self.read_nbt_blob()?;
        Ok(())
    }

    fn skip_nbt_payload(&mut self, tag_id: u8) -> Result<(), BufferError> {
        match tag_id {
            0 => Ok(()),
            1 => {
                self.ensure_remaining(1)?;
                self.offset += 1;
                Ok(())
            }
            2 => {
                self.ensure_remaining(2)?;
                self.offset += 2;
                Ok(())
            }
            3 | 5 => {
                self.ensure_remaining(4)?;
                self.offset += 4;
                Ok(())
            }
            4 | 6 => {
                self.ensure_remaining(8)?;
                self.offset += 8;
                Ok(())
            }
            7 => self.skip_nbt_array(1),
            8 => self.skip_nbt_string(),
            9 => {
                let child_tag_id = self.read_u8()?;
                let len = self.read_i32()?;

                if len < 0 {
                    return Err(BufferError::NegativeLength(len));
                }

                for _ in 0..len as usize {
                    self.skip_nbt_payload(child_tag_id)?;
                }

                Ok(())
            }
            10 => {
                loop {
                    let child_tag_id = self.read_u8()?;

                    if child_tag_id == 0 {
                        break;
                    }

                    self.skip_nbt_string()?;
                    self.skip_nbt_payload(child_tag_id)?;
                }

                Ok(())
            }
            11 => self.skip_nbt_array(4),
            12 => self.skip_nbt_array(8),
            other => Err(BufferError::InvalidNbtTag(other)),
        }
    }

    fn skip_nbt_array(&mut self, element_size: usize) -> Result<(), BufferError> {
        let len = self.read_i32()?;

        if len < 0 {
            return Err(BufferError::NegativeLength(len));
        }

        let len = len as usize;
        let byte_len = len
            .checked_mul(element_size)
            .ok_or(BufferError::ByteArrayTooLong {
                max_len: usize::MAX / element_size,
                actual: len,
            })?;
        self.ensure_remaining(byte_len)?;
        self.offset += byte_len;
        Ok(())
    }
}

#[derive(Default)]
pub struct PacketWriter {
    output: Vec<u8>,
}

impl PacketWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write_u8(&mut self, value: u8) {
        self.output.push(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.output.push(value as u8);
    }

    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(u8::from(value));
    }

    pub fn write_i16(&mut self, value: i16) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u16(&mut self, value: u16) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i32(&mut self, value: i32) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_i64(&mut self, value: i64) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_f32(&mut self, value: f32) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_f64(&mut self, value: f64) {
        self.output.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_var_i32(&mut self, value: i32) {
        encode_i32(value, &mut self.output);
    }

    pub fn write_string(&mut self, value: &str, max_chars: usize) -> Result<(), BufferError> {
        let actual_chars = value.chars().count();

        if actual_chars > max_chars {
            return Err(BufferError::DecodedStringTooLong {
                max_chars,
                actual: actual_chars,
            });
        }

        let encoded = value.as_bytes();

        if encoded.len() > MAX_MINECRAFT_STRING_BYTES {
            return Err(BufferError::EncodedStringTooLong {
                max_bytes: MAX_MINECRAFT_STRING_BYTES,
                actual: encoded.len(),
            });
        }

        let max_bytes = max_chars.saturating_mul(4);

        if encoded.len() > max_bytes {
            return Err(BufferError::EncodedStringTooLong {
                max_bytes,
                actual: encoded.len(),
            });
        }

        self.write_var_i32(encoded.len() as i32);
        self.output.extend_from_slice(encoded);
        Ok(())
    }

    pub fn write_chat(&mut self, value: &str) -> Result<(), BufferError> {
        self.write_string(value, 32_767)
    }

    pub fn write_byte_array(&mut self, value: &[u8]) -> Result<(), BufferError> {
        self.write_byte_array_bounded(value, usize::MAX)
    }

    pub fn write_byte_array_bounded(
        &mut self,
        value: &[u8],
        max_len: usize,
    ) -> Result<(), BufferError> {
        if value.len() > max_len {
            return Err(BufferError::ByteArrayTooLong {
                max_len,
                actual: value.len(),
            });
        }

        self.write_var_i32(value.len() as i32);
        self.output.extend_from_slice(value);
        Ok(())
    }

    pub fn write_bytes(&mut self, value: &[u8]) {
        self.output.extend_from_slice(value);
    }

    pub fn write_nbt_blob(&mut self, value: Option<&[u8]>) {
        match value {
            Some(value) => self.output.extend_from_slice(value),
            None => self.write_u8(0),
        }
    }

    pub fn write_uuid_bytes(&mut self, value: &[u8; 16]) {
        self.output.extend_from_slice(value);
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.output
    }
}

#[cfg(test)]
mod tests {
    use super::{BufferError, PacketReader, PacketWriter};

    #[test]
    fn string_roundtrip() {
        let mut writer = PacketWriter::new();
        writer
            .write_string("hypixel.net", 255)
            .expect("string should encode");

        let bytes = writer.into_inner();
        let mut reader = PacketReader::new(&bytes);
        let decoded = reader.read_string(255).expect("string should decode");

        assert_eq!(decoded, "hypixel.net");
        reader.finish().expect("reader should be exhausted");
    }

    #[test]
    fn reports_trailing_bytes() {
        let reader = PacketReader::new(&[0x00]);
        assert_eq!(
            reader.finish(),
            Err(BufferError::TrailingBytes { remaining: 1 })
        );
    }

    #[test]
    fn nbt_blob_roundtrip_preserves_raw_bytes() {
        let raw_nbt = [
            10, 0, 0, 8, 0, 3, b'f', b'o', b'o', 0, 3, b'b', b'a', b'r', 0,
        ];
        let mut writer = PacketWriter::new();
        writer.write_nbt_blob(Some(&raw_nbt));

        let bytes = writer.into_inner();
        let mut reader = PacketReader::new(&bytes);
        let decoded = reader.read_nbt_blob().expect("nbt blob should decode");

        assert_eq!(decoded, Some(raw_nbt.to_vec()));
        reader.finish().expect("reader should be exhausted");
    }

    #[test]
    fn decodes_null_nbt_blob() {
        let mut reader = PacketReader::new(&[0]);
        assert_eq!(
            reader.read_nbt_blob().expect("null nbt should decode"),
            None
        );
    }

    #[test]
    fn uuid_bytes_roundtrip() {
        let uuid = [
            0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
            0x77, 0x88,
        ];
        let mut writer = PacketWriter::new();
        writer.write_uuid_bytes(&uuid);

        let bytes = writer.into_inner();
        let mut reader = PacketReader::new(&bytes);
        assert_eq!(reader.read_uuid_bytes().expect("uuid should decode"), uuid);
        reader.finish().expect("reader should be exhausted");
    }
}
