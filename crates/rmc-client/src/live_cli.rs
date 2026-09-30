use crate::shell::{ClientShell, ClientShellConfig, ShellAdvanceOutput};
use crate::verification_cli::{hypixel_gate_decision, GateDecision};
use rmc_game::combat::{CombatConfig, CombatSnapshot, CombatState};
use rmc_game::input::{InputFrame, PhysicalInput};
use rmc_game::player::Vec3;
use rmc_game::simulation::SimulationEvent;
use rmc_game::usability::{UsabilitySnapshot, UsabilityState};
use rmc_net::address::resolve_connect_target;
use rmc_net::auth::{MojangSessionJoiner, OnlineAccount, SessionJoiner};
use rmc_net::codec::login::{EncryptionResponse, LoginServerboundPacket};
use rmc_net::codec::play::PlayClientboundPacket;
use rmc_net::crypto::build_login_encryption_response;
use rmc_net::driver::{DriverEvent, HeadlessDriver, HeadlessDriverConfig};
use rmc_net::headless::AuthenticationMode;
use rmc_net::session::{PlayerPose, SessionAction};
use rmc_net::trace::{
    PacketTelemetryHistory, PacketTelemetrySnapshot, PacketTraceEvent, TraceSink,
};
use rmc_net::transport::{DriverTransport, TransportError};
use rmc_net::zlib::DefaultZlibCodec;
use rmc_render::{
    ChunkMeshPipeline, FrustumConfig, MeshBuildConfig, RenderTelemetryHistory,
    RenderTelemetrySnapshot, RenderTelemetrySummary,
};
use rmc_ui::PvPHud;
use rmc_world::{WorldConfig, WorldMetrics, WorldSnapshot};
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct LiveCliOptions {
    pub server_host: String,
    pub server_port: u16,
    pub username: String,
    pub profile_id: Option<String>,
    pub access_token: Option<String>,
    pub duration_secs: u64,
    pub frame_time_ms: u64,
    pub mesh_jobs_per_frame: usize,
    pub brand: String,
    pub locale: String,
    pub view_distance: i8,
    pub capture_mouse: bool,
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub sprint: bool,
    pub sneak: bool,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
    pub trace_path: Option<String>,
    pub stop_after_join: bool,
    pub unsafe_hypixel: bool,
}

