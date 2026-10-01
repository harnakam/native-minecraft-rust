//! Environmental movement queries over received block states.
use crate::{collision::Aabb, BlockPos, WorldSnapshot};

#[derive(Clone, Copy, Debug, Default)]
pub struct MovementEnvironment {
    pub water: bool,
    pub lava: bool,
    pub ladder: bool,
    pub web: bool,
    pub soul_sand_contacts: u8,
    pub water_flow: [f64; 3],
}

impl WorldSnapshot {
    pub fn eye_in_water(&self, eye: [f64; 3]) -> bool {
        let pos = BlockPos::new(
            eye[0].floor() as i32,
            eye[1].floor() as i32,
            eye[2].floor() as i32,
        );
        let state = self.block_state_or_air(pos);
        if !matches!(state >> 4, 8 | 9) {
            return false;
        }
        let level = if state & 15 >= 8 { 0 } else { state & 15 };
        let height = (level as f32 + 1.0) / 9.0 - 0.11111111_f32;
        eye[1] < f64::from((pos.y + 1) as f32 - height)
    }
    pub fn liquid_escape_clear(&self, bounds: Aabb) -> bool {
        if !self.collision_boxes(bounds).is_empty() {
            return false;
        }
        for x in bounds.min[0].floor() as i32..=bounds.max[0].floor() as i32 {
            for y in bounds.min[1].floor() as i32..=bounds.max[1].floor() as i32 {
                for z in bounds.min[2].floor() as i32..=bounds.max[2].floor() as i32 {
                    if matches!(self.block_state_or_air(BlockPos::new(x, y, z)) >> 4, 8..=11) {
                        return false;
                    }
                }
            }
        }
        true
    }
    pub fn movement_environment(&self, bounds: Aabb) -> MovementEnvironment {
        let mut result = MovementEnvironment::default();
        let feet = BlockPos::new(
            ((bounds.min[0] + bounds.max[0]) * 0.5).floor() as i32,
            bounds.min[1].floor() as i32,
            ((bounds.min[2] + bounds.max[2]) * 0.5).floor() as i32,
        );
        result.ladder = matches!(self.block_state_or_air(feet) >> 4, 65 | 106);
        let water_bounds = Aabb::new(
            [
                bounds.min[0] + 0.001,
                bounds.min[1] + f64::from(0.4_f32) + 0.001,
                bounds.min[2] + 0.001,
            ],
            [
                bounds.max[0] - 0.001,
                bounds.max[1] - f64::from(0.4_f32) - 0.001,
                bounds.max[2] - 0.001,
            ],
        );
        let lava_bounds = Aabb::new(
            [
                bounds.min[0] + f64::from(0.1_f32),
                bounds.min[1] + f64::from(0.4_f32),
                bounds.min[2] + f64::from(0.1_f32),
            ],
            [
                bounds.max[0] - f64::from(0.1_f32),
                bounds.max[1] - f64::from(0.4_f32),
                bounds.max[2] - f64::from(0.1_f32),
            ],
        );
        // World.handleMaterialAcceleration / isMaterialInBB use floor(max + 1),
        // including a block exactly at an integer maximum coordinate.
        let scan = |b: Aabb| {
            let min = b.min.map(|v| v.floor() as i32);
            let end = b.max.map(|v| (v + 1.0).floor() as i32);
            (min, end)
        };
        let (min, end) = scan(water_bounds);
        let water_loaded = end[1] >= 0
            && min[1] < 256
            && (min[0] >> 4..=end[0] >> 4).all(|x| {
                (min[2] >> 4..=end[2] >> 4)
                    .all(|z| self.chunk(crate::ChunkPos::new(x, z)).is_some())
            });
        if water_loaded {
            for x in min[0]..end[0] {
                for y in min[1]..end[1] {
                    for z in min[2]..end[2] {
                        let pos = BlockPos::new(x, y, z);
                        let state = self.block_state_or_air(pos);
                        if matches!(state >> 4, 8 | 9) {
                            let level = if state & 15 >= 8 { 0 } else { state & 15 };
                            let surface = f64::from((y + 1) as f32 - (level as f32 + 1.0) / 9.0);
                            if f64::from(end[1]) >= surface {
                                result.water = true;
                                let flow = self.liquid_flow(pos, 8);
                                for axis in 0..3 {
                                    result.water_flow[axis] += flow[axis];
                                }
                            }
                        }
                    }
                }
            }
        }
        let (min, end) = scan(lava_bounds);
        for x in min[0]..end[0] {
            for y in min[1]..end[1] {
                for z in min[2]..end[2] {
                    if matches!(
                        self.block_state_or_air(BlockPos::new(x, y, z)) >> 4,
                        10 | 11
                    ) {
                        result.lava = true;
                    }
                }
            }
        }
        for x in bounds.min[0].floor() as i32..=(bounds.max[0] - 0.001).floor() as i32 {
            for y in bounds.min[1].floor() as i32..=(bounds.max[1] - 0.001).floor() as i32 {
                for z in bounds.min[2].floor() as i32..=(bounds.max[2] - 0.001).floor() as i32 {
                    let pos = BlockPos::new(x, y, z);
                    let state = self.block_state_or_air(pos);
                    match state >> 4 {
                        88 => {
                            result.soul_sand_contacts = result.soul_sand_contacts.saturating_add(1)
                        }
                        30 => result.web = true,
                        _ => {}
                    }
                }
            }
        }
        let length = result.water_flow.iter().map(|v| v * v).sum::<f64>().sqrt();
        if length >= 1.0e-4 {
            for v in &mut result.water_flow {
                *v = *v / length * 0.014;
            }
        }
        if length < 1.0e-4 {
            result.water_flow = [0.0; 3];
        }
        result
    }

