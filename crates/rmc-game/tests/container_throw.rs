use rmc_game::inventory::InventoryState;
use rmc_net::codec::play::{
    ConfirmTransactionClientboundPacket, ItemStack, PlayServerboundPacket, SetSlotPacket,
};

#[test]
fn throw_one_and_stack_predict_and_reject_restore() {
    let mut inventory = InventoryState::new();
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 9,
        item: Some(ItemStack::simple(1, 12, 0)),
    });
    let PlayServerboundPacket::ClickWindow(click) =
        inventory.queue_throw_click(0, 9, false).unwrap()
    else {
        panic!()
    };
    assert_eq!((click.mode, click.button, click.clicked_item), (4, 0, None));
    assert_eq!(
        inventory.inventory_window().slots[9]
            .as_ref()
            .unwrap()
            .count,
        11
    );
    inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
        window_id: 0,
        action_number: click.action_number,
        accepted: false,
    });
    assert_eq!(
        inventory.inventory_window().slots[9]
            .as_ref()
            .unwrap()
            .count,
        12
    );
    inventory.queue_throw_click(0, 9, true).unwrap();
    assert!(inventory.inventory_window().slots[9].is_none());
}

#[test]
fn carried_stack_blocks_throw_and_invalid_slots_are_rejected() {
    let mut inventory = InventoryState::new();
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 9,
        item: Some(ItemStack::simple(1, 12, 0)),
    });
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: -1,
        slot_id: -1,
        item: Some(ItemStack::simple(5, 2, 0)),
    });
    inventory.queue_throw_click(0, 9, true).unwrap();
    assert_eq!(
        inventory.inventory_window().slots[9]
            .as_ref()
            .unwrap()
            .count,
        12
    );
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 2);
    assert!(inventory.queue_throw_click(0, -999, false).is_err());
    assert!(inventory.queue_throw_click(7, 9, false).is_err());
}

#[test]
fn chest_player_alias_and_rejection_restore_both_views() {
    use rmc_net::codec::play::{OpenWindowPacket, WindowItemsPacket};
    let mut inventory = InventoryState::new();
    inventory.apply_open_window(&OpenWindowPacket {
        window_id: 1,
        inventory_type: "minecraft:container".into(),
        window_title_json: "{\"text\":\"Chest\"}".into(),
        slot_count: 27,
        entity_id: None,
    });
    let mut slots = vec![None; 63];
    slots[27] = Some(ItemStack::simple(1, 12, 0));
    inventory.apply_window_items(&WindowItemsPacket {
        window_id: 1,
        items: slots,
    });
    let PlayServerboundPacket::ClickWindow(click) =
        inventory.queue_throw_click(1, 27, false).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        inventory.inventory_window().slots[9]
            .as_ref()
            .unwrap()
            .count,
        11
    );
    assert_eq!(
        inventory.open_window().unwrap().slots[27]
            .as_ref()
            .unwrap()
            .count,
        11
    );
    inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
        window_id: 1,
        action_number: click.action_number,
        accepted: false,
    });
    assert_eq!(
        inventory.inventory_window().slots[9]
            .as_ref()
            .unwrap()
            .count,
        12
    );
    assert_eq!(
        inventory.open_window().unwrap().slots[27]
            .as_ref()
            .unwrap()
            .count,
        12
    );
}

#[test]
fn creative_clone_obeys_cursor_and_game_mode_and_item_limit() {
    let mut inventory = InventoryState::new();
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 9,
        item: Some(ItemStack::simple(1, 12, 0)),
    });
    inventory.queue_clone_click(0, 9, false).unwrap();
    assert!(inventory.carried_item().is_none());
    let PlayServerboundPacket::ClickWindow(click) =
        inventory.queue_clone_click(0, 9, true).unwrap()
    else {
        panic!()
    };
    assert_eq!((click.mode, click.clicked_item), (3, None));
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 64);
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 10,
        item: Some(ItemStack::simple(276, 1, 0)),
    });
    inventory.queue_clone_click(0, 10, true).unwrap();
    assert_eq!(inventory.carried_item().as_ref().unwrap().item_id, 1);
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: -1,
        slot_id: -1,
        item: None,
    });
    inventory.queue_clone_click(0, 10, true).unwrap();
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 1);
}
