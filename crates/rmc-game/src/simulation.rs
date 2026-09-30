//! Local simulation layer between network state and render state.

use crate::camera::CameraState;
use crate::input::MovementInput;
use crate::player::{LocalPlayerState, ShellLocomotionConfig, Vec3};
use rmc_world::{collision::Aabb, BlockPos, WorldSnapshot};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuthoritativePlayerState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
}

impl Default for AuthoritativePlayerState {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            on_ground: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KnockbackImpulse {
    pub velocity: Vec3,
    pub resets_sprint: bool,
}

impl KnockbackImpulse {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            velocity: Vec3::new(x, y, z),
            resets_sprint: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimulationEvent {
    AuthoritativeState(AuthoritativePlayerState),
    Knockback(KnockbackImpulse),
    Teleport {
        position: Vec3,
        yaw: f32,
        pitch: f32,
        flags: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionBounds {
    pub min_x: f64,
    pub max_x: f64,
    pub min_z: f64,
    pub max_z: f64,
    pub floor_y: f64,
    pub ceiling_y: f64,
}

impl CollisionBounds {
    pub fn vanilla_shell_arena() -> Self {
        Self {
            min_x: -128.0,
            max_x: 128.0,
            min_z: -128.0,
            max_z: 128.0,
            floor_y: 0.0,
            ceiling_y: 256.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationConfig {
    pub locomotion: ShellLocomotionConfig,
    pub ground_acceleration: f64,
    pub air_acceleration: f64,
    pub ground_friction: f64,
    pub air_friction: f64,
    pub gravity_per_tick: f64,
    pub vertical_drag: f64,
    pub jump_velocity: f64,
    pub network_blend_factor: f32,
    pub network_snap_distance: f64,
    pub sprint_reset_ticks: u8,
    pub collision: CollisionBounds,
}

impl SimulationConfig {
    pub fn vanilla() -> Self {
        Self {
            locomotion: ShellLocomotionConfig::vanilla_shell(),
            ground_acceleration: 0.16277136,
            air_acceleration: f64::from(0.02_f32),
            ground_friction: f64::from(0.6_f32 * 0.91_f32),
            air_friction: f64::from(0.91_f32),
            gravity_per_tick: 0.08,
            vertical_drag: f64::from(0.98_f32),
            jump_velocity: f64::from(0.42_f32),
            network_blend_factor: 0.25,
            network_snap_distance: 2.0,
            sprint_reset_ticks: 1,
            collision: CollisionBounds::vanilla_shell_arena(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterpolationState {
    pub previous_position: Vec3,
    pub current_position: Vec3,
}

impl InterpolationState {
    pub fn new(position: Vec3) -> Self {
        Self {
            previous_position: position,
            current_position: position,
        }
    }

    pub fn sample_position(&self, alpha: f32) -> Vec3 {
        self.previous_position.lerp(self.current_position, alpha)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimulationSnapshot {
    pub network: AuthoritativePlayerState,
    pub player: LocalPlayerState,
    pub velocity: Vec3,
    pub interpolation: InterpolationState,
    pub sprint_reset_ticks: u8,
}

impl SimulationSnapshot {
    pub fn render_position(&self, alpha: f32) -> Vec3 {
        self.interpolation.sample_position(alpha)
    }
}

pub struct LocalSimulationLayer {
    config: SimulationConfig,
    network: AuthoritativePlayerState,
    player: LocalPlayerState,
    velocity: Vec3,
    interpolation: InterpolationState,
    pending_knockback: Option<Vec3>,
    pending_server_state: Option<AuthoritativePlayerState>,
    sprint_reset_ticks: u8,
}

impl LocalSimulationLayer {
    pub fn new(config: SimulationConfig) -> Self {
        let initial_position = Vec3::new(0.0, config.collision.floor_y, 0.0);
        let network = AuthoritativePlayerState {
            position: initial_position,
            ..AuthoritativePlayerState::default()
        };
        let player = LocalPlayerState {
            position: initial_position,
            ..LocalPlayerState::default()
        };

        Self {
            config,
            network,
            player,
            velocity: Vec3::ZERO,
            interpolation: InterpolationState::new(initial_position),
            pending_knockback: None,
            pending_server_state: None,
            sprint_reset_ticks: 0,
        }
    }

    pub fn apply_event(&mut self, event: SimulationEvent) {
        match event {
            SimulationEvent::AuthoritativeState(state) => self.apply_authoritative_state(state),
            SimulationEvent::Knockback(impulse) => self.apply_knockback(impulse),
            SimulationEvent::Teleport {
                position, flags, ..
            } => {
                self.apply_pending_server_state();
                self.apply_pending_knockback();
                let previous = self.player.position;
                let resolved = Vec3::new(
                    position.x + if flags & 1 != 0 { previous.x } else { 0.0 },
                    position.y + if flags & 2 != 0 { previous.y } else { 0.0 },
                    position.z + if flags & 4 != 0 { previous.z } else { 0.0 },
                );
                let velocity = Vec3::new(
                    if flags & 1 != 0 { self.velocity.x } else { 0.0 },
                    if flags & 2 != 0 { self.velocity.y } else { 0.0 },
                    if flags & 4 != 0 { self.velocity.z } else { 0.0 },
                );
                self.apply_authoritative_state(AuthoritativePlayerState {
                    position: resolved,
                    velocity,
                    on_ground: false,
                });
                self.apply_pending_server_state();
                self.interpolation.current_position = resolved;
            }
        }
    }

    pub fn apply_authoritative_state(&mut self, state: AuthoritativePlayerState) {
        self.network = state;
        self.pending_server_state = Some(state);
    }

    pub fn apply_knockback(&mut self, impulse: KnockbackImpulse) {
        self.pending_knockback = Some(impulse.velocity);

        if impulse.resets_sprint {
            self.sprint_reset_ticks = self.config.sprint_reset_ticks;
            self.player.sprinting = false;
        }
    }

    pub fn tick(&mut self, movement: MovementInput, camera: CameraState, selected_hotbar_slot: u8) {
        self.tick_internal(movement, camera, selected_hotbar_slot, None);
    }

    pub fn tick_with_world(
        &mut self,
        movement: MovementInput,
        camera: CameraState,
        selected_hotbar_slot: u8,
        world: &WorldSnapshot,
    ) {
        self.tick_internal(movement, camera, selected_hotbar_slot, Some(world));
    }

    fn tick_internal(
        &mut self,
        movement: MovementInput,
        camera: CameraState,
        selected_hotbar_slot: u8,
        world: Option<&WorldSnapshot>,
    ) {
        self.interpolation.previous_position = self.player.position;
        self.player.selected_hotbar_slot = selected_hotbar_slot;
        self.apply_pending_server_state();
        self.apply_pending_knockback();
        if let Some(world) = world {
            self.config.ground_friction = f64::from(
                world.slipperiness_at(BlockPos::new(
                    self.player.position.x.floor() as i32,
                    self.player.position.y.floor() as i32 - 1,
                    self.player.position.z.floor() as i32,
                )) * 0.91_f32,
            );
        }
        for motion in [
            &mut self.velocity.x,
            &mut self.velocity.y,
            &mut self.velocity.z,
        ] {
            if motion.abs() < 0.005 {
                *motion = 0.0;
            }
        }
        let friction = if self.player.on_ground {
            self.config.ground_friction
        } else {
            self.config.air_friction
        };
        self.apply_horizontal_input(movement, camera);
        self.apply_jump(movement, camera);
        if let Some(world) = world {
            self.resolve_terrain(world);
        } else {
            self.resolve_collisions();
        }
        self.integrate_vertical_motion();
        if let Some(world) = world {
            let pos = BlockPos::new(
                self.player.position.x.floor() as i32,
                0,
                self.player.position.z.floor() as i32,
            );
            if world.chunk(pos.chunk_pos()).is_none() {
                self.velocity.y = if self.player.position.y > 0.0 {
                    -0.1 * self.config.vertical_drag
                } else {
                    0.0
                };
            }
        }
        self.velocity.x *= friction;
        self.velocity.z *= friction;
        self.interpolation.current_position = self.player.position;

        if self.sprint_reset_ticks > 0 {
            self.sprint_reset_ticks -= 1;
        }
    }

    pub fn snapshot(&self) -> SimulationSnapshot {
        SimulationSnapshot {
            network: self.network,
            player: self.player.clone(),
            velocity: self.velocity,
            interpolation: self.interpolation,
            sprint_reset_ticks: self.sprint_reset_ticks,
        }
    }

    pub fn player(&self) -> &LocalPlayerState {
        &self.player
    }

    pub fn velocity(&self) -> Vec3 {
        self.velocity
    }

    fn apply_pending_server_state(&mut self) {
        let Some(state) = self.pending_server_state.take() else {
            return;
        };

        self.player.position = state.position;
        self.interpolation.previous_position = state.position;

        self.velocity = state.velocity;
        self.player.on_ground = state.on_ground;
    }

    fn apply_horizontal_input(&mut self, movement: MovementInput, camera: CameraState) {
        self.player.sneaking = movement.sneak;
        self.player.sprinting =
            movement.sprint && movement.forward > 0.0 && self.sprint_reset_ticks == 0;

        let mut speed = self.config.locomotion.walk_speed_per_tick;

        if self.player.sprinting {
            speed *= self.config.locomotion.sprint_multiplier;
        }

        let mut movement = movement;
        movement.forward *= 0.98;
        movement.strafe *= 0.98;
        if self.player.sneaking {
            movement.forward *= self.config.locomotion.sneak_multiplier as f32;
            movement.strafe *= self.config.locomotion.sneak_multiplier as f32;
        }
        let (wish_x, wish_z) = wish_direction(movement, camera);

        if wish_x == 0.0 && wish_z == 0.0 {
            return;
        }

        let acceleration = if self.player.on_ground {
            let friction = self.config.ground_friction as f32;
            speed
                * f64::from(
                    self.config.ground_acceleration as f32 / (friction * friction * friction),
                )
        } else {
            self.config.air_acceleration * if self.player.sprinting { 1.3 } else { 1.0 }
        };

        self.velocity.x += wish_x * acceleration;
        self.velocity.z += wish_z * acceleration;
    }

    fn apply_jump(&mut self, movement: MovementInput, camera: CameraState) {
        if movement.jump && self.player.on_ground {
            self.velocity.y = self.config.jump_velocity;
            if self.player.sprinting {
                let yaw = f64::from(camera.yaw).to_radians();
                self.velocity.x -= yaw.sin() * f64::from(0.2_f32);
                self.velocity.z += yaw.cos() * f64::from(0.2_f32);
            }
        }
    }

    fn apply_pending_knockback(&mut self) {
        let Some(velocity) = self.pending_knockback.take() else {
            return;
        };
        self.velocity = velocity;
        if velocity.y > 0.0 {
            self.player.on_ground = false;
        }
    }

    fn integrate_vertical_motion(&mut self) {
        self.velocity.y =
            (self.velocity.y - self.config.gravity_per_tick) * self.config.vertical_drag;
    }

    fn resolve_collisions(&mut self) {
        let mut next_position = self.player.position.add(self.velocity);

        if next_position.x < self.config.collision.min_x {
            next_position.x = self.config.collision.min_x;
            self.velocity.x = 0.0;
        } else if next_position.x > self.config.collision.max_x {
            next_position.x = self.config.collision.max_x;
            self.velocity.x = 0.0;
        }

        if next_position.z < self.config.collision.min_z {
            next_position.z = self.config.collision.min_z;
            self.velocity.z = 0.0;
        } else if next_position.z > self.config.collision.max_z {
            next_position.z = self.config.collision.max_z;
            self.velocity.z = 0.0;
        }

        if next_position.y <= self.config.collision.floor_y {
            next_position.y = self.config.collision.floor_y;
            self.velocity.y = 0.0;
            self.player.on_ground = true;
        } else {
            self.player.on_ground = false;
        }

        if next_position.y > self.config.collision.ceiling_y {
            next_position.y = self.config.collision.ceiling_y;
            self.velocity.y = 0.0;
        }

        self.player.position = next_position;
    }

    fn resolve_terrain(&mut self, world: &WorldSnapshot) {
        let position = self.player.position;
        let initial = Aabb::new(
            [position.x - 0.3, position.y, position.z - 0.3],
            [position.x + 0.3, position.y + 1.8, position.z + 0.3],
        );
        let mut desired = [self.velocity.x, self.velocity.y, self.velocity.z];
        if self.player.on_ground && self.player.sneaking {
            let reduce = |v: f64| {
                if v.abs() < 0.05 {
                    0.0
                } else {
                    v - v.signum() * 0.05
                }
            };
            while desired[0] != 0.0
                && world
                    .collision_boxes(initial.offset([desired[0], -1.0, 0.0]))
                    .is_empty()
            {
                desired[0] = reduce(desired[0]);
            }
            while desired[2] != 0.0
                && world
                    .collision_boxes(initial.offset([0.0, -1.0, desired[2]]))
                    .is_empty()
            {
                desired[2] = reduce(desired[2]);
            }
            while desired[0] != 0.0
                && desired[2] != 0.0
                && world
                    .collision_boxes(initial.offset([desired[0], -1.0, desired[2]]))
                    .is_empty()
            {
                desired[0] = reduce(desired[0]);
                desired[2] = reduce(desired[2]);
            }
        }
        let obstacles = world.collision_boxes(initial.swept(desired));
        let (mut bounds, mut actual) = clip_motion(initial, desired, &obstacles);
        let grounded = self.player.on_ground || (desired[1] < 0.0 && actual[1] != desired[1]);
        if grounded && (actual[0] != desired[0] || actual[2] != desired[2]) {
            let step_obstacles =
                world.collision_boxes(initial.swept([desired[0], 0.6, desired[2]]));
            let (stepped, step_delta) =
                clip_motion(initial, [desired[0], 0.6, desired[2]], &step_obstacles);
            if step_delta[0] * step_delta[0] + step_delta[2] * step_delta[2]
                > actual[0] * actual[0] + actual[2] * actual[2]
            {
                let down_obstacles =
                    world.collision_boxes(stepped.swept([0.0, -step_delta[1], 0.0]));
                let (landed, down_delta) =
                    clip_motion(stepped, [0.0, -step_delta[1], 0.0], &down_obstacles);
                bounds = landed;
                actual = [step_delta[0], step_delta[1] + down_delta[1], step_delta[2]];
            }
        }
        self.player.position = Vec3::new(
            (bounds.min[0] + bounds.max[0]) * 0.5,
            bounds.min[1],
            (bounds.min[2] + bounds.max[2]) * 0.5,
        );
        self.player.on_ground = desired[1] < 0.0 && actual[1] != desired[1];
        if actual[0] != desired[0] {
            self.velocity.x = 0.0;
        }
        if actual[1] != desired[1] {
            self.velocity.y = 0.0;
        }
        if actual[2] != desired[2] {
            self.velocity.z = 0.0;
        }
        if actual[0] != desired[0] || actual[2] != desired[2] {
            self.player.sprinting = false;
        }
    }
}

fn clip_motion(mut bounds: Aabb, mut motion: [f64; 3], obstacles: &[Aabb]) -> (Aabb, [f64; 3]) {
    for axis in [1, 0, 2] {
        for obstacle in obstacles {
            motion[axis] = obstacle.clip_axis(bounds, axis, motion[axis]);
        }
        let mut delta = [0.0; 3];
        delta[axis] = motion[axis];
        bounds = bounds.offset(delta);
    }
    (bounds, motion)
}

fn wish_direction(movement: MovementInput, camera: CameraState) -> (f64, f64) {
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

    (forward * -sin + strafe * cos, forward * cos + strafe * sin)
}

#[cfg(test)]
mod tests {
    use super::{
        AuthoritativePlayerState, KnockbackImpulse, LocalSimulationLayer, SimulationConfig,
    };
    use crate::camera::CameraState;
    use crate::input::MovementInput;
    use crate::player::Vec3;

    #[test]
    fn vanilla_first_ground_tick_accelerates_before_drag() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.tick(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );
        assert!((simulation.player().position.z - 0.098).abs() < 1.0e-6);
        assert!((simulation.velocity().z - 0.098 * 0.546).abs() < 1.0e-6);
    }

    #[test]
    fn vanilla_jump_moves_before_gravity() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.tick(
            MovementInput {
                jump: true,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );
        assert!((simulation.player().position.y - f64::from(0.42_f32)).abs() < 1.0e-9);
        assert!(
            (simulation.velocity().y - (f64::from(0.42_f32) - 0.08) * f64::from(0.98_f32)).abs()
                < 1.0e-9
        );
    }

    #[test]
    fn small_server_correction_is_exact_and_not_blended() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.apply_authoritative_state(AuthoritativePlayerState {
            position: Vec3::new(0.5, 0.0, 0.5),
            velocity: Vec3::ZERO,
            on_ground: true,
        });
        simulation.tick(MovementInput::default(), CameraState::default(), 0);
        assert_eq!(simulation.player().position, Vec3::new(0.5, 0.0, 0.5));
    }

    #[test]
    fn relative_teleports_preserve_axis_velocity_and_resolve_in_order() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.apply_knockback(KnockbackImpulse::new(0.3, 0.2, 0.1));
        simulation.apply_event(super::SimulationEvent::Teleport {
            position: Vec3::new(2.0, 4.0, 6.0),
            yaw: 0.0,
            pitch: 0.0,
            flags: 1,
        });
        simulation.apply_event(super::SimulationEvent::Teleport {
            position: Vec3::new(1.0, 0.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            flags: 7,
        });
        assert_eq!(simulation.player().position, Vec3::new(3.0, 4.0, 6.0));
        assert_eq!(simulation.velocity(), Vec3::new(0.3, 0.0, 0.0));
    }

    #[test]
    fn server_velocity_replaces_previous_motion() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.tick(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );
        simulation.apply_knockback(KnockbackImpulse::new(0.4, 0.2, 0.0));
        simulation.tick(MovementInput::default(), CameraState::default(), 0);
        assert!((simulation.player().position.z - 0.098).abs() < 1.0e-6);
        assert!((simulation.player().position.x - 0.4).abs() < 1.0e-9);
    }

    #[test]
    fn applies_knockback_and_resets_sprint() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());

        simulation.tick(
            MovementInput {
                forward: 1.0,
                sprint: true,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );
        simulation.apply_knockback(KnockbackImpulse::new(0.4, 0.2, 0.0));
        simulation.tick(
            MovementInput {
                forward: 1.0,
                sprint: true,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );

        let snapshot = simulation.snapshot();
        assert!(!snapshot.player.sprinting);
        assert!(snapshot.velocity.x > 0.0 || snapshot.velocity.y > 0.0);
    }

    #[test]
    fn resolves_floor_collision() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.apply_knockback(KnockbackImpulse::new(0.0, -1.0, 0.0));
        simulation.tick(MovementInput::default(), CameraState::default(), 0);

        let snapshot = simulation.snapshot();
        assert_eq!(snapshot.player.position.y, 0.0);
        assert!(snapshot.player.on_ground);
    }

    #[test]
    fn interpolates_between_previous_and_current_positions() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.tick(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );

        let snapshot = simulation.snapshot();
        let sampled = snapshot.render_position(0.5);
        assert!(sampled.z > snapshot.interpolation.previous_position.z);
        assert!(sampled.z < snapshot.interpolation.current_position.z);
    }

    #[test]
    fn applies_server_correction_before_render() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        simulation.tick(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );
        simulation.apply_authoritative_state(AuthoritativePlayerState {
            position: Vec3::new(10.0, 0.0, 0.0),
            velocity: Vec3::ZERO,
            on_ground: true,
        });
        simulation.tick(MovementInput::default(), CameraState::default(), 0);

        let snapshot = simulation.snapshot();
        assert_eq!(snapshot.network.position.x, 10.0);
        assert_eq!(snapshot.player.position.x, 10.0);
    }
}
