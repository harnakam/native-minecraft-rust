//! Chunk, block, and world-state storage for the M3 world pipeline.

use rmc_net::codec::play::{
    max_chunk_data_len, BlockChangePacket, BlockPosition as NetBlockPosition, ChunkDataPacket,
    MapChunkBulkPacket, MultiBlockChangePacket, PlayClientboundPacket, CHUNK_BIOME_BYTES,
    SECTION_BLOCK_COUNT, SECTION_DATA_BYTES, SECTION_LIGHT_BYTES,
};
use std::collections::HashMap;

pub mod collision;
pub mod time;
pub mod weather;

pub const CHUNK_EDGE: usize = 16;
pub const CHUNK_HEIGHT: usize = 256;
pub const SECTION_COUNT: usize = 16;
pub const AIR_BLOCK_STATE_ID: BlockStateId = 0;

pub type BlockStateId = u16;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn min_block_x(self) -> i32 {
        self.x * CHUNK_EDGE as i32
    }

    pub fn min_block_z(self) -> i32 {
        self.z * CHUNK_EDGE as i32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn chunk_pos(self) -> ChunkPos {
        ChunkPos::new(
            self.x.div_euclid(CHUNK_EDGE as i32),
            self.z.div_euclid(CHUNK_EDGE as i32),
        )
    }

    pub fn local_x(self) -> u8 {
        self.x.rem_euclid(CHUNK_EDGE as i32) as u8
    }

    pub fn local_y(self) -> u8 {
        self.y.rem_euclid(CHUNK_EDGE as i32) as u8
    }

    pub fn local_z(self) -> u8 {
        self.z.rem_euclid(CHUNK_EDGE as i32) as u8
    }

    pub fn section_index(self) -> Option<usize> {
        if !(0..CHUNK_HEIGHT as i32).contains(&self.y) {
            return None;
        }

        Some((self.y as usize) >> 4)
    }
}

impl From<NetBlockPosition> for BlockPos {
    fn from(value: NetBlockPosition) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldConfig {
    pub has_sky_light: bool,
}

impl WorldConfig {
    pub const fn overworld() -> Self {
        Self {
            has_sky_light: true,
        }
    }

