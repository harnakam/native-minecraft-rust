//! Frozen play-state clientbound packet contracts for protocol 47.

use super::{FieldSpec, PacketDirection, PacketSpec, ProtocolState};

const PLAY_COMPRESSION: &str =
    "Threshold-dependent after login compression is enabled. Uncompressed framing is still valid for packets below threshold.";

pub const KEEP_ALIVE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x00,
    name: "Keep Alive",
    java_class: "net.minecraft.network.play.server.S00PacketKeepAlive",
    java_handler: "INetHandlerPlayClient.handleKeepAlive",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "id",
        encoding: "VarInt",
        notes: "Echoed back by the serverbound Keep Alive packet.",
    }],
};

pub const JOIN_GAME: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x01,
    name: "Join Game",
    java_class: "net.minecraft.network.play.server.S01PacketJoinGame",
    java_handler: "INetHandlerPlayClient.handleJoinGame",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "Int",
            notes: "Client player entity id.",
        },
        FieldSpec {
            name: "game_mode_and_hardcore",
            encoding: "UnsignedByte bitfield",
            notes: "Bit 0x08 = hardcore. Lower bits = game mode id.",
        },
        FieldSpec {
            name: "dimension",
            encoding: "Byte",
            notes: "Dimension id encoded as signed byte.",
        },
        FieldSpec {
            name: "difficulty",
            encoding: "UnsignedByte",
            notes: "EnumDifficulty id.",
        },
        FieldSpec {
            name: "max_players",
            encoding: "UnsignedByte",
            notes: "Tab-list capacity hint.",
        },
        FieldSpec {
            name: "level_type",
            encoding: "String(16)",
            notes: "World type name such as default or flat.",
        },
        FieldSpec {
            name: "reduced_debug_info",
            encoding: "Boolean",
            notes: "Toggles reduced F3 output.",
        },
    ],
};

pub const CHAT_MESSAGE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x02,
    name: "Chat Message",
    java_class: "net.minecraft.network.play.server.S02PacketChat",
    java_handler: "INetHandlerPlayClient.handleChat",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "chat_component",
            encoding: "Chat",
            notes: "JSON chat component payload.",
        },
        FieldSpec {
            name: "position",
            encoding: "Byte",
            notes: "0 = chat box, 1 = system, 2 = above hotbar.",
        },
    ],
};

pub const UPDATE_HEALTH: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x06,
    name: "Update Health",
    java_class: "net.minecraft.network.play.server.S06PacketUpdateHealth",
    java_handler: "INetHandlerPlayClient.handleUpdateHealth",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "health",
            encoding: "Float",
            notes: "Player health points.",
        },
        FieldSpec {
            name: "food_level",
            encoding: "VarInt",
            notes: "Food bar level.",
        },
        FieldSpec {
            name: "saturation",
            encoding: "Float",
            notes: "Food saturation modifier.",
        },
    ],
};

pub const RESPAWN: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x07,
    name: "Respawn",
    java_class: "net.minecraft.network.play.server.S07PacketRespawn",
    java_handler: "INetHandlerPlayClient.handleRespawn",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "dimension",
            encoding: "Int",
            notes: "Dimension id encoded as full i32.",
        },
        FieldSpec {
            name: "difficulty",
            encoding: "UnsignedByte",
            notes: "EnumDifficulty id.",
        },
        FieldSpec {
            name: "game_mode",
            encoding: "UnsignedByte",
            notes: "WorldSettings.GameType id.",
        },
        FieldSpec {
            name: "level_type",
            encoding: "String(16)",
            notes: "World type name. Null decodes to default.",
        },
    ],
};

pub const PLAYER_POSITION_AND_LOOK: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x08,
    name: "Player Position And Look",
    java_class: "net.minecraft.network.play.server.S08PacketPlayerPosLook",
    java_handler: "INetHandlerPlayClient.handlePlayerPosLook",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "x",
            encoding: "Double",
            notes: "Absolute or relative based on flags bitset.",
        },
        FieldSpec {
            name: "y",
            encoding: "Double",
            notes: "Absolute or relative based on flags bitset.",
        },
        FieldSpec {
            name: "z",
            encoding: "Double",
            notes: "Absolute or relative based on flags bitset.",
        },
        FieldSpec {
            name: "yaw",
            encoding: "Float",
            notes: "Absolute or relative based on flags bitset.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "Float",
            notes: "Absolute or relative based on flags bitset.",
        },
        FieldSpec {
            name: "flags",
            encoding: "UnsignedByte bitset",
            notes: "Bits: X, Y, Z, Y_ROT, X_ROT.",
        },
    ],
};

