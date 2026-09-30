//! Minecraft VarInt helpers used by packet framing and packet bodies.

/// A Minecraft VarInt may consume at most 5 bytes.
pub const MAX_VARINT_BYTES: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VarIntError {
    Incomplete,
    TooLarge,
}

pub fn encoded_len_i32(value: i32) -> usize {
    let mut remaining = value as u32;
    let mut len = 1;

    while (remaining & !0x7f) != 0 {
        remaining >>= 7;
        len += 1;
    }

    len
}

pub fn encode_i32(value: i32, out: &mut Vec<u8>) {
    let mut remaining = value as u32;

    loop {
        if (remaining & !0x7f) == 0 {
            out.push(remaining as u8);
            return;
        }

        out.push(((remaining & 0x7f) | 0x80) as u8);
        remaining >>= 7;
    }
}

pub fn encode_usize(value: usize, out: &mut Vec<u8>) {
    encode_i32(value as i32, out);
}

pub fn decode_i32(input: &[u8]) -> Result<(i32, usize), VarIntError> {
    let mut value = 0u32;

    for (index, byte) in input.iter().copied().enumerate() {
        value |= u32::from(byte & 0x7f) << (index * 7);

        if (byte & 0x80) == 0 {
            return Ok((value as i32, index + 1));
        }

        if index + 1 == MAX_VARINT_BYTES {
            return Err(VarIntError::TooLarge);
        }
    }

    Err(VarIntError::Incomplete)
}

#[cfg(test)]
mod tests {
    use super::{decode_i32, encode_i32, encoded_len_i32, VarIntError};

    #[test]
    fn roundtrip_common_values() {
        let values = [0, 1, 2, 127, 128, 255, 2_097_151, i32::MAX, -1];

        for value in values {
            let mut encoded = Vec::new();
            encode_i32(value, &mut encoded);
            let (decoded, read) = decode_i32(&encoded).expect("value should decode");

            assert_eq!(decoded, value);
            assert_eq!(read, encoded.len());
            assert_eq!(read, encoded_len_i32(value));
        }
    }

    #[test]
    fn reports_incomplete_input() {
        assert_eq!(decode_i32(&[0x80]), Err(VarIntError::Incomplete));
    }

    #[test]
    fn reports_oversized_input() {
        assert_eq!(
            decode_i32(&[0xff, 0xff, 0xff, 0xff, 0xff]),
            Err(VarIntError::TooLarge)
        );
    }
}
