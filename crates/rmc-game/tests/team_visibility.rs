use rmc_game::usability::UsabilityState;
use rmc_net::codec::play::{PlayClientboundPacket, TeamAction, TeamsPacket};
#[test]
fn friendly_visibility_requires_same_team_and_flag_two_and_tracks_membership_updates() {
    let mut state = UsabilityState::new();
    let mut team = TeamsPacket {
        name: "red".into(),
        action: TeamAction::Create,
        display_name: "Red".into(),
        prefix: "".into(),
        suffix: "".into(),
        friendly_flags: 2,
        name_tag_visibility: "always".into(),
        color: 12,
        players: vec!["viewer".into(), "target".into()],
    };
    assert!(!state.friendly_invisibles_visible("viewer", "target"));
    state.apply_play_packet(&PlayClientboundPacket::Teams(team.clone()));
    assert!(state.friendly_invisibles_visible("viewer", "target"));
    assert!(!state.friendly_invisibles_visible("outsider", "target"));
    team.action = TeamAction::Update;
    team.friendly_flags = 1;
    state.apply_play_packet(&PlayClientboundPacket::Teams(team.clone()));
    assert!(!state.friendly_invisibles_visible("viewer", "target"));
    team.friendly_flags = 2;
    state.apply_play_packet(&PlayClientboundPacket::Teams(team.clone()));
    team.action = TeamAction::RemovePlayers;
    team.players = vec!["target".into()];
    state.apply_play_packet(&PlayClientboundPacket::Teams(team));
    assert!(!state.friendly_invisibles_visible("viewer", "target"));
}
