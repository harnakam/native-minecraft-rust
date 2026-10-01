use rmc_net::codec::play::{BlockBreakAnimationPacket, BlockPosition, PlayClientboundPacket};
use rmc_world::{WorldConfig, WorldSnapshot};
fn update(world: &mut WorldSnapshot, id: i32, pos: BlockPosition, progress: u8) {
    let packet = PlayClientboundPacket::BlockBreakAnimation(BlockBreakAnimationPacket {
        breaker_id: id,
        position: pos,
        progress,
    });
    let wire = packet.encode_packet().unwrap().packet_bytes();
    let decoded = PlayClientboundPacket::decode_packet(&wire).unwrap();
    assert_eq!(decoded, packet);
    world.apply_play_packet(&decoded).unwrap();
}
#[test]
fn updates_replace_position_invalid_stages_remove_and_expiry_runs_every_twenty_ticks() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    let pos = BlockPosition::new(1, 64, 2);
    update(&mut world, 7, pos, 0);
    update(&mut world, 8, pos, 9);
    world.advance_time(400);
    assert_eq!(world.block_damage().entries().len(), 2);
    world.advance_time(1);
    assert_eq!(world.block_damage().entries().len(), 2);
    update(&mut world, 7, BlockPosition::new(3, 64, 2), 5);
    world.advance_time(19);
    assert_eq!(world.block_damage().entries().len(), 1);
    assert_eq!(world.block_damage().entries()[&7].stage, 5);
    assert_eq!(
        world.block_damage().entries()[&7].position,
        BlockPosition::new(3, 64, 2)
    );
    update(&mut world, 7, pos, 255);
    assert!(world.block_damage().entries().is_empty());
    update(&mut world, 7, pos, 9);
    update(&mut world, 7, pos, 10);
    assert!(world.block_damage().entries().is_empty());
}
#[test]
fn wire_uses_unsigned_stage_and_packed_signed_position() {
    let packet = PlayClientboundPacket::BlockBreakAnimation(BlockBreakAnimationPacket {
        breaker_id: 300,
        position: BlockPosition::new(-1, -1, -1),
        progress: 255,
    });
    let wire = packet.encode_packet().unwrap().packet_bytes();
    let mut expected = vec![0x25, 0xac, 2];
    expected.extend([255; 9]);
    assert_eq!(wire, expected);
    assert!(PlayClientboundPacket::decode_packet(&wire[..wire.len() - 1]).is_err());
}
