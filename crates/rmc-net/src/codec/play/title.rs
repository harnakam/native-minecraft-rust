//! S45PacketTitle action layouts from protocol 47.
use crate::buffer::{PacketReader, PacketWriter};
use crate::codec::CodecError;
#[derive(Clone, Debug, PartialEq)]
pub enum TitlePacket {
    Title(String),
    Subtitle(String),
    Times {
        fade_in: i32,
        stay: i32,
        fade_out: i32,
    },
    Clear,
    Reset,
}
impl TitlePacket {
    pub(super) fn read(reader: &mut PacketReader<'_>) -> Result<Self, CodecError> {
        Ok(match reader.read_var_i32()? {
            0 => Self::Title(reader.read_chat()?),
            1 => Self::Subtitle(reader.read_chat()?),
            2 => Self::Times {
                fade_in: reader.read_i32()?,
                stay: reader.read_i32()?,
                fade_out: reader.read_i32()?,
            },
            3 => Self::Clear,
            4 => Self::Reset,
            actual => {
                return Err(CodecError::InvalidEnumValue {
                    enum_name: "TitleAction",
                    actual,
                })
            }
        })
    }
    pub(super) fn write(&self, writer: &mut PacketWriter) -> Result<(), CodecError> {
        match self {
            Self::Title(text) => {
                writer.write_var_i32(0);
                writer.write_chat(text)?;
            }
            Self::Subtitle(text) => {
                writer.write_var_i32(1);
                writer.write_chat(text)?;
            }
            Self::Times {
                fade_in,
                stay,
                fade_out,
            } => {
                writer.write_var_i32(2);
                writer.write_i32(*fade_in);
                writer.write_i32(*stay);
                writer.write_i32(*fade_out);
            }
            Self::Clear => writer.write_var_i32(3),
            Self::Reset => writer.write_var_i32(4),
        }
        Ok(())
    }
}
