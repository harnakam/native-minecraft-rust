use crate::shell::{ClientShell, ClientShellConfig};
use rmc_game::input::{InputFrame, PhysicalInput};
use rmc_net::codec::play::ChunkDataPacket;
use rmc_net::framing::CompressionDisposition;
use rmc_net::protocol::{PacketDirection, ProtocolState};
use rmc_net::trace::{build_trace_event_lossy_at, PacketTelemetryHistory};
use rmc_render::{ChunkMeshPipeline, FrustumConfig, MeshBuildConfig, RenderTelemetryHistory};
use rmc_world::{WorldConfig, WorldSnapshot};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct PerfCliOptions {
    pub frames: u32,
    pub frame_time_ms: u64,
    pub chunk_radius: i32,
    pub mesh_jobs_per_frame: usize,
    pub redirty_interval: u32,
    pub forward: bool,
    pub sprint: bool,
    pub trace_path: Option<String>,
}

impl PerfCliOptions {
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            frames: 120,
            frame_time_ms: 16,
            chunk_radius: 3,
            mesh_jobs_per_frame: 4,
            redirty_interval: 30,
            forward: false,
            sprint: false,
            trace_path: None,
        };

        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "perf" => {}
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
                "--chunk-radius" => {
                    options.chunk_radius = next_value(&mut args, "--chunk-radius")?
                        .parse()
                        .map_err(|_| "invalid --chunk-radius value".to_owned())?;
                }
                "--mesh-jobs-per-frame" => {
                    options.mesh_jobs_per_frame = next_value(&mut args, "--mesh-jobs-per-frame")?
                        .parse()
                        .map_err(|_| "invalid --mesh-jobs-per-frame value".to_owned())?;
                }
                "--redirty-interval" => {
                    options.redirty_interval = next_value(&mut args, "--redirty-interval")?
                        .parse()
                        .map_err(|_| "invalid --redirty-interval value".to_owned())?;
                }
                "--forward" => options.forward = true,
                "--sprint" => options.sprint = true,
                "--trace" => options.trace_path = Some(next_value(&mut args, "--trace")?),
                other => return Err(format!("unknown argument: {other}\n\n{}", Self::usage())),
            }
        }

        if options.frames == 0 {
            return Err("--frames must be at least 1".to_owned());
        }

        if options.chunk_radius < 0 {
            return Err("--chunk-radius must be >= 0".to_owned());
        }

        Ok(options)
    }

    pub fn usage() -> String {
        [
            "usage:",
            "  rmc-client perf [options]",
            "",
            "options:",
            "  --frames N",
            "  --frame-ms N",
            "  --chunk-radius N",
            "  --mesh-jobs-per-frame N",
            "  --redirty-interval N",
            "  --forward",
            "  --sprint",
            "  --trace PATH",
        ]
        .join("\n")
    }
}