pub const SPAWN_PLAYER: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x0C,
    name: "Spawn Player",
    java_class: "net.minecraft.network.play.server.S0CPacketSpawnPlayer",
    java_handler: "INetHandlerPlayClient.handleSpawnPlayer",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Spawned player entity id.",
        },
        FieldSpec {
            name: "player_uuid",
            encoding: "UUID",
            notes: "Raw 16-byte UUID.",
        },
        FieldSpec {
            name: "x",
            encoding: "Int",
            notes: "Fixed-point absolute position: floor(pos * 32.0).",
        },
        FieldSpec {
            name: "y",
            encoding: "Int",
            notes: "Fixed-point absolute position: floor(pos * 32.0).",
        },
        FieldSpec {
            name: "z",
            encoding: "Int",
            notes: "Fixed-point absolute position: floor(pos * 32.0).",
        },
        FieldSpec {
            name: "yaw",
            encoding: "Byte angle",
            notes: "Rotation scaled to 0..255.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "Byte angle",
            notes: "Rotation scaled to 0..255.",
        },
        FieldSpec {
            name: "held_item",
            encoding: "Short",
            notes: "Current held item id, 0 for empty.",
        },
        FieldSpec {
            name: "metadata",
            encoding: "DataWatcher list",
            notes: "Watchable-object stream terminated by 0x7F.",
        },
    ],
};

pub const ENTITY_VELOCITY: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x12,
    name: "Entity Velocity",
    java_class: "net.minecraft.network.play.server.S12PacketEntityVelocity",
    java_handler: "INetHandlerPlayClient.handleEntityVelocity",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Target entity id.",
        },
        FieldSpec {
            name: "velocity_x",
            encoding: "Short",
            notes: "Motion clamped to +/-3.9 and scaled by 8000.",
        },
        FieldSpec {
            name: "velocity_y",
            encoding: "Short",
            notes: "Motion clamped to +/-3.9 and scaled by 8000.",
        },
        FieldSpec {
            name: "velocity_z",
            encoding: "Short",
            notes: "Motion clamped to +/-3.9 and scaled by 8000.",
        },
    ],
};

pub const CHUNK_DATA: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x21,
    name: "Chunk Data",
    java_class: "net.minecraft.network.play.server.S21PacketChunkData",
    java_handler: "INetHandlerPlayClient.handleChunkData",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "chunk_x",
            encoding: "Int",
            notes: "Chunk x coordinate.",
        },
        FieldSpec {
            name: "chunk_z",
            encoding: "Int",
            notes: "Chunk z coordinate.",
        },
        FieldSpec {
            name: "full_chunk",
            encoding: "Boolean",
            notes: "True when biome data is included.",
        },
        FieldSpec {
            name: "primary_bitmask",
            encoding: "UnsignedShort bitmask",
            notes: "Bitset for present chunk sections 0..15.",
        },
        FieldSpec {
            name: "data",
            encoding: "ByteArray(VarInt length prefix)",
            notes: "Packed section data, block light, optional sky light, optional biome array.",
        },
    ],
};

pub const MULTI_BLOCK_CHANGE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x22,
    name: "Multi Block Change",
    java_class: "net.minecraft.network.play.server.S22PacketMultiBlockChange",
    java_handler: "INetHandlerPlayClient.handleMultiBlockChange",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "chunk_x",
            encoding: "Int",
            notes: "Chunk x coordinate.",
        },
        FieldSpec {
            name: "chunk_z",
            encoding: "Int",
            notes: "Chunk z coordinate.",
        },
        FieldSpec {
            name: "record_count",
            encoding: "VarInt",
            notes: "Number of block update records.",
        },
        FieldSpec {
            name: "records",
            encoding: "Vec<BlockChangeRecord>(record_count)",
            notes: "Each record = packed_position: Short, block_state_id: VarInt.",
        },
    ],
};

pub const BLOCK_CHANGE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x23,
    name: "Block Change",
    java_class: "net.minecraft.network.play.server.S23PacketBlockChange",
    java_handler: "INetHandlerPlayClient.handleBlockChange",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "position",
            encoding: "Position",
            notes: "Packed 64-bit block position.",
        },
        FieldSpec {
            name: "block_state_id",
            encoding: "VarInt",
            notes: "Global block-state registry id.",
        },
    ],
};

