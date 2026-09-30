pub use crate::client::entity::player::EntityTracker;
use crate::client::network::net_handler_play_client::apply_inbound_play_packet;
use crate::shell::{ClientShell, ClientShellConfig, ShellAdvanceOutput};
use rmc_game::combat::{CombatConfig, CombatSnapshot, CombatState};
use rmc_game::input::InputFrame;
use rmc_game::inventory::InventoryState;
use rmc_game::mining::{MiningContext, MiningState, MiningTarget};
use rmc_game::player::Vec3;
use rmc_game::simulation::SimulationEvent;
use rmc_game::usability::{UsabilitySnapshot, UsabilityState};
use rmc_net::address::resolve_connect_target;
use rmc_net::auth::{MojangSessionJoiner, OnlineAccount, SessionJoiner};
use rmc_net::codec::login::{EncryptionResponse, LoginServerboundPacket};
use rmc_net::codec::play::{
    AnimationPacket, BlockPosition, PlayClientboundPacket, PlayServerboundPacket,
    PlayerBlockPlacementPacket, SpawnPlayerPacket,
};
use rmc_net::crypto::build_login_encryption_response;
use rmc_net::driver::{DriverEvent, HeadlessDriver, HeadlessDriverConfig};
use rmc_net::headless::AuthenticationMode;
use rmc_net::session::{PlayerPose, SessionAction};
use rmc_net::trace::{PacketTraceEvent, TraceSink};
use rmc_net::transport::{DriverTransport, TransportError};
use rmc_net::zlib::DefaultZlibCodec;
use rmc_render::{
    ChunkMeshPipeline, FrustumConfig, MeshBuildConfig, RenderTelemetryHistory,
    RenderTelemetrySummary, WorldRenderSnapshot,
};
use rmc_ui::PvPHud;
use rmc_world::{WorldConfig, WorldSnapshot};
use std::io;
use std::net::TcpStream;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct LiveRuntimeConfig {
    pub server_host: String,
    pub server_port: u16,
    pub username: String,
    pub authentication: AuthenticationMode,
    pub brand: String,
    pub locale: String,
    pub view_distance: i8,
    pub mesh_jobs_per_frame: usize,
    pub capture_mouse: bool,
}

impl LiveRuntimeConfig {
    pub fn hypixel(account: OnlineAccount) -> Self {
        Self {
            server_host: "hypixel.net".to_owned(),
            server_port: 25565,
            username: account.username.clone(),
            authentication: AuthenticationMode::Online(account),
            brand: "RustMinecraft".to_owned(),
            locale: "en_US".to_owned(),
            view_distance: 8,
            mesh_jobs_per_frame: 4,
            capture_mouse: true,
        }
    }

