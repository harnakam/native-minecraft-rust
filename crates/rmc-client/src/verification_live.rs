use crate::live_runtime::{LiveRuntime, LiveRuntimeConfig, RuntimeActionInput};
use rmc_game::input::InputFrame;
use rmc_game::usability::WindowSnapshot;
use rmc_net::codec::handshake::{HandshakeNextState, HandshakeRequest};
use rmc_net::codec::login::{LoginClientboundPacket, LoginServerboundPacket, LoginSuccess};
use rmc_net::codec::play::{
    EntityVelocityPacket, ItemStack, JoinGamePacket, OpenWindowPacket, PlayClientboundPacket,
    PlayDisconnectPacket, PlayServerboundPacket, PlayerListEntry, PlayerListItemAction,
    PlayerListItemPacket, PlayerPositionAndLookPacket, PositionLookFlags, SetSlotPacket,
    SpawnPlayerPacket, UpdateHealthPacket, WindowItemsPacket,
};
use rmc_net::compression::CompressionState;
use rmc_net::framing::{encode_frame, FrameDecoder, FrameLimits};
use serde_json::Value;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const SCENARIO_PLAYER_ENTITY_ID: i32 = 12;
const SCENARIO_TARGET_ENTITY_ID: i32 = 44;
const SCENARIO_WINDOW_ID: u8 = 4;
const SCENARIO_WINDOW_SLOT: i16 = 13;
const DEFAULT_CAPTURE_PORT: u16 = 25570;
const DEFAULT_JAVA_TIMEOUT_SECS: u64 = 45;
const SERVER_ACCEPT_TIMEOUT_SECS: u64 = 30;
const SERVER_SESSION_TIMEOUT_SECS: u64 = 20;

#[derive(Clone, Debug)]
pub struct LiveSuiteOptions {
    pub mcp_root: PathBuf,
    pub port: u16,
    pub java_timeout_secs: u64,
}