pub const SET_EXPERIENCE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x1F,
    name: "Set Experience",
    java_class: "net.minecraft.network.play.server.S1FPacketSetExperience",
    java_handler: "INetHandlerPlayClient.handleSetExperience",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "progress",
            encoding: "Float",
            notes: "Experience bar progress.",
        },
        FieldSpec {
            name: "level",
            encoding: "VarInt",
            notes: "Experience level.",
        },
        FieldSpec {
            name: "total",
            encoding: "VarInt",
            notes: "Total experience.",
        },
    ],
};

pub const ENTITY_METADATA: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x1C,
    name: "Entity Metadata",
    java_class: "net.minecraft.network.play.server.S1CPacketEntityMetadata",
    java_handler: "INetHandlerPlayClient.handleEntityMetadata",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Existing entity id.",
        },
        FieldSpec {
            name: "metadata",
            encoding: "DataWatcher",
            notes: "Typed values terminated by byte 127.",
        },
    ],
};

pub const TIME_UPDATE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x03,
    name: "Time Update",
    java_class: "net.minecraft.network.play.server.S03PacketTimeUpdate",
    java_handler: "INetHandlerPlayClient.handleTimeUpdate",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "total_world_time",
            encoding: "Long",
            notes: "Signed total world ticks.",
        },
        FieldSpec {
            name: "world_time",
            encoding: "Long",
            notes: "Negative stops daylight cycling; -1 encodes frozen zero.",
        },
    ],
};

pub const ENTITY_EQUIPMENT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x04,
    name: "Entity Equipment",
    java_class: "net.minecraft.network.play.server.S04PacketEntityEquipment",
    java_handler: "INetHandlerPlayClient.handleEntityEquipment",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Existing entity id.",
        },
        FieldSpec {
            name: "slot",
            encoding: "Short",
            notes: "0 held item, 1 boots, 2 leggings, 3 chestplate, 4 helmet.",
        },
        FieldSpec {
            name: "item",
            encoding: "Slot",
            notes: "Nullable full item stack with NBT.",
        },
    ],
};

pub const WINDOW_PROPERTY: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x31,
    name: "Window Property",
    java_class: "net.minecraft.network.play.server.S31PacketWindowProperty",
    java_handler: "INetHandlerPlayClient.handleWindowProperty",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "window_id",
            encoding: "UnsignedByte",
            notes: "Current container id.",
        },
        FieldSpec {
            name: "property",
            encoding: "Short",
            notes: "Signed property index.",
        },
        FieldSpec {
            name: "value",
            encoding: "Short",
            notes: "Signed property value.",
        },
    ],
};

pub const WINDOW_ITEMS: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x30,
    name: "Window Items",
    java_class: "net.minecraft.network.play.server.S30PacketWindowItems",
    java_handler: "INetHandlerPlayClient.handleWindowItems",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "window_id",
            encoding: "UnsignedByte",
            notes: "0 = player inventory; otherwise open container id.",
        },
        FieldSpec {
            name: "slot_count",
            encoding: "Short",
            notes: "Number of following Slot entries.",
        },
        FieldSpec {
            name: "items",
            encoding: "Vec<Slot>(slot_count)",
            notes: "Each Slot uses ItemStack wire encoding from PacketBuffer.",
        },
    ],
};

pub const CONFIRM_TRANSACTION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x32,
    name: "Confirm Transaction",
    java_class: "net.minecraft.network.play.server.S32PacketConfirmTransaction",
    java_handler: "INetHandlerPlayClient.handleConfirmTransaction",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "window_id",
            encoding: "UnsignedByte",
            notes: "0 = inventory container, otherwise open container window id.",
        },
        FieldSpec {
            name: "action_number",
            encoding: "Short",
            notes: "Transaction id echoed by Click Window.",
        },
        FieldSpec {
            name: "accepted",
            encoding: "Boolean",
            notes: "False requires an explicit serverbound confirm in vanilla.",
        },
    ],
};

pub const DISCONNECT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x40,
    name: "Play Disconnect",
    java_class: "net.minecraft.network.play.server.S40PacketDisconnect",
    java_handler: "INetHandlerPlayClient.handleDisconnect",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "reason",
        encoding: "Chat",
        notes: "JSON chat component encoded as String(32767).",
    }],
};

