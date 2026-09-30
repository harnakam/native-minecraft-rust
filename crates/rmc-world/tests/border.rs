use rmc_net::codec::play::{PlayClientboundPacket, WorldBorderPacket as Packet};
use rmc_world::{border::WorldBorder, WorldConfig, WorldSnapshot};

#[test]
fn border_actions_interpolate_in_float_and_clamp_world_limits() {
    let mut border = WorldBorder::default();
    assert_eq!(
        border.bounds_at(0),
        [-29999984.0, 29999984.0, -29999984.0, 29999984.0]
    );
    border.receive_at(
        &Packet::Initialize {
            x: 1.0,
            z: -2.0,
            from: 16.0,
            to: 32.0,
            milliseconds: 1000,
            size: 20,
            warning_distance: 7,
            warning_time: 10,
        },
        100,
    );
    assert_eq!(border.diameter_at(600), 24.0);
    assert_eq!(border.bounds_at(600), [-11.0, 13.0, -14.0, 10.0]);
    assert_eq!(border.closest_distance_at(15.0, 0.0, 600), -2.0);
    assert_eq!(border.diameter_at(1100), 32.0);
    assert_eq!(border.diameter_at(2000), 32.0);
    border.receive_at(&Packet::SetCenter { x: 10.0, z: 10.0 }, 2000);
    assert_eq!(border.bounds_at(2000), [-6.0, 20.0, -6.0, 20.0]);
    border.receive_at(&Packet::SetWarningTime(3), 2000);
    border.receive_at(&Packet::SetWarningBlocks(4), 2000);
    assert_eq!((border.warning_time, border.warning_distance), (3, 4));
    border.receive_at(&Packet::SetSize { diameter: 2.0 }, 2000);
    assert_eq!(border.bounds_at(2000), [9.0, 11.0, 9.0, 11.0]);
    border.receive_at(
        &Packet::LerpSize {
            from: 10.0,
            to: 20.0,
            milliseconds: 3,
        },
        0,
    );
    assert_eq!(
        border.diameter_at(1).to_bits(),
        (10.0 + 10.0 * f64::from(1.0_f32 / 3.0)).to_bits()
    );
}

#[test]
fn received_border_updates_reach_world_state() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    world
        .apply_play_packet(&PlayClientboundPacket::WorldBorder(Packet::SetSize {
            diameter: 64.0,
        }))
        .unwrap();
    assert_eq!(world.border_mut().diameter_at(0), 64.0);
    world
        .apply_play_packet(&PlayClientboundPacket::WorldBorder(Packet::SetCenter {
            x: 4.0,
            z: -4.0,
        }))
        .unwrap();
    assert_eq!(world.border_mut().center, [4.0, -4.0]);
}

#[test]
fn collision_border_uses_loaded_columns_and_outside_hysteresis() {
    use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
    use rmc_world::collision::Aabb;
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    world
        .apply_play_packet(&PlayClientboundPacket::WorldBorder(Packet::SetSize {
            diameter: 4.0,
        }))
        .unwrap();
    let area = Aabb::new([1.8, 64.0, 0.1], [2.8, 65.8, 0.9]);
    let mut outside = false;
    assert!(world
        .collision_boxes_for_player(area, [1.5, 64.0, 0.5], &mut outside)
        .is_empty());
    world
        .apply_block_change(&BlockChangePacket {
            position: BlockPosition::new(0, 0, 0),
            block_state_id: 16,
        })
        .unwrap();
    let obstacles = world.collision_boxes_for_player(area, [1.5, 64.0, 0.5], &mut outside);
    assert_eq!(obstacles.len(), 2);
    assert_eq!(obstacles[0], Aabb::new([2.0, 64.0, 0.0], [3.0, 65.0, 1.0]));
    assert!(!outside);
    assert!(world
        .collision_boxes_for_player(area, [4.0, 64.0, 0.5], &mut outside)
        .is_empty());
    assert!(outside);
    assert!(world
        .collision_boxes_for_player(area, [1.5, 64.0, 0.5], &mut outside)
        .is_empty());
    assert!(outside);
    assert_eq!(
        world
            .collision_boxes_for_player(area, [0.5, 64.0, 0.5], &mut outside)
            .len(),
        2
    );
    assert!(!outside);
}
