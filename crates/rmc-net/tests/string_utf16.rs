use rmc_net::buffer::{PacketReader, PacketWriter};
use rmc_net::codec::play::{ChatMessageServerboundPacket, PlayServerboundPacket};

#[test]
fn packet_strings_apply_java_utf16_length_not_scalar_count() {
    let text = "\u{1f600}";
    let mut writer = PacketWriter::new();
    assert!(writer.write_string(text, 1).is_err());
    writer.write_string(text, 2).unwrap();
    let bytes = writer.into_inner();
    assert_eq!(bytes, vec![4, 0xf0, 0x9f, 0x98, 0x80]);
    assert!(PacketReader::new(&bytes).read_string(1).is_err());
    assert_eq!(PacketReader::new(&bytes).read_string(2).unwrap(), text);
    let packet = ChatMessageServerboundPacket::vanilla(&text.repeat(60));
    assert_eq!(packet.message.encode_utf16().count(), 100);
    assert_eq!(packet.message.chars().count(), 50);
    let bytes = PlayServerboundPacket::ChatMessage(packet.clone())
        .encode_packet()
        .unwrap()
        .packet_bytes();
    assert_eq!(
        PlayServerboundPacket::decode_packet(&bytes).unwrap(),
        PlayServerboundPacket::ChatMessage(packet)
    );
    let too_long = PlayServerboundPacket::ChatMessage(ChatMessageServerboundPacket {
        message: text.repeat(51),
    });
    assert!(too_long.encode_packet().is_err());
}

#[test]
fn chat_boundary_matches_java_isolated_surrogate_wire_replacement() {
    let message = format!("{}\u{1f600}", "x".repeat(99));
    let packet = ChatMessageServerboundPacket::vanilla(&message);
    assert_eq!(packet.message, format!("{}?", "x".repeat(99)));
    let bytes = PlayServerboundPacket::ChatMessage(packet)
        .encode_packet()
        .unwrap()
        .packet_bytes();
    assert_eq!(bytes.len(), 102);
    assert_eq!(&bytes[..2], &[1, 100]);
    assert_eq!(*bytes.last().unwrap(), 63);
}

#[test]
fn packet_strings_replace_malformed_utf8_like_java_8() {
    let cases: &[(&[u8], &str)] = &[
        (&[0xed, 0xa0, 0x80], "\u{fffd}"),
        (&[0xed, 0xbf, 0xbf], "\u{fffd}"),
        (&[0xed, 0xa0], "\u{fffd}"),
        (&[0xe0, 0x80, 0x80], "\u{fffd}\u{fffd}\u{fffd}"),
        (
            &[0xf0, 0x80, 0x80, 0x80],
            "\u{fffd}\u{fffd}\u{fffd}\u{fffd}",
        ),
        (
            &[0xf4, 0x90, 0x80, 0x80],
            "\u{fffd}\u{fffd}\u{fffd}\u{fffd}",
        ),
        (&[0xe1, 0x80, 0x41], "\u{fffd}A"),
        (&[0xf1, 0x80, 0x80, 0x41], "\u{fffd}A"),
        (&[0xc0, 0x80], "\u{fffd}\u{fffd}"),
        (&[0xed, 0xa0, 0x41], "\u{fffd}A"),
    ];
    for &(input, expected) in cases {
        let mut bytes = vec![input.len() as u8];
        bytes.extend_from_slice(input);
        let mut reader = PacketReader::new(&bytes);
        assert_eq!(reader.read_string(10).unwrap(), expected, "{input:x?}");
        assert_eq!(reader.remaining(), 0);
    }
    assert_eq!(
        PacketReader::new(&[3, 0xed, 0xa0, 0x80])
            .read_string(1)
            .unwrap(),
        "\u{fffd}"
    );
    assert!(PacketReader::new(&[3, 0xe0, 0x80, 0x80])
        .read_string(1)
        .is_err());
    assert!(PacketReader::new(&[3, 0xed, 0xa0]).read_string(10).is_err());
}
