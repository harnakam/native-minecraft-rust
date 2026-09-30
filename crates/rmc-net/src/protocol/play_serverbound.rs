//! Frozen play-state serverbound packet contracts for protocol 47.

use super::{FieldSpec, PacketDirection, PacketSpec, ProtocolState};

const PLAY_COMPRESSION: &str =
    "Threshold-dependent after login compression is enabled. Uncompressed framing is still valid for packets below threshold.";

pub const KEEP_ALIVE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x00,
    name: "Keep Alive",
    java_class: "net.minecraft.network.play.client.C00PacketKeepAlive",
    java_handler: "INetHandlerPlayServer.processKeepAlive",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "id",
        encoding: "VarInt",
        notes: "Echo of the clientbound keep alive id.",
    }],
};

pub const CHAT_MESSAGE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x01,
    name: "Chat Message",
    java_class: "net.minecraft.network.play.client.C01PacketChatMessage",
    java_handler: "INetHandlerPlayServer.processChatMessage",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "message",
        encoding: "String(100)",
        notes: "Constructor truncates to 100 chars before send.",
    }],
};

pub const USE_ENTITY: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x02,
    name: "Use Entity",
    java_class: "net.minecraft.network.play.client.C02PacketUseEntity",
    java_handler: "INetHandlerPlayServer.processUseEntity",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Target entity id.",
        },
        FieldSpec {
            name: "action",
            encoding: "Enum VarInt",
            notes: "INTERACT, ATTACK, or INTERACT_AT.",
        },
        FieldSpec {
            name: "target_x",
            encoding: "Float (conditional)",
            notes: "Present only when action == INTERACT_AT.",
        },
        FieldSpec {
            name: "target_y",
            encoding: "Float (conditional)",
            notes: "Present only when action == INTERACT_AT.",
        },
        FieldSpec {
            name: "target_z",
            encoding: "Float (conditional)",
            notes: "Present only when action == INTERACT_AT.",
        },
    ],
};

pub const PLAYER: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x03,
    name: "Player",
    java_class: "net.minecraft.network.play.client.C03PacketPlayer",
    java_handler: "INetHandlerPlayServer.processPlayer",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "on_ground",
        encoding: "Boolean byte",
        notes: "Single byte 0 or 1, not PacketBuffer.readBoolean().",
    }],
};

pub const PLAYER_POSITION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x04,
    name: "Player Position",
    java_class: "net.minecraft.network.play.client.C03PacketPlayer$C04PacketPlayerPosition",
    java_handler: "INetHandlerPlayServer.processPlayer",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "x",
            encoding: "Double",
            notes: "Absolute player x position.",
        },
        FieldSpec {
            name: "y",
            encoding: "Double",
            notes: "Absolute bounding-box min y.",
        },
        FieldSpec {
            name: "z",
            encoding: "Double",
            notes: "Absolute player z position.",
        },
        FieldSpec {
            name: "on_ground",
            encoding: "Boolean byte",
            notes: "Single byte 0 or 1.",
        },
    ],
};

pub const PLAYER_LOOK: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x05,
    name: "Player Look",
    java_class: "net.minecraft.network.play.client.C03PacketPlayer$C05PacketPlayerLook",
    java_handler: "INetHandlerPlayServer.processPlayer",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "yaw",
            encoding: "Float",
            notes: "Absolute yaw.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "Float",
            notes: "Absolute pitch.",
        },
        FieldSpec {
            name: "on_ground",
            encoding: "Boolean byte",
            notes: "Single byte 0 or 1.",
        },
    ],
};

pub const PLAYER_POSITION_AND_LOOK: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x06,
    name: "Player Position And Look",
    java_class: "net.minecraft.network.play.client.C03PacketPlayer$C06PacketPlayerPosLook",
    java_handler: "INetHandlerPlayServer.processPlayer",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "x",
            encoding: "Double",
            notes: "Absolute player x position.",
        },
        FieldSpec {
            name: "y",
            encoding: "Double",
            notes: "Absolute bounding-box min y.",
        },
        FieldSpec {
            name: "z",
            encoding: "Double",
            notes: "Absolute player z position.",
        },
        FieldSpec {
            name: "yaw",
            encoding: "Float",
            notes: "Absolute yaw.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "Float",
            notes: "Absolute pitch.",
        },
        FieldSpec {
            name: "on_ground",
            encoding: "Boolean byte",
            notes: "Single byte 0 or 1.",
        },
    ],
};

