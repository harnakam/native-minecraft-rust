mod live_cli;
mod live_runtime;
mod perf_cli;
mod play_assets;
mod play_cli;
mod shell;
mod usability_cli;
mod verification_cli;
mod verification_live;

use crate::live_cli::{run_live_cli, LiveCliOptions};
use crate::perf_cli::{run_perf_cli, PerfCliOptions};
use crate::play_cli::{run_play_cli, PlayCliOptions};
use crate::shell::{ClientShell, ClientShellConfig, ShellAdvanceOutput};
use crate::usability_cli::{run_usability_cli, UsabilityCliOptions};
use crate::verification_cli::{run_verify_cli, VerifyCliOptions};
use rmc_game::combat::{CombatConfig, CombatState};
use rmc_game::input::{InputFrame, PhysicalInput};
use rmc_game::inventory::InventoryState;
use rmc_game::player::Vec3;
use rmc_game::simulation::{AuthoritativePlayerState, KnockbackImpulse, SimulationEvent};
use rmc_net::auth::OnlineAccount;
use rmc_net::codec::play::{
    ConfirmTransactionClientboundPacket, EntityVelocityPacket, ItemStack, PlayServerboundPacket,
    UpdateHealthPacket,
};
use rmc_net::headless::{run_headless, AuthenticationMode, HeadlessRunConfig, HeadlessRunSummary};
use rmc_net::trace::{LineWriterTraceSink, NoopTraceSink};
use std::env;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::time::Duration;

fn main() {
    if let Err(error) = real_main() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn real_main() -> Result<(), String> {
    let raw_args: Vec<String> = env::args().skip(1).collect();

    if raw_args.is_empty()
        || raw_args
            .iter()
            .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
    {
        println!("{}", usage());
        return Ok(());
    }

    match raw_args[0].as_str() {
        "headless" => run_headless_cli(raw_args),
        "live" => run_live_cli(raw_args),
        "play" => run_play_cli(raw_args),
        "shell" => run_shell_cli(raw_args),
        "combat" => run_combat_cli(raw_args),
        "perf" => run_perf_cli(raw_args),
        "usability" => run_usability_cli(raw_args),
        "verify" => run_verify_cli(raw_args),
        other => Err(format!("unknown subcommand: {other}\n\n{}", usage())),
    }
}

fn run_headless_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = HeadlessCliOptions::parse(raw_args)?;
    let mut config = HeadlessRunConfig::vanilla_headless(
        options.server_host,
        options.server_port,
        options.username.clone(),
    );
    config.read_timeout = Duration::from_millis(options.read_timeout_ms);
    config.max_runtime = Some(Duration::from_secs(options.duration_secs));
    config.stop_after_join = options.stop_after_join;
    config.driver.client_brand = options.brand;
    config.driver.client_settings.locale = options.locale;
    config.driver.client_settings.view_distance = options.view_distance;

    let authentication = match (options.profile_id, options.access_token) {
        (Some(profile_id), Some(access_token)) => AuthenticationMode::Online(OnlineAccount {
            username: options.username,
            profile_id,
            access_token,
        }),
        (None, None) => AuthenticationMode::Offline,
        _ => {
            return Err(
                "both --uuid and --access-token must be provided for online-mode login".to_owned(),
            )
        }
    };

    let summary = if let Some(trace_path) = options.trace_path {
        let file = File::create(&trace_path)
            .map_err(|error| format!("failed to create trace file {trace_path}: {error}"))?;
        let writer = BufWriter::new(file);
        let mut sink = LineWriterTraceSink::new(writer);
        let summary = run_headless(&config, &authentication, &mut sink)
            .map_err(|error| format!("headless run failed: {error:?}"))?;
        sink.finish()
            .map_err(|error| format!("failed to flush trace file {trace_path}: {error}"))?;
        summary
    } else {
        let mut sink = NoopTraceSink;
        run_headless(&config, &authentication, &mut sink)
            .map_err(|error| format!("headless run failed: {error:?}"))?
    };

    print_summary(&summary).map_err(|error| format!("failed to write summary: {error}"))?;
    Ok(())
}

fn run_shell_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = ShellCliOptions::parse(raw_args)?;
    let mut shell = ClientShell::new(ClientShellConfig::vanilla());
    shell.set_mouse_captured(options.capture_mouse);
    let mut trace_lines = Vec::new();

    let frame_input = InputFrame {
        pressed_inputs: options.pressed_inputs(),
        mouse_delta_x: options.mouse_delta_x,
        mouse_delta_y: options.mouse_delta_y,
        hotbar_scroll: options.hotbar_scroll,
        ..InputFrame::default()
    };

    let mut last_output = None;

    for frame_index in 0..options.frames {
        let frame_events = options.events_for_frame(frame_index);
        trace_lines.extend(render_frame_events(frame_index, &frame_events));
        let output = shell.advance_with_events(
            Duration::from_millis(options.frame_time_ms),
            &frame_input,
            &frame_events,
        );
        trace_lines.extend(render_shell_trace(frame_index, &output));
        last_output = Some(output);
    }

    let output = last_output.ok_or_else(|| "shell run produced no frames".to_owned())?;

    if let Some(trace_path) = &options.trace_path {
        write_lines(trace_path, &trace_lines)?;
    }

    println!("ticks_run={}", output.ticks_run);
    println!("total_ticks={}", output.total_ticks);
    println!("camera_yaw={}", output.render.camera.yaw);
    println!("camera_pitch={}", output.render.camera.pitch);
    println!("mouse_captured={}", output.render.camera.mouse_captured);
    println!("render_position_x={}", output.render.camera.position.x);
    println!("render_position_y={}", output.render.camera.position.y);
    println!("render_position_z={}", output.render.camera.position.z);
    println!(
        "network_position_x={}",
        output.render.simulation.network_position.x
    );
    println!(
        "network_position_y={}",
        output.render.simulation.network_position.y
    );
    println!(
        "network_position_z={}",
        output.render.simulation.network_position.z
    );
    println!(
        "simulated_position_x={}",
        output.render.simulation.simulated_position.x
    );
    println!(
        "simulated_position_y={}",
        output.render.simulation.simulated_position.y
    );
    println!(
        "simulated_position_z={}",
        output.render.simulation.simulated_position.z
    );
    println!("velocity_x={}", output.render.simulation.velocity.x);
    println!("velocity_y={}", output.render.simulation.velocity.y);
    println!("velocity_z={}", output.render.simulation.velocity.z);
    println!(
        "raw_frame_ms={:.3}",
        output.performance.pacing.raw_frame_time.as_secs_f32() * 1000.0
    );
    println!(
        "paced_frame_ms={:.3}",
        output.performance.pacing.paced_frame_time.as_secs_f32() * 1000.0
    );
    println!("frame_jitter_ms={:.3}", output.performance.pacing.jitter_ms);
    println!(
        "spike_detected={}",
        output.performance.pacing.spike_detected
    );
    println!("packet_count={}", output.performance.packet_count);
    println!("on_ground={}", output.render.simulation.on_ground);
    println!("sprinting={}", output.render.simulation.sprinting);
    println!("sneaking={}", output.render.simulation.sneaking);
    println!(
        "sprint_reset_ticks={}",
        output.render.simulation.sprint_reset_ticks
    );
    println!(
        "selected_hotbar_slot={}",
        output.render.hud.hotbar.selected_slot
    );

    for line in output.render.hud.render_lines() {
        println!("{line}");
    }

    for line in render_shell_packet_lines(&output) {
        println!("{line}");
    }

    Ok(())
}