pub const DESTROY_ENTITIES: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x13,
    name: "Destroy Entities",
    java_class: "net.minecraft.network.play.server.S13PacketDestroyEntities",
    java_handler: "INetHandlerPlayClient.handleDestroyEntities",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_count",
            encoding: "VarInt",
            notes: "Number of following entity ids.",
        },
        FieldSpec {
            name: "entity_ids",
            encoding: "Vec<VarInt>(entity_count)",
            notes: "Entities removed from the client world.",
        },
    ],
};

pub const ENTITY_RELATIVE_MOVE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x15,
    name: "Entity Relative Move",
    java_class: "net.minecraft.network.play.server.S14PacketEntity$S15PacketEntityRelMove",
    java_handler: "INetHandlerPlayClient.handleEntityMovement",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Moved entity id.",
        },
        FieldSpec {
            name: "delta_x",
            encoding: "Byte / 32.0",
            notes: "Relative fixed-point movement added to serverPosX.",
        },
        FieldSpec {
            name: "delta_y",
            encoding: "Byte / 32.0",
            notes: "Relative fixed-point movement added to serverPosY.",
        },
        FieldSpec {
            name: "delta_z",
            encoding: "Byte / 32.0",
            notes: "Relative fixed-point movement added to serverPosZ.",
        },
        FieldSpec {
            name: "on_ground",
            encoding: "Boolean",
            notes: "Server on-ground state.",
        },
    ],
};

pub const ENTITY_LOOK: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x16,
    name: "Entity Look",
    java_class: "net.minecraft.network.play.server.S14PacketEntity$S16PacketEntityLook",
    java_handler: "INetHandlerPlayClient.handleEntityMovement",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Rotated entity id.",
        },
        FieldSpec {
            name: "yaw",
            encoding: "Byte angle",
            notes: "Body yaw scaled to 0..255.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "Byte angle",
            notes: "Body pitch scaled to 0..255.",
        },
        FieldSpec {
            name: "on_ground",
            encoding: "Boolean",
            notes: "Server on-ground state.",
        },
    ],
};

pub const ENTITY_LOOK_AND_RELATIVE_MOVE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x17,
    name: "Entity Look And Relative Move",
    java_class: "net.minecraft.network.play.server.S14PacketEntity$S17PacketEntityLookMove",
    java_handler: "INetHandlerPlayClient.handleEntityMovement",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Moved entity id.",
        },
        FieldSpec {
            name: "delta_x",
            encoding: "Byte / 32.0",
            notes: "Relative fixed-point movement added to serverPosX.",
        },
        FieldSpec {
            name: "delta_y",
            encoding: "Byte / 32.0",
            notes: "Relative fixed-point movement added to serverPosY.",
        },
        FieldSpec {
            name: "delta_z",
            encoding: "Byte / 32.0",
            notes: "Relative fixed-point movement added to serverPosZ.",
        },
        FieldSpec {
            name: "yaw",
            encoding: "Byte angle",
            notes: "Body yaw scaled to 0..255.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "Byte angle",
            notes: "Body pitch scaled to 0..255.",
        },
        FieldSpec {
            name: "on_ground",
            encoding: "Boolean",
            notes: "Server on-ground state.",
        },
    ],
};

pub const ENTITY_TELEPORT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x18,
    name: "Entity Teleport",
    java_class: "net.minecraft.network.play.server.S18PacketEntityTeleport",
    java_handler: "INetHandlerPlayClient.handleEntityTeleport",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Teleported entity id.",
        },
        FieldSpec {
            name: "x",
            encoding: "Int / 32.0",
            notes: "Absolute fixed-point position.",
        },
        FieldSpec {
            name: "y",
            encoding: "Int / 32.0",
            notes: "Absolute fixed-point position.",
        },
        FieldSpec {
            name: "z",
            encoding: "Int / 32.0",
            notes: "Absolute fixed-point position.",
        },
        FieldSpec {
            name: "yaw",
            encoding: "Byte angle",
            notes: "Body yaw scaled to 0..255.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "Byte angle",
            notes: "Body pitch scaled to 0..255.",
        },
        FieldSpec {
            name: "on_ground",
            encoding: "Boolean",
            notes: "Server on-ground state.",
        },
    ],
};