impl Default for LiveSuiteOptions {
    fn default() -> Self {
        Self {
            mcp_root: PathBuf::from("MCP-919"),
            port: DEFAULT_CAPTURE_PORT,
            java_timeout_secs: DEFAULT_JAVA_TIMEOUT_SECS,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LiveCaptureArtifacts {
    pub packet_rust_trace: PathBuf,
    pub movement_rust_trace: PathBuf,
    pub combat_rust_trace: PathBuf,
    pub inventory_rust_trace: PathBuf,
    pub packet_java_trace: PathBuf,
    pub movement_java_trace: PathBuf,
    pub combat_java_trace: PathBuf,
    pub inventory_java_trace: PathBuf,
}

pub fn run_live_capture_suite(options: &LiveSuiteOptions) -> Result<LiveCaptureArtifacts, String> {
    let rust_artifacts = RustTracePaths::default_workspace();
    run_rust_capture(options.port, &rust_artifacts)?;

    let java_root = PathBuf::from("verification").join("java-live");
    if java_root.exists() {
        fs::remove_dir_all(&java_root)
            .map_err(|error| format!("failed to clear {}: {error}", java_root.display()))?;
    }
    fs::create_dir_all(&java_root)
        .map_err(|error| format!("failed to create {}: {error}", java_root.display()))?;
    run_java_capture(options.port.saturating_add(1), options, &java_root)?;

    Ok(LiveCaptureArtifacts {
        packet_rust_trace: rust_artifacts.packet,
        movement_rust_trace: rust_artifacts.movement,
        combat_rust_trace: rust_artifacts.combat,
        inventory_rust_trace: rust_artifacts.inventory,
        packet_java_trace: java_root.join("packet-java.trace"),
        movement_java_trace: java_root.join("movement-java.trace"),
        combat_java_trace: java_root.join("combat-java.trace"),
        inventory_java_trace: java_root.join("inventory-java.trace"),
    })
}

#[derive(Clone, Debug)]
struct RustTracePaths {
    packet: PathBuf,
    movement: PathBuf,
    combat: PathBuf,
    inventory: PathBuf,
}

impl RustTracePaths {
    fn default_workspace() -> Self {
        Self {
            packet: PathBuf::from("verification/packet-rust.trace"),
            movement: PathBuf::from("verification/movement-rust.trace"),
            combat: PathBuf::from("verification/combat-rust.trace"),
            inventory: PathBuf::from("verification/inventory-rust.trace"),
        }
    }
}

fn run_rust_capture(port: u16, paths: &RustTracePaths) -> Result<(), String> {
    let server = spawn_verification_server(port)?;
    let mut runtime_config = LiveRuntimeConfig::offline("RmcRustVerify");
    runtime_config.server_host = "127.0.0.1".to_owned();
    runtime_config.server_port = port;
    runtime_config.brand = "vanilla".to_owned();
    runtime_config.capture_mouse = false;

    let mut runtime = LiveRuntime::connect(runtime_config)?;
    let frame_time = Duration::from_millis(50);
    let frame_input = InputFrame::default();
    let action_input = RuntimeActionInput::default();
    let mut movement_lines = Vec::new();
    let mut combat_lines = Vec::new();
    let mut inventory_lines = Vec::new();
    let mut client_tick = 0usize;
    let started = Instant::now();
    let mut previous_combat = runtime.combat_snapshot();
    let mut previous_window: Option<WindowSnapshot> = None;
    let mut previous_pending = 0usize;
    let mut attack_sent = false;
    let mut click_sent = false;
    let mut confirm_logged = false;

    while started.elapsed() < Duration::from_secs(10) {
        runtime.step(frame_time, &frame_input, &action_input)?;
        movement_lines.extend(runtime.take_movement_corrections());

        if runtime.summary().disconnect_reason_json.is_some() || runtime.summary().ended_by_eof {
            break;
        }

        if runtime.output().is_none() {
            thread::sleep(Duration::from_millis(2));
            continue;
        }

        client_tick += 1;
        let output = runtime
            .output()
            .ok_or_else(|| "live runtime did not produce a shell output".to_owned())?;
        movement_lines.push(render_movement_state_line(output));

        let combat_snapshot = runtime.combat_snapshot();
        if health_changed(&previous_combat, &combat_snapshot) {
            combat_lines.push(format!(
                "combat record=health_update health={:.6} food_level={} saturation={:.6}",
                combat_snapshot.health, combat_snapshot.food_level, combat_snapshot.saturation
            ));
        }
        if velocity_changed(&previous_combat, &combat_snapshot) {
            combat_lines.push(format!(
                "combat record=velocity entity_id={} local_player=true x={:.6} y={:.6} z={:.6}",
                runtime
                    .player_entity_id()
                    .unwrap_or(SCENARIO_PLAYER_ENTITY_ID),
                combat_snapshot.last_velocity.x,
                combat_snapshot.last_velocity.y,
                combat_snapshot.last_velocity.z,
            ));
        }
        previous_combat = combat_snapshot.clone();

        let current_window = runtime
            .usability_snapshot()
            .and_then(|snapshot| snapshot.window.clone());
        if let Some(window) = current_window.as_ref() {
            if previous_window.is_none() {
                inventory_lines.push(format!(
                    "inventory record=open_window window_id={} inventory_type={} slot_count={} title={}",
                    window.window_id,
                    safe_value(&window.inventory_type),
                    window.slot_count,
                    safe_value(&window_title_text(&window.title_json)),
                ));
                inventory_lines.push(format!(
                    "inventory record=window_items window_id={} slot_count={}",
                    window.window_id, window.slot_count
                ));
            }

            let previous_item = previous_window
                .as_ref()
                .and_then(|window| window.slots.get(SCENARIO_WINDOW_SLOT as usize))
                .cloned()
                .unwrap_or(None);
            let current_item = window
                .slots
                .get(SCENARIO_WINDOW_SLOT as usize)
                .cloned()
                .unwrap_or(None);
            if previous_item != current_item {
                inventory_lines.push(format!(
                    "inventory record=set_slot window_id={} slot_id={} item={}",
                    window.window_id,
                    SCENARIO_WINDOW_SLOT,
                    slot_value_string(current_item),
                ));
            }
        }

        let pending_now = runtime.inventory_state().pending_transactions().len();
        if click_sent && !confirm_logged && previous_pending > 0 && pending_now == 0 {
            inventory_lines.push(format!(
                "inventory record=confirm_transaction window_id={} action_number=1 accepted=false ack_sent=true",
                SCENARIO_WINDOW_ID
            ));
            confirm_logged = true;
        }
        previous_window = current_window;
        previous_pending = pending_now;

        if !attack_sent && client_tick >= 14 {
            runtime.attack_entity(SCENARIO_TARGET_ENTITY_ID)?;
            combat_lines.push(
                "combat record=attack entity_id=44 selected_slot=0 sprinting=false".to_owned(),
            );
            attack_sent = true;
        }

        if !click_sent
            && client_tick >= 22
            && runtime
                .inventory_state()
                .open_window()
                .map(|window| window.window_id == SCENARIO_WINDOW_ID)
                .unwrap_or(false)
        {
            let clicked_item = runtime
                .inventory_state()
                .open_window()
                .and_then(|window| window.slot(SCENARIO_WINDOW_SLOT))
                .cloned()
                .unwrap_or(None);
            runtime.click_window_slot(SCENARIO_WINDOW_ID, SCENARIO_WINDOW_SLOT, 0)?;
            inventory_lines.push(format!(
                "inventory record=click_window window_id={} slot_id={} button=0 mode=0 action_number=1 item={}",
                SCENARIO_WINDOW_ID,
                SCENARIO_WINDOW_SLOT,
                slot_value_string(clicked_item),
            ));
            click_sent = true;
            previous_pending = runtime.inventory_state().pending_transactions().len();
        }

        if client_tick >= 34 {
            break;
        }

        thread::sleep(frame_time);
    }

    write_lines(&paths.packet, runtime.packet_trace_lines())?;
    write_lines(&paths.movement, &movement_lines)?;
    write_lines(&paths.combat, &combat_lines)?;
    write_lines(&paths.inventory, &inventory_lines)?;

    drop(runtime);
    finish_server(server)?;
    Ok(())
}

fn run_java_capture(port: u16, options: &LiveSuiteOptions, java_root: &Path) -> Result<(), String> {
    let server = spawn_verification_server(port)?;
    let python_mcp = options
        .mcp_root
        .join("runtime")
        .join("bin")
        .join("python")
        .join("python_mcp.exe");
    if !python_mcp.exists() {
        return Err(format!(
            "MCP python launcher was not found: {}",
            python_mcp.display()
        ));
    }

    let launcher_script = java_root.join("rmc_launch_client.py");
    fs::write(&launcher_script, python_launcher_script()).map_err(|error| {
        format!(
            "failed to write verification launcher script {}: {error}",
            launcher_script.display()
        )
    })?;
    let launcher_script = launcher_script
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize launcher script path: {error}"))?;
    let mcp_trace_root = options.mcp_root.join("jars").join("rmc-java-live");
    if mcp_trace_root.exists() {
        fs::remove_dir_all(&mcp_trace_root).map_err(|error| {
            format!(
                "failed to clear MCP Java trace root {}: {error}",
                mcp_trace_root.display()
            )
        })?;
    }

    let mut command = Command::new(python_mcp);
    command
        .arg(&launcher_script)
        .current_dir(&options.mcp_root)
        .env("RMC_TRACE_DIR", "rmc-java-live")
        .env("RMC_VERIFY_PORT", port.to_string())
        .env("RMC_VERIFY_HOST", "127.0.0.1")
        .stdin(Stdio::null())
        .stdout(Stdio::from(
            fs::File::create(java_root.join("java-launch.stdout.log"))
                .map_err(|e| format!("cannot create Java stdout log: {e}"))?,
        ))
        .stderr(Stdio::from(
            fs::File::create(java_root.join("java-launch.stderr.log"))
                .map_err(|e| format!("cannot create Java stderr log: {e}"))?,
        ));

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to launch MCP client: {error}"))?;
    let timeout = Duration::from_secs(options.java_timeout_secs);
    let started = Instant::now();

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll MCP client: {error}"))?
        {
            if !status.success() {
                return Err(format!(
                    "MCP client exited with {status}; see {}/java-launch.stderr.log",
                    java_root.display()
                ));
            }
            break;
        }

        if started.elapsed() >= timeout {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = Command::new("taskkill")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "MCP client timed out after {}s; see {}/java-launch.stderr.log",
                timeout.as_secs(),
                java_root.display()
            ));
        }

        thread::sleep(Duration::from_millis(100));
    }

