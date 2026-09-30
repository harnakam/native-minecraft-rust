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
