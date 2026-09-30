//! Packet-body codecs built on top of the frozen protocol contract.

use crate::buffer::BufferError;
use crate::varint::{decode_i32, encode_i32, VarIntError};

pub mod handshake;
pub mod login;
pub mod play;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodecError {
    Buffer(BufferError),
    VarInt(VarIntError),
    InvalidPacketId(i32),
    UnexpectedPacketId {
        expected: u8,
        actual: u8,
    },
    UnknownPacketId(u8),
    InvalidNextState(i32),
    InvalidCompressionThreshold(i32),
    InvalidEnumValue {
        enum_name: &'static str,
        actual: i32,
    },
}

impl From<BufferError> for CodecError {
    fn from(value: BufferError) -> Self {
        Self::Buffer(value)
    }
}

impl From<VarIntError> for CodecError {
    fn from(value: VarIntError) -> Self {
        Self::VarInt(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedPacket {
    pub packet_id: u8,
    pub body: Vec<u8>,
}

impl EncodedPacket {
    pub fn new(packet_id: u8, body: Vec<u8>) -> Self {
        Self { packet_id, body }
    }

    pub fn packet_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        encode_i32(i32::from(self.packet_id), &mut bytes);
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

pub fn split_packet_bytes(packet_bytes: &[u8]) -> Result<(u8, &[u8]), CodecError> {
    let (packet_id, packet_id_len) = decode_i32(packet_bytes)?;
    let packet_id = u8::try_from(packet_id).map_err(|_| CodecError::InvalidPacketId(packet_id))?;
    Ok((packet_id, &packet_bytes[packet_id_len..]))
}