impl LiveCliOptions {
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            server_host: "hypixel.net".to_owned(),
            server_port: 25565,
            username: "Player".to_owned(),
            profile_id: None,
            access_token: None,
            duration_secs: 30,
            frame_time_ms: 16,
            mesh_jobs_per_frame: 4,
            brand: "RustMinecraft".to_owned(),
            locale: "en_US".to_owned(),
            view_distance: 8,
            capture_mouse: true,
            forward: false,
            back: false,
            left: false,
            right: false,
            jump: false,
            sprint: false,
            sneak: false,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            trace_path: None,
            stop_after_join: false,
            unsafe_hypixel: false,
        };
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "live" => {}
                "--help" | "-h" => return Err(Self::usage()),
                "--server" => {
                    let (host, port) = parse_server_address(&next_value(&mut args, "--server")?)?;
                    options.server_host = host;
                    options.server_port = port;
                }
                "--username" => options.username = next_value(&mut args, "--username")?,
                "--uuid" => options.profile_id = Some(next_value(&mut args, "--uuid")?),
                "--access-token" => {
                    options.access_token = Some(next_value(&mut args, "--access-token")?)
                }
                "--duration-secs" => {
                    options.duration_secs = next_value(&mut args, "--duration-secs")?
                        .parse()
                        .map_err(|_| "invalid --duration-secs value".to_owned())?;
                }
                "--frame-ms" => {
                    options.frame_time_ms = next_value(&mut args, "--frame-ms")?
                        .parse()
                        .map_err(|_| "invalid --frame-ms value".to_owned())?;
                }
                "--mesh-jobs-per-frame" => {
                    options.mesh_jobs_per_frame = next_value(&mut args, "--mesh-jobs-per-frame")?
                        .parse()
                        .map_err(|_| "invalid --mesh-jobs-per-frame value".to_owned())?;
                }
                "--brand" => options.brand = next_value(&mut args, "--brand")?,
                "--locale" => options.locale = next_value(&mut args, "--locale")?,
                "--view-distance" => {
                    options.view_distance = next_value(&mut args, "--view-distance")?
                        .parse()
                        .map_err(|_| "invalid --view-distance value".to_owned())?;
                }
                "--no-capture-mouse" => options.capture_mouse = false,
                "--forward" => options.forward = true,
                "--back" => options.back = true,
                "--left" => options.left = true,
                "--right" => options.right = true,
                "--jump" => options.jump = true,
                "--sprint" => options.sprint = true,
                "--sneak" => options.sneak = true,
                "--mouse-dx" => {
                    options.mouse_delta_x = next_value(&mut args, "--mouse-dx")?
                        .parse()
                        .map_err(|_| "invalid --mouse-dx value".to_owned())?;
                }
                "--mouse-dy" => {
                    options.mouse_delta_y = next_value(&mut args, "--mouse-dy")?
                        .parse()
                        .map_err(|_| "invalid --mouse-dy value".to_owned())?;
                }
                "--trace" => options.trace_path = Some(next_value(&mut args, "--trace")?),
                "--stop-after-join" => options.stop_after_join = true,
                "--unsafe-hypixel" => options.unsafe_hypixel = true,
                other => return Err(format!("unknown argument: {other}\n\n{}", Self::usage())),
            }
        }
        if options.frame_time_ms == 0 {
            return Err("--frame-ms must be at least 1".to_owned());
        }
        if options.mesh_jobs_per_frame == 0 {
            return Err("--mesh-jobs-per-frame must be at least 1".to_owned());
        }
        match (&options.profile_id, &options.access_token) {
            (Some(_), Some(_)) | (None, None) => {}
            _ => {
                return Err(
                    "both --uuid and --access-token must be provided for online-mode login"
                        .to_owned(),
                )
            }
        }
        Ok(options)
    }

    pub fn authentication(&self) -> AuthenticationMode {
        match (&self.profile_id, &self.access_token) {
            (Some(profile_id), Some(access_token)) => AuthenticationMode::Online(OnlineAccount {
                username: self.username.clone(),
                profile_id: profile_id.clone(),
                access_token: access_token.clone(),
            }),
            _ => AuthenticationMode::Offline,
        }
    }

    pub fn frame_duration(&self) -> Duration {
        Duration::from_millis(self.frame_time_ms)
    }

    pub fn usage() -> String {
        [
            "usage:",
            "  rmc-client live --server HOST[:PORT] --username NAME [options]",
            "",
            "options:",
            "  --uuid PROFILE_UUID --access-token ACCESS_TOKEN",
            "  --duration-secs N",
            "  --frame-ms N",
            "  --mesh-jobs-per-frame N",
            "  --brand VALUE",
            "  --locale VALUE",
            "  --view-distance N",
            "  --forward --back --left --right --jump --sprint --sneak",
            "  --mouse-dx DX --mouse-dy DY",
            "  --trace PATH",
            "  --no-capture-mouse",
            "  --stop-after-join",
            "  --unsafe-hypixel",
        ]
        .join("\n")
    }

    fn pressed_inputs(&self) -> Vec<PhysicalInput> {
        let mut inputs = Vec::new();
        if self.forward {
            inputs.push(PhysicalInput::KeyW);
        }
        if self.back {
            inputs.push(PhysicalInput::KeyS);
        }
        if self.left {
            inputs.push(PhysicalInput::KeyA);
        }
        if self.right {
            inputs.push(PhysicalInput::KeyD);
        }
        if self.jump {
            inputs.push(PhysicalInput::Space);
        }
        if self.sprint {
            inputs.push(PhysicalInput::LeftControl);
        }
        if self.sneak {
            inputs.push(PhysicalInput::LeftShift);
        }
        inputs
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct LiveRunSummary {
    reached_play: bool,
    joined_game: bool,
    compression_enabled: bool,
    encryption_enabled: bool,
    ignored_packets: usize,
    disconnect_reason_json: Option<String>,
    ended_by_eof: bool,
    timed_out: bool,
    frames: usize,
    total_ticks_run: usize,
}

#[derive(Default)]
struct RuntimeTrace {
    lines: Vec<String>,
    telemetry: PacketTelemetryHistory,
}

impl RuntimeTrace {
    fn push_line(&mut self, line: String) {
        self.lines.push(line);
    }
}

impl TraceSink for RuntimeTrace {
    fn record(&mut self, event: PacketTraceEvent) {
        self.telemetry.record(&event);
        self.lines.push(event.summary_line());
    }
}

pub fn run_live_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = LiveCliOptions::parse(raw_args)?;
    if !options.unsafe_hypixel {
        match hypixel_gate_decision(&options.server_host) {
            GateDecision::Allowed => {}
            GateDecision::Blocked(reason) => return Err(reason),
        }
    }
    let authentication = options.authentication();
    let frame_duration = options.frame_duration();
    let frame_input = InputFrame {
        pressed_inputs: options.pressed_inputs(),
        mouse_delta_x: options.mouse_delta_x,
        mouse_delta_y: options.mouse_delta_y,
        ..InputFrame::default()
    };
    let mut shell = ClientShell::new(ClientShellConfig::vanilla());
    shell.set_mouse_captured(options.capture_mouse);
    let mut usability = UsabilityState::new();
    usability.settings_mut().locale = options.locale.clone();
    usability.settings_mut().view_distance = options.view_distance;
    let mut combat = CombatState::new(CombatConfig::vanilla());
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    let mesh_config = MeshBuildConfig {
        max_jobs_per_frame: options.mesh_jobs_per_frame,
        ..MeshBuildConfig::default()
    };
    let mut mesh_pipeline = ChunkMeshPipeline::with_config(mesh_config);
    let mut render_history = RenderTelemetryHistory::default();
    let mut trace = RuntimeTrace::default();
    let mut summary = LiveRunSummary::default();
    let mut pending_simulation_events = Vec::new();
    let mut last_output: Option<ShellAdvanceOutput> = None;
    let mut last_hud: Option<PvPHud> = None;
    let mut last_usability_snapshot: Option<UsabilitySnapshot> = None;
    let mut next_frame_deadline = None;

    let mut driver_config = HeadlessDriverConfig::vanilla_headless(
        47,
        options.server_host.clone(),
        options.server_port,
        options.username.clone(),
    );
    driver_config.client_brand = options.brand.clone();
    driver_config.client_settings.locale = options.locale.clone();
    driver_config.client_settings.view_distance = options.view_distance;

    let connect_target = resolve_connect_target(&options.server_host, options.server_port);
    let stream = TcpStream::connect((connect_target.host.as_str(), connect_target.port)).map_err(
        |error| {
            format!(
                "failed to connect to {}:{} (resolved {}:{}): {error}",
                options.server_host, options.server_port, connect_target.host, connect_target.port
            )
        },
    )?;
    stream
        .set_nodelay(true)
        .map_err(|error| format!("failed to enable TCP_NODELAY: {error}"))?;
    stream
        .set_read_timeout(Some(frame_duration))
        .map_err(|error| format!("failed to set read timeout: {error}"))?;

    let mut transport = DriverTransport::new(stream);
    let mut driver = HeadlessDriver::new(driver_config);
    let zlib = DefaultZlibCodec;
    let joiner = MojangSessionJoiner;
    let started = Instant::now();

    driver
        .bootstrap_login(Some(&zlib), Some(&mut trace))
        .map_err(|error| format!("failed to bootstrap login: {error:?}"))?;
    transport
        .flush_outbound(&mut driver)
        .map_err(|error| format!("failed to flush outbound login frames: {error:?}"))?;

    loop {
        if started.elapsed() >= Duration::from_secs(options.duration_secs) {
            summary.timed_out = true;
            break;
        }
        let cycle = match transport.read_once(&mut driver, Some(&zlib), Some(&mut trace)) {
            Ok(cycle) => cycle,
            Err(TransportError::Io(error)) if is_timeout(&error) => {
                flush_if_pending(&mut transport, &mut driver)?;
                drive_live_frame(
                    frame_duration,
                    &frame_input,
                    &mut shell,
                    &mut combat,
                    &mut usability,
                    &world,
                    &mut mesh_pipeline,
                    &mut driver,
                    &zlib,
                    &mut trace,
                    &mut render_history,
                    &mut summary,
                    &mut pending_simulation_events,
                    &mut last_output,
                    &mut last_hud,
                    &mut last_usability_snapshot,
                    &mut next_frame_deadline,
                )?;
                flush_if_pending(&mut transport, &mut driver)?;
                continue;
            }
            Err(error) => return Err(format!("live transport failed: {error:?}")),
        };
        if cycle.reached_eof {
            summary.ended_by_eof = true;
            break;
        }
        for event in cycle.events {
            match event {
                DriverEvent::InboundPlayPacket(packet) => {
                    shell
                        .apply_player_packet(&packet, driver.session().snapshot().player_entity_id);
                    apply_inbound_play_packet(
                        &packet,
                        driver.local_pose(),
                        driver.session().snapshot().player_entity_id,
                        &mut world,
                        &mut mesh_pipeline,
                        &mut combat,
                        &mut pending_simulation_events,
                    )?;
                    match &packet {
                        PlayClientboundPacket::JoinGame(packet) => {
                            world = WorldSnapshot::new(world_config_for_dimension(i32::from(
                                packet.dimension,
                            )));
                            mesh_pipeline = ChunkMeshPipeline::with_config(mesh_config);
                            next_frame_deadline = None;
                        }
                        PlayClientboundPacket::PlayerPositionAndLook(_) => {
                            if next_frame_deadline.is_none() {
                                next_frame_deadline = Some(Instant::now());
                            }
                        }
                        PlayClientboundPacket::Respawn(packet) => {
                            next_frame_deadline = None;
                            world =
                                WorldSnapshot::new(world_config_for_dimension(packet.dimension));
                            mesh_pipeline = ChunkMeshPipeline::with_config(mesh_config);
                        }
                        _ => {}
                    }
                    let usability_update = usability.apply_play_packet(&packet);
                    for packet in usability_update.outbound_packets {
                        driver
                            .queue_play_packet(&packet, Some(&zlib), Some(&mut trace))
                            .map_err(|error| {
                                format!("failed to queue usability packet: {error:?}")
                            })?;
                    }
                }
                DriverEvent::IgnoredPacket { .. } => summary.ignored_packets += 1,
                DriverEvent::SessionAction(action) => handle_session_action(
                    action,
                    &authentication,
                    &joiner,
                    &zlib,
                    &mut transport,
                    &mut driver,
                    &mut trace,
                    &mut summary,
                    &mut next_frame_deadline,
                )?,
            }
        }
        flush_if_pending(&mut transport, &mut driver)?;
        if summary.disconnect_reason_json.is_some() {
            break;
        }
        if options.stop_after_join && summary.joined_game {
            break;
        }
        drive_live_frame(
            frame_duration,
            &frame_input,
            &mut shell,
            &mut combat,
            &mut usability,
            &world,
            &mut mesh_pipeline,
            &mut driver,
            &zlib,
            &mut trace,
            &mut render_history,
            &mut summary,
            &mut pending_simulation_events,
            &mut last_output,
            &mut last_hud,
            &mut last_usability_snapshot,
            &mut next_frame_deadline,
        )?;
        flush_if_pending(&mut transport, &mut driver)?;
    }

    let world_metrics = world.metrics();
    let render_summary = render_history.snapshot();
    let packet_summary = trace.telemetry.snapshot();
    if let Some(trace_path) = &options.trace_path {
        write_lines(trace_path, &trace.lines)?;
    }
    print_summary(
        &summary,
        &world_metrics,
        &render_summary,
        &packet_summary,
        last_output.as_ref(),
        last_hud.as_ref(),
        last_usability_snapshot.as_ref(),
        &combat.snapshot(),
    )
}