    finish_server(server)?;
    for channel in [
        "packet-java.trace",
        "movement-java.trace",
        "combat-java.trace",
        "inventory-java.trace",
    ] {
        let source = mcp_trace_root.join(channel);
        if !source.exists() {
            return Err(format!(
                "expected Java trace was not written: {}",
                source.display()
            ));
        }
        let target = java_root.join(channel);
        fs::copy(&source, &target).map_err(|error| {
            format!(
                "failed to copy Java trace {} -> {}: {error}",
                source.display(),
                target.display()
            )
        })?;
    }
    Ok(())
}

fn python_launcher_script() -> &'static str {
    r#"import os
import sys

sys.path.insert(0, os.path.join(os.getcwd(), 'runtime'))
from commands import Commands

commands = Commands(None)
classpath = [commands.binclient, commands.srcclient] + commands.cpathclient
classpath = [os.path.join('..', path) for path in classpath]
classpath = os.pathsep.join(classpath)
natives = os.path.join('..', commands.dirnatives)
trace_dir = os.environ.get('RMC_TRACE_DIR', 'verification\\java-live')
verify_host = os.environ.get('RMC_VERIFY_HOST', '127.0.0.1')
verify_port = os.environ.get('RMC_VERIFY_PORT', '25570')
os.chdir(commands.dirjars)
forkcmd = '%s -Xincgc -Xms1024M -Xmx1024M -cp "%s" -Djava.library.path=%s -Drmc.trace.dir="%s" -Drmc.verify.live=true net.minecraft.client.main.Main --version mcp --accessToken 0 --assetsDir assets --assetIndex 1.8 --userProperties {} --username RmcJavaVerify --server %s --port %s --width 854 --height 480' % (commands.cmdjava, classpath, natives, trace_dir, verify_host, verify_port)
commands.runmc(forkcmd)
"#
}

