use rmc_game::{
    camera::CameraState,
    input::MovementInput,
    simulation::{LocalSimulationLayer, SimulationConfig},
};
use rmc_net::codec::play::{PlayClientboundPacket, PlayerAbilitiesPacket};
fn tick(sim: &mut LocalSimulationLayer, jump: bool) {
    sim.tick(
        MovementInput {
            jump,
            ..MovementInput::default()
        },
        CameraState::default(),
        0,
    );
}
fn enabled() -> LocalSimulationLayer {
    let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
    sim.apply_player_packet(
        &PlayClientboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: 13,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }),
        Some(1),
    );
    sim
}
#[test]
fn second_jump_press_enables_flight_and_notifies_once() {
    let mut sim = enabled();
    tick(&mut sim, true);
    assert!(sim.take_ability_changes().is_empty());
    tick(&mut sim, false);
    tick(&mut sim, true);
    assert_eq!(
        sim.take_ability_changes(),
        vec![PlayerAbilitiesPacket {
            flags: 15,
            flying_speed: 0.05,
            walking_speed: 0.1
        }]
    );
    for _ in 0..3 {
        tick(&mut sim, true);
    }
    assert!(sim.take_ability_changes().is_empty());
    tick(&mut sim, false);
    tick(&mut sim, true);
    tick(&mut sim, false);
    tick(&mut sim, true);
    assert_eq!(sim.take_ability_changes()[0].flags, 13);
}
#[test]
fn expired_double_jump_and_server_denial_do_not_start_flight() {
    let mut sim = enabled();
    tick(&mut sim, true);
    for _ in 0..6 {
        tick(&mut sim, false);
    }
    tick(&mut sim, true);
    assert!(sim.take_ability_changes().is_empty());
    sim.apply_player_packet(
        &PlayClientboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: 0,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }),
        Some(1),
    );
    tick(&mut sim, false);
    tick(&mut sim, true);
    assert!(sim.take_ability_changes().is_empty());
}
#[test]
fn landing_ends_flight_and_preserves_authoritative_capability_bits() {
    let mut sim = enabled();
    sim.apply_player_packet(
        &PlayClientboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: 15,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }),
        Some(1),
    );
    tick(&mut sim, false);
    assert_eq!(sim.take_ability_changes()[0].flags, 13);
}

#[test]
fn spectator_scroll_changes_real_motion_and_clamps_without_ability_packet() {
    use rmc_net::codec::play::ChangeGameStatePacket;
    let mut baseline = LocalSimulationLayer::new(SimulationConfig::vanilla());
    let mut faster = LocalSimulationLayer::new(SimulationConfig::vanilla());
    assert_eq!(baseline.adjust_spectator_fly_speed(1), None);
    for sim in [&mut baseline, &mut faster] {
        sim.apply_player_packet(
            &PlayClientboundPacket::ChangeGameState(ChangeGameStatePacket {
                reason: 3,
                value: 3.0,
            }),
            Some(1),
        );
    }
    assert_eq!(
        faster.adjust_spectator_fly_speed(10),
        Some(0.05_f32 + 0.005_f32)
    );
    assert!(faster.take_ability_changes().is_empty());
    for sim in [&mut baseline, &mut faster] {
        sim.tick(
            MovementInput {
                forward: 1.0,
                ..Default::default()
            },
            CameraState::default(),
            0,
        );
    }
    assert!(faster.player().position.z > baseline.player().position.z);
    for _ in 0..100 {
        faster.adjust_spectator_fly_speed(1);
    }
    assert_eq!(faster.adjust_spectator_fly_speed(1), Some(0.2));
    for _ in 0..100 {
        faster.adjust_spectator_fly_speed(-1);
    }
    assert_eq!(faster.adjust_spectator_fly_speed(-1), Some(0.0));
}
