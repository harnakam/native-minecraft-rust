use crate::shell::{ClientShell, ClientShellConfig};
use crate::verification_live::{run_live_capture_suite, LiveSuiteOptions};
use rmc_game::combat::{CombatConfig, CombatState};
use rmc_game::input::{InputFrame, PhysicalInput};
use rmc_game::inventory::InventoryState;
use rmc_game::player::Vec3;
use rmc_game::simulation::{AuthoritativePlayerState, KnockbackImpulse, SimulationEvent};
use rmc_java::{install_mcp_java_trace, JavaTraceInstallOptions};
use rmc_net::codec::play::{
    ConfirmTransactionClientboundPacket, EntityVelocityPacket, ItemStack, OpenWindowPacket,
    PlayServerboundPacket, SetSlotPacket, UpdateHealthPacket,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_STATUS_PATH: &str = "verification/status.json";

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VerificationStatus {
    #[serde(default)]
    pub comparison_version: u32,
    pub hypixel_play_allowed: bool,
    pub summary: String,
    pub required_reports: Vec<RequiredReportStatus>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RequiredReportStatus {
    pub kind: String,
    pub path: String,
    pub present: bool,
    pub passed: bool,
    #[serde(default)]
    pub source: VerificationSource,
    #[serde(default)]
    pub gate_eligible: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationReport {
    #[serde(default)]
    pub comparison_version: u32,
    pub kind: String,
    pub rust_event_count: usize,
    pub java_event_count: usize,
    pub missing_java_trace: bool,
    pub passed: bool,
    pub summary: String,
    pub diffs: Vec<TraceDiff>,
    #[serde(default)]
    pub source: VerificationSource,
    #[serde(default)]
    pub gate_eligible: bool,
    #[serde(default)]
    pub rust_trace_path: Option<String>,
    #[serde(default)]
    pub java_trace_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceDiff {
    pub index: usize,
    pub reason: String,
    pub rust_line: Option<String>,
    pub java_line: Option<String>,
}

#[derive(Clone, Debug)]
struct NormalizedRecord {
    label: String,
    fields: BTreeMap<String, String>,
    original: String,
}

#[derive(Clone, Debug)]
pub enum GateDecision {
    Allowed,
    Blocked(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationSource {
    #[default]
    Unknown,
    Sample,
    CanonicalScenario,
    LiveCapture,
}

impl VerificationSource {
    fn is_gate_eligible(&self) -> bool {
        matches!(self, Self::LiveCapture)
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Sample => "sample",
            Self::CanonicalScenario => "canonical_scenario",
            Self::LiveCapture => "live_capture",
        }
    }
}

#[derive(Debug)]
pub struct VerifyCliOptions {
    command: VerifyCommand,
}

#[derive(Debug)]
enum VerifyCommand {
    TraceDiff(TraceDiffOptions),
    Scenario(ScenarioOptions),
    Gate(GateOptions),
    JavaInstall(JavaInstallOptions),
    LiveSuite(LiveSuiteCliOptions),
}

#[derive(Debug)]
struct TraceDiffOptions {
    kind: String,
    rust_trace: String,
    java_trace: String,
    report_path: Option<String>,
    source: VerificationSource,
}

#[derive(Debug)]
struct ScenarioOptions {
    kind: String,
    rust_trace_path: Option<String>,
    java_trace_path: Option<String>,
    report_path: Option<String>,
}

#[derive(Debug)]
struct GateOptions {
    packet_report: Option<String>,
    movement_report: Option<String>,
    combat_report: Option<String>,
    inventory_report: Option<String>,
    status_path: String,
}

#[derive(Debug)]
struct JavaInstallOptions {
    mcp_root: Option<String>,
}

#[derive(Debug)]
struct LiveSuiteCliOptions {
    mcp_root: Option<String>,
    port: u16,
    java_timeout_secs: u64,
}

impl VerifyCliOptions {
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut args = args.into_iter();
        match args.next().as_deref() {
            Some("verify") => {}
            _ => return Err(Self::usage()),
        }

        let Some(subcommand) = args.next() else {
            return Err(Self::usage());
        };

        let command = match subcommand.as_str() {
            "trace-diff" => VerifyCommand::TraceDiff(parse_trace_diff(args)?),
            "scenario" => VerifyCommand::Scenario(parse_scenario(args)?),
            "gate" => VerifyCommand::Gate(parse_gate(args)?),
            "java-install" => VerifyCommand::JavaInstall(parse_java_install(args)?),
            "live-suite" => VerifyCommand::LiveSuite(parse_live_suite(args)?),
            "--help" | "-h" => return Err(Self::usage()),
            other => {
                return Err(format!(
                    "unknown verify subcommand: {other}\n\n{}",
                    Self::usage()
                ))
            }
        };

        Ok(Self { command })
    }

    pub fn usage() -> String {
        [
            "usage:",
            "  rmc-client verify trace-diff --kind packet --rust TRACE --java TRACE [--report PATH] [--source sample|live_capture]",
            "  rmc-client verify scenario --kind movement|combat|inventory [--rust-out PATH] [--java TRACE] [--report PATH]",
            "  rmc-client verify gate [--packet-report PATH] [--movement-report PATH] [--combat-report PATH] [--inventory-report PATH] [--status PATH]",
            "  rmc-client verify java-install [--mcp-root PATH]",
            "  rmc-client verify live-suite [--mcp-root PATH] [--port N] [--java-timeout-secs N]",
            "",
            "notes:",
            "  trace-diff compares normalized Java-vs-Rust traces.",
            "  scenario emits the Rust-side canonical verification trace for one replayable scenario.",
            "  gate writes verification/status.json and only accepts live_capture reports for Hypixel.",
            "  java-install patches local MCP-919 sources with the Java trace hooks used by trace-diff.",
            "  live-suite captures Rust and patched-MCP traces from the same local scripted session, diffs them, and updates the gate.",
        ]
        .join("\n")
    }
}

fn parse_trace_diff<I>(args: I) -> Result<TraceDiffOptions, String>
where
    I: Iterator<Item = String>,
{
    let mut options = TraceDiffOptions {
        kind: "packet".to_owned(),
        rust_trace: String::new(),
        java_trace: String::new(),
        report_path: None,
        source: VerificationSource::Sample,
    };
    let mut args = args;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--kind" => options.kind = next_value(&mut args, "--kind")?,
            "--rust" => options.rust_trace = next_value(&mut args, "--rust")?,
            "--java" => options.java_trace = next_value(&mut args, "--java")?,
            "--report" => options.report_path = Some(next_value(&mut args, "--report")?),
            "--source" => options.source = parse_source(next_value(&mut args, "--source")?)?,
            "--help" | "-h" => return Err(VerifyCliOptions::usage()),
            other => {
                return Err(format!(
                    "unknown argument: {other}\n\n{}",
                    VerifyCliOptions::usage()
                ))
            }
        }
    }

    if options.rust_trace.is_empty() || options.java_trace.is_empty() {
        return Err("trace-diff requires both --rust and --java".to_owned());
    }

    Ok(options)
}

fn parse_scenario<I>(args: I) -> Result<ScenarioOptions, String>
where
    I: Iterator<Item = String>,
{
    let mut options = ScenarioOptions {
        kind: String::new(),
        rust_trace_path: None,
        java_trace_path: None,
        report_path: None,
    };
    let mut args = args;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--kind" => options.kind = next_value(&mut args, "--kind")?,
            "--rust-out" => options.rust_trace_path = Some(next_value(&mut args, "--rust-out")?),
            "--java" => options.java_trace_path = Some(next_value(&mut args, "--java")?),
            "--report" => options.report_path = Some(next_value(&mut args, "--report")?),
            "--help" | "-h" => return Err(VerifyCliOptions::usage()),
            other => {
                return Err(format!(
                    "unknown argument: {other}\n\n{}",
                    VerifyCliOptions::usage()
                ))
            }
        }
    }

    if options.kind.is_empty() {
        return Err("scenario requires --kind".to_owned());
    }

    Ok(options)
}

