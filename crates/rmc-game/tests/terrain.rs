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

#[test]
fn depth_strider_matches_mcp_water_tick_and_leaves_lava_unchanged() {
    fn travel(liquid: u16, level: i16) -> (Vec3, Vec3) {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        for x in 6..=10 {
            for z in 6..=10 {
                for y in 64..=65 {
                    put(&mut world, x, y, z, liquid << 4);
                }
            }
        }
        let mut sim = player(Vec3::new(8.5, 64.0, 8.5));
        sim.apply_authoritative_state(AuthoritativePlayerState {
            position: Vec3::new(8.5, 64.0, 8.5),
            velocity: Vec3::ZERO,
            on_ground: true,
        });
        sim.set_depth_strider(level);
        sim.tick_with_world(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
            &world,
        );
        (sim.player().position, sim.velocity())
    }
    // Captured from the actual MCP919 EntityLivingBase travel method.
    let (position, velocity) = travel(9, 1);
    assert!((position.z - 8.545733332633972).abs() < 1e-12);
    assert!((velocity.z - 0.03271457769911024).abs() < 1e-12);
    assert_eq!(velocity.y, -0.02);
    assert_eq!(travel(9, 3), travel(9, 5));
    assert_eq!(travel(11, 0), travel(11, 3));
}

#[test]
fn spectator_crosses_terrain_and_border_then_survival_restores_collision() {
    use rmc_net::codec::play::{ChangeGameStatePacket, PlayClientboundPacket, WorldBorderPacket};
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    for y in [64, 65] {
        put(&mut world, 2, y, 0, 16);
    }
    world
        .apply_play_packet(&PlayClientboundPacket::WorldBorder(
            WorldBorderPacket::SetSize { diameter: 4.0 },
        ))
        .unwrap();
    let mut simulation = player(Vec3::new(1.5, 64.0, 0.5));
    let state = AuthoritativePlayerState {
        position: Vec3::new(1.5, 64.0, 0.5),
        velocity: Vec3::new(0.7, 0.0, 0.0),
        on_ground: true,
    };
    simulation.apply_authoritative_state(state);
    simulation.apply_player_packet(
        &PlayClientboundPacket::ChangeGameState(ChangeGameStatePacket {
            reason: 3,
            value: 3.0,
        }),
        Some(1),
    );
    simulation.tick_with_world(MovementInput::default(), CameraState::default(), 0, &world);
    assert!(simulation.player().position.x > 2.0);
    assert!(!simulation.player().on_ground);
    simulation.apply_player_packet(
        &PlayClientboundPacket::ChangeGameState(ChangeGameStatePacket {
            reason: 3,
            value: 0.0,
        }),
        Some(1),
    );
    simulation.apply_authoritative_state(AuthoritativePlayerState {
        position: Vec3::new(1.5, 64.0, 0.5),
        velocity: Vec3::new(0.7, 0.0, 0.0),
        on_ground: true,
    });
    simulation.tick_with_world(MovementInput::default(), CameraState::default(), 1, &world);
    assert!(simulation.player().position.x < 2.0);
    assert_eq!(simulation.velocity().x, 0.0);
}
