//! Camera orientation and mouse-capture state for the M2 shell.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseSettings {
    pub sensitivity: f32,
    pub invert_y: bool,
}

impl MouseSettings {
    pub fn vanilla() -> Self {
        Self {
            sensitivity: 0.5,
            invert_y: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CameraCaptureState {
    pub mouse_captured: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraState {
    pub yaw: f32,
    pub pitch: f32,
    pub capture: CameraCaptureState,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            capture: CameraCaptureState {
                mouse_captured: false,
            },
        }
    }
}

impl CameraState {
    pub fn set_mouse_captured(&mut self, captured: bool) {
        self.capture.mouse_captured = captured;
    }

    pub fn apply_mouse_delta(
        &mut self,
        mouse_delta_x: f32,
        mouse_delta_y: f32,
        settings: MouseSettings,
    ) {
        if !self.capture.mouse_captured {
            return;
        }

        let base = settings.sensitivity.clamp(0.0, 1.0) * 0.6 + 0.2;
        let multiplier = base * base * base * 8.0;
        let vertical = if settings.invert_y {
            -mouse_delta_y
        } else {
            mouse_delta_y
        };

        self.yaw += mouse_delta_x * multiplier;
        self.pitch = (self.pitch - vertical * multiplier).clamp(-90.0, 90.0);
    }
}

#[cfg(test)]
mod tests {
    use super::{CameraState, MouseSettings};

    #[test]
    fn ignores_mouse_delta_when_not_captured() {
        let mut camera = CameraState::default();
        camera.apply_mouse_delta(10.0, 10.0, MouseSettings::vanilla());
        assert_eq!(camera.yaw, 0.0);
        assert_eq!(camera.pitch, 0.0);
    }

    #[test]
    fn clamps_pitch() {
        let mut camera = CameraState::default();
        camera.set_mouse_captured(true);
        camera.apply_mouse_delta(0.0, -500.0, MouseSettings::vanilla());
        assert_eq!(camera.pitch, 90.0);
    }
}
