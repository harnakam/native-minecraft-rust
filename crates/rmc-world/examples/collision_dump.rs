use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
use rmc_world::{BlockPos, WorldConfig, WorldSnapshot};
fn main() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    for id in 0..=197 {
        for meta in 0..16 {
            world
                .apply_block_change(&BlockChangePacket {
                    position: BlockPosition { x: 0, y: 64, z: 0 },
                    block_state_id: (id << 4) | meta,
                })
                .unwrap();
            print!("shape {id} {meta}");
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
        }
    }
}
