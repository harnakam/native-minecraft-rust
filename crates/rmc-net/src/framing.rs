//! Stream framing for Minecraft packet transport.

use crate::compression::{CompressionError, CompressionState, ZlibCodec};
use crate::varint::{decode_i32, encode_usize, VarIntError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameLimits {
    pub max_wire_size: usize,
    pub max_payload_size: usize,
}

impl Default for FrameLimits {
    fn default() -> Self {
        Self {
            max_wire_size: 2 * 1024 * 1024,
            max_payload_size: 8 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionDisposition {
    Disabled,
    Uncompressed,
    Compressed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PacketFrame {
    pub packet_bytes: Vec<u8>,
    pub wire_size: usize,
    pub compression: CompressionDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FrameError {
    VarInt(VarIntError),
    NegativeLength(i32),
    FrameTooLarge { actual: usize, limit: usize },
    PayloadTooLarge { actual: usize, limit: usize },
    Compression(CompressionError),
}

impl From<VarIntError> for FrameError {
    fn from(value: VarIntError) -> Self {
        Self::VarInt(value)
    }
}

impl From<CompressionError> for FrameError {
    fn from(value: CompressionError) -> Self {
        Self::Compression(value)
    }
}

pub struct FrameDecoder {
    compression: CompressionState,
    limits: FrameLimits,
    buffer: Vec<u8>,
}

impl FrameDecoder {
    pub fn new(compression: CompressionState) -> Self {
        Self::with_limits(compression, FrameLimits::default())
    }

    pub fn with_limits(compression: CompressionState, limits: FrameLimits) -> Self {
        Self {
            compression,
            limits,
            buffer: Vec::new(),
        }
    }

    pub fn compression(&self) -> CompressionState {
        self.compression
    }

    pub fn set_compression(&mut self, compression: CompressionState) {
        self.compression = compression;
    }

    pub fn queue_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn buffered_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn try_next_frame(
        &mut self,
        codec: Option<&dyn ZlibCodec>,
    ) -> Result<Option<PacketFrame>, FrameError> {
        let (frame_len_i32, prefix_len) = match decode_i32(&self.buffer) {
            Ok(value) => value,
            Err(VarIntError::Incomplete) => return Ok(None),
            Err(error) => return Err(FrameError::VarInt(error)),
        };

        if frame_len_i32 < 0 {
            return Err(FrameError::NegativeLength(frame_len_i32));
        }

        let frame_len = frame_len_i32 as usize;

        if frame_len > self.limits.max_wire_size {
            return Err(FrameError::FrameTooLarge {
                actual: frame_len,
                limit: self.limits.max_wire_size,
            });
        }

        let total_len = prefix_len + frame_len;

        if self.buffer.len() < total_len {
            return Ok(None);
        }

        let frame_bytes = self.buffer[prefix_len..total_len].to_vec();
        self.buffer.drain(..total_len);

        let decoded = match self.compression {
            CompressionState::Disabled => PacketFrame {
                packet_bytes: frame_bytes,
                wire_size: total_len,
                compression: CompressionDisposition::Disabled,
            },
            CompressionState::Enabled { threshold } => {
                decode_compressed_frame(frame_bytes, total_len, threshold, codec, self.limits)?
            }
        };

        if decoded.packet_bytes.len() > self.limits.max_payload_size {
            return Err(FrameError::PayloadTooLarge {
                actual: decoded.packet_bytes.len(),
                limit: self.limits.max_payload_size,
            });
        }

        Ok(Some(decoded))
    }
}

pub fn encode_frame(
    packet_bytes: &[u8],
    compression: CompressionState,
    codec: Option<&dyn ZlibCodec>,
    limits: FrameLimits,
) -> Result<Vec<u8>, FrameError> {
    if packet_bytes.len() > limits.max_payload_size {
        return Err(FrameError::PayloadTooLarge {
            actual: packet_bytes.len(),
            limit: limits.max_payload_size,
        });
    }

    let body = match compression {
        CompressionState::Disabled => packet_bytes.to_vec(),
        CompressionState::Enabled { threshold } => {
            encode_compressed_body(packet_bytes, threshold, codec)?
        }
    };

    if body.len() > limits.max_wire_size {
        return Err(FrameError::FrameTooLarge {
            actual: body.len(),
            limit: limits.max_wire_size,
        });
    }

    let mut framed = Vec::new();
    encode_usize(body.len(), &mut framed);
    framed.extend_from_slice(&body);
    Ok(framed)
}

fn decode_compressed_frame(
    frame_bytes: Vec<u8>,
    wire_size: usize,
    threshold: usize,
    codec: Option<&dyn ZlibCodec>,
    limits: FrameLimits,
) -> Result<PacketFrame, FrameError> {
    let (data_len_i32, data_len_prefix) = decode_i32(&frame_bytes)?;

    if data_len_i32 < 0 {
        return Err(FrameError::NegativeLength(data_len_i32));
    }

    let data_len = data_len_i32 as usize;
    let body = &frame_bytes[data_len_prefix..];

    if data_len == 0 {
        return Ok(PacketFrame {
            packet_bytes: body.to_vec(),
            wire_size,
            compression: CompressionDisposition::Uncompressed,
        });
    }

    if data_len < threshold {
        return Err(FrameError::Compression(CompressionError::BelowThreshold {
            actual: data_len,
            threshold,
        }));
    }

    if data_len > limits.max_payload_size {
        return Err(FrameError::PayloadTooLarge {
            actual: data_len,
            limit: limits.max_payload_size,
        });
    }

    let codec = codec.ok_or(CompressionError::MissingCodec)?;
    let packet_bytes = codec.decompress(body, data_len)?;

    if packet_bytes.len() != data_len {
        return Err(FrameError::PayloadTooLarge {
            actual: packet_bytes.len(),
            limit: data_len,
        });
    }

    Ok(PacketFrame {
        packet_bytes,
        wire_size,
        compression: CompressionDisposition::Compressed,
    })
}

fn encode_compressed_body(
    packet_bytes: &[u8],
    threshold: usize,
    codec: Option<&dyn ZlibCodec>,
) -> Result<Vec<u8>, FrameError> {
    let mut body = Vec::new();

    if packet_bytes.len() < threshold {
        body.push(0);
        body.extend_from_slice(packet_bytes);
        return Ok(body);
    }

    let codec = codec.ok_or(CompressionError::MissingCodec)?;
    let compressed = codec.compress(packet_bytes)?;
    encode_usize(packet_bytes.len(), &mut body);
    body.extend_from_slice(&compressed);
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::{encode_frame, CompressionDisposition, FrameDecoder, FrameLimits};
    use crate::compression::{CompressionError, CompressionState, ZlibCodec};

    struct IdentityCodec;

    impl ZlibCodec for IdentityCodec {
        fn compress(&self, input: &[u8]) -> Result<Vec<u8>, CompressionError> {
            Ok(input.to_vec())
        }

        fn decompress(
            &self,
            input: &[u8],
            _expected_len: usize,
        ) -> Result<Vec<u8>, CompressionError> {
            Ok(input.to_vec())
        }
    }

    #[test]
    fn roundtrips_uncompressed_frame() {
        let limits = FrameLimits::default();
        let frame = encode_frame(
            &[0x01, 0x02, 0x03],
            CompressionState::Disabled,
            None,
            limits,
        )
        .expect("frame should encode");
        let mut decoder = FrameDecoder::new(CompressionState::Disabled);
        decoder.queue_bytes(&frame);

        let decoded = decoder
            .try_next_frame(None)
            .expect("frame should decode")
            .expect("frame should be present");

        assert_eq!(decoded.packet_bytes, vec![0x01, 0x02, 0x03]);
        assert_eq!(decoded.compression, CompressionDisposition::Disabled);
    }

    #[test]
    fn roundtrips_threshold_uncompressed_frame() {
        let limits = FrameLimits::default();
        let frame = encode_frame(
            &[0x01, 0x02],
            CompressionState::enabled(32),
            Some(&IdentityCodec),
            limits,
        )
        .expect("frame should encode");
        let mut decoder = FrameDecoder::new(CompressionState::enabled(32));
        decoder.queue_bytes(&frame);

        let decoded = decoder
            .try_next_frame(Some(&IdentityCodec))
            .expect("frame should decode")
            .expect("frame should be present");

        assert_eq!(decoded.packet_bytes, vec![0x01, 0x02]);
        assert_eq!(decoded.compression, CompressionDisposition::Uncompressed);
    }

    #[test]
    fn roundtrips_compressed_frame() {
        let limits = FrameLimits::default();
        let packet = vec![0x01; 64];
        let frame = encode_frame(
            &packet,
            CompressionState::enabled(32),
            Some(&IdentityCodec),
            limits,
        )
        .expect("frame should encode");
        let mut decoder = FrameDecoder::new(CompressionState::enabled(32));
        decoder.queue_bytes(&frame);

        let decoded = decoder
            .try_next_frame(Some(&IdentityCodec))
            .expect("frame should decode")
            .expect("frame should be present");

        assert_eq!(decoded.packet_bytes, packet);
        assert_eq!(decoded.compression, CompressionDisposition::Compressed);
    }
}