pub const ENTITY_HEAD_LOOK: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x19,
    name: "Entity Head Look",
    java_class: "net.minecraft.network.play.server.S19PacketEntityHeadLook",
    java_handler: "INetHandlerPlayClient.handleEntityHeadLook",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "entity_id",
            encoding: "VarInt",
            notes: "Entity whose head yaw changed.",
        },
        FieldSpec {
            name: "head_yaw",
            encoding: "Byte angle",
            notes: "Head yaw scaled to 0..255.",
        },
    ],
};

pub const MAP_CHUNK_BULK: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x26,
    name: "Map Chunk Bulk",
    java_class: "net.minecraft.network.play.server.S26PacketMapChunkBulk",
    java_handler: "INetHandlerPlayClient.handleMapChunkBulk",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "sky_light_sent",
            encoding: "Boolean",
            notes: "True when skylight arrays are present for each chunk section payload.",
        },
        FieldSpec {
            name: "column_count",
            encoding: "VarInt",
            notes: "Number of following chunk column headers and payloads.",
        },
        FieldSpec {
            name: "columns",
            encoding: "Vec<ChunkBulkColumn>(column_count)",
            notes: "Each header = chunk_x:Int, chunk_z:Int, primary_bitmask:UnsignedShort. Payload bytes are concatenated after all headers and sized from bitcount(primary_bitmask), skylight flag, and full_chunk=true.",
        },
    ],
};

pub const SOUND_EFFECT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x29,
    name: "Sound Effect",
    java_class: "net.minecraft.network.play.server.S29PacketSoundEffect",
    java_handler: "INetHandlerPlayClient.handleSoundEffect",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "sound_name",
            encoding: "String(256)",
            notes: "Vanilla sound event name.",
        },
        FieldSpec {
            name: "effect_position_x",
            encoding: "Int / 8.0",
            notes: "Fixed-point world x position.",
        },
        FieldSpec {
            name: "effect_position_y",
            encoding: "Int / 8.0",
            notes: "Fixed-point world y position.",
        },
        FieldSpec {
            name: "effect_position_z",
            encoding: "Int / 8.0",
            notes: "Fixed-point world z position.",
        },
        FieldSpec {
            name: "volume",
            encoding: "Float",
            notes: "Playback volume multiplier.",
        },
        FieldSpec {
            name: "pitch",
            encoding: "UnsignedByte / 63.0",
            notes: "Playback pitch scalar.",
        },
    ],
};

pub const OPEN_WINDOW: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x2D,
    name: "Open Window",
    java_class: "net.minecraft.network.play.server.S2DPacketOpenWindow",
    java_handler: "INetHandlerPlayClient.handleOpenWindow",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "window_id",
            encoding: "UnsignedByte",
            notes: "Container window id.",
        },
        FieldSpec {
            name: "inventory_type",
            encoding: "String(32)",
            notes: "GUI type id such as minecraft:chest or EntityHorse.",
        },
        FieldSpec {
            name: "window_title",
            encoding: "Chat",
            notes: "JSON chat title for the GUI.",
        },
        FieldSpec {
            name: "slot_count",
            encoding: "UnsignedByte",
            notes: "Reported container slot count.",
        },
        FieldSpec {
            name: "entity_id",
            encoding: "Int (conditional)",
            notes: "Present only when inventory_type == EntityHorse.",
        },
    ],
};

pub const CLOSE_WINDOW: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x2E,
    name: "Close Window",
    java_class: "net.minecraft.network.play.server.S2EPacketCloseWindow",
    java_handler: "INetHandlerPlayClient.handleCloseWindow",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "window_id",
        encoding: "UnsignedByte",
        notes: "Window id being closed.",
    }],
};

pub const SET_SLOT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x2F,
    name: "Set Slot",
    java_class: "net.minecraft.network.play.server.S2FPacketSetSlot",
    java_handler: "INetHandlerPlayClient.handleSetSlot",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "window_id",
            encoding: "Byte",
            notes: "0 = player inventory, positive = open container, -1 = carried item, -2 = silent player-inventory update.",
        },
        FieldSpec {
            name: "slot",
            encoding: "Short",
            notes: "Target slot index.",
        },
        FieldSpec {
            name: "item",
            encoding: "Slot",
            notes: "Updated item stack snapshot.",
        },
    ],
};

