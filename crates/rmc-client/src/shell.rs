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
}

impl ClientShell {
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
