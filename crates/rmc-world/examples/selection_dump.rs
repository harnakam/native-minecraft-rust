use rmc_net::codec::play::{BlockChangePacket, BlockPosition};
use rmc_world::{WorldConfig, WorldSnapshot};
fn main() {
    let mut world = WorldSnapshot::new(WorldConfig::overworld());
    let samples = [
        [0.25, 0.25],
        [0.25, 0.75],
        [0.75, 0.25],
        [0.75, 0.75],
        [0.5, 0.5],
    ];
    for id in 0..=197 {
        for meta in 0..16 {
            world
                .apply_block_change(&BlockChangePacket {
                    position: BlockPosition::new(0, 64, 0),
                    block_state_id: id * 16 + meta,
                })
                .unwrap();
            for face in 0..6 {
                for (sample, coordinates) in samples.iter().enumerate() {
                    let axis = face / 2;
                    let mut origin = [0.0; 3];
                    let mut direction = [0.0; 3];
                    let mut component = 0;
                    for k in 0..3 {
                        let base = if k == 1 { 64.0 } else { 0.0 };
                        origin[k] = base
                            + if k == axis {
                                if face % 2 == 0 {
                                    -2.0
                                } else {
                                    2.0
                                }
                            } else {
                                let value = coordinates[component];
                                component += 1;
                                value
                            };
                    }
                    direction[axis] = if face % 2 == 0 { 1.0 } else { -1.0 };
                    let hit = world.raycast(origin, direction, 4.0);
                    let value = hit
                        .map(|hit| format!("{},{}", hit.distance, hit.face))
                        .unwrap_or_else(|| "-".into());
                    println!("selectionray {id} {meta} {face} {sample} {value}");
                }
            }
        }
    }
}