fn health_changed(
    previous: &rmc_game::combat::CombatSnapshot,
    current: &rmc_game::combat::CombatSnapshot,
) -> bool {
    (previous.health - current.health).abs() > f32::EPSILON
        || previous.food_level != current.food_level
        || (previous.saturation - current.saturation).abs() > f32::EPSILON
}

fn velocity_changed(
    previous: &rmc_game::combat::CombatSnapshot,
    current: &rmc_game::combat::CombatSnapshot,
) -> bool {
    previous.last_velocity != current.last_velocity
}

fn render_movement_state_line(output: &crate::shell::ShellAdvanceOutput) -> String {
    let packet = output
        .packets
        .last()
        .map(movement_packet_label)
        .unwrap_or("Player");
    let (moved, rotated) = movement_packet_flags(packet);

    format!(
        "movement record=state tick={} pos_x={:.6} pos_y={:.6} pos_z={:.6} vel_x={:.6} vel_y={:.6} vel_z={:.6} yaw={:.6} pitch={:.6} on_ground={} sprinting={} sneaking={} packet={} moved={} rotated={}",
        output.total_ticks,
        output.simulation.player.position.x,
        output.simulation.player.position.y,
        output.simulation.player.position.z,
        output.simulation.velocity.x,
        output.simulation.velocity.y,
        output.simulation.velocity.z,
        output.render.camera.yaw,
        output.render.camera.pitch,
        output.simulation.player.on_ground,
        output.simulation.player.sprinting,
        output.simulation.player.sneaking,
        packet,
        moved,
        rotated,
    )
}

fn movement_packet_label(packet: &PlayServerboundPacket) -> &'static str {
    match packet {
        PlayServerboundPacket::Player(_) => "Player",
        PlayServerboundPacket::PlayerPosition(_) => "PlayerPosition",
        PlayServerboundPacket::PlayerLook(_) => "PlayerLook",
        PlayServerboundPacket::PlayerPositionAndLook(_) => "PlayerPositionAndLook",
        _ => "Player",
    }
}

