//! S44PacketWorldBorder action-specific protocol 47 fields.
use crate::buffer::{PacketReader, PacketWriter};
use crate::codec::CodecError;

#[derive(Clone, Debug, PartialEq)]
pub enum WorldBorderPacket {
    SetSize {
        diameter: f64,
    },
    LerpSize {
        from: f64,
        to: f64,
        milliseconds: i64,
    },
    SetCenter {
        x: f64,
        z: f64,
    },
    Initialize {
        x: f64,
        z: f64,
        from: f64,
        to: f64,
        milliseconds: i64,
        size: i32,
        warning_distance: i32,
        warning_time: i32,
    },
    SetWarningTime(i32),
    SetWarningBlocks(i32),
}

impl WorldBorderPacket {
    pub(super) fn read(reader: &mut PacketReader<'_>) -> Result<Self, CodecError> {
        let action = reader.read_var_i32()?;
        Ok(match action {
            0 => Self::SetSize {
                diameter: reader.read_f64()?,
            },
            1 => Self::LerpSize {
                from: reader.read_f64()?,
                to: reader.read_f64()?,
                milliseconds: reader.read_var_i64()?,
            },
            2 => Self::SetCenter {
                x: reader.read_f64()?,
                z: reader.read_f64()?,
            },
            3 => Self::Initialize {
                x: reader.read_f64()?,
                z: reader.read_f64()?,
                from: reader.read_f64()?,
                to: reader.read_f64()?,
                milliseconds: reader.read_var_i64()?,
                size: reader.read_var_i32()?,
                warning_distance: reader.read_var_i32()?,
                warning_time: reader.read_var_i32()?,
            },
            4 => Self::SetWarningTime(reader.read_var_i32()?),
            5 => Self::SetWarningBlocks(reader.read_var_i32()?),
            actual => {
                return Err(CodecError::InvalidEnumValue {
                    enum_name: "WorldBorderAction",
                    actual,
                })
            }
        })
    }

    pub(super) fn write(&self, writer: &mut PacketWriter) {
        match *self {
            Self::SetSize { diameter } => {
                writer.write_var_i32(0);
                writer.write_f64(diameter);
            }
            Self::LerpSize {
                from,
                to,
                milliseconds,
            } => {
                writer.write_var_i32(1);
                writer.write_f64(from);
                writer.write_f64(to);
                writer.write_var_i64(milliseconds);
            }
            Self::SetCenter { x, z } => {
                writer.write_var_i32(2);
                writer.write_f64(x);
                writer.write_f64(z);
            }
            Self::Initialize {
                x,
                z,
                from,
                to,
                milliseconds,
                size,
                warning_distance,
                warning_time,
            } => {
                writer.write_var_i32(3);
                writer.write_f64(x);
                writer.write_f64(z);
                writer.write_f64(from);
                writer.write_f64(to);
                writer.write_var_i64(milliseconds);
                writer.write_var_i32(size);
                writer.write_var_i32(warning_distance);
                writer.write_var_i32(warning_time);
            }
            Self::SetWarningTime(value) => {
                writer.write_var_i32(4);
                writer.write_var_i32(value);
            }
            Self::SetWarningBlocks(value) => {
                writer.write_var_i32(5);
                writer.write_var_i32(value);
            }
        }
    }
}
