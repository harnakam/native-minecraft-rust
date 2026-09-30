//! Typed protocol 47 DataWatcher values; list order is preserved for updates.
use super::{read_slot, Slot};
use crate::buffer::PacketReader;
use crate::codec::CodecError;
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Byte(i8),
    Short(i16),
    Int(i32),
    Float(f32),
    String(String),
    Item(Slot),
    Position([i32; 3]),
    Rotation([f32; 3]),
}
pub fn decode(bytes: &[u8]) -> Result<Vec<(u8, Value)>, CodecError> {
    let mut reader = PacketReader::new(bytes);
    let mut values = Vec::new();
    loop {
        let header = reader.read_u8()?;
        if header == 127 {
            break;
        }
        let value = match header >> 5 {
            0 => Value::Byte(reader.read_i8()?),
            1 => Value::Short(reader.read_i16()?),
            2 => Value::Int(reader.read_i32()?),
            3 => Value::Float(reader.read_f32()?),
            4 => Value::String(reader.read_string(32767)?),
            5 => Value::Item(read_slot(&mut reader)?),
            6 => Value::Position([reader.read_i32()?, reader.read_i32()?, reader.read_i32()?]),
            7 => Value::Rotation([reader.read_f32()?, reader.read_f32()?, reader.read_f32()?]),
            _ => unreachable!(),
        };
        values.push((header & 31, value));
    }
    reader.finish()?;
    Ok(values)
}