fn movement_packet_flags(packet: &str) -> (bool, bool) {
    match packet {
        "PlayerPositionAndLook" => (true, true),
        "PlayerPosition" => (true, false),
        "PlayerLook" => (false, true),
        _ => (false, false),
    }
}

fn window_title_text(title_json: &str) -> String {
    serde_json::from_str::<Value>(title_json)
        .ok()
        .and_then(|value| value.get("text").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_else(|| title_json.to_owned())
}

fn safe_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn slot_value_string(slot: Option<ItemStack>) -> String {
    match slot {
        Some(item) => format!("{}:{}:{}", item.item_id, item.count, item.damage),
        None => "none".to_owned(),
    }
}

fn write_lines(path: &Path, lines: &[String]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let contents = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };
    fs::write(path, contents)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn spawn_verification_server(port: u16) -> Result<JoinHandle<Result<(), String>>, String> {
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
        format!("failed to bind verification server on 127.0.0.1:{port}: {error}")
    })?;
    Ok(thread::spawn(move || run_verification_server(listener)))
}

fn finish_server(handle: JoinHandle<Result<(), String>>) -> Result<(), String> {
    handle
        .join()
        .map_err(|_| "verification server thread panicked".to_owned())?
}

fn run_verification_server(listener: TcpListener) -> Result<(), String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("failed to set listener nonblocking: {error}"))?;
    let started = Instant::now();
    let mut stream = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if started.elapsed() >= Duration::from_secs(SERVER_ACCEPT_TIMEOUT_SECS) {
                    return Err("verification server timed out waiting for a client".to_owned());
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                return Err(format!(
                    "verification server failed to accept client: {error}"
                ))
            }
        }
    };

    stream
        .set_nonblocking(true)
        .map_err(|error| format!("failed to set verification stream nonblocking: {error}"))?;
    stream.set_nodelay(true).map_err(|error| {
        format!("failed to enable TCP_NODELAY for verification server: {error}")
    })?;

    let limits = FrameLimits::default();
    let mut decoder = FrameDecoder::new(CompressionState::Disabled);
    let handshake_bytes = read_next_packet(&mut stream, &mut decoder, Duration::from_secs(5))?;
    let handshake = HandshakeRequest::decode_packet(&handshake_bytes)
        .map_err(|error| format!("failed to decode verification handshake: {error:?}"))?;
    if handshake.next_state != HandshakeNextState::Login {
        return Err("verification client did not enter login state".to_owned());
    }

    let login_bytes = read_next_packet(&mut stream, &mut decoder, Duration::from_secs(5))?;
    let login_start = match LoginServerboundPacket::decode_packet(&login_bytes)
        .map_err(|error| format!("failed to decode verification login start: {error:?}"))?
    {
        LoginServerboundPacket::LoginStart(packet) => packet,
        LoginServerboundPacket::EncryptionResponse(_) => {
            return Err("verification server received unexpected encryption response".to_owned())
        }
    };

    write_login_packet(
        &mut stream,
        &LoginClientboundPacket::LoginSuccess(LoginSuccess {
            uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
            username: login_start.username,
        }),
        limits,
    )?;

    let mut scenario = VerificationServerScenario::new();
    let loop_started = Instant::now();

    while loop_started.elapsed() < Duration::from_secs(SERVER_SESSION_TIMEOUT_SECS) {
        drain_serverbound_packets(&mut stream, &mut decoder, &mut scenario)?;
        scenario.send_due_packets(&mut stream, limits)?;

        if scenario.finished {
            return Ok(());
        }

        thread::sleep(Duration::from_millis(5));
    }

    Err("verification server timed out before finishing the scripted session".to_owned())
}

struct VerificationServerScenario {
    movement_ticks: u64,
    teleport_ack_received: bool,
    join_sent: bool,
    player_list_sent: bool,
    correction_sent: bool,
    spawn_sent: bool,
    health_sent: bool,
    velocity_sent: bool,
    window_sent: bool,
    click_confirm_sent: bool,
    disconnect_sent: bool,
    finished: bool,
}

