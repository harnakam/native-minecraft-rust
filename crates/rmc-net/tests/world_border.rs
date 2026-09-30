use rmc_net::codec::play::{PlayClientboundPacket, WorldBorderPacket};

#[test]
fn all_border_actions_roundtrip_and_reject_truncated_fields() {
    let packets = [
        WorldBorderPacket::SetSize { diameter: 16.0 },
        WorldBorderPacket::LerpSize {
            from: 16.0,
            to: 32.0,
            milliseconds: -1,
        },
        WorldBorderPacket::SetCenter { x: 1.0, z: -2.0 },
        WorldBorderPacket::Initialize {
            x: 1.0,
            z: -2.0,
            from: 16.0,
            to: 32.0,
            milliseconds: 128,
            size: 300,
            warning_distance: 5,
            warning_time: 15,
        },
        WorldBorderPacket::SetWarningTime(15),
        WorldBorderPacket::SetWarningBlocks(5),
    ];
    for (action, packet) in packets.into_iter().enumerate() {
        let packet = PlayClientboundPacket::WorldBorder(packet);
        let bytes = packet.encode_packet().unwrap().packet_bytes();
        assert_eq!(bytes[0..2], [0x44, action as u8]);
        assert_eq!(
            PlayClientboundPacket::decode_packet(&bytes).unwrap(),
            packet
        );
        for length in 1..bytes.len() {
            assert!(PlayClientboundPacket::decode_packet(&bytes[..length]).is_err());
        }
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(PlayClientboundPacket::decode_packet(&trailing).is_err());
        if action == 3 {
            assert_eq!(
                bytes,
                vec![
                    0x44, 3, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0, 0xc0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0x30,
                    0, 0, 0, 0, 0, 0, 0x40, 0x40, 0, 0, 0, 0, 0, 0, 0x80, 1, 0xac, 2, 5, 15,
                ]
            );
        }
    }
    assert!(PlayClientboundPacket::decode_packet(&[0x44, 6]).is_err());
    assert!(PlayClientboundPacket::decode_packet(&[
        0x44, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 128, 128, 128, 128, 128, 128,
        128, 128, 128
    ])
    .is_err());
}
