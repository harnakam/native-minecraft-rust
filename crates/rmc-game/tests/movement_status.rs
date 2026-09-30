use rmc_game::{
    camera::CameraState,
    input::MovementInput,
    simulation::{LocalSimulationLayer, SimulationConfig},
};
use rmc_net::codec::play::*;
#[test]
fn hunger_and_blindness_prevent_starting_sprint() {
    let movement = MovementInput {
        forward: 1.0,
        sprint: true,
        ..MovementInput::default()
    };
    let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
    sim.apply_player_packet(
        &PlayClientboundPacket::UpdateHealth(UpdateHealthPacket {
            health: 20.0,
            food_level: 6,
            saturation: 0.0,
        }),
        Some(1),
    );
    sim.tick(movement, CameraState::default(), 0);
    assert!(!sim.player().sprinting);
    sim.apply_player_packet(
        &PlayClientboundPacket::UpdateHealth(UpdateHealthPacket {
            health: 20.0,
            food_level: 20,
            saturation: 5.0,
        }),
        Some(1),
    );
    sim.apply_player_packet(
        &PlayClientboundPacket::EntityEffect(EntityEffectPacket {
            entity_id: 1,
            effect_id: 15,
            amplifier: 0,
            duration: 100,
            hide_particles: 0,
        }),
        Some(1),
    );
    sim.tick(movement, CameraState::default(), 0);
    assert!(!sim.player().sprinting);
    sim.apply_player_packet(
        &PlayClientboundPacket::RemoveEntityEffect(RemoveEntityEffectPacket {
            entity_id: 1,
            effect_id: 15,
        }),
        Some(1),
    );
    sim.tick(movement, CameraState::default(), 0);
    assert!(sim.player().sprinting);
}
#[test]
fn jump_boost_changes_jump_impulse_for_the_local_entity_only() {
    let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
    sim.apply_player_packet(
        &PlayClientboundPacket::EntityEffect(EntityEffectPacket {
            entity_id: 1,
            effect_id: 8,
            amplifier: 1,
            duration: 100,
            hide_particles: 0,
        }),
        Some(1),
    );
    sim.tick(
        MovementInput {
            jump: true,
            ..MovementInput::default()
        },
        CameraState::default(),
        0,
    );
    assert!((sim.player().position.y - (f64::from(0.42_f32) + f64::from(0.2_f32))).abs() < 1e-9);
}
#[test]
fn synchronized_attribute_modifiers_affect_ground_acceleration() {
    let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
    sim.apply_player_packet(
        &PlayClientboundPacket::EntityProperties(EntityPropertiesPacket {
            entity_id: 1,
            attributes: vec![EntityAttribute {
                name: "generic.movementSpeed".into(),
                base: 0.1,
                modifiers: vec![AttributeModifier {
                    uuid: [1; 16],
                    amount: 0.5,
                    operation: 2,
                }],
            }],
        }),
        Some(1),
    );
    sim.tick(
        MovementInput {
            forward: 1.0,
            ..MovementInput::default()
        },
        CameraState::default(),
        0,
    );
    assert!((sim.player().position.z - 0.147).abs() < 1e-6);
}
#[test]
fn server_flight_uses_flight_speed_and_vertical_drag() {
    let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
    sim.apply_player_packet(
        &PlayClientboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: 6,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }),
        Some(1),
    );
    sim.tick(
        MovementInput {
            jump: true,
            ..MovementInput::default()
        },
        CameraState::default(),
        0,
    );
    assert!((sim.player().position.y - f64::from(0.05_f32 * 3.0)).abs() < 1e-9);
    assert!((sim.velocity().y - f64::from(0.05_f32 * 3.0) * 0.6).abs() < 1e-9);
}

#[test]
fn explosion_adds_motion_instead_of_replacing_existing_velocity() {
    use rmc_game::player::Vec3;
    use rmc_game::simulation::AuthoritativePlayerState;
    let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
    sim.apply_player_packet(
        &PlayClientboundPacket::PlayerPositionAndLook(PlayerPositionAndLookPacket {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            flags: PositionLookFlags::from_bits(0),
        }),
        Some(1),
    );
    sim.apply_authoritative_state(AuthoritativePlayerState {
        position: Vec3::new(0.0, 64.0, 0.0),
        velocity: Vec3::new(0.1, 0.2, 0.3),
        on_ground: false,
    });
    sim.tick(MovementInput::default(), CameraState::default(), 0);
    let before = sim.velocity();
    sim.apply_event(rmc_game::simulation::SimulationEvent::AddVelocity(
        Vec3::new(0.25, 0.5, -0.25),
    ));
    assert_eq!(sim.velocity(), before.add(Vec3::new(0.25, 0.5, -0.25)));
}
