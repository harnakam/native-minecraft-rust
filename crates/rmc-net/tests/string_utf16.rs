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
