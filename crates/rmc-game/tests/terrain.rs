use rmc_game::{
    camera::CameraState,
    input::MovementInput,
    player::Vec3,
    simulation::{AuthoritativePlayerState, LocalSimulationLayer, SimulationConfig},
};
use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
use rmc_world::{WorldConfig, WorldSnapshot};

fn put(world: &mut WorldSnapshot, x: i32, y: i32, z: i32, state: u16) {
    world
        .apply_block_change(&BlockChangePacket {
            position: BlockPosition { x, y, z },
            block_state_id: state.into(),
        })
        .unwrap();
}
fn player(position: Vec3) -> LocalSimulationLayer {
    let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
    simulation.apply_authoritative_state(AuthoritativePlayerState {
        position,
        velocity: Vec3::ZERO,
        on_ground: false,
    });
    simulation
}

#[test]
fn live_terrain_has_no_synthetic_floor_or_arena_wall() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    put(&mut world, 257, 64, 256, 16);
    let mut simulation = player(Vec3::new(256.0, 0.0, 256.0));
    for _ in 0..10 {
        simulation.tick_with_world(MovementInput::default(), CameraState::default(), 0, &world);
    }
    assert!(simulation.player().position.y < -1.0);
    assert_eq!(simulation.player().position.x, 256.0);
    assert!(!simulation.player().on_ground);
}

#[test]
fn unloaded_chunk_uses_vanilla_vertical_wait_motion() {
    let world = WorldSnapshot::new(WorldConfig::overworld());
    let mut simulation = player(Vec3::new(0.5, 64.0, 0.5));
    simulation.tick_with_world(MovementInput::default(), CameraState::default(), 0, &world);
    assert!((simulation.velocity().y + 0.1 * f64::from(0.98_f32)).abs() < 1.0e-9);
}

#[test]
fn lands_on_received_floor_and_cannot_walk_through_wall() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    for x in -2..=2 {
        for z in -2..=4 {
            put(&mut world, x, 63, z, 16);
        }
    }
    for y in 64..=66 {
        put(&mut world, 0, y, 2, 16);
    }
    let mut simulation = player(Vec3::new(0.5, 65.0, 0.5));
    for _ in 0..50 {
        simulation.tick_with_world(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
            &world,
        );
    }
    assert_eq!(simulation.player().position.y, 64.0);
    assert!(simulation.player().on_ground);
    assert!((simulation.player().position.z - 1.7).abs() < 1.0e-6);
}

#[test]
fn sneaking_preserves_support_at_platform_edge() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    put(&mut world, 0, 63, 0, 16);
    let mut simulation = player(Vec3::new(0.5, 64.0, 0.5));
    for _ in 0..100 {
        simulation.tick_with_world(
            MovementInput {
                forward: 1.0,
                sneak: true,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
            &world,
        );
    }
    assert_eq!(simulation.player().position.y, 64.0);
    assert!(simulation.player().position.z < 1.31);
}

#[test]
fn steps_onto_bottom_slab_without_jumping() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    for z in -1..=3 {
        put(&mut world, 0, 63, z, 16);
    }
    put(&mut world, 0, 64, 1, 44 << 4);
    let mut simulation = player(Vec3::new(0.5, 64.0, 0.5));
    for _ in 0..8 {
        simulation.tick_with_world(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
            &world,
        );
    }
    assert!(simulation.player().position.z > 1.0);
    assert_eq!(simulation.player().position.y, 64.5);
}
#[test]
fn water_and_lava_use_their_own_drag_and_gravity() {
    for (id, drag) in [(9, f64::from(0.8_f32)), (11, 0.5)] {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        put(&mut world, 0, 64, 0, id << 4);
        put(&mut world, 0, 65, 0, id << 4);
        let mut simulation = player(Vec3::new(0.5, 64.0, 0.5));
        simulation.apply_authoritative_state(AuthoritativePlayerState {
            position: Vec3::new(0.5, 64.0, 0.5),
            velocity: Vec3::new(0.1, -0.1, 0.0),
            on_ground: false,
        });
        simulation.tick_with_world(MovementInput::default(), CameraState::default(), 0, &world);
        assert!((simulation.velocity().x - 0.1 * drag).abs() < 1e-9);
        assert!((simulation.velocity().y - (-0.1 * drag - 0.02)).abs() < 1e-9);
    }
}

#[test]
fn ladder_clamps_descent_and_sneak_holds_position() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    put(&mut world, 0, 64, 0, (65 << 4) | 2);
    let mut simulation = player(Vec3::new(0.5, 64.5, 0.5));
    simulation.apply_authoritative_state(AuthoritativePlayerState {
        position: Vec3::new(0.5, 64.5, 0.5),
        velocity: Vec3::new(0.0, -0.6, 0.0),
        on_ground: false,
    });
    simulation.tick_with_world(
        MovementInput {
            sneak: true,
            ..MovementInput::default()
        },
        CameraState::default(),
        0,
        &world,
    );
    assert_eq!(simulation.player().position.y, 64.5);
}
