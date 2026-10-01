//! Play-state codecs for the protocol 47 packet set used by headless, world, and PvP bring-up.

mod maps;
mod statistics;
mod statistics_registry;
pub use statistics::{statistic_is_achievement, StatisticsPacket};
pub mod metadata;
mod title;
pub use maps::{MapIcon, MapPatch, MapsPacket};
mod world_border;
pub use title::TitlePacket;
pub use world_border::WorldBorderPacket;

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerListHeaderFooterPacket {
    pub header_json: String,
    pub footer_json: String,
}

use crate::buffer::{BufferError, PacketReader, PacketWriter};
use crate::codec::{split_packet_bytes, CodecError, EncodedPacket};

#[derive(Clone, Debug, PartialEq)]
pub struct ResourcePackSendPacket {
    pub url: String,
    pub hash: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourcePackStatusPacket {
    pub hash: String,
    /// 0 loaded, 1 declined, 2 failed download, 3 accepted.
    pub action: i32,
}

pub const MAX_CUSTOM_PAYLOAD_BYTES: usize = 32_767;
pub const MAX_EXPLOSION_RECORDS: usize = 262_144;
pub const MAX_DESTROYED_ENTITIES: usize = 8_192;
pub const SECTION_BLOCK_COUNT: usize = 16 * 16 * 16;
pub const SECTION_DATA_BYTES: usize = SECTION_BLOCK_COUNT * 2;
pub const SECTION_LIGHT_BYTES: usize = SECTION_BLOCK_COUNT / 2;
pub const CHUNK_BIOME_BYTES: usize = 16 * 16;
pub const MAX_MULTI_BLOCK_CHANGE_RECORDS: usize = SECTION_BLOCK_COUNT;
pub const MAX_MAP_CHUNK_BULK_COLUMNS: usize = 512;
pub const MAX_WINDOW_ITEMS: usize = 512;
pub const MAX_PLAYER_LIST_ENTRIES: usize = 1024;
pub const MAX_PLAYER_PROPERTIES: usize = 64;
pub const MAX_TEAM_PLAYERS: usize = 1024;
pub const MAX_SERVERBOUND_CHAT_CHARS: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeepAlivePacket {
    pub id: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JoinGamePacket {
    pub entity_id: i32,
    pub game_mode: u8,
    pub hardcore: bool,
    pub dimension: i8,
    pub difficulty: u8,
    pub max_players: u8,
    pub level_type: String,
    pub reduced_debug_info: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UpdateHealthPacket {
    pub health: f32,
    pub food_level: i32,
    pub saturation: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerAbilitiesPacket {
    pub flags: u8,
    pub flying_speed: f32,
    pub walking_speed: f32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeldItemChangeClientboundPacket {
    pub slot: i8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ExplosionPacket {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub strength: f32,
    pub records: Vec<[i8; 3]>,
    pub motion: [f32; 3],
}
impl ExplosionPacket {
    pub fn affected_positions(&self) -> impl Iterator<Item = BlockPosition> + '_ {
        self.records.iter().map(|offset| {
            BlockPosition::new(
                (self.x as i32).wrapping_add(i32::from(offset[0])),
                (self.y as i32).wrapping_add(i32::from(offset[1])),
                (self.z as i32).wrapping_add(i32::from(offset[2])),
            )
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ChangeGameStatePacket {
    pub reason: u8,
    pub value: f32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityEffectPacket {
    pub entity_id: i32,
    pub effect_id: u8,
    pub amplifier: u8,
    pub duration: i32,
    pub hide_particles: u8,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoveEntityEffectPacket {
    pub entity_id: i32,
    pub effect_id: u8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct AttributeModifier {
    pub uuid: [u8; 16],
    pub amount: f64,
    pub operation: u8,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EntityAttribute {
    pub name: String,
    pub base: f64,
    pub modifiers: Vec<AttributeModifier>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct EntityPropertiesPacket {
    pub entity_id: i32,
    pub attributes: Vec<EntityAttribute>,
}

fn bounded_count(count: i32, maximum: usize) -> Result<usize, CodecError> {
    if count < 0 {
        return Err(CodecError::Buffer(BufferError::NegativeLength(count)));
    }
    if count as usize > maximum {
        return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
            max_len: maximum,
            actual: count as usize,
        }));
    }
    Ok(count as usize)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RespawnPacket {
    pub dimension: i32,
    pub difficulty: u8,
    pub game_mode: u8,
    pub level_type: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PositionLookFlags {
    bits: u8,
}

impl PositionLookFlags {
    pub const X: u8 = 0x01;
    pub const Y: u8 = 0x02;
    pub const Z: u8 = 0x04;
    pub const Y_ROT: u8 = 0x08;
    pub const X_ROT: u8 = 0x10;

    pub const fn from_bits(bits: u8) -> Self {
        Self { bits }
    }

    pub const fn bits(self) -> u8 {
        self.bits
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerPositionAndLookPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub flags: PositionLookFlags,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpawnPlayerPacket {
    pub entity_id: i32,
    pub player_uuid: [u8; 16],
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub held_item: i16,
    pub metadata: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DestroyEntitiesPacket {
    pub entity_ids: Vec<i32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityRelativeMovePacket {
    pub entity_id: i32,
    pub delta_x: i8,
    pub delta_y: i8,
    pub delta_z: i8,
    pub on_ground: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityLookPacket {
    pub entity_id: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityLookMovePacket {
    pub entity_id: i32,
    pub delta_x: i8,
    pub delta_y: i8,
    pub delta_z: i8,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityTeleportPacket {
    pub entity_id: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub yaw: u8,
    pub pitch: u8,
    pub on_ground: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntityHeadLookPacket {
    pub entity_id: i32,
    pub head_yaw: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayDisconnectPacket {
    pub reason_json: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct BlockPosition {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPosition {
    pub const ORIGIN: Self = Self { x: 0, y: 0, z: 0 };
    pub const USE_ITEM_SENTINEL: Self = Self {
        x: -1,
        y: -1,
        z: -1,
    };

    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    fn decode(packed: i64) -> Self {
        Self {
            x: sign_extend(((packed >> 38) & 0x3ffffff) as i32, 26),
            y: sign_extend(((packed >> 26) & 0xfff) as i32, 12),
            z: sign_extend((packed & 0x3ffffff) as i32, 26),
        }
    }

    fn encode(self) -> i64 {
        (((self.x as i64) & 0x3ffffff) << 38)
            | (((self.y as i64) & 0xfff) << 26)
            | ((self.z as i64) & 0x3ffffff)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemStack {
    pub item_id: i16,
    pub count: u8,
    pub damage: i16,
    pub nbt: Option<Vec<u8>>,
}

impl ItemStack {
    pub fn enchantment_level(&self, id: i32) -> i32 {
        let Some(bytes) = self.nbt.as_deref() else {
            return 0;
        };
        let Ok(root) = crate::nbt::parse(bytes) else {
            return 0;
        };
        let Some(list) = root.get("ench").and_then(crate::nbt::Tag::list) else {
            return 0;
        };
        for enchantment in list {
            if enchantment
                .get("id")
                .and_then(crate::nbt::Tag::short)
                .map(i32::from)
                == Some(id)
            {
                return enchantment
                    .get("lvl")
                    .and_then(crate::nbt::Tag::short)
                    .unwrap_or(0) as i32;
            }
        }
        0
    }
    pub fn tags_equal(&self, other: &Self) -> bool {
        match (&self.nbt, &other.nbt) {
            (None, None) => true,
            (Some(a), Some(b)) => {
                matches!((crate::nbt::parse(a),crate::nbt::parse(b)),(Ok(a),Ok(b)) if a==b)
            }
            _ => false,
        }
    }
    pub fn simple(item_id: i16, count: u8, damage: i16) -> Self {
        Self {
            item_id,
            count,
            damage,
            nbt: None,
        }
    }
}

pub type Slot = Option<ItemStack>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChunkDataPacket {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub full_chunk: bool,
    pub primary_bitmask: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiBlockChangeRecord {
    pub packed_position: u16,
    pub block_state_id: i32,
}

impl MultiBlockChangeRecord {
    pub fn local_x(&self) -> u8 {
        ((self.packed_position >> 12) & 0x0f) as u8
    }

    pub fn local_y(&self) -> u8 {
        (self.packed_position & 0xff) as u8
    }

    pub fn local_z(&self) -> u8 {
        ((self.packed_position >> 8) & 0x0f) as u8
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiBlockChangePacket {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub records: Vec<MultiBlockChangeRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockChangePacket {
    pub position: BlockPosition,
    pub block_state_id: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapChunkBulkColumn {
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub primary_bitmask: u16,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapChunkBulkPacket {
    pub sky_light_sent: bool,
    pub columns: Vec<MapChunkBulkColumn>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityVelocityPacket {
    pub entity_id: i32,
    pub velocity_x: i16,
    pub velocity_y: i16,
    pub velocity_z: i16,
}

impl EntityVelocityPacket {
    pub fn motion_x(&self) -> f64 {
        f64::from(self.velocity_x) / 8000.0
    }

    pub fn motion_y(&self) -> f64 {
        f64::from(self.velocity_y) / 8000.0
    }

    pub fn motion_z(&self) -> f64 {
        f64::from(self.velocity_z) / 8000.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowItemsPacket {
    pub window_id: u8,
    pub items: Vec<Slot>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowPropertyPacket {
    pub window_id: u8,
    pub property: i16,
    pub value: i16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityEquipmentPacket {
    pub entity_id: i32,
    pub slot: i16,
    pub item: Slot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimeUpdatePacket {
    pub total_world_time: i64,
    pub world_time: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityMetadataPacket {
    pub entity_id: i32,
    pub metadata: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetExperiencePacket {
    pub progress: f32,
    pub level: i32,
    pub total: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmTransactionClientboundPacket {
    pub window_id: u8,
    pub action_number: i16,
    pub accepted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessagePacket {
    pub message_json: String,
    pub position: i8,
}

impl ChatMessagePacket {
    pub fn is_chat(&self) -> bool {
        self.position == 1 || self.position == 2
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoundEffectPacket {
    pub sound_name: String,
    pub effect_position_x: i32,
    pub effect_position_y: i32,
    pub effect_position_z: i32,
    pub volume: f32,
    pub pitch: u8,
}

impl SoundEffectPacket {
    pub fn x(&self) -> f64 {
        f64::from(self.effect_position_x) / 8.0
    }

    pub fn y(&self) -> f64 {
        f64::from(self.effect_position_y) / 8.0
    }

    pub fn z(&self) -> f64 {
        f64::from(self.effect_position_z) / 8.0
    }

    pub fn pitch_value(&self) -> f32 {
        f32::from(self.pitch) / 63.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenWindowPacket {
    pub window_id: u8,
    pub inventory_type: String,
    pub window_title_json: String,
    pub slot_count: u8,
    pub entity_id: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloseWindowPacket {
    pub window_id: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetSlotPacket {
    pub window_id: i8,
    pub slot_id: i16,
    pub item: Slot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlayerListItemAction {
    AddPlayer,
    UpdateGameMode,
    UpdateLatency,
    UpdateDisplayName,
    RemovePlayer,
}

impl PlayerListItemAction {
    fn from_i32(value: i32) -> Result<Self, CodecError> {
        match value {
            0 => Ok(Self::AddPlayer),
            1 => Ok(Self::UpdateGameMode),
            2 => Ok(Self::UpdateLatency),
            3 => Ok(Self::UpdateDisplayName),
            4 => Ok(Self::RemovePlayer),
            _ => Err(CodecError::InvalidEnumValue {
                enum_name: "PlayerListItemAction",
                actual: value,
            }),
        }
    }

    fn id(self) -> i32 {
        match self {
            Self::AddPlayer => 0,
            Self::UpdateGameMode => 1,
            Self::UpdateLatency => 2,
            Self::UpdateDisplayName => 3,
            Self::RemovePlayer => 4,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerListEntry {
    pub uuid: [u8; 16],
    pub name: Option<String>,
    pub properties: Vec<PlayerProperty>,
    pub game_mode: Option<i32>,
    pub latency: Option<i32>,
    pub display_name_json: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerListItemPacket {
    pub action: PlayerListItemAction,
    pub entries: Vec<PlayerListEntry>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScoreboardObjectiveMode {
    Create,
    Remove,
    Update,
}

impl ScoreboardObjectiveMode {
    fn from_i8(value: i8) -> Result<Self, CodecError> {
        match value {
            0 => Ok(Self::Create),
            1 => Ok(Self::Remove),
            2 => Ok(Self::Update),
            _ => Err(CodecError::InvalidEnumValue {
                enum_name: "ScoreboardObjectiveMode",
                actual: i32::from(value),
            }),
        }
    }

    fn id(self) -> i8 {
        match self {
            Self::Create => 0,
            Self::Remove => 1,
            Self::Update => 2,
        }
    }

    fn includes_display_fields(self) -> bool {
        matches!(self, Self::Create | Self::Update)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScoreboardObjectivePacket {
    pub objective_name: String,
    pub mode: ScoreboardObjectiveMode,
    pub objective_value: String,
    pub render_type: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateScoreAction {
    Change,
    Remove,
}

impl UpdateScoreAction {
    fn from_i32(value: i32) -> Result<Self, CodecError> {
        match value {
            0 => Ok(Self::Change),
            1 => Ok(Self::Remove),
            _ => Err(CodecError::InvalidEnumValue {
                enum_name: "UpdateScoreAction",
                actual: value,
            }),
        }
    }

    fn id(self) -> i32 {
        match self {
            Self::Change => 0,
            Self::Remove => 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateScorePacket {
    pub score_name: String,
    pub action: UpdateScoreAction,
    pub objective_name: String,
    pub value: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DisplayScoreboardPacket {
    pub position: i8,
    pub score_name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TeamAction {
    Create,
    Remove,
    Update,
    AddPlayers,
    RemovePlayers,
}

impl TeamAction {
    fn from_i8(value: i8) -> Result<Self, CodecError> {
        match value {
            0 => Ok(Self::Create),
            1 => Ok(Self::Remove),
            2 => Ok(Self::Update),
            3 => Ok(Self::AddPlayers),
            4 => Ok(Self::RemovePlayers),
            _ => Err(CodecError::InvalidEnumValue {
                enum_name: "TeamAction",
                actual: i32::from(value),
            }),
        }
    }

    fn id(self) -> i8 {
        match self {
            Self::Create => 0,
            Self::Remove => 1,
            Self::Update => 2,
            Self::AddPlayers => 3,
            Self::RemovePlayers => 4,
        }
    }

    fn includes_team_info(self) -> bool {
        matches!(self, Self::Create | Self::Update)
    }

    fn includes_players(self) -> bool {
        matches!(self, Self::Create | Self::AddPlayers | Self::RemovePlayers)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TeamsPacket {
    pub name: String,
    pub action: TeamAction,
    pub display_name: String,
    pub prefix: String,
    pub suffix: String,
    pub friendly_flags: u8,
    pub name_tag_visibility: String,
    pub color: i8,
    pub players: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayClientboundPacket {
    KeepAlive(KeepAlivePacket),
    JoinGame(JoinGamePacket),
    ChatMessage(ChatMessagePacket),
    UpdateHealth(UpdateHealthPacket),
    Respawn(RespawnPacket),
    PlayerPositionAndLook(PlayerPositionAndLookPacket),
    SpawnPlayer(SpawnPlayerPacket),
    EntityVelocity(EntityVelocityPacket),
    PlayerAbilities(PlayerAbilitiesPacket),
    HeldItemChange(HeldItemChangeClientboundPacket),
    ChangeGameState(ChangeGameStatePacket),
    Explosion(ExplosionPacket),
    EntityEffect(EntityEffectPacket),
    RemoveEntityEffect(RemoveEntityEffectPacket),
    EntityProperties(EntityPropertiesPacket),
    DestroyEntities(DestroyEntitiesPacket),
    EntityRelativeMove(EntityRelativeMovePacket),
    EntityLook(EntityLookPacket),
    EntityLookMove(EntityLookMovePacket),
    EntityTeleport(EntityTeleportPacket),
    EntityHeadLook(EntityHeadLookPacket),
    ChunkData(ChunkDataPacket),
    MultiBlockChange(MultiBlockChangePacket),
    BlockChange(BlockChangePacket),
    MapChunkBulk(MapChunkBulkPacket),
    Maps(MapsPacket),
    Statistics(StatisticsPacket),
    SoundEffect(SoundEffectPacket),
    OpenWindow(OpenWindowPacket),
    CloseWindow(CloseWindowPacket),
    SetSlot(SetSlotPacket),
    WindowItems(WindowItemsPacket),
    WindowProperty(WindowPropertyPacket),
    EntityEquipment(EntityEquipmentPacket),
    ResourcePackSend(ResourcePackSendPacket),
    WorldBorder(WorldBorderPacket),
    Title(TitlePacket),
    PlayerListHeaderFooter(PlayerListHeaderFooterPacket),
    ServerDifficulty(u8),
    TimeUpdate(TimeUpdatePacket),
    EntityMetadata(EntityMetadataPacket),
    SetExperience(SetExperiencePacket),
    ConfirmTransaction(ConfirmTransactionClientboundPacket),
    PlayerListItem(PlayerListItemPacket),
    ScoreboardObjective(ScoreboardObjectivePacket),
    UpdateScore(UpdateScorePacket),
    DisplayScoreboard(DisplayScoreboardPacket),
    Teams(TeamsPacket),
    Disconnect(PlayDisconnectPacket),
}

impl PlayClientboundPacket {
    pub fn decode_packet(packet_bytes: &[u8]) -> Result<Self, CodecError> {
        let (packet_id, body) = split_packet_bytes(packet_bytes)?;
        Self::decode_body(packet_id, body)
    }

    pub fn decode_body(packet_id: u8, body: &[u8]) -> Result<Self, CodecError> {
        let mut reader = PacketReader::new(body);

        let packet = match packet_id {
            0x37 => Self::Statistics(StatisticsPacket::read(&mut reader)?),
            0x34 => Self::Maps(MapsPacket::read(&mut reader)?),
            0x00 => Self::KeepAlive(KeepAlivePacket {
                id: reader.read_var_i32()?,
            }),
            0x01 => {
                let entity_id = reader.read_i32()?;
                let game_mode_and_hardcore = reader.read_u8()?;
                let dimension = reader.read_i8()?;
                let difficulty = reader.read_u8()?;
                let max_players = reader.read_u8()?;
                let level_type = reader.read_string(16)?;
                let reduced_debug_info = reader.read_bool()?;

                Self::JoinGame(JoinGamePacket {
                    entity_id,
                    game_mode: game_mode_and_hardcore & 0x07,
                    hardcore: (game_mode_and_hardcore & 0x08) != 0,
                    dimension,
                    difficulty,
                    max_players,
                    level_type,
                    reduced_debug_info,
                })
            }
            0x02 => Self::ChatMessage(ChatMessagePacket {
                message_json: reader.read_chat()?,
                position: reader.read_i8()?,
            }),
            0x47 => Self::PlayerListHeaderFooter(PlayerListHeaderFooterPacket {
                header_json: reader.read_chat()?,
                footer_json: reader.read_chat()?,
            }),
            0x45 => Self::Title(TitlePacket::read(&mut reader)?),
            0x44 => Self::WorldBorder(WorldBorderPacket::read(&mut reader)?),
            0x41 => Self::ServerDifficulty(reader.read_u8()? % 4),
            0x48 => Self::ResourcePackSend(ResourcePackSendPacket {
                url: reader.read_string(32767)?,
                hash: reader.read_string(40)?,
            }),
            0x03 => Self::TimeUpdate(TimeUpdatePacket {
                total_world_time: reader.read_i64()?,
                world_time: reader.read_i64()?,
            }),
            0x04 => Self::EntityEquipment(EntityEquipmentPacket {
                entity_id: reader.read_var_i32()?,
                slot: reader.read_i16()?,
                item: read_slot(&mut reader)?,
            }),
            0x06 => Self::UpdateHealth(UpdateHealthPacket {
                health: reader.read_f32()?,
                food_level: reader.read_var_i32()?,
                saturation: reader.read_f32()?,
            }),
            0x07 => Self::Respawn(RespawnPacket {
                dimension: reader.read_i32()?,
                difficulty: reader.read_u8()?,
                game_mode: reader.read_u8()?,
                level_type: reader.read_string(16)?,
            }),
            0x08 => Self::PlayerPositionAndLook(PlayerPositionAndLookPacket {
                x: reader.read_f64()?,
                y: reader.read_f64()?,
                z: reader.read_f64()?,
                yaw: reader.read_f32()?,
                pitch: reader.read_f32()?,
                flags: PositionLookFlags::from_bits(reader.read_u8()?),
            }),
            0x0C => Self::SpawnPlayer(SpawnPlayerPacket {
                entity_id: reader.read_var_i32()?,
                player_uuid: reader.read_uuid_bytes()?,
                x: reader.read_i32()?,
                y: reader.read_i32()?,
                z: reader.read_i32()?,
                yaw: reader.read_u8()?,
                pitch: reader.read_u8()?,
                held_item: reader.read_i16()?,
                metadata: reader.read_data_watcher_blob()?,
            }),
            0x12 => Self::EntityVelocity(EntityVelocityPacket {
                entity_id: reader.read_var_i32()?,
                velocity_x: reader.read_i16()?,
                velocity_y: reader.read_i16()?,
                velocity_z: reader.read_i16()?,
            }),
            0x13 => {
                let entity_count = reader.read_var_i32()?;

                if entity_count < 0 {
                    return Err(CodecError::Buffer(BufferError::NegativeLength(
                        entity_count,
                    )));
                }

                let entity_count = entity_count as usize;

                if entity_count > MAX_DESTROYED_ENTITIES {
                    return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                        max_len: MAX_DESTROYED_ENTITIES,
                        actual: entity_count,
                    }));
                }

                let mut entity_ids = Vec::with_capacity(entity_count);

                for _ in 0..entity_count {
                    entity_ids.push(reader.read_var_i32()?);
                }

                Self::DestroyEntities(DestroyEntitiesPacket { entity_ids })
            }
            0x15 => Self::EntityRelativeMove(EntityRelativeMovePacket {
                entity_id: reader.read_var_i32()?,
                delta_x: reader.read_i8()?,
                delta_y: reader.read_i8()?,
                delta_z: reader.read_i8()?,
                on_ground: reader.read_bool()?,
            }),
            0x16 => Self::EntityLook(EntityLookPacket {
                entity_id: reader.read_var_i32()?,
                yaw: reader.read_u8()?,
                pitch: reader.read_u8()?,
                on_ground: reader.read_bool()?,
            }),
            0x17 => Self::EntityLookMove(EntityLookMovePacket {
                entity_id: reader.read_var_i32()?,
                delta_x: reader.read_i8()?,
                delta_y: reader.read_i8()?,
                delta_z: reader.read_i8()?,
                yaw: reader.read_u8()?,
                pitch: reader.read_u8()?,
                on_ground: reader.read_bool()?,
            }),
            0x18 => Self::EntityTeleport(EntityTeleportPacket {
                entity_id: reader.read_var_i32()?,
                x: reader.read_i32()?,
                y: reader.read_i32()?,
                z: reader.read_i32()?,
                yaw: reader.read_u8()?,
                pitch: reader.read_u8()?,
                on_ground: reader.read_bool()?,
            }),
            0x19 => Self::EntityHeadLook(EntityHeadLookPacket {
                entity_id: reader.read_var_i32()?,
                head_yaw: reader.read_u8()?,
            }),
            0x1D => Self::EntityEffect(EntityEffectPacket {
                entity_id: reader.read_var_i32()?,
                effect_id: reader.read_u8()?,
                amplifier: reader.read_u8()?,
                duration: reader.read_var_i32()?,
                hide_particles: reader.read_u8()?,
            }),
            0x1E => Self::RemoveEntityEffect(RemoveEntityEffectPacket {
                entity_id: reader.read_var_i32()?,
                effect_id: reader.read_u8()?,
            }),
            0x20 => {
                let entity_id = reader.read_var_i32()?;
                let count = bounded_count(reader.read_i32()?, 256)?;
                let mut attributes = Vec::with_capacity(count);
                for _ in 0..count {
                    let name = reader.read_string(64)?;
                    let base = reader.read_f64()?;
                    let count = bounded_count(reader.read_var_i32()?, 256)?;
                    let mut modifiers = Vec::with_capacity(count);
                    for _ in 0..count {
                        let uuid = reader.read_uuid_bytes()?;
                        let amount = reader.read_f64()?;
                        let operation = reader.read_u8()?;
                        if operation > 2 {
                            return Err(CodecError::InvalidEnumValue {
                                enum_name: "AttributeOperation",
                                actual: operation.into(),
                            });
                        }
                        modifiers.push(AttributeModifier {
                            uuid,
                            amount,
                            operation,
                        });
                    }
                    attributes.push(EntityAttribute {
                        name,
                        base,
                        modifiers,
                    });
                }
                Self::EntityProperties(EntityPropertiesPacket {
                    entity_id,
                    attributes,
                })
            }
            0x27 => {
                let (x, y, z, strength) = (
                    reader.read_f32()?,
                    reader.read_f32()?,
                    reader.read_f32()?,
                    reader.read_f32()?,
                );
                let count = bounded_count(reader.read_i32()?, MAX_EXPLOSION_RECORDS)?;
                let mut records = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    records.push([reader.read_i8()?, reader.read_i8()?, reader.read_i8()?]);
                }
                Self::Explosion(ExplosionPacket {
                    x,
                    y,
                    z,
                    strength,
                    records,
                    motion: [reader.read_f32()?, reader.read_f32()?, reader.read_f32()?],
                })
            }
            0x2B => Self::ChangeGameState(ChangeGameStatePacket {
                reason: reader.read_u8()?,
                value: reader.read_f32()?,
            }),
            0x09 => Self::HeldItemChange(HeldItemChangeClientboundPacket {
                slot: reader.read_i8()?,
            }),
            0x39 => Self::PlayerAbilities(PlayerAbilitiesPacket {
                flags: reader.read_u8()?,
                flying_speed: reader.read_f32()?,
                walking_speed: reader.read_f32()?,
            }),
            0x21 => Self::ChunkData(ChunkDataPacket {
                chunk_x: reader.read_i32()?,
                chunk_z: reader.read_i32()?,
                full_chunk: reader.read_bool()?,
                primary_bitmask: reader.read_u16()?,
                data: reader.read_byte_array(max_chunk_data_len(16, true, true))?,
            }),
            0x22 => {
                let chunk_x = reader.read_i32()?;
                let chunk_z = reader.read_i32()?;
                let record_count = reader.read_var_i32()?;

                if record_count < 0 {
                    return Err(CodecError::Buffer(BufferError::NegativeLength(
                        record_count,
                    )));
                }

                let record_count = record_count as usize;

                if record_count > MAX_MULTI_BLOCK_CHANGE_RECORDS {
                    return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                        max_len: MAX_MULTI_BLOCK_CHANGE_RECORDS,
                        actual: record_count,
                    }));
                }

                let mut records = Vec::with_capacity(record_count);

                for _ in 0..record_count {
                    records.push(MultiBlockChangeRecord {
                        packed_position: reader.read_u16()?,
                        block_state_id: reader.read_var_i32()?,
                    });
                }

                Self::MultiBlockChange(MultiBlockChangePacket {
                    chunk_x,
                    chunk_z,
                    records,
                })
            }
            0x23 => Self::BlockChange(BlockChangePacket {
                position: BlockPosition::decode(reader.read_i64()?),
                block_state_id: reader.read_var_i32()?,
            }),
            0x26 => {
                let sky_light_sent = reader.read_bool()?;
                let column_count = reader.read_var_i32()?;

                if column_count < 0 {
                    return Err(CodecError::Buffer(BufferError::NegativeLength(
                        column_count,
                    )));
                }

                let column_count = column_count as usize;

                if column_count > MAX_MAP_CHUNK_BULK_COLUMNS {
                    return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                        max_len: MAX_MAP_CHUNK_BULK_COLUMNS,
                        actual: column_count,
                    }));
                }

                let mut headers = Vec::with_capacity(column_count);

                for _ in 0..column_count {
                    headers.push((reader.read_i32()?, reader.read_i32()?, reader.read_u16()?));
                }

                let mut columns = Vec::with_capacity(column_count);

                for (chunk_x, chunk_z, primary_bitmask) in headers {
                    let section_count = primary_bitmask.count_ones() as usize;
                    let data_len = max_chunk_data_len(section_count, sky_light_sent, true);
                    columns.push(MapChunkBulkColumn {
                        chunk_x,
                        chunk_z,
                        primary_bitmask,
                        data: reader.read_bytes(data_len)?,
                    });
                }

                Self::MapChunkBulk(MapChunkBulkPacket {
                    sky_light_sent,
                    columns,
                })
            }
            0x29 => Self::SoundEffect(SoundEffectPacket {
                sound_name: reader.read_string(256)?,
                effect_position_x: reader.read_i32()?,
                effect_position_y: reader.read_i32()?,
                effect_position_z: reader.read_i32()?,
                volume: reader.read_f32()?,
                pitch: reader.read_u8()?,
            }),
            0x2D => {
                let window_id = reader.read_u8()?;
                let inventory_type = reader.read_string(32)?;
                let window_title_json = reader.read_chat()?;
                let slot_count = reader.read_u8()?;
                let entity_id = if inventory_type == "EntityHorse" {
                    Some(reader.read_i32()?)
                } else {
                    None
                };

                Self::OpenWindow(OpenWindowPacket {
                    window_id,
                    inventory_type,
                    window_title_json,
                    slot_count,
                    entity_id,
                })
            }
            0x2E => Self::CloseWindow(CloseWindowPacket {
                window_id: reader.read_u8()?,
            }),
            0x2F => Self::SetSlot(SetSlotPacket {
                window_id: reader.read_i8()?,
                slot_id: reader.read_i16()?,
                item: read_slot(&mut reader)?,
            }),
            0x1F => Self::SetExperience(SetExperiencePacket {
                progress: reader.read_f32()?,
                level: reader.read_var_i32()?,
                total: reader.read_var_i32()?,
            }),
            0x1C => Self::EntityMetadata(EntityMetadataPacket {
                entity_id: reader.read_var_i32()?,
                metadata: reader.read_data_watcher_blob()?,
            }),
            0x30 => {
                let window_id = reader.read_u8()?;
                let slot_count = reader.read_i16()?;

                if slot_count < 0 {
                    return Err(CodecError::Buffer(BufferError::NegativeLength(i32::from(
                        slot_count,
                    ))));
                }

                let slot_count = slot_count as usize;

                if slot_count > MAX_WINDOW_ITEMS {
                    return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                        max_len: MAX_WINDOW_ITEMS,
                        actual: slot_count,
                    }));
                }

                let mut items = Vec::with_capacity(slot_count);

                for _ in 0..slot_count {
                    items.push(read_slot(&mut reader)?);
                }

                Self::WindowItems(WindowItemsPacket { window_id, items })
            }
            0x31 => Self::WindowProperty(WindowPropertyPacket {
                window_id: reader.read_u8()?,
                property: reader.read_i16()?,
                value: reader.read_i16()?,
            }),
            0x32 => Self::ConfirmTransaction(ConfirmTransactionClientboundPacket {
                window_id: reader.read_u8()?,
                action_number: reader.read_i16()?,
                accepted: reader.read_bool()?,
            }),
            0x38 => {
                let action = PlayerListItemAction::from_i32(reader.read_var_i32()?)?;
                let entry_count = reader.read_var_i32()?;

                if entry_count < 0 {
                    return Err(CodecError::Buffer(BufferError::NegativeLength(entry_count)));
                }

                let entry_count = entry_count as usize;

                if entry_count > MAX_PLAYER_LIST_ENTRIES {
                    return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                        max_len: MAX_PLAYER_LIST_ENTRIES,
                        actual: entry_count,
                    }));
                }

                let mut entries = Vec::with_capacity(entry_count);

                for _ in 0..entry_count {
                    let uuid = reader.read_uuid_bytes()?;
                    let entry = match action {
                        PlayerListItemAction::AddPlayer => {
                            let name = reader.read_string(16)?;
                            let property_count = reader.read_var_i32()?;

                            if property_count < 0 {
                                return Err(CodecError::Buffer(BufferError::NegativeLength(
                                    property_count,
                                )));
                            }

                            let property_count = property_count as usize;

                            if property_count > MAX_PLAYER_PROPERTIES {
                                return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                                    max_len: MAX_PLAYER_PROPERTIES,
                                    actual: property_count,
                                }));
                            }

                            let mut properties = Vec::with_capacity(property_count);

                            for _ in 0..property_count {
                                properties.push(PlayerProperty {
                                    name: reader.read_string(32_767)?,
                                    value: reader.read_string(32_767)?,
                                    signature: if reader.read_bool()? {
                                        Some(reader.read_string(32_767)?)
                                    } else {
                                        None
                                    },
                                });
                            }

                            let game_mode = reader.read_var_i32()?;
                            let latency = reader.read_var_i32()?;
                            let display_name_json = if reader.read_bool()? {
                                Some(reader.read_chat()?)
                            } else {
                                None
                            };

                            PlayerListEntry {
                                uuid,
                                name: Some(name),
                                properties,
                                game_mode: Some(game_mode),
                                latency: Some(latency),
                                display_name_json,
                            }
                        }
                        PlayerListItemAction::UpdateGameMode => PlayerListEntry {
                            uuid,
                            name: None,
                            properties: Vec::new(),
                            game_mode: Some(reader.read_var_i32()?),
                            latency: None,
                            display_name_json: None,
                        },
                        PlayerListItemAction::UpdateLatency => PlayerListEntry {
                            uuid,
                            name: None,
                            properties: Vec::new(),
                            game_mode: None,
                            latency: Some(reader.read_var_i32()?),
                            display_name_json: None,
                        },
                        PlayerListItemAction::UpdateDisplayName => PlayerListEntry {
                            uuid,
                            name: None,
                            properties: Vec::new(),
                            game_mode: None,
                            latency: None,
                            display_name_json: if reader.read_bool()? {
                                Some(reader.read_chat()?)
                            } else {
                                None
                            },
                        },
                        PlayerListItemAction::RemovePlayer => PlayerListEntry {
                            uuid,
                            name: None,
                            properties: Vec::new(),
                            game_mode: None,
                            latency: None,
                            display_name_json: None,
                        },
                    };

                    entries.push(entry);
                }

                Self::PlayerListItem(PlayerListItemPacket { action, entries })
            }
            0x3B => {
                let objective_name = reader.read_string(16)?;
                let mode = ScoreboardObjectiveMode::from_i8(reader.read_i8()?)?;
                let (objective_value, render_type) = if mode.includes_display_fields() {
                    (reader.read_string(32)?, reader.read_string(16)?)
                } else {
                    (String::new(), String::new())
                };

                Self::ScoreboardObjective(ScoreboardObjectivePacket {
                    objective_name,
                    mode,
                    objective_value,
                    render_type,
                })
            }
            0x3C => {
                let score_name = reader.read_string(40)?;
                let action = UpdateScoreAction::from_i32(reader.read_var_i32()?)?;
                let objective_name = reader.read_string(16)?;
                let value = if action == UpdateScoreAction::Remove {
                    0
                } else {
                    reader.read_var_i32()?
                };

                Self::UpdateScore(UpdateScorePacket {
                    score_name,
                    action,
                    objective_name,
                    value,
                })
            }
            0x3D => Self::DisplayScoreboard(DisplayScoreboardPacket {
                position: reader.read_i8()?,
                score_name: reader.read_string(16)?,
            }),
            0x3E => {
                let name = reader.read_string(16)?;
                let action = TeamAction::from_i8(reader.read_i8()?)?;
                let (display_name, prefix, suffix, friendly_flags, name_tag_visibility, color) =
                    if action.includes_team_info() {
                        (
                            reader.read_string(32)?,
                            reader.read_string(16)?,
                            reader.read_string(16)?,
                            reader.read_u8()?,
                            reader.read_string(32)?,
                            reader.read_i8()?,
                        )
                    } else {
                        (
                            String::new(),
                            String::new(),
                            String::new(),
                            0,
                            String::new(),
                            -1,
                        )
                    };
                let players = if action.includes_players() {
                    let player_count = reader.read_var_i32()?;

                    if player_count < 0 {
                        return Err(CodecError::Buffer(BufferError::NegativeLength(
                            player_count,
                        )));
                    }

                    let player_count = player_count as usize;

                    if player_count > MAX_TEAM_PLAYERS {
                        return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                            max_len: MAX_TEAM_PLAYERS,
                            actual: player_count,
                        }));
                    }

                    let mut players = Vec::with_capacity(player_count);

                    for _ in 0..player_count {
                        players.push(reader.read_string(40)?);
                    }

                    players
                } else {
                    Vec::new()
                };

                Self::Teams(TeamsPacket {
                    name,
                    action,
                    display_name,
                    prefix,
                    suffix,
                    friendly_flags,
                    name_tag_visibility,
                    color,
                    players,
                })
            }
            0x40 => Self::Disconnect(PlayDisconnectPacket {
                reason_json: reader.read_chat()?,
            }),
            other => return Err(CodecError::UnknownPacketId(other)),
        };

        reader.finish()?;
        Ok(packet)
    }

    pub fn encode_packet(&self) -> Result<EncodedPacket, CodecError> {
        let mut writer = PacketWriter::new();

        let packet_id = match self {
            Self::Statistics(packet) => {
                packet.write(&mut writer)?;
                0x37
            }
            Self::Maps(packet) => {
                packet.write(&mut writer)?;
                0x34
            }
            Self::KeepAlive(packet) => {
                writer.write_var_i32(packet.id);
                0x00
            }
            Self::Explosion(packet) => {
                writer.write_f32(packet.x);
                writer.write_f32(packet.y);
                writer.write_f32(packet.z);
                writer.write_f32(packet.strength);
                bounded_count(packet.records.len() as i32, MAX_EXPLOSION_RECORDS)?;
                writer.write_i32(packet.records.len() as i32);
                for record in &packet.records {
                    for value in record {
                        writer.write_i8(*value);
                    }
                }
                for value in packet.motion {
                    writer.write_f32(value);
                }
                0x27
            }
            Self::ChangeGameState(packet) => {
                writer.write_u8(packet.reason);
                writer.write_f32(packet.value);
                0x2B
            }
            Self::HeldItemChange(packet) => {
                writer.write_i8(packet.slot);
                0x09
            }
            Self::PlayerAbilities(packet) => {
                writer.write_u8(packet.flags);
                writer.write_f32(packet.flying_speed);
                writer.write_f32(packet.walking_speed);
                0x39
            }
            Self::EntityEffect(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_u8(packet.effect_id);
                writer.write_u8(packet.amplifier);
                writer.write_var_i32(packet.duration);
                writer.write_u8(packet.hide_particles);
                0x1D
            }
            Self::RemoveEntityEffect(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_u8(packet.effect_id);
                0x1E
            }
            Self::EntityProperties(packet) => {
                writer.write_var_i32(packet.entity_id);
                bounded_count(packet.attributes.len() as i32, 256)?;
                writer.write_i32(packet.attributes.len() as i32);
                for attribute in &packet.attributes {
                    writer.write_string(&attribute.name, 64)?;
                    writer.write_f64(attribute.base);
                    bounded_count(attribute.modifiers.len() as i32, 256)?;
                    writer.write_var_i32(attribute.modifiers.len() as i32);
                    for modifier in &attribute.modifiers {
                        if modifier.operation > 2 {
                            return Err(CodecError::InvalidEnumValue {
                                enum_name: "AttributeOperation",
                                actual: modifier.operation.into(),
                            });
                        }
                        writer.write_uuid_bytes(&modifier.uuid);
                        writer.write_f64(modifier.amount);
                        writer.write_u8(modifier.operation);
                    }
                }
                0x20
            }
            Self::JoinGame(packet) => {
                writer.write_i32(packet.entity_id);
                let mut game_mode_and_hardcore = packet.game_mode & 0x07;

                if packet.hardcore {
                    game_mode_and_hardcore |= 0x08;
                }

                writer.write_u8(game_mode_and_hardcore);
                writer.write_i8(packet.dimension);
                writer.write_u8(packet.difficulty);
                writer.write_u8(packet.max_players);
                writer.write_string(&packet.level_type, 16)?;
                writer.write_bool(packet.reduced_debug_info);
                0x01
            }
            Self::ChatMessage(packet) => {
                writer.write_chat(&packet.message_json)?;
                writer.write_i8(packet.position);
                0x02
            }
            Self::UpdateHealth(packet) => {
                writer.write_f32(packet.health);
                writer.write_var_i32(packet.food_level);
                writer.write_f32(packet.saturation);
                0x06
            }
            Self::Respawn(packet) => {
                writer.write_i32(packet.dimension);
                writer.write_u8(packet.difficulty);
                writer.write_u8(packet.game_mode);
                writer.write_string(&packet.level_type, 16)?;
                0x07
            }
            Self::PlayerPositionAndLook(packet) => {
                writer.write_f64(packet.x);
                writer.write_f64(packet.y);
                writer.write_f64(packet.z);
                writer.write_f32(packet.yaw);
                writer.write_f32(packet.pitch);
                writer.write_u8(packet.flags.bits());
                0x08
            }
            Self::SpawnPlayer(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_uuid_bytes(&packet.player_uuid);
                writer.write_i32(packet.x);
                writer.write_i32(packet.y);
                writer.write_i32(packet.z);
                writer.write_u8(packet.yaw);
                writer.write_u8(packet.pitch);
                writer.write_i16(packet.held_item);
                writer.write_bytes(&packet.metadata);
                0x0C
            }
            Self::EntityVelocity(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_i16(packet.velocity_x);
                writer.write_i16(packet.velocity_y);
                writer.write_i16(packet.velocity_z);
                0x12
            }
            Self::DestroyEntities(packet) => {
                writer.write_var_i32(packet.entity_ids.len() as i32);

                for entity_id in &packet.entity_ids {
                    writer.write_var_i32(*entity_id);
                }

                0x13
            }
            Self::EntityRelativeMove(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_i8(packet.delta_x);
                writer.write_i8(packet.delta_y);
                writer.write_i8(packet.delta_z);
                writer.write_bool(packet.on_ground);
                0x15
            }
            Self::EntityLook(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_u8(packet.yaw);
                writer.write_u8(packet.pitch);
                writer.write_bool(packet.on_ground);
                0x16
            }
            Self::EntityLookMove(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_i8(packet.delta_x);
                writer.write_i8(packet.delta_y);
                writer.write_i8(packet.delta_z);
                writer.write_u8(packet.yaw);
                writer.write_u8(packet.pitch);
                writer.write_bool(packet.on_ground);
                0x17
            }
            Self::EntityTeleport(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_i32(packet.x);
                writer.write_i32(packet.y);
                writer.write_i32(packet.z);
                writer.write_u8(packet.yaw);
                writer.write_u8(packet.pitch);
                writer.write_bool(packet.on_ground);
                0x18
            }
            Self::EntityHeadLook(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_u8(packet.head_yaw);
                0x19
            }
            Self::ChunkData(packet) => {
                writer.write_i32(packet.chunk_x);
                writer.write_i32(packet.chunk_z);
                writer.write_bool(packet.full_chunk);
                writer.write_u16(packet.primary_bitmask);
                writer.write_byte_array(&packet.data)?;
                0x21
            }
            Self::MultiBlockChange(packet) => {
                writer.write_i32(packet.chunk_x);
                writer.write_i32(packet.chunk_z);
                writer.write_var_i32(packet.records.len() as i32);

                for record in &packet.records {
                    writer.write_u16(record.packed_position);
                    writer.write_var_i32(record.block_state_id);
                }

                0x22
            }
            Self::BlockChange(packet) => {
                writer.write_i64(packet.position.encode());
                writer.write_var_i32(packet.block_state_id);
                0x23
            }
            Self::MapChunkBulk(packet) => {
                writer.write_bool(packet.sky_light_sent);
                writer.write_var_i32(packet.columns.len() as i32);

                for column in &packet.columns {
                    writer.write_i32(column.chunk_x);
                    writer.write_i32(column.chunk_z);
                    writer.write_u16(column.primary_bitmask);
                }

                for column in &packet.columns {
                    writer.write_bytes(&column.data);
                }

                0x26
            }
            Self::SoundEffect(packet) => {
                writer.write_string(&packet.sound_name, 256)?;
                writer.write_i32(packet.effect_position_x);
                writer.write_i32(packet.effect_position_y);
                writer.write_i32(packet.effect_position_z);
                writer.write_f32(packet.volume);
                writer.write_u8(packet.pitch);
                0x29
            }
            Self::OpenWindow(packet) => {
                writer.write_u8(packet.window_id);
                writer.write_string(&packet.inventory_type, 32)?;
                writer.write_chat(&packet.window_title_json)?;
                writer.write_u8(packet.slot_count);

                if packet.inventory_type == "EntityHorse" {
                    writer.write_i32(packet.entity_id.unwrap_or_default());
                }

                0x2D
            }
            Self::CloseWindow(packet) => {
                writer.write_u8(packet.window_id);
                0x2E
            }
            Self::SetSlot(packet) => {
                writer.write_i8(packet.window_id);
                writer.write_i16(packet.slot_id);
                write_slot(&mut writer, &packet.item);
                0x2F
            }
            Self::WindowItems(packet) => {
                writer.write_u8(packet.window_id);
                writer.write_i16(packet.items.len() as i16);

                for item in &packet.items {
                    write_slot(&mut writer, item);
                }

                0x30
            }
            Self::SetExperience(packet) => {
                writer.write_f32(packet.progress);
                writer.write_var_i32(packet.level);
                writer.write_var_i32(packet.total);
                0x1F
            }
            Self::EntityMetadata(packet) => {
                metadata::decode(&packet.metadata)?;
                writer.write_var_i32(packet.entity_id);
                writer.write_bytes(&packet.metadata);
                0x1C
            }
            Self::ResourcePackSend(packet) => {
                writer.write_string(&packet.url, 32767)?;
                writer.write_string(&packet.hash, 40)?;
                0x48
            }
            Self::PlayerListHeaderFooter(packet) => {
                writer.write_chat(&packet.header_json)?;
                writer.write_chat(&packet.footer_json)?;
                0x47
            }
            Self::Title(packet) => {
                packet.write(&mut writer)?;
                0x45
            }
            Self::WorldBorder(packet) => {
                packet.write(&mut writer);
                0x44
            }
            Self::ServerDifficulty(difficulty) => {
                writer.write_u8(*difficulty % 4);
                0x41
            }
            Self::TimeUpdate(packet) => {
                writer.write_i64(packet.total_world_time);
                writer.write_i64(packet.world_time);
                0x03
            }
            Self::EntityEquipment(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_i16(packet.slot);
                write_slot(&mut writer, &packet.item);
                0x04
            }
            Self::WindowProperty(packet) => {
                writer.write_u8(packet.window_id);
                writer.write_i16(packet.property);
                writer.write_i16(packet.value);
                0x31
            }
            Self::ConfirmTransaction(packet) => {
                writer.write_u8(packet.window_id);
                writer.write_i16(packet.action_number);
                writer.write_bool(packet.accepted);
                0x32
            }
            Self::PlayerListItem(packet) => {
                writer.write_var_i32(packet.action.id());
                writer.write_var_i32(packet.entries.len() as i32);

                for entry in &packet.entries {
                    writer.write_uuid_bytes(&entry.uuid);

                    match packet.action {
                        PlayerListItemAction::AddPlayer => {
                            writer.write_string(entry.name.as_deref().unwrap_or(""), 16)?;
                            writer.write_var_i32(entry.properties.len() as i32);

                            for property in &entry.properties {
                                writer.write_string(&property.name, 32_767)?;
                                writer.write_string(&property.value, 32_767)?;

                                if let Some(signature) = &property.signature {
                                    writer.write_bool(true);
                                    writer.write_string(signature, 32_767)?;
                                } else {
                                    writer.write_bool(false);
                                }
                            }

                            writer.write_var_i32(entry.game_mode.unwrap_or_default());
                            writer.write_var_i32(entry.latency.unwrap_or_default());

                            if let Some(display_name_json) = &entry.display_name_json {
                                writer.write_bool(true);
                                writer.write_chat(display_name_json)?;
                            } else {
                                writer.write_bool(false);
                            }
                        }
                        PlayerListItemAction::UpdateGameMode => {
                            writer.write_var_i32(entry.game_mode.unwrap_or_default());
                        }
                        PlayerListItemAction::UpdateLatency => {
                            writer.write_var_i32(entry.latency.unwrap_or_default());
                        }
                        PlayerListItemAction::UpdateDisplayName => {
                            if let Some(display_name_json) = &entry.display_name_json {
                                writer.write_bool(true);
                                writer.write_chat(display_name_json)?;
                            } else {
                                writer.write_bool(false);
                            }
                        }
                        PlayerListItemAction::RemovePlayer => {}
                    }
                }

                0x38
            }
            Self::ScoreboardObjective(packet) => {
                writer.write_string(&packet.objective_name, 16)?;
                writer.write_i8(packet.mode.id());

                if packet.mode.includes_display_fields() {
                    writer.write_string(&packet.objective_value, 32)?;
                    writer.write_string(&packet.render_type, 16)?;
                }

                0x3B
            }
            Self::UpdateScore(packet) => {
                writer.write_string(&packet.score_name, 40)?;
                writer.write_var_i32(packet.action.id());
                writer.write_string(&packet.objective_name, 16)?;

                if packet.action != UpdateScoreAction::Remove {
                    writer.write_var_i32(packet.value);
                }

                0x3C
            }
            Self::DisplayScoreboard(packet) => {
                writer.write_i8(packet.position);
                writer.write_string(&packet.score_name, 16)?;
                0x3D
            }
            Self::Teams(packet) => {
                writer.write_string(&packet.name, 16)?;
                writer.write_i8(packet.action.id());

                if packet.action.includes_team_info() {
                    writer.write_string(&packet.display_name, 32)?;
                    writer.write_string(&packet.prefix, 16)?;
                    writer.write_string(&packet.suffix, 16)?;
                    writer.write_u8(packet.friendly_flags);
                    writer.write_string(&packet.name_tag_visibility, 32)?;
                    writer.write_i8(packet.color);
                }

                if packet.action.includes_players() {
                    writer.write_var_i32(packet.players.len() as i32);

                    for player in &packet.players {
                        writer.write_string(player, 40)?;
                    }
                }

                0x3E
            }
            Self::Disconnect(packet) => {
                writer.write_chat(&packet.reason_json)?;
                0x40
            }
        };

        Ok(EncodedPacket::new(packet_id, writer.into_inner()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerPacket {
    pub on_ground: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerPositionPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerLookPacket {
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerPositionLookServerboundPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub on_ground: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessageServerboundPacket {
    pub message: String,
}

impl ChatMessageServerboundPacket {
    pub fn vanilla(message: &str) -> Self {
        let mut units = 0;
        let mut truncated = String::new();
        for character in message.chars() {
            if units == MAX_SERVERBOUND_CHAT_CHARS {
                break;
            }
            if units + character.len_utf16() > MAX_SERVERBOUND_CHAT_CHARS {
                // Java substring leaves the high surrogate here. UTF-8 getBytes
                // replaces that isolated surrogate with ASCII question mark.
                truncated.push('?');
                break;
            }
            units += character.len_utf16();
            truncated.push(character);
        }
        Self { message: truncated }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UseEntityAction {
    Interact,
    Attack,
    InteractAt,
}

impl UseEntityAction {
    fn from_i32(value: i32) -> Result<Self, CodecError> {
        match value {
            0 => Ok(Self::Interact),
            1 => Ok(Self::Attack),
            2 => Ok(Self::InteractAt),
            _ => Err(CodecError::InvalidEnumValue {
                enum_name: "UseEntityAction",
                actual: value,
            }),
        }
    }

    fn id(self) -> i32 {
        match self {
            Self::Interact => 0,
            Self::Attack => 1,
            Self::InteractAt => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UseEntityPacket {
    pub entity_id: i32,
    pub action: UseEntityAction,
    pub target: Option<[f32; 3]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiggingAction {
    StartDestroyBlock,
    AbortDestroyBlock,
    StopDestroyBlock,
    DropAllItems,
    DropItem,
    ReleaseUseItem,
}

impl DiggingAction {
    fn from_i32(value: i32) -> Result<Self, CodecError> {
        match value {
            0 => Ok(Self::StartDestroyBlock),
            1 => Ok(Self::AbortDestroyBlock),
            2 => Ok(Self::StopDestroyBlock),
            3 => Ok(Self::DropAllItems),
            4 => Ok(Self::DropItem),
            5 => Ok(Self::ReleaseUseItem),
            _ => Err(CodecError::InvalidEnumValue {
                enum_name: "DiggingAction",
                actual: value,
            }),
        }
    }

    fn id(self) -> i32 {
        match self {
            Self::StartDestroyBlock => 0,
            Self::AbortDestroyBlock => 1,
            Self::StopDestroyBlock => 2,
            Self::DropAllItems => 3,
            Self::DropItem => 4,
            Self::ReleaseUseItem => 5,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayerDiggingPacket {
    pub action: DiggingAction,
    pub position: BlockPosition,
    pub face: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerBlockPlacementPacket {
    pub position: BlockPosition,
    pub face: u8,
    pub held_item: Slot,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
}

impl PlayerBlockPlacementPacket {
    pub fn use_item(held_item: Slot) -> Self {
        Self {
            position: BlockPosition::USE_ITEM_SENTINEL,
            face: 255,
            held_item,
            cursor_x: 0.0,
            cursor_y: 0.0,
            cursor_z: 0.0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeldItemChangePacket {
    pub slot: i16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnimationPacket;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntityActionKind {
    StartSneaking,
    StopSneaking,
    StopSleeping,
    StartSprinting,
    StopSprinting,
    RidingJump,
    OpenInventory,
}

impl EntityActionKind {
    fn from_i32(value: i32) -> Result<Self, CodecError> {
        match value {
            0 => Ok(Self::StartSneaking),
            1 => Ok(Self::StopSneaking),
            2 => Ok(Self::StopSleeping),
            3 => Ok(Self::StartSprinting),
            4 => Ok(Self::StopSprinting),
            5 => Ok(Self::RidingJump),
            6 => Ok(Self::OpenInventory),
            _ => Err(CodecError::InvalidEnumValue {
                enum_name: "EntityActionKind",
                actual: value,
            }),
        }
    }

    fn id(self) -> i32 {
        match self {
            Self::StartSneaking => 0,
            Self::StopSneaking => 1,
            Self::StopSleeping => 2,
            Self::StartSprinting => 3,
            Self::StopSprinting => 4,
            Self::RidingJump => 5,
            Self::OpenInventory => 6,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityActionPacket {
    pub entity_id: i32,
    pub action: EntityActionKind,
    pub aux_data: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CloseWindowServerboundPacket {
    pub window_id: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickWindowPacket {
    pub window_id: u8,
    pub slot_id: i16,
    pub button: i8,
    pub action_number: i16,
    pub mode: i8,
    pub clicked_item: Slot,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfirmTransactionServerboundPacket {
    pub window_id: u8,
    pub action_number: i16,
    pub accepted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClientSettingsPacket {
    pub locale: String,
    pub view_distance: i8,
    pub chat_visibility: u8,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
}

impl ClientSettingsPacket {
    pub fn vanilla_headless_defaults() -> Self {
        Self {
            locale: "en_US".to_owned(),
            view_distance: 8,
            chat_visibility: 0,
            chat_colors: true,
            displayed_skin_parts: 0x7f,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustomPayloadPacket {
    pub channel: String,
    pub data: Vec<u8>,
}

impl CustomPayloadPacket {
    pub fn brand_payload(brand: &str) -> Result<Self, CodecError> {
        let mut writer = PacketWriter::new();
        writer.write_string(brand, 32_767)?;

        Ok(Self {
            channel: "MC|Brand".to_owned(),
            data: writer.into_inner(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayServerboundPacket {
    KeepAlive(KeepAlivePacket),
    ChatMessage(ChatMessageServerboundPacket),
    UseEntity(UseEntityPacket),
    Player(PlayerPacket),
    PlayerPosition(PlayerPositionPacket),
    PlayerLook(PlayerLookPacket),
    PlayerPositionAndLook(PlayerPositionLookServerboundPacket),
    PlayerDigging(PlayerDiggingPacket),
    PlayerBlockPlacement(PlayerBlockPlacementPacket),
    HeldItemChange(HeldItemChangePacket),
    Animation(AnimationPacket),
    EntityAction(EntityActionPacket),
    CloseWindow(CloseWindowServerboundPacket),
    ClickWindow(ClickWindowPacket),
    ConfirmTransaction(ConfirmTransactionServerboundPacket),
    ClientStatus(i32),
    ResourcePackStatus(ResourcePackStatusPacket),
    PlayerAbilities(PlayerAbilitiesPacket),
    ClientSettings(ClientSettingsPacket),
    CustomPayload(CustomPayloadPacket),
}

impl PlayServerboundPacket {
    pub fn decode_packet(packet_bytes: &[u8]) -> Result<Self, CodecError> {
        let (packet_id, body) = split_packet_bytes(packet_bytes)?;
        Self::decode_body(packet_id, body)
    }

    pub fn decode_body(packet_id: u8, body: &[u8]) -> Result<Self, CodecError> {
        let mut reader = PacketReader::new(body);

        let packet = match packet_id {
            0x19 => {
                let hash = reader.read_string(40)?;
                let action = reader.read_var_i32()?;
                if !(0..=3).contains(&action) {
                    return Err(CodecError::InvalidEnumValue {
                        enum_name: "ResourcePackStatus",
                        actual: action,
                    });
                }
                Self::ResourcePackStatus(ResourcePackStatusPacket { hash, action })
            }
            0x13 => Self::PlayerAbilities(PlayerAbilitiesPacket {
                flags: reader.read_u8()?,
                flying_speed: reader.read_f32()?,
                walking_speed: reader.read_f32()?,
            }),
            0x16 => {
                let action = reader.read_var_i32()?;
                if !(0..=2).contains(&action) {
                    return Err(CodecError::InvalidEnumValue {
                        enum_name: "ClientStatus",
                        actual: action,
                    });
                }
                Self::ClientStatus(action)
            }
            0x00 => Self::KeepAlive(KeepAlivePacket {
                id: reader.read_var_i32()?,
            }),
            0x01 => Self::ChatMessage(ChatMessageServerboundPacket {
                message: reader.read_string(MAX_SERVERBOUND_CHAT_CHARS)?,
            }),
            0x02 => {
                let entity_id = reader.read_var_i32()?;
                let action = UseEntityAction::from_i32(reader.read_var_i32()?)?;
                let target = if action == UseEntityAction::InteractAt {
                    Some([reader.read_f32()?, reader.read_f32()?, reader.read_f32()?])
                } else {
                    None
                };

                Self::UseEntity(UseEntityPacket {
                    entity_id,
                    action,
                    target,
                })
            }
            0x03 => Self::Player(PlayerPacket {
                on_ground: reader.read_bool()?,
            }),
            0x04 => Self::PlayerPosition(PlayerPositionPacket {
                x: reader.read_f64()?,
                y: reader.read_f64()?,
                z: reader.read_f64()?,
                on_ground: reader.read_bool()?,
            }),
            0x05 => Self::PlayerLook(PlayerLookPacket {
                yaw: reader.read_f32()?,
                pitch: reader.read_f32()?,
                on_ground: reader.read_bool()?,
            }),
            0x06 => Self::PlayerPositionAndLook(PlayerPositionLookServerboundPacket {
                x: reader.read_f64()?,
                y: reader.read_f64()?,
                z: reader.read_f64()?,
                yaw: reader.read_f32()?,
                pitch: reader.read_f32()?,
                on_ground: reader.read_bool()?,
            }),
            0x07 => Self::PlayerDigging(PlayerDiggingPacket {
                action: DiggingAction::from_i32(reader.read_var_i32()?)?,
                position: BlockPosition::decode(reader.read_i64()?),
                face: reader.read_u8()?,
            }),
            0x08 => Self::PlayerBlockPlacement(PlayerBlockPlacementPacket {
                position: BlockPosition::decode(reader.read_i64()?),
                face: reader.read_u8()?,
                held_item: read_slot(&mut reader)?,
                cursor_x: byte_to_cursor_fraction(reader.read_u8()?),
                cursor_y: byte_to_cursor_fraction(reader.read_u8()?),
                cursor_z: byte_to_cursor_fraction(reader.read_u8()?),
            }),
            0x09 => Self::HeldItemChange(HeldItemChangePacket {
                slot: reader.read_i16()?,
            }),
            0x0A => Self::Animation(AnimationPacket),
            0x0B => Self::EntityAction(EntityActionPacket {
                entity_id: reader.read_var_i32()?,
                action: EntityActionKind::from_i32(reader.read_var_i32()?)?,
                aux_data: reader.read_var_i32()?,
            }),
            0x0D => Self::CloseWindow(CloseWindowServerboundPacket {
                window_id: reader.read_u8()?,
            }),
            0x0E => Self::ClickWindow(ClickWindowPacket {
                window_id: reader.read_u8()?,
                slot_id: reader.read_i16()?,
                button: reader.read_i8()?,
                action_number: reader.read_i16()?,
                mode: reader.read_i8()?,
                clicked_item: read_slot(&mut reader)?,
            }),
            0x0F => Self::ConfirmTransaction(ConfirmTransactionServerboundPacket {
                window_id: reader.read_u8()?,
                action_number: reader.read_i16()?,
                accepted: reader.read_bool()?,
            }),
            0x15 => Self::ClientSettings(ClientSettingsPacket {
                locale: reader.read_string(7)?,
                view_distance: reader.read_i8()?,
                chat_visibility: reader.read_u8()?,
                chat_colors: reader.read_bool()?,
                displayed_skin_parts: reader.read_u8()?,
            }),
            0x17 => Self::CustomPayload(CustomPayloadPacket {
                channel: reader.read_string(20)?,
                data: reader.read_remaining_bytes(MAX_CUSTOM_PAYLOAD_BYTES)?,
            }),
            other => return Err(CodecError::UnknownPacketId(other)),
        };

        reader.finish()?;
        Ok(packet)
    }

    pub fn encode_packet(&self) -> Result<EncodedPacket, CodecError> {
        let mut writer = PacketWriter::new();

        let packet_id = match self {
            Self::ResourcePackStatus(packet) => {
                if !(0..=3).contains(&packet.action) {
                    return Err(CodecError::InvalidEnumValue {
                        enum_name: "ResourcePackStatus",
                        actual: packet.action,
                    });
                }
                writer.write_string(&packet.hash, 40)?;
                writer.write_var_i32(packet.action);
                0x19
            }
            Self::PlayerAbilities(packet) => {
                writer.write_u8(packet.flags);
                writer.write_f32(packet.flying_speed);
                writer.write_f32(packet.walking_speed);
                0x13
            }
            Self::ClientStatus(action) => {
                if !(0..=2).contains(action) {
                    return Err(CodecError::InvalidEnumValue {
                        enum_name: "ClientStatus",
                        actual: *action,
                    });
                }
                writer.write_var_i32(*action);
                0x16
            }
            Self::KeepAlive(packet) => {
                writer.write_var_i32(packet.id);
                0x00
            }
            Self::ChatMessage(packet) => {
                writer.write_string(&packet.message, MAX_SERVERBOUND_CHAT_CHARS)?;
                0x01
            }
            Self::UseEntity(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_var_i32(packet.action.id());

                if let Some(target) = packet.target {
                    writer.write_f32(target[0]);
                    writer.write_f32(target[1]);
                    writer.write_f32(target[2]);
                }

                0x02
            }
            Self::Player(packet) => {
                writer.write_bool(packet.on_ground);
                0x03
            }
            Self::PlayerPosition(packet) => {
                writer.write_f64(packet.x);
                writer.write_f64(packet.y);
                writer.write_f64(packet.z);
                writer.write_bool(packet.on_ground);
                0x04
            }
            Self::PlayerLook(packet) => {
                writer.write_f32(packet.yaw);
                writer.write_f32(packet.pitch);
                writer.write_bool(packet.on_ground);
                0x05
            }
            Self::PlayerPositionAndLook(packet) => {
                writer.write_f64(packet.x);
                writer.write_f64(packet.y);
                writer.write_f64(packet.z);
                writer.write_f32(packet.yaw);
                writer.write_f32(packet.pitch);
                writer.write_bool(packet.on_ground);
                0x06
            }
            Self::PlayerDigging(packet) => {
                writer.write_var_i32(packet.action.id());
                writer.write_i64(packet.position.encode());
                writer.write_u8(packet.face);
                0x07
            }
            Self::PlayerBlockPlacement(packet) => {
                writer.write_i64(packet.position.encode());
                writer.write_u8(packet.face);
                write_slot(&mut writer, &packet.held_item);
                writer.write_u8(cursor_fraction_to_byte(packet.cursor_x));
                writer.write_u8(cursor_fraction_to_byte(packet.cursor_y));
                writer.write_u8(cursor_fraction_to_byte(packet.cursor_z));
                0x08
            }
            Self::HeldItemChange(packet) => {
                writer.write_i16(packet.slot);
                0x09
            }
            Self::Animation(_) => 0x0A,
            Self::EntityAction(packet) => {
                writer.write_var_i32(packet.entity_id);
                writer.write_var_i32(packet.action.id());
                writer.write_var_i32(packet.aux_data);
                0x0B
            }
            Self::CloseWindow(packet) => {
                writer.write_u8(packet.window_id);
                0x0D
            }
            Self::ClickWindow(packet) => {
                writer.write_u8(packet.window_id);
                writer.write_i16(packet.slot_id);
                writer.write_i8(packet.button);
                writer.write_i16(packet.action_number);
                writer.write_i8(packet.mode);
                write_slot(&mut writer, &packet.clicked_item);
                0x0E
            }
            Self::ConfirmTransaction(packet) => {
                writer.write_u8(packet.window_id);
                writer.write_i16(packet.action_number);
                writer.write_bool(packet.accepted);
                0x0F
            }
            Self::ClientSettings(packet) => {
                writer.write_string(&packet.locale, 7)?;
                writer.write_i8(packet.view_distance);
                writer.write_u8(packet.chat_visibility);
                writer.write_bool(packet.chat_colors);
                writer.write_u8(packet.displayed_skin_parts);
                0x15
            }
            Self::CustomPayload(packet) => {
                writer.write_string(&packet.channel, 20)?;
                if packet.data.len() > MAX_CUSTOM_PAYLOAD_BYTES {
                    return Err(CodecError::Buffer(BufferError::ByteArrayTooLong {
                        max_len: MAX_CUSTOM_PAYLOAD_BYTES,
                        actual: packet.data.len(),
                    }));
                }

                let mut body = writer.into_inner();
                body.extend_from_slice(&packet.data);

                return Ok(EncodedPacket::new(0x17, body));
            }
        };

        Ok(EncodedPacket::new(packet_id, writer.into_inner()))
    }
}

fn read_slot(reader: &mut PacketReader<'_>) -> Result<Slot, CodecError> {
    let item_id = reader.read_i16()?;

    if item_id < 0 {
        return Ok(None);
    }

    Ok(Some(ItemStack {
        item_id,
        count: reader.read_u8()?,
        damage: reader.read_i16()?,
        nbt: reader.read_nbt_blob()?,
    }))
}

fn write_slot(writer: &mut PacketWriter, slot: &Slot) {
    match slot {
        Some(item) => {
            writer.write_i16(item.item_id);
            writer.write_u8(item.count);
            writer.write_i16(item.damage);
            writer.write_nbt_blob(item.nbt.as_deref());
        }
        None => writer.write_i16(-1),
    }
}

fn byte_to_cursor_fraction(value: u8) -> f32 {
    f32::from(value) / 16.0
}

fn cursor_fraction_to_byte(value: f32) -> u8 {
    let clamped = value.clamp(0.0, 0.9375);
    (clamped * 16.0) as u8
}

fn sign_extend(value: i32, bits: u32) -> i32 {
    let shift = 32 - bits;
    (value << shift) >> shift
}

pub fn max_chunk_data_len(
    section_count: usize,
    has_sky_light: bool,
    include_biomes: bool,
) -> usize {
    let mut len = section_count * SECTION_DATA_BYTES;
    len += section_count * SECTION_LIGHT_BYTES;

    if has_sky_light {
        len += section_count * SECTION_LIGHT_BYTES;
    }

    if include_biomes {
        len += CHUNK_BIOME_BYTES;
    }

    len
}

#[cfg(test)]
mod tests {
    use super::{
        max_chunk_data_len, AnimationPacket, BlockChangePacket, BlockPosition, ChatMessagePacket,
        ChatMessageServerboundPacket, ChunkDataPacket, ClickWindowPacket, ClientSettingsPacket,
        CloseWindowPacket, CloseWindowServerboundPacket, ConfirmTransactionClientboundPacket,
        ConfirmTransactionServerboundPacket, CustomPayloadPacket, DiggingAction,
        DisplayScoreboardPacket, EntityActionKind, EntityActionPacket, EntityVelocityPacket,
        HeldItemChangePacket, ItemStack, JoinGamePacket, KeepAlivePacket, MapChunkBulkColumn,
        MapChunkBulkPacket, MultiBlockChangePacket, MultiBlockChangeRecord, OpenWindowPacket,
        PlayClientboundPacket, PlayDisconnectPacket, PlayServerboundPacket,
        PlayerBlockPlacementPacket, PlayerDiggingPacket, PlayerListEntry, PlayerListItemAction,
        PlayerListItemPacket, PlayerLookPacket, PlayerPacket, PlayerPositionAndLookPacket,
        PlayerPositionLookServerboundPacket, PlayerPositionPacket, PlayerProperty,
        PositionLookFlags, RespawnPacket, ScoreboardObjectiveMode, ScoreboardObjectivePacket,
        SetSlotPacket, SoundEffectPacket, TeamAction, TeamsPacket, UpdateHealthPacket,
        UpdateScoreAction, UpdateScorePacket, UseEntityAction, UseEntityPacket, WindowItemsPacket,
    };

    fn demo_item_stack() -> ItemStack {
        ItemStack {
            item_id: 261,
            count: 1,
            damage: 0,
            nbt: Some(vec![10, 0, 0, 0]),
        }
    }

    #[test]
    fn roundtrips_clientbound_play_packets() {
        let cases = [
            PlayClientboundPacket::KeepAlive(KeepAlivePacket { id: 42 }),
            PlayClientboundPacket::JoinGame(JoinGamePacket {
                entity_id: 7,
                game_mode: 1,
                hardcore: true,
                dimension: 0,
                difficulty: 2,
                max_players: 20,
                level_type: "default".to_owned(),
                reduced_debug_info: false,
            }),
            PlayClientboundPacket::ChatMessage(ChatMessagePacket {
                message_json: "{\"text\":\"Queue popped\"}".to_owned(),
                position: 1,
            }),
            PlayClientboundPacket::UpdateHealth(UpdateHealthPacket {
                health: 18.0,
                food_level: 20,
                saturation: 5.0,
            }),
            PlayClientboundPacket::Respawn(RespawnPacket {
                dimension: 0,
                difficulty: 2,
                game_mode: 0,
                level_type: "default".to_owned(),
            }),
            PlayClientboundPacket::PlayerPositionAndLook(PlayerPositionAndLookPacket {
                x: 1.5,
                y: 64.0,
                z: -2.25,
                yaw: 90.0,
                pitch: 15.0,
                flags: PositionLookFlags::from_bits(
                    PositionLookFlags::X | PositionLookFlags::Y_ROT,
                ),
            }),
            PlayClientboundPacket::EntityVelocity(EntityVelocityPacket {
                entity_id: 7,
                velocity_x: 1200,
                velocity_y: 800,
                velocity_z: -400,
            }),
            PlayClientboundPacket::ChunkData(ChunkDataPacket {
                chunk_x: 0,
                chunk_z: 0,
                full_chunk: true,
                primary_bitmask: 0x0001,
                data: vec![0; max_chunk_data_len(1, true, true)],
            }),
            PlayClientboundPacket::MultiBlockChange(MultiBlockChangePacket {
                chunk_x: 0,
                chunk_z: 0,
                records: vec![
                    MultiBlockChangeRecord {
                        packed_position: 0x1234,
                        block_state_id: 5,
                    },
                    MultiBlockChangeRecord {
                        packed_position: 0xabcd,
                        block_state_id: 1024,
                    },
                ],
            }),
            PlayClientboundPacket::BlockChange(BlockChangePacket {
                position: BlockPosition::new(12, 64, -7),
                block_state_id: 9,
            }),
            PlayClientboundPacket::MapChunkBulk(MapChunkBulkPacket {
                sky_light_sent: true,
                columns: vec![
                    MapChunkBulkColumn {
                        chunk_x: 0,
                        chunk_z: 0,
                        primary_bitmask: 0x0001,
                        data: vec![0; max_chunk_data_len(1, true, true)],
                    },
                    MapChunkBulkColumn {
                        chunk_x: 1,
                        chunk_z: 0,
                        primary_bitmask: 0x0003,
                        data: vec![0; max_chunk_data_len(2, true, true)],
                    },
                ],
            }),
            PlayClientboundPacket::SoundEffect(SoundEffectPacket {
                sound_name: "note.pling".to_owned(),
                effect_position_x: 128,
                effect_position_y: 520,
                effect_position_z: -64,
                volume: 0.75,
                pitch: 42,
            }),
            PlayClientboundPacket::OpenWindow(OpenWindowPacket {
                window_id: 4,
                inventory_type: "minecraft:chest".to_owned(),
                window_title_json: "{\"text\":\"Loot\"}".to_owned(),
                slot_count: 27,
                entity_id: None,
            }),
            PlayClientboundPacket::CloseWindow(CloseWindowPacket { window_id: 4 }),
            PlayClientboundPacket::SetSlot(SetSlotPacket {
                window_id: 0,
                slot_id: 36,
                item: Some(ItemStack::simple(276, 1, 0)),
            }),
            PlayClientboundPacket::WindowItems(WindowItemsPacket {
                window_id: 0,
                items: vec![Some(demo_item_stack()), None],
            }),
            PlayClientboundPacket::ConfirmTransaction(ConfirmTransactionClientboundPacket {
                window_id: 1,
                action_number: 5,
                accepted: false,
            }),
            PlayClientboundPacket::PlayerListItem(PlayerListItemPacket {
                action: PlayerListItemAction::AddPlayer,
                entries: vec![PlayerListEntry {
                    uuid: [1; 16],
                    name: Some("Rush".to_owned()),
                    properties: vec![PlayerProperty {
                        name: "textures".to_owned(),
                        value: "value".to_owned(),
                        signature: Some("sig".to_owned()),
                    }],
                    game_mode: Some(1),
                    latency: Some(38),
                    display_name_json: Some("{\"text\":\"[MVP+] Rush\"}".to_owned()),
                }],
            }),
            PlayClientboundPacket::ScoreboardObjective(ScoreboardObjectivePacket {
                objective_name: "bw".to_owned(),
                mode: ScoreboardObjectiveMode::Create,
                objective_value: "BED WARS".to_owned(),
                render_type: "integer".to_owned(),
            }),
            PlayClientboundPacket::UpdateScore(UpdateScorePacket {
                score_name: "Kills".to_owned(),
                action: UpdateScoreAction::Change,
                objective_name: "bw".to_owned(),
                value: 3,
            }),
            PlayClientboundPacket::DisplayScoreboard(DisplayScoreboardPacket {
                position: 1,
                score_name: "bw".to_owned(),
            }),
            PlayClientboundPacket::Teams(TeamsPacket {
                name: "red".to_owned(),
                action: TeamAction::Create,
                display_name: "Red".to_owned(),
                prefix: "c".to_owned(),
                suffix: "!".to_owned(),
                friendly_flags: 0,
                name_tag_visibility: "always".to_owned(),
                color: 12,
                players: vec!["Rush".to_owned()],
            }),
            PlayClientboundPacket::Disconnect(PlayDisconnectPacket {
                reason_json: "{\"text\":\"Server closed\"}".to_owned(),
            }),
        ];

        for packet in cases {
            let encoded = packet.encode_packet().expect("packet should encode");
            let decoded = PlayClientboundPacket::decode_packet(&encoded.packet_bytes())
                .expect("packet should decode");
            assert_eq!(decoded, packet);
        }
    }

    #[test]
    fn roundtrips_serverbound_play_packets() {
        let cases = [
            PlayServerboundPacket::KeepAlive(KeepAlivePacket { id: 42 }),
            PlayServerboundPacket::ChatMessage(ChatMessageServerboundPacket::vanilla("gg")),
            PlayServerboundPacket::UseEntity(UseEntityPacket {
                entity_id: 7,
                action: UseEntityAction::Attack,
                target: None,
            }),
            PlayServerboundPacket::Player(PlayerPacket { on_ground: true }),
            PlayServerboundPacket::PlayerPosition(PlayerPositionPacket {
                x: 12.0,
                y: 65.62,
                z: -5.0,
                on_ground: false,
            }),
            PlayServerboundPacket::PlayerLook(PlayerLookPacket {
                yaw: 180.0,
                pitch: -10.0,
                on_ground: true,
            }),
            PlayServerboundPacket::PlayerPositionAndLook(PlayerPositionLookServerboundPacket {
                x: 9.0,
                y: 70.0,
                z: 3.5,
                yaw: 45.0,
                pitch: 5.0,
                on_ground: true,
            }),
            PlayServerboundPacket::PlayerDigging(PlayerDiggingPacket {
                action: DiggingAction::ReleaseUseItem,
                position: BlockPosition::ORIGIN,
                face: 0,
            }),
            PlayServerboundPacket::PlayerBlockPlacement(PlayerBlockPlacementPacket::use_item(
                Some(demo_item_stack()),
            )),
            PlayServerboundPacket::HeldItemChange(HeldItemChangePacket { slot: 3 }),
            PlayServerboundPacket::Animation(AnimationPacket),
            PlayServerboundPacket::EntityAction(EntityActionPacket {
                entity_id: 7,
                action: EntityActionKind::StartSprinting,
                aux_data: 0,
            }),
            PlayServerboundPacket::CloseWindow(CloseWindowServerboundPacket { window_id: 2 }),
            PlayServerboundPacket::ClickWindow(ClickWindowPacket {
                window_id: 0,
                slot_id: 36,
                button: 0,
                action_number: 9,
                mode: 2,
                clicked_item: Some(ItemStack::simple(276, 1, 0)),
            }),
            PlayServerboundPacket::ConfirmTransaction(ConfirmTransactionServerboundPacket {
                window_id: 0,
                action_number: 9,
                accepted: true,
            }),
            PlayServerboundPacket::ClientSettings(ClientSettingsPacket::vanilla_headless_defaults()),
            PlayServerboundPacket::CustomPayload(
                CustomPayloadPacket::brand_payload("RustMinecraft")
                    .expect("brand payload should encode"),
            ),
        ];

        for packet in cases {
            let encoded = packet.encode_packet().expect("packet should encode");
            let decoded = PlayServerboundPacket::decode_packet(&encoded.packet_bytes())
                .expect("packet should decode");
            assert_eq!(decoded, packet);
        }
    }

    #[test]
    fn roundtrips_block_position_encoding() {
        let position = BlockPosition::new(-30, 255, 19);
        let decoded = BlockPosition::decode(position.encode());
        assert_eq!(decoded, position);
    }
}
