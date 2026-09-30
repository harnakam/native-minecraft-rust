use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
use rmc_world::{BlockPos, WorldConfig, WorldSnapshot};
fn set(world: &mut WorldSnapshot, x: i32, z: i32, id: i32) {
    world
        .apply_block_change(&BlockChangePacket {
            position: BlockPosition::new(x, 64, z),
            block_state_id: id << 4,
        })
        .unwrap();
}
fn main() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    let directions = [(-1, 0), (1, 0), (0, -1), (0, 1)];
    for target in [85, 113, 188, 189, 190, 191, 192, 101, 102, 160, 139] {
        for neighbor in 0..=197 {
            for mask in 0..16 {
                for &(dx, dz) in &directions {
                    set(&mut world, dx, dz, 0);
                }
                set(&mut world, 0, 0, target);
                for (side, &(x, z)) in directions.iter().enumerate() {
                    if mask & (1 << side) != 0 {
                        set(&mut world, x, z, neighbor);
                    }
                }
                print!("shape {} 0", target * 10000 + neighbor * 16 + mask);
                for b in world.block_collision_boxes(BlockPos::new(0, 64, 0)) {
                    print!(
                        " {},{},{},{},{},{}",
                        b.min[0],
                        b.min[1] - 64.0,
                        b.min[2],
                        b.max[0],
                        b.max[1] - 64.0,
                        b.max[2]
                    );
                }
                println!();
                print!("shape {} 1", target * 10000 + neighbor * 16 + mask);
                for b in world.selection_boxes(BlockPos::new(0, 64, 0)) {
                    print!(
                        " {},{},{},{},{},{}",
                        b.min[0],
                        b.min[1] - 64.0,
                        b.min[2],
                        b.max[0],
                        b.max[1] - 64.0,
                        b.max[2]
                    );
                }
                println!();
            }
        }
    }
}
