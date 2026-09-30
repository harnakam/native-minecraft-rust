//! Slot insertion and container merge restrictions corresponding to MCP919 inventory classes.
use super::ContainerSnapshot;
use super::{furnace, item_stack_limit};
use rmc_net::codec::play::ItemStack;

pub(super) fn is_potion_ingredient(item: &ItemStack) -> bool {
    matches!(
        item.item_id,
        289 | 331 | 348 | 353 | 370 | 372 | 375 | 376 | 377 | 378 | 382 | 396 | 414
    ) || (matches!(item.item_id, 349 | 350) && item.damage == 3)
}

pub(super) fn can_merge(window: &ContainerSnapshot, slot: i16) -> bool {
    if window.slot(slot).is_none() {
        return false;
    }
    if window.window_id == 0 {
        return slot != 0;
    }
    match window.metadata.as_ref().map(|m| m.inventory_type.as_str()) {
        Some("minecraft:crafting_table") => slot != 0,
        Some(
            "minecraft:chest"
            | "minecraft:container"
            | "minecraft:hopper"
            | "minecraft:dispenser"
            | "minecraft:dropper"
            | "minecraft:furnace"
            | "minecraft:brewing_stand"
            | "minecraft:enchanting_table"
            | "minecraft:beacon"
            | "EntityHorse",
        ) => true,
        _ => false,
    }
}