fn run_combat_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = CombatCliOptions::parse(raw_args)?;
    let mut combat = CombatState::new(CombatConfig::vanilla());
    let mut inventory = InventoryState::new();
    let mut trace_lines = Vec::new();
    let mut packets = Vec::new();

    if let Some(slot) = options.select_slot {
        if let Some(packet) = inventory.sync_selected_hotbar_slot(slot) {
            packets.push(packet);
        }
    }

    packets.extend(combat.sync_action_state(
        options.player_entity_id,
        options.sprinting,
        options.sneaking,
    ));

    if let Some(entity_id) = options.attack_entity {
        packets.extend(combat.attack_entity(entity_id));
    }

    if let Some(entity_id) = options.interact_entity {
        packets.extend(combat.interact_entity(entity_id));
    }

    if let Some(entity_id) = options.interact_at_entity {
        packets.extend(combat.interact_at_entity(entity_id, options.interact_at_hit()?));
    }

    if let Some(item) = options.use_item() {
        packets.extend(combat.start_using_item(Some(item)));
    }

    if options.release_use_item {
        packets.extend(combat.release_using_item());
    }

    if let Some(slot_id) = options.click_slot {
        let packet = inventory.queue_click(
            options.click_window_id,
            slot_id,
            options.click_button,
            options.click_mode,
            options.click_item(),
        );
        let action_number = match &packet {
            PlayServerboundPacket::ClickWindow(packet) => packet.action_number,
            _ => 0,
        };
        packets.push(packet);

        if options.reject_transaction {
            let update =
                inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
                    window_id: options.click_window_id,
                    action_number,
                    accepted: false,
                });
            trace_lines.push(format!(
                "transaction window_id={} action_number={} accepted=false ack_packets={}",
                options.click_window_id,
                action_number,
                update.outbound_packets.len()
            ));
            packets.extend(update.outbound_packets);
        }
    }

    if let Some(health) = options.health {
        let update = combat.apply_health_update(&UpdateHealthPacket {
            health,
            food_level: options.food_level,
            saturation: options.saturation,
        });
        trace_lines.push(format!(
            "health_update health={:.3} food_level={} saturation={:.3} hurt_feedback={}",
            health, options.food_level, options.saturation, update.hurt_feedback
        ));
    }

    if let Some(packet) = options.velocity_packet() {
        let update = combat.apply_entity_velocity(&packet, Some(options.player_entity_id));
        trace_lines.push(format!(
            "velocity entity_id={} x={:.6} y={:.6} z={:.6} simulation_events={}",
            packet.entity_id,
            packet.motion_x(),
            packet.motion_y(),
            packet.motion_z(),
            update.simulation_events.len()
        ));

        for event in update.simulation_events {
            match event {
                SimulationEvent::Knockback(impulse) => trace_lines.push(format!(
                    "simulation_event=Knockback x={:.6} y={:.6} z={:.6} resets_sprint={}",
                    impulse.velocity.x,
                    impulse.velocity.y,
                    impulse.velocity.z,
                    impulse.resets_sprint
                )),
                SimulationEvent::AddVelocity(motion) => trace_lines.push(format!(
                    "simulation_event=AddVelocity x={} y={} z={}",
                    motion.x, motion.y, motion.z
                )),
                SimulationEvent::AuthoritativeState(_) | SimulationEvent::Teleport { .. } => {}
            }
        }
    }

    let snapshot = combat.snapshot();
    println!("selected_hotbar_slot={}", inventory.selected_hotbar_slot());
    println!("server_sprint_state={}", snapshot.server_sprint_state);
    println!("server_sneak_state={}", snapshot.server_sneak_state);
    println!("using_item={}", snapshot.using_item.is_some());
    println!(
        "using_item_ticks={}",
        snapshot
            .using_item
            .as_ref()
            .map(|state| state.use_ticks)
            .unwrap_or(0)
    );
    println!("health={}", snapshot.health);
    println!("food_level={}", snapshot.food_level);
    println!("saturation={}", snapshot.saturation);
    println!("hurt_ticks={}", snapshot.hurt_ticks);
    println!("last_velocity_x={}", snapshot.last_velocity.x);
    println!("last_velocity_y={}", snapshot.last_velocity.y);
    println!("last_velocity_z={}", snapshot.last_velocity.z);
    println!(
        "pending_transactions={}",
        inventory.pending_transactions().len()
    );

    for (index, packet) in packets.iter().enumerate() {
        trace_lines.push(render_shell_packet_line(index as u64 + 1, packet));
    }

    if let Some(trace_path) = &options.trace_path {
        write_lines(trace_path, &trace_lines)?;
    }

    for line in trace_lines {
        println!("{line}");
    }

    Ok(())
}