pub const PLAYER_LIST_ITEM: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x38,
    name: "Player List Item",
    java_class: "net.minecraft.network.play.server.S38PacketPlayerListItem",
    java_handler: "INetHandlerPlayClient.handlePlayerListItem",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "action",
            encoding: "Enum VarInt",
            notes:
                "ADD_PLAYER, UPDATE_GAME_MODE, UPDATE_LATENCY, UPDATE_DISPLAY_NAME, REMOVE_PLAYER.",
        },
        FieldSpec {
            name: "entry_count",
            encoding: "VarInt",
            notes: "Number of following action-dependent entries.",
        },
        FieldSpec {
            name: "entries",
            encoding: "Vec<ActionSpecificPlayerListEntry>(entry_count)",
            notes: "Each entry starts with UUID and then includes fields based on action.",
        },
    ],
};

pub const SCOREBOARD_OBJECTIVE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x3B,
    name: "Scoreboard Objective",
    java_class: "net.minecraft.network.play.server.S3BPacketScoreboardObjective",
    java_handler: "INetHandlerPlayClient.handleScoreboardObjective",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "objective_name",
            encoding: "String(16)",
            notes: "Score objective internal name.",
        },
        FieldSpec {
            name: "mode",
            encoding: "Byte",
            notes: "0 = create, 1 = remove, 2 = update.",
        },
        FieldSpec {
            name: "objective_value",
            encoding: "String(32) (conditional)",
            notes: "Present when mode is create or update.",
        },
        FieldSpec {
            name: "render_type",
            encoding: "String(16) (conditional)",
            notes: "Present when mode is create or update.",
        },
    ],
};

pub const UPDATE_SCORE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x3C,
    name: "Update Score",
    java_class: "net.minecraft.network.play.server.S3CPacketUpdateScore",
    java_handler: "INetHandlerPlayClient.handleUpdateScore",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "score_name",
            encoding: "String(40)",
            notes: "Rendered entry or player name.",
        },
        FieldSpec {
            name: "action",
            encoding: "Enum VarInt",
            notes: "CHANGE or REMOVE.",
        },
        FieldSpec {
            name: "objective_name",
            encoding: "String(16)",
            notes: "Target objective name. Empty string is allowed for remove.",
        },
        FieldSpec {
            name: "value",
            encoding: "VarInt (conditional)",
            notes: "Present unless action == REMOVE.",
        },
    ],
};

pub const DISPLAY_SCOREBOARD: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x3D,
    name: "Display Scoreboard",
    java_class: "net.minecraft.network.play.server.S3DPacketDisplayScoreboard",
    java_handler: "INetHandlerPlayClient.handleDisplayScoreboard",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "position",
            encoding: "Byte",
            notes: "Sidebar, list, or below-name slot id.",
        },
        FieldSpec {
            name: "score_name",
            encoding: "String(16)",
            notes: "Objective name to display, empty string clears the slot.",
        },
    ],
};

pub const TEAMS: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x3E,
    name: "Teams",
    java_class: "net.minecraft.network.play.server.S3EPacketTeams",
    java_handler: "INetHandlerPlayClient.handleTeams",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "name",
            encoding: "String(16)",
            notes: "Registered team name.",
        },
        FieldSpec {
            name: "action",
            encoding: "Byte",
            notes: "0 = create, 1 = remove, 2 = update, 3 = add players, 4 = remove players.",
        },
        FieldSpec {
            name: "team_info",
            encoding: "CreateOrUpdateFields (conditional)",
            notes: "Display name, prefix, suffix, friendly flags, name-tag visibility, and color when action is create or update.",
        },
        FieldSpec {
            name: "players",
            encoding: "Vec<String(40)> (conditional)",
            notes: "Present when action is create, add players, or remove players.",
        },
    ],
};