pub fn run_perf_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = PerfCliOptions::parse(raw_args)?;
    let mut shell = ClientShell::new(ClientShellConfig::vanilla());
    shell.set_mouse_captured(true);
    let world = build_flat_world(options.chunk_radius)?;
    let mut pipeline = ChunkMeshPipeline::with_config(MeshBuildConfig {
        max_jobs_per_frame: options.mesh_jobs_per_frame.max(1),
        ..MeshBuildConfig::default()
    });
    pipeline.mark_all_dirty(&world);

    let frame_input = InputFrame {
        pressed_inputs: pressed_inputs(&options),
        ..InputFrame::default()
    };

    let mut render_history = RenderTelemetryHistory::default();
    let mut packet_history = PacketTelemetryHistory::default();
    let mut trace_lines = Vec::new();
    let mut peak_packet_count = 0usize;
    let mut peak_pacing_jitter_ms = 0.0f32;
    let mut total_ticks_run = 0usize;
    let mut synthetic_clock = SystemTime::UNIX_EPOCH;

    for frame_index in 0..options.frames {
        if options.redirty_interval != 0
            && frame_index != 0
            && frame_index % options.redirty_interval == 0
        {
            pipeline.mark_all_dirty(&world);
        }

        let output = shell.advance(Duration::from_millis(options.frame_time_ms), &frame_input);
        let cycle = pipeline.rebuild_dirty_budgeted(&world);
        let snapshot =
            pipeline.snapshot_for_camera(&world, &output.render, FrustumConfig::debug_default());
        let render_telemetry = pipeline.telemetry_snapshot(&world, &snapshot, &cycle);
        render_history.record(render_telemetry);

        peak_packet_count = peak_packet_count.max(output.performance.packet_count);
        peak_pacing_jitter_ms = peak_pacing_jitter_ms.max(output.performance.pacing.jitter_ms);
        total_ticks_run += output.ticks_run;
        let frame_span = output.performance.pacing.paced_frame_time;
        let packet_spacing = if output.packets.is_empty() {
            Duration::ZERO
        } else {
            Duration::from_secs_f64(frame_span.as_secs_f64() / output.packets.len() as f64)
        };
        let mut packet_clock = synthetic_clock;

        trace_lines.push(format!(
            "frame={} raw_ms={:.3} paced_ms={:.3} jitter_ms={:.3} ticks_run={} visible_chunks={} dirty_chunks={} scheduled_jobs={} rebuilt_chunks={} uploaded_vertices={} uploaded_indices={} packet_count={}",
            frame_index,
            output.performance.pacing.raw_frame_time.as_secs_f32() * 1000.0,
            output.performance.pacing.paced_frame_time.as_secs_f32() * 1000.0,
            output.performance.pacing.jitter_ms,
            output.ticks_run,
            render_telemetry.visible_chunks,
            render_telemetry.dirty_chunks,
            render_telemetry.scheduled_jobs,
            render_telemetry.rebuilt_chunks,
            render_telemetry.uploaded_vertices,
            render_telemetry.uploaded_indices,
            output.performance.packet_count
        ));

        for packet in &output.packets {
            packet_clock = packet_clock + packet_spacing;
            let encoded = packet
                .encode_packet()
                .map_err(|error| format!("failed to encode perf packet: {error:?}"))?;
            let packet_bytes = encoded.packet_bytes();
            let event = build_trace_event_lossy_at(
                packet_clock,
                ProtocolState::Play,
                PacketDirection::Serverbound,
                CompressionDisposition::Disabled,
                &packet_bytes,
            )
            .map_err(|error| format!("failed to trace perf packet: {error:?}"))?;
            packet_history.record(&event);
            trace_lines.push(event.summary_line());
        }

        synthetic_clock = synthetic_clock + frame_span;
    }

    let render_summary = render_history.snapshot();
    let packet_summary = packet_history.snapshot();

    println!("frames={}", options.frames);
    println!("total_ticks_run={total_ticks_run}");
    println!(
        "average_visible_chunks={:.3}",
        render_summary.average_visible_chunks
    );
    println!(
        "average_uploaded_vertices={:.3}",
        render_summary.average_uploaded_vertices
    );
    println!(
        "average_uploaded_indices={:.3}",
        render_summary.average_uploaded_indices
    );
    println!(
        "average_scheduled_jobs={:.3}",
        render_summary.average_scheduled_jobs
    );
    println!("peak_dirty_chunks={}", render_summary.peak_dirty_chunks);
    println!("peak_culled_chunks={}", render_summary.peak_culled_chunks);
    println!("peak_packet_count={peak_packet_count}");
    println!("peak_pacing_jitter_ms={peak_pacing_jitter_ms:.3}");
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

    if let Some(trace_path) = &options.trace_path {
        write_lines(trace_path, &trace_lines)?;
    }

    Ok(())
}

fn pressed_inputs(options: &PerfCliOptions) -> Vec<PhysicalInput> {
    let mut inputs = Vec::new();

    if options.forward {
        inputs.push(PhysicalInput::KeyW);
    }

    if options.sprint {
        inputs.push(PhysicalInput::LeftControl);
    }

    inputs
}

fn build_flat_world(chunk_radius: i32) -> Result<WorldSnapshot, String> {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());

    for chunk_x in -chunk_radius..=chunk_radius {
        for chunk_z in -chunk_radius..=chunk_radius {
            world
                .apply_chunk_data(&flat_chunk_packet(chunk_x, chunk_z))
                .map_err(|error| {
                    format!("failed to apply synthetic chunk {chunk_x},{chunk_z}: {error:?}")
                })?;
        }
    }

    Ok(world)
}

fn flat_chunk_packet(chunk_x: i32, chunk_z: i32) -> ChunkDataPacket {
    let mut states = vec![0u16; 16 * 16 * 16];

    for local_z in 0..16usize {
        for local_x in 0..16usize {
            let index = (local_z << 4) | local_x;
            states[index] = 1;
        }
    }

    let mut data = Vec::with_capacity((16 * 16 * 16 * 2) + (16 * 16 * 16 / 2) * 2 + (16 * 16));

    for state in states {
        data.extend_from_slice(&state.to_le_bytes());
    }

    data.extend_from_slice(&vec![0u8; 16 * 16 * 16 / 2]);
    data.extend_from_slice(&vec![15u8; 16 * 16 * 16 / 2]);
    data.extend_from_slice(&[1u8; 16 * 16]);

    ChunkDataPacket {
        chunk_x,
        chunk_z,
        full_chunk: true,
        primary_bitmask: 0x0001,
        data,
    }
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
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