    pub const fn no_sky() -> Self {
        Self {
            has_sky_light: false,
        }
    }
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self::overworld()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorldError {
    InvalidBlockStateId(i32),
    UnexpectedChunkDataLen {
        chunk: ChunkPos,
        expected: usize,
        actual: usize,
        primary_bitmask: u16,
        full_chunk: bool,
        has_sky_light: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChunkSection {
    y_index: u8,
    block_states: Vec<BlockStateId>,
    block_light: Vec<u8>,
    sky_light: Option<Vec<u8>>,
    non_air_blocks: usize,
}

impl ChunkSection {
    pub fn new_empty(y_index: u8, has_sky_light: bool) -> Self {
        Self {
            y_index,
            block_states: vec![AIR_BLOCK_STATE_ID; SECTION_BLOCK_COUNT],
            block_light: vec![0; SECTION_LIGHT_BYTES],
            sky_light: has_sky_light.then(|| vec![0; SECTION_LIGHT_BYTES]),
            non_air_blocks: 0,
        }
    }

    pub fn from_wire(
        y_index: u8,
        block_bytes: &[u8],
        block_light: &[u8],
        sky_light: Option<&[u8]>,
    ) -> Result<Self, WorldError> {
        let mut section = Self::new_empty(y_index, sky_light.is_some());

        for (index, chunk) in block_bytes.chunks_exact(2).enumerate() {
            let value = u16::from_le_bytes([chunk[0], chunk[1]]);
            section.block_states[index] = value;

            if value != AIR_BLOCK_STATE_ID {
                section.non_air_blocks += 1;
            }
        }

        if !block_light.is_empty() {
            section.block_light.copy_from_slice(block_light);
        }

        if let Some(sky_light_bytes) = sky_light {
            if let Some(storage) = &mut section.sky_light {
                storage.copy_from_slice(sky_light_bytes);
            }
        }

        Ok(section)
    }

    pub fn y_index(&self) -> u8 {
        self.y_index
    }

    pub fn non_air_blocks(&self) -> usize {
        self.non_air_blocks
    }

    pub fn is_empty(&self) -> bool {
        self.non_air_blocks == 0
    }

    pub fn block_state_at(&self, local_x: u8, local_y: u8, local_z: u8) -> BlockStateId {
        self.block_states[section_index(local_x, local_y, local_z)]
    }

    pub fn block_light_at(&self, local_x: u8, local_y: u8, local_z: u8) -> u8 {
        nibble_at(&self.block_light, section_index(local_x, local_y, local_z))
    }

    pub fn sky_light_at(&self, local_x: u8, local_y: u8, local_z: u8) -> u8 {
        self.sky_light
            .as_ref()
            .map(|values| nibble_at(values, section_index(local_x, local_y, local_z)))
            .unwrap_or(0)
    }

    pub fn set_block_state(&mut self, local_x: u8, local_y: u8, local_z: u8, state: BlockStateId) {
        let index = section_index(local_x, local_y, local_z);
        let previous = self.block_states[index];

        if previous == state {
            return;
        }

        if previous == AIR_BLOCK_STATE_ID && state != AIR_BLOCK_STATE_ID {
            self.non_air_blocks += 1;
        } else if previous != AIR_BLOCK_STATE_ID && state == AIR_BLOCK_STATE_ID {
            self.non_air_blocks -= 1;
        }

        self.block_states[index] = state;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChunkColumn {
    pos: ChunkPos,
    has_sky_light: bool,
    sections: Vec<Option<ChunkSection>>,
    biomes: Option<[u8; CHUNK_BIOME_BYTES]>,
}

impl ChunkColumn {
    pub fn new(pos: ChunkPos, has_sky_light: bool) -> Self {
        Self {
            pos,
            has_sky_light,
            sections: vec![None; SECTION_COUNT],
            biomes: None,
        }
    }

    pub fn pos(&self) -> ChunkPos {
        self.pos
    }

    pub fn has_sky_light(&self) -> bool {
        self.has_sky_light
    }

    pub fn section_mask(&self) -> u16 {
        self.sections
            .iter()
            .enumerate()
            .fold(0u16, |mask, (index, section)| {
                if section.is_some() {
                    mask | (1u16 << index)
                } else {
                    mask
                }
            })
    }

    pub fn loaded_sections(&self) -> usize {
        self.sections
            .iter()
            .filter(|section| section.is_some())
            .count()
    }

    pub fn non_air_blocks(&self) -> usize {
        self.sections
            .iter()
            .filter_map(|section| section.as_ref())
            .map(ChunkSection::non_air_blocks)
            .sum()
    }

    pub fn biomes(&self) -> Option<&[u8; CHUNK_BIOME_BYTES]> {
        self.biomes.as_ref()
    }

    pub fn section(&self, index: usize) -> Option<&ChunkSection> {
        self.sections
            .get(index)
            .and_then(|section| section.as_ref())
    }

    pub fn sections(&self) -> impl Iterator<Item = (usize, &ChunkSection)> {
        self.sections
            .iter()
            .enumerate()
            .filter_map(|(index, section)| section.as_ref().map(|section| (index, section)))
    }

    pub fn block_state_at(&self, local_x: u8, y: u8, local_z: u8) -> Option<BlockStateId> {
        let section = self.section((y as usize) >> 4)?;
        Some(section.block_state_at(local_x, y & 0x0f, local_z))
    }

    pub fn block_light_at(&self, local_x: u8, y: u8, local_z: u8) -> Option<u8> {
        let section = self.section((y as usize) >> 4)?;
        Some(section.block_light_at(local_x, y & 0x0f, local_z))
    }

    pub fn sky_light_at(&self, local_x: u8, y: u8, local_z: u8) -> Option<u8> {
        let section = self.section((y as usize) >> 4)?;
        Some(section.sky_light_at(local_x, y & 0x0f, local_z))
    }

    pub fn replace_sections(
        &mut self,
        primary_bitmask: u16,
        decoded_sections: Vec<(usize, ChunkSection)>,
        full_chunk: bool,
        biomes: Option<[u8; CHUNK_BIOME_BYTES]>,
    ) {
        if full_chunk {
            self.sections.fill(None);
        }

        for (index, section) in decoded_sections {
            self.sections[index] = (!section.is_empty()).then_some(section);
        }

        if full_chunk {
            for index in 0..SECTION_COUNT {
                if (primary_bitmask & (1u16 << index)) == 0 {
                    self.sections[index] = None;
                }
            }

            self.biomes = biomes;
        }
    }

    pub fn set_block_state(&mut self, local_x: u8, y: u8, local_z: u8, state: BlockStateId) {
        let section_index = (y as usize) >> 4;
        let local_y = y & 0x0f;

        if self.sections[section_index].is_none() {
            if state == AIR_BLOCK_STATE_ID {
                return;
            }

            self.sections[section_index] = Some(ChunkSection::new_empty(
                section_index as u8,
                self.has_sky_light,
            ));
        }

        if let Some(section) = &mut self.sections[section_index] {
            section.set_block_state(local_x, local_y, local_z, state);

            if section.is_empty() {
                self.sections[section_index] = None;
            }
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorldMetrics {
    pub loaded_chunks: usize,
    pub loaded_sections: usize,
    pub loaded_non_air_blocks: usize,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorldChangeSummary {
    pub touched_chunks: Vec<ChunkPos>,
    pub removed_chunks: Vec<ChunkPos>,
    pub changed_blocks: usize,
    pub rebuilt_sections: usize,
}

impl WorldChangeSummary {
    fn touch_chunk(&mut self, chunk: ChunkPos) {
        if !self.touched_chunks.contains(&chunk) {
            self.touched_chunks.push(chunk);
        }
    }

    fn remove_chunk(&mut self, chunk: ChunkPos) {
        if !self.removed_chunks.contains(&chunk) {
            self.removed_chunks.push(chunk);
        }
    }

    fn merge(&mut self, other: Self) {
        for chunk in other.touched_chunks {
            self.touch_chunk(chunk);
        }

        for chunk in other.removed_chunks {
            self.remove_chunk(chunk);
        }

        self.changed_blocks += other.changed_blocks;
        self.rebuilt_sections += other.rebuilt_sections;
    }
}

#[derive(Clone, Debug, Default)]
pub struct WorldSnapshot {
    weather: weather::Weather,
    time: time::WorldTime,
    config: WorldConfig,
    chunks: HashMap<ChunkPos, ChunkColumn>,
}

impl WorldSnapshot {
    pub fn new(config: WorldConfig) -> Self {
        Self {
            weather: weather::Weather::default(),
            time: time::WorldTime::default(),
            config,
            chunks: HashMap::new(),
        }
    }

    pub fn config(&self) -> WorldConfig {
        self.config
    }

    pub fn time(&self) -> time::WorldTime {
        self.time
    }

    pub fn weather(&self) -> weather::Weather {
        self.weather
    }

    pub fn advance_time(&mut self, ticks: usize) {
        self.time.advance(ticks);
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&ChunkColumn> {
        self.chunks.get(&pos)
    }

    pub fn chunks(&self) -> impl Iterator<Item = &ChunkColumn> {
        self.chunks.values()
    }

    pub fn chunk_positions(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.chunks.keys().copied()
    }

    pub fn metrics(&self) -> WorldMetrics {
        WorldMetrics {
            loaded_chunks: self.chunks.len(),
            loaded_sections: self.chunks.values().map(ChunkColumn::loaded_sections).sum(),
            loaded_non_air_blocks: self.chunks.values().map(ChunkColumn::non_air_blocks).sum(),
        }
    }

    pub fn block_state_at(&self, pos: BlockPos) -> Option<BlockStateId> {
        let section_index = pos.section_index()?;
        let chunk = self.chunks.get(&pos.chunk_pos())?;
        let section = chunk.section(section_index)?;
        Some(section.block_state_at(pos.local_x(), pos.local_y(), pos.local_z()))
    }

    pub fn block_state_or_air(&self, pos: BlockPos) -> BlockStateId {
        self.block_state_at(pos).unwrap_or(AIR_BLOCK_STATE_ID)
    }

    pub fn block_light_at(&self, pos: BlockPos) -> u8 {
        let Some(section_index) = pos.section_index() else {
            return 0;
        };
        self.chunks
            .get(&pos.chunk_pos())
            .and_then(|chunk| chunk.section(section_index))
            .map(|section| section.block_light_at(pos.local_x(), pos.local_y(), pos.local_z()))
            .unwrap_or(0)
    }

    pub fn sky_light_at(&self, pos: BlockPos) -> u8 {
        let Some(section_index) = pos.section_index() else {
            return 0;
        };
        self.chunks
            .get(&pos.chunk_pos())
            .and_then(|chunk| chunk.section(section_index))
            .map(|section| section.sky_light_at(pos.local_x(), pos.local_y(), pos.local_z()))
            .unwrap_or(0)
    }

    pub fn apply_chunk_data(
        &mut self,
        packet: &ChunkDataPacket,
    ) -> Result<WorldChangeSummary, WorldError> {
        let chunk = ChunkPos::new(packet.chunk_x, packet.chunk_z);

        if packet.full_chunk && packet.primary_bitmask == 0 && packet.data.is_empty() {
            self.chunks.remove(&chunk);

            let mut summary = WorldChangeSummary::default();
            summary.remove_chunk(chunk);
            return Ok(summary);
        }

        let (decoded_sections, biomes) = decode_chunk_payload(
            chunk,
            packet.primary_bitmask,
            &packet.data,
            packet.full_chunk,
            self.config.has_sky_light,
        )?;

        let rebuilt_sections = decoded_sections.len();
        let column = self
            .chunks
            .entry(chunk)
            .or_insert_with(|| ChunkColumn::new(chunk, self.config.has_sky_light));
        column.replace_sections(
            packet.primary_bitmask,
            decoded_sections,
            packet.full_chunk,
            biomes,
        );

        let mut summary = WorldChangeSummary::default();
        summary.touch_chunk(chunk);
        summary.changed_blocks = column.non_air_blocks();
        summary.rebuilt_sections = rebuilt_sections;
        Ok(summary)
    }

    pub fn apply_map_chunk_bulk(
        &mut self,
        packet: &MapChunkBulkPacket,
    ) -> Result<WorldChangeSummary, WorldError> {
        let mut summary = WorldChangeSummary::default();

        for column in &packet.columns {
            let chunk = ChunkPos::new(column.chunk_x, column.chunk_z);
            let (decoded_sections, biomes) = decode_chunk_payload(
                chunk,
                column.primary_bitmask,
                &column.data,
                true,
                packet.sky_light_sent,
            )?;

            let rebuilt_sections = decoded_sections.len();
            let world_column = self
                .chunks
                .entry(chunk)
                .or_insert_with(|| ChunkColumn::new(chunk, packet.sky_light_sent));
            world_column.has_sky_light = packet.sky_light_sent;
            world_column.replace_sections(column.primary_bitmask, decoded_sections, true, biomes);

            let mut chunk_summary = WorldChangeSummary::default();
            chunk_summary.touch_chunk(chunk);
            chunk_summary.changed_blocks = world_column.non_air_blocks();
            chunk_summary.rebuilt_sections = rebuilt_sections;
            summary.merge(chunk_summary);
        }

        Ok(summary)
    }

    pub fn apply_block_change(
        &mut self,
        packet: &BlockChangePacket,
    ) -> Result<WorldChangeSummary, WorldError> {
        let block_pos = BlockPos::from(packet.position);
        let state = decode_block_state_id(packet.block_state_id)?;
        let chunk_pos = block_pos.chunk_pos();

        if !(0..CHUNK_HEIGHT as i32).contains(&block_pos.y) {
            return Ok(WorldChangeSummary::default());
        }

        if state == AIR_BLOCK_STATE_ID && !self.chunks.contains_key(&chunk_pos) {
            return Ok(WorldChangeSummary::default());
        }

        let column = self
            .chunks
            .entry(chunk_pos)
            .or_insert_with(|| ChunkColumn::new(chunk_pos, self.config.has_sky_light));
        column.set_block_state(
            block_pos.local_x(),
            block_pos.y as u8,
            block_pos.local_z(),
            state,
        );

        let mut summary = WorldChangeSummary::default();
        summary.touch_chunk(chunk_pos);
        summary.changed_blocks = 1;
        Ok(summary)
    }

    pub fn apply_multi_block_change(
        &mut self,
        packet: &MultiBlockChangePacket,
    ) -> Result<WorldChangeSummary, WorldError> {
        let chunk_pos = ChunkPos::new(packet.chunk_x, packet.chunk_z);
        let has_non_air = packet
            .records
            .iter()
            .any(|record| record.block_state_id != AIR_BLOCK_STATE_ID as i32);

        if !has_non_air && !self.chunks.contains_key(&chunk_pos) {
            return Ok(WorldChangeSummary::default());
        }

        let column = self
            .chunks
            .entry(chunk_pos)
            .or_insert_with(|| ChunkColumn::new(chunk_pos, self.config.has_sky_light));

        for record in &packet.records {
            let state = decode_block_state_id(record.block_state_id)?;
            column.set_block_state(record.local_x(), record.local_y(), record.local_z(), state);
        }

        let mut summary = WorldChangeSummary::default();
        summary.touch_chunk(chunk_pos);
        summary.changed_blocks = packet.records.len();
        Ok(summary)
    }

    pub fn apply_play_packet(
        &mut self,
        packet: &PlayClientboundPacket,
    ) -> Result<Option<WorldChangeSummary>, WorldError> {
        let summary = match packet {
            PlayClientboundPacket::ChangeGameState(packet) => {
                self.weather.receive(packet.reason, packet.value);
                None
            }
            PlayClientboundPacket::TimeUpdate(packet) => {
                self.time
                    .receive(packet.total_world_time, packet.world_time);
                None
            }
            PlayClientboundPacket::ChunkData(packet) => Some(self.apply_chunk_data(packet)?),
            PlayClientboundPacket::MultiBlockChange(packet) => {
                Some(self.apply_multi_block_change(packet)?)
            }
            PlayClientboundPacket::BlockChange(packet) => Some(self.apply_block_change(packet)?),
            PlayClientboundPacket::Explosion(packet) => {
                let mut changes = WorldChangeSummary::default();
                for position in packet.affected_positions() {
                    let update =
                        self.apply_block_change(&rmc_net::codec::play::BlockChangePacket {
                            position,
                            block_state_id: 0,
                        })?;
                    changes.merge(update);
                }
                Some(changes)
            }
            PlayClientboundPacket::MapChunkBulk(packet) => Some(self.apply_map_chunk_bulk(packet)?),
            PlayClientboundPacket::KeepAlive(_)
            | PlayClientboundPacket::JoinGame(_)
            | PlayClientboundPacket::ChatMessage(_)
            | PlayClientboundPacket::UpdateHealth(_)
            | PlayClientboundPacket::Respawn(_)
            | PlayClientboundPacket::PlayerPositionAndLook(_)
            | PlayClientboundPacket::SpawnPlayer(_)
            | PlayClientboundPacket::EntityVelocity(_)
            | PlayClientboundPacket::DestroyEntities(_)
            | PlayClientboundPacket::EntityRelativeMove(_)
            | PlayClientboundPacket::EntityLook(_)
            | PlayClientboundPacket::EntityLookMove(_)
            | PlayClientboundPacket::EntityTeleport(_)
            | PlayClientboundPacket::EntityHeadLook(_)
            | PlayClientboundPacket::SoundEffect(_)
            | PlayClientboundPacket::OpenWindow(_)
            | PlayClientboundPacket::CloseWindow(_)
            | PlayClientboundPacket::SetSlot(_)
            | PlayClientboundPacket::WindowItems(_)
            | PlayClientboundPacket::WindowProperty(_)
            | PlayClientboundPacket::EntityEquipment(_)
            | PlayClientboundPacket::ConfirmTransaction(_)
            | PlayClientboundPacket::PlayerListItem(_)
            | PlayClientboundPacket::ScoreboardObjective(_)
            | PlayClientboundPacket::UpdateScore(_)
            | PlayClientboundPacket::DisplayScoreboard(_)
            | PlayClientboundPacket::Teams(_)
            | PlayClientboundPacket::PlayerAbilities(_)
            | PlayClientboundPacket::HeldItemChange(_)
            | PlayClientboundPacket::EntityEffect(_)
            | PlayClientboundPacket::RemoveEntityEffect(_)
            | PlayClientboundPacket::EntityProperties(_)
            | PlayClientboundPacket::Disconnect(_) => None,
        };

        Ok(summary)
    }
}

fn decode_chunk_payload(
    chunk: ChunkPos,
    primary_bitmask: u16,
    data: &[u8],
    full_chunk: bool,
    has_sky_light: bool,
) -> Result<(Vec<(usize, ChunkSection)>, Option<[u8; CHUNK_BIOME_BYTES]>), WorldError> {
    let section_count = primary_bitmask.count_ones() as usize;
    let expected_len = max_chunk_data_len(section_count, has_sky_light, full_chunk);

    if data.len() != expected_len {
        return Err(WorldError::UnexpectedChunkDataLen {
            chunk,
            expected: expected_len,
            actual: data.len(),
            primary_bitmask,
            full_chunk,
            has_sky_light,
        });
    }

    let mut offset = 0usize;
    let mut sections = Vec::with_capacity(section_count);

    for section_index in 0..SECTION_COUNT {
        if (primary_bitmask & (1u16 << section_index)) == 0 {
            continue;
        }

        let block_bytes = &data[offset..offset + SECTION_DATA_BYTES];
        offset += SECTION_DATA_BYTES;
        sections.push((
            section_index,
            ChunkSection::from_wire(section_index as u8, block_bytes, &[], None)?,
        ));
    }

    for (_, section) in &mut sections {
        section.block_light = data[offset..offset + SECTION_LIGHT_BYTES].to_vec();
        offset += SECTION_LIGHT_BYTES;
    }

    if has_sky_light {
        for (_, section) in &mut sections {
            section.sky_light = Some(data[offset..offset + SECTION_LIGHT_BYTES].to_vec());
            offset += SECTION_LIGHT_BYTES;
        }
    }

    let biomes = if full_chunk {
        let mut biome_array = [0u8; CHUNK_BIOME_BYTES];
        biome_array.copy_from_slice(&data[offset..offset + CHUNK_BIOME_BYTES]);
        Some(biome_array)
    } else {
        None
    };

    Ok((sections, biomes))
}

fn decode_block_state_id(block_state_id: i32) -> Result<BlockStateId, WorldError> {
    u16::try_from(block_state_id).map_err(|_| WorldError::InvalidBlockStateId(block_state_id))
}

fn section_index(local_x: u8, local_y: u8, local_z: u8) -> usize {
    ((local_y as usize) << 8) | ((local_z as usize) << 4) | local_x as usize
}

fn nibble_at(storage: &[u8], index: usize) -> u8 {
    let byte = storage[index >> 1];

    if (index & 1) == 0 {
        byte & 0x0f
    } else {
        (byte >> 4) & 0x0f
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlockPos, ChunkPos, WorldConfig, WorldSnapshot, AIR_BLOCK_STATE_ID, CHUNK_BIOME_BYTES,
        SECTION_BLOCK_COUNT, SECTION_DATA_BYTES, SECTION_LIGHT_BYTES,
    };
    use rmc_net::codec::play::{
        BlockChangePacket, BlockPosition, ChunkDataPacket, MultiBlockChangePacket,
        MultiBlockChangeRecord,
    };

    fn encode_section(state_overrides: &[(usize, u16)], has_sky_light: bool) -> Vec<u8> {
        let mut states = vec![0u16; SECTION_BLOCK_COUNT];

        for (index, state) in state_overrides {
            states[*index] = *state;
        }

        let mut bytes = Vec::with_capacity(
            SECTION_DATA_BYTES
                + SECTION_LIGHT_BYTES
                + if has_sky_light {
                    SECTION_LIGHT_BYTES
                } else {
                    0
                }
                + CHUNK_BIOME_BYTES,
        );

        for state in states {
            bytes.extend_from_slice(&state.to_le_bytes());
        }

        bytes.extend_from_slice(&vec![0u8; SECTION_LIGHT_BYTES]);

        if has_sky_light {
            bytes.extend_from_slice(&vec![15u8; SECTION_LIGHT_BYTES]);
        }

        bytes.extend_from_slice(&[1u8; CHUNK_BIOME_BYTES]);
        bytes
    }

    #[test]
    fn applies_full_chunk_data_and_reads_block_state() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&ChunkDataPacket {
                chunk_x: 0,
                chunk_z: 0,
                full_chunk: true,
                primary_bitmask: 0x0001,
                data: encode_section(&[(0, 5), (1, 7)], true),
            })
            .expect("chunk data should apply");

        assert_eq!(world.metrics().loaded_chunks, 1);
        assert_eq!(world.block_state_at(BlockPos::new(0, 0, 0)), Some(5));
        assert_eq!(world.block_state_at(BlockPos::new(1, 0, 0)), Some(7));
    }

    #[test]
    fn applies_block_updates_to_existing_chunk() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&ChunkDataPacket {
                chunk_x: 0,
                chunk_z: 0,
                full_chunk: true,
                primary_bitmask: 0x0001,
                data: encode_section(&[], true),
            })
            .expect("chunk data should apply");

        world
            .apply_block_change(&BlockChangePacket {
                position: BlockPosition::new(3, 12, 4),
                block_state_id: 11,
            })
            .expect("block change should apply");

        assert_eq!(world.block_state_at(BlockPos::new(3, 12, 4)), Some(11));
    }

    #[test]
    fn applies_multi_block_change() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_multi_block_change(&MultiBlockChangePacket {
                chunk_x: 1,
                chunk_z: -1,
                records: vec![
                    MultiBlockChangeRecord {
                        packed_position: (2u16 << 12) | (4u16 << 8) | 9u16,
                        block_state_id: 13,
                    },
                    MultiBlockChangeRecord {
                        packed_position: (1u16 << 12) | (7u16 << 8) | 15u16,
                        block_state_id: 42,
                    },
                ],
            })
            .expect("multi block change should apply");

        assert_eq!(
            world.block_state_at(BlockPos::new(
                ChunkPos::new(1, -1).min_block_x() + 2,
                9,
                ChunkPos::new(1, -1).min_block_z() + 4,
            )),
            Some(13)
        );
        assert_eq!(
            world.block_state_at(BlockPos::new(
                ChunkPos::new(1, -1).min_block_x() + 1,
                15,
                ChunkPos::new(1, -1).min_block_z() + 7,
            )),
            Some(42)
        );
    }

    #[test]
    fn unloads_chunk_with_empty_full_chunk_packet() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&ChunkDataPacket {
                chunk_x: 0,
                chunk_z: 0,
                full_chunk: true,
                primary_bitmask: 0x0001,
                data: encode_section(&[(0, 5)], true),
            })
            .expect("chunk data should apply");

        world
            .apply_chunk_data(&ChunkDataPacket {
                chunk_x: 0,
                chunk_z: 0,
                full_chunk: true,
                primary_bitmask: 0,
                data: Vec::new(),
            })
            .expect("empty packet should unload chunk");

        assert_eq!(world.metrics().loaded_chunks, 0);
        assert_eq!(world.block_state_at(BlockPos::new(0, 0, 0)), None);
        assert_eq!(
            world.block_state_or_air(BlockPos::new(0, 0, 0)),
            AIR_BLOCK_STATE_ID
        );
    }
}

pub mod environment;
mod selection_properties;
