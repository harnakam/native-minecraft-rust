use rmc_game::inventory::InventoryState;
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
