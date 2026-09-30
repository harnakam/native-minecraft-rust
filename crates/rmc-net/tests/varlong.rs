use rmc_net::buffer::{PacketReader, PacketWriter};
use rmc_net::varint::{decode_i64, encode_i64, VarIntError};

#[test]
fn varlong_preserves_signed_java_long_bits_and_buffer_offsets() {
    for value in [
        0,
        1,
        127,
        128,
        255,
        i32::MAX as i64 + 1,
        i64::MAX,
        i64::MIN,
        -1,
    ] {
        let mut writer = PacketWriter::new();
        writer.write_var_i64(value);
        writer.write_u8(42);
        let bytes = writer.into_inner();
        let mut reader = PacketReader::new(&bytes);
        assert_eq!(reader.read_var_i64().unwrap(), value);
        assert_eq!(reader.read_u8().unwrap(), 42);
        reader.finish().unwrap();
    }
    let mut bytes = Vec::new();
    encode_i64(-1, &mut bytes);
    assert_eq!(bytes, vec![255, 255, 255, 255, 255, 255, 255, 255, 255, 1]);
    assert_eq!(decode_i64(&[128, 1]), Ok((128, 2)));
    assert_eq!(decode_i64(&[128]), Err(VarIntError::Incomplete));
    assert_eq!(decode_i64(&[128; 10]), Err(VarIntError::TooLarge));
    // Java long shifts retain only the low bit of the tenth payload byte.
    assert_eq!(
        decode_i64(&[255, 255, 255, 255, 255, 255, 255, 255, 255, 127]),
        Ok((-1, 10))
    );
}
