//! Frozen handshake-state packet contracts for protocol 47.

use super::{FieldSpec, PacketDirection, PacketSpec, ProtocolState};

pub const HANDSHAKE_REQUEST: PacketSpec = PacketSpec {
    state: ProtocolState::Handshake,
    direction: PacketDirection::Serverbound,
    id: 0x00,
    name: "Handshake Request",
    java_class: "net.minecraft.network.handshake.client.C00Handshake",
    java_handler: "INetHandlerHandshakeServer.processHandshake",
    compression: "Never compressed in the handshake state.",
    fields: &[
        FieldSpec {
            name: "protocol_version",
            encoding: "VarInt",
            notes: "47 for Minecraft 1.8.9.",
        },
        FieldSpec {
            name: "server_address",
            encoding: "String(255)",
            notes: "VarInt-prefixed UTF-8 host string.",
        },
        FieldSpec {
            name: "server_port",
            encoding: "UnsignedShort",
            notes: "Big-endian network port.",
        },
        FieldSpec {
            name: "next_state",
            encoding: "VarInt enum id",
            notes: "1 = STATUS, 2 = LOGIN. Hypixel join path uses 2.",
        },
    ],
};

pub const PACKETS: &[PacketSpec] = &[HANDSHAKE_REQUEST];
