use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
use rmc_world::{collision::Aabb, BlockPos, WorldConfig, WorldSnapshot};

fn set(world: &mut WorldSnapshot, x: i32, y: i32, z: i32, state: u16) {
    world
        .apply_block_change(&BlockChangePacket {
            position: BlockPosition { x, y, z },
            block_state_id: i32::from(state),
        })
        .unwrap();
}

#[test]
fn cube_clips_a_falling_player_to_its_top() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    set(&mut world, -1, 63, -1, 16);
    let player = Aabb::new([-0.8, 64.1, -0.8], [-0.2, 65.9, -0.2]);
    let obstacles = world.collision_boxes(player.swept([0.0, -0.5, 0.0]));
    assert_eq!(obstacles.len(), 1);
    assert!((obstacles[0].clip_axis(player, 1, -0.5) + 0.1).abs() < 1.0e-9);
}

#[test]
fn slabs_and_non_solid_blocks_have_distinct_shapes() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    set(&mut world, 0, 0, 0, 44 << 4);
    set(&mut world, 1, 0, 0, (44 << 4) | 8);
    set(&mut world, 2, 0, 0, 31 << 4);
    let boxes = world.collision_boxes(Aabb::new([0.0, 0.0, 0.0], [3.0, 2.0, 1.0]));
    assert_eq!(boxes.len(), 2);
    assert_eq!(boxes[0].max[1], 0.5);
    assert_eq!(boxes[1].min[1], 0.5);
}

#[test]
fn ray_hits_first_wall_and_reports_face() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    set(&mut world, 0, 64, 2, 16);
    set(&mut world, 0, 64, 3, 16);
    let hit = world
        .raycast([0.5, 64.5, 0.5], [0.0, 0.0, 1.0], 4.5)
        .unwrap();
    assert_eq!(hit.position, BlockPos::new(0, 64, 2));
    assert_eq!(hit.face, 2);
    assert_eq!(hit.distance, 1.5);
    assert!(world
        .raycast([0.5, 64.5, 0.5], [0.0, 0.0, 1.0], 1.0)
        .is_none());
}
