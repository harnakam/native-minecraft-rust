use rmc_game::inventory::InventoryState;
use rmc_net::codec::play::{
    ItemStack, OpenWindowPacket, PlayServerboundPacket, Slot, WindowItemsPacket,
};
fn stack(s: &Slot) -> String {
    s.as_ref().map_or("-".into(), |s| {
        format!("{}:{}:{}", s.item_id, s.count, s.damage)
    })
}
fn main() {
    for (kind, size) in [
        ("chest", 27),
        ("hopper", 5),
        ("dispenser", 9),
        ("dropper", 9),
        ("beacon", 1),
        ("furnace", 3),
    ] {
        for source in 0..size + 36 {
            for id in [1, 339, 35, 264, 265, 15, 17, 263] {
                for count in [1, 32] {
                    for pattern in 0..3 {
                        let mut state = InventoryState::new();
                        state.apply_open_window(&OpenWindowPacket {
                            window_id: 1,
                            inventory_type: format!("minecraft:{kind}"),
                            window_title_json: "{}".into(),
                            slot_count: size as u8,
                            entity_id: None,
                        });
                        let mut slots = vec![
                            if pattern == 0 {
                                None
                            } else {
                                Some(ItemStack::simple(1, 64, 0))
                            };
                            size + 36
                        ];
                        if pattern == 2 {
                            for slot in [0, size + 35] {
                                slots[slot] = Some(ItemStack::simple(id, 60, 1));
                            }
                        }
                        slots[source] = Some(ItemStack::simple(id, count, 0));
                        state.apply_window_items(&WindowItemsPacket {
                            window_id: 1,
                            items: slots,
                        });
                        let PlayServerboundPacket::ClickWindow(click) =
                            state.queue_transfer_click(1, source as i16, 0).unwrap()
                        else {
                            panic!()
                        };
                        print!(
                            "transfer {kind} {source} {id} {count} {pattern} {}",
                            stack(&click.clicked_item)
                        );
                        for s in &state.open_window().unwrap().slots {
                            print!(" {}", stack(s));
                        }
                        println!();
                    }
                }
            }
        }
    }
}
