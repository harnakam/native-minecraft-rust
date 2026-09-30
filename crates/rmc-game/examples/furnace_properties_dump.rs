use rmc_game::inventory::{furnace, item_properties};
fn main() {
    for id in 0..=i16::MAX {
        if item_properties::properties(id).is_some() {
            println!("fuel {id} {}", furnace::fuel_ticks(id));
        }
    }
    for r in furnace::RECIPES {
        println!("smelt {} {} {} {} {}", r.0, r.1, r.2, r.3, r.4);
    }
}
