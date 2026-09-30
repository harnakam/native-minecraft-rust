//! Frozen login-state packet contracts for protocol 47.

use super::{FieldSpec, PacketDirection, PacketSpec, ProtocolState};

pub const CLIENTBOUND_DISCONNECT: PacketSpec = PacketSpec {
    state: ProtocolState::Login,
    direction: PacketDirection::Clientbound,
    id: 0x00,
    name: "Login Disconnect",
    java_class: "net.minecraft.network.login.server.S00PacketDisconnect",
    java_handler: "INetHandlerLoginClient.handleDisconnect",
    compression: "Uncompressed before Set Compression. Threshold framing applies only if the server already enabled compression.",
    fields: &[FieldSpec {
        name: "reason",
        encoding: "Chat",
        notes: "JSON chat component encoded as String(32767).",
    }],
};

pub const CLIENTBOUND_ENCRYPTION_REQUEST: PacketSpec = PacketSpec {
    state: ProtocolState::Login,
    direction: PacketDirection::Clientbound,
    id: 0x01,
    name: "Encryption Request",
    java_class: "net.minecraft.network.login.server.S01PacketEncryptionRequest",
    java_handler: "INetHandlerLoginClient.handleEncryptionRequest",
    compression: "Uncompressed in the normal 1.8.9 login flow.",
    fields: &[
        FieldSpec {
            name: "server_id",
            encoding: "String(20)",
            notes: "Hashed server id string used in session auth.",
        },
        FieldSpec {
            name: "public_key",
            encoding: "ByteArray(VarInt length prefix)",
            notes: "DER-encoded RSA public key bytes.",
        },
        FieldSpec {
            name: "verify_token",
            encoding: "ByteArray(VarInt length prefix)",
            notes: "Opaque server challenge bytes.",
        },
    ],
};

pub const CLIENTBOUND_LOGIN_SUCCESS: PacketSpec = PacketSpec {
    state: ProtocolState::Login,
    direction: PacketDirection::Clientbound,
    id: 0x02,
    name: "Login Success",
    java_class: "net.minecraft.network.login.server.S02PacketLoginSuccess",
    java_handler: "INetHandlerLoginClient.handleLoginSuccess",
    compression: "Compressed if sent after Set Compression. Otherwise uncompressed.",
    fields: &[
        FieldSpec {
            name: "uuid_string",
            encoding: "String(36)",
            notes: "UUID serialized as ASCII text, not as raw 16-byte UUID.",
        },
        FieldSpec {
            name: "username",
            encoding: "String(16)",
            notes: "GameProfile name.",
        },
    ],
};

pub const CLIENTBOUND_SET_COMPRESSION: PacketSpec = PacketSpec {
    state: ProtocolState::Login,
    direction: PacketDirection::Clientbound,
    id: 0x03,
    name: "Set Compression",
    java_class: "net.minecraft.network.login.server.S03PacketEnableCompression",
    java_handler: "INetHandlerLoginClient.handleEnableCompression",
    compression:
        "Never compressed itself. Enables threshold-based compressed framing for following packets.",
    fields: &[FieldSpec {
        name: "threshold",
        encoding: "VarInt",
        notes:
            "Packets with uncompressed size >= threshold use compressed framing after this point.",
    }],
};

pub const SERVERBOUND_LOGIN_START: PacketSpec = PacketSpec {
    state: ProtocolState::Login,
    direction: PacketDirection::Serverbound,
    id: 0x00,
    name: "Login Start",
    java_class: "net.minecraft.network.login.client.C00PacketLoginStart",
    java_handler: "INetHandlerLoginServer.processLoginStart",
    compression: "Never compressed in the normal login start path.",
    fields: &[FieldSpec {
        name: "username",
        encoding: "String(16)",
        notes: "GameProfile name only. UUID is not sent here in 1.8.9.",
    }],
};

pub const SERVERBOUND_ENCRYPTION_RESPONSE: PacketSpec = PacketSpec {
    state: ProtocolState::Login,
    direction: PacketDirection::Serverbound,
    id: 0x01,
    name: "Encryption Response",
    java_class: "net.minecraft.network.login.client.C01PacketEncryptionResponse",
    java_handler: "INetHandlerLoginServer.processEncryptionResponse",
    compression: "Uncompressed in the normal login flow before Set Compression is processed.",
    fields: &[
        FieldSpec {
            name: "shared_secret",
            encoding: "ByteArray(VarInt length prefix)",
            notes: "RSA-encrypted AES shared secret bytes.",
        },
        FieldSpec {
            name: "verify_token",
            encoding: "ByteArray(VarInt length prefix)",
            notes: "RSA-encrypted verify token bytes.",
        },
    ],
};

pub const CLIENTBOUND_PACKETS: &[PacketSpec] = &[
    CLIENTBOUND_DISCONNECT,
    CLIENTBOUND_ENCRYPTION_REQUEST,
    CLIENTBOUND_LOGIN_SUCCESS,
    CLIENTBOUND_SET_COMPRESSION,
];

pub const SERVERBOUND_PACKETS: &[PacketSpec] =
    &[SERVERBOUND_LOGIN_START, SERVERBOUND_ENCRYPTION_RESPONSE];

pub const PACKETS: &[PacketSpec] = &[
    CLIENTBOUND_DISCONNECT,
    CLIENTBOUND_ENCRYPTION_REQUEST,
    CLIENTBOUND_LOGIN_SUCCESS,
    CLIENTBOUND_SET_COMPRESSION,
    SERVERBOUND_LOGIN_START,
    SERVERBOUND_ENCRYPTION_RESPONSE,
];
