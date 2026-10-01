//! Container click algorithms corresponding to net.minecraft.inventory.Container.
use super::{item_has_subtypes, item_stack_limit, predict_pickup, InventoryState};
use rmc_net::codec::play::PlayServerboundPacket;

impl InventoryState {
    pub fn can_drag_slot(&self, window_id: u8, slot_id: i16) -> bool {
        let Some(cursor) = self.carried_item.as_ref() else {
            return false;
        };
        let Some(slot) = self.slot(window_id, slot_id) else {
            return false;
        };
        let (valid, _) = self.slot_rules(window_id, slot_id, Some(cursor));
        valid
            && slot.as_ref().is_none_or(|stack| {
                stack.item_id == cursor.item_id
                    && stack.damage == cursor.damage
                    && stack.tags_equal(cursor)
                    && stack.count <= item_stack_limit(cursor.item_id)
            })
    }

    /// Container mode 5: start, select slots, distribute, reset.
    pub fn queue_drag_click(
        &mut self,
        window_id: u8,
        slot_id: i16,
        button: i8,
        creative: bool,
    ) -> Result<PlayServerboundPacket, &'static str> {
        if window_id != 0
            && self
                .open_window
                .as_ref()
                .is_none_or(|w| w.window_id != window_id)
        {
            return Err("Invalid drag window");
        }
        let event = button as u8 & 3;
        let mode = button as u8 >> 2 & 3;
        let previous_event = if self.drag.is_some() { 1 } else { 0 };
        let transition_valid = previous_event == event || (previous_event == 1 && event == 2);
        if !transition_valid || self.carried_item.is_none() {
            self.drag = None;
        } else if event == 0 {
            self.drag = if mode <= 1 || (mode == 2 && creative) {
                Some((window_id, mode, Default::default()))
            } else {
                None
            };
        } else if event == 1 {
            if self.can_drag_slot(window_id, slot_id) {
                let count = self.carried_item.as_ref().unwrap().count as usize;
                if let Some((id, _, slots)) = self.drag.as_mut() {
                    if *id == window_id && count > slots.len() {
                        slots.insert(slot_id);
                    }
                }
            }
        } else if event == 2 {
            let (id, mode, slots) = self.drag.take().unwrap();
            if id == window_id && !slots.is_empty() {
                let before = if id == 0 {
                    self.inventory_window.clone()
                } else {
                    self.open_window.as_ref().unwrap().clone()
                };
                let previous_cursor = self.carried_item.clone();
                let cursor = previous_cursor.as_ref().unwrap();
                let mut after = before.clone();
                let mut remaining = i32::from(cursor.count);
                for &slot in &slots {
                    if !self.can_drag_slot(id, slot) || usize::from(cursor.count) < slots.len() {
                        continue;
                    }
                    let old_count = before.slots[slot as usize].as_ref().map_or(0, |s| s.count);
                    let amount = match mode {
                        0 => usize::from(cursor.count) / slots.len(),
                        1 => 1,
                        _ => usize::from(item_stack_limit(cursor.item_id)),
                    };
                    let (_, limit) = self.slot_rules(id, slot, Some(cursor));
                    let count = (usize::from(old_count) + amount).min(usize::from(limit));
                    remaining -= count as i32 - i32::from(old_count);
                    let mut stack = cursor.clone();
                    stack.count = count as u8;
                    after.slots[slot as usize] = Some(stack);
                }
                self.carried_item = if remaining > 0 {
                    let mut stack = cursor.clone();
                    stack.count = remaining as u8;
                    Some(stack)
                } else {
                    None
                };
                let packet = self.queue_click(id, slot_id, button, 5, None);
                if let PlayServerboundPacket::ClickWindow(click) = &packet {
                    self.pickup_predictions
                        .insert((id, click.action_number), (before, previous_cursor));
                }
                if super::crafting::grid_width(&after).is_some_and(|width| {
                    slots
                        .iter()
                        .any(|&slot| slot >= 1 && slot as usize <= width * width)
                }) {
                    super::crafting::refresh(&mut after);
                }
                if id == 0 {
                    self.inventory_window = after;
                    self.sync_player_inventory_to_open();
                } else {
                    self.open_window = Some(after);
                    self.sync_open_player_inventory();
                }
                return Ok(packet);
            }
        } else {
            self.drag = None;
        }
        Ok(self.queue_click(window_id, slot_id, button, 5, None))
    }

    /// Container.slotClick mode 6: partial stacks first, then full stacks.
    pub fn queue_collect_click(
        &mut self,
        window_id: u8,
        slot_id: i16,
        button: i8,
    ) -> Result<PlayServerboundPacket, &'static str> {
        if !matches!(button, 0 | 1) || self.slot(window_id, slot_id).is_none() {
            return Err("Invalid collect click");
        }
        let before = if window_id == 0 {
            self.inventory_window.clone()
        } else {
            self.open_window.as_ref().unwrap().clone()
        };
        if !self.can_collect_slot(window_id, slot_id) {
            return Err("Collect requires implemented container take/merge rules");
        }
        let previous_cursor = self.carried_item.clone();
        let mut after = before.clone();
        if before.slot(slot_id).unwrap().is_none() {
            if let Some(cursor) = self.carried_item.as_mut() {
                let limit = item_stack_limit(cursor.item_id);
                let mut indices: Vec<usize> = (0..after.slots.len()).collect();
                if button == 1 {
                    indices.reverse();
                }
                for pass in 0..2 {
                    for &index in &indices {
                        if cursor.count >= limit {
                            break;
                        }
                        // ContainerPlayer disallows merging from its crafting result.
                        if !super::slot::can_merge(&before, index as i16) {
                            continue;
                        }
                        let Some(stack) = after.slots[index].as_mut() else {
                            continue;
                        };
                        if stack.item_id != cursor.item_id
                            || stack.damage != cursor.damage
                            || !stack.tags_equal(cursor)
                            || (pass == 0 && stack.count == item_stack_limit(stack.item_id))
                        {
                            continue;
                        }
                        let amount = stack.count.min(limit - cursor.count);
                        cursor.count += amount;
                        stack.count -= amount;
                        if stack.count == 0 {
                            after.slots[index] = None;
                        }
                    }
                }
            }
        }
        let packet = self.queue_click(window_id, slot_id, button, 6, None);
        if after != before || self.carried_item != previous_cursor {
            if let PlayServerboundPacket::ClickWindow(click) = &packet {
                self.pickup_predictions
                    .insert((window_id, click.action_number), (before, previous_cursor));
            }
            if window_id == 0 {
                self.inventory_window = after;
                self.sync_player_inventory_to_open();
            } else {
                self.open_window = Some(after);
                self.sync_open_player_inventory();
            }
        }
        Ok(packet)
    }

    /// Shift-click transfer for player storage and unrestricted chest storage.
    pub fn queue_transfer_click(
        &mut self,
        window_id: u8,
        slot_id: i16,
        button: i8,
    ) -> Result<PlayServerboundPacket, &'static str> {
        if !matches!(button, 0 | 1) {
            return Err("Invalid shift-click button");
        }
        let item = self
            .slot(window_id, slot_id)
            .ok_or("Invalid inventory slot")?
            .clone();
        let before = if window_id == 0 {
            self.inventory_window.clone()
        } else {
            self.open_window.as_ref().unwrap().clone()
        };
        let mut after = before.clone();
        if slot_id == 0 && super::crafting::grid_width(&before).is_some() {
            let returned = super::crafting::transfer_output(&mut after);
            let packet = self.queue_click(window_id, slot_id, button, 1, returned);
            if after != before {
                if let PlayServerboundPacket::ClickWindow(click) = &packet {
                    self.pickup_predictions.insert(
                        (window_id, click.action_number),
                        (before, self.carried_item.clone()),
                    );
                }
                if window_id == 0 {
                    self.inventory_window = after;
                    self.sync_player_inventory_to_open();
                } else {
                    self.open_window = Some(after);
                    self.sync_open_player_inventory();
                }
            }
            return Ok(packet);
        }
        let mut returned = None;
        if let Some(mut stack) = item.clone() {
            let (start, end, reverse) = if window_id == 0 {
                let armor = (298..=317)
                    .contains(&stack.item_id)
                    .then(|| 5 + (stack.item_id as usize - 298) % 4);
                if slot_id < 9 {
                    (9, 45, false)
                } else if let Some(armor) = armor.filter(|a| after.slots[*a].is_none()) {
                    (armor, armor + 1, false)
                } else if slot_id < 36 {
                    (36, 45, false)
                } else {
                    (9, 36, false)
                }
            } else if before
                .metadata
                .as_ref()
                .is_some_and(|m| m.inventory_type == "minecraft:enchanting_table")
                && slot_id >= 2
                && !(stack.item_id == 351 && stack.damage == 4)
            {
                if after.slots[0].is_none() {
                    let mut item = stack.clone();
                    item.count = 1;
                    // MCP constructs a fresh stack when splitting a larger tagged stack.
                    if stack.count > 1 {
                        item.nbt = None;
                    }
                    after.slots[0] = Some(item);
                    stack.count -= 1;
                }
                (0, 0, false)
            } else {
                super::transfer::destination(&before, slot_id as usize, &stack)?
            };
            if after.slots.len() < end {
                return Err("Incomplete player inventory");
            }
            let mut indices: Vec<usize> = (start..end).collect();
            if reverse {
                indices.reverse();
            }
            let limit = item_stack_limit(stack.item_id);
            if limit > 1 {
                for &index in &indices {
                    if let Some(target) = &mut after.slots[index] {
                        if target.item_id == stack.item_id
                            && (!item_has_subtypes(stack.item_id) || target.damage == stack.damage)
                            && target.tags_equal(&stack)
                        {
                            let amount = stack.count.min(limit.saturating_sub(target.count));
                            target.count += amount;
                            stack.count -= amount;
                            if stack.count == 0 {
                                break;
                            }
                        }
                    }
                }
            }
            if stack.count > 0 {
                if let Some(&index) = indices.iter().find(|&&i| after.slots[i].is_none()) {
                    after.slots[index] = Some(stack.clone());
                    stack.count = 0;
                }
            }
            if stack.count != item.as_ref().unwrap().count {
                returned = item;
                after.slots[slot_id as usize] = (stack.count > 0).then_some(stack);
            }
        }
        if after != before
            && super::crafting::grid_width(&after)
                .is_some_and(|width| slot_id >= 1 && slot_id as usize <= width * width)
        {
            super::crafting::refresh(&mut after);
        }
        let packet = self.queue_click(window_id, slot_id, button, 1, returned);
        if after != before {
            if let PlayServerboundPacket::ClickWindow(click) = &packet {
                self.pickup_predictions.insert(
                    (window_id, click.action_number),
                    (before, self.carried_item.clone()),
                );
            }
            if window_id == 0 {
                self.inventory_window = after;
                self.sync_player_inventory_to_open();
            } else {
                self.open_window = Some(after);
                self.sync_open_player_inventory();
            }
        }
        Ok(packet)
    }

    /// Creative-only Container.slotClick mode 3, preserving source NBT.
    pub fn queue_clone_click(
        &mut self,
        window_id: u8,
        slot_id: i16,
        creative: bool,
    ) -> Result<PlayServerboundPacket, &'static str> {
        let item = self
            .slot(window_id, slot_id)
            .ok_or("Invalid inventory slot")?
            .clone();
        let before = if window_id == 0 {
            self.inventory_window.clone()
        } else {
            self.open_window.as_ref().unwrap().clone()
        };
        let previous_cursor = self.carried_item.clone();
        let packet = self.queue_click(window_id, slot_id, 0, 3, None);
        if creative && previous_cursor.is_none() {
            if let Some(mut item) = item {
                item.count = item_stack_limit(item.item_id);
                self.carried_item = Some(item);
                if let PlayServerboundPacket::ClickWindow(click) = &packet {
                    self.pickup_predictions
                        .insert((window_id, click.action_number), (before, previous_cursor));
                }
            }
        }
        Ok(packet)
    }

    /// Container.slotClick mode 4: null return stack, with prediction only when
    /// the cursor is empty. Result slots require their onPickup side effects.
    pub fn queue_throw_click(
        &mut self,
        window_id: u8,
        slot_id: i16,
        whole_stack: bool,
    ) -> Result<PlayServerboundPacket, &'static str> {
        let item = self
            .slot(window_id, slot_id)
            .ok_or("Invalid inventory slot")?
            .clone();
        let before = if window_id == 0 {
            self.inventory_window.clone()
        } else {
            self.open_window.as_ref().unwrap().clone()
        };
        let result_slot = if window_id == 0 {
            slot_id == 0
        } else {
            match before.metadata.as_ref().map(|m| m.inventory_type.as_str()) {
                Some("minecraft:crafting_table") => slot_id == 0,
                Some("minecraft:anvil" | "minecraft:villager" | "minecraft:furnace") => {
                    slot_id == 2
                }
                _ => false,
            }
        };
        if result_slot && item.is_some() && self.carried_item.is_none() {
            return Err("Throwing recipe results requires pickup side effects");
        }
        let packet = self.queue_click(window_id, slot_id, i8::from(whole_stack), 4, None);
        if self.carried_item.is_none() {
            if let Some(mut item) = item {
                item.count = if whole_stack {
                    0
                } else {
                    item.count.saturating_sub(1)
                };
                let remaining = (item.count > 0).then_some(item);
                if let PlayServerboundPacket::ClickWindow(click) = &packet {
                    self.pickup_predictions
                        .insert((window_id, click.action_number), (before, None));
                }
                if window_id == 0 {
                    self.inventory_window.set_slot(slot_id, remaining);
                    self.sync_player_inventory_to_open();
                } else {
                    self.open_window
                        .as_mut()
                        .unwrap()
                        .set_slot(slot_id, remaining);
                    self.sync_open_player_inventory();
                }
            }
        }
        Ok(packet)
    }
    pub fn queue_hotbar_swap(
        &mut self,
        window_id: u8,
        slot_id: i16,
        hotbar: u8,
    ) -> Result<PlayServerboundPacket, &'static str> {
        if hotbar > 8 {
            return Err("Invalid hotbar index");
        }
        if self.slot(window_id, slot_id).is_none() {
            return Err("Invalid inventory slot");
        }
        let before = if window_id == 0 {
            self.inventory_window.clone()
        } else {
            self.open_window.as_ref().unwrap().clone()
        };
        let offset = before.player_inventory_offset() as i16;
        let player_slot = if window_id == 0 && slot_id >= 5 {
            Some(slot_id)
        } else if window_id != 0 && slot_id >= offset {
            let index = slot_id - offset;
            Some(if index < 27 {
                index + 9
            } else {
                index - 27 + 36
            })
        } else {
            None
        };
        let output = if window_id == 0 {
            slot_id == 0
        } else {
            match before.metadata.as_ref().map(|m| m.inventory_type.as_str()) {
                Some("minecraft:crafting_table") => slot_id == 0,
                Some("minecraft:anvil" | "minecraft:villager") => slot_id == 2,
                _ => false,
            }
        };
        if output {
            return Err("Number-key swaps on recipe result slots are not implemented");
        }
        let source = before.slot(slot_id).unwrap().clone();
        let hotbar_slot = 36 + i16::from(hotbar);
        let displaced = self
            .inventory_window
            .slot(hotbar_slot)
            .ok_or("Incomplete player inventory")?
            .clone();
        let (valid, _) = self.slot_rules(window_id, slot_id, displaced.as_ref());
        let mut player = self.inventory_window.clone();
        let mut container = before.clone();
        let empty = (36..45)
            .chain(9..36)
            .find(|slot| player.slot(*slot).is_some_and(Option::is_none));
        let can_swap = displaced.is_none() || (player_slot.is_some() && valid) || empty.is_some();
        let mut replacement = source.clone();
        if source.is_some() && can_swap {
            player.set_slot(hotbar_slot, source);
            if displaced.is_none() || (player_slot.is_some() && valid) {
                replacement = displaced;
            } else {
                if let Some(mut item) = displaced {
                    for slot in (36..45).chain(9..36) {
                        let Some(stack) = player.slots[slot].as_mut() else {
                            continue;
                        };
                        if stack.item_id == item.item_id
                            && (!item_has_subtypes(item.item_id) || stack.damage == item.damage)
                            && stack.tags_equal(&item)
                        {
                            let amount = item
                                .count
                                .min(item_stack_limit(item.item_id).saturating_sub(stack.count));
                            stack.count += amount;
                            item.count -= amount;
                            if item.count == 0 {
                                break;
                            }
                        }
                    }
                    if item.count > 0 {
                        player.set_slot(empty.unwrap(), Some(item));
                    }
                }
                replacement = None;
            }
        } else if source.is_none() && displaced.is_some() && valid {
            player.set_slot(hotbar_slot, None);
            replacement = displaced;
        }
        if let Some(slot) = player_slot {
            player.set_slot(slot, replacement);
        } else {
            container.set_slot(slot_id, replacement);
        }
        // Mode 2 returns null, even when the number-key swap changes both slots.
        let packet = self.queue_click(window_id, slot_id, hotbar as i8, 2, None);
        if let PlayServerboundPacket::ClickWindow(click) = &packet {
            self.pickup_predictions.insert(
                (window_id, click.action_number),
                (before, self.carried_item.clone()),
            );
        }
        self.inventory_window = player;
        if window_id != 0 {
            self.open_window = Some(container);
        }
        self.sync_player_inventory_to_open();
        Ok(packet)
    }

    pub fn queue_pickup_click(
        &mut self,
        window_id: u8,
        slot_id: i16,
        button: i8,
    ) -> PlayServerboundPacket {
        self.drag = None;
        let clicked_item = self.slot(window_id, slot_id).cloned().unwrap_or(None);
        let before = if window_id == 0 {
            Some(self.inventory_window.clone())
        } else {
            self.open_window
                .as_ref()
                .filter(|window| window.window_id == window_id)
                .cloned()
        };
        let previous_cursor = self.carried_item.clone();
        let packet = self.queue_click(window_id, slot_id, button, 0, clicked_item.clone());
        if let (Some(before), PlayServerboundPacket::ClickWindow(click)) = (before, &packet) {
            if slot_id == -999 && matches!(button, 0 | 1) {
                self.pickup_predictions
                    .insert((window_id, click.action_number), (before, previous_cursor));
                if button == 0 {
                    self.carried_item = None;
                } else if let Some(item) = self.carried_item.as_mut() {
                    item.count = item.count.saturating_sub(1);
                    if item.count == 0 {
                        self.carried_item = None;
                    }
                }
                return packet;
            }
            if matches!(button, 0 | 1) && slot_id >= 0 && before.slot(slot_id).is_some() {
                let is_result = slot_id == 0 && super::crafting::grid_width(&before).is_some();
                self.pickup_predictions
                    .insert((window_id, click.action_number), (before, previous_cursor));
                let (valid, limit) =
                    self.slot_rules(window_id, slot_id, self.carried_item.as_ref());
                let (slot, cursor) = if is_result {
                    let result = clicked_item.clone();
                    match (result.as_ref(), self.carried_item.as_ref()) {
                        (Some(_), None) => (None, result),
                        (Some(output), Some(cursor))
                            if output.item_id == cursor.item_id
                                && output.damage == cursor.damage
                                && output.tags_equal(cursor)
                                && u16::from(output.count) + u16::from(cursor.count)
                                    <= u16::from(item_stack_limit(cursor.item_id)) =>
                        {
                            let mut cursor = cursor.clone();
                            cursor.count += output.count;
                            (None, Some(cursor))
                        }
                        _ => (result, self.carried_item.clone()),
                    }
                } else {
                    predict_pickup(
                        clicked_item.clone(),
                        self.carried_item.take(),
                        button,
                        valid,
                        limit,
                    )
                };
                let crafted = is_result && clicked_item.is_some() && slot.is_none();
                self.carried_item = cursor;
                if window_id == 0 {
                    self.inventory_window.set_slot(slot_id, slot);
                    if crafted {
                        super::crafting::consume(&mut self.inventory_window);
                    } else if (1..=4).contains(&slot_id) {
                        super::crafting::refresh(&mut self.inventory_window);
                    }
                } else if let Some(window) = &mut self.open_window {
                    window.set_slot(slot_id, slot);
                    if crafted {
                        super::crafting::consume(window);
                    } else if super::crafting::grid_width(window)
                        .is_some_and(|width| slot_id >= 1 && slot_id as usize <= width * width)
                    {
                        super::crafting::refresh(window);
                    }
                }
            }
        }
        if window_id == 0 {
            self.sync_player_inventory_to_open();
        } else {
            self.sync_open_player_inventory();
        }
        packet
    }
}

