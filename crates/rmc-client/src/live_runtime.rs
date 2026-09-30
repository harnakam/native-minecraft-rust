use crate::shell::{ClientShell, ClientShellConfig, ShellAdvanceOutput};
use rmc_game::combat::{CombatConfig, CombatSnapshot, CombatState};
use rmc_game::input::InputFrame;
use rmc_game::inventory::InventoryState;
use rmc_game::player::Vec3;
use rmc_game::simulation::SimulationEvent;
use rmc_game::usability::{UsabilitySnapshot, UsabilityState};
use rmc_net::address::resolve_connect_target;
use rmc_net::auth::{MojangSessionJoiner, OnlineAccount, SessionJoiner};
use rmc_net::codec::login::{EncryptionResponse, LoginServerboundPacket};
use rmc_net::codec::play::{
    AnimationPacket, BlockPosition, EntityHeadLookPacket, EntityLookMovePacket, EntityLookPacket,
    EntityRelativeMovePacket, EntityTeleportPacket, PlayClientboundPacket, PlayServerboundPacket,
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
use std::collections::BTreeMap;
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackedPlayerEntity {
    pub entity_id: i32,
    pub uuid: [u8; 16],
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
    pub on_ground: bool,
    pub held_item: i16,
}

#[derive(Default)]
pub struct EntityTracker {
    players: BTreeMap<i32, TrackedPlayerEntity>,
}

impl EntityTracker {
    pub fn clear(&mut self) {
        self.players.clear();
    }

    pub fn players(&self) -> impl Iterator<Item = &TrackedPlayerEntity> {
        self.players.values()
    }

    pub fn apply_packet(&mut self, packet: &PlayClientboundPacket) {
        match packet {
            PlayClientboundPacket::SpawnPlayer(packet) => self.apply_spawn_player(packet),
            PlayClientboundPacket::DestroyEntities(packet) => {
                for entity_id in &packet.entity_ids {
                    self.players.remove(entity_id);
                }
            }
            PlayClientboundPacket::EntityRelativeMove(packet) => self.apply_relative_move(packet),
            PlayClientboundPacket::EntityLook(packet) => self.apply_entity_look(packet),
            PlayClientboundPacket::EntityLookMove(packet) => self.apply_entity_look_move(packet),
            PlayClientboundPacket::EntityTeleport(packet) => self.apply_entity_teleport(packet),
            PlayClientboundPacket::EntityHeadLook(packet) => self.apply_head_look(packet),
            _ => {}
        }
    }

    fn apply_spawn_player(&mut self, packet: &SpawnPlayerPacket) {
        let yaw = angle_to_degrees(packet.yaw);
        self.players.insert(
            packet.entity_id,
            TrackedPlayerEntity {
                entity_id: packet.entity_id,
                uuid: packet.player_uuid,
                position: Vec3::new(
                    f64::from(packet.x) / 32.0,
                    f64::from(packet.y) / 32.0,
                    f64::from(packet.z) / 32.0,
                ),
                yaw,
                pitch: angle_to_degrees(packet.pitch),
                head_yaw: yaw,
                on_ground: false,
                held_item: packet.held_item,
            },
        );
    }

    fn apply_relative_move(&mut self, packet: &EntityRelativeMovePacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.position.x += f64::from(packet.delta_x) / 32.0;
            entity.position.y += f64::from(packet.delta_y) / 32.0;
            entity.position.z += f64::from(packet.delta_z) / 32.0;
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_entity_look(&mut self, packet: &EntityLookPacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.yaw = angle_to_degrees(packet.yaw);
            entity.pitch = angle_to_degrees(packet.pitch);
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_entity_look_move(&mut self, packet: &EntityLookMovePacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.position.x += f64::from(packet.delta_x) / 32.0;
            entity.position.y += f64::from(packet.delta_y) / 32.0;
            entity.position.z += f64::from(packet.delta_z) / 32.0;
            entity.yaw = angle_to_degrees(packet.yaw);
            entity.pitch = angle_to_degrees(packet.pitch);
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_entity_teleport(&mut self, packet: &EntityTeleportPacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.position = Vec3::new(
                f64::from(packet.x) / 32.0,
                f64::from(packet.y) / 32.0,
                f64::from(packet.z) / 32.0,
            );
            entity.yaw = angle_to_degrees(packet.yaw);
            entity.pitch = angle_to_degrees(packet.pitch);
            entity.on_ground = packet.on_ground;
        }
    }

    fn apply_head_look(&mut self, packet: &EntityHeadLookPacket) {
        if let Some(entity) = self.players.get_mut(&packet.entity_id) {
            entity.head_yaw = angle_to_degrees(packet.head_yaw);
        }
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
    world: WorldSnapshot,
    entity_tracker: EntityTracker,
    mesh_pipeline: ChunkMeshPipeline,
    render_history: RenderTelemetryHistory,
    trace: RuntimeTrace,
    summary: LiveRuntimeSummary,
    pending_simulation_events: Vec<SimulationEvent>,
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
            world: WorldSnapshot::new(WorldConfig::overworld()),
            entity_tracker: EntityTracker::default(),
            mesh_pipeline: ChunkMeshPipeline::with_config(mesh_config),
            render_history: RenderTelemetryHistory::default(),
            trace: RuntimeTrace::default(),
            summary: LiveRuntimeSummary::default(),
            pending_simulation_events: Vec::new(),
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

        if self.summary.disconnect_reason_json.is_some() || self.summary.ended_by_eof {
            return Ok(());
        }

        if !self.summary.joined_game {
            self.flush_if_pending()?;
            return Ok(());
        }

        let frame_events = self.pending_simulation_events.clone();
        self.pending_simulation_events.clear();
        let output =
            self.shell
                .advance_in_world(frame_delta, frame_input, &frame_events, &self.world);

        for _ in 0..output.ticks_run {
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
            3.0,
            &self.world,
        );

        if actions.attack_pressed {
            if let Some(target) = targeted_entity {
                for packet in self.combat.attack_entity(target.entity_id) {
                    self.queue_play_packet(&packet)?;
                }
            }
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

    pub fn world_render(&self) -> Option<&WorldRenderSnapshot> {
        self.last_world_render.as_ref()
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

    pub fn attack_entity(&mut self, entity_id: i32) -> Result<(), String> {
        for packet in self.combat.attack_entity(entity_id) {
            self.queue_play_packet(&packet)?;
        }
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
                        if let PlayClientboundPacket::PlayerPositionAndLook(correction) = &packet {
                            self.trace.movement_corrections.push(format!("movement record=server_correction x={:.6} y={:.6} z={:.6} yaw={:.6} pitch={:.6} flags={}", correction.x, correction.y, correction.z, correction.yaw, correction.pitch, if correction.flags.bits() == 0 { "__".to_owned() } else { correction.flags.bits().to_string() }));
                        }
                        self.entity_tracker.apply_packet(&packet);
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
                self.summary.joined_game = true;
                self.world =
                    WorldSnapshot::new(world_config_for_dimension(i32::from(packet.dimension)));
                self.entity_tracker.clear();
                self.mesh_pipeline = ChunkMeshPipeline::with_config(self.mesh_config);
            }
            SessionAction::Disconnected { reason_json } => {
                self.summary.disconnect_reason_json = Some(reason_json);
            }
            SessionAction::Respawned(packet) => {
                self.world = WorldSnapshot::new(world_config_for_dimension(packet.dimension));
                self.entity_tracker.clear();
                self.mesh_pipeline = ChunkMeshPipeline::with_config(self.mesh_config);
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

fn apply_inbound_play_packet(
    packet: &PlayClientboundPacket,
    _local_pose: PlayerPose,
    player_entity_id: Option<i32>,
    world: &mut WorldSnapshot,
    mesh_pipeline: &mut ChunkMeshPipeline,
    combat: &mut CombatState,
    pending_simulation_events: &mut Vec<SimulationEvent>,
) -> Result<(), String> {
    if let Some(changes) = world
        .apply_play_packet(packet)
        .map_err(|error| format!("failed to apply world packet: {error:?}"))?
    {
        mesh_pipeline.apply_world_changes(&changes);
    }

    if let PlayClientboundPacket::PlayerPositionAndLook(packet) = packet {
        pending_simulation_events.push(SimulationEvent::Teleport {
            position: Vec3::new(packet.x, packet.y, packet.z),
            yaw: packet.yaw,
            pitch: packet.pitch,
            flags: packet.flags.bits(),
        });
    }

    let combat_update = combat.apply_play_packet(packet, player_entity_id);
    pending_simulation_events.extend(combat_update.simulation_events);
    Ok(())
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

fn angle_to_degrees(value: u8) -> f32 {
    (f32::from(value) * 360.0) / 256.0
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
