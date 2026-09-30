use rmc_net::codec::play::{PlayClientboundPacket, WindowPropertyPacket};

#[test]
fn window_property_matches_unsigned_id_and_signed_short_wire_layout() {
    let packet = PlayClientboundPacket::WindowProperty(WindowPropertyPacket {
        window_id: 255,
        property: -2,
        value: -32768,
    });
    let encoded = packet.encode_packet().unwrap();
    assert_eq!(encoded.packet_id, 0x31);
    assert_eq!(encoded.body, [255, 255, 254, 128, 0]);
    assert_eq!(
        PlayClientboundPacket::decode_body(0x31, &encoded.body).unwrap(),
        packet
    );
    assert!(PlayClientboundPacket::decode_body(0x31, &[1, 0, 0, 0]).is_err());
}