fn parse_gate<I>(args: I) -> Result<GateOptions, String>
where
    I: Iterator<Item = String>,
{
    let mut options = GateOptions {
        packet_report: None,
        movement_report: None,
        combat_report: None,
        inventory_report: None,
        status_path: DEFAULT_STATUS_PATH.to_owned(),
    };
    let mut args = args;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--packet-report" => {
                options.packet_report = Some(next_value(&mut args, "--packet-report")?)
            }
            "--movement-report" => {
                options.movement_report = Some(next_value(&mut args, "--movement-report")?)
            }
            "--combat-report" => {
                options.combat_report = Some(next_value(&mut args, "--combat-report")?)
            }
            "--inventory-report" => {
                options.inventory_report = Some(next_value(&mut args, "--inventory-report")?)
            }
            "--status" => options.status_path = next_value(&mut args, "--status")?,
            "--help" | "-h" => return Err(VerifyCliOptions::usage()),
            other => {
                return Err(format!(
                    "unknown argument: {other}\n\n{}",
                    VerifyCliOptions::usage()
                ))
            }
        }
    }

    Ok(options)
}

fn parse_java_install<I>(args: I) -> Result<JavaInstallOptions, String>
where
    I: Iterator<Item = String>,
{
    let mut options = JavaInstallOptions { mcp_root: None };
    let mut args = args;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--mcp-root" => options.mcp_root = Some(next_value(&mut args, "--mcp-root")?),
            "--help" | "-h" => return Err(VerifyCliOptions::usage()),
            other => {
                return Err(format!(
                    "unknown argument: {other}\n\n{}",
                    VerifyCliOptions::usage()
                ))
            }
        }
    }

    Ok(options)
}

fn parse_live_suite<I>(args: I) -> Result<LiveSuiteCliOptions, String>
where
    I: Iterator<Item = String>,
{
    let mut options = LiveSuiteCliOptions {
        mcp_root: None,
        port: 25570,
        java_timeout_secs: 45,
    };
    let mut args = args;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--mcp-root" => options.mcp_root = Some(next_value(&mut args, "--mcp-root")?),
            "--port" => {
                options.port = next_value(&mut args, "--port")?
                    .parse()
                    .map_err(|_| "invalid --port value".to_owned())?;
            }
            "--java-timeout-secs" => {
                options.java_timeout_secs =
                    next_value(&mut args, "--java-timeout-secs")?
                        .parse()
                        .map_err(|_| "invalid --java-timeout-secs value".to_owned())?;
            }
            "--help" | "-h" => return Err(VerifyCliOptions::usage()),
            other => {
                return Err(format!(
                    "unknown argument: {other}\n\n{}",
                    VerifyCliOptions::usage()
                ))
            }
        }
    }

    Ok(options)
}

fn parse_source(value: String) -> Result<VerificationSource, String> {
    match value.as_str() {
        "sample" => Ok(VerificationSource::Sample),
        "live_capture" => Ok(VerificationSource::LiveCapture),
        "canonical_scenario" => Ok(VerificationSource::CanonicalScenario),
        "unknown" => Ok(VerificationSource::Unknown),
        other => Err(format!(
            "unsupported verification source: {other}. expected sample or live_capture"
        )),
    }
}

pub fn run_verify_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = VerifyCliOptions::parse(raw_args)?;

    match options.command {
        VerifyCommand::TraceDiff(options) => run_trace_diff(options),
        VerifyCommand::Scenario(options) => run_scenario(options),
        VerifyCommand::Gate(options) => run_gate(options),
        VerifyCommand::JavaInstall(options) => run_java_install(options),
        VerifyCommand::LiveSuite(options) => run_live_suite(options),
    }
}

pub fn hypixel_gate_decision(server_host: &str) -> GateDecision {
    if !is_hypixel_target(server_host) {
        return GateDecision::Allowed;
    }

    let path = default_status_path();
    let Ok(status) = load_verification_status(&path) else {
        return GateDecision::Blocked(format!(
            "Hypixel is blocked until verification/status.json exists and passes. Run `rmc-client verify gate` after packet, movement, combat, and inventory diffs are collected. Expected status file: {}",
            path.display()
        ));
    };

    if status.hypixel_play_allowed && status.comparison_version == 2 {
        GateDecision::Allowed
    } else {
        GateDecision::Blocked(format!(
            "Strict comparison evidence is required. {}",
            status.summary
        ))
    }
}

pub fn load_verification_status(path: &Path) -> Result<VerificationStatus, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

pub fn is_hypixel_target(host: &str) -> bool {
    let host = host.trim().to_ascii_lowercase();
    host == "hypixel.net" || host == "mc.hypixel.net" || host.ends_with(".hypixel.net")
}

fn run_trace_diff(options: TraceDiffOptions) -> Result<(), String> {
    let rust_path = PathBuf::from(&options.rust_trace);
    let java_path = PathBuf::from(&options.java_trace);
    let rust_lines = load_trace_lines(&rust_path)?;
    let java_lines = load_trace_lines(&java_path)?;
    let report = build_report(
        &options.kind,
        &rust_lines,
        Some(&java_lines),
        options.source,
        Some(&rust_path),
        Some(&java_path),
    )?;
    let report_path = options
        .report_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_report_path(&options.kind));
    write_json_file(&report_path, &report)?;
    print_report_summary(&report, Some(&report_path));
    Ok(())
}

