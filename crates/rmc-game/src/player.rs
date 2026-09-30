//! Local player shell state and movement-packet emission.

use crate::camera::CameraState;
use crate::input::MovementInput;
use rmc_net::codec::play::{
    PlayServerboundPacket, PlayerLookPacket, PlayerPacket, PlayerPositionLookServerboundPacket,
    PlayerPositionPacket,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn squared_distance_to(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    pub fn scale(self, factor: f64) -> Self {
        Self {
            x: self.x * factor,
            y: self.y * factor,
            z: self.z * factor,
        }
    }

    pub fn lerp(self, other: Self, alpha: f32) -> Self {
        let alpha = f64::from(alpha.clamp(0.0, 1.0));
        Self {
            x: self.x + (other.x - self.x) * alpha,
            y: self.y + (other.y - self.y) * alpha,
            z: self.z + (other.z - self.z) * alpha,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalPlayerState {
    pub position: Vec3,
    pub on_ground: bool,
    pub sprinting: bool,
    pub sneaking: bool,
    pub selected_hotbar_slot: u8,
}

impl Default for LocalPlayerState {
    fn default() -> Self {
        Self {
            position: Vec3::default(),
            on_ground: true,
            sprinting: false,
            sneaking: false,
            selected_hotbar_slot: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellLocomotionConfig {
    pub walk_speed_per_tick: f64,
    pub sprint_multiplier: f64,
    pub sneak_multiplier: f64,
}

impl ShellLocomotionConfig {
    pub fn vanilla_shell() -> Self {
        Self {
            walk_speed_per_tick: 0.1,
            sprint_multiplier: 1.3,
            sneak_multiplier: 0.3,
        }
    }
}

impl LocalPlayerState {
    pub fn apply_shell_movement(
        &mut self,
        movement: MovementInput,
        camera: CameraState,
        config: ShellLocomotionConfig,
    ) {
        self.sprinting = movement.sprint;
        self.sneaking = movement.sneak;

        let mut speed = config.walk_speed_per_tick;

        if movement.sprint {
            speed *= config.sprint_multiplier;
        }

        if movement.sneak {
            speed *= config.sneak_multiplier;
        }

        let mut forward = f64::from(movement.forward);
        let mut strafe = f64::from(movement.strafe);
        let magnitude = (forward * forward + strafe * strafe).sqrt();

        if magnitude > 1.0 {
            forward /= magnitude;
            strafe /= magnitude;
        }

        let yaw_radians = f64::from(camera.yaw).to_radians();
        let sin = yaw_radians.sin();
        let cos = yaw_radians.cos();

        self.position.x += (forward * -sin + strafe * cos) * speed;
        self.position.z += (forward * cos + strafe * sin) * speed;

        if movement.jump {
            self.position.y += speed;
            self.on_ground = false;
        } else {
            self.on_ground = true;
        }
    }
}

pub struct WalkingPacketEmitter {
    last_sent_position: Vec3,
    last_sent_yaw: f32,
    last_sent_pitch: f32,
    ticks_since_last_position_packet: u32,
}

impl Default for WalkingPacketEmitter {
    fn default() -> Self {
        Self {
            last_sent_position: Vec3::default(),
            last_sent_yaw: 0.0,
            last_sent_pitch: 0.0,
            ticks_since_last_position_packet: 0,
        }
    }
}

impl WalkingPacketEmitter {
    pub fn next_packet(
        &mut self,
        player: &LocalPlayerState,
        camera: CameraState,
    ) -> PlayServerboundPacket {
        let position_changed = player.position.squared_distance_to(self.last_sent_position)
            > 9.0e-4
            || self.ticks_since_last_position_packet >= 20;
        let rotation_changed =
            camera.yaw != self.last_sent_yaw || camera.pitch != self.last_sent_pitch;

        let packet = match (position_changed, rotation_changed) {
            (true, true) => {
                PlayServerboundPacket::PlayerPositionAndLook(PlayerPositionLookServerboundPacket {
                    x: player.position.x,
                    y: player.position.y,
                    z: player.position.z,
                    yaw: camera.yaw,
                    pitch: camera.pitch,
                    on_ground: player.on_ground,
                })
            }
            (true, false) => PlayServerboundPacket::PlayerPosition(PlayerPositionPacket {
                x: player.position.x,
                y: player.position.y,
                z: player.position.z,
                on_ground: player.on_ground,
            }),
            (false, true) => PlayServerboundPacket::PlayerLook(PlayerLookPacket {
                yaw: camera.yaw,
                pitch: camera.pitch,
                on_ground: player.on_ground,
            }),
            (false, false) => PlayServerboundPacket::Player(PlayerPacket {
                on_ground: player.on_ground,
            }),
        };

        if position_changed {
            self.last_sent_position = player.position;
            self.ticks_since_last_position_packet = 0;
        } else {
            self.ticks_since_last_position_packet += 1;
        }

        if rotation_changed {
            self.last_sent_yaw = camera.yaw;
            self.last_sent_pitch = camera.pitch;
        }

        packet
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalPlayerState, ShellLocomotionConfig, Vec3, WalkingPacketEmitter};
    use crate::camera::CameraState;
    use crate::input::MovementInput;
    use rmc_net::codec::play::PlayServerboundPacket;

    #[test]
    fn emits_position_and_look_when_both_change() {
        let mut player = LocalPlayerState::default();
        let camera = CameraState {
            yaw: 45.0,
            pitch: 0.0,
            ..CameraState::default()
        };
        player.position = Vec3 {
            x: 1.0,
            y: 64.0,
            z: 1.0,
        };

        let packet = WalkingPacketEmitter::default().next_packet(&player, camera);
        assert!(matches!(
            packet,
            PlayServerboundPacket::PlayerPositionAndLook(_)
        ));
    }

    #[test]
    fn applies_shell_movement_from_camera_yaw() {
        let mut player = LocalPlayerState::default();
        player.apply_shell_movement(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState {
                yaw: 0.0,
                ..CameraState::default()
            },
            ShellLocomotionConfig::vanilla_shell(),
        );

        assert!(player.position.z > 0.0);
    }
}