pub const PLAYER_ABILITIES: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x39,
    name: "Player Abilities",
    java_class: "net.minecraft.network.play.server.S39PacketPlayerAbilities",
    java_handler: "INetHandlerPlayClient.handlePlayerAbilities",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "payload",
        encoding: "Byte, Float, Float",
        notes: "Protocol 47; verified against local MCP919 packet readers.",
    }],
};
pub const ENTITY_EFFECT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x1D,
    name: "Entity Effect",
    java_class: "net.minecraft.network.play.server.S1DPacketEntityEffect",
    java_handler: "INetHandlerPlayClient.handleEntityEffect",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "payload",
        encoding: "VarInt, Byte, Byte, VarInt, Byte",
        notes: "Protocol 47; verified against local MCP919 packet readers.",
    }],
};
pub const REMOVE_ENTITY_EFFECT: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x1E,
    name: "Remove Entity Effect",
    java_class: "net.minecraft.network.play.server.S1EPacketRemoveEntityEffect",
    java_handler: "INetHandlerPlayClient.handleRemoveEntityEffect",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "payload",
        encoding: "VarInt, Byte",
        notes: "Protocol 47; verified against local MCP919 packet readers.",
    }],
};
pub const ENTITY_PROPERTIES: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x20,
    name: "Entity Properties",
    java_class: "net.minecraft.network.play.server.S20PacketEntityProperties",
    java_handler: "INetHandlerPlayClient.handleEntityProperties",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "payload",
        encoding: "VarInt, Int, Attribute snapshots",
        notes: "Protocol 47; verified against local MCP919 packet readers.",
    }],
};
pub const HELD_ITEM_CHANGE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x09,
    name: "Held Item Change",
    java_class: "net.minecraft.network.play.server.S09PacketHeldItemChange",
    java_handler: "INetHandlerPlayClient.handleHeldItemChange",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "slot",
        encoding: "Byte",
        notes: "Hotbar index; only 0 through 8 is applied.",
    }],
};
pub const PACKETS: &[PacketSpec] = &[
    WORLD_BORDER,
    TITLE,
    PLAYER_LIST_HEADER_FOOTER,
    SERVER_DIFFICULTY,
    RESOURCE_PACK_SEND,
    CHANGE_GAME_STATE,
    EXPLOSION,
    HELD_ITEM_CHANGE,
    PLAYER_ABILITIES,
    ENTITY_EFFECT,
    REMOVE_ENTITY_EFFECT,
    ENTITY_PROPERTIES,
    KEEP_ALIVE,
    JOIN_GAME,
    CHAT_MESSAGE,
    UPDATE_HEALTH,
    RESPAWN,
    PLAYER_POSITION_AND_LOOK,
    SPAWN_PLAYER,
    ENTITY_VELOCITY,
    DESTROY_ENTITIES,
    ENTITY_RELATIVE_MOVE,
    ENTITY_LOOK,
    ENTITY_LOOK_AND_RELATIVE_MOVE,
    ENTITY_TELEPORT,
    ENTITY_HEAD_LOOK,
    CHUNK_DATA,
    MULTI_BLOCK_CHANGE,
    BLOCK_CHANGE,
    MAP_CHUNK_BULK,
    MAPS,
    STATISTICS,
    SPAWN_POSITION,
    BLOCK_BREAK_ANIMATION,
    SOUND_EFFECT,
    OPEN_WINDOW,
    CLOSE_WINDOW,
    SET_SLOT,
    WINDOW_ITEMS,
    WINDOW_PROPERTY,
    ENTITY_EQUIPMENT,
    TIME_UPDATE,
    ENTITY_METADATA,
    SET_EXPERIENCE,
    CONFIRM_TRANSACTION,
    PLAYER_LIST_ITEM,
    SCOREBOARD_OBJECTIVE,
    UPDATE_SCORE,
    DISPLAY_SCOREBOARD,
    TEAMS,
    DISCONNECT,
];

pub const CHANGE_GAME_STATE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x2B,
    name: "Change Game State",
    java_class: "net.minecraft.network.play.server.S2BPacketChangeGameState",
    java_handler: "INetHandlerPlayClient.handleChangeGameState",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "reason",
            encoding: "UnsignedByte",
            notes: "Reason 3 changes game mode.",
        },
        FieldSpec {
            name: "value",
            encoding: "Float",
            notes: "Reason-dependent value.",
        },
    ],
};

pub const EXPLOSION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x27,
    name: "Explosion",
    java_class: "net.minecraft.network.play.server.S27PacketExplosion",
    java_handler: "INetHandlerPlayClient.handleExplosion",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "position_strength",
            encoding: "Float x4",
            notes: "XYZ center and strength.",
        },
        FieldSpec {
            name: "records",
            encoding: "Int, Byte[3] x count",
            notes: "Signed offsets from truncated center coordinates.",
        },
        FieldSpec {
            name: "motion",
            encoding: "Float x3",
            notes: "Velocity added to local player motion.",
        },
    ],
};

