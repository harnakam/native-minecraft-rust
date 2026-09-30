use rmc_game::inventory::InventoryState;

#[test]
fn server_hotbar_selection_accepts_only_vanilla_indices_without_echo() {
    use rmc_net::codec::play::{HeldItemChangeClientboundPacket, PlayClientboundPacket};
    let mut inventory = InventoryState::new();
    for (slot, expected) in [(8, 8), (-1, 8), (9, 8), (3, 3)] {
        let update = inventory.apply_play_packet(&PlayClientboundPacket::HeldItemChange(
            HeldItemChangeClientboundPacket { slot },
        ));
        assert_eq!(inventory.selected_hotbar_slot(), expected);
        assert!(update.outbound_packets.is_empty());
    }
}
use rmc_net::codec::play::{
    ConfirmTransactionClientboundPacket, ItemStack, PlayServerboundPacket, SetSlotPacket,
};

#[test]
fn pickup_predicts_cursor_then_places_one_with_right_click() {
    let mut inventory = InventoryState::new();
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 36,
        item: Some(ItemStack::simple(5, 5, 0)),
    });
    let packet = inventory.queue_pickup_click(0, 36, 0);
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 5);
    assert_eq!(inventory.inventory_window().slot(36), Some(&None));
    if let PlayServerboundPacket::ClickWindow(packet) = packet {
        assert_eq!(packet.clicked_item.unwrap().count, 5);
    } else {
        panic!("expected click packet");
    }
    inventory.queue_pickup_click(0, 37, 1);
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 4);
    assert_eq!(
        inventory
            .inventory_window()
            .slot(37)
            .unwrap()
            .as_ref()
            .unwrap()
            .count,
        1
    );
}

#[test]
fn rejected_pickup_rolls_back_prediction() {
    let mut inventory = InventoryState::new();
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 36,
        item: Some(ItemStack::simple(5, 5, 0)),
    });
    let packet = inventory.queue_pickup_click(0, 36, 1);
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 3);
    let PlayServerboundPacket::ClickWindow(click) = packet else {
        panic!("expected click");
    };
    let result = inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
        window_id: 0,
        action_number: click.action_number,
        accepted: false,
    });
    assert_eq!(inventory.carried_item(), &None);
    assert_eq!(
        inventory
            .inventory_window()
            .slot(36)
            .unwrap()
            .as_ref()
            .unwrap()
            .count,
        5
    );
    assert_eq!(result.outbound_packets.len(), 1);
}
#[test]
fn player_inventory_in_a_chest_is_the_same_hotbar() {
    use rmc_net::codec::play::{OpenWindowPacket, WindowItemsPacket};
    let mut inventory = InventoryState::new();
    inventory.apply_open_window(&OpenWindowPacket {
        window_id: 4,
        inventory_type: "minecraft:chest".into(),
        window_title_json: "{}".into(),
        slot_count: 27,
        entity_id: None,
    });
    let mut items = vec![None; 63];
    items[54] = Some(ItemStack::simple(276, 1, 0));
    inventory.apply_window_items(&WindowItemsPacket {
        window_id: 4,
        items,
    });
    assert_eq!(inventory.selected_hotbar_item().unwrap().item_id, 276);
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: -2,
        slot_id: 0,
        item: Some(ItemStack::simple(5, 32, 0)),
    });
    assert_eq!(inventory.selected_hotbar_item().unwrap().item_id, 5);
    assert_eq!(
        inventory
            .open_window()
            .unwrap()
            .slot(54)
            .unwrap()
            .as_ref()
            .unwrap()
            .item_id,
        5
    );
    let mut player_items = vec![None; 45];
    player_items[36] = Some(ItemStack::simple(261, 1, 0));
    inventory.apply_window_items(&WindowItemsPacket {
        window_id: 0,
        items: player_items,
    });
    assert_eq!(inventory.selected_hotbar_item().unwrap().item_id, 261);
    assert_eq!(
        inventory
            .open_window()
            .unwrap()
            .slot(54)
            .unwrap()
            .as_ref()
            .unwrap()
            .item_id,
        261
    );
}
#[test]
fn outside_right_click_drops_one_cursor_item() {
    let mut inventory = InventoryState::new();
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: -1,
        slot_id: -1,
        item: Some(ItemStack::simple(5, 5, 0)),
    });
    inventory.queue_pickup_click(0, -999, 1);
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 4);
}
#[test]
fn inventory_screen_opens_real_window_zero_and_closes_with_protocol_packet() {
    let mut inventory = InventoryState::new();
    assert!(matches!(
        inventory.open_player_inventory(),
        PlayServerboundPacket::ClientStatus(2)
    ));
    assert_eq!(inventory.open_window().unwrap().window_id, 0);
    assert_eq!(inventory.open_window().unwrap().slots.len(), 45);
    assert!(
        matches!(inventory.close_open_window(),Some(PlayServerboundPacket::CloseWindow(packet)) if packet.window_id==0)
    );
    assert!(inventory.open_window().is_none());
}
#[test]
fn armor_slots_reject_normal_blocks_and_crafting_result_rejects_placement() {
    let mut inventory = InventoryState::new();
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: -1,
        slot_id: -1,
        item: Some(ItemStack::simple(5, 5, 0)),
    });
    inventory.queue_pickup_click(0, 5, 0);
    assert_eq!(inventory.inventory_window().slot(5), Some(&None));
    assert_eq!(inventory.carried_item().as_ref().unwrap().count, 5);
    inventory.queue_pickup_click(0, 0, 0);
    assert_eq!(inventory.inventory_window().slot(0), Some(&None));
}