fn flush_if_pending(
    transport: &mut DriverTransport<TcpStream>,
    driver: &mut HeadlessDriver,
) -> Result<(), String> {
    if driver.has_pending_outbound() || transport.has_pending_outbound() {
        transport
            .flush_outbound(driver)
            .map_err(|error| format!("failed to flush outbound frames: {error:?}"))?;
    }
    Ok(())
}

fn drive_live_frame(
    frame_duration: Duration,
    frame_input: &InputFrame,
    shell: &mut ClientShell,
    combat: &mut CombatState,
    usability: &mut UsabilityState,
    world: &WorldSnapshot,
    mesh_pipeline: &mut ChunkMeshPipeline,
    driver: &mut HeadlessDriver,
    zlib: &DefaultZlibCodec,
    trace: &mut RuntimeTrace,
    render_history: &mut RenderTelemetryHistory,
    summary: &mut LiveRunSummary,
    pending_simulation_events: &mut Vec<SimulationEvent>,
    last_output: &mut Option<ShellAdvanceOutput>,
    last_hud: &mut Option<PvPHud>,
    last_usability_snapshot: &mut Option<UsabilitySnapshot>,
    next_frame_deadline: &mut Option<Instant>,
) -> Result<(), String> {
    let Some(mut deadline) = *next_frame_deadline else {
        return Ok(());
    };

    while summary.joined_game && Instant::now() >= deadline {
        let frame_events = pending_simulation_events.clone();
        pending_simulation_events.clear();
        let output = shell.advance_in_world(frame_duration, frame_input, &frame_events, world);

        for _ in 0..output.ticks_run {
            combat.tick_feedback();
        }

        if let Some(packet) = usability
            .inventory_mut()
            .sync_selected_hotbar_slot(output.simulation.player.selected_hotbar_slot)
        {
            driver
                .queue_play_packet(&packet, Some(zlib), Some(trace))
                .map_err(|error| format!("failed to queue held-item sync: {error:?}"))?;
        }

        if let Some(player_entity_id) = driver.session().snapshot().player_entity_id {
            for packet in combat.sync_action_state(
                player_entity_id,
                output.simulation.player.sprinting,
                output.simulation.player.sneaking,
            ) {
                driver
                    .queue_play_packet(&packet, Some(zlib), Some(trace))
                    .map_err(|error| format!("failed to queue action-state packet: {error:?}"))?;
            }
        }

        for packet in &output.packets {
            driver
                .queue_play_packet(packet, Some(zlib), Some(trace))
                .map_err(|error| format!("failed to queue walking packet: {error:?}"))?;
        }
        driver.set_local_pose(player_pose_from_output(&output));

        let cycle = mesh_pipeline.rebuild_dirty_budgeted(world);
        let render_snapshot = mesh_pipeline.snapshot_for_camera(
            world,
            &output.render,
            FrustumConfig::debug_default(),
        );
        let render_telemetry = mesh_pipeline.telemetry_snapshot(world, &render_snapshot, &cycle);
        render_history.record(render_telemetry);

        let usability_snapshot = usability.snapshot();
        let hud = PvPHud::from_snapshot(&usability_snapshot);

        trace.push_line(render_live_frame_line(
            summary.frames as u64,
            &output,
            &world.metrics(),
            &render_telemetry,
            &combat.snapshot(),
            &usability_snapshot,
        ));

        summary.frames += 1;
        summary.total_ticks_run += output.ticks_run;
        *last_output = Some(output);
        *last_hud = Some(hud);
        *last_usability_snapshot = Some(usability_snapshot);
        deadline += frame_duration;
    }

    *next_frame_deadline = Some(deadline);
    Ok(())
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

    if let PlayClientboundPacket::Explosion(packet) = packet {
        if packet.motion.iter().all(|v| v.is_finite()) {
            pending_simulation_events.push(SimulationEvent::AddVelocity(Vec3::new(
                f64::from(packet.motion[0]),
                f64::from(packet.motion[1]),
                f64::from(packet.motion[2]),
            )));
        }
    }
    let combat_update = combat.apply_play_packet(packet, player_entity_id);
    pending_simulation_events.extend(combat_update.simulation_events);
    Ok(())
}

