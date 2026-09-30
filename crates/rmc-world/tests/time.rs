use rmc_net::codec::play::{PlayClientboundPacket, TimeUpdatePacket};
use rmc_world::{WorldConfig, WorldSnapshot};
#[test]
fn frozen_daylight_keeps_total_time_ticking_and_positive_updates_resume_it() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    world
        .apply_play_packet(&PlayClientboundPacket::TimeUpdate(TimeUpdatePacket {
            total_world_time: 200,
            world_time: -6000,
        }))
        .unwrap();
    world.advance_time(3);
    assert_eq!(world.time().total_world_time, 203);
    assert_eq!(world.time().world_time, 6000);
    assert!(!world.time().daylight_cycle);
    world
        .apply_play_packet(&PlayClientboundPacket::TimeUpdate(TimeUpdatePacket {
            total_world_time: 300,
            world_time: 12000,
        }))
        .unwrap();
    world.advance_time(2);
    assert_eq!(world.time().world_time, 12002);
    assert!(world.time().daylight_cycle);
}

#[test]
fn frozen_zero_sentinel_and_java_long_overflow_are_preserved() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    world
        .apply_play_packet(&PlayClientboundPacket::TimeUpdate(TimeUpdatePacket {
            total_world_time: i64::MAX,
            world_time: -1,
        }))
        .unwrap();
    world.advance_time(1);
    assert_eq!(world.time().world_time, 1);
    assert_eq!(world.time().total_world_time, i64::MIN);
    world
        .apply_play_packet(&PlayClientboundPacket::TimeUpdate(TimeUpdatePacket {
            total_world_time: 0,
            world_time: i64::MAX - 1,
        }))
        .unwrap();
    world.advance_time(3);
    assert_eq!(world.time().world_time, i64::MIN);
    assert!(!world.time().daylight_cycle);
}
