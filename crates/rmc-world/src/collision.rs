//! Terrain queries derived from MCP919 block bounds and AxisAlignedBB behavior.
//! Coordinates are in world space; queries never synthesize a floor in unloaded terrain.
use crate::{BlockPos, WorldSnapshot};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Aabb {
    pub const fn new(min: [f64; 3], max: [f64; 3]) -> Self {
        Self { min, max }
    }

    pub fn offset(self, delta: [f64; 3]) -> Self {
        Self::new(
            std::array::from_fn(|i| self.min[i] + delta[i]),
            std::array::from_fn(|i| self.max[i] + delta[i]),
        )
    }

    pub fn swept(self, delta: [f64; 3]) -> Self {
        Self::new(
            std::array::from_fn(|i| self.min[i] + delta[i].min(0.0)),
            std::array::from_fn(|i| self.max[i] + delta[i].max(0.0)),
        )
    }

    pub fn intersects(self, other: Self) -> bool {
        (0..3).all(|i| self.max[i] > other.min[i] && self.min[i] < other.max[i])
    }

    pub fn clip_axis(self, moving: Self, axis: usize, delta: f64) -> f64 {
        if (0..3)
            .any(|i| i != axis && (moving.max[i] <= self.min[i] || moving.min[i] >= self.max[i]))
        {
            return delta;
        }
        if delta > 0.0 && moving.max[axis] <= self.min[axis] {
            delta.min(self.min[axis] - moving.max[axis])
        } else if delta < 0.0 && moving.min[axis] >= self.max[axis] {
            delta.max(self.max[axis] - moving.min[axis])
        } else {
            delta
        }
    }

    pub fn ray_hit(self, origin: [f64; 3], direction: [f64; 3], reach: f64) -> Option<(f64, u8)> {
        let mut near = 0.0_f64;
        let mut far = reach;
        let mut face = 0;
        for axis in 0..3 {
            if direction[axis].abs() < 1.0e-12 {
                if origin[axis] < self.min[axis] || origin[axis] > self.max[axis] {
                    return None;
                }
                continue;
            }
            let a = (self.min[axis] - origin[axis]) / direction[axis];
            let b = (self.max[axis] - origin[axis]) / direction[axis];
            let entry = a.min(b);
            if entry > near {
                near = entry;
                face = match (axis, direction[axis] > 0.0) {
                    (0, true) => 4,
                    (0, false) => 5,
                    (1, true) => 0,
                    (1, false) => 1,
                    (2, true) => 2,
                    _ => 3,
                };
            }
            far = far.min(a.max(b));
            if near > far {
                return None;
            }
        }
        (far >= 0.0 && near <= reach).then_some((near, face))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockHit {
    pub position: BlockPos,
    pub distance: f64,
    /// Protocol 47 face index: down, up, north, south, west, east.
    pub face: u8,
    pub point: [f64; 3],
}

impl WorldSnapshot {
    pub fn collision_boxes(&self, area: Aabb) -> Vec<Aabb> {
        let mut result = Vec::new();
        for x in area.min[0].floor() as i32..=area.max[0].floor() as i32 {
            for y in (area.min[1].floor() as i32 - 1).max(0)..=(area.max[1].floor() as i32).min(255)
            {
                for z in area.min[2].floor() as i32..=area.max[2].floor() as i32 {
                    for bounds in self.block_collision_boxes(BlockPos::new(x, y, z)) {
                        if bounds.intersects(area) {
                            result.push(bounds);
                        }
                    }
                }
            }
        }
        result
    }

    pub fn slipperiness_at(&self, position: BlockPos) -> f32 {
        match self.block_state_or_air(position) >> 4 {
            79 | 174 => 0.98,
            165 => 0.8,
            _ => 0.6,
        }
    }

    pub fn block_collision_boxes(&self, pos: BlockPos) -> Vec<Aabb> {
        let state = self.block_state_or_air(pos);
        let id = state >> 4;
        let meta = state & 15;
        let box_at = |min, max| {
            Aabb::new(min, max).offset([f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)])
        };
        let bounds = match id {
            // Fluids, flowers, rails, fire, redstone, crops and portals have no collision.
            0
            | 6
            | 8..=11
            | 27
            | 28
            | 30..=32
            | 36..=40
            | 50
            | 51
            | 55
            | 59
            | 63
            | 66
            | 68
            | 69
            | 70
            | 72
            | 75..=77
            | 83
            | 90
            | 104..=106
            | 115
            | 119
            | 131
            | 132
            | 141..=143
            | 147
            | 148
            | 157
            | 175
            | 176
            | 177 => return Vec::new(),
            44 | 126 | 182 => {
                if meta & 8 != 0 {
                    ([0.0, 0.5, 0.0], [1.0, 1.0, 1.0])
                } else {
                    ([0.0, 0.0, 0.0], [1.0, 0.5, 1.0])
                }
            }
            29 | 33 if meta & 8 != 0 => match meta & 7 {
                0 => ([0.0, 0.25, 0.0], [1.0, 1.0, 1.0]),
                1 => ([0.0, 0.0, 0.0], [1.0, 0.75, 1.0]),
                2 => ([0.0, 0.0, 0.25], [1.0, 1.0, 1.0]),
                3 => ([0.0, 0.0, 0.0], [1.0, 1.0, 0.75]),
                4 => ([0.25, 0.0, 0.0], [1.0, 1.0, 1.0]),
                _ => ([0.0, 0.0, 0.0], [0.75, 1.0, 1.0]),
            },
            34 => {
                let (head_min, head_max, rod_min, rod_max) = match meta & 7 {
                    0 => (
                        [0.0, 0.0, 0.0],
                        [1.0, 0.25, 1.0],
                        [0.375, 0.25, 0.375],
                        [0.625, 1.0, 0.625],
                    ),
                    1 => (
                        [0.0, 0.75, 0.0],
                        [1.0, 1.0, 1.0],
                        [0.375, 0.0, 0.375],
                        [0.625, 0.75, 0.625],
                    ),
                    2 => (
                        [0.0, 0.0, 0.0],
                        [1.0, 1.0, 0.25],
                        [0.25, 0.375, 0.25],
                        [0.75, 0.625, 1.0],
                    ),
                    3 => (
                        [0.0, 0.0, 0.75],
                        [1.0, 1.0, 1.0],
                        [0.25, 0.375, 0.0],
                        [0.75, 0.625, 0.75],
                    ),
                    4 => (
                        [0.0, 0.0, 0.0],
                        [0.25, 1.0, 1.0],
                        [0.375, 0.25, 0.25],
                        [0.625, 0.75, 1.0],
                    ),
                    _ => (
                        [0.75, 0.0, 0.0],
                        [1.0, 1.0, 1.0],
                        [0.0, 0.375, 0.25],
                        [0.75, 0.625, 0.75],
                    ),
                };
                return vec![box_at(head_min, head_max), box_at(rod_min, rod_max)];
            }
            93 | 94 | 149 | 150 => ([0.0, 0.0, 0.0], [1.0, 0.125, 1.0]),
            122 => ([0.0625, 0.0, 0.0625], [0.9375, 1.0, 0.9375]),
            127 => {
                let age = (meta >> 2).min(2);
                let size = f64::from(4 + age * 2);
                let min = (8.0 - size * 0.5) / 16.0;
                let max = (8.0 + size * 0.5) / 16.0;
                let bottom = f64::from(7 - age * 2) / 16.0;
                match meta & 3 {
                    0 => ([min, bottom, (15.0 - size) / 16.0], [max, 0.75, 0.9375]),
                    1 => ([0.0625, bottom, min], [(1.0 + size) / 16.0, 0.75, max]),
                    2 => ([min, bottom, 0.0625], [max, 0.75, (1.0 + size) / 16.0]),
                    _ => ([(15.0 - size) / 16.0, bottom, min], [0.9375, 0.75, max]),
                }
            }
            140 => ([0.3125, 0.0, 0.3125], [0.6875, 0.375, 0.6875]),
            144 => match meta & 7 {
                2 => ([0.25, 0.25, 0.5], [0.75, 0.75, 1.0]),
                3 => ([0.25, 0.25, 0.0], [0.75, 0.75, 0.5]),
                4 => ([0.5, 0.25, 0.25], [1.0, 0.75, 0.75]),
                5 => ([0.0, 0.25, 0.25], [0.5, 0.75, 0.75]),
                _ => ([0.25, 0.0, 0.25], [0.75, 0.5, 0.75]),
            },
            26 => ([0.0, 0.0, 0.0], [1.0, 0.5625, 1.0]),
            60 => ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
            78 => ([0.0, 0.0, 0.0], [1.0, f64::from(meta & 7) / 8.0, 1.0]),
            88 => ([0.0, 0.0, 0.0], [1.0, 0.875, 1.0]),
            81 => ([0.0625, 0.0, 0.0625], [0.9375, 0.9375, 0.9375]),
            54 | 146 => {
                let adjacent = |dx, dz| {
                    self.block_state_or_air(BlockPos::new(pos.x + dx, pos.y, pos.z + dz)) >> 4 == id
                };
                (
                    [
                        if adjacent(-1, 0) { 0.0 } else { 0.0625 },
                        0.0,
                        if adjacent(0, -1) { 0.0 } else { 0.0625 },
                    ],
                    [
                        if adjacent(1, 0) { 1.0 } else { 0.9375 },
                        0.875,
                        if adjacent(0, 1) { 1.0 } else { 0.9375 },
                    ],
                )
            }
            130 => ([0.0625, 0.0, 0.0625], [0.9375, 0.875, 0.9375]),
            64 | 71 | 193..=197 => {
                let lower_pos = if meta & 8 != 0 {
                    BlockPos::new(pos.x, pos.y - 1, pos.z)
                } else {
                    pos
                };
                let lower = self.block_state_or_air(lower_pos) & 15;
                let upper = self.block_state_or_air(BlockPos::new(
                    lower_pos.x,
                    lower_pos.y + 1,
                    lower_pos.z,
                )) & 15;
                let mut face = lower & 3;
                if lower & 4 != 0 {
                    face = (face + if upper & 1 != 0 { 3 } else { 1 }) & 3;
                }
                match face {
                    0 => ([0.0, 0.0, 0.0], [0.1875, 1.0, 1.0]),
                    1 => ([0.0, 0.0, 0.0], [1.0, 1.0, 0.1875]),
                    2 => ([0.8125, 0.0, 0.0], [1.0, 1.0, 1.0]),
                    _ => ([0.0, 0.0, 0.8125], [1.0, 1.0, 1.0]),
                }
            }
            96 | 167 => {
                if meta & 4 == 0 {
                    if meta & 8 != 0 {
                        ([0.0, 0.8125, 0.0], [1.0, 1.0, 1.0])
                    } else {
                        ([0.0, 0.0, 0.0], [1.0, 0.1875, 1.0])
                    }
                } else {
                    match meta & 3 {
                        0 => ([0.0, 0.0, 0.8125], [1.0, 1.0, 1.0]),
                        1 => ([0.0, 0.0, 0.0], [1.0, 1.0, 0.1875]),
                        2 => ([0.8125, 0.0, 0.0], [1.0, 1.0, 1.0]),
                        _ => ([0.0, 0.0, 0.0], [0.1875, 1.0, 1.0]),
                    }
                }
            }
            101 | 102 | 160 => {
                let connects = |dx, dz| {
                    let neighbor =
                        self.block_state_or_air(BlockPos::new(pos.x + dx, pos.y, pos.z + dz)) >> 4;
                    matches!(neighbor, 20 | 95 | 101 | 102 | 160) || is_full_cube(neighbor)
                };
                let (west, east, north, south) = (
                    connects(-1, 0),
                    connects(1, 0),
                    connects(0, -1),
                    connects(0, 1),
                );
                let isolated = !(west || east || north || south);
                let mut boxes = Vec::new();
                if west || east || isolated {
                    boxes.push(box_at(
                        [if west || isolated { 0.0 } else { 0.5 }, 0.0, 0.4375],
                        [if east || isolated { 1.0 } else { 0.5 }, 1.0, 0.5625],
                    ));
                }
                if north || south || isolated {
                    boxes.push(box_at(
                        [0.4375, 0.0, if north || isolated { 0.0 } else { 0.5 }],
                        [0.5625, 1.0, if south || isolated { 1.0 } else { 0.5 }],
                    ));
                }
                return boxes;
            }
            139 => {
                let connects = |dx, dz| {
                    let neighbor =
                        self.block_state_or_air(BlockPos::new(pos.x + dx, pos.y, pos.z + dz)) >> 4;
                    neighbor == 139 || matches!(neighbor, 107 | 183..=187) || is_full_cube(neighbor)
                };
                let (west, east, north, south) = (
                    connects(-1, 0),
                    connects(1, 0),
                    connects(0, -1),
                    connects(0, 1),
                );
                let mut min = [
                    if west { 0.0 } else { 0.25 },
                    0.0,
                    if north { 0.0 } else { 0.25 },
                ];
                let mut max = [
                    if east { 1.0 } else { 0.75 },
                    1.5,
                    if south { 1.0 } else { 0.75 },
                ];
                if north && south && !west && !east {
                    min[0] = 0.3125;
                    max[0] = 0.6875;
                }
                if west && east && !north && !south {
                    min[2] = 0.3125;
                    max[2] = 0.6875;
                }
                (min, max)
            }
            154 => {
                return vec![
                    box_at([0.0, 0.0, 0.0], [1.0, 0.625, 1.0]),
                    box_at([0.0, 0.0, 0.0], [0.125, 1.0, 1.0]),
                    box_at([0.0, 0.0, 0.0], [1.0, 1.0, 0.125]),
                    box_at([0.875, 0.0, 0.0], [1.0, 1.0, 1.0]),
                    box_at([0.0, 0.0, 0.875], [1.0, 1.0, 1.0]),
                ]
            }
            92 => (
                [f64::from(1 + 2 * meta.min(6)) / 16.0, 0.0, 0.0625],
                [0.9375, 0.5, 0.9375],
            ),
            117 => {
                return vec![
                    box_at([0.0, 0.0, 0.0], [1.0, 0.125, 1.0]),
                    box_at([0.4375, 0.0, 0.4375], [0.5625, 0.875, 0.5625]),
                ]
            }
            118 => {
                return vec![
                    box_at([0.0, 0.0, 0.0], [1.0, 0.3125, 1.0]),
                    box_at([0.0, 0.0, 0.0], [0.125, 1.0, 1.0]),
                    box_at([0.875, 0.0, 0.0], [1.0, 1.0, 1.0]),
                    box_at([0.0, 0.0, 0.0], [1.0, 1.0, 0.125]),
                    box_at([0.0, 0.0, 0.875], [1.0, 1.0, 1.0]),
                ]
            }
            145 => {
                if meta & 1 != 0 {
                    ([0.0, 0.0, 0.125], [1.0, 1.0, 0.875])
                } else {
                    ([0.125, 0.0, 0.0], [0.875, 1.0, 1.0])
                }
            }
            120 => {
                let mut boxes = vec![box_at([0.0, 0.0, 0.0], [1.0, 0.8125, 1.0])];
                if meta & 4 != 0 {
                    boxes.push(box_at([0.3125, 0.8125, 0.3125], [0.6875, 1.0, 0.6875]));
                }
                return boxes;
            }

            111 => ([0.0, 0.0, 0.0], [1.0, 0.015625, 1.0]),
            116 => ([0.0, 0.0, 0.0], [1.0, 0.75, 1.0]),
            151 | 178 => ([0.0, 0.0, 0.0], [1.0, 0.375, 1.0]),
            171 => ([0.0, 0.0, 0.0], [1.0, 0.0625, 1.0]),
            65 => match meta % 6 {
                2 => ([0.0, 0.0, 0.875], [1.0, 1.0, 1.0]),
                3 => ([0.0, 0.0, 0.0], [1.0, 1.0, 0.125]),
                4 => ([0.875, 0.0, 0.0], [1.0, 1.0, 1.0]),
                5 => ([0.0, 0.0, 0.0], [0.125, 1.0, 1.0]),
                _ => ([0.0, 0.0, 0.875], [1.0, 1.0, 1.0]),
            },
            107 | 183..=187 => {
                if meta & 4 != 0 {
                    return Vec::new();
                }
                if meta & 1 == 0 {
                    ([0.0, 0.0, 0.375], [1.0, 1.5, 0.625])
                } else {
                    ([0.375, 0.0, 0.0], [0.625, 1.5, 1.0])
                }
            }
            85 | 113 | 188..=192 => {
                let connects = |dx, dz| {
                    let neighbor =
                        self.block_state_or_air(BlockPos::new(pos.x + dx, pos.y, pos.z + dz)) >> 4;
                    neighbor == id || matches!(neighbor, 107 | 183..=187) || is_full_cube(neighbor)
                };
                let min_x = if connects(-1, 0) { 0.0 } else { 0.375 };
                let max_x = if connects(1, 0) { 1.0 } else { 0.625 };
                let min_z = if connects(0, -1) { 0.0 } else { 0.375 };
                let max_z = if connects(0, 1) { 1.0 } else { 0.625 };
                return vec![
                    box_at([min_x, 0.0, 0.375], [max_x, 1.5, 0.625]),
                    box_at([0.375, 0.0, min_z], [0.625, 1.5, max_z]),
                ];
            }
            53 | 67 | 108 | 109 | 114 | 128 | 134..=136 | 156 | 163 | 164 | 180 => {
                let upper = meta & 4 != 0;
                let facing = meta & 3;
                let directions = [(1, 0), (-1, 0), (0, 1), (0, -1)];
                let half = |direction: u16, x: usize, z: usize| match direction {
                    0 => x == 1,
                    1 => x == 0,
                    2 => z == 1,
                    _ => z == 0,
                };
                let state_at =
                    |dx, dz| self.block_state_or_air(BlockPos::new(pos.x + dx, pos.y, pos.z + dz));
                let stair = |state: u16| {
                    matches!(
                        state >> 4,
                        53 | 67 | 108 | 109 | 114 | 128 | 134..=136 | 156 | 163 | 164 | 180
                    ) && (state & 4 == meta & 4)
                };
                let same = |dx, dz| {
                    let state = state_at(dx, dz);
                    stair(state) && state & 3 == facing
                };
                let (dx, dz) = directions[facing as usize];
                let front = state_at(dx, dz);
                let behind = state_at(-dx, -dz);
                let mut occupied = [[false; 2]; 2];
                for x in 0..2 {
                    for z in 0..2 {
                        occupied[x][z] = half(facing, x, z);
                    }
                }
                let perpendicular = |direction: u16| (direction < 2) != (facing < 2);
                let mut outer = false;
                if stair(front) && perpendicular(front & 3) {
                    let direction = front & 3;
                    let (nx, nz) = directions[direction as usize];
                    if !same(-nx, -nz) {
                        for x in 0..2 {
                            for z in 0..2 {
                                occupied[x][z] &= half(direction, x, z);
                            }
                        }
                        outer = true;
                    }
                }
                if !outer && stair(behind) && perpendicular(behind & 3) {
                    let direction = behind & 3;
                    let (nx, nz) = directions[direction as usize];
                    if !same(nx, nz) {
                        for x in 0..2 {
                            for z in 0..2 {
                                occupied[x][z] |= half(direction, x, z);
                            }
                        }
                    }
                }
                let mut boxes = vec![box_at(
                    [0.0, if upper { 0.5 } else { 0.0 }, 0.0],
                    [1.0, if upper { 1.0 } else { 0.5 }, 1.0],
                )];
                for x in 0..2 {
                    for z in 0..2 {
                        if occupied[x][z] {
                            boxes.push(box_at(
                                [
                                    x as f64 * 0.5,
                                    if upper { 0.0 } else { 0.5 },
                                    z as f64 * 0.5,
                                ],
                                [
                                    (x + 1) as f64 * 0.5,
                                    if upper { 0.5 } else { 1.0 },
                                    (z + 1) as f64 * 0.5,
                                ],
                            ));
                        }
                    }
                }
                return boxes;
            }
            _ => ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        };
        if bounds.1[1] <= bounds.0[1] {
            Vec::new()
        } else {
            vec![box_at(bounds.0, bounds.1)]
        }
    }

    /// DDA traversal; only visit crossed cells, then intersect the actual block bounds.
    pub fn raycast(&self, origin: [f64; 3], direction: [f64; 3], reach: f64) -> Option<BlockHit> {
        if !reach.is_finite()
            || !(0.0..=64.0).contains(&reach)
            || origin
                .iter()
                .chain(direction.iter())
                .any(|v| !v.is_finite())
        {
            return None;
        }
        let length = direction.iter().map(|v| v * v).sum::<f64>().sqrt();
        if length < 1.0e-12 {
            return None;
        }
        let direction = direction.map(|v| v / length);
        let mut cell = origin.map(|v| v.floor() as i32);
        let step = direction.map(|v| {
            if v > 0.0 {
                1
            } else if v < 0.0 {
                -1
            } else {
                0
            }
        });
        let mut next = std::array::from_fn::<_, 3, _>(|i| {
            if step[i] == 0 {
                f64::INFINITY
            } else {
                (f64::from(cell[i] + if step[i] > 0 { 1 } else { 0 }) - origin[i]) / direction[i]
            }
        });
        let stride = direction.map(|v| {
            if v == 0.0 {
                f64::INFINITY
            } else {
                (1.0 / v).abs()
            }
        });
        for _ in 0..256 {
            let pos = BlockPos::new(cell[0], cell[1], cell[2]);
            let hit = self
                .block_collision_boxes(pos)
                .into_iter()
                .filter_map(|bounds| bounds.ray_hit(origin, direction, reach))
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((distance, face)) = hit {
                return Some(BlockHit {
                    position: pos,
                    distance,
                    face,
                    point: std::array::from_fn(|i| origin[i] + direction[i] * distance),
                });
            }
            let axis = (0..3).min_by(|&a, &b| next[a].total_cmp(&next[b]))?;
            if next[axis] > reach {
                return None;
            }
            cell[axis] += step[axis];
            next[axis] += stride[axis];
        }
        None
    }
}

fn is_full_cube(id: u16) -> bool {
    matches!(id, 1..=5 | 7 | 12..=25 | 35 | 41..=43 | 45..=49 | 56..=58 | 61 | 62 | 73 | 74 | 79 | 80 | 82 | 86..=89 | 91 | 95 | 97..=103 | 110 | 112 | 121 | 123..=125 | 129 | 133 | 137 | 152 | 153 | 155 | 158..=162 | 165 | 168..=170 | 172..=174 | 179 | 181)
}
