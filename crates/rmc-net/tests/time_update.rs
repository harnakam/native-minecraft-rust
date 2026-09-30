use rmc_net::codec::play::{PlayClientboundPacket, TimeUpdatePacket};
#[test]
fn time_update_keeps_two_signed_longs_and_frozen_daylight_sentinel() {
    let packet = PlayClientboundPacket::TimeUpdate(TimeUpdatePacket {
        total_world_time: 0x0102030405060708,
        world_time: -1,
    });
    let encoded = packet.encode_packet().unwrap();
    assert_eq!(encoded.packet_id, 3);
    assert_eq!(&encoded.body[..8], &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(&encoded.body[8..], &[255; 8]);
    assert_eq!(
        PlayClientboundPacket::decode_body(3, &encoded.body).unwrap(),
        packet
    );
    assert!(PlayClientboundPacket::decode_body(3, &[0; 15]).is_err());
}
