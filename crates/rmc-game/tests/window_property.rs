use rmc_game::inventory::InventoryState;
use rmc_net::codec::play::{
    OpenWindowPacket, PlayClientboundPacket, WindowItemsPacket, WindowPropertyPacket,
};

#[test]
fn only_current_container_receives_properties_and_slot_refresh_preserves_them() {
    let mut state = InventoryState::new();
    let packet = PlayClientboundPacket::WindowProperty(WindowPropertyPacket {
        window_id: 1,
        property: 2,
        value: 120,
    });
    state.apply_play_packet(&packet);
    assert!(state.open_window().is_none());
    state.apply_open_window(&OpenWindowPacket {
        window_id: 1,
        inventory_type: "minecraft:furnace".into(),
        window_title_json: "{}".into(),
        slot_count: 3,
        entity_id: None,
    });
    state.apply_play_packet(&packet);
    assert_eq!(state.open_window().unwrap().properties.get(&2), Some(&120));
    state.apply_play_packet(&PlayClientboundPacket::WindowProperty(
        WindowPropertyPacket {
            window_id: 2,
            property: 2,
            value: 5,
        },
    ));
    state.apply_window_items(&WindowItemsPacket {
        window_id: 1,
        items: vec![None; 39],
    });
    assert_eq!(state.open_window().unwrap().properties.get(&2), Some(&120));
    state.apply_open_window(&OpenWindowPacket {
        window_id: 2,
        inventory_type: "minecraft:furnace".into(),
        window_title_json: "{}".into(),
        slot_count: 3,
        entity_id: None,
    });
    assert!(state.open_window().unwrap().properties.is_empty());
}

#[test]
fn rejected_click_does_not_roll_back_newer_server_progress() {
    use rmc_net::codec::play::{ConfirmTransactionClientboundPacket, ItemStack};
    let mut state = InventoryState::new();
    state.apply_open_window(&OpenWindowPacket {
        window_id: 1,
        inventory_type: "minecraft:furnace".into(),
        window_title_json: "{}".into(),
        slot_count: 3,
        entity_id: None,
    });
    state.apply_window_items(&WindowItemsPacket {
        window_id: 1,
        items: vec![Some(ItemStack::simple(1, 1, 0)); 39],
    });
    let PlayClientboundPacket::WindowProperty(progress) =
        PlayClientboundPacket::WindowProperty(WindowPropertyPacket {
            window_id: 1,
            property: 2,
            value: 120,
        })
    else {
        panic!()
    };
    let rmc_net::codec::play::PlayServerboundPacket::ClickWindow(click) =
        state.queue_pickup_click(1, 3, 0)
    else {
        panic!()
    };
    state.apply_play_packet(&PlayClientboundPacket::WindowProperty(progress));
    state.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
        window_id: 1,
        action_number: click.action_number,
        accepted: false,
    });
    assert_eq!(state.open_window().unwrap().properties.get(&2), Some(&120));
}
