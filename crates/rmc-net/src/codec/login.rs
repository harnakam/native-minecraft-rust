//! Login packet codec for the frozen M1 packet set.

use crate::buffer::{PacketReader, PacketWriter};
use crate::codec::{split_packet_bytes, CodecError, EncodedPacket};

const MAX_LOGIN_BYTE_ARRAY_LEN: usize = 65_535;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginDisconnect {
    pub reason_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptionRequest {
    pub server_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginSuccess {
    pub uuid_string: String,
    pub username: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetCompression {
    pub threshold: i32,
}

impl SetCompression {
    pub fn validated_threshold(&self) -> Result<usize, CodecError> {
        usize::try_from(self.threshold)
            .map_err(|_| CodecError::InvalidCompressionThreshold(self.threshold))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginStart {
    pub username: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoginClientboundPacket {
    Disconnect(LoginDisconnect),
    EncryptionRequest(EncryptionRequest),
    LoginSuccess(LoginSuccess),
    SetCompression(SetCompression),
}

impl LoginClientboundPacket {
    pub fn decode_packet(packet_bytes: &[u8]) -> Result<Self, CodecError> {
        let (packet_id, body) = split_packet_bytes(packet_bytes)?;
        Self::decode_body(packet_id, body)
    }

    pub fn decode_body(packet_id: u8, body: &[u8]) -> Result<Self, CodecError> {
        let mut reader = PacketReader::new(body);

        let packet = match packet_id {
            0x00 => Self::Disconnect(LoginDisconnect {
                reason_json: reader.read_chat()?,
            }),
            0x01 => Self::EncryptionRequest(EncryptionRequest {
                server_id: reader.read_string(20)?,
                public_key: reader.read_byte_array(MAX_LOGIN_BYTE_ARRAY_LEN)?,
                verify_token: reader.read_byte_array(MAX_LOGIN_BYTE_ARRAY_LEN)?,
            }),
            0x02 => Self::LoginSuccess(LoginSuccess {
                uuid_string: reader.read_string(36)?,
                username: reader.read_string(16)?,
            }),
            0x03 => Self::SetCompression(SetCompression {
                threshold: reader.read_var_i32()?,
            }),
            other => return Err(CodecError::UnknownPacketId(other)),
        };

        reader.finish()?;
        Ok(packet)
    }

    pub fn encode_packet(&self) -> Result<EncodedPacket, CodecError> {
        let mut writer = PacketWriter::new();

        let packet_id = match self {
            Self::Disconnect(packet) => {
                writer.write_chat(&packet.reason_json)?;
                0x00
            }
            Self::EncryptionRequest(packet) => {
                writer.write_string(&packet.server_id, 20)?;
                writer.write_byte_array_bounded(&packet.public_key, MAX_LOGIN_BYTE_ARRAY_LEN)?;
                writer.write_byte_array_bounded(&packet.verify_token, MAX_LOGIN_BYTE_ARRAY_LEN)?;
                0x01
            }
            Self::LoginSuccess(packet) => {
                writer.write_string(&packet.uuid_string, 36)?;
                writer.write_string(&packet.username, 16)?;
                0x02
            }
            Self::SetCompression(packet) => {
                packet.validated_threshold()?;
                writer.write_var_i32(packet.threshold);
                0x03
            }
        };

        Ok(EncodedPacket::new(packet_id, writer.into_inner()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoginServerboundPacket {
    LoginStart(LoginStart),
    EncryptionResponse(EncryptionResponse),
}

impl LoginServerboundPacket {
    pub fn decode_packet(packet_bytes: &[u8]) -> Result<Self, CodecError> {
        let (packet_id, body) = split_packet_bytes(packet_bytes)?;
        Self::decode_body(packet_id, body)
    }

    pub fn decode_body(packet_id: u8, body: &[u8]) -> Result<Self, CodecError> {
        let mut reader = PacketReader::new(body);

        let packet = match packet_id {
            0x00 => Self::LoginStart(LoginStart {
                username: reader.read_string(16)?,
            }),
            0x01 => Self::EncryptionResponse(EncryptionResponse {
                shared_secret: reader.read_byte_array(MAX_LOGIN_BYTE_ARRAY_LEN)?,
                verify_token: reader.read_byte_array(MAX_LOGIN_BYTE_ARRAY_LEN)?,
            }),
            other => return Err(CodecError::UnknownPacketId(other)),
        };

        reader.finish()?;
        Ok(packet)
    }

    pub fn encode_packet(&self) -> Result<EncodedPacket, CodecError> {
        let mut writer = PacketWriter::new();

        let packet_id = match self {
            Self::LoginStart(packet) => {
                writer.write_string(&packet.username, 16)?;
                0x00
            }
            Self::EncryptionResponse(packet) => {
                writer.write_byte_array_bounded(&packet.shared_secret, MAX_LOGIN_BYTE_ARRAY_LEN)?;
                writer.write_byte_array_bounded(&packet.verify_token, MAX_LOGIN_BYTE_ARRAY_LEN)?;
                0x01
            }
        };

        Ok(EncodedPacket::new(packet_id, writer.into_inner()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EncryptionRequest, EncryptionResponse, LoginClientboundPacket, LoginDisconnect,
        LoginServerboundPacket, LoginStart, LoginSuccess, SetCompression,
    };

    #[test]
    fn roundtrips_clientbound_login_packets() {
        let cases = [
            LoginClientboundPacket::Disconnect(LoginDisconnect {
                reason_json: "{\"text\":\"Denied\"}".to_owned(),
            }),
            LoginClientboundPacket::EncryptionRequest(EncryptionRequest {
                server_id: "session".to_owned(),
                public_key: vec![1, 2, 3],
                verify_token: vec![4, 5, 6],
            }),
            LoginClientboundPacket::LoginSuccess(LoginSuccess {
                uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                username: "Player".to_owned(),
            }),
            LoginClientboundPacket::SetCompression(SetCompression { threshold: 256 }),
        ];

        for packet in cases {
            let encoded = packet.encode_packet().expect("packet should encode");
            let decoded = LoginClientboundPacket::decode_packet(&encoded.packet_bytes())
                .expect("packet should decode");
            assert_eq!(decoded, packet);
        }
    }

    #[test]
    fn roundtrips_serverbound_login_packets() {
        let cases = [
            LoginServerboundPacket::LoginStart(LoginStart {
                username: "Player".to_owned(),
            }),
            LoginServerboundPacket::EncryptionResponse(EncryptionResponse {
                shared_secret: vec![7, 8, 9],
                verify_token: vec![10, 11],
            }),
        ];

        for packet in cases {
            let encoded = packet.encode_packet().expect("packet should encode");
            let decoded = LoginServerboundPacket::decode_packet(&encoded.packet_bytes())
                .expect("packet should decode");
            assert_eq!(decoded, packet);
        }
    }
}