fn run_scenario(options: ScenarioOptions) -> Result<(), String> {
    let rust_lines = match options.kind.as_str() {
        "movement" => movement_scenario_lines(),
        "combat" => combat_scenario_lines(),
        "inventory" => inventory_scenario_lines(),
        other => {
            return Err(format!(
                "unsupported scenario kind: {other}. expected movement, combat, or inventory"
            ))
        }
    };

    let rust_trace_path = options
        .rust_trace_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_trace_path(&options.kind));
    write_lines(&rust_trace_path, &rust_lines)?;

    let java_lines = if let Some(path) = &options.java_trace_path {
        Some(load_trace_lines(Path::new(path))?)
    } else {
        None
    };

    let report = build_report(
        &options.kind,
        &rust_lines,
        java_lines.as_deref(),
        VerificationSource::CanonicalScenario,
        Some(&rust_trace_path),
        options.java_trace_path.as_deref().map(Path::new),
    )?;
    let report_path = options
        .report_path
        .map(PathBuf::from)
        .unwrap_or_else(|| default_report_path(&options.kind));
    write_json_file(&report_path, &report)?;
    print_report_summary(&report, Some(&report_path));
    println!("rust_trace_path={}", rust_trace_path.display());
    Ok(())
}

fn run_gate(options: GateOptions) -> Result<(), String> {
    let required = [
        (
            "packet",
            options
                .packet_report
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_report_path("packet")),
        ),
        (
            "movement",
            options
                .movement_report
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_report_path("movement")),
        ),
        (
            "combat",
            options
                .combat_report
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_report_path("combat")),
        ),
        (
            "inventory",
            options
                .inventory_report
                .as_deref()
                .map(PathBuf::from)
                .unwrap_or_else(|| default_report_path("inventory")),
        ),
    ];

    let mut required_reports = Vec::new();
    let mut missing = Vec::new();
    let mut failing = Vec::new();
    let mut non_production = Vec::new();

    for (kind, path) in required {
        match load_report(&path) {
            Ok(report) => {
                let gate_eligible = report.gate_eligible
                    && report.comparison_version == 2
                    && report.source.is_gate_eligible()
                    && report.rust_event_count > 0
                    && report.java_event_count > 0;
                required_reports.push(RequiredReportStatus {
                    kind: kind.to_owned(),
                    path: path.display().to_string(),
                    present: true,
                    passed: report.passed,
                    source: report.source.clone(),
                    gate_eligible,
                });
                if !report.passed {
                    failing.push(format!("{kind} ({})", path.display()));
                } else if !gate_eligible {
                    non_production.push(format!(
                        "{kind} ({}, source={})",
                        path.display(),
                        report.source.label()
                    ));
                }
            }
            Err(_) => {
                required_reports.push(RequiredReportStatus {
                    kind: kind.to_owned(),
                    path: path.display().to_string(),
                    present: false,
                    passed: false,
                    source: VerificationSource::Unknown,
                    gate_eligible: false,
                });
                missing.push(format!("{kind} ({})", path.display()));
            }
        }
    }

    let hypixel_play_allowed =
        missing.is_empty() && failing.is_empty() && non_production.is_empty();
    let summary = if hypixel_play_allowed {
        "Hypixel verification gate is open. Packet, movement, combat, and inventory live-capture parity reports all passed."
            .to_owned()
    } else {
        let mut reasons = Vec::new();
        if !missing.is_empty() {
            reasons.push(format!("missing reports: {}", missing.join(", ")));
        }
        if !failing.is_empty() {
            reasons.push(format!("failing reports: {}", failing.join(", ")));
        }
        if !non_production.is_empty() {
            reasons.push(format!(
                "non-production reports: {}",
                non_production.join(", ")
            ));
        }
        format!(
            "Hypixel verification gate remains closed until all required reports are live_capture and pass. {}",
            reasons.join("; ")
        )
    };

    let status = VerificationStatus {
        comparison_version: 2,
        hypixel_play_allowed,
        summary,
        required_reports,
    };
    let status_path = PathBuf::from(options.status_path);
    write_json_file(&status_path, &status)?;
    println!("hypixel_play_allowed={}", status.hypixel_play_allowed);
    println!("status_path={}", status_path.display());
    println!("summary={}", status.summary);
    for report in &status.required_reports {
        println!(
            "report kind={} present={} passed={} gate_eligible={} source={} path={}",
            report.kind,
            report.present,
            report.passed,
            report.gate_eligible,
            report.source.label(),
            report.path
        );
    }
    Ok(())
}

fn run_java_install(options: JavaInstallOptions) -> Result<(), String> {
    let install_options = if let Some(mcp_root) = options.mcp_root {
        JavaTraceInstallOptions {
            mcp_root: PathBuf::from(mcp_root),
        }
    } else {
        JavaTraceInstallOptions::default_workspace()
    };

    let summary = install_mcp_java_trace(&install_options)?;
    println!("helper_path={}", summary.helper_path.display());
    println!("scenario_path={}", summary.scenario_path.display());
    println!("backup_root={}", summary.backup_root.display());
    println!("modified_file_count={}", summary.modified_files.len());
    for path in summary.modified_files {
        println!("modified_file={}", path.display());
    }
    Ok(())
}

fn run_live_suite(options: LiveSuiteCliOptions) -> Result<(), String> {
    let mut suite_options = LiveSuiteOptions::default();
    if let Some(mcp_root) = options.mcp_root {
        suite_options.mcp_root = PathBuf::from(mcp_root);
    }
    suite_options.port = options.port;
    suite_options.java_timeout_secs = options.java_timeout_secs;

    let artifacts = run_live_capture_suite(&suite_options)?;
    let packet_report = diff_live_artifact(
        "packet",
        &artifacts.packet_rust_trace,
        &artifacts.packet_java_trace,
    )?;
    let movement_report = diff_live_artifact(
        "movement",
        &artifacts.movement_rust_trace,
        &artifacts.movement_java_trace,
    )?;
    let combat_report = diff_live_artifact(
        "combat",
        &artifacts.combat_rust_trace,
        &artifacts.combat_java_trace,
    )?;
    let inventory_report = diff_live_artifact(
        "inventory",
        &artifacts.inventory_rust_trace,
        &artifacts.inventory_java_trace,
    )?;

    run_gate(GateOptions {
        packet_report: Some(packet_report.display().to_string()),
        movement_report: Some(movement_report.display().to_string()),
        combat_report: Some(combat_report.display().to_string()),
        inventory_report: Some(inventory_report.display().to_string()),
        status_path: default_status_path().display().to_string(),
    })?;
    if load_verification_status(&default_status_path())?.hypixel_play_allowed {
        Ok(())
    } else {
        Err(
            "Strict Java/Rust parity comparisons failed; inspect verification/*-report.json"
                .to_owned(),
        )
    }
}

fn diff_live_artifact(kind: &str, rust_path: &Path, java_path: &Path) -> Result<PathBuf, String> {
    let rust_lines = load_trace_lines(rust_path)?;
    let java_lines = load_trace_lines(java_path)?;
    let report = build_report(
        kind,
        &rust_lines,
        Some(&java_lines),
        VerificationSource::LiveCapture,
        Some(rust_path),
        Some(java_path),
    )?;
    let report_path = default_report_path(kind);
    write_json_file(&report_path, &report)?;
    print_report_summary(&report, Some(&report_path));
    Ok(report_path)
}

