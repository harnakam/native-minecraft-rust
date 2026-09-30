//! Numerical behavior used by the vanilla 1.8.9 movement implementation.
use std::sync::OnceLock;

fn table() -> &'static [f32; 65536] {
    static TABLE: OnceLock<Box<[f32; 65536]>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let values: Vec<f32> = (0..65536)
            .map(|i| ((i as f64) * std::f64::consts::PI * 2.0 / 65536.0).sin() as f32)
            .collect();
        values.into_boxed_slice().try_into().unwrap()
    })
}

pub fn sin(angle: f32) -> f32 {
    table()[((angle * 10430.378_f32) as i32 & 65535) as usize]
}

pub fn cos(angle: f32) -> f32 {
    table()[((angle * 10430.378_f32 + 16384.0_f32) as i32 & 65535) as usize]
}

pub fn move_flying(strafe: f32, forward: f32, acceleration: f32, yaw: f32) -> (f64, f64) {
    let squared = strafe * strafe + forward * forward;
    if squared < 1.0e-4_f32 {
        return (0.0, 0.0);
    }
    let divisor = (f64::from(squared).sqrt() as f32).max(1.0);
    let factor = acceleration / divisor;
    let strafe = strafe * factor;
    let forward = forward * factor;
    let angle = yaw * std::f32::consts::PI / 180.0_f32;
    let (sine, cosine) = (sin(angle), cos(angle));
    (
        f64::from(strafe * cosine - forward * sine),
        f64::from(forward * cosine + strafe * sine),
    )
}