#[cfg(test)]
mod collect_tests {
    use super::*;
    use rmc_net::codec::play::{ConfirmTransactionClientboundPacket, ItemStack};

    fn state() -> InventoryState {
        let mut state = InventoryState::new();
        state.carried_item = Some(ItemStack::simple(1, 10, 0));
        state.inventory_window.slots[9] = Some(ItemStack::simple(1, 64, 0));
        state.inventory_window.slots[10] = Some(ItemStack::simple(1, 20, 0));
        state.inventory_window.slots[11] = Some(ItemStack::simple(1, 30, 0));
        state.inventory_window.slots[12] = Some(ItemStack::simple(1, 7, 1));
        state.inventory_window.slots[0] = Some(ItemStack::simple(1, 6, 0));
        state
    }

    #[test]
    fn collects_partial_stacks_before_full_and_rolls_back_rejection() {
        let mut state = state();
        let before = state.inventory_window.clone();
        let packet = state.queue_collect_click(0, 13, 0).unwrap();
        let PlayServerboundPacket::ClickWindow(click) = packet else {
            panic!("wrong packet")
        };
        assert_eq!(click.mode, 6);
        assert_eq!(click.clicked_item, None);
        assert_eq!(state.carried_item.as_ref().unwrap().count, 64);
        assert_eq!(state.inventory_window.slots[9].as_ref().unwrap().count, 60);
        assert_eq!(state.inventory_window.slots[10], None);
        assert_eq!(state.inventory_window.slots[11], None);
        assert_eq!(state.inventory_window.slots[12].as_ref().unwrap().count, 7);
        assert_eq!(state.inventory_window.slots[0].as_ref().unwrap().count, 6);
        let update = state.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
            window_id: 0,
            action_number: click.action_number,
            accepted: false,
        });
        assert_eq!(update.rejected_transactions, vec![(0, click.action_number)]);
        assert_eq!(state.inventory_window, before);
        assert_eq!(state.carried_item.as_ref().unwrap().count, 10);
    }

    #[test]
    fn collection_keeps_distinct_nbt_stacks_separate() {
        let mut state = state();
        let tags = vec![10, 0, 0, 3, 0, 1, b'x', 0, 0, 0, 1, 0];
        state.carried_item.as_mut().unwrap().nbt = Some(tags.clone());
        state.inventory_window.slots[10].as_mut().unwrap().nbt = Some(tags);
        let other = vec![10, 0, 0, 3, 0, 1, b'x', 0, 0, 0, 2, 0];
        state.inventory_window.slots[11].as_mut().unwrap().nbt = Some(other);
        state.queue_collect_click(0, 13, 0).unwrap();
        assert_eq!(state.carried_item.as_ref().unwrap().count, 30);
        assert_eq!(state.inventory_window.slots[10], None);
        assert_eq!(state.inventory_window.slots[11].as_ref().unwrap().count, 30);
        assert_eq!(state.inventory_window.slots[9].as_ref().unwrap().count, 64);
    }

    #[test]
    fn reverse_direction_and_occupied_target_follow_source_guard() {
        let mut state = state();
        state.carried_item.as_mut().unwrap().count = 40;
        state.queue_collect_click(0, 13, 1).unwrap();
        assert_eq!(state.inventory_window.slots[11].as_ref().unwrap().count, 6);
        assert_eq!(state.inventory_window.slots[10].as_ref().unwrap().count, 20);
        let mut state = super::collect_tests::state();
        let before = state.inventory_window.clone();
        state.queue_collect_click(0, 10, 0).unwrap();
        assert_eq!(state.inventory_window, before);
        assert_eq!(state.carried_item.as_ref().unwrap().count, 10);
    }
}

