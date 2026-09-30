use rmc_game::usability::UsabilityState;
use rmc_net::codec::play::{PlayClientboundPacket, SetExperiencePacket};
#[test]
fn experience_updates_snapshot_and_resets_for_new_player() {
    let mut state = UsabilityState::new();
    state.apply_play_packet(&PlayClientboundPacket::SetExperience(SetExperiencePacket {
        progress: 0.5,
        level: 7,
        total: 128,
    }));
    let experience = state.snapshot().experience;
    assert_eq!(
        (experience.progress, experience.level, experience.total),
        (0.5, 7, 128)
    );
    state.reset_experience();
    assert_eq!(state.snapshot().experience, Default::default());
}
