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
#[test]
fn doors_trapdoors_panes_and_hoppers_are_not_full_cubes() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    set(&mut world, 0, 64, 0, 64 << 4);
    set(&mut world, 0, 65, 0, (64 << 4) | 8);
    assert_eq!(
        world.block_collision_boxes(BlockPos::new(0, 64, 0))[0].max[0],
        0.1875
    );
    set(&mut world, 0, 64, 0, (64 << 4) | 4);
    assert_eq!(
        world.block_collision_boxes(BlockPos::new(0, 65, 0))[0].max[2],
        0.1875
    );
    set(&mut world, 2, 64, 0, 96 << 4);
    assert_eq!(
        world.block_collision_boxes(BlockPos::new(2, 64, 0))[0].max[1],
        64.1875
    );
    set(&mut world, 4, 64, 0, 102 << 4);
    assert_eq!(
        world.block_collision_boxes(BlockPos::new(4, 64, 0)).len(),
        2
    );
    set(&mut world, 6, 64, 0, 154 << 4);
    assert_eq!(
        world.block_collision_boxes(BlockPos::new(6, 64, 0)).len(),
        5
    );
}

#[test]
fn stairs_form_outer_and_inner_corners_from_neighbors() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    set(&mut world, 0, 64, 0, 53 << 4); // east
    set(&mut world, 1, 64, 0, (53 << 4) | 3); // north, outer
    let boxes = world.block_collision_boxes(BlockPos::new(0, 64, 0));
    let upper_volume: f64 = boxes
        .iter()
        .filter(|b| b.min[1] >= 64.5)
        .map(|b| (b.max[0] - b.min[0]) * (b.max[2] - b.min[2]))
        .sum();
    assert_eq!(upper_volume, 0.25);
    set(&mut world, 1, 64, 0, 0);
    set(&mut world, -1, 64, 0, (53 << 4) | 3);
    let boxes = world.block_collision_boxes(BlockPos::new(0, 64, 0));
    let upper_volume: f64 = boxes
        .iter()
        .filter(|b| b.min[1] >= 64.5)
        .map(|b| (b.max[0] - b.min[0]) * (b.max[2] - b.min[2]))
        .sum();
    assert_eq!(upper_volume, 0.75);
}
#[test]
fn noncolliding_plant_is_selectable_and_liquid_is_not_a_solid_target() {
    use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
    use rmc_world::{BlockPos, WorldConfig, WorldSnapshot};
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    world
        .apply_block_change(&BlockChangePacket {
            position: BlockPosition::new(0, 64, 0),
            block_state_id: 37 << 4,
        })
        .unwrap();
    assert!(world
        .block_collision_boxes(BlockPos::new(0, 64, 0))
        .is_empty());
    assert!(world
        .raycast([0.5, 64.5, -2.0], [0.0, 0.0, 1.0], 4.0)
        .is_some());
    world
        .apply_block_change(&BlockChangePacket {
            position: BlockPosition::new(0, 64, 0),
            block_state_id: 9 << 4,
        })
        .unwrap();
    assert!(world
        .raycast([0.5, 64.5, -2.0], [0.0, 0.0, 1.0], 4.0)
        .is_none());
}

#[test]
fn ray_from_inside_hits_exit_and_surface_hits_keep_the_actual_face() {
    let bounds = Aabb::new([0.0; 3], [1.0; 3]);
    assert_eq!(
        bounds.ray_hit([0.5; 3], [1.0, 0.0, 0.0], 4.0),
        Some((0.5, 5))
    );
    assert_eq!(
        bounds.ray_hit([0.0, 0.5, 0.5], [1.0, 0.0, 0.0], 4.0),
        Some((0.0, 4))
    );
    assert_eq!(bounds.ray_hit([0.5; 3], [0.0, 1.0, 0.0], 0.25), None);
    // Java ignores face intersections when that component of the segment is tiny.
    assert_eq!(
        bounds.ray_hit([0.0, 0.5, 0.5], [1.0, 0.0, 0.0], 0.0001),
        None
    );
}

#[test]
fn stair_internal_surfaces_follow_vanilla_octant_trace_order() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    set(&mut world, 0, 64, 0, 53 << 4);
    for direction in [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0]] {
        let hit = world.raycast([0.5, 64.5, 0.5], direction, 4.0).unwrap();
        assert_eq!((hit.distance, hit.face), (0.0, 5));
    }
}

#[test]
fn fence_and_pane_connections_use_distinct_mcp_material_predicates() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    set(&mut world, 0, 64, 0, 85 << 4);
    set(&mut world, 1, 64, 0, 188 << 4);
    let max_x = |world: &WorldSnapshot| {
        world
            .block_collision_boxes(BlockPos::new(0, 64, 0))
            .iter()
            .map(|b| b.max[0])
            .fold(0.0, f64::max)
    };
    assert_eq!(max_x(&world), 1.0); // Different wood fences connect.
    for neighbor in [113, 20, 95, 86, 91, 166] {
        set(&mut world, 1, 64, 0, neighbor << 4);
        assert_eq!(max_x(&world), 0.625, "neighbor {neighbor}");
    }
    set(&mut world, 0, 64, 0, 102 << 4);
    set(&mut world, 1, 64, 0, 86 << 4);
    let bounds = world.selection_boxes(BlockPos::new(0, 64, 0));
    assert_eq!(bounds[0].min[0], 0.4375);
    assert_eq!(bounds[0].max[0], 1.0); // Panes connect to pumpkin's full-block flag.
}
