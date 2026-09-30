//! Headless protocol session state for M1 bring-up.

use crate::codec::handshake::{HandshakeNextState, HandshakeRequest};
use crate::codec::login::{
    EncryptionRequest, LoginClientboundPacket, LoginServerboundPacket, LoginStart, LoginSuccess,
};
use crate::codec::play::{
    ConfirmTransactionClientboundPacket, EntityVelocityPacket, JoinGamePacket, KeepAlivePacket,
    PlayClientboundPacket, PlayServerboundPacket, PlayerPositionAndLookPacket,
    PlayerPositionLookServerboundPacket, PositionLookFlags, RespawnPacket, UpdateHealthPacket,
    WindowItemsPacket,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionPhase {
    Handshake,
    Login,
    Play,
    Disconnected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginProfile {
    pub uuid_string: String,
    pub username: String,
}

impl From<&LoginSuccess> for LoginProfile {
    fn from(value: &LoginSuccess) -> Self {
        Self {
            uuid_string: value.uuid_string.clone(),
            username: value.username.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionSnapshot {
    pub phase: SessionPhase,
    pub compression_threshold: Option<usize>,
    pub profile: Option<LoginProfile>,
    pub player_entity_id: Option<i32>,
    pub dimension: Option<i32>,
    pub last_server_position_and_look: Option<PlayerPositionAndLookPacket>,
    pub health: Option<f32>,
    pub food_level: Option<i32>,
    pub saturation: Option<f32>,
    pub disconnect_reason_json: Option<String>,
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            phase: SessionPhase::Handshake,
            compression_threshold: None,
            profile: None,
            player_entity_id: None,
            dimension: None,
            last_server_position_and_look: None,
            health: None,
            food_level: None,
            saturation: None,
            disconnect_reason_json: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionAction {
    EncryptionRequested(EncryptionRequest),
    EnableCompression { threshold: usize },
    EnterPlay { profile: LoginProfile },
    JoinedGame(JoinGamePacket),
    UsabilityPacket(PlayClientboundPacket),
    HealthUpdated(UpdateHealthPacket),
    Respawned(RespawnPacket),
    EntityVelocityReceived(EntityVelocityPacket),
    WindowItemsUpdated(WindowItemsPacket),
    TransactionConfirmed(ConfirmTransactionClientboundPacket),
    ReplyKeepAlive { id: i32 },
    TeleportCorrectionRequired(PlayerPositionAndLookPacket),
    Disconnected { reason_json: String },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerPose {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionError {
    InvalidPhase {
        action: &'static str,
        actual: SessionPhase,
    },
    InvalidCompressionThreshold(i32),
}

#[derive(Default)]
pub struct HeadlessSession {
    snapshot: SessionSnapshot,
}

impl HeadlessSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> &SessionSnapshot {
        &self.snapshot
    }

    pub fn begin_login(
        &mut self,
        protocol_version: i32,
        server_address: impl Into<String>,
        server_port: u16,
    ) -> Result<HandshakeRequest, SessionError> {
        self.require_phase("begin_login", SessionPhase::Handshake)?;
        self.snapshot.phase = SessionPhase::Login;

        Ok(HandshakeRequest {
            protocol_version,
            server_address: server_address.into(),
            server_port,
            next_state: HandshakeNextState::Login,
        })
    }

    pub fn login_start_packet(
        &self,
        username: impl Into<String>,
    ) -> Result<LoginServerboundPacket, SessionError> {
        self.require_phase("login_start_packet", SessionPhase::Login)?;

        Ok(LoginServerboundPacket::LoginStart(LoginStart {
            username: username.into(),
        }))
    }

    pub fn apply_login_packet(
        &mut self,
        packet: &LoginClientboundPacket,
    ) -> Result<Vec<SessionAction>, SessionError> {
        self.require_phase("apply_login_packet", SessionPhase::Login)?;

        let action = match packet {
            LoginClientboundPacket::Disconnect(packet) => {
                self.snapshot.phase = SessionPhase::Disconnected;
                self.snapshot.disconnect_reason_json = Some(packet.reason_json.clone());
                Some(SessionAction::Disconnected {
                    reason_json: packet.reason_json.clone(),
                })
            }
            LoginClientboundPacket::EncryptionRequest(packet) => {
                Some(SessionAction::EncryptionRequested(packet.clone()))
            }
            LoginClientboundPacket::LoginSuccess(packet) => {
                let profile = LoginProfile::from(packet);
                self.snapshot.phase = SessionPhase::Play;
                self.snapshot.profile = Some(profile.clone());
                Some(SessionAction::EnterPlay { profile })
            }
            LoginClientboundPacket::SetCompression(packet) => {
                let threshold = usize::try_from(packet.threshold)
                    .map_err(|_| SessionError::InvalidCompressionThreshold(packet.threshold))?;
                self.snapshot.compression_threshold = Some(threshold);
                Some(SessionAction::EnableCompression { threshold })
            }
        };

        Ok(action.into_iter().collect())
    }

    pub fn apply_play_packet(
        &mut self,
        packet: &PlayClientboundPacket,
    ) -> Result<Vec<SessionAction>, SessionError> {
        self.require_phase("apply_play_packet", SessionPhase::Play)?;

        let action = match packet {
            PlayClientboundPacket::KeepAlive(packet) => {
                Some(SessionAction::ReplyKeepAlive { id: packet.id })
            }
            PlayClientboundPacket::JoinGame(packet) => {
                self.snapshot.player_entity_id = Some(packet.entity_id);
                self.snapshot.dimension = Some(i32::from(packet.dimension));
                Some(SessionAction::JoinedGame(packet.clone()))
            }
            PlayClientboundPacket::ChatMessage(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::ChatMessage(packet.clone()),
            )),
            PlayClientboundPacket::UpdateHealth(packet) => {
                self.snapshot.health = Some(packet.health);
                self.snapshot.food_level = Some(packet.food_level);
                self.snapshot.saturation = Some(packet.saturation);
                Some(SessionAction::HealthUpdated(packet.clone()))
            }
            PlayClientboundPacket::Respawn(packet) => {
                self.snapshot.dimension = Some(packet.dimension);
                self.snapshot.last_server_position_and_look = None;
                Some(SessionAction::Respawned(packet.clone()))
            }
            PlayClientboundPacket::PlayerPositionAndLook(packet) => {
                self.snapshot.last_server_position_and_look = Some(packet.clone());
                Some(SessionAction::TeleportCorrectionRequired(packet.clone()))
            }
            PlayClientboundPacket::EntityVelocity(packet) => {
                Some(SessionAction::EntityVelocityReceived(packet.clone()))
            }
            PlayClientboundPacket::SoundEffect(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::SoundEffect(packet.clone()),
            )),
            PlayClientboundPacket::OpenWindow(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::OpenWindow(packet.clone()),
            )),
            PlayClientboundPacket::CloseWindow(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::CloseWindow(packet.clone()),
            )),
            PlayClientboundPacket::SetSlot(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::SetSlot(packet.clone()),
            )),
            PlayClientboundPacket::WindowItems(packet) => {
                Some(SessionAction::WindowItemsUpdated(packet.clone()))
            }
            PlayClientboundPacket::WindowProperty(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::WindowProperty(packet.clone()),
            )),
            PlayClientboundPacket::ConfirmTransaction(packet) => {
                Some(SessionAction::TransactionConfirmed(packet.clone()))
            }
            PlayClientboundPacket::PlayerListItem(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::PlayerListItem(packet.clone()),
            )),
            PlayClientboundPacket::ScoreboardObjective(packet) => {
                Some(SessionAction::UsabilityPacket(
                    PlayClientboundPacket::ScoreboardObjective(packet.clone()),
                ))
            }
            PlayClientboundPacket::UpdateScore(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::UpdateScore(packet.clone()),
            )),
            PlayClientboundPacket::DisplayScoreboard(packet) => {
                Some(SessionAction::UsabilityPacket(
                    PlayClientboundPacket::DisplayScoreboard(packet.clone()),
                ))
            }
            PlayClientboundPacket::Teams(packet) => Some(SessionAction::UsabilityPacket(
                PlayClientboundPacket::Teams(packet.clone()),
            )),
            PlayClientboundPacket::Disconnect(packet) => {
                self.snapshot.phase = SessionPhase::Disconnected;
                self.snapshot.disconnect_reason_json = Some(packet.reason_json.clone());
                Some(SessionAction::Disconnected {
                    reason_json: packet.reason_json.clone(),
                })
            }
            PlayClientboundPacket::PlayerAbilities(_)
            | PlayClientboundPacket::Explosion(_)
            | PlayClientboundPacket::ChangeGameState(_)
            | PlayClientboundPacket::HeldItemChange(_)
            | PlayClientboundPacket::EntityEffect(_)
            | PlayClientboundPacket::RemoveEntityEffect(_)
            | PlayClientboundPacket::EntityProperties(_)
            | PlayClientboundPacket::ChunkData(_)
            | PlayClientboundPacket::MultiBlockChange(_)
            | PlayClientboundPacket::BlockChange(_)
            | PlayClientboundPacket::MapChunkBulk(_)
            | PlayClientboundPacket::SpawnPlayer(_)
            | PlayClientboundPacket::DestroyEntities(_)
            | PlayClientboundPacket::EntityRelativeMove(_)
            | PlayClientboundPacket::EntityLook(_)
            | PlayClientboundPacket::EntityLookMove(_)
            | PlayClientboundPacket::EntityTeleport(_)
            | PlayClientboundPacket::EntityEquipment(_)
            | PlayClientboundPacket::EntityHeadLook(_) => None,
        };

        Ok(action.into_iter().collect())
    }

    pub fn keep_alive_response(id: i32) -> PlayServerboundPacket {
        PlayServerboundPacket::KeepAlive(KeepAlivePacket { id })
    }

    pub fn resolve_player_position_and_look(
        packet: &PlayerPositionAndLookPacket,
        base_pose: PlayerPose,
    ) -> PlayerPose {
        PlayerPose {
            x: if (packet.flags.bits() & PositionLookFlags::X) != 0 {
                base_pose.x + packet.x
            } else {
                packet.x
            },
            y: if (packet.flags.bits() & PositionLookFlags::Y) != 0 {
                base_pose.y + packet.y
            } else {
                packet.y
            },
            z: if (packet.flags.bits() & PositionLookFlags::Z) != 0 {
                base_pose.z + packet.z
            } else {
                packet.z
            },
            yaw: if (packet.flags.bits() & PositionLookFlags::Y_ROT) != 0 {
                base_pose.yaw + packet.yaw
            } else {
                packet.yaw
            },
            pitch: if (packet.flags.bits() & PositionLookFlags::X_ROT) != 0 {
                base_pose.pitch + packet.pitch
            } else {
                packet.pitch
            },
        }
    }

    pub fn teleport_response(
        packet: &PlayerPositionAndLookPacket,
        base_pose: PlayerPose,
        on_ground: bool,
    ) -> PlayServerboundPacket {
        let resolved = Self::resolve_player_position_and_look(packet, base_pose);

        PlayServerboundPacket::PlayerPositionAndLook(PlayerPositionLookServerboundPacket {
            x: resolved.x,
            y: resolved.y,
            z: resolved.z,
            yaw: resolved.yaw,
            pitch: resolved.pitch,
            on_ground,
        })
    }

    fn require_phase(
        &self,
        action: &'static str,
        expected: SessionPhase,
    ) -> Result<(), SessionError> {
        if self.snapshot.phase == expected {
            Ok(())
        } else {
            Err(SessionError::InvalidPhase {
                action,
                actual: self.snapshot.phase,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HeadlessSession, LoginProfile, PlayerPose, SessionAction, SessionPhase};
    use crate::codec::login::{
        EncryptionRequest, LoginClientboundPacket, LoginDisconnect, LoginSuccess, SetCompression,
    };
    use crate::codec::play::{
        ConfirmTransactionClientboundPacket, JoinGamePacket, KeepAlivePacket,
        PlayClientboundPacket, PlayDisconnectPacket, PlayerPositionAndLookPacket,
        PositionLookFlags, UpdateHealthPacket,
    };

    #[test]
    fn transitions_from_handshake_to_play() {
        let mut session = HeadlessSession::new();
        let handshake = session
            .begin_login(47, "hypixel.net", 25565)
            .expect("handshake should be created");
        assert_eq!(handshake.protocol_version, 47);
        assert_eq!(session.snapshot().phase, SessionPhase::Login);

        let login_start = session
            .login_start_packet("Player")
            .expect("login start should be available");
        assert!(matches!(
            login_start,
            crate::codec::login::LoginServerboundPacket::LoginStart(_)
        ));

        let actions = session
            .apply_login_packet(&LoginClientboundPacket::SetCompression(SetCompression {
                threshold: 256,
            }))
            .expect("compression packet should apply");
        assert_eq!(
            actions,
            vec![SessionAction::EnableCompression { threshold: 256 }]
        );

        let actions = session
            .apply_login_packet(&LoginClientboundPacket::LoginSuccess(LoginSuccess {
                uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                username: "Player".to_owned(),
            }))
            .expect("login success should apply");

        assert_eq!(
            actions,
            vec![SessionAction::EnterPlay {
                profile: LoginProfile {
                    uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                    username: "Player".to_owned(),
                },
            }]
        );
        assert_eq!(session.snapshot().phase, SessionPhase::Play);
    }

    #[test]
    fn exposes_encryption_and_play_actions() {
        let mut session = HeadlessSession::new();
        session
            .begin_login(47, "hypixel.net", 25565)
            .expect("handshake should be created");

        let actions = session
            .apply_login_packet(&LoginClientboundPacket::EncryptionRequest(
                EncryptionRequest {
                    server_id: "session".to_owned(),
                    public_key: vec![1, 2, 3],
                    verify_token: vec![4, 5, 6],
                },
            ))
            .expect("encryption request should apply");
        assert_eq!(
            actions,
            vec![SessionAction::EncryptionRequested(EncryptionRequest {
                server_id: "session".to_owned(),
                public_key: vec![1, 2, 3],
                verify_token: vec![4, 5, 6],
            })]
        );

        session
            .apply_login_packet(&LoginClientboundPacket::LoginSuccess(LoginSuccess {
                uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                username: "Player".to_owned(),
            }))
            .expect("login success should apply");

        let join_actions = session
            .apply_play_packet(&PlayClientboundPacket::JoinGame(JoinGamePacket {
                entity_id: 12,
                game_mode: 0,
                hardcore: false,
                dimension: 0,
                difficulty: 1,
                max_players: 20,
                level_type: "default".to_owned(),
                reduced_debug_info: false,
            }))
            .expect("join game should apply");
        assert_eq!(
            join_actions,
            vec![SessionAction::JoinedGame(JoinGamePacket {
                entity_id: 12,
                game_mode: 0,
                hardcore: false,
                dimension: 0,
                difficulty: 1,
                max_players: 20,
                level_type: "default".to_owned(),
                reduced_debug_info: false,
            })]
        );
        assert_eq!(session.snapshot().player_entity_id, Some(12));

        let server_position = PlayerPositionAndLookPacket {
            x: 5.0,
            y: 70.0,
            z: -3.0,
            yaw: 180.0,
            pitch: 0.0,
            flags: PositionLookFlags::from_bits(PositionLookFlags::Y),
        };
        let actions = session
            .apply_play_packet(&PlayClientboundPacket::PlayerPositionAndLook(
                server_position.clone(),
            ))
            .expect("position packet should apply");
        assert_eq!(
            actions,
            vec![SessionAction::TeleportCorrectionRequired(
                server_position.clone()
            )]
        );

        let resolved = HeadlessSession::resolve_player_position_and_look(
            &server_position,
            PlayerPose {
                x: 10.0,
                y: 65.0,
                z: 1.0,
                yaw: 90.0,
                pitch: -5.0,
            },
        );
        assert_eq!(resolved.y, 135.0);

        let response = HeadlessSession::teleport_response(
            &server_position,
            PlayerPose {
                x: 10.0,
                y: 65.0,
                z: 1.0,
                yaw: 90.0,
                pitch: -5.0,
            },
            false,
        );
        assert_eq!(
            response,
            crate::codec::play::PlayServerboundPacket::PlayerPositionAndLook(
                crate::codec::play::PlayerPositionLookServerboundPacket {
                    x: 5.0,
                    y: 135.0,
                    z: -3.0,
                    yaw: 180.0,
                    pitch: 0.0,
                    on_ground: false,
                }
            )
        );
    }

    #[test]
    fn handles_keepalive_and_disconnects() {
        let mut session = HeadlessSession::new();
        session
            .begin_login(47, "hypixel.net", 25565)
            .expect("handshake should be created");

        let actions = session
            .apply_login_packet(&LoginClientboundPacket::Disconnect(LoginDisconnect {
                reason_json: "{\"text\":\"Denied\"}".to_owned(),
            }))
            .expect("disconnect should apply");
        assert_eq!(
            actions,
            vec![SessionAction::Disconnected {
                reason_json: "{\"text\":\"Denied\"}".to_owned(),
            }]
        );
        assert_eq!(session.snapshot().phase, SessionPhase::Disconnected);

        let mut session = HeadlessSession::new();
        session
            .begin_login(47, "hypixel.net", 25565)
            .expect("handshake should be created");
        session
            .apply_login_packet(&LoginClientboundPacket::LoginSuccess(LoginSuccess {
                uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                username: "Player".to_owned(),
            }))
            .expect("login success should apply");

        let actions = session
            .apply_play_packet(&PlayClientboundPacket::KeepAlive(KeepAlivePacket {
                id: 99,
            }))
            .expect("keepalive should apply");
        assert_eq!(actions, vec![SessionAction::ReplyKeepAlive { id: 99 }]);
        assert_eq!(
            HeadlessSession::keep_alive_response(99),
            crate::codec::play::PlayServerboundPacket::KeepAlive(KeepAlivePacket { id: 99 })
        );

        let actions = session
            .apply_play_packet(&PlayClientboundPacket::Disconnect(PlayDisconnectPacket {
                reason_json: "{\"text\":\"Bye\"}".to_owned(),
            }))
            .expect("play disconnect should apply");
        assert_eq!(
            actions,
            vec![SessionAction::Disconnected {
                reason_json: "{\"text\":\"Bye\"}".to_owned(),
            }]
        );
        assert_eq!(session.snapshot().phase, SessionPhase::Disconnected);
    }

    #[test]
    fn surfaces_health_and_transaction_actions() {
        let mut session = HeadlessSession::new();
        session
            .begin_login(47, "hypixel.net", 25565)
            .expect("handshake should be created");
        session
            .apply_login_packet(&LoginClientboundPacket::LoginSuccess(LoginSuccess {
                uuid_string: "00000000-0000-0000-0000-000000000000".to_owned(),
                username: "Player".to_owned(),
            }))
            .expect("login success should apply");

        let actions = session
            .apply_play_packet(&PlayClientboundPacket::UpdateHealth(UpdateHealthPacket {
                health: 18.0,
                food_level: 20,
                saturation: 5.0,
            }))
            .expect("health update should apply");
        assert_eq!(
            actions,
            vec![SessionAction::HealthUpdated(UpdateHealthPacket {
                health: 18.0,
                food_level: 20,
                saturation: 5.0,
            })]
        );

        let actions = session
            .apply_play_packet(&PlayClientboundPacket::ConfirmTransaction(
                ConfirmTransactionClientboundPacket {
                    window_id: 0,
                    action_number: 4,
                    accepted: false,
                },
            ))
            .expect("confirm transaction should apply");
        assert_eq!(
            actions,
            vec![SessionAction::TransactionConfirmed(
                ConfirmTransactionClientboundPacket {
                    window_id: 0,
                    action_number: 4,
                    accepted: false,
                }
            )]
        );
    }
}
