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
        for x in bounds.min[0].floor() as i32..=(bounds.max[0] - 0.001).floor() as i32 {
            for y in bounds.min[1].floor() as i32..=(bounds.max[1] - 0.001).floor() as i32 {
                for z in bounds.min[2].floor() as i32..=(bounds.max[2] - 0.001).floor() as i32 {
                    let pos = BlockPos::new(x, y, z);
                    let state = self.block_state_or_air(pos);
                    let cube = Aabb::new(
                        [x as f64, y as f64, z as f64],
                        [x as f64 + 1.0, y as f64 + 1.0, z as f64 + 1.0],
                    );
                    match state >> 4 {
                        88 => {
                            result.soul_sand_contacts = result.soul_sand_contacts.saturating_add(1)
                        }
                        30 => result.web = true,
                        8 | 9 if cube.intersects(water_bounds) => {
                            result.water = true;
                            let flow = self.liquid_flow(pos, 8);
                            for axis in 0..3 {
                                result.water_flow[axis] += flow[axis];
                            }
                        }
                        10 | 11 if cube.intersects(lava_bounds) => result.lava = true,
                        _ => {}
                    }
                }
            }
        }
        let length = result.water_flow.iter().map(|v| v * v).sum::<f64>().sqrt();
        if length > 0.0 {
            for v in &mut result.water_flow {
                *v = *v / length * 0.014;
            }
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