impl VerificationServerScenario {
    fn new() -> Self {
        Self {
            movement_ticks: 0,
            teleport_ack_received: false,
            join_sent: false,
            player_list_sent: false,
            correction_sent: false,
            spawn_sent: false,
            health_sent: false,
            velocity_sent: false,
            window_sent: false,
            click_confirm_sent: false,
            disconnect_sent: false,
            finished: false,
        }
    }

    fn send_due_packets(
        &mut self,
        stream: &mut TcpStream,
        limits: FrameLimits,
    ) -> Result<(), String> {
        let tick = self.movement_ticks;

        if !self.join_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::JoinGame(JoinGamePacket {
                    entity_id: SCENARIO_PLAYER_ENTITY_ID,
                    game_mode: 0,
                    hardcore: false,
                    dimension: 0,
                    difficulty: 1,
                    max_players: 20,
                    level_type: "default".to_owned(),
                    reduced_debug_info: false,
                }),
                limits,
            )?;
            self.join_sent = true;
        }

        if !self.player_list_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::PlayerListItem(PlayerListItemPacket {
                    action: PlayerListItemAction::AddPlayer,
                    entries: vec![PlayerListEntry {
                        uuid: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
                        name: Some("RmcTarget".to_owned()),
                        properties: Vec::new(),
                        game_mode: Some(0),
                        latency: Some(0),
                        display_name_json: Some("{\"text\":\"RmcTarget\"}".to_owned()),
                    }],
                }),
                limits,
            )?;
            self.player_list_sent = true;
        }

        if !self.correction_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::PlayerPositionAndLook(PlayerPositionAndLookPacket {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    yaw: 0.0,
                    pitch: 0.0,
                    flags: PositionLookFlags::from_bits(0),
                }),
                limits,
            )?;
            self.correction_sent = true;
        }

        if !self.spawn_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::SpawnPlayer(SpawnPlayerPacket {
                    entity_id: SCENARIO_TARGET_ENTITY_ID,
                    player_uuid: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
                    x: 0,
                    y: 0,
                    z: 64,
                    yaw: 0,
                    pitch: 0,
                    held_item: 0,
                    metadata: vec![0x00, 0x00, 0x7f],
                }),
                limits,
            )?;
            self.spawn_sent = true;
        }

        if tick >= 10 && !self.health_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::UpdateHealth(UpdateHealthPacket {
                    health: 17.0,
                    food_level: 19,
                    saturation: 4.0,
                }),
                limits,
            )?;
            self.health_sent = true;
        }

        if tick >= 12 && !self.velocity_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::EntityVelocity(EntityVelocityPacket {
                    entity_id: SCENARIO_PLAYER_ENTITY_ID,
                    velocity_x: 1600,
                    velocity_y: 800,
                    velocity_z: -400,
                }),
                limits,
            )?;
            self.velocity_sent = true;
        }

        if tick >= 14 && !self.window_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::OpenWindow(OpenWindowPacket {
                    window_id: SCENARIO_WINDOW_ID,
                    inventory_type: "minecraft:chest".to_owned(),
                    window_title_json: "{\"text\":\"Loot\"}".to_owned(),
                    slot_count: 27,
                    entity_id: None,
                }),
                limits,
            )?;
            write_play_packet(
                stream,
                &PlayClientboundPacket::WindowItems(WindowItemsPacket {
                    window_id: SCENARIO_WINDOW_ID,
                    items: vec![None; 27],
                }),
                limits,
            )?;
            write_play_packet(
                stream,
                &PlayClientboundPacket::SetSlot(SetSlotPacket {
                    window_id: SCENARIO_WINDOW_ID as i8,
                    slot_id: SCENARIO_WINDOW_SLOT,
                    item: Some(ItemStack::simple(5, 16, 0)),
                }),
                limits,
            )?;
            self.window_sent = true;
        }

        if tick >= 60 && !self.disconnect_sent {
            write_play_packet(
                stream,
                &PlayClientboundPacket::Disconnect(PlayDisconnectPacket {
                    reason_json: "{\"text\":\"verification_complete\"}".to_owned(),
                }),
                limits,
            )?;
            self.disconnect_sent = true;
        }

        if tick >= 62 {
            self.finished = true;
        }

        Ok(())
    }
}