fn print_summary(summary: &HeadlessRunSummary) -> io::Result<()> {
    println!("reached_play={}", summary.reached_play);
    println!("joined_game={}", summary.joined_game);
    println!("compression_enabled={}", summary.compression_enabled);
    println!("encryption_enabled={}", summary.encryption_enabled);
    println!("ignored_packets={}", summary.ignored_packets);
    println!("ended_by_eof={}", summary.ended_by_eof);
    println!("timed_out={}", summary.timed_out);

    if let Some(reason) = &summary.disconnect_reason_json {
        println!("disconnect_reason={reason}");
    }

    Ok(())
}

#[derive(Debug)]
struct HeadlessCliOptions {
    server_host: String,
    server_port: u16,
    username: String,
    profile_id: Option<String>,
    access_token: Option<String>,
    trace_path: Option<String>,
    duration_secs: u64,
    read_timeout_ms: u64,
    stop_after_join: bool,
    brand: String,
    locale: String,
    view_distance: i8,
}

impl HeadlessCliOptions {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            server_host: String::new(),
            server_port: 25565,
            username: String::new(),
            profile_id: None,
            access_token: None,
            trace_path: None,
            duration_secs: 30,
            read_timeout_ms: 250,
            stop_after_join: false,
            brand: "RustMinecraft".to_owned(),
            locale: "en_US".to_owned(),
            view_distance: 8,
        };

        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "headless" => {}
                "--help" | "-h" => return Err(Self::usage()),
                "--server" => {
                    let value = next_value(&mut args, "--server")?;
                    let (host, port) = parse_server_address(&value)?;
                    options.server_host = host;
                    options.server_port = port;
                }
                "--username" => {
                    options.username = next_value(&mut args, "--username")?;
                }
                "--uuid" => {
                    options.profile_id = Some(next_value(&mut args, "--uuid")?);
                }
                "--access-token" => {
                    options.access_token = Some(next_value(&mut args, "--access-token")?);
                }
                "--trace" => {
                    options.trace_path = Some(next_value(&mut args, "--trace")?);
                }
                "--duration-secs" => {
                    options.duration_secs = next_value(&mut args, "--duration-secs")?
                        .parse()
                        .map_err(|_| "invalid --duration-secs value".to_owned())?;
                }
                "--read-timeout-ms" => {
                    options.read_timeout_ms =
                        next_value(&mut args, "--read-timeout-ms")?
                            .parse()
                            .map_err(|_| "invalid --read-timeout-ms value".to_owned())?;
                }
                "--stop-after-join" => {
                    options.stop_after_join = true;
                }
                "--brand" => {
                    options.brand = next_value(&mut args, "--brand")?;
                }
                "--locale" => {
                    options.locale = next_value(&mut args, "--locale")?;
                }
                "--view-distance" => {
                    options.view_distance = next_value(&mut args, "--view-distance")?
                        .parse()
                        .map_err(|_| "invalid --view-distance value".to_owned())?;
                }
                other => {
                    return Err(format!("unknown argument: {other}\n\n{}", Self::usage()));
                }
            }
        }

        if options.server_host.is_empty() || options.username.is_empty() {
            return Err(Self::usage());
        }

        Ok(options)
    }

    fn usage() -> String {
        [
            "usage:",
            "  rmc-client headless --server hypixel.net:25565 --username NAME [--uuid UUID --access-token TOKEN]",
            "",
            "options:",
            "  --trace PATH",
            "  --duration-secs N",
            "  --read-timeout-ms N",
            "  --stop-after-join",
            "  --brand NAME",
            "  --locale LOCALE",
            "  --view-distance N",
        ]
        .join("\n")
    }
}

#[derive(Debug)]
struct ShellCliOptions {
    frames: u32,
    frame_time_ms: u64,
    mouse_delta_x: f32,
    mouse_delta_y: f32,
    hotbar_scroll: i8,
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
    jump: bool,
    sneak: bool,
    sprint: bool,
    capture_mouse: bool,
    trace_path: Option<String>,
    server_frame: u32,
    server_x: Option<f64>,
    server_y: Option<f64>,
    server_z: Option<f64>,
    server_vx: f64,
    server_vy: f64,
    server_vz: f64,
    server_airborne: bool,
    knockback_frame: u32,
    knockback_x: Option<f64>,
    knockback_y: Option<f64>,
    knockback_z: Option<f64>,
}

