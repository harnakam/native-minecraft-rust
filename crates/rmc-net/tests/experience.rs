use rmc_net::codec::play::{PlayClientboundPacket, SetExperiencePacket};
#[test]
fn experience_wire_order_is_progress_level_total() {
    let packet = PlayClientboundPacket::SetExperience(SetExperiencePacket {
        progress: 0.5,
        level: 7,
        total: 128,
    });
    let encoded = packet.encode_packet().unwrap();
    assert_eq!(encoded.packet_id, 0x1f);
    assert_eq!(encoded.body, [63, 0, 0, 0, 7, 128, 1]);
    assert_eq!(
        PlayClientboundPacket::decode_body(0x1f, &encoded.body).unwrap(),
        packet
    );
}
