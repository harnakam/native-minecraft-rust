//! Headless wire driver for the M1 protocol bring-up.

use crate::codec::handshake::HandshakeRequest;
use crate::codec::login::{LoginClientboundPacket, LoginServerboundPacket};
use crate::codec::play::{
    ClientSettingsPacket, CustomPayloadPacket, PlayClientboundPacket, PlayServerboundPacket,
};
use crate::codec::{split_packet_bytes, CodecError};
use crate::compression::{CompressionState, ZlibCodec};
use crate::framing::{encode_frame, CompressionDisposition, FrameDecoder, FrameError, FrameLimits};
use crate::protocol::{PacketDirection, ProtocolState};
use crate::session::{HeadlessSession, PlayerPose, SessionAction, SessionError, SessionPhase};
use crate::trace::{build_trace_event_lossy, PacketTraceEvent, TraceError, TraceSink};
use std::mem;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeadlessDriverConfig {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub username: String,
    pub frame_limits: FrameLimits,
    pub teleport_on_ground: bool,
    pub client_settings: ClientSettingsPacket,
    pub client_brand: String,
}

impl HeadlessDriverConfig {
    pub fn vanilla_headless(
        protocol_version: i32,
        server_address: impl Into<String>,
        server_port: u16,
        username: impl Into<String>,
    ) -> Self {
        Self {
            protocol_version,
            server_address: server_address.into(),
            server_port,
            username: username.into(),
            frame_limits: FrameLimits::default(),
            teleport_on_ground: false,
            client_settings: ClientSettingsPacket::vanilla_headless_defaults(),
            client_brand: "RustMinecraft".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriverError {
    Codec(CodecError),
    Frame(FrameError),
    Session(SessionError),
    Trace(TraceError),
    UnexpectedInboundPhase(SessionPhase),
}

impl From<CodecError> for DriverError {
    fn from(value: CodecError) -> Self {
        Self::Codec(value)
    }
}

impl From<FrameError> for DriverError {
    fn from(value: FrameError) -> Self {
        Self::Frame(value)
    }
}

impl From<SessionError> for DriverError {
    fn from(value: SessionError) -> Self {
        Self::Session(value)
    }
}

impl From<TraceError> for DriverError {
    fn from(value: TraceError) -> Self {
        Self::Trace(value)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DriverEvent {
    InboundPlayPacket(PlayClientboundPacket),
    SessionAction(SessionAction),
    IgnoredPacket {
        state: ProtocolState,
        direction: PacketDirection,
        packet_id: i32,
        packet_name: String,
        known_packet: bool,
    },
}

pub struct HeadlessDriver {
    config: HeadlessDriverConfig,
    session: HeadlessSession,
    decoder: FrameDecoder,
    outbound_frames: Vec<Vec<u8>>,
    local_pose: PlayerPose,
}

impl HeadlessDriver {
    pub fn new(config: HeadlessDriverConfig) -> Self {
        Self {
            decoder: FrameDecoder::with_limits(CompressionState::Disabled, config.frame_limits),
            config,
            session: HeadlessSession::new(),
            outbound_frames: Vec::new(),
            local_pose: PlayerPose {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw: 0.0,
                pitch: 0.0,
            },
        }
    }

    pub fn config(&self) -> &HeadlessDriverConfig {
        &self.config
    }

    pub fn session(&self) -> &HeadlessSession {
        &self.session
    }

    pub fn local_pose(&self) -> PlayerPose {
        self.local_pose
    }

    pub fn set_local_pose(&mut self, pose: PlayerPose) {
        self.local_pose = pose;
    }

    pub fn compression(&self) -> CompressionState {
        self.decoder.compression()
    }

    pub fn has_pending_outbound(&self) -> bool {
        !self.outbound_frames.is_empty()
    }

    pub fn bootstrap_login(
        &mut self,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: Option<&mut dyn TraceSink>,
    ) -> Result<(), DriverError> {
        let mut trace_sink = trace_sink;
        let handshake = self.session.begin_login(
            self.config.protocol_version,
            self.config.server_address.clone(),
            self.config.server_port,
        )?;
        self.queue_handshake_packet(&handshake, codec, &mut trace_sink)?;

        let login_start = self
            .session
            .login_start_packet(self.config.username.clone())?;
        self.queue_login_packet_inner(&login_start, codec, &mut trace_sink)?;

        Ok(())
    }

    pub fn queue_login_packet(
        &mut self,
        packet: &LoginServerboundPacket,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: Option<&mut dyn TraceSink>,
    ) -> Result<(), DriverError> {
        let mut trace_sink = trace_sink;
        self.queue_login_packet_inner(packet, codec, &mut trace_sink)
    }

    pub fn queue_play_packet(
        &mut self,
        packet: &PlayServerboundPacket,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: Option<&mut dyn TraceSink>,
    ) -> Result<(), DriverError> {
        let mut trace_sink = trace_sink;
        self.queue_play_packet_inner(packet, codec, &mut trace_sink)
    }

    pub fn drain_outbound_frames(&mut self) -> Vec<Vec<u8>> {
        mem::take(&mut self.outbound_frames)
    }

    pub fn feed_wire_bytes(
        &mut self,
        bytes: &[u8],
        codec: Option<&dyn ZlibCodec>,
        trace_sink: Option<&mut dyn TraceSink>,
    ) -> Result<Vec<DriverEvent>, DriverError> {
        let mut trace_sink = trace_sink;
        let mut events = Vec::new();
        self.decoder.queue_bytes(bytes);

        while let Some(frame) = self.decoder.try_next_frame(codec)? {
            let state = phase_to_state(self.session.snapshot().phase).ok_or(
                DriverError::UnexpectedInboundPhase(self.session.snapshot().phase),
            )?;
            let trace = build_trace_event_lossy(
                state,
                PacketDirection::Clientbound,
                frame.compression,
                &frame.packet_bytes,
            )?;
            record_trace(&mut trace_sink, trace.clone());

            let (packet_id, _) = split_packet_bytes(&frame.packet_bytes)?;

            match self.session.snapshot().phase {
                SessionPhase::Login => {
                    if !supports_login_clientbound(packet_id) {
                        events.push(ignored_event(
                            ProtocolState::Login,
                            PacketDirection::Clientbound,
                            &trace,
                        ));
                        continue;
                    }

                    let packet = LoginClientboundPacket::decode_packet(&frame.packet_bytes)?;
                    let actions = self.session.apply_login_packet(&packet)?;
                    self.apply_actions(actions, codec, &mut trace_sink, &mut events)?;
                }
                SessionPhase::Play => {
                    if !supports_play_clientbound(packet_id) {
                        events.push(ignored_event(
                            ProtocolState::Play,
                            PacketDirection::Clientbound,
                            &trace,
                        ));
                        continue;
                    }

                    let packet = PlayClientboundPacket::decode_packet(&frame.packet_bytes)?;
                    events.push(DriverEvent::InboundPlayPacket(packet.clone()));
                    let actions = self.session.apply_play_packet(&packet)?;
                    self.apply_actions(actions, codec, &mut trace_sink, &mut events)?;
                }
                phase => return Err(DriverError::UnexpectedInboundPhase(phase)),
            }
        }

        Ok(events)
    }

    fn apply_actions(
        &mut self,
        actions: Vec<SessionAction>,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: &mut Option<&mut dyn TraceSink>,
        events: &mut Vec<DriverEvent>,
    ) -> Result<(), DriverError> {
        for action in actions {
            match &action {
                SessionAction::EnableCompression { threshold } => {
                    self.decoder
                        .set_compression(CompressionState::enabled(*threshold));
                }
                SessionAction::ReplyKeepAlive { id } => {
                    let packet = HeadlessSession::keep_alive_response(*id);
                    self.queue_play_packet_inner(&packet, codec, trace_sink)?;
                }
                SessionAction::TeleportCorrectionRequired(packet) => {
                    let response = HeadlessSession::teleport_response(
                        packet,
                        self.local_pose,
                        self.config.teleport_on_ground,
                    );
                    self.queue_play_packet_inner(&response, codec, trace_sink)?;
                }
                SessionAction::JoinedGame(_) => {
                    let settings =
                        PlayServerboundPacket::ClientSettings(self.config.client_settings.clone());
                    self.queue_play_packet_inner(&settings, codec, trace_sink)?;

                    let brand = PlayServerboundPacket::CustomPayload(
                        CustomPayloadPacket::brand_payload(&self.config.client_brand)?,
                    );
                    self.queue_play_packet_inner(&brand, codec, trace_sink)?;
                }
                SessionAction::EnterPlay { .. } => {
                    self.local_pose = PlayerPose {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                        yaw: 0.0,
                        pitch: 0.0,
                    };
                }
                SessionAction::HealthUpdated(_)
                | SessionAction::Respawned(_)
                | SessionAction::EntityVelocityReceived(_)
                | SessionAction::WindowItemsUpdated(_)
                | SessionAction::TransactionConfirmed(_)
                | SessionAction::UsabilityPacket(_)
                | SessionAction::EncryptionRequested(_)
                | SessionAction::Disconnected { .. } => {}
            }

            events.push(DriverEvent::SessionAction(action));
        }

        Ok(())
    }

    fn queue_handshake_packet(
        &mut self,
        packet: &HandshakeRequest,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: &mut Option<&mut dyn TraceSink>,
    ) -> Result<(), DriverError> {
        let encoded = packet.encode_packet()?;
        self.queue_packet_bytes(
            ProtocolState::Handshake,
            PacketDirection::Serverbound,
            &encoded.packet_bytes(),
            codec,
            trace_sink,
        )
    }

    fn queue_login_packet_inner(
        &mut self,
        packet: &LoginServerboundPacket,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: &mut Option<&mut dyn TraceSink>,
    ) -> Result<(), DriverError> {
        let encoded = packet.encode_packet()?;
        self.queue_packet_bytes(
            ProtocolState::Login,
            PacketDirection::Serverbound,
            &encoded.packet_bytes(),
            codec,
            trace_sink,
        )
    }

    fn queue_play_packet_inner(
        &mut self,
        packet: &PlayServerboundPacket,
        codec: Option<&dyn ZlibCodec>,
        trace_sink: &mut Option<&mut dyn TraceSink>,
    ) -> Result<(), DriverError> {
        let encoded = packet.encode_packet()?;
        self.queue_packet_bytes(
            ProtocolState::Play,
            PacketDirection::Serverbound,
            &encoded.packet_bytes(),
            codec,
            trace_sink,
        )?;
        self.update_local_pose_from_outbound_play(packet);
        Ok(())
    }

    fn queue_packet_bytes(
        &mut self,
        state: ProtocolState,
        direction: PacketDirection,
        packet_bytes: &[u8],
        codec: Option<&dyn ZlibCodec>,
        trace_sink: &mut Option<&mut dyn TraceSink>,
    ) -> Result<(), DriverError> {
        let compression = self.decoder.compression();
        let framed = encode_frame(packet_bytes, compression, codec, self.config.frame_limits)?;

        record_trace(
            trace_sink,
            build_trace_event_lossy(
                state,
                direction,
                outbound_compression_disposition(compression, packet_bytes.len()),
                packet_bytes,
            )?,
        );

        self.outbound_frames.push(framed);
        Ok(())
    }

    fn update_local_pose_from_outbound_play(&mut self, packet: &PlayServerboundPacket) {
        match packet {
            PlayServerboundPacket::KeepAlive(_)
            | PlayServerboundPacket::ChatMessage(_)
            | PlayServerboundPacket::UseEntity(_)
            | PlayServerboundPacket::Player(_)
            | PlayServerboundPacket::PlayerDigging(_)
            | PlayServerboundPacket::PlayerBlockPlacement(_)
            | PlayServerboundPacket::HeldItemChange(_)
            | PlayServerboundPacket::Animation(_)
            | PlayServerboundPacket::EntityAction(_)
            | PlayServerboundPacket::CloseWindow(_)
            | PlayServerboundPacket::ClickWindow(_)
            | PlayServerboundPacket::ConfirmTransaction(_)
            | PlayServerboundPacket::PlayerAbilities(_)
            | PlayServerboundPacket::ClientStatus(_)
            | PlayServerboundPacket::ClientSettings(_)
            | PlayServerboundPacket::CustomPayload(_) => {}
            PlayServerboundPacket::PlayerPosition(packet) => {
                self.local_pose.x = packet.x;
                self.local_pose.y = packet.y;
                self.local_pose.z = packet.z;
            }
            PlayServerboundPacket::PlayerLook(packet) => {
                self.local_pose.yaw = packet.yaw;
                self.local_pose.pitch = packet.pitch;
            }
            PlayServerboundPacket::PlayerPositionAndLook(packet) => {
                self.local_pose.x = packet.x;
                self.local_pose.y = packet.y;
                self.local_pose.z = packet.z;
                self.local_pose.yaw = packet.yaw;
                self.local_pose.pitch = packet.pitch;
            }
        }
    }
}

fn phase_to_state(phase: SessionPhase) -> Option<ProtocolState> {
    match phase {
        SessionPhase::Handshake => Some(ProtocolState::Handshake),
        SessionPhase::Login => Some(ProtocolState::Login),
        SessionPhase::Play => Some(ProtocolState::Play),
        SessionPhase::Disconnected => None,
    }
}

fn outbound_compression_disposition(
    compression: CompressionState,
    packet_len: usize,
) -> CompressionDisposition {
    match compression {
        CompressionState::Disabled => CompressionDisposition::Disabled,
        CompressionState::Enabled { threshold } if packet_len < threshold => {
            CompressionDisposition::Uncompressed
        }
        CompressionState::Enabled { .. } => CompressionDisposition::Compressed,
    }
}

fn supports_login_clientbound(packet_id: u8) -> bool {
    matches!(packet_id, 0x00 | 0x01 | 0x02 | 0x03)
}

fn supports_play_clientbound(packet_id: u8) -> bool {
    crate::protocol::play_clientbound::PACKETS
        .iter()
        .any(|packet| packet.id == packet_id)
}

fn ignored_event(
    state: ProtocolState,
    direction: PacketDirection,
    trace: &PacketTraceEvent,
) -> DriverEvent {
    DriverEvent::IgnoredPacket {
        state,
        direction,
        packet_id: trace.packet_id,
        packet_name: trace.packet_name.clone(),
        known_packet: trace.known_packet,
    }
}

fn record_trace(trace_sink: &mut Option<&mut dyn TraceSink>, event: PacketTraceEvent) {
    if let Some(trace_sink) = trace_sink.as_mut() {
        (**trace_sink).record(event);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_implemented_clientbound_contract_reaches_the_runtime() {
        for packet in crate::protocol::play_clientbound::PACKETS {
            assert!(
                super::supports_play_clientbound(packet.id),
                "{} was silently discarded",
                packet.name
            );
        }
    }

    use super::{DriverEvent, HeadlessDriver, HeadlessDriverConfig};
    use crate::codec::login::{LoginClientboundPacket, LoginSuccess, SetCompression};
    use crate::codec::play::{
        ClientSettingsPacket, CustomPayloadPacket, KeepAlivePacket, PlayClientboundPacket,
        PlayServerboundPacket, PlayerPositionAndLookPacket, PlayerPositionLookServerboundPacket,
        PositionLookFlags,
    };
    use crate::compression::{CompressionError, ZlibCodec};
    use crate::framing::{encode_frame, FrameDecoder, FrameLimits};
    use crate::session::{PlayerPose, SessionAction};

    struct IdentityCodec;

    impl ZlibCodec for IdentityCodec {
        fn compress(&self, input: &[u8]) -> Result<Vec<u8>, CompressionError> {
            Ok(input.to_vec())
        }

        fn decompress(
            &self,
            input: &[u8],
            _expected_len: usize,
        ) -> Result<Vec<u8>, CompressionError> {
            Ok(input.to_vec())
        }
    }

    fn driver() -> HeadlessDriver {
        HeadlessDriver::new(HeadlessDriverConfig::vanilla_headless(
            47,
            "hypixel.net",
            25565,
            "Player",
        ))
    }

    fn frame_login(
        packet: &LoginClientboundPacket,
        compression: crate::compression::CompressionState,
    ) -> Vec<u8> {
        let encoded = packet.encode_packet().expect("packet should encode");
        encode_frame(
            &encoded.packet_bytes(),
            compression,
            Some(&IdentityCodec),
            FrameLimits::default(),
        )
        .expect("frame should encode")
    }

    fn frame_play(
        packet: &PlayClientboundPacket,
        compression: crate::compression::CompressionState,
    ) -> Vec<u8> {
        let encoded = packet.encode_packet().expect("packet should encode");
        encode_frame(
            &encoded.packet_bytes(),
            compression,
            Some(&IdentityCodec),
            FrameLimits::default(),
        )
        .expect("frame should encode")
    }

    #[test]
    fn bootstraps_login_frames() {
        let mut driver = driver();
        driver
            .bootstrap_login(None, None)
            .expect("bootstrap should succeed");

        let frames = driver.drain_outbound_frames();
        assert_eq!(frames.len(), 2);
    }

    #[test]
    fn enables_compression_and_enters_play() {
        let mut driver = driver();
        driver
            .bootstrap_login(None, None)
            .expect("bootstrap should succeed");

        let events = driver
            .feed_wire_bytes(
                &frame_login(
                    &LoginClientboundPacket::SetCompression(SetCompression { threshold: 1 }),
                    crate::compression::CompressionState::Disabled,
                ),
                Some(&IdentityCodec),
                None,
            )
            .expect("set compression should decode");
        assert_eq!(
            events,
            vec![DriverEvent::SessionAction(
                SessionAction::EnableCompression { threshold: 1 }
            )]
        );

        let events = driver
            .feed_wire_bytes(
                &frame_login(
                    &LoginClientboundPacket::LoginSuccess(LoginSuccess {
                        uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                        username: "Player".to_owned(),
                    }),
                    crate::compression::CompressionState::enabled(1),
                ),
                Some(&IdentityCodec),
                None,
            )
            .expect("login success should decode");

        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            DriverEvent::SessionAction(SessionAction::EnterPlay { .. })
        ));
    }

    #[test]
    fn ignores_unsupported_play_packets_and_replies_to_keepalive_and_teleport() {
        let mut driver = driver();
        driver
            .bootstrap_login(None, None)
            .expect("bootstrap should succeed");
        driver
            .feed_wire_bytes(
                &frame_login(
                    &LoginClientboundPacket::LoginSuccess(LoginSuccess {
                        uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                        username: "Player".to_owned(),
                    }),
                    crate::compression::CompressionState::Disabled,
                ),
                None,
                None,
            )
            .expect("login success should decode");
        driver.set_local_pose(PlayerPose {
            x: 10.0,
            y: 65.0,
            z: 1.0,
            yaw: 90.0,
            pitch: -5.0,
        });

        let unsupported = encode_frame(
            &[0x3F],
            crate::compression::CompressionState::Disabled,
            None,
            FrameLimits::default(),
        )
        .expect("frame should encode");
        let events = driver
            .feed_wire_bytes(&unsupported, None, None)
            .expect("unsupported packet should be ignored");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], DriverEvent::IgnoredPacket { .. }));

        let events = driver
            .feed_wire_bytes(
                &frame_play(
                    &PlayClientboundPacket::KeepAlive(KeepAlivePacket { id: 99 }),
                    crate::compression::CompressionState::Disabled,
                ),
                None,
                None,
            )
            .expect("keepalive should decode");
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            DriverEvent::InboundPlayPacket(PlayClientboundPacket::KeepAlive(KeepAlivePacket {
                id: 99
            }))
        ));
        assert!(matches!(
            &events[1],
            DriverEvent::SessionAction(SessionAction::ReplyKeepAlive { id: 99 })
        ));