fn build_report(
    kind: &str,
    rust_lines: &[String],
    java_lines: Option<&[String]>,
    source: VerificationSource,
    rust_trace_path: Option<&Path>,
    java_trace_path: Option<&Path>,
) -> Result<VerificationReport, String> {
    let rust_records = parse_trace_lines(kind, rust_lines)?;
    let java_records = if let Some(java_lines) = java_lines {
        parse_trace_lines(kind, java_lines)?
    } else {
        Vec::new()
    };
    let missing_java_trace = java_lines.is_none();
    let diffs = if rust_records.is_empty() || (!missing_java_trace && java_records.is_empty()) {
        vec![TraceDiff {
            index: 0,
            reason: "empty trace cannot prove compatibility".to_owned(),
            rust_line: None,
            java_line: None,
        }]
    } else if missing_java_trace {
        vec![TraceDiff {
            index: 0,
            reason: "missing Java trace input".to_owned(),
            rust_line: rust_lines.first().cloned(),
            java_line: None,
        }]
    } else {
        diff_records(kind, &rust_records, &java_records)
    };
    let passed = !missing_java_trace && diffs.is_empty();
    let gate_eligible = passed && source.is_gate_eligible();
    let summary = if missing_java_trace {
        format!(
            "{kind} verification is incomplete: Rust trace has {} records but no Java trace was supplied.",
            rust_records.len()
        )
    } else if passed {
        let suffix = if gate_eligible {
            "gate_eligible=true".to_owned()
        } else {
            format!("gate_eligible=false source={}", source.label())
        };
        format!(
            "{kind} verification passed: {} Rust records matched {} Java records. {}.",
            rust_records.len(),
            java_records.len(),
            suffix
        )
    } else {
        format!(
            "{kind} verification failed: {} diff records across {} Rust and {} Java records.",
            diffs.len(),
            rust_records.len(),
            java_records.len()
        )
    };

    Ok(VerificationReport {
        comparison_version: 2,
        kind: kind.to_owned(),
        rust_event_count: rust_records.len(),
        java_event_count: java_records.len(),
        missing_java_trace,
        passed,
        summary,
        diffs,
        source,
        gate_eligible,
        rust_trace_path: rust_trace_path.map(|path| path.display().to_string()),
        java_trace_path: java_trace_path.map(|path| path.display().to_string()),
    })
}

fn movement_scenario_lines() -> Vec<String> {
    #[derive(Clone)]
    struct Step {
        pressed: Vec<PhysicalInput>,
        released: Vec<PhysicalInput>,
        mouse_dx: f32,
        mouse_dy: f32,
        events: Vec<SimulationEvent>,
    }

    let mut shell = ClientShell::new(ClientShellConfig::vanilla());
    shell.set_mouse_captured(true);
    let frame_time = Duration::from_millis(50);
    let steps = vec![
        Step {
            pressed: vec![PhysicalInput::KeyW, PhysicalInput::LeftControl],
            released: Vec::new(),
            mouse_dx: 2.0,
            mouse_dy: -0.5,
            events: Vec::new(),
        },
        Step {
            pressed: Vec::new(),
            released: Vec::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: Vec::new(),
        },
        Step {
            pressed: Vec::new(),
            released: Vec::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: vec![SimulationEvent::Knockback(KnockbackImpulse::new(
                0.35, 0.42, -0.08,
            ))],
        },
        Step {
            pressed: Vec::new(),
            released: Vec::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: Vec::new(),
        },
        Step {
            pressed: Vec::new(),
            released: Vec::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: vec![SimulationEvent::AuthoritativeState(
                AuthoritativePlayerState {
                    position: Vec3::new(2.0, 0.0, 1.5),
                    velocity: Vec3::ZERO,
                    on_ground: true,
                },
            )],
        },
        Step {
            pressed: vec![PhysicalInput::KeyD],
            released: Vec::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: Vec::new(),
        },
        Step {
            pressed: Vec::new(),
            released: vec![PhysicalInput::LeftControl],
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: Vec::new(),
        },
        Step {
            pressed: Vec::new(),
            released: vec![PhysicalInput::KeyW, PhysicalInput::KeyD],
            mouse_dx: 0.0,
            mouse_dy: 0.0,
            events: Vec::new(),
        },
    ];

    let mut lines = Vec::new();

    for (step_index, step) in steps.into_iter().enumerate() {
        let frame = InputFrame {
            pressed_inputs: step.pressed,
            released_inputs: step.released,
            mouse_delta_x: step.mouse_dx,
            mouse_delta_y: step.mouse_dy,
            hotbar_scroll: 0,
        };
        let output = shell.advance_with_events(frame_time, &frame, &step.events);
        lines.push(format!(
            "movement record=state step={} total_ticks={} ticks_run={} pos_x={:.6} pos_y={:.6} pos_z={:.6} vel_x={:.6} vel_y={:.6} vel_z={:.6} yaw={:.6} pitch={:.6} network_x={:.6} network_y={:.6} network_z={:.6} on_ground={} sprinting={} sneaking={} sprint_reset_ticks={} selected_slot={}",
            step_index,
            output.total_ticks,
            output.ticks_run,
            output.simulation.player.position.x,
            output.simulation.player.position.y,
            output.simulation.player.position.z,
            output.simulation.velocity.x,
            output.simulation.velocity.y,
            output.simulation.velocity.z,
            output.render.camera.yaw,
            output.render.camera.pitch,
            output.simulation.network.position.x,
            output.simulation.network.position.y,
            output.simulation.network.position.z,
            output.simulation.player.on_ground,
            output.simulation.player.sprinting,
            output.simulation.player.sneaking,
            output.simulation.sprint_reset_ticks,
            output.simulation.player.selected_hotbar_slot,
        ));

        let first_tick = output.total_ticks.saturating_sub(output.ticks_run as u64);
        for (packet_index, packet) in output.packets.iter().enumerate() {
            lines.push(render_serverbound_packet_line(
                "movement",
                step_index,
                packet_index,
                first_tick + packet_index as u64 + 1,
                packet,
            ));
        }
    }

    lines
}