impl ShellCliOptions {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            frames: 60,
            frame_time_ms: 16,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            hotbar_scroll: 0,
            forward: false,
            back: false,
            left: false,
            right: false,
            jump: false,
            sneak: false,
            sprint: false,
            capture_mouse: true,
            trace_path: None,
            server_frame: 0,
            server_x: None,
            server_y: None,
            server_z: None,
            server_vx: 0.0,
            server_vy: 0.0,
            server_vz: 0.0,
            server_airborne: false,
            knockback_frame: 0,
            knockback_x: None,
            knockback_y: None,
            knockback_z: None,
        };

        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "shell" => {}
                "--help" | "-h" => return Err(Self::usage()),
                "--frames" => {
                    options.frames = next_value(&mut args, "--frames")?
                        .parse()
                        .map_err(|_| "invalid --frames value".to_owned())?;
                }
                "--frame-ms" => {
                    options.frame_time_ms = next_value(&mut args, "--frame-ms")?
                        .parse()
                        .map_err(|_| "invalid --frame-ms value".to_owned())?;
                }
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
                "--hotbar-scroll" => {
                    options.hotbar_scroll = next_value(&mut args, "--hotbar-scroll")?
                        .parse()
                        .map_err(|_| "invalid --hotbar-scroll value".to_owned())?;
                }
                "--trace" => {
                    options.trace_path = Some(next_value(&mut args, "--trace")?);
                }
                "--server-frame" => {
                    options.server_frame = next_value(&mut args, "--server-frame")?
                        .parse()
                        .map_err(|_| "invalid --server-frame value".to_owned())?;
                }
                "--server-x" => {
                    options.server_x = Some(
                        next_value(&mut args, "--server-x")?
                            .parse()
                            .map_err(|_| "invalid --server-x value".to_owned())?,
                    );
                }
                "--server-y" => {
                    options.server_y = Some(
                        next_value(&mut args, "--server-y")?
                            .parse()
                            .map_err(|_| "invalid --server-y value".to_owned())?,
                    );
                }
                "--server-z" => {
                    options.server_z = Some(
                        next_value(&mut args, "--server-z")?
                            .parse()
                            .map_err(|_| "invalid --server-z value".to_owned())?,
                    );
                }
                "--server-vx" => {
                    options.server_vx = next_value(&mut args, "--server-vx")?
                        .parse()
                        .map_err(|_| "invalid --server-vx value".to_owned())?;
                }
                "--server-vy" => {
                    options.server_vy = next_value(&mut args, "--server-vy")?
                        .parse()
                        .map_err(|_| "invalid --server-vy value".to_owned())?;
                }
                "--server-vz" => {
                    options.server_vz = next_value(&mut args, "--server-vz")?
                        .parse()
                        .map_err(|_| "invalid --server-vz value".to_owned())?;
                }
                "--server-airborne" => options.server_airborne = true,
                "--knockback-frame" => {
                    options.knockback_frame =
                        next_value(&mut args, "--knockback-frame")?
                            .parse()
                            .map_err(|_| "invalid --knockback-frame value".to_owned())?;
                }
                "--knockback-x" => {
                    options.knockback_x = Some(
                        next_value(&mut args, "--knockback-x")?
                            .parse()
                            .map_err(|_| "invalid --knockback-x value".to_owned())?,
                    );
                }
                "--knockback-y" => {
                    options.knockback_y = Some(
                        next_value(&mut args, "--knockback-y")?
                            .parse()
                            .map_err(|_| "invalid --knockback-y value".to_owned())?,
                    );
                }
                "--knockback-z" => {
                    options.knockback_z = Some(
                        next_value(&mut args, "--knockback-z")?
                            .parse()
                            .map_err(|_| "invalid --knockback-z value".to_owned())?,
                    );
                }
                "--forward" => options.forward = true,
                "--back" => options.back = true,
                "--left" => options.left = true,
                "--right" => options.right = true,
                "--jump" => options.jump = true,
                "--sneak" => options.sneak = true,
                "--sprint" => options.sprint = true,
                "--no-capture-mouse" => options.capture_mouse = false,
                other => {
                    return Err(format!("unknown argument: {other}\n\n{}", Self::usage()));
                }
            }
        }

        if options.frames == 0 {
            return Err("--frames must be at least 1".to_owned());
        }

        validate_vec3_args(
            "server",
            options.server_x,
            options.server_y,
            options.server_z,
        )?;
        validate_vec3_args(
            "knockback",
            options.knockback_x,
            options.knockback_y,
            options.knockback_z,
        )?;

        let has_server_event =
            options.server_x.is_some() && options.server_y.is_some() && options.server_z.is_some();
        let has_knockback_event = options.knockback_x.is_some()
            && options.knockback_y.is_some()
            && options.knockback_z.is_some();

        if !has_server_event
            && (options.server_frame != 0
                || options.server_airborne
                || options.server_vx != 0.0
                || options.server_vy != 0.0
                || options.server_vz != 0.0)
        {
            return Err(
                "server correction flags require --server-x, --server-y, and --server-z".to_owned(),
            );
        }

        if !has_knockback_event && options.knockback_frame != 0 {
            return Err(
                "knockback frame requires --knockback-x, --knockback-y, and --knockback-z"
                    .to_owned(),
            );
        }

        if has_server_event && options.server_frame >= options.frames {
            return Err("--server-frame must be smaller than --frames".to_owned());
        }

        if has_knockback_event && options.knockback_frame >= options.frames {
            return Err("--knockback-frame must be smaller than --frames".to_owned());
        }

        Ok(options)
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
        if self.sneak {
            inputs.push(PhysicalInput::LeftShift);
        }
        if self.sprint {
            inputs.push(PhysicalInput::LeftControl);
        }

        inputs
    }

    fn events_for_frame(&self, frame_index: u32) -> Vec<SimulationEvent> {
        let mut events = Vec::new();

        if frame_index == self.server_frame {
            if let Some(state) = self.authoritative_state() {
                events.push(SimulationEvent::AuthoritativeState(state));
            }
        }

        if frame_index == self.knockback_frame {
            if let Some(impulse) = self.knockback() {
                events.push(SimulationEvent::Knockback(impulse));
            }
        }

        events
    }

    fn authoritative_state(&self) -> Option<AuthoritativePlayerState> {
        Some(AuthoritativePlayerState {
            position: optional_vec3(self.server_x, self.server_y, self.server_z)?,
            velocity: Vec3::new(self.server_vx, self.server_vy, self.server_vz),
            on_ground: !self.server_airborne,
        })
    }

    fn knockback(&self) -> Option<KnockbackImpulse> {
        let velocity = optional_vec3(self.knockback_x, self.knockback_y, self.knockback_z)?;
        Some(KnockbackImpulse::new(velocity.x, velocity.y, velocity.z))
    }

    fn usage() -> String {
        [
            "usage:",
            "  rmc-client shell [--frames N] [--frame-ms N] [--mouse-dx DX] [--mouse-dy DY] [--forward] [--left] [--sprint]",
            "",
            "options:",
            "  --back",
            "  --right",
            "  --jump",
            "  --sneak",
            "  --hotbar-scroll N",
            "  --trace PATH",
            "  --server-frame N --server-x X --server-y Y --server-z Z",
            "  --server-vx X --server-vy Y --server-vz Z",
            "  --server-airborne",
            "  --knockback-frame N --knockback-x X --knockback-y Y --knockback-z Z",
            "  --no-capture-mouse",
        ]
        .join("\n")
    }
}