#[cfg(test)]
mod drag_tests {
    use super::*;
    use rmc_net::codec::play::{ConfirmTransactionClientboundPacket, ItemStack};
    fn run(mode: u8, count: u8, creative: bool) -> InventoryState {
        let mut state = InventoryState::new();
        state.carried_item = Some(ItemStack::simple(1, count, 0));
        state
            .queue_drag_click(0, -999, (mode * 4) as i8, creative)
            .unwrap();
        for slot in [9, 10, 10, 11] {
            state
                .queue_drag_click(0, slot, (mode * 4 + 1) as i8, creative)
                .unwrap();
        }
        state
            .queue_drag_click(0, -999, (mode * 4 + 2) as i8, creative)
            .unwrap();
        state
    }
    #[test]
    fn even_single_and_creative_distribution_with_duplicate_slots() {
        let even = run(0, 14, false);
        for slot in 9..=11 {
            assert_eq!(even.inventory_window.slots[slot].as_ref().unwrap().count, 4);
        }
        assert_eq!(even.carried_item.as_ref().unwrap().count, 2);
        let single = run(1, 14, false);
        for slot in 9..=11 {
            assert_eq!(
                single.inventory_window.slots[slot].as_ref().unwrap().count,
                1
            );
        }
        assert_eq!(single.carried_item.as_ref().unwrap().count, 11);
        let creative = run(2, 3, true);
        for slot in 9..=11 {
            assert_eq!(
                creative.inventory_window.slots[slot]
                    .as_ref()
                    .unwrap()
                    .count,
                64
            );
        }
        assert_eq!(creative.carried_item, None);
        let invalid = run(2, 3, false);
        assert!(invalid.inventory_window.slots.iter().all(Option::is_none));
    }
    #[test]
    fn insufficient_quantity_caps_selection_and_wrong_transition_resets() {
        let small = run(0, 2, false);
        assert_eq!(small.inventory_window.slots[9].as_ref().unwrap().count, 1);
        assert_eq!(small.inventory_window.slots[10].as_ref().unwrap().count, 1);
        assert_eq!(small.inventory_window.slots[11], None);
        let mut state = InventoryState::new();
        state.carried_item = Some(ItemStack::simple(1, 12, 0));
        state.queue_drag_click(0, 9, 1, false).unwrap();
        state.queue_drag_click(0, -999, 2, false).unwrap();
        assert!(state.inventory_window.slots.iter().all(Option::is_none));
    }
    #[test]
    fn capacity_slot_rules_and_rejection_preserve_cursor_and_slots() {
        let mut state = InventoryState::new();
        state.carried_item = Some(ItemStack::simple(1, 12, 0));
        state.inventory_window.slots[9] = Some(ItemStack::simple(1, 63, 0));
        let before = state.inventory_window.clone();
        state.queue_drag_click(0, -999, 0, false).unwrap();
        for slot in [0, 5, 9, 10] {
            state.queue_drag_click(0, slot, 1, false).unwrap();
        }
        let packet = state.queue_drag_click(0, -999, 2, false).unwrap();
        assert_eq!(state.inventory_window.slots[9].as_ref().unwrap().count, 64);
        assert_eq!(state.inventory_window.slots[10].as_ref().unwrap().count, 6);
        assert_eq!(state.carried_item.as_ref().unwrap().count, 5);
        let PlayServerboundPacket::ClickWindow(click) = packet else {
            panic!()
        };
        assert_eq!(click.clicked_item, None);
        state.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
            window_id: 0,
            action_number: click.action_number,
            accepted: false,
        });
        assert_eq!(state.inventory_window, before);
        assert_eq!(state.carried_item.as_ref().unwrap().count, 12);
        state.queue_drag_click(0, -999, 0, false).unwrap();
        state.close_open_window();
        assert!(state.drag.is_none());
    }
}
