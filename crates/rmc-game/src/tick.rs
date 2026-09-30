//! Fixed-step timer used by the M2 main-thread shell.

use std::time::Duration;

pub const VANILLA_TICK_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FixedStepConfig {
    pub tick_interval: Duration,
    pub max_frame_time: Duration,
    pub max_ticks_per_frame: usize,
}

impl FixedStepConfig {
    pub fn vanilla() -> Self {
        Self {
            tick_interval: VANILLA_TICK_INTERVAL,
            max_frame_time: Duration::from_millis(250),
            max_ticks_per_frame: 10,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePacingConfig {
    pub smoothing_factor: f32,
    pub max_adjustment_ratio: f32,
    pub spike_reset_ratio: f32,
}

impl FramePacingConfig {
    pub fn vanilla() -> Self {
        Self {
            smoothing_factor: 0.35,
            max_adjustment_ratio: 0.20,
            spike_reset_ratio: 3.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FramePacingSnapshot {
    pub raw_frame_time: Duration,
    pub paced_frame_time: Duration,
    pub smoothed_frame_time: Duration,
    pub jitter_ms: f32,
    pub spike_detected: bool,
}

pub struct FramePacer {
    config: FramePacingConfig,
    smoothed_frame_time: Option<Duration>,
}

impl FramePacer {
    pub fn new(config: FramePacingConfig) -> Self {
        Self {
            config,
            smoothed_frame_time: None,
        }
    }

    pub fn pace(&mut self, raw_frame_time: Duration) -> FramePacingSnapshot {
        let raw_secs = raw_frame_time.as_secs_f32();
        let previous_secs = self
            .smoothed_frame_time
            .unwrap_or(raw_frame_time)
            .as_secs_f32();
        let spike_detected =
            previous_secs > 0.0 && raw_secs > previous_secs * self.config.spike_reset_ratio;
        let smoothed_secs = if spike_detected {
            raw_secs
        } else {
            previous_secs
                + (raw_secs - previous_secs) * self.config.smoothing_factor.clamp(0.0, 1.0)
        };

        let max_adjustment = raw_secs * self.config.max_adjustment_ratio.max(0.0);
        let paced_secs = if max_adjustment <= f32::EPSILON {
            raw_secs
        } else {
            smoothed_secs.clamp(raw_secs - max_adjustment, raw_secs + max_adjustment)
        };
        let smoothed_frame_time = Duration::from_secs_f32(smoothed_secs.max(0.0));
        let paced_frame_time = Duration::from_secs_f32(paced_secs.max(0.0));
        self.smoothed_frame_time = Some(smoothed_frame_time);

        FramePacingSnapshot {
            raw_frame_time,
            paced_frame_time,
            smoothed_frame_time,
            jitter_ms: (raw_secs - paced_secs).abs() * 1000.0,
            spike_detected,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepResult {
    pub ticks_to_run: usize,
    pub interpolation_alpha: f32,
    pub clamped_frame_time: Duration,
    pub total_ticks: u64,
}

pub struct FixedStepTimer {
    config: FixedStepConfig,
    accumulator: Duration,
    total_ticks: u64,
}

impl FixedStepTimer {
    pub fn new(config: FixedStepConfig) -> Self {
        Self {
            config,
            accumulator: Duration::ZERO,
            total_ticks: 0,
        }
    }

    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }

    pub fn advance(&mut self, frame_delta: Duration) -> StepResult {
        let clamped_frame_time = frame_delta.min(self.config.max_frame_time);
        self.accumulator += clamped_frame_time;

        let mut ticks_to_run = 0usize;

        while self.accumulator >= self.config.tick_interval
            && ticks_to_run < self.config.max_ticks_per_frame
        {
            self.accumulator -= self.config.tick_interval;
            ticks_to_run += 1;
            self.total_ticks += 1;
        }

        if ticks_to_run == self.config.max_ticks_per_frame
            && self.accumulator > self.config.tick_interval
        {
            self.accumulator = self.config.tick_interval;
        }

        StepResult {
            ticks_to_run,
            interpolation_alpha: self.accumulator.as_secs_f32()
                / self.config.tick_interval.as_secs_f32(),
            clamped_frame_time,
            total_ticks: self.total_ticks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        FixedStepConfig, FixedStepTimer, FramePacer, FramePacingConfig, VANILLA_TICK_INTERVAL,
    };
    use std::time::Duration;

    fn assert_duration_close(actual: Duration, expected: Duration) {
        let delta = actual.abs_diff(expected);
        assert!(
            delta <= Duration::from_micros(1),
            "expected {expected:?}, got {actual:?}, delta {delta:?}"
        );
    }

    #[test]
    fn advances_in_20_tps_steps() {
        let mut timer = FixedStepTimer::new(FixedStepConfig::vanilla());
        let first = timer.advance(Duration::from_millis(16));
        assert_eq!(first.ticks_to_run, 0);

        let second = timer.advance(Duration::from_millis(34));
        assert_eq!(second.ticks_to_run, 1);
        assert_eq!(second.total_ticks, 1);
    }

    #[test]
    fn clamps_large_frames() {
        let mut timer = FixedStepTimer::new(FixedStepConfig {
            tick_interval: VANILLA_TICK_INTERVAL,
            max_frame_time: Duration::from_millis(100),
            max_ticks_per_frame: 2,
        });

        let result = timer.advance(Duration::from_secs(1));
        assert_eq!(result.clamped_frame_time, Duration::from_millis(100));
        assert_eq!(result.ticks_to_run, 2);
    }

    #[test]
    fn frame_pacer_smooths_small_jitter() {
        let mut pacer = FramePacer::new(FramePacingConfig::vanilla());
        let baseline = pacer.pace(Duration::from_millis(16));
        let jittered = pacer.pace(Duration::from_millis(20));

        assert_duration_close(baseline.paced_frame_time, Duration::from_millis(16));
        assert!(jittered.paced_frame_time < Duration::from_millis(20));
        assert!(jittered.jitter_ms > 0.0);
    }

    #[test]
    fn frame_pacer_resets_on_spike() {
        let mut pacer = FramePacer::new(FramePacingConfig::vanilla());
        pacer.pace(Duration::from_millis(16));
        let spike = pacer.pace(Duration::from_millis(80));

        assert!(spike.spike_detected);
        assert_duration_close(spike.paced_frame_time, Duration::from_millis(80));
    }
}