fn combat_scenario_lines() -> Vec<String> {
    let mut combat = CombatState::new(CombatConfig::vanilla());
    let mut lines = Vec::new();
    let player_entity_id = 12;
    let mut packet_seq = 0usize;

    for packet in combat.sync_action_state(player_entity_id, true, false) {
        lines.push(render_serverbound_packet_line(
            "combat", 0, packet_seq, 0, &packet,
        ));
        packet_seq += 1;
    }

    for packet in combat.attack_entity(44) {
        lines.push(render_serverbound_packet_line(
            "combat", 1, packet_seq, 0, &packet,
        ));
        packet_seq += 1;
    }

    for packet in combat.start_using_item(Some(ItemStack::simple(261, 1, 0))) {
        lines.push(render_serverbound_packet_line(
            "combat", 2, packet_seq, 0, &packet,
        ));
        packet_seq += 1;
    }

    combat.tick_feedback();
    combat.tick_feedback();
    let snapshot = combat.snapshot();
    lines.push(format!(
        "combat record=state step=2 using_item={} use_ticks={} hurt_ticks={} health={:.3} food_level={} saturation={:.3}",
        snapshot.using_item.is_some(),
        snapshot
            .using_item
            .as_ref()
            .map(|state| state.use_ticks)
            .unwrap_or(0),
        snapshot.hurt_ticks,
        snapshot.health,
        snapshot.food_level,
        snapshot.saturation,
    ));

    for packet in combat.release_using_item() {
        lines.push(render_serverbound_packet_line(
            "combat", 3, packet_seq, 0, &packet,
        ));
        packet_seq += 1;
    }

    let health_update = combat.apply_health_update(&UpdateHealthPacket {
        health: 17.0,
        food_level: 19,
        saturation: 4.0,
    });
    let snapshot = combat.snapshot();
    lines.push(format!(
        "combat record=health_update step=4 health={:.3} food_level={} saturation={:.3} hurt_feedback={} hurt_ticks={}",
        snapshot.health,
        snapshot.food_level,
        snapshot.saturation,
        health_update.hurt_feedback,
        snapshot.hurt_ticks,
    ));

    let velocity_update = combat.apply_entity_velocity(
        &EntityVelocityPacket {
            entity_id: player_entity_id,
            velocity_x: 1600,
            velocity_y: 800,
            velocity_z: -400,
        },
        Some(player_entity_id),
    );
    let snapshot = combat.snapshot();
    lines.push(format!(
        "combat record=velocity step=5 vel_x={:.6} vel_y={:.6} vel_z={:.6} simulation_events={}",
        snapshot.last_velocity.x,
        snapshot.last_velocity.y,
        snapshot.last_velocity.z,
        velocity_update.simulation_events.len(),
    ));
    for event in velocity_update.simulation_events {
        match event {
            SimulationEvent::Knockback(impulse) => lines.push(format!(
                "combat record=simulation_event step=5 event=Knockback x={:.6} y={:.6} z={:.6} resets_sprint={}",
                impulse.velocity.x,
                impulse.velocity.y,
                impulse.velocity.z,
                impulse.resets_sprint,
            )),
            SimulationEvent::AddVelocity(motion) => lines.push(format!("simulation_event=AddVelocity x={} y={} z={}",motion.x,motion.y,motion.z)),
            SimulationEvent::AuthoritativeState(_) | SimulationEvent::Teleport { .. } => {}
        }
    }

    let snapshot = combat.snapshot();
    lines.push(format!(
        "combat record=final step=6 server_sprint_state={} server_sneak_state={} using_item={} health={:.3} food_level={} saturation={:.3} hurt_ticks={} last_vel_x={:.6} last_vel_y={:.6} last_vel_z={:.6}",
        snapshot.server_sprint_state,
        snapshot.server_sneak_state,
        snapshot.using_item.is_some(),
        snapshot.health,
        snapshot.food_level,
        snapshot.saturation,
        snapshot.hurt_ticks,
        snapshot.last_velocity.x,
        snapshot.last_velocity.y,
        snapshot.last_velocity.z,
    ));

    lines
}

fn inventory_scenario_lines() -> Vec<String> {
    let mut inventory = InventoryState::new();
    let mut lines = Vec::new();
    let mut packet_seq = 0usize;

    if let Some(packet) = inventory.sync_selected_hotbar_slot(2) {
        lines.push(render_serverbound_packet_line(
            "inventory",
            0,
            packet_seq,
            0,
            &packet,
        ));
        packet_seq += 1;
    }

    let open_update = inventory.apply_open_window(&OpenWindowPacket {
        window_id: 4,
        inventory_type: "minecraft:chest".to_owned(),
        window_title_json: "{\"text\":\"Loot\"}".to_owned(),
        slot_count: 27,
        entity_id: None,
    });
    lines.push(format!(
        "inventory record=open_window step=1 window_id=4 updated_windows={} slot_count={} inventory_type=minecraft:chest",
        open_update.updated_windows.len(),
        inventory
            .open_window()
            .and_then(|window| window.metadata.as_ref())
            .map(|metadata| metadata.slot_count)
            .unwrap_or(0),
    ));

    let slot_update = inventory.apply_set_slot(&SetSlotPacket {
        window_id: 4,
        slot_id: 13,
        item: Some(ItemStack::simple(5, 16, 0)),
    });
    lines.push(format!(
        "inventory record=set_slot step=2 window_id=4 slot_id=13 updated_windows={} slot_item={}",
        slot_update.updated_windows.len(),
        slot_value_string(
            inventory
                .open_window()
                .and_then(|window| window.slot(13))
                .cloned()
                .unwrap_or(None)
        ),
    ));

    let click_packet = inventory.queue_pickup_click(4, 13, 0);
    lines.push(render_serverbound_packet_line(
        "inventory",
        3,
        packet_seq,
        0,
        &click_packet,
    ));
    packet_seq += 1;
    lines.push(format!(
        "inventory record=pending_transaction step=3 count={}",
        inventory.pending_transactions().len()
    ));

    let confirm_update =
        inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
            window_id: 4,
            action_number: 1,
            accepted: false,
        });
    lines.push(format!(
        "inventory record=confirm_transaction step=4 accepted=false rejected_transactions={} ack_packets={} pending_transactions={}",
        confirm_update.rejected_transactions.len(),
        confirm_update.outbound_packets.len(),
        inventory.pending_transactions().len(),
    ));
    for packet in &confirm_update.outbound_packets {
        lines.push(render_serverbound_packet_line(
            "inventory",
            4,
            packet_seq,
            0,
            packet,
        ));
        packet_seq += 1;
    }

    if let Some(packet) = inventory.close_open_window() {
        lines.push(render_serverbound_packet_line(
            "inventory",
            5,
            packet_seq,
            0,
            &packet,
        ));
    }

    lines.push(format!(
        "inventory record=final step=6 selected_slot={} window_open={} carried_item={} pending_transactions={}",
        inventory.selected_hotbar_slot(),
        inventory.open_window().is_some(),
        slot_value_string(inventory.carried_item().clone()),
        inventory.pending_transactions().len(),
    ));

    lines
}

fn parse_trace_lines(kind: &str, lines: &[String]) -> Result<Vec<NormalizedRecord>, String> {
    let mut records = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let record = if kind == "packet" && line.starts_with("packet ") {
            parse_key_value_line(line)?
        } else if kind == "packet" && line.contains(" id=") && line.contains(" len=") {
            parse_packet_summary_line(line)?
        } else {
            parse_key_value_line(line)?
        };
        records.push(record);
    }

    Ok(records)
}

