//! World daylight coefficients corresponding to MCP919 World brightness methods.
use rmc_world::time::WorldTime;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Daylight {
    pub sun_brightness: f32,
    pub skylight_subtracted: u8,
    pub sky_color_multiplier: f32,
}
impl Daylight {
    pub fn calculate(time: WorldTime, partial: f32, rain: f32, thunder: f32) -> Self {
        let cosine =
            rmc_game::math::cos(time.celestial_angle(partial) * std::f32::consts::PI * 2.0);
        let weather = |mut value: f32| {
            value = (f64::from(value) * (1.0 - f64::from(rain * 5.0) / 16.0)) as f32;
            (f64::from(value) * (1.0 - f64::from(thunder * 5.0) / 16.0)) as f32
        };
        let sky_color_multiplier = (cosine * 2.0 + 0.5).clamp(0.0, 1.0);
        let sun_brightness =
            weather(1.0 - (1.0 - (cosine * 2.0 + 0.2)).clamp(0.0, 1.0)) * 0.8 + 0.2;
        let skylight_subtracted =
            ((1.0 - weather(1.0 - (1.0 - (cosine * 2.0 + 0.5)).clamp(0.0, 1.0))) * 11.0) as u8;
        Self {
            sun_brightness,
            skylight_subtracted,
            sky_color_multiplier,
        }
    }
}
