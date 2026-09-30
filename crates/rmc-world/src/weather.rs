//! Received WorldClient weather: packet setters update current and previous values equally.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Weather {
    pub raining: bool,
    pub rain_strength: f32,
    pub raw_thunder_strength: f32,
}
impl Weather {
    pub fn thunder_strength(self) -> f32 {
        self.raw_thunder_strength * self.rain_strength
    }
    pub(crate) fn receive(&mut self, reason: u8, value: f32) {
        match reason {
            1 => {
                self.raining = true;
                self.rain_strength = 0.0;
            }
            2 => {
                self.raining = false;
                self.rain_strength = 1.0;
            }
            7 if value.is_finite() => self.rain_strength = value,
            8 if value.is_finite() => self.raw_thunder_strength = value,
            _ => {}
        }
    }
}
