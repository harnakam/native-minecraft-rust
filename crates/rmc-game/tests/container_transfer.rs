use rmc_game::inventory::InventoryState;
use rmc_net::codec::play::{ItemStack, OpenWindowPacket, PlayServerboundPacket, WindowItemsPacket};

fn chest() -> InventoryState {
    let mut state = InventoryState::new();
    state.apply_open_window(&OpenWindowPacket {
        window_id: 1,
        inventory_type: "minecraft:container".into(),
        window_title_json: "{}".into(),
        slot_count: 27,
        entity_id: None,
    });
    state
}

#[test]
fn chest_transfer_merges_before_empty_in_reverse_player_order() {
    let mut state = chest();
    let mut slots = vec![None; 63];
    slots[0] = Some(ItemStack::simple(1, 32, 0));
    slots[62] = Some(ItemStack::simple(1, 60, 0));
    state.apply_window_items(&WindowItemsPacket {
        window_id: 1,
        items: slots,
    });
    let PlayServerboundPacket::ClickWindow(click) = state.queue_transfer_click(1, 0, 0).unwrap()
    else {
        panic!()
    };
    assert_eq!(click.mode, 1);
    assert_eq!(click.clicked_item, Some(ItemStack::simple(1, 32, 0)));
    let window = state.open_window().unwrap();
    assert!(window.slots[0].is_none());
    assert_eq!(window.slots[62], Some(ItemStack::simple(1, 64, 0)));
    assert_eq!(window.slots[61], Some(ItemStack::simple(1, 28, 0)));
    assert_eq!(
        state.inventory_window().slots[43],
        Some(ItemStack::simple(1, 28, 0))
    );
}

#[test]
fn full_chest_transfer_is_null_and_changes_nothing() {
    let mut state = chest();
    let mut slots = vec![Some(ItemStack::simple(1, 64, 0)); 63];
    slots[27] = Some(ItemStack::simple(5, 12, 0));
    state.apply_window_items(&WindowItemsPacket {
        window_id: 1,
        items: slots.clone(),
    });
    let PlayServerboundPacket::ClickWindow(click) = state.queue_transfer_click(1, 27, 1).unwrap()
    else {
        panic!()
    };
    assert!(click.clicked_item.is_none());
    assert_eq!(state.open_window().unwrap().slots, slots);
}

#[test]
fn player_transfer_equips_armor_and_rejection_restores_storage() {
    use rmc_net::codec::play::{ConfirmTransactionClientboundPacket, SetSlotPacket};
    let mut state = InventoryState::new();
    state.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 9,
        item: Some(ItemStack::simple(310, 1, 17)),
    });
    let PlayServerboundPacket::ClickWindow(click) = state.queue_transfer_click(0, 9, 0).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        state.inventory_window().slots[5],
        Some(ItemStack::simple(310, 1, 17))
    );
    assert!(state.inventory_window().slots[9].is_none());
    state.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
        window_id: 0,
        action_number: click.action_number,
        accepted: false,
    });
    assert!(state.inventory_window().slots[5].is_none());
    assert_eq!(
        state.inventory_window().slots[9],
        Some(ItemStack::simple(310, 1, 17))
    );
}

#[test]
fn partial_transfer_returns_original_count_and_keeps_remainder() {
    let mut state = chest();
    let mut slots = vec![Some(ItemStack::simple(5, 64, 0)); 63];
    slots[0] = Some(ItemStack::simple(1, 32, 0));
    slots[62] = Some(ItemStack::simple(1, 60, 0));
    state.apply_window_items(&WindowItemsPacket {
        window_id: 1,
        items: slots,
    });
    let PlayServerboundPacket::ClickWindow(click) = state.queue_transfer_click(1, 0, 0).unwrap()
    else {
        panic!()
    };
    assert_eq!(click.clicked_item, Some(ItemStack::simple(1, 32, 0)));
    assert_eq!(
        state.open_window().unwrap().slots[0],
        Some(ItemStack::simple(1, 28, 0))
    );
    assert_eq!(
        state.open_window().unwrap().slots[62],
        Some(ItemStack::simple(1, 64, 0))
    );
}

#[test]
fn transfer_ignores_non_subtype_metadata_but_preserves_subtypes() {
    for (id, merged) in [(339, true), (35, false)] {
        let mut state = chest();
        let mut slots = vec![None; 63];
        slots[0] = Some(ItemStack::simple(id, 12, 0));
        slots[62] = Some(ItemStack::simple(id, 50, 1));
        state.apply_window_items(&WindowItemsPacket {
            window_id: 1,
            items: slots,
        });
        state.queue_transfer_click(1, 0, 0).unwrap();
        assert_eq!(
            state.open_window().unwrap().slots[62]
                .as_ref()
                .unwrap()
                .count,
            if merged { 62 } else { 50 }
        );
        assert_eq!(state.open_window().unwrap().slots[61].is_none(), merged);
    }
}