        let outbound = driver.drain_outbound_frames();
        assert_eq!(outbound.len(), 3);

        let mut decoder = FrameDecoder::new(crate::compression::CompressionState::Disabled);
        decoder.queue_bytes(&outbound[2]);
        let frame = decoder
            .try_next_frame(None)
            .expect("frame should decode")
            .expect("frame should be present");
        let packet = PlayServerboundPacket::decode_packet(&frame.packet_bytes)
            .expect("packet should decode");
        assert_eq!(
            packet,
            PlayServerboundPacket::KeepAlive(KeepAlivePacket { id: 99 })
        );

        let events = driver
            .feed_wire_bytes(
                &frame_play(
                    &PlayClientboundPacket::PlayerPositionAndLook(PlayerPositionAndLookPacket {
                        x: 5.0,
                        y: 70.0,
                        z: -3.0,
                        yaw: 180.0,
                        pitch: 0.0,
                        flags: PositionLookFlags::from_bits(PositionLookFlags::Y),
                    }),
                    crate::compression::CompressionState::Disabled,
                ),
                None,
                None,
            )
            .expect("position and look should decode");
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            DriverEvent::InboundPlayPacket(PlayClientboundPacket::PlayerPositionAndLook(_))
        ));

        let outbound = driver.drain_outbound_frames();
        assert_eq!(outbound.len(), 1);

        let mut decoder = FrameDecoder::new(crate::compression::CompressionState::Disabled);
        decoder.queue_bytes(&outbound[0]);
        let frame = decoder
            .try_next_frame(None)
            .expect("frame should decode")
            .expect("frame should be present");
        let packet = PlayServerboundPacket::decode_packet(&frame.packet_bytes)
            .expect("packet should decode");
        assert_eq!(
            packet,
            PlayServerboundPacket::PlayerPositionAndLook(PlayerPositionLookServerboundPacket {
                x: 5.0,
                y: 135.0,
                z: -3.0,
                yaw: 180.0,
                pitch: 0.0,
                on_ground: false,
            })
        );
    }

    #[test]
    fn sends_client_settings_and_brand_after_join_game() {
        let mut driver = driver();
        driver
            .bootstrap_login(None, None)
            .expect("bootstrap should succeed");
        driver
            .feed_wire_bytes(
                &frame_login(
                    &LoginClientboundPacket::LoginSuccess(LoginSuccess {
                        uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                        username: "Player".to_owned(),
                    }),
                    crate::compression::CompressionState::Disabled,
                ),
                None,
                None,
            )
            .expect("login success should decode");
        driver.drain_outbound_frames();

        let events = driver
            .feed_wire_bytes(
                &frame_play(
                    &PlayClientboundPacket::JoinGame(crate::codec::play::JoinGamePacket {
                        entity_id: 7,
                        game_mode: 0,
                        hardcore: false,
                        dimension: 0,
                        difficulty: 2,
                        max_players: 20,
                        level_type: "default".to_owned(),
                        reduced_debug_info: false,
                    }),
                    crate::compression::CompressionState::Disabled,
                ),
                None,
                None,
            )
            .expect("join game should decode");
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            DriverEvent::InboundPlayPacket(PlayClientboundPacket::JoinGame(_))
        ));

        let outbound = driver.drain_outbound_frames();
        assert_eq!(outbound.len(), 2);

        let mut decoder = FrameDecoder::new(crate::compression::CompressionState::Disabled);
        decoder.queue_bytes(&outbound[0]);
        let frame = decoder
            .try_next_frame(None)
            .expect("frame should decode")
            .expect("frame should be present");
        let packet = PlayServerboundPacket::decode_packet(&frame.packet_bytes)
            .expect("packet should decode");
        assert_eq!(
            packet,
            PlayServerboundPacket::ClientSettings(ClientSettingsPacket::vanilla_headless_defaults())
        );

        let mut decoder = FrameDecoder::new(crate::compression::CompressionState::Disabled);
        decoder.queue_bytes(&outbound[1]);
        let frame = decoder
            .try_next_frame(None)
            .expect("frame should decode")
            .expect("frame should be present");
        let packet = PlayServerboundPacket::decode_packet(&frame.packet_bytes)
            .expect("packet should decode");
        assert_eq!(
            packet,
            PlayServerboundPacket::CustomPayload(
                CustomPayloadPacket::brand_payload("RustMinecraft")
                    .expect("brand payload should encode")
            )
        );
    }
}
