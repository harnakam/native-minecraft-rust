use rmc_net::codec::play::PlayClientboundPacket;
use rmc_world::{WorldConfig, WorldSnapshot};

#[test]
fn difficulty_wire_has_one_byte_and_updates_world() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    for raw in [0_u8, 1, 2, 3, 4, 255] {
        let packet = PlayClientboundPacket::decode_packet(&[0x41, raw]).unwrap();
        assert_eq!(packet, PlayClientboundPacket::ServerDifficulty(raw % 4));
        assert_eq!(
            packet.encode_packet().unwrap().packet_bytes(),
            vec![0x41, raw % 4]
        );
        world.apply_play_packet(&packet).unwrap();
        assert_eq!(world.difficulty(), Some(raw % 4));
    }
    assert!(PlayClientboundPacket::decode_packet(&[0x41]).is_err());
    assert!(PlayClientboundPacket::decode_packet(&[0x41, 2, 1]).is_err());
}