fn parse_packet_summary_line(line: &str) -> Result<NormalizedRecord, String> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 5 {
        return Err(format!("invalid packet trace line: {line}"));
    }

    let len_index = tokens
        .iter()
        .position(|token| token.starts_with("len="))
        .ok_or_else(|| format!("missing len= field in packet trace line: {line}"))?;
    let compression_index = tokens
        .iter()
        .position(|token| token.starts_with("compression="))
        .ok_or_else(|| format!("missing compression= field in packet trace line: {line}"))?;
    let id_index = tokens
        .iter()
        .position(|token| token.starts_with("id="))
        .ok_or_else(|| format!("missing id= field in packet trace line: {line}"))?;

    if id_index + 1 >= len_index {
        return Err(format!("missing packet name in packet trace line: {line}"));
    }

    let packet_name = tokens[id_index + 1..len_index].join(" ");
    let mut fields = BTreeMap::new();
    fields.insert("state".to_owned(), tokens[0].to_owned());
    fields.insert("direction".to_owned(), tokens[1].to_owned());
    fields.insert(
        "id".to_owned(),
        normalize_field_value("id", tokens[id_index].trim_start_matches("id=")),
    );
    fields.insert(
        "name".to_owned(),
        normalize_field_value("name", &packet_name),
    );
    fields.insert(
        "len".to_owned(),
        normalize_field_value("len", tokens[len_index].trim_start_matches("len=")),
    );
    fields.insert(
        "compression".to_owned(),
        normalize_field_value(
            "compression",
            tokens[compression_index].trim_start_matches("compression="),
        ),
    );

    Ok(NormalizedRecord {
        label: "packet".to_owned(),
        fields,
        original: line.to_owned(),
    })
}

fn parse_key_value_line(line: &str) -> Result<NormalizedRecord, String> {
    let mut label_tokens = Vec::new();
    let mut fields = BTreeMap::new();

    for token in line.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            fields.insert(key.to_owned(), normalize_field_value(key, value));
        } else {
            label_tokens.push(token.to_owned());
        }
    }

    if fields.is_empty() {
        return Err(format!("trace line has no key=value fields: {line}"));
    }

    Ok(NormalizedRecord {
        label: if label_tokens.is_empty() {
            "record".to_owned()
        } else {
            label_tokens.join(" ")
        },
        fields,
        original: line.to_owned(),
    })
}

fn diff_records(
    kind: &str,
    rust_records: &[NormalizedRecord],
    java_records: &[NormalizedRecord],
) -> Vec<TraceDiff> {
    // TCP preserves order within each direction. Independent send/receive event
    // interleavings depend on scheduling, so compare both ordered streams in full.
    let mut rust_records = rust_records.to_vec();
    let mut java_records = java_records.to_vec();
    if kind == "packet" {
        for records in [&mut rust_records, &mut java_records] {
            records.sort_by_key(|record| record.fields.get("direction").cloned());
        }
    }
    let mut diffs = Vec::new();
    let max_len = rust_records.len().max(java_records.len());

    for index in 0..max_len {
        match (rust_records.get(index), java_records.get(index)) {
            (Some(rust), Some(java)) => {
                let mut mismatches = Vec::new();
                if rust.label != java.label {
                    mismatches.push(format!(
                        "label mismatch: rust={} java={}",
                        rust.label, java.label
                    ));
                }

                let mut keys = BTreeSet::new();
                keys.extend(rust.fields.keys().cloned());
                keys.extend(java.fields.keys().cloned());

                for key in keys {
                    match (rust.fields.get(&key), java.fields.get(&key)) {
                        (Some(rust_value), Some(java_value)) => {
                            if !values_match(kind, &key, rust_value, java_value) {
                                mismatches.push(format!(
                                    "field {} mismatch: rust={} java={}",
                                    key, rust_value, java_value
                                ));
                            }
                        }
                        (Some(_), None) => {
                            mismatches.push(format!("field {} missing in Java trace", key))
                        }
                        (None, Some(_)) => {
                            mismatches.push(format!("field {} missing in Rust trace", key))
                        }
                        (None, None) => {}
                    }
                }

                if !mismatches.is_empty() {
                    diffs.push(TraceDiff {
                        index,
                        reason: mismatches.join("; "),
                        rust_line: Some(rust.original.clone()),
                        java_line: Some(java.original.clone()),
                    });
                }
            }
            (Some(rust), None) => diffs.push(TraceDiff {
                index,
                reason: "record missing in Java trace".to_owned(),
                rust_line: Some(rust.original.clone()),
                java_line: None,
            }),
            (None, Some(java)) => diffs.push(TraceDiff {
                index,
                reason: "record missing in Rust trace".to_owned(),
                rust_line: None,
                java_line: Some(java.original.clone()),
            }),
            (None, None) => {}
        }
    }

    diffs
}

fn values_match(kind: &str, key: &str, rust_value: &str, java_value: &str) -> bool {
    if rust_value == java_value {
        return true;
    }

    if kind == "packet" && key == "state" {
        let canonical = |state: &str| match state {
            "Handshake" | "Handshaking" => "handshake".to_owned(),
            other => other.to_ascii_lowercase(),
        };
        return canonical(rust_value) == canonical(java_value);
    }
    if kind == "packet" && key == "name" {
        return packet_name_matches(rust_value, java_value);
    }

    match (parse_numeric(rust_value), parse_numeric(java_value)) {
        (Some(rust_numeric), Some(java_numeric)) => {
            let tolerance = numeric_tolerance(kind, key);
            (rust_numeric - java_numeric).abs() <= tolerance
        }
        _ => false,
    }
}

fn packet_name_matches(rust_value: &str, java_value: &str) -> bool {
    if rust_value == java_value {
        return true;
    }

    if java_value
        .chars()
        .all(|character| character.is_ascii_digit())
    {
        return true;
    }

    normalize_packet_name(rust_value) == normalize_packet_name(java_value)
}

fn normalize_packet_name(value: &str) -> String {
    let mut trimmed = value.trim();
    // Java protocol classes may carry C00/S08 prefixes with or without Packet.
    let bytes = trimmed.as_bytes();
    if bytes.len() >= 3
        && matches!(bytes[0], b'C' | b'S')
        && bytes[1].is_ascii_hexdigit()
        && bytes[2].is_ascii_hexdigit()
    {
        trimmed = trimmed[3..].trim_start_matches('_');
    }
    trimmed = trimmed.trim_start_matches("Packet");
    let compact: String = trimmed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    match compact.as_str() {
        "handshakerequest" => "handshake".to_owned(),
        "playerpositionandlook" | "playerposlook" => "playerpositionlook".to_owned(),
        other => other.to_owned(),
    }
}

