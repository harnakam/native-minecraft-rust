use rmc_game::mining::{MiningContext, MiningState, MiningTarget};
use rmc_net::codec::play::{BlockPosition, DiggingAction, ItemStack, PlayServerboundPacket};
fn main() {
    for scenario in [
        "stone",
        "switch",
        "release",
        "bedrock",
        "slime",
        "creative",
        "creative_sword",
    ] {
        let mut state = MiningState::default();
        let mut blocks = [true; 2];
        let id = match scenario {
            "bedrock" => 7,
            "slime" => 165,
            _ => 1,
        };
        let context = MiningContext {
            held_item: Some(ItemStack::simple(
                if scenario == "creative_sword" {
                    276
                } else {
                    278
                },
                1,
                0,
            )),
            game_mode: if scenario.starts_with("creative") {
                1
            } else {
                0
            },
            on_ground: true,
            ..MiningContext::default()
        };
        for tick in 1..=20 {
            let index = usize::from(scenario == "switch" && tick >= 4);
            let held = scenario != "release" || tick < 4;
            let target = blocks[index].then_some(MiningTarget {
                position: BlockPosition::new(index as i32, 64, 0),
                face: 1,
                block_state: id << 4,
            });
            let update = state.update(tick == 1, held, 1, target, &context);
            let events = update
                .packets
                .iter()
                .filter_map(|packet| {
                    if let PlayServerboundPacket::PlayerDigging(packet) = packet {
                        let action = match packet.action {
                            DiggingAction::StartDestroyBlock => 0,
                            DiggingAction::AbortDestroyBlock => 1,
                            DiggingAction::StopDestroyBlock => 2,
                            _ => panic!("Unexpected mining action"),
                        };
                        Some(format!(
                            "{action},{},{},{},{}",
                            packet.position.x, packet.position.y, packet.position.z, packet.face
                        ))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join(";");
            for position in update.destroyed {
                blocks[position.x as usize] = false;
            }
            let progress = state.progress();
            println!(
                "minestate {scenario} {tick} {} {} {} {}",
                progress.map_or(0.0, |(_, damage)| damage),
                progress.is_some(),
                !blocks[index],
                if events.is_empty() {
                    "-"
                } else {
                    events.as_str()
                }
            );
        }
    }
}
