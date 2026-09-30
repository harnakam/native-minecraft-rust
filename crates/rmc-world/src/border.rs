//! Client WorldBorder state with Java millisecond and float interpolation semantics.
use rmc_net::codec::play::WorldBorderPacket;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_else(|e| -(e.duration().as_millis() as i64))
}

#[derive(Clone, Debug)]
pub struct WorldBorder {
    pub center: [f64; 2],
    pub world_size: i32,
    pub warning_time: i32,
    pub warning_distance: i32,
    from: f64,
    to: f64,
    start: i64,
    end: i64,
}
impl Default for WorldBorder {
    fn default() -> Self {
        Self {
            center: [0.0; 2],
            world_size: 29999984,
            warning_time: 15,
            warning_distance: 5,
            from: 60000000.0,
            to: 60000000.0,
            start: 0,
            end: 0,
        }
    }
}
impl WorldBorder {
    fn transition(&mut self, from: f64, to: f64, milliseconds: i64, now: i64) {
        self.from = from;
        self.to = to;
        self.start = now;
        self.end = now.wrapping_add(milliseconds);
    }
    pub fn receive_at(&mut self, packet: &WorldBorderPacket, now: i64) {
        match *packet {
            WorldBorderPacket::SetSize { diameter } => self.transition(diameter, diameter, 0, now),
            WorldBorderPacket::LerpSize {
                from,
                to,
                milliseconds,
            } => self.transition(from, to, milliseconds, now),
            WorldBorderPacket::SetCenter { x, z } => self.center = [x, z],
            WorldBorderPacket::Initialize {
                x,
                z,
                from,
                to,
                milliseconds,
                size,
                warning_distance,
                warning_time,
            } => {
                self.center = [x, z];
                if milliseconds > 0 {
                    self.transition(from, to, milliseconds, now);
                } else {
                    self.transition(to, to, 0, now);
                }
                self.world_size = size;
                self.warning_distance = warning_distance;
                self.warning_time = warning_time;
            }
            WorldBorderPacket::SetWarningTime(value) => self.warning_time = value,
            WorldBorderPacket::SetWarningBlocks(value) => self.warning_distance = value,
        }
    }
    pub fn diameter_at(&mut self, now: i64) -> f64 {
        if self.from != self.to {
            let fraction =
                (now.wrapping_sub(self.start) as f32) / (self.end.wrapping_sub(self.start) as f32);
            if fraction < 1.0 {
                return self.from + (self.to - self.from) * f64::from(fraction);
            }
            self.transition(self.to, self.to, 0, now);
        }
        self.from
    }
    /// [minimum X, maximum X, minimum Z, maximum Z].
    pub fn bounds_at(&mut self, now: i64) -> [f64; 4] {
        let half = self.diameter_at(now) / 2.0;
        let low = f64::from(self.world_size.wrapping_neg());
        let high = f64::from(self.world_size);
        [
            java_max(self.center[0] - half, low),
            java_min(self.center[0] + half, high),
            java_max(self.center[1] - half, low),
            java_min(self.center[1] + half, high),
        ]
    }
    pub fn warning_strength_at(&mut self, x: f64, z: f64, now: i64) -> f32 {
        let distance = self.closest_distance_at(x, z, now) as f32;
        let speed = if self.end == self.start {
            0.0
        } else {
            (self.from - self.to).abs() / self.end.wrapping_sub(self.start) as f64
        };
        let moving_distance = math_min(
            speed * f64::from(self.warning_time) * 1000.0,
            (self.to - self.diameter_at(now)).abs(),
        );
        let threshold = if moving_distance.is_nan() {
            f64::NAN
        } else {
            f64::from(self.warning_distance).max(moving_distance)
        };
        if f64::from(distance) < threshold {
            1.0 - (f64::from(distance) / threshold) as f32
        } else {
            0.0
        }
    }

    pub fn closest_distance_at(&mut self, x: f64, z: f64, now: i64) -> f64 {
        let [min_x, max_x, min_z, max_z] = self.bounds_at(now);
        math_min(
            math_min(math_min(x - min_x, max_x - x), z - min_z),
            max_z - z,
        )
    }
}
// WorldBorder uses conditional clamps, which retain NaN instead of replacing it.
fn java_max(value: f64, bound: f64) -> f64 {
    if value < bound {
        bound
    } else {
        value
    }
}
fn java_min(value: f64, bound: f64) -> f64 {
    if value > bound {
        bound
    } else {
        value
    }
}

fn math_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == 0.0 && b == 0.0 {
        return if a.is_sign_negative() || b.is_sign_negative() {
            -0.0
        } else {
            0.0
        };
    }
    if a <= b {
        a
    } else {
        b
    }
}