    fn liquid_flow(&self, pos: BlockPos, flowing_id: u16) -> [f64; 3] {
        let level = |p| {
            let state = self.block_state_or_air(p);
            if (state >> 4) == flowing_id || (state >> 4) == flowing_id + 1 {
                let meta = (state & 15) as i32;
                if meta >= 8 {
                    0
                } else {
                    meta
                }
            } else {
                -1
            }
        };
        let current = level(pos);
        let mut flow = [0.0; 3];
        let mut falling_wall = false;
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let neighbor = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
            let decay = level(neighbor);
            let blocked = !self.block_collision_boxes(neighbor).is_empty();
            let delta = if decay >= 0 {
                decay - current
            } else if !blocked {
                let below = level(BlockPos::new(neighbor.x, neighbor.y - 1, neighbor.z));
                if below >= 0 {
                    below - (current - 8)
                } else {
                    0
                }
            } else {
                0
            };
            flow[0] += (dx * delta) as f64;
            flow[2] += (dz * delta) as f64;
            falling_wall |= blocked
                || !self
                    .block_collision_boxes(BlockPos::new(neighbor.x, neighbor.y + 1, neighbor.z))
                    .is_empty();
        }
        let normalize = |mut v: [f64; 3]| {
            let length = v.iter().map(|n| n * n).sum::<f64>().sqrt();
            if length >= 1.0e-4 {
                for n in &mut v {
                    *n /= length;
                }
            } else {
                v = [0.0; 3];
            }
            v
        };
        if self.block_state_or_air(pos) & 15 >= 8 && falling_wall {
            flow = normalize(flow);
            flow[1] -= 6.0;
        }
        normalize(flow)
    }
}

#[cfg(test)]
mod fluid_bounds_tests {
    use super::*;
    use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
    fn world(id: u16, meta: u16, all: bool) -> WorldSnapshot {
        let mut world = WorldSnapshot::new(crate::WorldConfig::overworld());
        if all {
            for x in -1..=1 {
                for z in -1..=1 {
                    world
                        .apply_block_change(&BlockChangePacket {
                            position: BlockPosition::new(x * 16 + 8, 200, z * 16 + 8),
                            block_state_id: 16,
                        })
                        .unwrap();
                }
            }
        }
        world
            .apply_block_change(&BlockChangePacket {
                position: BlockPosition::new(0, 0, 0),
                block_state_id: i32::from(id << 4 | meta),
            })
            .unwrap();
        world
    }
    #[test]
    fn integer_maximum_includes_fluid_and_missing_neighbor_suppresses_water() {
        let bounds = Aabb::new([-0.999, 0.0, 0.2], [0.001, 1.8, 0.8]);
        assert!(world(8, 0, true).movement_environment(bounds).water);
        assert!(!world(8, 0, false).movement_environment(bounds).water);
        let lava = Aabb::new(
            [-0.8999999985098839, 0.0, 0.2],
            [0.10000000149011612, 1.8, 0.8],
        );
        assert!(world(10, 0, false).movement_environment(lava).lava);
    }
    #[test]
    #[ignore = "requires local MCP919 fluid bounds Java oracle"]
    fn local_java_fluid_bounds_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/fluid-bounds-java-oracle.log");
        let oracle = std::fs::read_to_string(path).unwrap();
        let mut cases = 0;
        for line in oracle.lines().filter(|line| line.starts_with("FLUID|")) {
            let f: Vec<_> = line.split('|').collect();
            let x: f64 = f[3].parse().unwrap();
            let y: f64 = f[4].parse().unwrap();
            let world = world(f[1].parse().unwrap(), f[2].parse().unwrap(), f[5] == "1");
            let actual =
                world.movement_environment(Aabb::new([x, y, 0.2], [x + 1.0, y + 1.8, 0.8]));
            assert_eq!(actual.water, f[6] == "true", "{line}");
            assert_eq!(actual.lava, f[7] == "true", "{line}");
            cases += 1;
        }
        assert_eq!(cases, 600);
        println!("{cases} MCP919 fluid boundary and loaded-area cases match");
    }
}
