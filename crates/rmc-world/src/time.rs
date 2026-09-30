//! WorldClient time transitions, independent of render frame pacing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldTime {
    pub total_world_time: i64,
    pub world_time: i64,
    pub daylight_cycle: bool,
}
impl Default for WorldTime {
    fn default() -> Self {
        Self {
            total_world_time: 0,
            world_time: 0,
            daylight_cycle: true,
        }
    }
}
impl WorldTime {
    pub(crate) fn receive(&mut self, total: i64, time: i64) {
        self.total_world_time = total;
        self.daylight_cycle = time >= 0;
        self.world_time = if time < 0 { time.wrapping_neg() } else { time };
    }
    pub(crate) fn advance(&mut self, ticks: usize) {
        self.total_world_time = self.total_world_time.wrapping_add(ticks as i64);
        for _ in 0..ticks {
            if !self.daylight_cycle {
                break;
            }
            self.world_time = self.world_time.wrapping_add(1);
            // WorldClient.setWorldTime also updates the game rule after overflow.
            if self.world_time < 0 {
                self.world_time = self.world_time.wrapping_neg();
                self.daylight_cycle = false;
            }
        }
    }
}