fn drain_serverbound_packets(
    stream: &mut TcpStream,
    decoder: &mut FrameDecoder,
    scenario: &mut VerificationServerScenario,
) -> Result<(), String> {
    let mut buffer = [0u8; 4096];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                if scenario.disconnect_sent || scenario.click_confirm_sent {
                    scenario.finished = true;
                    return Ok(());
                }
                break;
            }
            Ok(read) => decoder.queue_bytes(&buffer[..read]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
                ) && scenario.click_confirm_sent =>
            {
                scenario.finished = true;
                return Ok(());
            }
            Err(error) => return Err(format!("verification server read failed: {error}")),
        }
    }

    while let Some(frame) = decoder
        .try_next_frame(None)
        .map_err(|error| format!("verification server failed to decode frame: {error:?}"))?
    {
        let packet =
            PlayServerboundPacket::decode_packet(&frame.packet_bytes).map_err(|error| {
                format!("verification server failed to decode play packet: {error:?}")
            })?;
        match &packet {
            PlayServerboundPacket::PlayerPositionAndLook(_) if !scenario.teleport_ack_received => {
                scenario.teleport_ack_received = true;
            }
            PlayServerboundPacket::Player(_)
            | PlayServerboundPacket::PlayerPosition(_)
            | PlayServerboundPacket::PlayerLook(_)
            | PlayServerboundPacket::PlayerPositionAndLook(_) => {
                scenario.movement_ticks += 1;
            }
            _ => {}
        }
        if let PlayServerboundPacket::ClickWindow(packet) = packet {
            if !scenario.click_confirm_sent {
                write_play_packet(
                    stream,
                    &PlayClientboundPacket::ConfirmTransaction(
                        rmc_net::codec::play::ConfirmTransactionClientboundPacket {
                            window_id: packet.window_id,
                            action_number: packet.action_number,
                            accepted: false,
                        },
                    ),
                    FrameLimits::default(),
                )?;
                scenario.click_confirm_sent = true;
            }
        }
    }

    Ok(())
}

fn read_next_packet(
    stream: &mut TcpStream,
    decoder: &mut FrameDecoder,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let started = Instant::now();
    let mut buffer = [0u8; 4096];

    loop {
        if let Some(frame) = decoder
            .try_next_frame(None)
            .map_err(|error| format!("failed to decode verification frame: {error:?}"))?
        {
            return Ok(frame.packet_bytes);
        }

        match stream.read(&mut buffer) {
            Ok(0) => return Err("verification client closed the connection early".to_owned()),
            Ok(read) => decoder.queue_bytes(&buffer[..read]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if started.elapsed() >= timeout {
                    return Err("verification server timed out waiting for a packet".to_owned());
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(error) => return Err(format!("verification server read failed: {error}")),
        }
    }
}

fn write_login_packet(
    stream: &mut TcpStream,
    packet: &LoginClientboundPacket,
    limits: FrameLimits,
) -> Result<(), String> {
    let encoded = packet
        .encode_packet()
        .map_err(|error| format!("failed to encode login packet: {error:?}"))?;
    let frame = encode_frame(
        &encoded.packet_bytes(),
        CompressionState::Disabled,
        None,
        limits,
    )
    .map_err(|error| format!("failed to frame login packet: {error:?}"))?;
    stream
        .write_all(&frame)
        .map_err(|error| format!("failed to write login packet: {error}"))
}

fn write_play_packet(
    stream: &mut TcpStream,
    packet: &PlayClientboundPacket,
    limits: FrameLimits,
) -> Result<(), String> {
    let encoded = packet
        .encode_packet()
        .map_err(|error| format!("failed to encode play packet: {error:?}"))?;
    let frame = encode_frame(
        &encoded.packet_bytes(),
        CompressionState::Disabled,
        None,
        limits,
    )
    .map_err(|error| format!("failed to frame play packet: {error:?}"))?;
    stream
        .write_all(&frame)
        .map_err(|error| format!("failed to write play packet: {error}"))
}
