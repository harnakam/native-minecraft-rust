//! Rendering facade, debug chunk visualization, and naive chunk mesh generation.

use rmc_game::camera::CameraState;
use rmc_game::player::Vec3;
use rmc_game::simulation::SimulationSnapshot;
use rmc_ui::MinimalHud;
use rmc_world::{
    BlockPos, BlockStateId, ChunkPos, WorldChangeSummary, WorldSnapshot, AIR_BLOCK_STATE_ID,
    CHUNK_EDGE, CHUNK_HEIGHT,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraViewSnapshot {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub mouse_captured: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimulationViewSnapshot {
    pub network_position: Vec3,
    pub simulated_position: Vec3,
    pub interpolated_position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub sprinting: bool,
    pub sneaking: bool,
    pub sprint_reset_ticks: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderFrameSnapshot {
    pub tick_index: u64,
    pub interpolation_alpha: f32,
    pub camera: CameraViewSnapshot,
    pub simulation: SimulationViewSnapshot,
    pub debug_entities: Vec<DebugEntityDraw>,
    pub hud: MinimalHud,
}

impl RenderFrameSnapshot {
    pub fn from_simulation_state(
        tick_index: u64,
        interpolation_alpha: f32,
        simulation: &SimulationSnapshot,
        camera: CameraState,
        hud: MinimalHud,
    ) -> Self {
        let interpolated_position = simulation.render_position(interpolation_alpha);
        let debug_entities = build_player_debug_draws(simulation, interpolated_position);

        Self {
            tick_index,
            interpolation_alpha,
            camera: CameraViewSnapshot {
                position: interpolated_position.add(Vec3::new(
                    0.0,
                    f64::from(
                        1.62_f32
                            - if simulation.player.sneaking {
                                0.08
                            } else {
                                0.0
                            },
                    ),
                    0.0,
                )),
                yaw: camera.yaw,
                pitch: camera.pitch,
                mouse_captured: camera.capture.mouse_captured,
            },
            simulation: SimulationViewSnapshot {
                network_position: simulation.network.position,
                simulated_position: simulation.player.position,
                interpolated_position,
                velocity: simulation.velocity,
                on_ground: simulation.player.on_ground,
                sprinting: simulation.player.sprinting,
                sneaking: simulation.player.sneaking,
                sprint_reset_ticks: simulation.sprint_reset_ticks,
            },
            debug_entities,
            hud,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugChunkBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebugChunkDraw {
    pub chunk: ChunkPos,
    pub bounds: DebugChunkBounds,
    pub section_mask: u16,
    pub non_air_blocks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugEntityBounds {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DebugEntityKind {
    LocalPlayer,
    NetworkGhost,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebugEntityDraw {
    pub kind: DebugEntityKind,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub on_ground: bool,
    pub bounds: DebugEntityBounds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrustumConfig {
    pub horizontal_fov_degrees: f32,
    pub vertical_fov_degrees: f32,
    pub view_distance_chunks: u8,
    pub near_plane: f32,
    pub padding_blocks: f32,
}

impl FrustumConfig {
    pub fn debug_default() -> Self {
        Self {
            horizontal_fov_degrees: 100.0,
            vertical_fov_degrees: 70.0,
            view_distance_chunks: 8,
            near_plane: 0.05,
            padding_blocks: 2.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChunkMeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub face: ChunkFace,
    pub block_state_id: BlockStateId,
    pub block_light: u8,
    pub sky_light: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChunkMesh {
    pub chunk: ChunkPos,
    pub vertices: Vec<ChunkMeshVertex>,
    pub indices: Vec<u32>,
    pub visible_faces: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshRebuildSummary {
    pub rebuilt_chunks: Vec<ChunkPos>,
    pub removed_chunks: Vec<ChunkPos>,
    pub uploads: Vec<ChunkMeshUpload>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChunkMeshUpload {
    pub chunk: ChunkPos,
    pub vertex_count: usize,
    pub index_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldRenderSnapshot {
    pub visible_chunks: Vec<ChunkPos>,
    pub debug_chunks: Vec<DebugChunkDraw>,
    pub chunk_meshes: Vec<ChunkMesh>,
    pub debug_entities: Vec<DebugEntityDraw>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeshBuildConfig {
    pub max_jobs_per_frame: usize,
    pub scratch_vertex_capacity: usize,
    pub scratch_index_capacity: usize,
}

impl MeshBuildConfig {
    pub fn performance_default() -> Self {
        Self {
            max_jobs_per_frame: 4,
            scratch_vertex_capacity: 24 * 16 * 16,
            scratch_index_capacity: 36 * 16 * 16,
        }
    }
}

impl Default for MeshBuildConfig {
    fn default() -> Self {
        Self::performance_default()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshBuildAction {
    Rebuild,
    Remove,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeshBuildJob {
    pub chunk: ChunkPos,
    pub action: MeshBuildAction,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeshBuildResult {
    pub chunk: ChunkPos,
    pub mesh: Option<ChunkMesh>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshBuildCycle {
    pub scheduled_jobs: Vec<MeshBuildJob>,
    pub completed_jobs: usize,
    pub remaining_dirty_chunks: usize,
    pub summary: MeshRebuildSummary,
    pub uploaded_vertices: usize,
    pub uploaded_indices: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderTelemetrySnapshot {
    pub visible_chunks: usize,
    pub cached_meshes: usize,
    pub dirty_chunks: usize,
    pub scheduled_jobs: usize,
    pub rebuilt_chunks: usize,
    pub removed_chunks: usize,
    pub uploaded_vertices: usize,
    pub uploaded_indices: usize,
    pub culled_chunks: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderTelemetryHistory {
    frames: usize,
    total_visible_chunks: usize,
    total_uploaded_vertices: usize,
    total_uploaded_indices: usize,
    total_scheduled_jobs: usize,
    peak_dirty_chunks: usize,
    peak_culled_chunks: usize,
}

impl RenderTelemetryHistory {
    pub fn record(&mut self, snapshot: RenderTelemetrySnapshot) {
        self.frames += 1;
        self.total_visible_chunks += snapshot.visible_chunks;
        self.total_uploaded_vertices += snapshot.uploaded_vertices;
        self.total_uploaded_indices += snapshot.uploaded_indices;
        self.total_scheduled_jobs += snapshot.scheduled_jobs;
        self.peak_dirty_chunks = self.peak_dirty_chunks.max(snapshot.dirty_chunks);
        self.peak_culled_chunks = self.peak_culled_chunks.max(snapshot.culled_chunks);
    }

    pub fn snapshot(&self) -> RenderTelemetrySummary {
        let frames = self.frames.max(1);

        RenderTelemetrySummary {
            frames: self.frames,
            average_visible_chunks: self.total_visible_chunks as f32 / frames as f32,
            average_uploaded_vertices: self.total_uploaded_vertices as f32 / frames as f32,
            average_uploaded_indices: self.total_uploaded_indices as f32 / frames as f32,
            average_scheduled_jobs: self.total_scheduled_jobs as f32 / frames as f32,
            peak_dirty_chunks: self.peak_dirty_chunks,
            peak_culled_chunks: self.peak_culled_chunks,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderTelemetrySummary {
    pub frames: usize,
    pub average_visible_chunks: f32,
    pub average_uploaded_vertices: f32,
    pub average_uploaded_indices: f32,
    pub average_scheduled_jobs: f32,
    pub peak_dirty_chunks: usize,
    pub peak_culled_chunks: usize,
}

#[derive(Default)]
struct ChunkMeshScratch {
    vertices: Vec<ChunkMeshVertex>,
    indices: Vec<u32>,
}

impl ChunkMeshScratch {
    fn with_config(config: MeshBuildConfig) -> Self {
        Self {
            vertices: Vec::with_capacity(config.scratch_vertex_capacity),
            indices: Vec::with_capacity(config.scratch_index_capacity),
        }
    }

    fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }
}

pub struct ChunkMeshPipeline {
    config: MeshBuildConfig,
    dirty_chunks: BTreeSet<ChunkPos>,
    chunk_meshes: BTreeMap<ChunkPos, ChunkMesh>,
    scratch: ChunkMeshScratch,
}

impl Default for ChunkMeshPipeline {
    fn default() -> Self {
        Self::with_config(MeshBuildConfig::default())
    }
}

impl ChunkMeshPipeline {
    pub fn with_config(config: MeshBuildConfig) -> Self {
        Self {
            config,
            dirty_chunks: BTreeSet::new(),
            chunk_meshes: BTreeMap::new(),
            scratch: ChunkMeshScratch::with_config(config),
        }
    }

    pub fn config(&self) -> MeshBuildConfig {
        self.config
    }

    pub fn dirty_chunk_count(&self) -> usize {
        self.dirty_chunks.len()
    }

    pub fn cached_mesh_count(&self) -> usize {
        self.chunk_meshes.len()
    }

    pub fn mark_dirty<I>(&mut self, chunks: I)
    where
        I: IntoIterator<Item = ChunkPos>,
    {
        self.dirty_chunks.extend(chunks);
    }

    pub fn apply_world_changes(&mut self, changes: &WorldChangeSummary) {
        self.mark_dirty(changes.touched_chunks.iter().copied());
        self.mark_dirty(changes.removed_chunks.iter().copied());
    }

    pub fn mark_all_dirty(&mut self, world: &WorldSnapshot) {
        self.mark_dirty(world.chunk_positions());
    }

    pub fn rebuild_dirty(&mut self, world: &WorldSnapshot) -> MeshRebuildSummary {
        self.rebuild_dirty_budgeted(world).summary
    }

    pub fn rebuild_dirty_budgeted(&mut self, world: &WorldSnapshot) -> MeshBuildCycle {
        let jobs = self.schedule_jobs(world);
        let results = execute_mesh_build_jobs(world, &jobs, &mut self.scratch);
        let mut cycle = self.commit_mesh_build_results(results);
        cycle.scheduled_jobs = jobs;
        cycle.completed_jobs = cycle.scheduled_jobs.len();
        cycle.remaining_dirty_chunks = self.dirty_chunks.len();
        cycle
    }

    pub fn snapshot(&self, world: &WorldSnapshot) -> WorldRenderSnapshot {
        let visible_chunks = sorted_chunk_positions(world);

        WorldRenderSnapshot {
            debug_chunks: build_debug_chunk_draws(world),
            chunk_meshes: visible_chunks
                .iter()
                .filter_map(|chunk| self.chunk_meshes.get(chunk))
                .cloned()
                .collect(),
            visible_chunks,
            debug_entities: Vec::new(),
        }
    }

    pub fn snapshot_for_camera(
        &self,
        world: &WorldSnapshot,
        frame: &RenderFrameSnapshot,
        config: FrustumConfig,
    ) -> WorldRenderSnapshot {
        let visible_chunks = cull_visible_chunks(world, frame.camera, config);
        let visible_chunk_set = visible_chunks.iter().copied().collect::<BTreeSet<_>>();

        WorldRenderSnapshot {
            debug_chunks: build_debug_chunk_draws(world)
                .into_iter()
                .filter(|draw| visible_chunk_set.contains(&draw.chunk))
                .collect(),
            chunk_meshes: visible_chunks
                .iter()
                .filter_map(|chunk| self.chunk_meshes.get(chunk))
                .cloned()
                .collect(),
            visible_chunks,
            debug_entities: frame.debug_entities.clone(),
        }
    }

    pub fn telemetry_snapshot(
        &self,
        world: &WorldSnapshot,
        snapshot: &WorldRenderSnapshot,
        cycle: &MeshBuildCycle,
    ) -> RenderTelemetrySnapshot {
        RenderTelemetrySnapshot {
            visible_chunks: snapshot.visible_chunks.len(),
            cached_meshes: self.chunk_meshes.len(),
            dirty_chunks: self.dirty_chunks.len(),
            scheduled_jobs: cycle.scheduled_jobs.len(),
            rebuilt_chunks: cycle.summary.rebuilt_chunks.len(),
            removed_chunks: cycle.summary.removed_chunks.len(),
            uploaded_vertices: cycle.uploaded_vertices,
            uploaded_indices: cycle.uploaded_indices,
            culled_chunks: world
                .metrics()
                .loaded_chunks
                .saturating_sub(snapshot.visible_chunks.len()),
        }
    }

    fn schedule_jobs(&mut self, world: &WorldSnapshot) -> Vec<MeshBuildJob> {
        let budget = self.config.max_jobs_per_frame.max(1);
        let scheduled = self
            .dirty_chunks
            .iter()
            .copied()
            .take(budget)
            .collect::<Vec<_>>();

        for chunk in &scheduled {
            self.dirty_chunks.remove(chunk);
        }

        scheduled
            .into_iter()
            .map(|chunk| MeshBuildJob {
                chunk,
                action: if world.chunk(chunk).is_some() {
                    MeshBuildAction::Rebuild
                } else {
                    MeshBuildAction::Remove
                },
            })
            .collect()
    }

    fn commit_mesh_build_results(&mut self, results: Vec<MeshBuildResult>) -> MeshBuildCycle {
        let mut cycle = MeshBuildCycle::default();

        for result in results {
            match result.mesh {
                Some(mesh) => {
                    cycle.summary.rebuilt_chunks.push(result.chunk);
                    cycle.summary.uploads.push(ChunkMeshUpload {
                        chunk: result.chunk,
                        vertex_count: mesh.vertices.len(),
                        index_count: mesh.indices.len(),
                    });
                    cycle.uploaded_vertices += mesh.vertices.len();
                    cycle.uploaded_indices += mesh.indices.len();
                    self.chunk_meshes.insert(result.chunk, mesh);
                }
                None => {
                    if self.chunk_meshes.remove(&result.chunk).is_some() {
                        cycle.summary.removed_chunks.push(result.chunk);
                    }
                }
            }
        }

        cycle
    }
}

pub fn build_debug_chunk_draws(world: &WorldSnapshot) -> Vec<DebugChunkDraw> {
    let mut draws = world
        .chunks()
        .map(|column| DebugChunkDraw {
            chunk: column.pos(),
            bounds: DebugChunkBounds {
                min: [
                    column.pos().min_block_x() as f32,
                    0.0,
                    column.pos().min_block_z() as f32,
                ],
                max: [
                    (column.pos().min_block_x() + 16) as f32,
                    256.0,
                    (column.pos().min_block_z() + 16) as f32,
                ],
            },
            section_mask: column.section_mask(),
            non_air_blocks: column.non_air_blocks(),
        })
        .collect::<Vec<_>>();
    draws.sort_by_key(|draw| draw.chunk);
    draws
}

pub fn build_world_render_snapshot(world: &WorldSnapshot) -> WorldRenderSnapshot {
    let chunk_positions = sorted_chunk_positions(world);

    WorldRenderSnapshot {
        visible_chunks: chunk_positions.clone(),
        debug_chunks: build_debug_chunk_draws(world),
        chunk_meshes: chunk_positions
            .into_iter()
            .map(|chunk| build_chunk_mesh(world, chunk))
            .collect(),
        debug_entities: Vec::new(),
    }
}

pub fn build_debug_entity_draws(frame: &RenderFrameSnapshot) -> Vec<DebugEntityDraw> {
    frame.debug_entities.clone()
}

pub fn cull_visible_chunks(
    world: &WorldSnapshot,
    camera: CameraViewSnapshot,
    config: FrustumConfig,
) -> Vec<ChunkPos> {
    let mut visible = world
        .chunk_positions()
        .filter(|chunk| is_chunk_visible(*chunk, camera, config))
        .collect::<Vec<_>>();
    visible.sort();
    visible
}

fn execute_mesh_build_jobs(
    world: &WorldSnapshot,
    jobs: &[MeshBuildJob],
    scratch: &mut ChunkMeshScratch,
) -> Vec<MeshBuildResult> {
    jobs.iter()
        .map(|job| match job.action {
            MeshBuildAction::Rebuild => MeshBuildResult {
                chunk: job.chunk,
                mesh: Some(build_chunk_mesh_with_scratch(world, job.chunk, scratch)),
            },
            MeshBuildAction::Remove => MeshBuildResult {
                chunk: job.chunk,
                mesh: None,
            },
        })
        .collect()
}

pub fn build_chunk_mesh(world: &WorldSnapshot, chunk: ChunkPos) -> ChunkMesh {
    let mut scratch = ChunkMeshScratch::default();
    build_chunk_mesh_with_scratch(world, chunk, &mut scratch)
}

fn build_chunk_mesh_with_scratch(
    world: &WorldSnapshot,
    chunk: ChunkPos,
    scratch: &mut ChunkMeshScratch,
) -> ChunkMesh {
    let Some(column) = world.chunk(chunk) else {
        return ChunkMesh {
            chunk,
            vertices: Vec::new(),
            indices: Vec::new(),
            visible_faces: 0,
        };
    };

    scratch.clear();
    let mut visible_faces = 0usize;

    for (section_index, section) in column.sections() {
        let section_base_y = (section_index as i32) * 16;

        for local_y in 0..16u8 {
            for local_z in 0..16u8 {
                for local_x in 0..16u8 {
                    let block_state_id = section.block_state_at(local_x, local_y, local_z);

                    if block_state_id == AIR_BLOCK_STATE_ID {
                        continue;
                    }

                    let block_pos = BlockPos::new(
                        chunk.min_block_x() + i32::from(local_x),
                        section_base_y + i32::from(local_y),
                        chunk.min_block_z() + i32::from(local_z),
                    );
                    let block_light = section.block_light_at(local_x, local_y, local_z);
                    let sky_light = section.sky_light_at(local_x, local_y, local_z);

                    for face in FACE_ORDER {
                        let neighbor = block_neighbor(block_pos, face);

                        if world.block_state_or_air(neighbor) != AIR_BLOCK_STATE_ID {
                            continue;
                        }

                        push_face(
                            &mut scratch.vertices,
                            &mut scratch.indices,
                            block_pos,
                            face,
                            block_state_id,
                            block_light,
                            sky_light,
                        );
                        visible_faces += 1;
                    }
                }
            }
        }
    }

    ChunkMesh {
        chunk,
        vertices: scratch.vertices.clone(),
        indices: scratch.indices.clone(),
        visible_faces,
    }
}

const FACE_ORDER: [ChunkFace; 6] = [
    ChunkFace::Down,
    ChunkFace::Up,
    ChunkFace::North,
    ChunkFace::South,
    ChunkFace::West,
    ChunkFace::East,
];

fn build_player_debug_draws(
    simulation: &SimulationSnapshot,
    interpolated_position: Vec3,
) -> Vec<DebugEntityDraw> {
    vec![
        debug_player_draw(
            DebugEntityKind::LocalPlayer,
            interpolated_position,
            simulation.velocity,
            simulation.player.on_ground,
        ),
        debug_player_draw(
            DebugEntityKind::NetworkGhost,
            simulation.network.position,
            simulation.network.velocity,
            simulation.network.on_ground,
        ),
    ]
}

fn debug_player_draw(
    kind: DebugEntityKind,
    position: Vec3,
    velocity: Vec3,
    on_ground: bool,
) -> DebugEntityDraw {
    let half_width = 0.3f32;
    let height = 1.8f32;
    let x = position.x as f32;
    let y = position.y as f32;
    let z = position.z as f32;

    DebugEntityDraw {
        kind,
        position: [x, y, z],
        velocity: [velocity.x as f32, velocity.y as f32, velocity.z as f32],
        on_ground,
        bounds: DebugEntityBounds {
            min: [x - half_width, y, z - half_width],
            max: [x + half_width, y + height, z + half_width],
        },
    }
}

fn sorted_chunk_positions(world: &WorldSnapshot) -> Vec<ChunkPos> {
    let mut chunk_positions = world.chunk_positions().collect::<Vec<_>>();
    chunk_positions.sort();
    chunk_positions
}

fn is_chunk_visible(chunk: ChunkPos, camera: CameraViewSnapshot, config: FrustumConfig) -> bool {
    let bounds_min = [chunk.min_block_x() as f32, 0.0, chunk.min_block_z() as f32];
    let bounds_max = [
        bounds_min[0] + CHUNK_EDGE as f32,
        CHUNK_HEIGHT as f32,
        bounds_min[2] + CHUNK_EDGE as f32,
    ];
    let center = [
        (bounds_min[0] + bounds_max[0]) * 0.5,
        (bounds_min[1] + bounds_max[1]) * 0.5,
        (bounds_min[2] + bounds_max[2]) * 0.5,
    ];
    let extents = [
        (bounds_max[0] - bounds_min[0]) * 0.5,
        (bounds_max[1] - bounds_min[1]) * 0.5,
        (bounds_max[2] - bounds_min[2]) * 0.5,
    ];
    let camera_position = [
        camera.position.x as f32,
        camera.position.y as f32,
        camera.position.z as f32,
    ];
    let to_center = [
        center[0] - camera_position[0],
        center[1] - camera_position[1],
        center[2] - camera_position[2],
    ];
    let (forward, right, up) = camera_basis(camera);
    let forward_distance = dot(to_center, forward);
    let right_distance = dot(to_center, right);
    let up_distance = dot(to_center, up);
    let forward_radius = projected_radius(extents, forward);
    let right_radius = projected_radius(extents, right);
    let up_radius = projected_radius(extents, up);
    let max_distance =
        f32::from(config.view_distance_chunks) * CHUNK_EDGE as f32 + config.padding_blocks;

    if forward_distance + forward_radius < config.near_plane {
        return false;
    }

    if forward_distance - forward_radius > max_distance {
        return false;
    }

    let horizontal_limit = (forward_distance.max(0.0) + forward_radius + config.padding_blocks)
        * (config.horizontal_fov_degrees.to_radians() * 0.5).tan();
    if right_distance.abs() - right_radius > horizontal_limit {
        return false;
    }

    let vertical_limit = (forward_distance.max(0.0) + forward_radius + config.padding_blocks)
        * (config.vertical_fov_degrees.to_radians() * 0.5).tan();
    up_distance.abs() - up_radius <= vertical_limit
}

fn camera_basis(camera: CameraViewSnapshot) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let yaw = camera.yaw.to_radians();
    let pitch = camera.pitch.to_radians();
    let pitch_cos = pitch.cos();
    let forward = normalize([-yaw.sin() * pitch_cos, -pitch.sin(), yaw.cos() * pitch_cos]);
    let mut right = [forward[2], 0.0, -forward[0]];
    if dot(right, right) <= f32::EPSILON {
        right = [1.0, 0.0, 0.0];
    } else {
        right = normalize(right);
    }
    let up = normalize(cross(right, forward));
    (forward, right, up)
}

fn projected_radius(extents: [f32; 3], axis: [f32; 3]) -> f32 {
    extents[0] * axis[0].abs() + extents[1] * axis[1].abs() + extents[2] * axis[2].abs()
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = dot(vector, vector).sqrt();
    if length <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    }
}

fn block_neighbor(pos: BlockPos, face: ChunkFace) -> BlockPos {
    match face {
        ChunkFace::Down => BlockPos::new(pos.x, pos.y - 1, pos.z),
        ChunkFace::Up => BlockPos::new(pos.x, pos.y + 1, pos.z),
        ChunkFace::North => BlockPos::new(pos.x, pos.y, pos.z - 1),
        ChunkFace::South => BlockPos::new(pos.x, pos.y, pos.z + 1),
        ChunkFace::West => BlockPos::new(pos.x - 1, pos.y, pos.z),
        ChunkFace::East => BlockPos::new(pos.x + 1, pos.y, pos.z),
    }
}

fn push_face(
    vertices: &mut Vec<ChunkMeshVertex>,
    indices: &mut Vec<u32>,
    pos: BlockPos,
    face: ChunkFace,
    block_state_id: BlockStateId,
    block_light: u8,
    sky_light: u8,
) {
    let (corners, uv, normal) = match face {
        ChunkFace::Down => (
            [
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            [0.0, -1.0, 0.0],
        ),
        ChunkFace::Up => (
            [
                [0.0, 1.0, 0.0],
                [0.0, 1.0, 1.0],
                [1.0, 1.0, 1.0],
                [1.0, 1.0, 0.0],
            ],
            [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [0.0, 1.0, 0.0],
        ),
        ChunkFace::North => (
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
            ],
            [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [0.0, 0.0, -1.0],
        ),
        ChunkFace::South => (
            [
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
            ],
            [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
            [0.0, 0.0, 1.0],
        ),
        ChunkFace::West => (
            [
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                [0.0, 1.0, 1.0],
                [0.0, 1.0, 0.0],
            ],
            [[1.0, 1.0], [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
            [-1.0, 0.0, 0.0],
        ),
        ChunkFace::East => (
            [
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 1.0, 1.0],
                [1.0, 0.0, 1.0],
            ],
            [[0.0, 1.0], [0.0, 0.0], [1.0, 0.0], [1.0, 1.0]],
            [1.0, 0.0, 0.0],
        ),
    };

    let base_index = vertices.len() as u32;

    for (corner, uv) in corners.into_iter().zip(uv) {
        vertices.push(ChunkMeshVertex {
            position: [
                pos.x as f32 + corner[0],
                pos.y as f32 + corner[1],
                pos.z as f32 + corner[2],
            ],
            normal,
            uv,
            face,
            block_state_id,
            block_light,
            sky_light,
        });
    }

    indices.extend_from_slice(&[
        base_index,
        base_index + 1,
        base_index + 2,
        base_index,
        base_index + 2,
        base_index + 3,
    ]);
}

#[cfg(test)]
mod tests {
    use super::{
        build_chunk_mesh, build_debug_chunk_draws, cull_visible_chunks, ChunkMeshPipeline,
        DebugEntityKind, FrustumConfig, MeshBuildConfig, RenderFrameSnapshot,
        RenderTelemetryHistory,
    };
    use rmc_game::camera::CameraState;
    use rmc_game::player::{LocalPlayerState, Vec3};
    use rmc_game::simulation::{AuthoritativePlayerState, InterpolationState, SimulationSnapshot};
    use rmc_net::codec::play::{ChunkDataPacket, CHUNK_BIOME_BYTES};
    use rmc_ui::MinimalHud;
    use rmc_world::{WorldConfig, WorldSnapshot};

    fn single_block_chunk(blocks: &[(usize, u16)]) -> ChunkDataPacket {
        single_block_chunk_at(0, 0, blocks)
    }

    fn single_block_chunk_at(
        chunk_x: i32,
        chunk_z: i32,
        blocks: &[(usize, u16)],
    ) -> ChunkDataPacket {
        let mut data = Vec::new();
        let mut states = vec![0u16; 16 * 16 * 16];

        for (index, state) in blocks {
            states[*index] = *state;
        }

        for state in states {
            data.extend_from_slice(&state.to_le_bytes());
        }

        data.extend_from_slice(&vec![0u8; 16 * 16 * 16 / 2]);
        data.extend_from_slice(&vec![15u8; 16 * 16 * 16 / 2]);
        data.extend_from_slice(&[0u8; CHUNK_BIOME_BYTES]);

        ChunkDataPacket {
            chunk_x,
            chunk_z,
            full_chunk: true,
            primary_bitmask: 0x0001,
            data,
        }
    }

    #[test]
    fn builds_debug_draws_from_loaded_chunks() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&single_block_chunk(&[(0, 1)]))
            .expect("chunk data should apply");

        let draws = build_debug_chunk_draws(&world);
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].non_air_blocks, 1);
    }

    #[test]
    fn meshes_single_block_as_six_faces() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&single_block_chunk(&[(0, 1)]))
            .expect("chunk data should apply");

        let mesh = build_chunk_mesh(&world, rmc_world::ChunkPos::new(0, 0));
        assert_eq!(mesh.visible_faces, 6);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn culls_shared_faces_between_adjacent_blocks() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&single_block_chunk(&[(0, 1), (1, 1)]))
            .expect("chunk data should apply");

        let mesh = build_chunk_mesh(&world, rmc_world::ChunkPos::new(0, 0));
        assert_eq!(mesh.visible_faces, 10);
    }

    #[test]
    fn rebuilds_only_dirty_chunks() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        let change = world
            .apply_chunk_data(&single_block_chunk(&[(0, 1)]))
            .expect("chunk data should apply");

        let mut pipeline = ChunkMeshPipeline::default();
        pipeline.apply_world_changes(&change);
        let summary = pipeline.rebuild_dirty(&world);

        assert_eq!(summary.rebuilt_chunks, vec![rmc_world::ChunkPos::new(0, 0)]);
        assert_eq!(pipeline.snapshot(&world).chunk_meshes.len(), 1);
    }

    #[test]
    fn respects_mesh_job_budget() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&single_block_chunk_at(0, 0, &[(0, 1)]))
            .expect("chunk data should apply");
        world
            .apply_chunk_data(&single_block_chunk_at(1, 0, &[(0, 1)]))
            .expect("chunk data should apply");

        let mut pipeline = ChunkMeshPipeline::with_config(MeshBuildConfig {
            max_jobs_per_frame: 1,
            ..MeshBuildConfig::default()
        });
        pipeline.mark_all_dirty(&world);

        let first = pipeline.rebuild_dirty_budgeted(&world);
        assert_eq!(first.scheduled_jobs.len(), 1);
        assert_eq!(first.remaining_dirty_chunks, 1);

        let second = pipeline.rebuild_dirty_budgeted(&world);
        assert_eq!(second.scheduled_jobs.len(), 1);
        assert_eq!(second.remaining_dirty_chunks, 0);
    }

    #[test]
    fn accumulates_render_telemetry() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&single_block_chunk(&[(0, 1)]))
            .expect("chunk data should apply");

        let mut pipeline = ChunkMeshPipeline::default();
        pipeline.mark_all_dirty(&world);
        let cycle = pipeline.rebuild_dirty_budgeted(&world);
        let snapshot = pipeline.snapshot(&world);
        let telemetry = pipeline.telemetry_snapshot(&world, &snapshot, &cycle);

        let mut history = RenderTelemetryHistory::default();
        history.record(telemetry);
        let summary = history.snapshot();

        assert_eq!(summary.frames, 1);
        assert!(summary.average_visible_chunks >= 1.0);
        assert!(summary.average_uploaded_vertices > 0.0);
    }

    #[test]
    fn culls_chunks_behind_the_camera() {
        let mut world = WorldSnapshot::new(WorldConfig::overworld());
        world
            .apply_chunk_data(&single_block_chunk_at(0, 4, &[(0, 1)]))
            .expect("front chunk should load");
        world
            .apply_chunk_data(&single_block_chunk_at(0, -4, &[(0, 1)]))
            .expect("rear chunk should load");

        let visible = cull_visible_chunks(
            &world,
            super::CameraViewSnapshot {
                position: Vec3::new(8.0, 64.0, 8.0),
                yaw: 0.0,
                pitch: 0.0,
                mouse_captured: true,
            },
            FrustumConfig::debug_default(),
        );

        assert!(visible.contains(&rmc_world::ChunkPos::new(0, 4)));
        assert!(!visible.contains(&rmc_world::ChunkPos::new(0, -4)));
    }

    #[test]
    fn render_frame_exposes_local_and_network_debug_entities() {
        let simulation = SimulationSnapshot {
            network: AuthoritativePlayerState {
                position: Vec3::new(6.0, 64.0, 6.0),
                velocity: Vec3::new(0.0, 0.0, 0.1),
                on_ground: true,
            },
            player: LocalPlayerState {
                position: Vec3::new(4.0, 64.0, 4.0),
                on_ground: true,
                sprinting: true,
                sneaking: false,
                selected_hotbar_slot: 0,
            },
            velocity: Vec3::new(0.1, 0.0, 0.0),
            interpolation: InterpolationState {
                previous_position: Vec3::new(4.0, 64.0, 4.0),
                current_position: Vec3::new(5.0, 64.0, 4.0),
            },
            sprint_reset_ticks: 0,
        };

        let frame = RenderFrameSnapshot::from_simulation_state(
            12,
            0.5,
            &simulation,
            CameraState::default(),
            MinimalHud::vanilla(0),
        );

        assert_eq!(frame.debug_entities.len(), 2);
        assert_eq!(frame.debug_entities[0].kind, DebugEntityKind::LocalPlayer);
        assert_eq!(frame.debug_entities[1].kind, DebugEntityKind::NetworkGhost);
        assert_eq!(frame.debug_entities[0].position, [4.5, 64.0, 4.0]);
        assert!((frame.camera.position.y - (64.0 + f64::from(1.62_f32))).abs() < 1.0e-9);
    }
}
