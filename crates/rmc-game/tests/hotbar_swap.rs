use rmc_game::inventory::InventoryState;
use rmc_net::codec::play::{
    ConfirmTransactionClientboundPacket, ItemStack, PlayServerboundPacket, SetSlotPacket,
};
#[test]
fn number_key_swap_uses_null_return_stack_and_server_updates_complete_it() {
    let mut inventory = InventoryState::new();
    let PlayServerboundPacket::ClickWindow(click) = inventory.queue_hotbar_swap(0, 9, 2).unwrap()
    else {
        panic!()
    };
    assert_eq!((click.slot_id, click.button, click.mode), (9, 2, 2));
    assert_eq!(click.clicked_item, None);
    assert_eq!(inventory.pending_transactions().len(), 1);
    inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
        window_id: 0,
        action_number: click.action_number,
        accepted: true,
    });
    let item = Some(ItemStack {
        item_id: 264,
        count: 1,
        damage: 0,
        nbt: None,
    });
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: 38,
        item: item.clone(),
    });
    assert_eq!(inventory.inventory_window().slot(38), Some(&item));
    assert!(inventory.pending_transactions().is_empty());
}
#[test]
fn number_key_swap_does_not_send_invalid_or_outside_slots() {
    let mut inventory = InventoryState::new();
    for (window, slot, hotbar) in [(0, -999, 0), (0, 45, 0), (9, 0, 0), (0, 9, 9)] {
        assert!(inventory.queue_hotbar_swap(window, slot, hotbar).is_err());
    }
    assert!(inventory.pending_transactions().is_empty());
}

fn item(id: i16, count: u8) -> Option<ItemStack> {
    Some(ItemStack {
        item_id: id,
        count,
        damage: 0,
        nbt: None,
    })
}
fn set(inventory: &mut InventoryState, slot: i16, stack: Option<ItemStack>) {
    inventory.apply_set_slot(&SetSlotPacket {
        window_id: 0,
        slot_id: slot,
        item: stack,
    });
}
#[test]
fn predicted_player_swap_is_restored_when_server_rejects_it() {
    let mut inventory = InventoryState::new();
    set(&mut inventory, 9, item(278, 1));
    set(&mut inventory, 36, item(264, 16));
    let before = inventory.inventory_window().clone();
    let PlayServerboundPacket::ClickWindow(click) = inventory.queue_hotbar_swap(0, 9, 0).unwrap()
    else {
        panic!()
    };
    assert_eq!(inventory.inventory_window().slot(36), Some(&item(278, 1)));
    assert_eq!(inventory.inventory_window().slot(9), Some(&item(264, 16)));
    inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
        window_id: 0,
        action_number: click.action_number,
        accepted: false,
    });
    assert_eq!(inventory.inventory_window(), &before);
}
#[test]
fn invalid_armor_replacement_requires_empty_inventory_slot() {
    let mut inventory = InventoryState::new();
    for slot in 9..45 {
        set(&mut inventory, slot, item(1, 64));
    }
    set(&mut inventory, 5, item(310, 1));
    inventory.queue_hotbar_swap(0, 5, 0).unwrap();
    assert_eq!(inventory.inventory_window().slot(5), Some(&item(310, 1)));
    assert_eq!(inventory.inventory_window().slot(36), Some(&item(1, 64)));
    set(&mut inventory, 10, None);
    inventory.queue_hotbar_swap(0, 5, 0).unwrap();
    assert_eq!(inventory.inventory_window().slot(5), Some(&None));
    assert_eq!(inventory.inventory_window().slot(36), Some(&item(310, 1)));
    assert_eq!(inventory.inventory_window().slot(10), Some(&item(1, 64)));
}
#[test]
fn chest_swap_merges_displaced_stack_then_uses_empty_player_slot() {
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
    slots[0] = item(264, 1);
    slots[27] = item(1, 50);
    slots[54] = item(1, 32);
    inventory.apply_window_items(&WindowItemsPacket {
        window_id: 1,
        items: slots,
    });
    inventory.queue_hotbar_swap(1, 0, 0).unwrap();
    assert_eq!(inventory.open_window().unwrap().slot(0), Some(&None));
    assert_eq!(inventory.inventory_window().slot(36), Some(&item(264, 1)));
    assert_eq!(inventory.inventory_window().slot(9), Some(&item(1, 64)));
    // First empty slot is hotbar index 1, before the main inventory slots.
    assert_eq!(inventory.inventory_window().slot(37), Some(&item(1, 18)));
    assert_eq!(
        inventory.open_window().unwrap().slot(55),
        Some(&item(1, 18))
    );
}
