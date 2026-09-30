//! Handshake packet codec.

use crate::buffer::{PacketReader, PacketWriter};
use crate::codec::{split_packet_bytes, CodecError, EncodedPacket};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandshakeNextState {
    Status = 1,
    Login = 2,
}

impl TryFrom<i32> for HandshakeNextState {
    type Error = CodecError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Status),
            2 => Ok(Self::Login),
            _ => Err(CodecError::InvalidNextState(value)),
        }
    }
}

impl From<HandshakeNextState> for i32 {
    fn from(value: HandshakeNextState) -> Self {
        value as i32
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandshakeRequest {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: HandshakeNextState,
}

impl HandshakeRequest {
    pub const PACKET_ID: u8 = 0x00;

    pub fn decode_packet(packet_bytes: &[u8]) -> Result<Self, CodecError> {
        let (packet_id, body) = split_packet_bytes(packet_bytes)?;

        if packet_id != Self::PACKET_ID {
            return Err(CodecError::UnexpectedPacketId {
                expected: Self::PACKET_ID,
                actual: packet_id,
            });
        }

        Self::decode_body(body)
    }

    pub fn decode_body(body: &[u8]) -> Result<Self, CodecError> {
        let mut reader = PacketReader::new(body);
        let protocol_version = reader.read_var_i32()?;
        let server_address = reader.read_string(255)?;
        let server_port = reader.read_u16()?;
        let next_state = HandshakeNextState::try_from(reader.read_var_i32()?)?;
        reader.finish()?;

        Ok(Self {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }

    pub fn encode_packet(&self) -> Result<EncodedPacket, CodecError> {
        Ok(EncodedPacket::new(Self::PACKET_ID, self.encode_body()?))
    }

    pub fn encode_body(&self) -> Result<Vec<u8>, CodecError> {
        let mut writer = PacketWriter::new();
        writer.write_var_i32(self.protocol_version);
        writer.write_string(&self.server_address, 255)?;
        writer.write_u16(self.server_port);
        writer.write_var_i32(self.next_state.into());
        Ok(writer.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::{HandshakeNextState, HandshakeRequest};

    #[test]
    fn roundtrips_handshake_packet() {
        let packet = HandshakeRequest {
            protocol_version: 47,
            server_address: "hypixel.net".to_owned(),
            server_port: 25565,
            next_state: HandshakeNextState::Login,
        };

        let encoded = packet.encode_packet().expect("packet should encode");
        let decoded =
            HandshakeRequest::decode_packet(&encoded.packet_bytes()).expect("packet should decode");

        assert_eq!(decoded, packet);
    }
}
