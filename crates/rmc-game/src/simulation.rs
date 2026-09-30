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
    AddVelocity(Vec3),
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
    jump_ticks: u8,
    in_web: bool,
    fluid_acceleration: Option<f32>,
    depth_strider: i16,
    fluid_in_water: bool,
    food_level: i32,
    alive: bool,
    allow_flying: bool,
    flying: bool,
    flying_speed: f32,
    walking_speed: f32,
    abilities_flags: u8,
    outside_border: bool,
    game_mode: u8,
    fly_toggle_ticks: u8,
    previous_jump: bool,
    ability_changes: Vec<rmc_net::codec::play::PlayerAbilitiesPacket>,
    attribute_speed: f64,
    air_movement_factor: f32,
    effects: std::collections::BTreeMap<u8, (u8, i32)>,
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
            jump_ticks: 0,
            in_web: false,
            fluid_acceleration: None,
            depth_strider: 0,
            fluid_in_water: false,
            food_level: 20,
            alive: true,
            allow_flying: false,
            flying: false,
            flying_speed: 0.05,
            walking_speed: 0.1,
            abilities_flags: 0,
            outside_border: false,
            game_mode: 0,
            fly_toggle_ticks: 0,
            previous_jump: false,
            ability_changes: Vec::new(),
            attribute_speed: f64::from(0.1_f32),
            air_movement_factor: 0.02,
            effects: std::collections::BTreeMap::new(),
        }
    }

    pub fn set_depth_strider(&mut self, level: i16) {
        self.depth_strider = level.clamp(0, 3);
    }

    pub fn apply_event(&mut self, event: SimulationEvent) {
        match event {
            SimulationEvent::AuthoritativeState(state) => self.apply_authoritative_state(state),
            SimulationEvent::Knockback(impulse) => self.apply_knockback(impulse),
            SimulationEvent::AddVelocity(motion) => {
                self.apply_pending_server_state();
                self.apply_pending_knockback();
                self.velocity = self.velocity.add(motion);
            }
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

    pub fn apply_player_packet(
        &mut self,
        packet: &rmc_net::codec::play::PlayClientboundPacket,
        local_entity: Option<i32>,
    ) {
        use rmc_net::codec::play::PlayClientboundPacket as Packet;
        match packet {
            Packet::PlayerAbilities(p) => {
                self.abilities_flags = p.flags & 15;
                if p.walking_speed.is_finite() {
                    self.walking_speed = p.walking_speed;
                }
                self.allow_flying = p.flags & 4 != 0;
                self.flying = p.flags & 2 != 0;
                if p.flying_speed.is_finite() {
                    self.flying_speed = p.flying_speed;
                }
            }
            Packet::UpdateHealth(p) => {
                self.food_level = p.food_level;
                self.alive = p.health > 0.0;
            }
            Packet::EntityEffect(p) if Some(p.entity_id) == local_entity => {
                self.effects.insert(p.effect_id, (p.amplifier, p.duration));
            }
            Packet::RemoveEntityEffect(p) if Some(p.entity_id) == local_entity => {
                self.effects.remove(&p.effect_id);
            }
            Packet::EntityProperties(p) if Some(p.entity_id) == local_entity => {
                for attribute in &p.attributes {
                    if attribute.name != "generic.movementSpeed" || !attribute.base.is_finite() {
                        continue;
                    }
                    // Local sprint owns this modifier; receiving it must not apply it twice.
                    let sprint_uuid = [
                        0x66, 0x2a, 0x6b, 0x8d, 0xda, 0x3e, 0x4c, 0x1c, 0x88, 0x13, 0x96, 0xea,
                        0x60, 0x97, 0x27, 0x8d,
                    ];
                    let modifiers: Vec<_> = attribute
                        .modifiers
                        .iter()
                        .filter(|m| m.uuid != sprint_uuid && m.amount.is_finite())
                        .collect();
                    let base = attribute.base
                        + modifiers
                            .iter()
                            .filter(|m| m.operation == 0)
                            .map(|m| m.amount)
                            .sum::<f64>();
                    let mut value = base;
                    for m in &modifiers {
                        if m.operation == 1 {
                            value += base * m.amount;
                        }
                    }
                    for m in &modifiers {
                        if m.operation == 2 {
                            value *= 1.0 + m.amount;
                        }
                    }
                    if value.is_finite() {
                        self.attribute_speed = value.clamp(0.0, 1024.0);
                    }
                }
            }
            Packet::JoinGame(packet) => self.set_game_mode(packet.game_mode),
            Packet::ChangeGameState(packet) if packet.reason == 3 => {
                self.set_game_mode(crate::game_mode::from_game_state(packet.value));
            }
            Packet::Respawn(packet) => {
                *self = Self::new(self.config);
                self.set_game_mode(packet.game_mode);
            }
            _ => {}
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
        let movement = if self.alive {
            movement
        } else {
            MovementInput::default()
        };
        if self.allow_flying {
            if self.game_mode == 3 && !self.flying {
                self.flying = true;
                self.queue_ability_change();
            } else if self.game_mode != 3 && movement.jump && !self.previous_jump {
                if self.fly_toggle_ticks == 0 {
                    self.fly_toggle_ticks = 7;
                } else {
                    self.flying = !self.flying;
                    self.fly_toggle_ticks = 0;
                    self.queue_ability_change();
                }
            }
        }
        self.previous_jump = movement.jump;
        self.effects.retain(|_, effect| {
            effect.1 -= 1;
            effect.1 > 0
        });
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
        let environment = world
            .map(|w| w.movement_environment(player_bounds(self.player.position)))
            .unwrap_or_default();
        self.velocity = self.velocity.add(Vec3::new(
            environment.water_flow[0],
            environment.water_flow[1],
            environment.water_flow[2],
        ));
        self.fluid_in_water = environment.water;
        self.fluid_acceleration = if !self.flying && (environment.water || environment.lava) {
            Some(0.02)
        } else {
            None
        };
        let friction = if environment.water && !self.flying {
            let level = self.water_enchantment_factor();
            f64::from(0.8_f32 + (0.54600006_f32 - 0.8_f32) * level / 3.0_f32)
        } else if environment.lava && !self.flying {
            0.5
        } else if self.player.on_ground {
            self.config.ground_friction
        } else {
            self.config.air_friction
        };
        self.jump_ticks = self.jump_ticks.saturating_sub(1);
        self.apply_horizontal_input(movement, camera);
        if self.flying {
            if movement.jump {
                self.velocity.y += f64::from(self.flying_speed * 3.0_f32);
            }
            if movement.sneak {
                self.velocity.y -= f64::from(self.flying_speed * 3.0_f32);
            }
        } else if environment.water || environment.lava {
            if movement.jump {
                self.velocity.y += f64::from(0.04_f32);
            }
        } else {
            self.apply_jump(movement, camera);
        }
        if environment.ladder && !self.flying {
            self.velocity.x = self
                .velocity
                .x
                .clamp(-f64::from(0.15_f32), f64::from(0.15_f32));
            self.velocity.z = self
                .velocity
                .z
                .clamp(-f64::from(0.15_f32), f64::from(0.15_f32));
            self.velocity.y = self.velocity.y.max(-0.15);
            if movement.sneak && self.velocity.y < 0.0 {
                self.velocity.y = 0.0;
            }
        }
        let web_slowed = self.in_web;
        if web_slowed {
            self.velocity.x *= 0.25;
            self.velocity.z *= 0.25;
            self.velocity.y *= f64::from(0.05_f32);
        }
        let motion_before_collision = self.velocity;
        let previous_y = self.player.position.y;
        if let Some(world) = world {
            self.resolve_terrain(world);
        } else {
            self.resolve_collisions();
        }
        if web_slowed {
            self.velocity = Vec3::ZERO;
        }
        let collided_horizontally = self.velocity.x != motion_before_collision.x
            || self.velocity.z != motion_before_collision.z;
        if environment.ladder && !self.flying && collided_horizontally {
            self.velocity.y = 0.2;
        }
        if self.flying {
            self.velocity.y = motion_before_collision.y * 0.6;
        } else if environment.water || environment.lava {
            self.velocity.y = self.velocity.y
                * if environment.water {
                    f64::from(0.8_f32)
                } else {
                    0.5
                }
                - 0.02;
        } else {
            self.integrate_vertical_motion();
        }
        if let Some(world) = world {
            self.in_web = world
                .movement_environment(player_bounds(self.player.position))
                .web;
            let pos = BlockPos::new(
                self.player.position.x.floor() as i32,
                0,
                self.player.position.z.floor() as i32,
            );
            if !self.flying
                && !environment.water
                && !environment.lava
                && world.chunk(pos.chunk_pos()).is_none()
            {
                self.velocity.y = if self.player.position.y > 0.0 {
                    -0.1 * self.config.vertical_drag
                } else {
                    0.0
                };
            }
        }
        self.velocity.x *= friction;
        self.velocity.z *= friction;
        if !self.flying && (environment.water || environment.lava) && collided_horizontally {
            if let Some(world) = world {
                let offset = [
                    self.velocity.x,
                    self.velocity.y + f64::from(0.6_f32) - self.player.position.y + previous_y,
                    self.velocity.z,
                ];
                if world.liquid_escape_clear(player_bounds(self.player.position).offset(offset)) {
                    self.velocity.y = f64::from(0.3_f32);
                }
            }
        }
        self.air_movement_factor = if self.player.sprinting {
            (f64::from(0.02_f32) + f64::from(0.02_f32) * 0.3) as f32
        } else {
            0.02
        };
        if self.flying && self.player.on_ground && self.game_mode != 3 {
            self.flying = false;
            self.queue_ability_change();
        }
        self.fly_toggle_ticks = self.fly_toggle_ticks.saturating_sub(1);
        self.interpolation.current_position = self.player.position;

        if self.sprint_reset_ticks > 0 {
            self.sprint_reset_ticks -= 1;
        }
    }

    fn set_game_mode(&mut self, mode: u8) {
        self.game_mode = mode;
        self.allow_flying = matches!(mode, 1 | 3);
        if mode == 3 {
            self.flying = true;
        } else if mode != 1 {
            self.flying = false;
        }
        self.abilities_flags = match mode {
            1 => 13,
            3 => 5,
            _ => 0,
        };
    }
    fn queue_ability_change(&mut self) {
        self.ability_changes
            .push(rmc_net::codec::play::PlayerAbilitiesPacket {
                flags: (self.abilities_flags & !2) | if self.flying { 2 } else { 0 },
                flying_speed: self.flying_speed,
                walking_speed: self.walking_speed,
            });
    }
    pub fn take_ability_changes(&mut self) -> Vec<rmc_net::codec::play::PlayerAbilitiesPacket> {
        std::mem::take(&mut self.ability_changes)
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

    pub fn effect_amplifier(&self, id: u8) -> Option<u8> {
        self.effects.get(&id).map(|effect| effect.0)
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

    fn water_enchantment_factor(&self) -> f32 {
        self.depth_strider as f32 * if self.player.on_ground { 1.0 } else { 0.5 }
    }

    fn apply_horizontal_input(&mut self, movement: MovementInput, camera: CameraState) {
        self.player.sneaking = movement.sneak;
        let forward = movement.forward * if movement.sneak { 0.3_f32 } else { 1.0 };
        let eligible = (self.food_level > 6 || self.allow_flying) && self.sprint_reset_ticks == 0;
        let start = movement.sprint && !self.effects.contains_key(&15);
        self.player.sprinting = (self.player.sprinting || start) && forward >= 0.8 && eligible;

        let mut speed = self.attribute_speed;

        if self.player.sprinting {
            speed *= 1.0 + f64::from(0.3_f32);
        }

        let mut movement = movement;
        movement.forward *= 0.98;
        movement.strafe *= 0.98;
        if self.player.sneaking {
            movement.forward *= self.config.locomotion.sneak_multiplier as f32;
            movement.strafe *= self.config.locomotion.sneak_multiplier as f32;
        }
        let acceleration = if let Some(fluid) = self.fluid_acceleration {
            let level = if fluid == 0.02 && self.fluid_in_water {
                self.water_enchantment_factor()
            } else {
                0.0
            };
            f64::from(fluid + (speed as f32 - fluid) * level / 3.0_f32)
        } else if self.player.on_ground {
            let friction = self.config.ground_friction as f32;
            f64::from(
                (speed as f32)
                    * (self.config.ground_acceleration as f32 / (friction * friction * friction)),
            )
        } else {
            f64::from(if self.flying {
                self.flying_speed * if self.player.sprinting { 2.0 } else { 1.0 }
            } else {
                self.air_movement_factor
            })
        };

        let (x, z) = crate::math::move_flying(
            movement.strafe,
            movement.forward,
            acceleration as f32,
            camera.yaw,
        );
        self.velocity.x += x;
        self.velocity.z += z;
    }

    fn apply_jump(&mut self, movement: MovementInput, camera: CameraState) {
        if !movement.jump {
            self.jump_ticks = 0;
        }
        if movement.jump && self.player.on_ground && self.jump_ticks == 0 {
            self.jump_ticks = 10;
            self.velocity.y = self.config.jump_velocity;
            if let Some((amplifier, _)) = self.effects.get(&8) {
                self.velocity.y += f64::from((u16::from(*amplifier) + 1) as f32 * 0.1_f32);
            }
            if self.player.sprinting {
                let yaw = camera.yaw * 0.017453292_f32;
                self.velocity.x -= f64::from(crate::math::sin(yaw) * 0.2_f32);
                self.velocity.z += f64::from(crate::math::cos(yaw) * 0.2_f32);
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
        let initial = player_bounds(position);
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
                    .collision_boxes_for_player(
                        initial.offset([desired[0], -1.0, 0.0]),
                        [position.x, position.y, position.z],
                        &mut self.outside_border,
                    )
                    .is_empty()
            {
                desired[0] = reduce(desired[0]);
            }
            while desired[2] != 0.0
                && world
                    .collision_boxes_for_player(
                        initial.offset([0.0, -1.0, desired[2]]),
                        [position.x, position.y, position.z],
                        &mut self.outside_border,
                    )
                    .is_empty()
            {
                desired[2] = reduce(desired[2]);
            }
            while desired[0] != 0.0
                && desired[2] != 0.0
                && world
                    .collision_boxes_for_player(
                        initial.offset([desired[0], -1.0, desired[2]]),
                        [position.x, position.y, position.z],
                        &mut self.outside_border,
                    )
                    .is_empty()
            {
                desired[0] = reduce(desired[0]);
                desired[2] = reduce(desired[2]);
            }
        }
        let obstacles = world.collision_boxes_for_player(
            initial.swept(desired),
            [position.x, position.y, position.z],
            &mut self.outside_border,
        );
        let (mut bounds, mut actual) = clip_motion(initial, desired, &obstacles);
        let grounded = self.player.on_ground || (desired[1] < 0.0 && actual[1] != desired[1]);
        if grounded && (actual[0] != desired[0] || actual[2] != desired[2]) {
            let step_obstacles = world.collision_boxes_for_player(
                initial.swept([desired[0], 0.6, desired[2]]),
                [position.x, position.y, position.z],
                &mut self.outside_border,
            );
            let (stepped, step_delta) =
                clip_motion(initial, [desired[0], 0.6, desired[2]], &step_obstacles);
            if step_delta[0] * step_delta[0] + step_delta[2] * step_delta[2]
                > actual[0] * actual[0] + actual[2] * actual[2]
            {
                let down_obstacles = world.collision_boxes_for_player(
                    stepped.swept([0.0, -step_delta[1], 0.0]),
                    [position.x, position.y, position.z],
                    &mut self.outside_border,
                );
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
        let supporting = world.block_state_or_air(BlockPos::new(
            self.player.position.x.floor() as i32,
            (self.player.position.y - f64::from(0.2_f32)).floor() as i32,
            self.player.position.z.floor() as i32,
        )) >> 4;
        if actual[1] != desired[1] {
            if supporting == 165 && !self.player.sneaking && self.velocity.y < 0.0 {
                self.velocity.y = -self.velocity.y;
            } else {
                self.velocity.y = 0.0;
            }
        }
        if supporting == 165
            && self.player.on_ground
            && !self.player.sneaking
            && self.velocity.y.abs() < 0.1
        {
            let slowdown = 0.4 + self.velocity.y.abs() * 0.2;
            self.velocity.x *= slowdown;
            self.velocity.z *= slowdown;
        }
        let contacts = world
            .movement_environment(player_bounds(self.player.position))
            .soul_sand_contacts;
        for _ in 0..contacts {
            self.velocity.x *= 0.4;
            self.velocity.z *= 0.4;
        }
        if actual[2] != desired[2] {
            self.velocity.z = 0.0;
        }
        if actual[0] != desired[0] || actual[2] != desired[2] {
            self.player.sprinting = false;
        }
    }
}

fn player_bounds(position: Vec3) -> Aabb {
    let radius = f64::from(0.6_f32) / 2.0;
    Aabb::new(
        [position.x - radius, position.y, position.z - radius],
        [
            position.x + radius,
            position.y + f64::from(1.8_f32),
            position.z + radius,
        ],
    )
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

#[cfg(test)]
mod tests {
    use super::{
        AuthoritativePlayerState, KnockbackImpulse, LocalSimulationLayer, SimulationConfig,
    };
    use crate::camera::CameraState;
    use crate::input::MovementInput;
    use crate::player::Vec3;

    #[test]
    fn held_jump_waits_ten_ticks_after_forced_early_landing() {
        let mut simulation = LocalSimulationLayer::new(SimulationConfig::vanilla());
        let jumping = MovementInput {
            jump: true,
            ..MovementInput::default()
        };
        simulation.tick(jumping, CameraState::default(), 0);
        simulation.apply_authoritative_state(AuthoritativePlayerState {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            on_ground: true,
        });
        simulation.tick(jumping, CameraState::default(), 0);
        assert_eq!(simulation.player().position.y, 0.0);
        simulation.tick(MovementInput::default(), CameraState::default(), 0);
        simulation.tick(jumping, CameraState::default(), 0);
        assert!((simulation.player().position.y - f64::from(0.42_f32)).abs() < 1e-9);
    }

    #[test]
    fn sprint_persists_after_sprint_key_release_while_forward_is_held() {
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
        simulation.tick(
            MovementInput {
                forward: 1.0,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );
        assert!(simulation.player().sprinting);
        simulation.tick(
            MovementInput {
                forward: 1.0,
                sneak: true,
                ..MovementInput::default()
            },
            CameraState::default(),
            0,
        );
        assert!(!simulation.player().sprinting);
    }

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