pub const PLAYER_DIGGING: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x07,
    name: "Player Digging",
    java_class: "net.minecraft.network.play.client.C07PacketPlayerDigging",
    java_handler: "INetHandlerPlayServer.processPlayerDigging",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "status",
            encoding: "Enum VarInt",
            notes: "START_DESTROY_BLOCK through RELEASE_USE_ITEM.",
        },
        FieldSpec {
            name: "position",
            encoding: "Position",
            notes: "Packed 64-bit block position.",
        },
        FieldSpec {
            name: "face",
            encoding: "UnsignedByte",
            notes: "EnumFacing index.",
        },
    ],
};

pub const PLAYER_BLOCK_PLACEMENT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x08,
    name: "Player Block Placement",
    java_class: "net.minecraft.network.play.client.C08PacketPlayerBlockPlacement",
    java_handler: "INetHandlerPlayServer.processPlayerBlockPlacement",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "position",
            encoding: "Position",
            notes: "Packed 64-bit block position. (-1,-1,-1) with face 255 means use-item path.",
        },
        FieldSpec {
            name: "face",
            encoding: "UnsignedByte",
            notes: "Clicked face id. 255 is the special use-item sentinel.",
        },
        FieldSpec {
            name: "held_item",
            encoding: "Slot",
            notes: "Copied held item stack snapshot.",
        },
        FieldSpec {
            name: "cursor_x",
            encoding: "UnsignedByte / 16.0",
            notes: "Encoded as byte in range 0..15 then divided by 16.0 on read.",
        },
        FieldSpec {
            name: "cursor_y",
            encoding: "UnsignedByte / 16.0",
            notes: "Encoded as byte in range 0..15 then divided by 16.0 on read.",
        },
        FieldSpec {
            name: "cursor_z",
            encoding: "UnsignedByte / 16.0",
            notes: "Encoded as byte in range 0..15 then divided by 16.0 on read.",
        },
    ],
};

pub const HELD_ITEM_CHANGE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x09,
    name: "Held Item Change",
    java_class: "net.minecraft.network.play.client.C09PacketHeldItemChange",
    java_handler: "INetHandlerPlayServer.processHeldItemChange",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "slot",
        encoding: "Short",
        notes: "Hotbar slot index 0..8.",
    }],
};

pub const ANIMATION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x0A,
    name: "Animation",
    java_class: "net.minecraft.network.play.client.C0APacketAnimation",
    java_handler: "INetHandlerPlayServer.handleAnimation",
    compression: PLAY_COMPRESSION,
    fields: &[],
};

pub const ENTITY_ACTION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x0B,
    name: "Entity Action",
    java_class: "net.minecraft.network.play.client.C0BPacketEntityAction",
    java_handler: "INetHandlerPlayServer.processEntityAction",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Sending player entity id.",
        },
        FieldSpec {
            name: "action",
            encoding: "Enum VarInt",
            notes: "START_SNEAKING through OPEN_INVENTORY.",
        },
        FieldSpec {
            name: "aux_data",
            encoding: "VarInt",
            notes: "Used for RIDING_JUMP power. Zero for most actions.",
        },
    ],
};

pub const CLOSE_WINDOW: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x0D,
    name: "Close Window",
    java_class: "net.minecraft.network.play.client.C0DPacketCloseWindow",
    java_handler: "INetHandlerPlayServer.processCloseWindow",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "window_id",
        encoding: "Byte",
        notes: "Open window id to close.",
    }],
};

pub const CLICK_WINDOW: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x0E,
    name: "Click Window",
    java_class: "net.minecraft.network.play.client.C0EPacketClickWindow",
    java_handler: "INetHandlerPlayServer.processClickWindow",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "window_id",
            encoding: "Byte",
            notes: "0 = player inventory; otherwise open window id.",
        },
        FieldSpec {
            name: "slot_id",
            encoding: "Short",
            notes: "Clicked slot index.",
        },
        FieldSpec {
            name: "button",
            encoding: "Byte",
            notes: "Mouse button or hotbar button depending on mode.",
        },
        FieldSpec {
            name: "action_number",
            encoding: "Short",
            notes: "Transaction id allocated by the client container.",
        },
        FieldSpec {
            name: "mode",
            encoding: "Byte",
            notes: "Inventory interaction mode.",
        },
        FieldSpec {
            name: "clicked_item",
            encoding: "Slot",
            notes: "Client-side result snapshot sent with the click.",
        },
    ],
};