pub(super) fn rules(
    window: &ContainerSnapshot,
    slot_id: i16,
    item: Option<&ItemStack>,
) -> (bool, u8) {
    let Some(item) = item else {
        return (true, 64);
    };
    let mut limit = item_stack_limit(item.item_id);
    let valid = if window.window_id == 0 {
        match slot_id {
            0 => false,
            5..=8 => {
                limit = 1;
                ((298..=317).contains(&item.item_id) && (item.item_id - 298) % 4 == slot_id - 5)
                    || (slot_id == 5 && matches!(item.item_id, 86 | 397))
            }
            _ => true,
        }
    } else {
        match window.metadata.as_ref().map(|m| m.inventory_type.as_str()) {
            Some("minecraft:crafting_table") if slot_id == 0 => false,
            Some("minecraft:furnace" | "minecraft:anvil" | "minecraft:villager")
                if slot_id == 2 =>
            {
                false
            }
            Some("minecraft:furnace") if slot_id == 1 => {
                if item.item_id == 325 {
                    limit = 1;
                }
                furnace::fuel_ticks(item.item_id) > 0 || item.item_id == 325
            }
            Some("minecraft:enchanting_table") if slot_id == 0 => {
                limit = 1;
                true
            }
            Some("minecraft:enchanting_table") if slot_id == 1 => {
                item.item_id == 351 && item.damage == 4
            }
            Some("minecraft:brewing_stand") if (0..=2).contains(&slot_id) => {
                limit = 1;
                matches!(item.item_id, 373 | 374)
            }
            Some("minecraft:brewing_stand") if slot_id == 3 => is_potion_ingredient(item),
            Some("minecraft:beacon") if slot_id == 0 => {
                limit = 1;
                matches!(item.item_id, 264 | 265 | 266 | 388)
            }
            Some("EntityHorse") if slot_id == 0 => {
                limit = 1;
                item.item_id == 329 && window.slot(slot_id).is_some_and(Option::is_none)
            }
            Some("EntityHorse") if slot_id == 1 => {
                limit = 1;
                (417..=419).contains(&item.item_id)
            }
            _ => true,
        }
    };
    (valid, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::InventoryState;
    use rmc_net::codec::play::{OpenWindowPacket, SetSlotPacket, WindowItemsPacket};
    fn state(kind: &str, count: u8) -> InventoryState {
        let mut state = InventoryState::new();
        state.apply_open_window(&OpenWindowPacket {
            window_id: 1,
            inventory_type: kind.into(),
            window_title_json: "{}".into(),
            slot_count: count,
            entity_id: None,
        });
        state.apply_window_items(&WindowItemsPacket {
            window_id: 1,
            items: vec![None; count as usize + 36],
        });
        state
    }
    #[test]
    fn brewing_ingredients_match_executed_java_registry_probe() {
        let java = [
            289, 331, 348, 353, 370, 372, 375, 376, 377, 378, 382, 396, 414,
        ];
        let state = state("minecraft:brewing_stand", 4);
        for id in 0..=431 {
            for damage in 0..16 {
                assert_eq!(
                    state
                        .slot_rules(1, 3, Some(&ItemStack::simple(id, 1, damage)))
                        .0,
                    java.contains(&id) || (matches!(id, 349 | 350) && damage == 3),
                    "item {id} damage {damage}"
                );
            }
        }
    }
    #[test]
    fn brewing_insertion_rejects_non_ingredients_in_pickup_and_drag() {
        let mut state = state("minecraft:brewing_stand", 4);
        state.apply_set_slot(&SetSlotPacket {
            window_id: -1,
            slot_id: -1,
            item: Some(ItemStack::simple(1, 12, 0)),
        });
        state.queue_pickup_click(1, 3, 0);
        assert_eq!(state.open_window.as_ref().unwrap().slots[3], None);
        assert_eq!(state.carried_item.as_ref().unwrap().count, 12);
        assert!(!state.can_drag_slot(1, 3));
        state.apply_set_slot(&SetSlotPacket {
            window_id: -1,
            slot_id: -1,
            item: Some(ItemStack::simple(353, 12, 0)),
        });
        assert!(state.can_drag_slot(1, 3));
        state.queue_pickup_click(1, 3, 0);
        assert_eq!(
            state.open_window.as_ref().unwrap().slots[3]
                .as_ref()
                .unwrap()
                .count,
            12
        );
    }
    #[test]
    fn workbench_collection_excludes_output_and_brewing_includes_material_slot() {
        let mut workbench = state("minecraft:crafting_table", 10);
        workbench.open_window.as_mut().unwrap().slots[0] = Some(ItemStack::simple(1, 30, 0));
        workbench.open_window.as_mut().unwrap().slots[1] = Some(ItemStack::simple(1, 20, 0));
        workbench.carried_item = Some(ItemStack::simple(1, 1, 0));
        workbench.queue_collect_click(1, 2, 0).unwrap();
        assert_eq!(workbench.carried_item.as_ref().unwrap().count, 21);
        assert_eq!(
            workbench.open_window.as_ref().unwrap().slots[0]
                .as_ref()
                .unwrap()
                .count,
            30
        );
        assert!(!workbench.can_collect_slot(1, 0));
        let mut brewing = state("minecraft:brewing_stand", 4);
        brewing.open_window.as_mut().unwrap().slots[3] = Some(ItemStack::simple(353, 10, 0));
        brewing.carried_item = Some(ItemStack::simple(353, 1, 0));
        brewing.queue_collect_click(1, 4, 0).unwrap();
        assert_eq!(brewing.carried_item.as_ref().unwrap().count, 11);
    }
    #[test]
    fn enchanting_shift_preserves_single_nbt_and_recreates_split_item() {
        for count in [1, 3] {
            let mut state = state("minecraft:enchanting_table", 2);
            let mut item = ItemStack::simple(1, count, 0);
            item.nbt = Some(vec![10, 0, 0, 0]);
            state.open_window.as_mut().unwrap().slots[2] = Some(item.clone());
            state.queue_transfer_click(1, 2, 0).unwrap();
            let window = state.open_window.as_ref().unwrap();
            let moved = window.slots[0].as_ref().unwrap();
            assert_eq!(moved.count, 1);
            assert_eq!(moved.nbt, if count == 1 { item.nbt } else { None });
            assert_eq!(
                window.slots[2].as_ref().map(|s| s.count),
                if count == 1 { None } else { Some(2) }
            );
        }
    }
    #[test]
    fn brewing_shift_prioritizes_empty_ingredient_then_storage_and_potion() {
        let mut state = state("minecraft:brewing_stand", 4);
        state.open_window.as_mut().unwrap().slots[4] = Some(ItemStack::simple(353, 12, 0));
        state.queue_transfer_click(1, 4, 0).unwrap();
        assert_eq!(
            state.open_window.as_ref().unwrap().slots[3]
                .as_ref()
                .unwrap()
                .count,
            12
        );
        state.open_window.as_mut().unwrap().slots[4] = Some(ItemStack::simple(353, 8, 0));
        state.queue_transfer_click(1, 4, 0).unwrap();
        assert_eq!(
            state.open_window.as_ref().unwrap().slots[31]
                .as_ref()
                .unwrap()
                .count,
            8
        );
        state.open_window.as_mut().unwrap().slots[5] = Some(ItemStack::simple(373, 1, 0));
        state.queue_transfer_click(1, 5, 0).unwrap();
        assert_eq!(
            state.open_window.as_ref().unwrap().slots[0]
                .as_ref()
                .unwrap()
                .item_id,
            373
        );
    }
}