    pub fn offline(username: impl Into<String>) -> Self {
        Self {
            server_host: "localhost".to_owned(),
            server_port: 25565,
            username: username.into(),
            authentication: AuthenticationMode::Offline,
            brand: "RustMinecraft".to_owned(),
            locale: "en_US".to_owned(),
            view_distance: 8,
            mesh_jobs_per_frame: 4,
            capture_mouse: true,
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeActionInput {
    pub attack_pressed: bool,
    pub attack_held: bool,
    pub use_pressed: bool,
    pub use_released: bool,
    pub close_window_pressed: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LiveRuntimeSummary {
    pub reached_play: bool,
    pub joined_game: bool,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub ignored_packets: usize,
    pub disconnect_reason_json: Option<String>,
    pub ended_by_eof: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetedEntity {
    pub entity_id: i32,
    pub uuid: [u8; 16],
    pub distance: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeTelemetry {
    pub render: RenderTelemetrySummary,
    pub frames: usize,
    pub total_ticks_run: usize,
}

#[derive(Default)]
struct RuntimeTrace {
    lines: Vec<String>,
    movement_corrections: Vec<String>,
}

impl TraceSink for RuntimeTrace {
    fn record(&mut self, event: PacketTraceEvent) {
        self.lines.push(event.summary_line());
    }
}

pub struct LiveRuntime {
    username: String,
    online_account: Option<OnlineAccount>,
    transport: DriverTransport<TcpStream>,
    driver: HeadlessDriver,
    zlib: DefaultZlibCodec,
    joiner: MojangSessionJoiner,
    shell: ClientShell,
    usability: UsabilityState,
    combat: CombatState,
    mining: MiningState,
    game_mode: u8,
    dimension: Option<i32>,
    world: WorldSnapshot,
    entity_tracker: EntityTracker,
    mesh_pipeline: ChunkMeshPipeline,
    render_history: RenderTelemetryHistory,
    trace: RuntimeTrace,
    summary: LiveRuntimeSummary,
    pending_simulation_events: Vec<SimulationEvent>,
    position_initialized: bool,
    last_output: Option<ShellAdvanceOutput>,
    last_world_render: Option<WorldRenderSnapshot>,
    last_hud: Option<PvPHud>,
    last_usability_snapshot: Option<UsabilitySnapshot>,
    last_targeted_entity: Option<TargetedEntity>,
    telemetry: RuntimeTelemetry,
    mesh_config: MeshBuildConfig,
}

impl LiveRuntime {
    pub fn connect(config: LiveRuntimeConfig) -> Result<Self, String> {
        let mut shell = ClientShell::new(ClientShellConfig::vanilla());
        shell.set_mouse_captured(config.capture_mouse);

        let mut usability = UsabilityState::new();
        usability.settings_mut().locale = config.locale.clone();
        usability.settings_mut().view_distance = config.view_distance;

        let mesh_config = MeshBuildConfig {
            max_jobs_per_frame: config.mesh_jobs_per_frame.max(1),
            ..MeshBuildConfig::default()
        };

        let mut driver_config = HeadlessDriverConfig::vanilla_headless(
            47,
            config.server_host.clone(),
            config.server_port,
            config.username().to_owned(),
        );
        driver_config.client_brand = config.brand.clone();
        driver_config.client_settings.locale = config.locale.clone();
        driver_config.client_settings.view_distance = config.view_distance;

        let connect_target = resolve_connect_target(&config.server_host, config.server_port);
        let stream = TcpStream::connect((connect_target.host.as_str(), connect_target.port))
            .map_err(|error| {
                format!(
                    "failed to connect to {}:{} (resolved {}:{}): {error}",
                    config.server_host,
                    config.server_port,
                    connect_target.host,
                    connect_target.port
                )
            })?;
        stream
            .set_nodelay(true)
            .map_err(|error| format!("failed to enable TCP_NODELAY: {error}"))?;
        stream
            .set_nonblocking(true)
            .map_err(|error| format!("failed to enable nonblocking mode: {error}"))?;

        let mut runtime = Self {
            username: config.username().to_owned(),
            online_account: match config.authentication {
                AuthenticationMode::Offline => None,
                AuthenticationMode::Online(account) => Some(account),
            },
            transport: DriverTransport::new(stream),
            driver: HeadlessDriver::new(driver_config),
            zlib: DefaultZlibCodec,
            joiner: MojangSessionJoiner,
            shell,
            usability,
            combat: CombatState::new(CombatConfig::vanilla()),
            mining: MiningState::default(),
            game_mode: 0,
            dimension: None,
            world: WorldSnapshot::new(WorldConfig::overworld()),
            entity_tracker: EntityTracker::default(),
            mesh_pipeline: ChunkMeshPipeline::with_config(mesh_config),
            render_history: RenderTelemetryHistory::default(),
            trace: RuntimeTrace::default(),
            summary: LiveRuntimeSummary::default(),
            pending_simulation_events: Vec::new(),
            position_initialized: false,
            last_output: None,
            last_world_render: None,
            last_hud: None,
            last_usability_snapshot: None,
            last_targeted_entity: None,
            telemetry: RuntimeTelemetry::default(),
            mesh_config,
        };

        runtime
            .driver
            .bootstrap_login(Some(&runtime.zlib), Some(&mut runtime.trace))
            .map_err(|error| format!("failed to bootstrap login: {error:?}"))?;
        runtime.flush_if_pending()?;
        Ok(runtime)
    }

    pub fn step(
        &mut self,
        frame_delta: Duration,
        frame_input: &InputFrame,
        actions: &RuntimeActionInput,
    ) -> Result<(), String> {
        self.pump_network()?;
        let no_actions = RuntimeActionInput::default();
        let actions = if self.combat.snapshot().health <= 0.0 {
            &no_actions
        } else {
            actions
        };

        if self.summary.disconnect_reason_json.is_some() || self.summary.ended_by_eof {
            return Ok(());
        }

        if !self.summary.joined_game || !self.position_initialized {
            self.flush_if_pending()?;
            return Ok(());
        }

        let depth_strider = self
            .usability
            .inventory()
            .inventory_window()
            .slots
            .iter()
            .skip(5)
            .take(4)
            .flatten()
            .map(|item| item.enchantment_level(8))
            .max()
            .unwrap_or(0)
            .clamp(0, 3);
        self.shell.set_depth_strider(depth_strider as i16);
        let frame_events = self.pending_simulation_events.clone();
        self.pending_simulation_events.clear();
        let output =
            self.shell
                .advance_in_world(frame_delta, frame_input, &frame_events, &self.world);

        for _ in 0..output.ticks_run {
            self.world.advance_time(1);
            self.combat.tick_feedback();
        }

        if let Some(packet) = self
            .usability
            .inventory_mut()
            .sync_selected_hotbar_slot(output.simulation.player.selected_hotbar_slot)
        {
            self.queue_play_packet(&packet)?;
        }

        if let Some(player_entity_id) = self.driver.session().snapshot().player_entity_id {
            for packet in self.combat.sync_action_state(
                player_entity_id,
                output.simulation.player.sprinting,
                output.simulation.player.sneaking,
            ) {
                self.queue_play_packet(&packet)?;
            }
        }

        for packet in &output.packets {
            self.queue_play_packet(packet)?;
        }

        self.driver.set_local_pose(player_pose_from_output(&output));

        let targeted_entity = pick_target_entity(
            &output,
            &self.entity_tracker,
            self.driver.session().snapshot().player_entity_id,
            if self.game_mode == 1 { 6.0 } else { 3.0 },
            &self.world,
        );

        if actions.attack_pressed {
            if let Some(target) = targeted_entity {
                for packet in self.combat.attack_entity(target.entity_id) {
                    self.queue_play_packet(&packet)?;
                }
            } else {
                self.queue_play_packet(&PlayServerboundPacket::Animation(AnimationPacket))?;
            }
        }

        let camera = output.render.camera;
        let direction = forward_vector(camera.yaw, camera.pitch);
        let block_hit = if targeted_entity.is_none() {
            self.world.raycast(
                [camera.position.x, camera.position.y, camera.position.z],
                [direction.x, direction.y, direction.z],
                if self.game_mode == 1 { 5.0 } else { 4.5 },
            )
        } else {
            None
        };
        let target = block_hit.map(|hit| MiningTarget {
            position: BlockPosition::new(hit.position.x, hit.position.y, hit.position.z),
            face: hit.face,
            block_state: self.world.block_state_or_air(hit.position),
        });
        let held_item = self.usability.inventory().selected_hotbar_item();
        let (haste, fatigue) = self.shell.mining_effects();
        let aqua_affinity = self
            .usability
            .inventory()
            .inventory_window()
            .slots
            .get(5..9)
            .unwrap_or(&[])
            .iter()
            .filter_map(|item| item.as_ref())
            .any(|item| item.enchantment_level(6) > 0);
        let context = MiningContext {
            game_mode: self.game_mode,
            on_ground: output.simulation.player.on_ground,
            underwater: self.world.eye_in_water([
                camera.position.x,
                camera.position.y,
                camera.position.z,
            ]),
            aqua_affinity,
            haste,
            fatigue,
            adventure_can_destroy: target.is_some_and(|target| {
                rmc_game::mining::adventure_can_destroy(target.block_state, held_item.as_ref())
            }),
            held_item,
        };
        let mining = self.mining.update(
            actions.attack_pressed && targeted_entity.is_none(),
            actions.attack_held,
            output.ticks_run,
            target,
            &context,
        );
        for packet in mining.packets {
            self.queue_play_packet(&packet)?;
        }
        for position in mining.destroyed {
            let changes = self
                .world
                .apply_block_change(&rmc_net::codec::play::BlockChangePacket {
                    position,
                    block_state_id: 0,
                })
                .map_err(|error| format!("failed to predict block removal: {error:?}"))?;
            self.mesh_pipeline.apply_world_changes(&changes);
        }

        if actions.use_pressed {
            let held_item = self.usability.inventory().selected_hotbar_item();
            let origin = output.render.camera.position;
            let direction = forward_vector(output.render.camera.yaw, output.render.camera.pitch);
            if let Some(target) = targeted_entity {
                for packet in self.combat.interact_entity(target.entity_id) {
                    self.queue_play_packet(&packet)?;
                }
            } else if let Some(hit) = self.world.raycast(
                [origin.x, origin.y, origin.z],
                [direction.x, direction.y, direction.z],
                4.5,
            ) {
                self.queue_play_packet(&block_use_packet(hit, held_item))?;
                self.queue_play_packet(&PlayServerboundPacket::Animation(AnimationPacket))?;
            } else {
                for packet in self.combat.start_using_item(held_item) {
                    self.queue_play_packet(&packet)?;
                }
            }
        }

        if actions.use_released {
            for packet in self.combat.release_using_item() {
                self.queue_play_packet(&packet)?;
            }
        }

        if actions.close_window_pressed {
            if let Some(packet) = self.usability.close_open_window() {
                self.queue_play_packet(&packet)?;
            }
        }

        let cycle = self.mesh_pipeline.rebuild_dirty_budgeted(&self.world);
        let world_render = self.mesh_pipeline.snapshot_for_camera(
            &self.world,
            &output.render,
            FrustumConfig::debug_default(),
        );
        let render_telemetry =
            self.mesh_pipeline
                .telemetry_snapshot(&self.world, &world_render, &cycle);
        self.render_history.record(render_telemetry);

        let usability_snapshot = self.usability.snapshot();
        let hud = PvPHud::from_snapshot(&usability_snapshot);

        self.telemetry.frames += 1;
        self.telemetry.total_ticks_run += output.ticks_run;
        self.telemetry.render = self.render_history.snapshot();
        self.last_output = Some(output);
        self.last_world_render = Some(world_render);
        self.last_hud = Some(hud);
        self.last_usability_snapshot = Some(usability_snapshot);
        self.last_targeted_entity = targeted_entity;

        self.flush_if_pending()?;
        Ok(())
    }

    pub fn summary(&self) -> &LiveRuntimeSummary {
        &self.summary
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn output(&self) -> Option<&ShellAdvanceOutput> {
        self.last_output.as_ref()
    }

    pub fn entity_tracker(&self) -> &EntityTracker {
        &self.entity_tracker
    }

    pub fn is_spectator(&self) -> bool {
        self.game_mode == 3
    }

    pub fn can_see_friendly_invisible(&self, target: &str) -> bool {
        self.usability
            .friendly_invisibles_visible(&self.username, target)
    }

    pub fn world_render(&self) -> Option<&WorldRenderSnapshot> {
        self.last_world_render.as_ref()
    }

    pub fn daylight(&self, partial_ticks: f32) -> Option<rmc_render::daylight::Daylight> {
        self.world.config().has_sky_light.then(|| {
            rmc_render::daylight::Daylight::calculate(
                self.world.time(),
                partial_ticks,
                self.world.weather().rain_strength,
                self.world.weather().thunder_strength(),
            )
        })
    }

    pub fn loaded_chunk_count(&self) -> usize {
        self.world.metrics().loaded_chunks
    }

    pub fn hud(&self) -> Option<&PvPHud> {
        self.last_hud.as_ref()
    }

    pub fn usability_snapshot(&self) -> Option<&UsabilitySnapshot> {
        self.last_usability_snapshot.as_ref()
    }

    pub fn combat_snapshot(&self) -> CombatSnapshot {
        self.combat.snapshot()
    }

    pub fn request_respawn(&mut self) -> Result<bool, String> {
        let Some(packet) = self.combat.request_respawn() else {
            return Ok(false);
        };
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()?;
        Ok(true)
    }

    pub fn inventory_state(&self) -> &InventoryState {
        self.usability.inventory()
    }

    pub fn packet_trace_lines(&self) -> &[String] {
        &self.trace.lines
    }

    pub fn take_movement_corrections(&mut self) -> Vec<String> {
        std::mem::take(&mut self.trace.movement_corrections)
    }

    pub fn player_entity_id(&self) -> Option<i32> {
        self.driver.session().snapshot().player_entity_id
    }

    pub fn targeted_entity(&self) -> Option<TargetedEntity> {
        self.last_targeted_entity
    }

    pub fn send_chat_message(&mut self, message: &str) -> Result<(), String> {
        let packet = self.usability.send_chat_message(message);
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()
    }

    pub fn click_window_slot(
        &mut self,
        window_id: u8,
        slot_id: i16,
        button: i8,
    ) -> Result<(), String> {
        if self.usability.inventory().pending_transactions().len() >= 128 {
            return Err("Waiting for server inventory acknowledgements".to_owned());
        }
        let packet = self
            .usability
            .inventory_mut()
            .queue_pickup_click(window_id, slot_id, button);
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()
    }

    pub fn transfer_window_slot(
        &mut self,
        window_id: u8,
        slot_id: i16,
        button: i8,
    ) -> Result<(), String> {
        if self.usability.inventory().pending_transactions().len() >= 128 {
            return Err("Waiting for server inventory acknowledgements".to_owned());
        }
        let packet = self
            .usability
            .inventory_mut()
            .queue_transfer_click(window_id, slot_id, button)
            .map_err(str::to_owned)?;
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()
    }

    pub fn clone_window_slot(&mut self, window_id: u8, slot_id: i16) -> Result<(), String> {
        if self.usability.inventory().pending_transactions().len() >= 128 {
            return Err("Waiting for server inventory acknowledgements".to_owned());
        }
        let packet = self
            .usability
            .inventory_mut()
            .queue_clone_click(window_id, slot_id, self.game_mode == 1)
            .map_err(str::to_owned)?;
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()
    }

    pub fn throw_window_slot(
        &mut self,
        window_id: u8,
        slot_id: i16,
        whole_stack: bool,
    ) -> Result<(), String> {
        if self.usability.inventory().pending_transactions().len() >= 128 {
            return Err("Waiting for server inventory acknowledgements".to_owned());
        }
        let packet = self
            .usability
            .inventory_mut()
            .queue_throw_click(window_id, slot_id, whole_stack)
            .map_err(str::to_owned)?;
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()
    }

    pub fn swap_window_slot_with_hotbar(
        &mut self,
        window_id: u8,
        slot_id: i16,
        hotbar: u8,
    ) -> Result<(), String> {
        if self.usability.inventory().pending_transactions().len() >= 128 {
            return Err("Waiting for server inventory acknowledgements".to_owned());
        }
        let packet = self
            .usability
            .inventory_mut()
            .queue_hotbar_swap(window_id, slot_id, hotbar)
            .map_err(str::to_owned)?;
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()
    }

    pub fn attack_entity(&mut self, entity_id: i32) -> Result<(), String> {
        for packet in self.combat.attack_entity(entity_id) {
            self.queue_play_packet(&packet)?;
        }
        self.flush_if_pending()
    }

    pub fn open_player_inventory(&mut self) -> Result<(), String> {
        let packet = self.usability.inventory_mut().open_player_inventory();
        self.last_usability_snapshot = Some(self.usability.snapshot());
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()
    }

    pub fn close_open_window(&mut self) -> Result<bool, String> {
        let Some(packet) = self.usability.close_open_window() else {
            return Ok(false);
        };
        self.queue_play_packet(&packet)?;
        self.flush_if_pending()?;
        Ok(true)
    }

    fn pump_network(&mut self) -> Result<(), String> {
        loop {
            let cycle = match self.transport.read_once(
                &mut self.driver,
                Some(&self.zlib),
                Some(&mut self.trace),
            ) {
                Ok(cycle) => cycle,
                Err(TransportError::Io(error)) if is_nonblocking_wait(&error) => break,
                Err(error) => return Err(format!("runtime transport failed: {error:?}")),
            };

            if cycle.reached_eof {
                self.summary.ended_by_eof = true;
                break;
            }

            if cycle.bytes_read == 0 {
                break;
            }

            for event in cycle.events {
                match event {
                    DriverEvent::InboundPlayPacket(packet) => {
                        if let PlayClientboundPacket::ChangeGameState(packet) = &packet {
                            if packet.reason == 3 {
                                let mode = packet.value as i32;
                                self.game_mode = if (0..=3).contains(&mode) {
                                    mode as u8
                                } else {
                                    0
                                };
                            }
                        }
                        self.shell.apply_player_packet(
                            &packet,
                            self.driver.session().snapshot().player_entity_id,
                        );
                        if let PlayClientboundPacket::PlayerPositionAndLook(correction) = &packet {
                            self.position_initialized = true;
                            self.trace.movement_corrections.push(format!("movement record=server_correction x={:.6} y={:.6} z={:.6} yaw={:.6} pitch={:.6} flags={}", correction.x, correction.y, correction.z, correction.yaw, correction.pitch, if correction.flags.bits() == 0 { "__".to_owned() } else { correction.flags.bits().to_string() }));
                        }
                        self.entity_tracker.apply_packet(&packet);
                        if let PlayClientboundPacket::SpawnPlayer(spawn) = &packet {
                            if let Some(name) = self.usability.player_name(&spawn.player_uuid) {
                                self.entity_tracker
                                    .set_profile_name(spawn.entity_id, name.to_owned());
                            }
                        }
                        apply_inbound_play_packet(
                            &packet,
                            self.driver.local_pose(),
                            self.driver.session().snapshot().player_entity_id,
                            &mut self.world,
                            &mut self.mesh_pipeline,
                            &mut self.combat,
                            &mut self.pending_simulation_events,
                        )?;
                    }
                    DriverEvent::IgnoredPacket { .. } => {
                        self.summary.ignored_packets += 1;
                    }
                    DriverEvent::SessionAction(action) => {
                        self.handle_session_action(action)?;
                    }
                }
            }

            self.flush_if_pending()?;

            if self.summary.disconnect_reason_json.is_some() {
                break;
            }
        }

        Ok(())
    }

    fn handle_session_action(&mut self, action: SessionAction) -> Result<(), String> {
        match action {
            SessionAction::EncryptionRequested(request) => {
                let account = self.online_account.as_ref().ok_or_else(|| {
                    "server requested online-mode encryption but the windowed client was started in offline mode"
                        .to_owned()
                })?;
                let material = build_login_encryption_response(&request)
                    .map_err(|error| format!("failed to build encryption response: {error:?}"))?;
                self.joiner
                    .join_server(account, &material.server_hash)
                    .map_err(|error| format!("failed to join Mojang session server: {error:?}"))?;

                let response = LoginServerboundPacket::EncryptionResponse(EncryptionResponse {
                    shared_secret: material.encrypted_shared_secret,
                    verify_token: material.encrypted_verify_token,
                });
                self.driver
                    .queue_login_packet(&response, Some(&self.zlib), Some(&mut self.trace))
                    .map_err(|error| format!("failed to queue encryption response: {error:?}"))?;
                self.flush_if_pending()?;
                self.transport
                    .enable_encryption(material.shared_secret)
                    .map_err(|error| format!("failed to enable transport encryption: {error:?}"))?;
                self.summary.encryption_enabled = true;
            }
            SessionAction::EnableCompression { .. } => {
                self.summary.compression_enabled = true;
            }
            SessionAction::EnterPlay { .. } => {
                self.summary.reached_play = true;
            }
            SessionAction::JoinedGame(packet) => {
                self.game_mode = packet.game_mode;
                self.dimension = Some(i32::from(packet.dimension));
                self.mining.reset();
                self.summary.joined_game = true;
                self.position_initialized = false;
                self.world =
                    WorldSnapshot::new(world_config_for_dimension(i32::from(packet.dimension)));
                self.entity_tracker.clear();
                self.mesh_pipeline = ChunkMeshPipeline::with_config(self.mesh_config);
            }
            SessionAction::Disconnected { reason_json } => {
                self.summary.disconnect_reason_json = Some(reason_json);
            }
            SessionAction::Respawned(packet) => {
                self.game_mode = packet.game_mode;
                self.mining.reset();
                self.position_initialized = false;
                self.pending_simulation_events.clear();
                self.usability.inventory_mut().reset_for_respawn();
                if self.dimension != Some(packet.dimension) {
                    self.world = WorldSnapshot::new(world_config_for_dimension(packet.dimension));
                    self.mesh_pipeline = ChunkMeshPipeline::with_config(self.mesh_config);
                    self.last_world_render = None;
                }
                self.dimension = Some(packet.dimension);
                self.entity_tracker.clear();
                self.last_output = None;
                self.last_targeted_entity = None;
                self.last_usability_snapshot = None;
            }
            // The protocol driver already emits these acknowledgements exactly once.
            SessionAction::ReplyKeepAlive { .. } | SessionAction::TeleportCorrectionRequired(_) => {
            }
            SessionAction::WindowItemsUpdated(packet) => {
                let update = self.usability.inventory_mut().apply_window_items(&packet);
                for packet in update.outbound_packets {
                    self.queue_play_packet(&packet)?;
                }
            }
            SessionAction::TransactionConfirmed(packet) => {
                let update = self
                    .usability
                    .inventory_mut()
                    .apply_confirm_transaction(&packet);
                for packet in update.outbound_packets {
                    self.queue_play_packet(&packet)?;
                }
            }
            SessionAction::UsabilityPacket(packet) => {
                let update = self.usability.apply_play_packet(&packet);
                for packet in update.outbound_packets {
                    self.queue_play_packet(&packet)?;
                }
            }
            SessionAction::HealthUpdated(_) | SessionAction::EntityVelocityReceived(_) => {}
        }

        Ok(())
    }

    fn queue_play_packet(&mut self, packet: &PlayServerboundPacket) -> Result<(), String> {
        self.driver
            .queue_play_packet(packet, Some(&self.zlib), Some(&mut self.trace))
            .map_err(|error| format!("failed to queue play packet: {error:?}"))
    }

    fn flush_if_pending(&mut self) -> Result<(), String> {
        if self.driver.has_pending_outbound() || self.transport.has_pending_outbound() {
            self.transport
                .flush_outbound(&mut self.driver)
                .map_err(|error| format!("failed to flush outbound frames: {error:?}"))?;
        }
        Ok(())
    }
}

fn player_pose_from_output(output: &ShellAdvanceOutput) -> PlayerPose {
    PlayerPose {
        x: output.simulation.player.position.x,
        y: output.simulation.player.position.y,
        z: output.simulation.player.position.z,
        yaw: output.render.camera.yaw,
        pitch: output.render.camera.pitch,
    }
}

fn world_config_for_dimension(dimension: i32) -> WorldConfig {
    if dimension == 0 {
        WorldConfig::overworld()
    } else {
        WorldConfig::no_sky()
    }
}

fn is_nonblocking_wait(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

fn pick_target_entity(
    output: &ShellAdvanceOutput,
    entities: &EntityTracker,
    local_player_entity_id: Option<i32>,
    max_distance: f64,
    world: &WorldSnapshot,
) -> Option<TargetedEntity> {
    let origin = output.render.camera.position;
    let direction = forward_vector(output.render.camera.yaw, output.render.camera.pitch);
    let wall_distance = world
        .raycast(
            [origin.x, origin.y, origin.z],
            [direction.x, direction.y, direction.z],
            max_distance,
        )
        .map_or(max_distance, |hit| hit.distance);

    entities
        .players()
        .filter(|entity| Some(entity.entity_id) != local_player_entity_id)
        .filter_map(|entity| {
            ray_intersects_aabb(
                origin,
                direction,
                Vec3::new(
                    entity.position.x - 0.4,
                    entity.position.y - 0.1,
                    entity.position.z - 0.4,
                ),
                Vec3::new(
                    entity.position.x + 0.4,
                    entity.position.y + 1.9,
                    entity.position.z + 0.4,
                ),
                wall_distance,
            )
            .map(|distance| TargetedEntity {
                entity_id: entity.entity_id,
                uuid: entity.uuid,
                distance,
            })
        })
        .min_by(|left, right| left.distance.partial_cmp(&right.distance).unwrap())
}

fn ray_intersects_aabb(
    origin: Vec3,
    direction: Vec3,
    min: Vec3,
    max: Vec3,
    max_distance: f64,
) -> Option<f64> {
    let mut t_min = 0.0f64;
    let mut t_max = max_distance;

    for (origin_axis, direction_axis, min_axis, max_axis) in [
        (origin.x, direction.x, min.x, max.x),
        (origin.y, direction.y, min.y, max.y),
        (origin.z, direction.z, min.z, max.z),
    ] {
        if direction_axis.abs() <= f64::EPSILON {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }

        let inv = 1.0 / direction_axis;
        let mut axis_min = (min_axis - origin_axis) * inv;
        let mut axis_max = (max_axis - origin_axis) * inv;

        if axis_min > axis_max {
            std::mem::swap(&mut axis_min, &mut axis_max);
        }

        t_min = t_min.max(axis_min);
        t_max = t_max.min(axis_max);

        if t_min > t_max {
            return None;
        }
    }

    Some(t_min)
}

fn forward_vector(yaw: f32, pitch: f32) -> Vec3 {
    let yaw = yaw.to_radians();
    let pitch = pitch.to_radians();
    let pitch_cos = f64::from(pitch.cos());
    Vec3::new(
        -f64::from(yaw.sin()) * pitch_cos,
        -f64::from(pitch.sin()),
        f64::from(yaw.cos()) * pitch_cos,
    )
}

fn block_use_packet(
    hit: rmc_world::collision::BlockHit,
    held_item: rmc_net::codec::play::Slot,
) -> PlayServerboundPacket {
    PlayServerboundPacket::PlayerBlockPlacement(PlayerBlockPlacementPacket {
        position: BlockPosition {
            x: hit.position.x,
            y: hit.position.y,
            z: hit.position.z,
        },
        face: hit.face,
        held_item,
        cursor_x: (hit.point[0] - f64::from(hit.position.x)) as f32,
        cursor_y: (hit.point[1] - f64::from(hit.position.y)) as f32,
        cursor_z: (hit.point[2] - f64::from(hit.position.z)) as f32,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmc_net::codec::play::{BlockChangePacket, BlockPosition};

    #[test]
    fn join_without_initial_position_does_not_emit_movement() {
        use rmc_net::codec::login::{LoginClientboundPacket, LoginSuccess};
        use rmc_net::codec::play::JoinGamePacket;
        use rmc_net::compression::CompressionState;
        use rmc_net::framing::{encode_frame, FrameLimits};
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut config = LiveRuntimeConfig::offline("ReadyTest");
        config.server_host = "127.0.0.1".into();
        config.server_port = port;
        let mut runtime = LiveRuntime::connect(config).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        server
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let mut bytes = [0; 4096];
        server.read(&mut bytes).unwrap();
        let login = LoginClientboundPacket::LoginSuccess(LoginSuccess {
            uuid_string: "00000000-0000-0000-0000-000000000000".into(),
            username: "ReadyTest".into(),
        })
        .encode_packet()
        .unwrap()
        .packet_bytes();
        let join = PlayClientboundPacket::JoinGame(JoinGamePacket {
            entity_id: 1,
            game_mode: 0,
            hardcore: false,
            dimension: 0,
            difficulty: 1,
            max_players: 20,
            level_type: "default".into(),
            reduced_debug_info: false,
        })
        .encode_packet()
        .unwrap()
        .packet_bytes();
        for packet in [login, join] {
            server
                .write_all(
                    &encode_frame(
                        &packet,
                        CompressionState::Disabled,
                        None,
                        FrameLimits::default(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        for _ in 0..20 {
            runtime
                .step(
                    Duration::from_millis(50),
                    &InputFrame::default(),
                    &RuntimeActionInput::default(),
                )
                .unwrap();
            if runtime.summary().joined_game {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(runtime.summary().joined_game);
        assert!(
            runtime.output().is_none(),
            "must await first server position before ticking"
        );
        use rmc_net::codec::play::{EntityEquipmentPacket, ItemStack};
        for packet in [
            PlayClientboundPacket::SpawnPlayer(SpawnPlayerPacket {
                entity_id: 7,
                player_uuid: [7; 16],
                x: 0,
                y: 0,
                z: 0,
                yaw: 0,
                pitch: 0,
                held_item: 0,
                metadata: vec![127],
            }),
            PlayClientboundPacket::EntityEquipment(EntityEquipmentPacket {
                entity_id: 7,
                slot: 4,
                item: Some(ItemStack::simple(310, 1, 17)),
            }),
            PlayClientboundPacket::ChangeGameState(rmc_net::codec::play::ChangeGameStatePacket {
                reason: 7,
                value: 0.75,
            }),
        ] {
            server
                .write_all(
                    &encode_frame(
                        &packet.encode_packet().unwrap().packet_bytes(),
                        CompressionState::Disabled,
                        None,
                        FrameLimits::default(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        for _ in 0..30 {
            runtime
                .step(
                    Duration::from_millis(50),
                    &InputFrame::default(),
                    &RuntimeActionInput::default(),
                )
                .unwrap();
            if runtime.world.weather().rain_strength == 0.75
                && runtime
                    .entity_tracker
                    .players()
                    .any(|p| p.equipment[4].is_some())
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            runtime.entity_tracker.players().next().unwrap().equipment[4],
            Some(ItemStack::simple(310, 1, 17))
        );
        assert_eq!(runtime.world.weather().rain_strength, 0.75);
    }

    #[test]
    fn block_interaction_encodes_selected_face_and_local_hit_coordinates() {
        let hit = rmc_world::collision::BlockHit {
            position: rmc_world::BlockPos::new(-1, 64, 2),
            distance: 1.5,
            face: 2,
            point: [-0.5, 64.75, 2.0],
        };
        let PlayServerboundPacket::PlayerBlockPlacement(packet) = block_use_packet(hit, None)
        else {
            panic!("expected placement");
        };
        assert_eq!(packet.position, BlockPosition { x: -1, y: 64, z: 2 });
        assert_eq!(packet.face, 2);
        assert_eq!(
            [packet.cursor_x, packet.cursor_y, packet.cursor_z],
            [0.5, 0.75, 0.0]
        );
    }

    #[test]
    fn death_received_over_tcp_emits_one_respawn_and_resets_on_server_response() {
        use rmc_net::codec::login::{LoginClientboundPacket, LoginSuccess};
        use rmc_net::codec::play::{
            JoinGamePacket, PlayerPositionAndLookPacket, PositionLookFlags, RespawnPacket,
            UpdateHealthPacket,
        };
        use rmc_net::compression::CompressionState;
        use rmc_net::framing::{encode_frame, FrameDecoder, FrameLimits};
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut config = LiveRuntimeConfig::offline("RespawnTest");
        config.server_host = "127.0.0.1".into();
        config.server_port = listener.local_addr().unwrap().port();
        let mut runtime = LiveRuntime::connect(config).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        server.set_nonblocking(true).unwrap();
        let login = LoginClientboundPacket::LoginSuccess(LoginSuccess {
            uuid_string: "00000000-0000-0000-0000-000000000000".into(),
            username: "RespawnTest".into(),
        })
        .encode_packet()
        .unwrap()
        .packet_bytes();
        let join = PlayClientboundPacket::JoinGame(JoinGamePacket {
            entity_id: 1,
            game_mode: 0,
            hardcore: false,
            dimension: 0,
            difficulty: 1,
            max_players: 20,
            level_type: "default".into(),
            reduced_debug_info: false,
        })
        .encode_packet()
        .unwrap()
        .packet_bytes();
        let position = PlayClientboundPacket::PlayerPositionAndLook(PlayerPositionAndLookPacket {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            yaw: 0.0,
            pitch: 0.0,
            flags: PositionLookFlags::from_bits(0),
        })
        .encode_packet()
        .unwrap()
        .packet_bytes();
        let death = PlayClientboundPacket::UpdateHealth(UpdateHealthPacket {
            health: 0.0,
            food_level: 20,
            saturation: 5.0,
        })
        .encode_packet()
        .unwrap()
        .packet_bytes();
        for bytes in [login, join, position, death] {
            server
                .write_all(
                    &encode_frame(
                        &bytes,
                        CompressionState::Disabled,
                        None,
                        FrameLimits::default(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        for _ in 0..60 {
            runtime
                .step(
                    Duration::from_millis(50),
                    &InputFrame::default(),
                    &RuntimeActionInput::default(),
                )
                .unwrap();
            if runtime.combat_snapshot().death_ticks >= 20 {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(runtime.combat_snapshot().health, 0.0);
        assert!(runtime.request_respawn().unwrap());
        assert!(!runtime.request_respawn().unwrap());
        let mut decoder = FrameDecoder::new(CompressionState::Disabled);
        let mut bytes = [0; 8192];
        let mut requests = 0;
        for _ in 0..30 {
            match server.read(&mut bytes) {
                Ok(count) => decoder.queue_bytes(&bytes[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => panic!("{error}"),
            }
            while let Some(frame) = decoder.try_next_frame(None).unwrap() {
                if frame.packet_bytes == [0x16, 0] {
                    requests += 1;
                }
            }
            if requests > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(requests, 1);
        runtime
            .world
            .apply_block_change(&rmc_net::codec::play::BlockChangePacket {
                position: rmc_net::codec::play::BlockPosition::new(0, 64, 0),
                block_state_id: 16,
            })
            .unwrap();
        runtime
            .usability
            .inventory_mut()
            .sync_selected_hotbar_slot(8);
        let respawn = PlayClientboundPacket::Respawn(RespawnPacket {
            dimension: 0,
            difficulty: 1,
            game_mode: 0,
            level_type: "default".into(),
        })
        .encode_packet()
        .unwrap()
        .packet_bytes();
        server
            .write_all(
                &encode_frame(
                    &respawn,
                    CompressionState::Disabled,
                    None,
                    FrameLimits::default(),
                )
                .unwrap(),
            )
            .unwrap();
        for _ in 0..30 {
            runtime
                .step(
                    Duration::from_millis(50),
                    &InputFrame::default(),
                    &RuntimeActionInput::default(),
                )
                .unwrap();
            if !runtime.combat_snapshot().respawn_requested {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(runtime.combat_snapshot().health, 20.0);
        assert!(!runtime.combat_snapshot().respawn_requested);
        assert_eq!(
            runtime
                .world
                .block_state_or_air(rmc_world::BlockPos::new(0, 64, 0)),
            16
        );
        assert_eq!(runtime.usability.inventory().selected_hotbar_slot(), 0);
        assert!(!runtime.position_initialized);
        runtime
            .handle_session_action(SessionAction::Respawned(RespawnPacket {
                dimension: -1,
                difficulty: 1,
                game_mode: 0,
                level_type: "default".into(),
            }))
            .unwrap();
        assert_eq!(runtime.world.metrics().loaded_chunks, 0);
        assert!(!runtime.world.config().has_sky_light);
        assert_eq!(runtime.dimension, Some(-1));
    }

    #[test]
    #[ignore = "requires a user-authorized local official 1.8.9 offline server with VanillaProbe operator"]
    fn official_server_confirms_mining_and_respawn() {
        let mut config = LiveRuntimeConfig::offline("VanillaProbe");
        config.server_host = "127.0.0.1".into();
        config.server_port = std::env::var("RMC_VANILLA_PORT")
            .expect("Set RMC_VANILLA_PORT for the isolated test server")
            .parse()
            .unwrap();
        let mut runtime = LiveRuntime::connect(config).unwrap();
        fn advance(runtime: &mut LiveRuntime, actions: &RuntimeActionInput) {
            runtime
                .step(Duration::from_millis(50), &InputFrame::default(), actions)
                .unwrap();
            std::thread::sleep(Duration::from_millis(50));
        }
        for _ in 0..300 {
            advance(&mut runtime, &RuntimeActionInput::default());
            if runtime.output().is_some() {
                break;
            }
        }
        assert!(
            runtime.output().is_some(),
            "server did not initialize position"
        );
        // Login position can precede the first health synchronization.
        for _ in 0..20 {
            advance(&mut runtime, &RuntimeActionInput::default());
        }
        if runtime.combat_snapshot().health <= 0.0 {
            for _ in 0..40 {
                advance(&mut runtime, &RuntimeActionInput::default());
            }
            assert!(runtime.request_respawn().unwrap());
            for _ in 0..100 {
                advance(&mut runtime, &RuntimeActionInput::default());
                if runtime.combat_snapshot().health > 0.0 && runtime.position_initialized {
                    break;
                }
            }
        }
        runtime
            .send_chat_message("/tp VanillaProbe 0.5 4 0.5 0 0")
            .unwrap();
        for _ in 0..200 {
            advance(&mut runtime, &RuntimeActionInput::default());
            if runtime
                .world
                .chunk(rmc_world::BlockPos::new(0, 0, 0).chunk_pos())
                .is_some()
            {
                break;
            }
        }
        for command in [
            "/gamerule keepInventory true",
            "/setblock 0 63 0 stone",
            "/setblock 0 65 2 stone",
            "/clear VanillaProbe",
            "/give VanillaProbe diamond_pickaxe",
            "/tp VanillaProbe 0.5 64 0.5 0 0",
        ] {
            runtime.send_chat_message(command).unwrap();
        }
        for _ in 0..200 {
            advance(&mut runtime, &RuntimeActionInput::default());
            let ready = runtime
                .usability
                .inventory()
                .selected_hotbar_item()
                .is_some_and(|item| item.item_id == 278)
                && runtime
                    .world
                    .block_state_or_air(rmc_world::BlockPos::new(0, 65, 2))
                    >> 4
                    == 1
                && runtime.output().is_some_and(|output| {
                    (output.simulation.player.position.x - 0.5).abs() < 0.001
                });
            if ready {
                break;
            }
        }
        assert_eq!(
            runtime
                .usability
                .inventory()
                .selected_hotbar_item()
                .unwrap()
                .item_id,
            278
        );
        assert_eq!(
            runtime
                .world
                .block_state_or_air(rmc_world::BlockPos::new(0, 65, 2))
                >> 4,
            1
        );
        for (source, hotbar, target) in [(36, 1, 37), (37, 0, 36)] {
            runtime
                .swap_window_slot_with_hotbar(0, source, hotbar)
                .unwrap();
            for _ in 0..100 {
                advance(&mut runtime, &RuntimeActionInput::default());
                if runtime
                    .usability
                    .inventory()
                    .inventory_window()
                    .slot(target)
                    .and_then(|item| item.as_ref())
                    .is_some_and(|item| item.item_id == 278)
                    && runtime
                        .usability
                        .inventory()
                        .pending_transactions()
                        .is_empty()
                {
                    break;
                }
            }
            assert_eq!(
                runtime
                    .usability
                    .inventory()
                    .inventory_window()
                    .slot(target)
                    .and_then(|item| item.as_ref())
                    .map(|item| item.item_id),
                Some(278),
                "official server did not apply number-key swap"
            );
            assert!(runtime
                .usability
                .inventory()
                .pending_transactions()
                .is_empty());
            let previous_chat_count = runtime.usability.snapshot().chat_lines.len();
            runtime.send_chat_message(&format!(r#"/testfor VanillaProbe {{Inventory:[{{Slot:{hotbar}b,id:"minecraft:diamond_pickaxe"}}]}}"#)).unwrap();
            let mut confirmed = false;
            for _ in 0..100 {
                advance(&mut runtime, &RuntimeActionInput::default());
                confirmed = runtime
                    .usability
                    .snapshot()
                    .chat_lines
                    .iter()
                    .skip(previous_chat_count)
                    .any(|line| line.message_json.contains("commands.testfor.success"));
                if confirmed {
                    break;
                }
            }
            assert!(
                confirmed,
                "official server did not confirm the swapped hotbar slot"
            );
        }
        runtime
            .send_chat_message(
                "/replaceitem entity VanillaProbe slot.inventory.0 minecraft:stone 12",
            )
            .unwrap();
        for _ in 0..100 {
            advance(&mut runtime, &RuntimeActionInput::default());
            if runtime
                .usability
                .inventory()
                .inventory_window()
                .slot(9)
                .and_then(|s| s.as_ref())
                .is_some_and(|s| s.item_id == 1 && s.count == 12)
            {
                break;
            }
        }
        assert_eq!(
            runtime
                .usability
                .inventory()
                .inventory_window()
                .slot(9)
                .and_then(|s| s.as_ref())
                .map(|s| s.count),
            Some(12)
        );
        for (source, target, nbt_slot) in [(9, 37, 1), (37, 9, 9)] {
            runtime.transfer_window_slot(0, source, 0).unwrap();
            for _ in 0..100 {
                advance(&mut runtime, &RuntimeActionInput::default());
                if runtime
                    .usability
                    .inventory()
                    .pending_transactions()
                    .is_empty()
                {
                    break;
                }
            }
            assert!(runtime
                .usability
                .inventory()
                .pending_transactions()
                .is_empty());
            assert_eq!(
                runtime
                    .usability
                    .inventory()
                    .inventory_window()
                    .slot(target)
                    .and_then(|s| s.as_ref())
                    .map(|s| (s.item_id, s.count)),
                Some((1, 12))
            );
            let previous_chat_count = runtime.usability.snapshot().chat_lines.len();
            runtime.send_chat_message(&format!(r#"/testfor VanillaProbe {{Inventory:[{{Slot:{nbt_slot}b,id:"minecraft:stone",Count:12b}}]}}"#)).unwrap();
            let mut confirmed = false;
            for _ in 0..100 {
                advance(&mut runtime, &RuntimeActionInput::default());
                confirmed = runtime
                    .usability
                    .snapshot()
                    .chat_lines
                    .iter()
                    .skip(previous_chat_count)
                    .any(|line| line.message_json.contains("commands.testfor.success"));
                if confirmed {
                    break;
                }
            }
            assert!(confirmed, "official server did not confirm shift transfer");
        }
        for (whole, count) in [(false, 11), (true, 0)] {
            runtime.throw_window_slot(0, 9, whole).unwrap();
            for _ in 0..100 {
                advance(&mut runtime, &RuntimeActionInput::default());
                if runtime
                    .usability
                    .inventory()
                    .pending_transactions()
                    .is_empty()
                {
                    break;
                }
            }
            assert!(runtime
                .usability
                .inventory()
                .pending_transactions()
                .is_empty());
            let previous_chat_count = runtime.usability.snapshot().chat_lines.len();
            let command = if count == 0 {
                r#"/testfor VanillaProbe {Inventory:[{Slot:9b,id:"minecraft:stone"}]}"#.to_owned()
            } else {
                format!(
                    r#"/testfor VanillaProbe {{Inventory:[{{Slot:9b,id:"minecraft:stone",Count:{count}b}}]}}"#
                )
            };
            runtime.send_chat_message(&command).unwrap();
            let expected = if count == 0 {
                "commands.testfor.failure"
            } else {
                "commands.testfor.success"
            };
            let mut confirmed = false;
            for _ in 0..100 {
                advance(&mut runtime, &RuntimeActionInput::default());
                confirmed = runtime
                    .usability
                    .snapshot()
                    .chat_lines
                    .iter()
                    .skip(previous_chat_count)
                    .any(|line| line.message_json.contains(expected));
                if confirmed {
                    break;
                }
            }
            assert!(
                confirmed,
                "official server inventory did not confirm throw mode"
            );
            assert_eq!(
                runtime
                    .usability
                    .inventory()
                    .inventory_window()
                    .slot(9)
                    .and_then(|s| s.as_ref())
                    .map_or(0, |s| s.count),
                count
            );
        }
        for tick in 0..40 {
            advance(
                &mut runtime,
                &RuntimeActionInput {
                    attack_pressed: tick == 0,
                    attack_held: true,
                    ..RuntimeActionInput::default()
                },
            );
            if runtime
                .world
                .block_state_or_air(rmc_world::BlockPos::new(0, 65, 2))
                == 0
            {
                break;
            }
        }
        assert_eq!(
            runtime
                .world
                .block_state_or_air(rmc_world::BlockPos::new(0, 65, 2)),
            0,
            "client did not complete mining"
        );
        runtime
            .send_chat_message("/testforblock 0 65 2 air")
            .unwrap();
        let mut confirmed = false;
        for _ in 0..100 {
            advance(&mut runtime, &RuntimeActionInput::default());
            confirmed = runtime
                .usability
                .snapshot()
                .chat_lines
                .iter()
                .any(|line| line.message_json.contains("commands.testforblock.success"));
            if confirmed {
                break;
            }
        }
        assert!(
            confirmed,
            "official server did not confirm the block was air"
        );
        for command in [
            "/setblock 0 65 2 furnace 2",
            "/replaceitem block 0 65 2 slot.container.0 minecraft:iron_ore 1",
            "/replaceitem block 0 65 2 slot.container.1 minecraft:coal 1",
        ] {
            runtime.send_chat_message(command).unwrap();
        }

        for _ in 0..100 {
            advance(&mut runtime, &RuntimeActionInput::default());
            let id = runtime
                .world
                .block_state_or_air(rmc_world::BlockPos::new(0, 65, 2))
                >> 4;
            if matches!(id, 61 | 62) {
                break;
            }
        }
        advance(
            &mut runtime,
            &RuntimeActionInput {
                use_pressed: true,
                ..RuntimeActionInput::default()
            },
        );
        let mut furnace_progress = false;
        for _ in 0..250 {
            advance(&mut runtime, &RuntimeActionInput::default());
            furnace_progress = runtime
                .usability
                .inventory()
                .open_window()
                .is_some_and(|window| {
                    window
                        .metadata
                        .as_ref()
                        .is_some_and(|m| m.inventory_type == "minecraft:furnace")
                        && window.properties.get(&0).is_some_and(|v| *v > 0)
                        && window.properties.get(&2).is_some_and(|v| *v > 0)
                        && window.properties.get(&3) == Some(&200)
                });
            if furnace_progress {
                break;
            }
        }
        assert!(
            furnace_progress,
            "official server furnace properties did not reach the runtime"
        );
        runtime.close_open_window().unwrap();
        runtime.send_chat_message("/kill").unwrap();
        for _ in 0..100 {
            advance(&mut runtime, &RuntimeActionInput::default());
            if runtime.combat_snapshot().death_ticks >= 20 {
                break;
            }
        }
        assert_eq!(runtime.combat_snapshot().health, 0.0);
        assert!(runtime.request_respawn().unwrap());
        for _ in 0..200 {
            advance(&mut runtime, &RuntimeActionInput::default());
            if runtime.combat_snapshot().health > 0.0 && runtime.position_initialized {
                break;
            }
        }
        assert_eq!(runtime.combat_snapshot().health, 20.0);
        assert!(
            runtime.position_initialized,
            "official server did not initialize the respawn position"
        );
        assert!(runtime.summary.disconnect_reason_json.is_none());
        println!("official 1.8.9: number-key swaps, shift transfers, single/stack throws, furnace progress, server-confirmed stone mining, death and respawn passed");
        if std::env::var("RMC_STOP_TEST_SERVER").as_deref() == Ok("1") {
            runtime.send_chat_message("/stop").unwrap();
            for _ in 0..100 {
                if let Err(error) = runtime.step(
                    Duration::from_millis(50),
                    &InputFrame::default(),
                    &RuntimeActionInput::default(),
                ) {
                    // The server closes active sockets while processing /stop.
                    assert!(
                        error.contains("ConnectionAborted") || error.contains("ConnectionReset"),
                        "unexpected shutdown error: {error}"
                    );
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
                if runtime.summary.ended_by_eof || runtime.summary.disconnect_reason_json.is_some()
                {
                    break;
                }
            }
        }
    }

    fn target_at(z: f64) -> EntityTracker {
        let mut entities = EntityTracker::default();
        entities.apply_spawn_player(&SpawnPlayerPacket {
            entity_id: 2,
            player_uuid: [2; 16],
            x: 0,
            y: 0,
            z: (z * 32.0) as i32,
            yaw: 0,
            pitch: 0,
            held_item: 0,
            metadata: vec![127],
        });
        entities
    }

    #[test]
    fn survival_targeting_enforces_reach_and_wall_occlusion() {
        let mut shell = ClientShell::new(ClientShellConfig::vanilla());
        let output = shell.advance(Duration::ZERO, &InputFrame::default());
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        assert!(pick_target_entity(&output, &target_at(2.5), Some(1), 3.0, &world).is_some());
        assert!(pick_target_entity(&output, &target_at(3.5), Some(1), 3.0, &world).is_none());
        world
            .apply_block_change(&BlockChangePacket {
                position: BlockPosition { x: 0, y: 1, z: 1 },
                block_state_id: 16,
            })
            .unwrap();
        assert!(pick_target_entity(&output, &target_at(2.5), Some(1), 3.0, &world).is_none());
    }
}
