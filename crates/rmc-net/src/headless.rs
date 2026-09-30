//! End-to-end headless protocol runner for M1.

use crate::address::resolve_connect_target;
use crate::auth::{AuthError, MojangSessionJoiner, OnlineAccount, SessionJoiner};
use crate::codec::login::{EncryptionResponse, LoginServerboundPacket};
use crate::crypto::{build_login_encryption_response, CryptoError};
use crate::driver::{DriverError, DriverEvent, HeadlessDriver, HeadlessDriverConfig};
use crate::session::SessionAction;
use crate::trace::TraceSink;
use crate::transport::{DriverTransport, TransportError};
use crate::zlib::DefaultZlibCodec;
use std::io;
use std::net::TcpStream;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthenticationMode {
    Offline,
    Online(OnlineAccount),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeadlessRunConfig {
    pub driver: HeadlessDriverConfig,
    pub read_timeout: Duration,
    pub max_runtime: Option<Duration>,
    pub stop_after_join: bool,
}

impl HeadlessRunConfig {
    pub fn vanilla_headless(
        server_address: impl Into<String>,
        server_port: u16,
        username: impl Into<String>,
    ) -> Self {
        Self {
            driver: HeadlessDriverConfig::vanilla_headless(
                47,
                server_address,
                server_port,
                username,
            ),
            read_timeout: Duration::from_millis(250),
            max_runtime: Some(Duration::from_secs(30)),
            stop_after_join: false,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HeadlessRunSummary {
    pub reached_play: bool,
    pub joined_game: bool,
    pub compression_enabled: bool,
    pub encryption_enabled: bool,
    pub ignored_packets: usize,
    pub disconnect_reason_json: Option<String>,
    pub ended_by_eof: bool,
    pub timed_out: bool,
}

#[derive(Debug)]
pub enum HeadlessRunError {
    Io(io::Error),
    Driver(DriverError),
    Transport(TransportError),
    Auth(AuthError),
    Crypto(CryptoError),
    OnlineAccountRequired,
}

impl From<io::Error> for HeadlessRunError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<DriverError> for HeadlessRunError {
    fn from(value: DriverError) -> Self {
        Self::Driver(value)
    }
}

impl From<TransportError> for HeadlessRunError {
    fn from(value: TransportError) -> Self {
        Self::Transport(value)
    }
}

impl From<AuthError> for HeadlessRunError {
    fn from(value: AuthError) -> Self {
        Self::Auth(value)
    }
}

impl From<CryptoError> for HeadlessRunError {
    fn from(value: CryptoError) -> Self {
        Self::Crypto(value)
    }
}

pub fn run_headless<T: TraceSink>(
    config: &HeadlessRunConfig,
    authentication: &AuthenticationMode,
    trace_sink: &mut T,
) -> Result<HeadlessRunSummary, HeadlessRunError> {
    run_headless_with_joiner(config, authentication, &MojangSessionJoiner, trace_sink)
}

pub fn run_headless_with_joiner<J: SessionJoiner, T: TraceSink>(
    config: &HeadlessRunConfig,
    authentication: &AuthenticationMode,
    joiner: &J,
    trace_sink: &mut T,
) -> Result<HeadlessRunSummary, HeadlessRunError> {
    let connect_target =
        resolve_connect_target(&config.driver.server_address, config.driver.server_port);
    let stream = TcpStream::connect((connect_target.host.as_str(), connect_target.port))?;
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(config.read_timeout))?;

    let mut transport = DriverTransport::new(stream);
    let mut driver = HeadlessDriver::new(config.driver.clone());
    let zlib = DefaultZlibCodec;
    let started = Instant::now();
    let mut summary = HeadlessRunSummary::default();

    driver.bootstrap_login(Some(&zlib), Some(&mut *trace_sink))?;
    transport.flush_outbound(&mut driver)?;

    loop {
        if let Some(max_runtime) = config.max_runtime {
            if started.elapsed() >= max_runtime {
                summary.timed_out = true;
                break;
            }
        }

        let cycle = match transport.read_once(&mut driver, Some(&zlib), Some(&mut *trace_sink)) {
            Ok(cycle) => cycle,
            Err(TransportError::Io(error)) if is_timeout(&error) => {
                if driver.has_pending_outbound() || transport.has_pending_outbound() {
                    transport.flush_outbound(&mut driver)?;
                }

                continue;
            }
            Err(error) => return Err(HeadlessRunError::Transport(error)),
        };

        if cycle.reached_eof {
            summary.ended_by_eof = true;
            break;
        }

        for event in cycle.events {
            handle_driver_event(
                event,
                authentication,
                joiner,
                &zlib,
                &mut transport,
                &mut driver,
                trace_sink,
                &mut summary,
            )?;
        }

        if driver.has_pending_outbound() || transport.has_pending_outbound() {
            transport.flush_outbound(&mut driver)?;
        }

        if summary.disconnect_reason_json.is_some() {
            break;
        }

        if config.stop_after_join && summary.joined_game {
            break;
        }
    }

    Ok(summary)
}

fn handle_driver_event<J: SessionJoiner, T: TraceSink>(
    event: DriverEvent,
    authentication: &AuthenticationMode,
    joiner: &J,
    zlib: &DefaultZlibCodec,
    transport: &mut DriverTransport<TcpStream>,
    driver: &mut HeadlessDriver,
    trace_sink: &mut T,
    summary: &mut HeadlessRunSummary,
) -> Result<(), HeadlessRunError> {
    match event {
        DriverEvent::InboundPlayPacket(_) => {}
        DriverEvent::IgnoredPacket { .. } => {
            summary.ignored_packets += 1;
        }
        DriverEvent::SessionAction(action) => match action {
            SessionAction::EncryptionRequested(request) => {
                let account = match authentication {
                    AuthenticationMode::Offline => {
                        return Err(HeadlessRunError::OnlineAccountRequired);
                    }
                    AuthenticationMode::Online(account) => account,
                };
                let material = build_login_encryption_response(&request)?;
                joiner.join_server(account, &material.server_hash)?;

                let response = LoginServerboundPacket::EncryptionResponse(EncryptionResponse {
                    shared_secret: material.encrypted_shared_secret,
                    verify_token: material.encrypted_verify_token,
                });
                driver.queue_login_packet(&response, Some(zlib), Some(&mut *trace_sink))?;
                transport.flush_outbound(driver)?;
                transport.enable_encryption(material.shared_secret)?;
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
        },
    }

    Ok(())
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}