#[derive(Debug)]
struct CombatCliOptions {
    player_entity_id: i32,
    sprinting: bool,
    sneaking: bool,
    select_slot: Option<u8>,
    attack_entity: Option<i32>,
    interact_entity: Option<i32>,
    interact_at_entity: Option<i32>,
    interact_at_x: Option<f32>,
    interact_at_y: Option<f32>,
    interact_at_z: Option<f32>,
    use_item_id: Option<i16>,
    use_item_count: u8,
    use_item_damage: i16,
    release_use_item: bool,
    click_window_id: u8,
    click_slot: Option<i16>,
    click_button: i8,
    click_mode: i8,
    click_item_id: Option<i16>,
    click_item_count: u8,
    click_item_damage: i16,
    reject_transaction: bool,
    health: Option<f32>,
    food_level: i32,
    saturation: f32,
    velocity_entity_id: Option<i32>,
    velocity_x: Option<i16>,
    velocity_y: Option<i16>,
    velocity_z: Option<i16>,
    trace_path: Option<String>,
}

impl CombatCliOptions {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            player_entity_id: 1,
            sprinting: false,
            sneaking: false,
            select_slot: None,
            attack_entity: None,
            interact_entity: None,
            interact_at_entity: None,
            interact_at_x: None,
            interact_at_y: None,
            interact_at_z: None,
            use_item_id: None,
            use_item_count: 1,
            use_item_damage: 0,
            release_use_item: false,
            click_window_id: 0,
            click_slot: None,
            click_button: 0,
            click_mode: 0,
            click_item_id: None,
            click_item_count: 1,
            click_item_damage: 0,
            reject_transaction: false,
            health: None,
            food_level: 20,
            saturation: 5.0,
            velocity_entity_id: None,
            velocity_x: None,
            velocity_y: None,
            velocity_z: None,
            trace_path: None,
        };

        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "combat" => {}
                "--help" | "-h" => return Err(Self::usage()),
                "--player-entity-id" => {
                    options.player_entity_id = next_value(&mut args, "--player-entity-id")?
                        .parse()
                        .map_err(|_| "invalid --player-entity-id value".to_owned())?;
                }
                "--sprinting" => options.sprinting = true,
                "--sneaking" => options.sneaking = true,
                "--select-slot" => {
                    options.select_slot = Some(
                        next_value(&mut args, "--select-slot")?
                            .parse()
                            .map_err(|_| "invalid --select-slot value".to_owned())?,
                    );
                }
                "--attack-entity" => {
                    options.attack_entity = Some(
                        next_value(&mut args, "--attack-entity")?
                            .parse()
                            .map_err(|_| "invalid --attack-entity value".to_owned())?,
                    );
                }
                "--interact-entity" => {
                    options.interact_entity = Some(
                        next_value(&mut args, "--interact-entity")?
                            .parse()
                            .map_err(|_| "invalid --interact-entity value".to_owned())?,
                    );
                }
                "--interact-at-entity" => {
                    options.interact_at_entity = Some(
                        next_value(&mut args, "--interact-at-entity")?
                            .parse()
                            .map_err(|_| "invalid --interact-at-entity value".to_owned())?,
                    );
                }
                "--interact-at-x" => {
                    options.interact_at_x = Some(
                        next_value(&mut args, "--interact-at-x")?
                            .parse()
                            .map_err(|_| "invalid --interact-at-x value".to_owned())?,
                    );
                }
                "--interact-at-y" => {
                    options.interact_at_y = Some(
                        next_value(&mut args, "--interact-at-y")?
                            .parse()
                            .map_err(|_| "invalid --interact-at-y value".to_owned())?,
                    );
                }
                "--interact-at-z" => {
                    options.interact_at_z = Some(
                        next_value(&mut args, "--interact-at-z")?
                            .parse()
                            .map_err(|_| "invalid --interact-at-z value".to_owned())?,
                    );
                }
                "--use-item-id" => {
                    options.use_item_id = Some(
                        next_value(&mut args, "--use-item-id")?
                            .parse()
                            .map_err(|_| "invalid --use-item-id value".to_owned())?,
                    );
                }
                "--use-item-count" => {
                    options.use_item_count = next_value(&mut args, "--use-item-count")?
                        .parse()
                        .map_err(|_| "invalid --use-item-count value".to_owned())?;
                }
                "--use-item-damage" => {
                    options.use_item_damage =
                        next_value(&mut args, "--use-item-damage")?
                            .parse()
                            .map_err(|_| "invalid --use-item-damage value".to_owned())?;
                }
                "--release-use-item" => options.release_use_item = true,
                "--click-window-id" => {
                    options.click_window_id =
                        next_value(&mut args, "--click-window-id")?
                            .parse()
                            .map_err(|_| "invalid --click-window-id value".to_owned())?;
                }
                "--click-slot" => {
                    options.click_slot = Some(
                        next_value(&mut args, "--click-slot")?
                            .parse()
                            .map_err(|_| "invalid --click-slot value".to_owned())?,
                    );
                }
                "--click-button" => {
                    options.click_button = next_value(&mut args, "--click-button")?
                        .parse()
                        .map_err(|_| "invalid --click-button value".to_owned())?;
                }
                "--click-mode" => {
                    options.click_mode = next_value(&mut args, "--click-mode")?
                        .parse()
                        .map_err(|_| "invalid --click-mode value".to_owned())?;
                }
                "--click-item-id" => {
                    options.click_item_id = Some(
                        next_value(&mut args, "--click-item-id")?
                            .parse()
                            .map_err(|_| "invalid --click-item-id value".to_owned())?,
                    );
                }
                "--click-item-count" => {
                    options.click_item_count = next_value(&mut args, "--click-item-count")?
                        .parse()
                        .map_err(|_| "invalid --click-item-count value".to_owned())?;
                }
                "--click-item-damage" => {
                    options.click_item_damage = next_value(&mut args, "--click-item-damage")?
                        .parse()
                        .map_err(|_| "invalid --click-item-damage value".to_owned())?;
                }
                "--reject-transaction" => options.reject_transaction = true,
                "--health" => {
                    options.health = Some(
                        next_value(&mut args, "--health")?
                            .parse()
                            .map_err(|_| "invalid --health value".to_owned())?,
                    );
                }
                "--food-level" => {
                    options.food_level = next_value(&mut args, "--food-level")?
                        .parse()
                        .map_err(|_| "invalid --food-level value".to_owned())?;
                }
                "--saturation" => {
                    options.saturation = next_value(&mut args, "--saturation")?
                        .parse()
                        .map_err(|_| "invalid --saturation value".to_owned())?;
                }
                "--velocity-entity-id" => {
                    options.velocity_entity_id = Some(
                        next_value(&mut args, "--velocity-entity-id")?
                            .parse()
                            .map_err(|_| "invalid --velocity-entity-id value".to_owned())?,
                    );
                }
                "--velocity-x" => {
                    options.velocity_x = Some(
                        next_value(&mut args, "--velocity-x")?
                            .parse()
                            .map_err(|_| "invalid --velocity-x value".to_owned())?,
                    );
                }
                "--velocity-y" => {
                    options.velocity_y = Some(
                        next_value(&mut args, "--velocity-y")?
                            .parse()
                            .map_err(|_| "invalid --velocity-y value".to_owned())?,
                    );
                }
                "--velocity-z" => {
                    options.velocity_z = Some(
                        next_value(&mut args, "--velocity-z")?
                            .parse()
                            .map_err(|_| "invalid --velocity-z value".to_owned())?,
                    );
                }
                "--trace" => {
                    options.trace_path = Some(next_value(&mut args, "--trace")?);
                }
                other => return Err(format!("unknown argument: {other}\n\n{}", Self::usage())),
            }
        }

        if let Some(slot) = options.select_slot {
            if slot > 8 {
                return Err("--select-slot must be in 0..8".to_owned());
            }
        }

        validate_f32_vec3_args(
            "interact-at",
            options.interact_at_x,
            options.interact_at_y,
            options.interact_at_z,
        )?;

        if options.interact_at_entity.is_none()
            && (options.interact_at_x.is_some()
                || options.interact_at_y.is_some()
                || options.interact_at_z.is_some())
        {
            return Err("interact-at hit coordinates require --interact-at-entity".to_owned());
        }

        if options.interact_at_entity.is_some()
            && (options.interact_at_x.is_none()
                || options.interact_at_y.is_none()
                || options.interact_at_z.is_none())
        {
            return Err(
                "--interact-at-entity requires --interact-at-x, --interact-at-y, and --interact-at-z"
                    .to_owned(),
            );
        }

        validate_i16_vec3_args(
            "velocity",
            options.velocity_x,
            options.velocity_y,
            options.velocity_z,
        )?;

        if (options.velocity_x.is_some()
            || options.velocity_y.is_some()
            || options.velocity_z.is_some())
            && options.velocity_entity_id.is_none()
        {
            options.velocity_entity_id = Some(options.player_entity_id);
        }

        if options.reject_transaction && options.click_slot.is_none() {
            return Err("--reject-transaction requires --click-slot".to_owned());
        }

        Ok(options)
    }

    fn interact_at_hit(&self) -> Result<[f32; 3], String> {
        Ok([
            self.interact_at_x
                .ok_or_else(|| "missing --interact-at-x".to_owned())?,
            self.interact_at_y
                .ok_or_else(|| "missing --interact-at-y".to_owned())?,
            self.interact_at_z
                .ok_or_else(|| "missing --interact-at-z".to_owned())?,
        ])
    }

    fn use_item(&self) -> Option<ItemStack> {
        Some(ItemStack::simple(
            self.use_item_id?,
            self.use_item_count,
            self.use_item_damage,
        ))
    }

    fn click_item(&self) -> Option<ItemStack> {
        Some(ItemStack::simple(
            self.click_item_id?,
            self.click_item_count,
            self.click_item_damage,
        ))
    }

    fn velocity_packet(&self) -> Option<EntityVelocityPacket> {
        Some(EntityVelocityPacket {
            entity_id: self.velocity_entity_id?,
            velocity_x: self.velocity_x?,
            velocity_y: self.velocity_y?,
            velocity_z: self.velocity_z?,
        })
    }

    fn usage() -> String {
        [
            "usage:",
            "  rmc-client combat [--player-entity-id N] [--sprinting] [--sneaking]",
            "",
            "options:",
            "  --select-slot N",
            "  --attack-entity N",
            "  --interact-entity N",
            "  --interact-at-entity N --interact-at-x X --interact-at-y Y --interact-at-z Z",
            "  --use-item-id ID [--use-item-count N] [--use-item-damage N]",
            "  --release-use-item",
            "  --click-window-id N --click-slot N [--click-button N] [--click-mode N]",
            "  --click-item-id ID [--click-item-count N] [--click-item-damage N]",
            "  --reject-transaction",
            "  --health VALUE [--food-level N] [--saturation VALUE]",
            "  --velocity-entity-id N --velocity-x N --velocity-y N --velocity-z N",
            "  --trace PATH",
        ]
        .join("\n")
    }
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn parse_server_address(value: &str) -> Result<(String, u16), String> {
    if let Some((host, port)) = value.rsplit_once(':') {
        if let Ok(port) = port.parse() {
            return Ok((host.to_owned(), port));
        }
    }

    Ok((value.to_owned(), 25565))
}