fn numeric_tolerance(kind: &str, key: &str) -> f64 {
    if kind == "packet" {
        return 0.0;
    }

    if matches!(
        key,
        "pos_x"
            | "pos_y"
            | "pos_z"
            | "vel_x"
            | "vel_y"
            | "vel_z"
            | "network_x"
            | "network_y"
            | "network_z"
            | "x"
            | "y"
            | "z"
            | "last_vel_x"
            | "last_vel_y"
            | "last_vel_z"
    ) {
        return 0.0005;
    }

    if matches!(key, "yaw" | "pitch" | "health" | "saturation") {
        return 0.001;
    }

    0.0
}

fn parse_numeric(value: &str) -> Option<f64> {
    if let Some(hex) = value.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).ok().map(|value| value as f64)
    } else {
        value.parse::<f64>().ok()
    }
}

fn normalize_field_value(key: &str, value: &str) -> String {
    let trimmed = value.trim().trim_matches('"').trim_matches('\'');
    match key {
        "packet" | "name" | "state" | "direction" | "compression" | "record" | "event"
        | "action" | "inventory_type" => trimmed.replace(' ', "_"),
        _ => trimmed.to_owned(),
    }
}

fn render_serverbound_packet_line(
    kind: &str,
    step: usize,
    packet_index: usize,
    tick: u64,
    packet: &PlayServerboundPacket,
) -> String {
    let mut parts = vec![
        kind.to_owned(),
        "record=packet".to_owned(),
        format!("step={step}"),
        format!("packet_index={packet_index}"),
        format!("tick={tick}"),
        format!("packet={}", packet_label(packet)),
    ];

    match packet {
        PlayServerboundPacket::UseEntity(packet) => {
            parts.push(format!("entity_id={}", packet.entity_id));
            parts.push(format!("action={:?}", packet.action));
            if let Some(target) = packet.target {
                parts.push(format!("target_x={:.6}", target[0]));
                parts.push(format!("target_y={:.6}", target[1]));
                parts.push(format!("target_z={:.6}", target[2]));
            }
        }
        PlayServerboundPacket::Player(packet) => {
            parts.push(format!("on_ground={}", packet.on_ground));
        }
        PlayServerboundPacket::PlayerPosition(packet) => {
            parts.push(format!("x={:.6}", packet.x));
            parts.push(format!("y={:.6}", packet.y));
            parts.push(format!("z={:.6}", packet.z));
            parts.push(format!("on_ground={}", packet.on_ground));
        }
        PlayServerboundPacket::PlayerLook(packet) => {
            parts.push(format!("yaw={:.6}", packet.yaw));
            parts.push(format!("pitch={:.6}", packet.pitch));
            parts.push(format!("on_ground={}", packet.on_ground));
        }
        PlayServerboundPacket::PlayerPositionAndLook(packet) => {
            parts.push(format!("x={:.6}", packet.x));
            parts.push(format!("y={:.6}", packet.y));
            parts.push(format!("z={:.6}", packet.z));
            parts.push(format!("yaw={:.6}", packet.yaw));
            parts.push(format!("pitch={:.6}", packet.pitch));
            parts.push(format!("on_ground={}", packet.on_ground));
        }
        PlayServerboundPacket::PlayerDigging(packet) => {
            parts.push(format!("action={:?}", packet.action));
            parts.push(format!("x={}", packet.position.x));
            parts.push(format!("y={}", packet.position.y));
            parts.push(format!("z={}", packet.position.z));
            parts.push(format!("face={}", packet.face));
        }
        PlayServerboundPacket::PlayerBlockPlacement(packet) => {
            parts.push(format!("x={}", packet.position.x));
            parts.push(format!("y={}", packet.position.y));
            parts.push(format!("z={}", packet.position.z));
            parts.push(format!("face={}", packet.face));
            parts.push(format!(
                "held_item={}",
                slot_value_string(packet.held_item.clone())
            ));
            parts.push(format!("cursor_x={:.6}", packet.cursor_x));
            parts.push(format!("cursor_y={:.6}", packet.cursor_y));
            parts.push(format!("cursor_z={:.6}", packet.cursor_z));
        }
        PlayServerboundPacket::HeldItemChange(packet) => {
            parts.push(format!("slot={}", packet.slot));
        }
        PlayServerboundPacket::EntityAction(packet) => {
            parts.push(format!("entity_id={}", packet.entity_id));
            parts.push(format!("action={:?}", packet.action));
            parts.push(format!("aux_data={}", packet.aux_data));
        }
        PlayServerboundPacket::CloseWindow(packet) => {
            parts.push(format!("window_id={}", packet.window_id));
        }
        PlayServerboundPacket::ClickWindow(packet) => {
            parts.push(format!("window_id={}", packet.window_id));
            parts.push(format!("slot_id={}", packet.slot_id));
            parts.push(format!("button={}", packet.button));
            parts.push(format!("action_number={}", packet.action_number));
            parts.push(format!("mode={}", packet.mode));
            parts.push(format!(
                "clicked_item={}",
                slot_value_string(packet.clicked_item.clone())
            ));
        }
        PlayServerboundPacket::ConfirmTransaction(packet) => {
            parts.push(format!("window_id={}", packet.window_id));
            parts.push(format!("action_number={}", packet.action_number));
            parts.push(format!("accepted={}", packet.accepted));
        }
        PlayServerboundPacket::ChatMessage(packet) => {
            parts.push(format!("message={}", packet.message.replace(' ', "_")));
        }
        PlayServerboundPacket::KeepAlive(packet) => {
            parts.push(format!("id={}", packet.id));
        }
        PlayServerboundPacket::ClientStatus(action) => {
            parts.push(format!("action={action}"));
        }
        PlayServerboundPacket::Animation(_)
        | PlayServerboundPacket::ClientSettings(_)
        | PlayServerboundPacket::CustomPayload(_) => {}
    }

    parts.join(" ")
}

fn packet_label(packet: &PlayServerboundPacket) -> &'static str {
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
        PlayServerboundPacket::ClientStatus(_) => "ClientStatus",
        PlayServerboundPacket::ClientSettings(_) => "ClientSettings",
        PlayServerboundPacket::CustomPayload(_) => "CustomPayload",
    }
}

fn slot_value_string(slot: Option<ItemStack>) -> String {
    match slot {
        Some(item) => format!("{}:{}:{}", item.item_id, item.count, item.damage),
        None => "none".to_owned(),
    }
}

fn load_trace_lines(path: &Path) -> Result<Vec<String>, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    Ok(contents
        .lines()
        .map(|line| line.trim_start_matches('\u{feff}').to_owned())
        .collect())
}

fn load_report(path: &Path) -> Result<VerificationReport, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn default_status_path() -> PathBuf {
    PathBuf::from(DEFAULT_STATUS_PATH)
}

