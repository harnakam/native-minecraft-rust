use rmc_net::codec::play::{BlockPosition, ExplosionPacket, PlayClientboundPacket};
#[test]
fn explosion_wire_offsets_are_signed_and_center_truncates_towards_zero() {
    // S27 layout: four floats, signed int count, signed byte triplets, three floats.
    let bytes = [
        0x27, 0xbf, 0xe0, 0, 0, 0x42, 0x80, 0x80, 0, 0x41, 0x08, 0, 0, 0x40, 0, 0, 0, 0, 0, 0, 2,
        0xfe, 1, 0, 0x7f, 0x80, 0x80, 0x3e, 0x80, 0, 0, 0x3f, 0, 0, 0, 0xbe, 0x80, 0, 0,
    ];
    let expected = ExplosionPacket {
        x: -1.75,
        y: 64.25,
        z: 8.5,
        strength: 2.0,
        records: vec![[-2, 1, 0], [127, -128, -128]],
        motion: [0.25, 0.5, -0.25],
    };
    let packet = PlayClientboundPacket::decode_packet(&bytes).unwrap();
    assert_eq!(packet, PlayClientboundPacket::Explosion(expected.clone()));
    assert_eq!(packet.encode_packet().unwrap().packet_bytes(), bytes);
    assert_eq!(
        expected.affected_positions().collect::<Vec<_>>(),
        vec![
            BlockPosition::new(-3, 65, 8),
            BlockPosition::new(126, -64, -120)
        ]
    );
}
#[test]
fn explosion_lengths_truncation_and_trailing_bytes_fail_closed() {
    let mut bytes = vec![0x27];
    bytes.extend_from_slice(&[0; 16]);
    for count in [-1_i32, 262145] {
        let mut invalid = bytes.clone();
        invalid.extend_from_slice(&count.to_be_bytes());
        assert!(PlayClientboundPacket::decode_packet(&invalid).is_err());
    }
    bytes.extend_from_slice(&1_i32.to_be_bytes());
    bytes.extend_from_slice(&[0; 2]);
    assert!(PlayClientboundPacket::decode_packet(&bytes).is_err());
    let mut zero = vec![0x27];
    zero.extend_from_slice(&[0; 32]);
    zero.push(0);
    assert!(PlayClientboundPacket::decode_packet(&zero).is_err());
}
