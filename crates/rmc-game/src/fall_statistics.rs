//! Entity.updateFallState and EntityPlayer.fall statistic accumulation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FallDistance(pub f32);
impl FallDistance {
    pub fn reset(&mut self) {
        self.0 = 0.0;
    }
    pub fn halve_in_lava(&mut self) {
        self.0 *= 0.5;
    }
    /// Landing consumes the prior accumulated distance; its final clipped step
    /// is excluded, matching Entity.updateFallState's on-ground branch.
    pub fn move_vertical(
        &mut self,
        delta_y: f64,
        on_ground: bool,
        allow_flying: bool,
    ) -> Option<i32> {
        if on_ground {
            if self.0 > 0.0 {
                let statistic = (!allow_flying && self.0 >= 2.0)
                    .then(|| (f64::from(self.0) * 100.0 + 0.5).floor() as i64 as i32);
                self.reset();
                return statistic;
            }
        } else if delta_y < 0.0 {
            self.0 = (f64::from(self.0) - delta_y) as f32;
        }
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn landing_threshold_clipped_last_step_and_resets() {
        let mut fall = FallDistance::default();
        assert_eq!(fall.move_vertical(-1.5, false, false), None);
        assert_eq!(fall.move_vertical(-1.0, true, false), None);
        assert_eq!(fall.0, 0.0);
        fall.move_vertical(-2.0, false, false);
        assert_eq!(fall.move_vertical(-0.9, true, false), Some(200));
        assert_eq!(fall.move_vertical(0.0, true, false), None);
        fall.move_vertical(-4.0, false, false);
        fall.halve_in_lava();
        assert_eq!(fall.0, 2.0);
        fall.reset();
        assert_eq!(fall.0, 0.0);
        fall.move_vertical(-5.0, false, true);
        assert_eq!(fall.move_vertical(-1.0, true, true), None);
        assert_eq!(fall.0, 0.0);
    }
    #[test]
    #[ignore = "requires local MCP919 fall statistics Java oracle"]
    fn local_java_fall_statistics_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/fall-statistics-java-oracle.log");
        let oracle = std::fs::read_to_string(path).unwrap();
        let mut cases = 0;
        for line in oracle.lines().filter(|line| line.starts_with("FALL|")) {
            let f: Vec<_> = line.split('|').collect();
            let mut state = FallDistance(f32::from_bits(u32::from_str_radix(f[1], 16).unwrap()));
            let amount = state.move_vertical(f[2].parse().unwrap(), f[3] == "1", f[4] == "1");
            assert_eq!(
                state.0.to_bits(),
                u32::from_str_radix(f[5], 16).unwrap(),
                "{line}"
            );
            let expected = if f[6] == "-" {
                None
            } else {
                Some(f[6].parse::<i32>().unwrap())
            };
            assert_eq!(amount, expected, "{line}");
            cases += 1;
        }
        assert_eq!(cases, 252);
        println!("{cases} MCP919 fall statistic cases match bit-for-bit");
    }
}