fn default_trace_path(kind: &str) -> PathBuf {
    PathBuf::from("verification").join(format!("{kind}-rust.trace"))
}

fn default_report_path(kind: &str) -> PathBuf {
    PathBuf::from("verification").join(format!("{kind}-report.json"))
}

fn write_lines(path: &Path, lines: &[String]) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let contents = if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    };
    fs::write(path, contents)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn write_json_file<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("failed to serialize {}: {error}", path.display()))?;
    fs::write(path, bytes).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    Ok(())
}

fn print_report_summary(report: &VerificationReport, report_path: Option<&Path>) {
    println!("kind={}", report.kind);
    println!("passed={}", report.passed);
    println!("source={}", report.source.label());
    println!("gate_eligible={}", report.gate_eligible);
    println!("missing_java_trace={}", report.missing_java_trace);
    println!("rust_event_count={}", report.rust_event_count);
    println!("java_event_count={}", report.java_event_count);
    println!("diff_count={}", report.diffs.len());
    println!("summary={}", report.summary);
    if let Some(report_path) = report_path {
        println!("report_path={}", report_path.display());
    }

    for diff in report.diffs.iter().take(12) {
        println!("diff index={} reason={}", diff.index, diff.reason);
    }
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

#[cfg(test)]
mod tests {
    use super::{
        build_report, is_hypixel_target, movement_scenario_lines, packet_name_matches,
        parse_packet_summary_line, slot_value_string, VerificationSource,
    };
    use rmc_net::codec::play::ItemStack;

    #[test]
    fn detects_hypixel_hosts() {
        assert!(is_hypixel_target("hypixel.net"));
        assert!(is_hypixel_target("mc.hypixel.net"));
        assert!(is_hypixel_target("duels.hypixel.net"));
        assert!(!is_hypixel_target("localhost"));
    }

    #[test]
    fn normalizes_packet_summary_lines() {
        let record = parse_packet_summary_line(
            "Play Clientbound id=0x00 Keep Alive len=2 compression=Disabled",
        )
        .expect("packet summary should parse");
        assert_eq!(record.label, "packet");
        assert_eq!(
            record.fields.get("name").map(String::as_str),
            Some("Keep_Alive")
        );
        assert_eq!(record.fields.get("id").map(String::as_str), Some("0x00"));
    }

    #[test]
    fn scenario_report_flags_missing_java_trace() {
        let rust_lines = movement_scenario_lines();
        let report = build_report(
            "movement",
            &rust_lines,
            None,
            VerificationSource::CanonicalScenario,
            None,
            None,
        )
        .expect("report should build");
        assert!(!report.passed);
        assert!(report.missing_java_trace);
        assert_eq!(report.java_event_count, 0);
        assert!(!report.gate_eligible);
    }

    #[test]
    fn live_comparison_does_not_discard_vertical_motion() {
        let rust = vec![
            "movement record=state tick=1 pos_x=1 pos_z=0 pos_y=0 vel_y=0 packet=PlayerPosition"
                .to_owned(),
        ];
        let java = vec![
            "movement record=state tick=1 pos_x=1 pos_z=0 pos_y=64 vel_y=1 packet=PlayerPosition"
                .to_owned(),
        ];
        let report = build_report(
            "movement",
            &rust,
            Some(&java),
            VerificationSource::LiveCapture,
            None,
            None,
        )
        .unwrap();
        assert!(!report.passed);
    }

    #[test]
    fn packet_comparison_preserves_each_tcp_direction_order() {
        let sent = "packet state=Play direction=Serverbound id=0x03 name=Player len=2 compression=Disabled".to_owned();
        let received = "packet state=Play direction=Clientbound id=0x00 name=Keep_Alive len=2 compression=Disabled".to_owned();
        let rust = vec![sent.clone(), received.clone()];
        let java = vec![received, sent];
        assert!(
            build_report(
                "packet",
                &rust,
                Some(&java),
                VerificationSource::LiveCapture,
                None,
                None
            )
            .unwrap()
            .passed
        );
        let changed = vec![java[0].clone(), java[1].replace("id=0x03", "id=0x04")];
        assert!(
            !build_report(
                "packet",
                &rust,
                Some(&changed),
                VerificationSource::LiveCapture,
                None,
                None
            )
            .unwrap()
            .passed
        );
        assert!(
            !build_report(
                "packet",
                &rust,
                Some(&java[..1]),
                VerificationSource::LiveCapture,
                None,
                None
            )
            .unwrap()
            .passed
        );
    }

    #[test]
    fn known_protocol_aliases_do_not_change_packet_identity() {
        let rust = vec![
            "Handshake Serverbound id=0x00 Handshake Request len=15 compression=Disabled"
                .to_owned(),
        ];
        let java = vec!["packet state=Handshaking direction=Serverbound id=0x00 name=C00_Handshake len=15 compression=Disabled".to_owned()];
        assert!(
            build_report(
                "packet",
                &rust,
                Some(&java),
                VerificationSource::LiveCapture,
                None,
                None
            )
            .unwrap()
            .passed
        );
    }

    #[test]
    fn empty_live_traces_are_not_evidence() {
        let report = build_report(
            "movement",
            &[],
            Some(&[]),
            VerificationSource::LiveCapture,
            None,
            None,
        )
        .unwrap();
        assert!(!report.passed);
        assert!(!report.gate_eligible);
    }

    #[test]
    fn identical_reports_pass() {
        let lines = vec![
            "combat record=packet step=0 packet=Animation".to_owned(),
            "combat record=state step=1 health=20.000".to_owned(),
        ];
        let report = build_report(
            "combat",
            &lines,
            Some(&lines),
            VerificationSource::LiveCapture,
            None,
            None,
        )
        .expect("report should build");
        assert!(report.passed);
        assert!(report.diffs.is_empty());
        assert!(report.gate_eligible);
    }

    #[test]
    fn packet_name_matching_accepts_java_fallbacks() {
        assert!(packet_name_matches("Keep_Alive", "C00PacketKeepAlive"));
        assert!(packet_name_matches("Chunk_Data", "S21PacketChunkData"));
        assert!(packet_name_matches("Chunk_Data", "33"));
    }

    #[test]
    fn sample_reports_do_not_unlock_gate() {
        let lines = vec!["packet state=Play direction=Clientbound id=0x00 name=Keep_Alive len=2 compression=Disabled".to_owned()];
        let report = build_report(
            "packet",
            &lines,
            Some(&lines),
            VerificationSource::Sample,
            None,
            None,
        )
        .expect("report should build");
        assert!(report.passed);
        assert!(!report.gate_eligible);
    }

    #[test]
    fn slot_strings_are_stable() {
        assert_eq!(slot_value_string(None), "none");
        assert_eq!(
            slot_value_string(Some(ItemStack::simple(261, 1, 0))),
            "261:1:0"
        );
    }
}