fn validate_vec3_args(
    label: &str,
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
) -> Result<(), String> {
    let present = [x.is_some(), y.is_some(), z.is_some()];

    if present.iter().all(|flag| !flag) || present.iter().all(|flag| *flag) {
        return Ok(());
    }

    Err(format!(
        "{label} event requires all of --{label}-x, --{label}-y, and --{label}-z"
    ))
}

fn validate_f32_vec3_args(
    label: &str,
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
) -> Result<(), String> {
    let present = [x.is_some(), y.is_some(), z.is_some()];

    if present.iter().all(|flag| !flag) || present.iter().all(|flag| *flag) {
        return Ok(());
    }

    Err(format!(
        "{label} event requires all of --{label}-x, --{label}-y, and --{label}-z"
    ))
}

fn validate_i16_vec3_args(
    label: &str,
    x: Option<i16>,
    y: Option<i16>,
    z: Option<i16>,
) -> Result<(), String> {
    let present = [x.is_some(), y.is_some(), z.is_some()];

    if present.iter().all(|flag| !flag) || present.iter().all(|flag| *flag) {
        return Ok(());
    }

    Err(format!(
        "{label} event requires all of --{label}-x, --{label}-y, and --{label}-z"
    ))
}

fn optional_vec3(x: Option<f64>, y: Option<f64>, z: Option<f64>) -> Option<Vec3> {
    Some(Vec3::new(x?, y?, z?))
}

