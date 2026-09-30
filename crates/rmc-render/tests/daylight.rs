use rmc_render::daylight::Daylight;
use rmc_world::time::WorldTime;
#[test]
fn noon_and_midnight_change_sky_but_preserve_block_light() {
    let noon = Daylight::calculate(
        WorldTime {
            world_time: 6000,
            ..Default::default()
        },
        0.0,
        0.0,
        0.0,
    );
    let night = Daylight::calculate(
        WorldTime {
            world_time: 18000,
            ..Default::default()
        },
        0.0,
        0.0,
        0.0,
    );
    assert_eq!(noon.skylight_subtracted, 0);
    assert_eq!(night.skylight_subtracted, 11);
    assert_eq!(noon.sun_brightness, 1.0);
    assert_eq!(night.sun_brightness, 0.2);
    assert_eq!(night.sky_color_multiplier, 0.0);
}

#[test]
fn rain_and_scaled_thunder_reduce_noon_brightness() {
    let time = WorldTime {
        world_time: 6000,
        ..Default::default()
    };
    let wet = Daylight::calculate(time, 0.0, 1.0, 1.0);
    assert_eq!(wet.sun_brightness, 0.578125);
    assert_eq!(wet.skylight_subtracted, 5);
}
