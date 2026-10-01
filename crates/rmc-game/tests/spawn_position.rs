use rmc_game::usability::UsabilityState;
use rmc_net::codec::play::{BlockPosition, PlayClientboundPacket};
use rmc_world::{WorldConfig, WorldSnapshot};
#[test]
fn spawn_packet_roundtrips_and_updates_both_spawn_owners() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    let mut player = UsabilityState::new();
    assert_eq!(world.spawn_position(), None);
    assert_eq!(player.forced_spawn_position(), None);
    for position in [
        BlockPosition::new(8, 64, 8),
        BlockPosition::new(-1, -1, -1),
        BlockPosition::new(-33554432, -2048, 33554431),
    ] {
        let packet = PlayClientboundPacket::SpawnPosition(position);
        let encoded = packet.encode_packet().unwrap();
        assert_eq!(encoded.packet_id, 5);
        assert_eq!(encoded.body.len(), 8);
        if position == BlockPosition::new(-1, -1, -1) {
            assert_eq!(encoded.body, [255; 8]);
        }
        let decoded = PlayClientboundPacket::decode_packet(&encoded.packet_bytes()).unwrap();
        assert_eq!(decoded, packet);
        world.apply_play_packet(&decoded).unwrap();
        player.apply_play_packet(&decoded);
        assert_eq!(world.spawn_position(), Some(position));
        assert_eq!(player.forced_spawn_position(), Some(position));
    }
    assert_eq!(
        WorldSnapshot::new(WorldConfig::overworld()).spawn_position(),
        None
    );
    assert!(PlayClientboundPacket::decode_body(5, &[0; 7]).is_err());
    assert!(PlayClientboundPacket::decode_body(5, &[0; 9]).is_err());
}
