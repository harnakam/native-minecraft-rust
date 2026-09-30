//! Container click algorithms corresponding to net.minecraft.inventory.Container.
use super::{item_stack_limit, predict_pickup, InventoryState};
use rmc_net::codec::play::PlayServerboundPacket;

impl InventoryState {
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
        let mut returned = None;
        if let Some(mut stack) = item.clone() {
            let (start, end, reverse) = if window_id == 0 {
                if slot_id == 0 {
                    return Err("Recipe transfer requires crafting side effects");
                }
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
            } else {
                if !matches!(
                    before.metadata.as_ref().map(|m| m.inventory_type.as_str()),
                    Some("minecraft:container" | "minecraft:chest")
                ) {
                    return Err("This container requires a specialized transfer algorithm");
                }
                let offset = before.player_inventory_offset();
                if before.slots.len() != offset + 36 {
                    return Err("Incomplete container inventory");
                }
                if (slot_id as usize) < offset {
                    (offset, after.slots.len(), true)
                } else {
                    (0, offset, false)
                }
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
                            && target.damage == stack.damage
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
                            && stack.damage == item.damage
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
                self.pickup_predictions
                    .insert((window_id, click.action_number), (before, previous_cursor));
                let (valid, limit) =
                    self.slot_rules(window_id, slot_id, self.carried_item.as_ref());
                let (slot, cursor) =
                    predict_pickup(clicked_item, self.carried_item.take(), button, valid, limit);
                self.carried_item = cursor;
                if window_id == 0 {
                    self.inventory_window.set_slot(slot_id, slot);
                } else if let Some(window) = &mut self.open_window {
                    window.set_slot(slot_id, slot);
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
