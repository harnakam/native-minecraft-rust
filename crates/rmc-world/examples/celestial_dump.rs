use rmc_world::time::WorldTime;
fn main() {
    for time in 0..24000 {
        for partial in [0.0, 0.25, 0.5, 0.75] {
            let state = WorldTime {
                world_time: time,
                ..Default::default()
            };
            println!(
                "celestial {time} {partial:.2} {} {}",
                state.celestial_angle(partial).to_bits() as i32,
                state.moon_phase()
            );
        }
    }
}
