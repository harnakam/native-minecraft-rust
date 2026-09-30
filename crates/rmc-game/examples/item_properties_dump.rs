fn main() {
    for id in 0..=i16::MAX {
        if let Some((limit, subtypes, damage)) =
            rmc_game::inventory::item_properties::properties(id)
        {
            println!("item_property {id} {limit} {subtypes} {damage}");
        }
    }
}