fn packet_name(packet: &PlayServerboundPacket) -> &'static str {
    match packet {
        PlayServerboundPacket::KeepAlive(_) => "KeepAlive",
        PlayServerboundPacket::ChatMessage(_) => "ChatMessage",
        PlayServerboundPacket::UseEntity(_) => "UseEntity",
        PlayServerboundPacket::Player(_) => "Player",
        PlayServerboundPacket::PlayerPosition(_) => "PlayerPosition",
        PlayServerboundPacket::PlayerLook(_) => "PlayerLook",
        PlayServerboundPacket::PlayerPositionAndLook(_) => "PlayerPositionAndLook",
        PlayServerboundPacket::PlayerDigging(_) => "PlayerDigging",
        PlayServerboundPacket::PlayerBlockPlacement(_) => "PlayerBlockPlacement",
        PlayServerboundPacket::HeldItemChange(_) => "HeldItemChange",
        PlayServerboundPacket::Animation(_) => "Animation",
        PlayServerboundPacket::EntityAction(_) => "EntityAction",
        PlayServerboundPacket::CloseWindow(_) => "CloseWindow",
        PlayServerboundPacket::ClickWindow(_) => "ClickWindow",
        PlayServerboundPacket::ConfirmTransaction(_) => "ConfirmTransaction",
        PlayServerboundPacket::PlayerAbilities(_) => "PlayerAbilities",
        PlayServerboundPacket::ClientStatus(_) => "ClientStatus",
        PlayServerboundPacket::ClientSettings(_) => "ClientSettings",
        PlayServerboundPacket::CustomPayload(_) => "CustomPayload",
    }
}

fn render_frame_events(frame_index: u32, events: &[SimulationEvent]) -> Vec<String> {
    events
        .iter()
        .map(|event| match event {
            SimulationEvent::AuthoritativeState(state) => format!(
                "frame={} event=AuthoritativeState x={:.6} y={:.6} z={:.6} vx={:.6} vy={:.6} vz={:.6} on_ground={}",
                frame_index,
                state.position.x,
                state.position.y,
                state.position.z,
                state.velocity.x,
                state.velocity.y,
                state.velocity.z,
                state.on_ground
            ),
            SimulationEvent::Knockback(impulse) => format!(
                "frame={} event=Knockback x={:.6} y={:.6} z={:.6} resets_sprint={}",
                frame_index,
                impulse.velocity.x,
                impulse.velocity.y,
                impulse.velocity.z,
                impulse.resets_sprint
            ),
            SimulationEvent::AddVelocity(motion) => format!("frame={frame_index} event=AddVelocity x={} y={} z={}",motion.x,motion.y,motion.z),
            SimulationEvent::Teleport { position, yaw, pitch, flags } => format!("frame={frame_index} event=Teleport x={} y={} z={} yaw={yaw} pitch={pitch} flags={flags}", position.x, position.y, position.z),
        })
        .collect()
}

fn render_shell_trace(frame_index: u32, output: &ShellAdvanceOutput) -> Vec<String> {
    let mut lines = vec![format!(
        "frame={} ticks_run={} total_ticks={} interpolation_alpha={:.6} raw_frame_ms={:.3} paced_frame_ms={:.3} frame_jitter_ms={:.3} spike_detected={} packet_count={} yaw={:.6} pitch={:.6} render_x={:.6} render_y={:.6} render_z={:.6} network_x={:.6} network_y={:.6} network_z={:.6} simulated_x={:.6} simulated_y={:.6} simulated_z={:.6} velocity_x={:.6} velocity_y={:.6} velocity_z={:.6} on_ground={} sprinting={} sneaking={} sprint_reset_ticks={} mouse_captured={} selected_hotbar_slot={}",
        frame_index,
        output.ticks_run,
        output.total_ticks,
        output.render.interpolation_alpha,
        output.performance.pacing.raw_frame_time.as_secs_f32() * 1000.0,
        output.performance.pacing.paced_frame_time.as_secs_f32() * 1000.0,
        output.performance.pacing.jitter_ms,
        output.performance.pacing.spike_detected,
        output.performance.packet_count,
        output.render.camera.yaw,
        output.render.camera.pitch,
        output.render.camera.position.x,
        output.render.camera.position.y,
        output.render.camera.position.z,
        output.render.simulation.network_position.x,
        output.render.simulation.network_position.y,
        output.render.simulation.network_position.z,
        output.render.simulation.simulated_position.x,
        output.render.simulation.simulated_position.y,
        output.render.simulation.simulated_position.z,
        output.render.simulation.velocity.x,
        output.render.simulation.velocity.y,
        output.render.simulation.velocity.z,
        output.render.simulation.on_ground,
        output.render.simulation.sprinting,
        output.render.simulation.sneaking,
        output.render.simulation.sprint_reset_ticks,
        output.render.camera.mouse_captured,
        output.render.hud.hotbar.selected_slot,
    )];
    lines.extend(render_shell_packet_lines(output));
    lines
}

