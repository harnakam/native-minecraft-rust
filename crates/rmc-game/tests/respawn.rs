use rmc_game::combat::{CombatConfig, CombatState};
use rmc_net::codec::play::{
    JoinGamePacket, PlayClientboundPacket, PlayServerboundPacket, RespawnPacket, UpdateHealthPacket,
};

#[test]
fn death_waits_twenty_ticks_then_sends_one_real_respawn_request() {
    let mut state = CombatState::new(CombatConfig::vanilla());
    assert!(state.request_respawn().is_none());
    state.apply_health_update(&UpdateHealthPacket {
        health: 0.0,
        food_level: 20,
        saturation: 5.0,
    });
    for _ in 0..19 {
        state.tick_feedback();
        assert!(state.request_respawn().is_none());
    }
    state.tick_feedback();
    assert_eq!(
        state.request_respawn(),
        Some(PlayServerboundPacket::ClientStatus(0))
    );
    assert!(state.request_respawn().is_none());
    state.apply_respawn(&RespawnPacket {
        dimension: 0,
        difficulty: 1,
        game_mode: 0,
        level_type: "default".into(),
    });
    assert_eq!(state.snapshot().health, 20.0);
    assert_eq!(state.snapshot().death_ticks, 0);
    assert!(!state.snapshot().respawn_requested);
    assert!(state.request_respawn().is_none());
}

#[test]
fn hardcore_death_does_not_send_a_survival_respawn() {
    let mut state = CombatState::new(CombatConfig::vanilla());
    state.apply_play_packet(
        &PlayClientboundPacket::JoinGame(JoinGamePacket {
            entity_id: 1,
            game_mode: 0,
            hardcore: true,
            dimension: 0,
            difficulty: 3,
            max_players: 20,
            level_type: "default".into(),
            reduced_debug_info: false,
        }),
        Some(1),
    );
    state.apply_health_update(&UpdateHealthPacket {
        health: 0.0,
        food_level: 20,
        saturation: 5.0,
    });
    for _ in 0..40 {
        state.tick_feedback();
    }
    assert!(state.request_respawn().is_none());
}

#[test]
fn respawn_creates_fresh_local_movement_and_inventory_state() {
    use rmc_game::{
        camera::CameraState,
        input::MovementInput,
        inventory::InventoryState,
        simulation::{LocalSimulationLayer, SimulationConfig},
    };
    use rmc_net::codec::play::{EntityEffectPacket, PlayerAbilitiesPacket};
    let mut sim = LocalSimulationLayer::new(SimulationConfig::vanilla());
    sim.apply_player_packet(
        &PlayClientboundPacket::EntityEffect(EntityEffectPacket {
            entity_id: 1,
            effect_id: 8,
            amplifier: 2,
            duration: 100,
            hide_particles: 0,
        }),
        Some(1),
    );
    sim.apply_player_packet(
        &PlayClientboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
            flags: 6,
            flying_speed: 0.2,
            walking_speed: 0.4,
        }),
        Some(1),
    );
    sim.tick(
        MovementInput {
            forward: 1.0,
            sprint: true,
            jump: true,
            ..MovementInput::default()
        },
        CameraState::default(),
        8,
    );
    sim.apply_player_packet(
        &PlayClientboundPacket::Respawn(RespawnPacket {
            dimension: 0,
            difficulty: 1,
            game_mode: 0,
            level_type: "default".into(),
        }),
        Some(1),
    );
    assert_eq!(sim.effect_amplifier(8), None);
    assert_eq!(sim.velocity(), rmc_game::player::Vec3::ZERO);
    assert_eq!(sim.player().selected_hotbar_slot, 0);
    assert!(!sim.player().sprinting);
    sim.tick(
        MovementInput {
            jump: true,
            ..MovementInput::default()
        },
        CameraState::default(),
        0,
    );
    assert!((sim.player().position.y - f64::from(0.42_f32)).abs() < 1e-9);
    let mut inventory = InventoryState::new();
    inventory.open_player_inventory();
    inventory.sync_selected_hotbar_slot(8);
    inventory.queue_pickup_click(0, 36, 0);
    inventory.reset_for_respawn();
    assert!(inventory.open_window().is_none());
    assert_eq!(inventory.selected_hotbar_slot(), 0);
    assert!(inventory
        .inventory_window()
        .slots
        .iter()
        .all(Option::is_none));
    assert!(inventory.pending_transactions().is_empty());
}
