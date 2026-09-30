use rmc_game::mining::{relative_hardness, MiningContext};
use rmc_net::codec::play::ItemStack;
fn main() {
    for scenario in [
        "normal",
        "airborne",
        "efficiency",
        "haste",
        "fatigue",
        "underwater",
    ] {
        for id in 0..=197 {
            for tool in [
                -1, 256, 257, 258, 267, 268, 269, 270, 271, 272, 273, 274, 275, 276, 277, 278, 279,
                283, 284, 285, 286, 359,
            ] {
                let mut held_item = if tool < 0 {
                    None
                } else {
                    Some(ItemStack::simple(tool, 1, 0))
                };
                if scenario == "efficiency" {
                    if let Some(item) = held_item.as_mut() {
                        item.nbt = Some(vec![
                            10, 0, 0, 9, 0, 4, b'e', b'n', b'c', b'h', 10, 0, 0, 0, 1, 2, 0, 2,
                            b'i', b'd', 0, 32, 2, 0, 3, b'l', b'v', b'l', 0, 2, 0, 0,
                        ]);
                    }
                }
                let context = MiningContext {
                    held_item,
                    on_ground: scenario != "airborne",
                    underwater: scenario == "underwater",
                    haste: if scenario == "haste" { Some(1) } else { None },
                    fatigue: if scenario == "fatigue" { Some(2) } else { None },
                    ..MiningContext::default()
                };
                println!(
                    "hardness {scenario} {id} {tool} {}",
                    relative_hardness(id << 4, &context)
                );
            }
        }
    }
}
