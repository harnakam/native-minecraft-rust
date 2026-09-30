use rmc_net::codec::play::{PlayClientboundPacket, PlayerListHeaderFooterPacket};
#[test]
fn tab_header_footer_wire_roundtrip_and_truncation() {
    let packet = PlayClientboundPacket::PlayerListHeaderFooter(PlayerListHeaderFooterPacket {
        header_json: "{\"text\":\"Header\"}".into(),
        footer_json: "{\"text\":\"Footer\"}".into(),
    });
    let bytes = packet.encode_packet().unwrap().packet_bytes();
    assert_eq!(bytes[0], 0x47);
    assert_eq!(
        PlayClientboundPacket::decode_packet(&bytes).unwrap(),
        packet
    );
    for end in 1..bytes.len() {
        assert!(PlayClientboundPacket::decode_packet(&bytes[..end]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(PlayClientboundPacket::decode_packet(&extra).is_err());
}