pub const CONFIRM_TRANSACTION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x0F,
    name: "Confirm Transaction",
    java_class: "net.minecraft.network.play.client.C0FPacketConfirmTransaction",
    java_handler: "INetHandlerPlayServer.processConfirmTransaction",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "window_id",
            encoding: "Byte",
            notes: "0 = inventory container; otherwise open window id.",
        },
        FieldSpec {
            name: "action_number",
            encoding: "Short",
            notes: "Transaction id being acknowledged.",
        },
        FieldSpec {
            name: "accepted",
            encoding: "Boolean byte",
            notes: "Encoded with raw byte 0 or 1 in the 1.8.9 class.",
        },
    ],
};

pub const CLIENT_SETTINGS: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x15,
    name: "Client Settings",
    java_class: "net.minecraft.network.play.client.C15PacketClientSettings",
    java_handler: "INetHandlerPlayServer.processClientSettings",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "locale",
            encoding: "String(7)",
            notes: "Language code such as en_US.",
        },
        FieldSpec {
            name: "view_distance",
            encoding: "Byte",
            notes: "Client render distance chunks.",
        },
        FieldSpec {
            name: "chat_visibility",
            encoding: "Byte",
            notes: "Enum id written as raw byte, not VarInt.",
        },
        FieldSpec {
            name: "colors_enabled",
            encoding: "Boolean",
            notes: "Chat colors enabled flag.",
        },
        FieldSpec {
            name: "displayed_skin_parts",
            encoding: "UnsignedByte bitmask",
            notes: "Model-part flags bitmask.",
        },
    ],
};

pub const CUSTOM_PAYLOAD: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x17,
    name: "Custom Payload",
    java_class: "net.minecraft.network.play.client.C17PacketCustomPayload",
    java_handler: "INetHandlerPlayServer.processVanilla250Packet",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "channel",
            encoding: "String(20)",
            notes: "Plugin or vanilla channel name, for example MC|Brand.",
        },
        FieldSpec {
            name: "data",
            encoding: "RemainingBytes(max 32767)",
            notes: "Consumes the rest of the packet body after channel decoding.",
        },
    ],
};

pub const CLIENT_STATUS: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x16,
    name: "Client Status",
    java_class: "net.minecraft.network.play.client.C16PacketClientStatus",
    java_handler: "INetHandlerPlayServer.processClientStatus",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "action",
        encoding: "VarInt",
        notes: "0 respawn; 1 statistics; 2 inventory achievement.",
    }],
};
pub const PACKETS: &[PacketSpec] = &[
    CLIENT_STATUS,
    PLAYER_ABILITIES,
    KEEP_ALIVE,
    CHAT_MESSAGE,
    USE_ENTITY,
    PLAYER,
    PLAYER_POSITION,
    PLAYER_LOOK,
    PLAYER_POSITION_AND_LOOK,
    PLAYER_DIGGING,
    PLAYER_BLOCK_PLACEMENT,
    HELD_ITEM_CHANGE,
    ANIMATION,
    ENTITY_ACTION,
    CLOSE_WINDOW,
    CLICK_WINDOW,
    CONFIRM_TRANSACTION,
    CLIENT_SETTINGS,
    CUSTOM_PAYLOAD,
];

pub const PLAYER_ABILITIES: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Serverbound,
    id: 0x13,
    name: "Player Abilities",
    java_class: "net.minecraft.network.play.client.C13PacketPlayerAbilities",
    java_handler: "INetHandlerPlayServer.processPlayerAbilities",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "flags",
            encoding: "Byte",
            notes: "Invulnerable, flying, allow-flight, creative bits.",
        },
        FieldSpec {
            name: "speeds",
            encoding: "Float x2",
            notes: "Fly speed then walk speed.",
        },
    ],
};
