//! Frozen protocol-contract entry points.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PacketDirection {
    Clientbound,
    Serverbound,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolState {
    Handshake,
    Login,
    Play,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FieldSpec {
    pub name: &'static str,
    pub encoding: &'static str,
    pub notes: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PacketSpec {
    pub state: ProtocolState,
    pub direction: PacketDirection,
    pub id: u8,
    pub name: &'static str,
    pub java_class: &'static str,
    pub java_handler: &'static str,
    pub compression: &'static str,
    pub fields: &'static [FieldSpec],
}

pub mod handshake;
pub mod login;
pub mod play_clientbound;
pub mod play_serverbound;

pub fn packets_for(state: ProtocolState, direction: PacketDirection) -> &'static [PacketSpec] {
    match (state, direction) {
        (ProtocolState::Handshake, PacketDirection::Serverbound) => handshake::PACKETS,
        (ProtocolState::Handshake, PacketDirection::Clientbound) => &[],
        (ProtocolState::Login, PacketDirection::Clientbound) => login::CLIENTBOUND_PACKETS,
        (ProtocolState::Login, PacketDirection::Serverbound) => login::SERVERBOUND_PACKETS,
        (ProtocolState::Play, PacketDirection::Clientbound) => play_clientbound::PACKETS,
        (ProtocolState::Play, PacketDirection::Serverbound) => play_serverbound::PACKETS,
    }
}

pub fn find_packet_spec(
    state: ProtocolState,
    direction: PacketDirection,
    packet_id: u8,
) -> Option<&'static PacketSpec> {
    packets_for(state, direction)
        .iter()
        .find(|spec| spec.id == packet_id)
}