fn handle_session_action(
    action: SessionAction,
    authentication: &AuthenticationMode,
    joiner: &MojangSessionJoiner,
    zlib: &DefaultZlibCodec,
    transport: &mut DriverTransport<TcpStream>,
    driver: &mut HeadlessDriver,
    trace: &mut RuntimeTrace,
    summary: &mut LiveRunSummary,
    next_frame_deadline: &mut Option<Instant>,
) -> Result<(), String> {
    match action {
        SessionAction::EncryptionRequested(request) => {
            let account = match authentication {
                AuthenticationMode::Offline => return Err(
                    "online-mode login requested but no --uuid/--access-token pair was provided"
                        .to_owned(),
                ),
                AuthenticationMode::Online(account) => account,
            };
            let material = build_login_encryption_response(&request)
                .map_err(|error| format!("failed to build encryption response: {error:?}"))?;
            joiner
                .join_server(account, &material.server_hash)
                .map_err(|error| format!("failed to join Mojang session server: {error:?}"))?;

            let response = LoginServerboundPacket::EncryptionResponse(EncryptionResponse {
                shared_secret: material.encrypted_shared_secret,
                verify_token: material.encrypted_verify_token,
            });
            driver
                .queue_login_packet(&response, Some(zlib), Some(trace))
                .map_err(|error| format!("failed to queue encryption response: {error:?}"))?;
            transport
                .flush_outbound(driver)
                .map_err(|error| format!("failed to flush encryption response: {error:?}"))?;
            transport
                .enable_encryption(material.shared_secret)
                .map_err(|error| format!("failed to enable transport encryption: {error:?}"))?;
            summary.encryption_enabled = true;
        }
        SessionAction::EnableCompression { .. } => {
            summary.compression_enabled = true;
        }
        SessionAction::EnterPlay { .. } => {
            summary.reached_play = true;
        }
        SessionAction::JoinedGame(_) => {
            summary.joined_game = true;
            *next_frame_deadline = None;
        }
        SessionAction::Disconnected { reason_json } => {
            summary.disconnect_reason_json = Some(reason_json);
        }
        SessionAction::HealthUpdated(_)
        | SessionAction::Respawned(_)
        | SessionAction::EntityVelocityReceived(_)
        | SessionAction::WindowItemsUpdated(_)
        | SessionAction::TransactionConfirmed(_)
        | SessionAction::UsabilityPacket(_)
        | SessionAction::ReplyKeepAlive { .. }
        | SessionAction::TeleportCorrectionRequired(_) => {}
    }

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

fn render_live_frame_line(
    frame_index: u64,
    output: &ShellAdvanceOutput,
    world_metrics: &WorldMetrics,
    render: &RenderTelemetrySnapshot,
    combat: &CombatSnapshot,
    usability: &UsabilitySnapshot,
) -> String {
    format!(
        "frame={} ticks_run={} total_ticks={} raw_frame_ms={:.3} paced_frame_ms={:.3} jitter_ms={:.3} render_x={:.6} render_y={:.6} render_z={:.6} loaded_chunks={} loaded_sections={} loaded_non_air={} visible_chunks={} dirty_chunks={} scheduled_jobs={} uploaded_vertices={} uploaded_indices={} health={:.3} hurt_ticks={} chat_lines={} tab_entries={} sidebar_visible={} window_open={}",
        frame_index,
        output.ticks_run,
        output.total_ticks,
        output.performance.pacing.raw_frame_time.as_secs_f32() * 1000.0,
        output.performance.pacing.paced_frame_time.as_secs_f32() * 1000.0,
        output.performance.pacing.jitter_ms,
        output.render.camera.position.x,
        output.render.camera.position.y,
        output.render.camera.position.z,
        world_metrics.loaded_chunks,
        world_metrics.loaded_sections,
        world_metrics.loaded_non_air_blocks,
        render.visible_chunks,
        render.dirty_chunks,
        render.scheduled_jobs,
        render.uploaded_vertices,
        render.uploaded_indices,
        combat.health,
        combat.hurt_ticks,
        usability.chat_lines.len(),
        usability.tab_list.len(),
        usability.sidebar.is_some(),
        usability.window.is_some(),
    )
}

fn print_summary(
    summary: &LiveRunSummary,
    world_metrics: &WorldMetrics,
    render_summary: &RenderTelemetrySummary,
    packet_summary: &PacketTelemetrySnapshot,
    last_output: Option<&ShellAdvanceOutput>,
    last_hud: Option<&PvPHud>,
    last_usability_snapshot: Option<&UsabilitySnapshot>,
    combat: &CombatSnapshot,
) -> Result<(), String> {
    println!("reached_play={}", summary.reached_play);
    println!("joined_game={}", summary.joined_game);
    println!("compression_enabled={}", summary.compression_enabled);
    println!("encryption_enabled={}", summary.encryption_enabled);
    println!("ignored_packets={}", summary.ignored_packets);
    println!("ended_by_eof={}", summary.ended_by_eof);
    println!("timed_out={}", summary.timed_out);
    println!("frames={}", summary.frames);
    println!("total_ticks_run={}", summary.total_ticks_run);
    println!("world_loaded_chunks={}", world_metrics.loaded_chunks);
    println!("world_loaded_sections={}", world_metrics.loaded_sections);
    println!(
        "world_loaded_non_air_blocks={}",
        world_metrics.loaded_non_air_blocks
    );
    println!(
        "render_average_visible_chunks={:.3}",
        render_summary.average_visible_chunks
    );
    println!(
        "render_average_uploaded_vertices={:.3}",
        render_summary.average_uploaded_vertices
    );
    println!(
        "render_average_uploaded_indices={:.3}",
        render_summary.average_uploaded_indices
    );
    println!(
        "render_average_scheduled_jobs={:.3}",
        render_summary.average_scheduled_jobs
    );
    println!(
        "render_peak_dirty_chunks={}",
        render_summary.peak_dirty_chunks
    );
    println!(
        "render_peak_culled_chunks={}",
        render_summary.peak_culled_chunks
    );
    println!("packet_total_packets={}", packet_summary.total_packets);
    println!("packet_total_bytes={}", packet_summary.total_bytes);
    println!(
        "packet_average_len={:.3}",
        packet_summary.average_packet_len
    );
    println!(
        "packet_average_inter_arrival_ms={:.3}",
        packet_summary.average_inter_arrival_ms
    );
    println!(
        "packet_max_inter_arrival_ms={:.3}",
        packet_summary.max_inter_arrival_ms
    );
    println!("health={:.3}", combat.health);
    println!("food_level={}", combat.food_level);
    println!("saturation={:.3}", combat.saturation);
    println!("hurt_ticks={}", combat.hurt_ticks);

    if let Some(reason) = &summary.disconnect_reason_json {
        println!("disconnect_reason_json={reason}");
    }
    if let Some(output) = last_output {
        println!("camera_yaw={}", output.render.camera.yaw);
        println!("camera_pitch={}", output.render.camera.pitch);
        println!("render_position_x={}", output.render.camera.position.x);
        println!("render_position_y={}", output.render.camera.position.y);
        println!("render_position_z={}", output.render.camera.position.z);
        println!(
            "selected_hotbar_slot={}",
            output.simulation.player.selected_hotbar_slot
        );
        for line in output.render.hud.render_lines() {
            println!("{line}");
        }
    }
    if let Some(snapshot) = last_usability_snapshot {
        println!("chat_lines={}", snapshot.chat_lines.len());
        println!("tab_entries={}", snapshot.tab_list.len());
        println!("sidebar_visible={}", snapshot.sidebar.is_some());
        println!("window_open={}", snapshot.window.is_some());
        println!("recent_sounds={}", snapshot.recent_sounds.len());
    }
    if let Some(hud) = last_hud {
        for line in hud.render_lines() {
            println!("{line}");
        }
    }
    io::stdout()
        .flush()
        .map_err(|error| format!("failed to flush summary: {error}"))
}

fn parse_server_address(value: &str) -> Result<(String, u16), String> {
    if let Some((host, port)) = value.rsplit_once(':') {
        if let Ok(port) = port.parse() {
            return Ok((host.to_owned(), port));
        }
    }
    Ok((value.to_owned(), 25565))
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn world_config_for_dimension(dimension: i32) -> WorldConfig {
    if dimension == 0 {
        WorldConfig::overworld()
    } else {
        WorldConfig::no_sky()
    }
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

fn write_lines(path: &str, lines: &[String]) -> Result<(), String> {
    let file = File::create(path).map_err(|error| format!("failed to create {path}: {error}"))?;
    let mut writer = BufWriter::new(file);
    for line in lines {
        writeln!(writer, "{line}").map_err(|error| format!("failed to write {path}: {error}"))?;
    }
    writer
        .flush()
        .map_err(|error| format!("failed to flush {path}: {error}"))
}