fn render_shell_packet_lines(output: &ShellAdvanceOutput) -> Vec<String> {
    let first_tick = output.total_ticks.saturating_sub(output.ticks_run as u64);

    output
        .packets
        .iter()
        .enumerate()
        .map(|(index, packet)| render_shell_packet_line(first_tick + index as u64 + 1, packet))
        .collect()
}

fn render_shell_packet_line(tick_index: u64, packet: &PlayServerboundPacket) -> String {
    match packet {
        PlayServerboundPacket::KeepAlive(keep_alive) => {
            format!(
                "tick={tick_index} packet={} id={}",
                packet_name(packet),
                keep_alive.id
            )
        }
        PlayServerboundPacket::ChatMessage(chat) => format!(
            "tick={tick_index} packet={} message={}",
            packet_name(packet),
            chat.message
        ),
        PlayServerboundPacket::UseEntity(use_entity) => {
            let mut line = format!(
                "tick={tick_index} packet={} entity_id={} action={:?}",
                packet_name(packet),
                use_entity.entity_id,
                use_entity.action
            );

            if let Some(target) = use_entity.target {
                line.push_str(&format!(
                    " target_x={:.6} target_y={:.6} target_z={:.6}",
                    target[0], target[1], target[2]
                ));
            }

            line
        }
        PlayServerboundPacket::Player(player) => format!(
            "tick={tick_index} packet={} on_ground={}",
            packet_name(packet),
            player.on_ground
        ),
        PlayServerboundPacket::PlayerPosition(position) => format!(
            "tick={tick_index} packet={} x={:.6} y={:.6} z={:.6} on_ground={}",
            packet_name(packet),
            position.x,
            position.y,
            position.z,
            position.on_ground
        ),
        PlayServerboundPacket::PlayerLook(look) => format!(
            "tick={tick_index} packet={} yaw={:.6} pitch={:.6} on_ground={}",
            packet_name(packet),
            look.yaw,
            look.pitch,
            look.on_ground
        ),
        PlayServerboundPacket::PlayerPositionAndLook(position_and_look) => format!(
            "tick={tick_index} packet={} x={:.6} y={:.6} z={:.6} yaw={:.6} pitch={:.6} on_ground={}",
            packet_name(packet),
            position_and_look.x,
            position_and_look.y,
            position_and_look.z,
            position_and_look.yaw,
            position_and_look.pitch,
            position_and_look.on_ground
        ),
        PlayServerboundPacket::PlayerDigging(digging) => format!(
            "tick={tick_index} packet={} action={:?} x={} y={} z={} face={}",
            packet_name(packet),
            digging.action,
            digging.position.x,
            digging.position.y,
            digging.position.z,
            digging.face
        ),
        PlayServerboundPacket::PlayerBlockPlacement(placement) => format!(
            "tick={tick_index} packet={} x={} y={} z={} face={} held_item={} cursor_x={:.6} cursor_y={:.6} cursor_z={:.6}",
            packet_name(packet),
            placement.position.x,
            placement.position.y,
            placement.position.z,
            placement.face,
            placement
                .held_item
                .as_ref()
                .map(|item| item.item_id.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            placement.cursor_x,
            placement.cursor_y,
            placement.cursor_z
        ),
        PlayServerboundPacket::HeldItemChange(held_item) => format!(
            "tick={tick_index} packet={} slot={}",
            packet_name(packet),
            held_item.slot
        ),
        PlayServerboundPacket::Animation(_) => {
            format!("tick={tick_index} packet={}", packet_name(packet))
        }
        PlayServerboundPacket::EntityAction(action) => format!(
            "tick={tick_index} packet={} entity_id={} action={:?} aux_data={}",
            packet_name(packet),
            action.entity_id,
            action.action,
            action.aux_data
        ),
        PlayServerboundPacket::CloseWindow(close_window) => format!(
            "tick={tick_index} packet={} window_id={}",
            packet_name(packet),
            close_window.window_id
        ),
        PlayServerboundPacket::ClickWindow(click) => format!(
            "tick={tick_index} packet={} window_id={} slot_id={} button={} action_number={} mode={} clicked_item={}",
            packet_name(packet),
            click.window_id,
            click.slot_id,
            click.button,
            click.action_number,
            click.mode,
            click
                .clicked_item
                .as_ref()
                .map(|item| format!("{}:{}:{}", item.item_id, item.count, item.damage))
                .unwrap_or_else(|| "none".to_owned())
        ),
        PlayServerboundPacket::ConfirmTransaction(confirm) => format!(
            "tick={tick_index} packet={} window_id={} action_number={} accepted={}",
            packet_name(packet),
            confirm.window_id,
            confirm.action_number,
            confirm.accepted
        ),
        PlayServerboundPacket::PlayerAbilities(abilities) => format!("tick={tick_index} packet=PlayerAbilities flags={} flying_speed={} walking_speed={}",abilities.flags,abilities.flying_speed,abilities.walking_speed),
        PlayServerboundPacket::ClientStatus(action) => format!("tick={tick_index} packet=ClientStatus action={action}"),
        PlayServerboundPacket::ClientSettings(settings) => format!(
            "tick={tick_index} packet={} locale={} view_distance={} chat_visibility={} chat_colors={} displayed_skin_parts={}",
            packet_name(packet),
            settings.locale,
            settings.view_distance,
            settings.chat_visibility,
            settings.chat_colors,
            settings.displayed_skin_parts
        ),
        PlayServerboundPacket::CustomPayload(payload) => format!(
            "tick={tick_index} packet={} channel={} payload_bytes={}",
            packet_name(packet),
            payload.channel,
            payload.data.len()
        ),
    }
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

fn usage() -> String {
    format!(
        "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}",
        HeadlessCliOptions::usage(),
        LiveCliOptions::usage(),
        PlayCliOptions::usage(),
        ShellCliOptions::usage(),
        CombatCliOptions::usage(),
        PerfCliOptions::usage(),
        UsabilityCliOptions::usage(),
        VerifyCliOptions::usage()
    )
}
