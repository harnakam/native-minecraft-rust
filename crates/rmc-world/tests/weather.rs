use rmc_net::codec::play::{ChangeGameStatePacket, PlayClientboundPacket};
use rmc_world::{WorldConfig, WorldSnapshot};
#[test]
fn rain_events_reset_strength_and_thunder_is_scaled_by_rain() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    for (reason, value) in [(1, 0.0), (7, 0.5), (8, 0.25)] {
        world
            .apply_play_packet(&PlayClientboundPacket::ChangeGameState(
                ChangeGameStatePacket { reason, value },
            ))
            .unwrap();
    }
    assert!(world.weather().raining);
    assert_eq!(world.weather().rain_strength, 0.5);
    assert_eq!(world.weather().thunder_strength(), 0.125);
    world.advance_time(100);
    assert_eq!(
        world.weather().rain_strength,
        0.5,
        "WorldClient does not locally advance weather"
    );
    world
        .apply_play_packet(&PlayClientboundPacket::ChangeGameState(
            ChangeGameStatePacket {
                reason: 2,
                value: 0.0,
            },
        ))
        .unwrap();
    assert!(!world.weather().raining);
    assert_eq!(world.weather().rain_strength, 1.0);
}
