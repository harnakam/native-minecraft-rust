use rmc_game::camera::{CameraState, MouseSettings};
use rmc_game::input::{InputFrame, InputSnapshot, KeyBindings};
use rmc_game::simulation::{
    LocalSimulationLayer, SimulationConfig, SimulationEvent, SimulationSnapshot,
};
use rmc_game::tick::{
    FixedStepConfig, FixedStepTimer, FramePacer, FramePacingConfig, FramePacingSnapshot,
};
use rmc_net::codec::play::PlayServerboundPacket;
use rmc_render::RenderFrameSnapshot;
use rmc_ui::MinimalHud;
use rmc_world::WorldSnapshot;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClientShellConfig {
    pub fixed_step: FixedStepConfig,
    pub pacing: FramePacingConfig,
    pub mouse: MouseSettings,
    pub simulation: SimulationConfig,
}

impl ClientShellConfig {
    pub fn vanilla() -> Self {
        Self {
            fixed_step: FixedStepConfig::vanilla(),
            pacing: FramePacingConfig::vanilla(),
            mouse: MouseSettings::vanilla(),
            simulation: SimulationConfig::vanilla(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellPerformanceSnapshot {
    pub pacing: FramePacingSnapshot,
    pub packet_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShellAdvanceOutput {
    pub statistic_increments: Vec<(&'static str, i32)>,
    pub ticks_run: usize,
    pub total_ticks: u64,
    pub packets: Vec<PlayServerboundPacket>,
    pub simulation: SimulationSnapshot,
    pub render: RenderFrameSnapshot,
    pub performance: ShellPerformanceSnapshot,
}

pub struct ClientShell {
    config: ClientShellConfig,
    timer: FixedStepTimer,
    frame_pacer: FramePacer,
    bindings: KeyBindings,
    input: InputSnapshot,
    camera: CameraState,
    simulation: LocalSimulationLayer,
    packet_emitter: rmc_game::player::WalkingPacketEmitter,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmc_net::codec::play::{HeldItemChangeClientboundPacket, PlayClientboundPacket};

    #[test]
    fn jump_statistics_follow_simulation_ticks_and_drain_once_per_frame() {
        use rmc_game::input::PhysicalInput;
        let mut shell = ClientShell::new(ClientShellConfig::vanilla());
        shell.set_mouse_captured(true);
        let press = InputFrame {
            pressed_inputs: vec![PhysicalInput::Space],
            ..InputFrame::default()
        };
        let zero = shell.advance(Duration::ZERO, &press);
        assert!(zero.statistic_increments.is_empty());
        let mut jump = shell.advance(Duration::from_millis(50), &InputFrame::default());
        for _ in 0..10 {
            if jump.ticks_run > 0 {
                break;
            }
            assert!(jump.statistic_increments.is_empty());
            jump = shell.advance(Duration::from_millis(50), &InputFrame::default());
        }
        assert!(jump.ticks_run > 0);
        assert_eq!(jump.statistic_increments, [("stat.jump", 1)]);
        let airborne = shell.advance(Duration::from_millis(50), &InputFrame::default());
        assert!(airborne.statistic_increments.is_empty());
        let no_tick = shell.advance(Duration::ZERO, &InputFrame::default());
        assert!(no_tick.statistic_increments.is_empty());
    }

    #[test]
    fn server_hotbar_change_survives_subsequent_input_frames() {
        let mut shell = ClientShell::new(ClientShellConfig::vanilla());
        shell.apply_player_packet(
            &PlayClientboundPacket::HeldItemChange(HeldItemChangeClientboundPacket { slot: 7 }),
            None,
        );
        for _ in 0..3 {
            let output = shell.advance(Duration::from_millis(50), &InputFrame::default());
            assert_eq!(output.simulation.player.selected_hotbar_slot, 7);
        }
    }
    #[test]
    fn teleport_and_explosion_motion_follow_packet_order_within_one_frame() {
        use rmc_game::player::Vec3;
        let correction = SimulationEvent::Teleport {
            position: Vec3::new(0.0, 64.0, 0.0),
            yaw: 0.0,
            pitch: 0.0,
            flags: 0,
        };
        let explosion = SimulationEvent::AddVelocity(Vec3::new(0.25, 0.5, -0.25));
        for (events, expected) in [
            ([correction, explosion], Vec3::new(0.25, 0.5, -0.25)),
            ([explosion, correction], Vec3::ZERO),
        ] {
            let mut shell = ClientShell::new(ClientShellConfig::vanilla());
            let output = shell.advance_with_events(Duration::ZERO, &InputFrame::default(), &events);
            assert_eq!(output.simulation.velocity, expected);
        }
    }
    #[test]
    fn double_jump_input_emits_real_player_abilities_before_movement() {
        use rmc_game::input::PhysicalInput;
        use rmc_net::codec::play::PlayerAbilitiesPacket;
        let mut shell = ClientShell::new(ClientShellConfig::vanilla());
        shell.set_mouse_captured(true);
        shell.apply_player_packet(
            &PlayClientboundPacket::PlayerAbilities(PlayerAbilitiesPacket {
                flags: 13,
                flying_speed: 0.05,
                walking_speed: 0.1,
            }),
            Some(1),
        );
        let press = InputFrame {
            pressed_inputs: vec![PhysicalInput::Space],
            ..InputFrame::default()
        };
        let release = InputFrame {
            released_inputs: vec![PhysicalInput::Space],
            ..InputFrame::default()
        };
        shell.advance(Duration::from_millis(50), &press);
        shell.advance(Duration::from_millis(50), &release);
        let output = shell.advance(Duration::from_millis(50), &press);
        assert!(
            matches!(output.packets.first(),Some(PlayServerboundPacket::PlayerAbilities(packet)) if packet.flags==15)
        );
        assert_eq!(
            output.packets[0].encode_packet().unwrap().packet_bytes()[0],
            0x13
        );
    }
}

impl ClientShell {
    pub fn set_depth_strider(&mut self, level: i16) {
        self.simulation.set_depth_strider(level);
    }

    pub fn mining_effects(&self) -> (Option<u8>, Option<u8>) {
        (
            self.simulation.effect_amplifier(3),
            self.simulation.effect_amplifier(4),
        )
    }
    pub fn new(config: ClientShellConfig) -> Self {
        Self {
            timer: FixedStepTimer::new(config.fixed_step),
            frame_pacer: FramePacer::new(config.pacing),
            config,
            bindings: KeyBindings::vanilla(),
            input: InputSnapshot::default(),
            camera: CameraState::default(),
            simulation: LocalSimulationLayer::new(config.simulation),
            packet_emitter: rmc_game::player::WalkingPacketEmitter::default(),
        }
    }

    pub fn adjust_spectator_fly_speed(&mut self, delta: i8) {
        self.simulation.adjust_spectator_fly_speed(delta);
    }

    pub fn apply_player_packet(
        &mut self,
        packet: &rmc_net::codec::play::PlayClientboundPacket,
        entity_id: Option<i32>,
    ) {
        if let rmc_net::codec::play::PlayClientboundPacket::HeldItemChange(packet) = packet {
            if (0..=8).contains(&packet.slot) {
                self.input.selected_hotbar_slot = packet.slot as u8;
            }
        }
        if matches!(
            packet,
            rmc_net::codec::play::PlayClientboundPacket::Respawn(_)
        ) {
            self.input = InputSnapshot::default();
            self.camera.yaw = 0.0;
            self.camera.pitch = 0.0;
            self.packet_emitter = rmc_game::player::WalkingPacketEmitter::default();
        }
        self.simulation.apply_player_packet(packet, entity_id);
    }

    pub fn set_mouse_captured(&mut self, captured: bool) {
        self.input.mouse_captured = captured;
        self.camera.set_mouse_captured(captured);
    }

    pub fn advance(
        &mut self,
        frame_delta: Duration,
        frame_input: &InputFrame,
    ) -> ShellAdvanceOutput {
        self.advance_with_events(frame_delta, frame_input, &[])
    }

    pub fn advance_with_events(
        &mut self,
        frame_delta: Duration,
        frame_input: &InputFrame,
        frame_events: &[SimulationEvent],
    ) -> ShellAdvanceOutput {
        self.advance_internal(frame_delta, frame_input, frame_events, None)
    }

    pub fn advance_in_world(
        &mut self,
        frame_delta: Duration,
        frame_input: &InputFrame,
        frame_events: &[SimulationEvent],
        world: &WorldSnapshot,
    ) -> ShellAdvanceOutput {
        self.advance_internal(frame_delta, frame_input, frame_events, Some(world))
    }

    fn advance_internal(
        &mut self,
        frame_delta: Duration,
        frame_input: &InputFrame,
        frame_events: &[SimulationEvent],
        world: Option<&WorldSnapshot>,
    ) -> ShellAdvanceOutput {
        let update = self.input.apply_frame(&self.bindings, frame_input);
        self.camera.set_mouse_captured(update.mouse_captured);
        self.camera.apply_mouse_delta(
            update.mouse_delta_x,
            update.mouse_delta_y,
            self.config.mouse,
        );

        for event in frame_events {
            if let SimulationEvent::Teleport {
                yaw, pitch, flags, ..
            } = *event
            {
                self.camera.yaw = yaw + if flags & 8 != 0 { self.camera.yaw } else { 0.0 };
                self.camera.pitch = pitch
                    + if flags & 16 != 0 {
                        self.camera.pitch
                    } else {
                        0.0
                    };
            }
            self.simulation.apply_event(*event);
        }

        let pacing = self.frame_pacer.pace(frame_delta);
        let schedule = self.timer.advance(pacing.paced_frame_time);
        let mut packets = Vec::with_capacity(schedule.ticks_to_run);
        let mut statistic_increments = Vec::new();

        for _ in 0..schedule.ticks_to_run {
            if let Some(world) = world {
                self.simulation.tick_with_world(
                    update.movement,
                    self.camera,
                    update.selected_hotbar_slot,
                    world,
                );
            } else {
                self.simulation
                    .tick(update.movement, self.camera, update.selected_hotbar_slot);
            }
            statistic_increments.extend(self.simulation.take_statistic_increments());
            packets.extend(
                self.simulation
                    .take_ability_changes()
                    .into_iter()
                    .map(PlayServerboundPacket::PlayerAbilities),
            );
            packets.push(
                self.packet_emitter
                    .next_packet(self.simulation.player(), self.camera),
            );
        }

        let simulation = self.simulation.snapshot();
        let hud = MinimalHud::vanilla(simulation.player.selected_hotbar_slot);
        let render = RenderFrameSnapshot::from_simulation_state(
            schedule.total_ticks,
            schedule.interpolation_alpha,
            &simulation,
            self.camera,
            hud,
        );

        ShellAdvanceOutput {
            statistic_increments,
            ticks_run: schedule.ticks_to_run,
            total_ticks: schedule.total_ticks,
            performance: ShellPerformanceSnapshot {
                pacing,
                packet_count: packets.len(),
            },
            packets,
            simulation,
            render,
        }
    }
}