pub const RESOURCE_PACK_SEND: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x48,
    name: "Resource Pack Send",
    java_class: "net.minecraft.network.play.server.S48PacketResourcePackSend",
    java_handler: "INetHandlerPlayClient.handleResourcePack",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "url",
            encoding: "String (32767)",
            notes: "MCP919 wire field.",
        },
        FieldSpec {
            name: "hash",
            encoding: "String (40)",
            notes: "MCP919 wire field.",
        },
    ],
};

pub const SERVER_DIFFICULTY: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x41,
    name: "Server Difficulty",
    java_class: "net.minecraft.network.play.server.S41PacketServerDifficulty",
    java_handler: "INetHandlerPlayClient.handleServerDifficulty",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "difficulty",
        encoding: "UnsignedByte",
        notes: "EnumDifficulty modulo four; no lock field on protocol 47 wire.",
    }],
};

pub const WORLD_BORDER: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x44,
    name: "World Border",
    java_class: "net.minecraft.network.play.server.S44PacketWorldBorder",
    java_handler: "INetHandlerPlayClient.handleWorldBorder",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "action and fields",
        encoding: "VarInt, action-specific Double/VarLong/VarInt",
        notes: "Six action layouts from MCP919; initialize warning distance precedes warning time.",
    }],
};

pub const TITLE: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x45,
    name: "Title",
    java_class: "net.minecraft.network.play.server.S45PacketTitle",
    java_handler: "INetHandlerPlayClient.handleTitle",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "action and fields",
        encoding: "VarInt, Chat or three Ints",
        notes: "Title/subtitle components, times, clear, reset; action-specific payload.",
    }],
};

pub const PLAYER_LIST_HEADER_FOOTER: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x47,
    name: "Player List Header Footer",
    java_class: "net.minecraft.network.play.server.S47PacketPlayerListHeaderFooter",
    java_handler: "INetHandlerPlayClient.handlePlayerListHeaderFooter",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "header, footer",
        encoding: "Chat, Chat",
        notes: "Two ordered chat components.",
    }],
};

pub const MAPS: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x34,
    name: "Maps",
    java_class: "net.minecraft.network.play.server.S34PacketMaps",
    java_handler: "INetHandlerPlayClient.handleMaps",
    compression: PLAY_COMPRESSION,
    fields: &[
        FieldSpec {
            name: "map id and scale",
            encoding: "VarInt, Byte",
            notes: "Persistent map identifier and signed scale.",
        },
        FieldSpec {
            name: "icons",
            encoding: "VarInt, packed Byte + Byte x + Byte y",
            notes: "Kind in high nibble, rotation in low nibble; replaces all icons.",
        },
        FieldSpec {
            name: "patch",
            encoding: "UnsignedByte columns; optional rows/x/y/ByteArray",
            notes: "Zero columns omits colors; row-major rectangle in the 128x128 map.",
        },
    ],
};

pub const STATISTICS: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x37,
    name: "Statistics",
    java_class: "net.minecraft.network.play.server.S37PacketStatistics",
    java_handler: "INetHandlerPlayClient.handleStatistics",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "entries",
        encoding: "VarInt count, String(32767) id, VarInt value",
        notes: "Absolute updates; unknown registry IDs ignored, duplicate IDs use the last value.",
    }],
};

pub const SPAWN_POSITION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x05,
    name: "Spawn Position",
    java_class: "net.minecraft.network.play.server.S05PacketSpawnPosition",
    java_handler: "INetHandlerPlayClient.handleSpawnPosition",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "position",
        encoding: "Position (Long)",
        notes: "Forced player spawn point and world spawn coordinates.",
    }],
};

pub const BLOCK_BREAK_ANIMATION: PacketSpec = PacketSpec {
    state: ProtocolState::Play,
    direction: PacketDirection::Clientbound,
    id: 0x25,
    name: "Block Break Animation",
    java_class: "net.minecraft.network.play.server.S25PacketBlockBreakAnim",
    java_handler: "INetHandlerPlayClient.handleBlockBreakAnim",
    compression: PLAY_COMPRESSION,
    fields: &[FieldSpec {
        name: "breaker, position, stage",
        encoding: "VarInt, Position, UnsignedByte",
        notes: "Stages 0..9 update; all other byte values remove the breaker entry.",
    }],
};
