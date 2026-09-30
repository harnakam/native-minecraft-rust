//! Inventory state, held-item sync, and transaction handling.

use rmc_net::codec::play::{
    ClickWindowPacket, CloseWindowPacket, CloseWindowServerboundPacket,
    ConfirmTransactionClientboundPacket, ConfirmTransactionServerboundPacket, HeldItemChangePacket,
    OpenWindowPacket, PlayClientboundPacket, PlayServerboundPacket, SetSlotPacket, Slot,
    WindowItemsPacket,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContainerMetadata {
    pub inventory_type: String,
    pub window_title_json: String,
    pub slot_count: u8,
    pub entity_id: Option<i32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContainerSnapshot {
    pub window_id: u8,
    pub slots: Vec<Slot>,
    pub metadata: Option<ContainerMetadata>,
}

impl ContainerSnapshot {
    pub fn player_inventory_offset(&self) -> usize {
        let Some(metadata) = self.metadata.as_ref() else {
            return 9;
        };
        match metadata.inventory_type.as_str() {
            "minecraft:crafting_table" => 10,
            "minecraft:furnace" | "minecraft:anvil" | "minecraft:villager" => 3,
            "minecraft:brewing_stand" => 4,
            "minecraft:enchanting_table" => 2,
            "minecraft:beacon" => 1,
            _ => metadata.slot_count as usize,
        }
    }
    pub fn slot(&self, slot_id: i16) -> Option<&Slot> {
        let slot_id = usize::try_from(slot_id).ok()?;
        self.slots.get(slot_id)
    }

    fn set_slot(&mut self, slot_id: i16, item: Slot) {
        let Ok(slot_id) = usize::try_from(slot_id) else {
            return;
        };

        if self.slots.len() <= slot_id {
            self.slots.resize(slot_id + 1, None);
        }

        self.slots[slot_id] = item;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingTransaction {
    pub window_id: u8,
    pub action_number: i16,
    pub slot_id: i16,
    pub button: i8,
    pub mode: i8,
    pub clicked_item: Slot,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct InventoryUpdate {
    pub outbound_packets: Vec<PlayServerboundPacket>,
    pub updated_windows: Vec<u8>,
    pub closed_windows: Vec<u8>,
    pub cursor_changed: bool,
    pub accepted_transactions: Vec<(u8, i16)>,
    pub rejected_transactions: Vec<(u8, i16)>,
}

impl InventoryUpdate {
    fn touch_window(&mut self, window_id: u8) {
        if !self.updated_windows.contains(&window_id) {
            self.updated_windows.push(window_id);
        }
    }

    fn close_window(&mut self, window_id: u8) {
        if !self.closed_windows.contains(&window_id) {
            self.closed_windows.push(window_id);
        }
    }
}

pub struct InventoryState {
    selected_hotbar_slot: u8,
    inventory_window: ContainerSnapshot,
    open_window: Option<ContainerSnapshot>,
    player_inventory_open: bool,
    carried_item: Slot,
    pending_transactions: Vec<PendingTransaction>,
    next_action_numbers: BTreeMap<u8, i16>,
    pickup_predictions: BTreeMap<(u8, i16), (ContainerSnapshot, Slot)>,
}

impl Default for InventoryState {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryState {
    pub fn new() -> Self {
        Self {
            selected_hotbar_slot: 0,
            inventory_window: ContainerSnapshot {
                window_id: 0,
                slots: vec![None; 45],
                metadata: None,
            },
            open_window: None,
            player_inventory_open: false,
            carried_item: None,
            pending_transactions: Vec::new(),
            next_action_numbers: BTreeMap::new(),
            pickup_predictions: BTreeMap::new(),
        }
    }

    pub fn reset_for_respawn(&mut self) {
        *self = Self::new();
    }

    pub fn selected_hotbar_slot(&self) -> u8 {
        self.selected_hotbar_slot
    }

    pub fn inventory_window(&self) -> &ContainerSnapshot {
        &self.inventory_window
    }

    pub fn selected_hotbar_item(&self) -> Slot {
        let slot_id = 36 + i16::from(self.selected_hotbar_slot);
        self.inventory_window.slot(slot_id).cloned().unwrap_or(None)
    }

    pub fn open_window(&self) -> Option<&ContainerSnapshot> {
        if self.player_inventory_open {
            Some(&self.inventory_window)
        } else {
            self.open_window.as_ref()
        }
    }
    pub fn open_player_inventory(&mut self) -> PlayServerboundPacket {
        self.player_inventory_open = true;
        PlayServerboundPacket::ClientStatus(2)
    }

    pub fn carried_item(&self) -> &Slot {
        &self.carried_item
    }

    pub fn pending_transactions(&self) -> &[PendingTransaction] {
        &self.pending_transactions
    }

    pub fn sync_selected_hotbar_slot(&mut self, slot: u8) -> Option<PlayServerboundPacket> {
        if slot > 8 || slot == self.selected_hotbar_slot {
            return None;
        }

        self.selected_hotbar_slot = slot;
        Some(PlayServerboundPacket::HeldItemChange(
            HeldItemChangePacket {
                slot: i16::from(slot),
            },
        ))
    }

    pub fn close_open_window(&mut self) -> Option<PlayServerboundPacket> {
        if self.player_inventory_open {
            self.player_inventory_open = false;
            return Some(PlayServerboundPacket::CloseWindow(
                CloseWindowServerboundPacket { window_id: 0 },
            ));
        }
        let window_id = self.open_window.as_ref()?.window_id;
        self.drop_open_window(window_id);
        Some(PlayServerboundPacket::CloseWindow(
            CloseWindowServerboundPacket { window_id },
        ))
    }

    pub fn queue_click(
        &mut self,
        window_id: u8,
        slot_id: i16,
        button: i8,
        mode: i8,
        clicked_item: Slot,
    ) -> PlayServerboundPacket {
        let action_number = self.next_action_number(window_id);
        self.pending_transactions.push(PendingTransaction {
            window_id,
            action_number,
            slot_id,
            button,
            mode,
            clicked_item: clicked_item.clone(),
        });

        PlayServerboundPacket::ClickWindow(ClickWindowPacket {
            window_id,
            slot_id,
            button,
            action_number,
            mode,
            clicked_item,
        })
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

    pub fn apply_open_window(&mut self, packet: &OpenWindowPacket) -> InventoryUpdate {
        self.player_inventory_open = false;
        self.open_window = Some(ContainerSnapshot {
            window_id: packet.window_id,
            slots: vec![None; packet.slot_count as usize],
            metadata: Some(ContainerMetadata {
                inventory_type: packet.inventory_type.clone(),
                window_title_json: packet.window_title_json.clone(),
                slot_count: packet.slot_count,
                entity_id: packet.entity_id,
            }),
        });

        let mut update = InventoryUpdate::default();
        update.touch_window(packet.window_id);
        update
    }

    pub fn apply_close_window(&mut self, packet: &CloseWindowPacket) -> InventoryUpdate {
        self.drop_open_window(packet.window_id);

        let mut update = InventoryUpdate::default();
        update.close_window(packet.window_id);
        update
    }

    pub fn apply_set_slot(&mut self, packet: &SetSlotPacket) -> InventoryUpdate {
        let mut update = InventoryUpdate::default();

        match packet.window_id {
            -1 => {
                if packet.slot_id == -1 {
                    self.carried_item = packet.item.clone();
                    update.cursor_changed = true;
                }
            }
            -2 => {
                let slot = match packet.slot_id {
                    0..=8 => packet.slot_id + 36,
                    9..=35 => packet.slot_id,
                    36..=39 => 44 - packet.slot_id,
                    _ => return update,
                };
                self.inventory_window.set_slot(slot, packet.item.clone());
                self.sync_player_inventory_to_open();
                update.touch_window(0);
            }
            0 => {
                self.inventory_window
                    .set_slot(packet.slot_id, packet.item.clone());
                self.sync_player_inventory_to_open();
                update.touch_window(0);
            }
            positive if positive > 0 => {
                let window_id = positive as u8;
                let previous_player_slots = self.inventory_window.slots.clone();
                self.ensure_open_window(window_id)
                    .set_slot(packet.slot_id, packet.item.clone());
                self.sync_open_player_inventory();
                update.touch_window(window_id);
                if self.inventory_window.slots != previous_player_slots {
                    update.touch_window(0);
                }
            }
            _ => {}
        }

        update
    }

    pub fn apply_window_items(&mut self, packet: &WindowItemsPacket) -> InventoryUpdate {
        if packet.window_id == 0 {
            self.inventory_window.slots = packet.items.clone();
            self.sync_player_inventory_to_open();
        } else {
            let metadata = self
                .open_window
                .as_ref()
                .filter(|window| window.window_id == packet.window_id)
                .and_then(|window| window.metadata.clone());

            self.open_window = Some(ContainerSnapshot {
                window_id: packet.window_id,
                slots: packet.items.clone(),
                metadata,
            });
        }

        if packet.window_id != 0 {
            self.sync_open_player_inventory();
        }
        let mut update = InventoryUpdate::default();
        update.touch_window(packet.window_id);
        if packet.window_id != 0 {
            update.touch_window(0);
        }
        update
    }

    pub fn apply_confirm_transaction(
        &mut self,
        packet: &ConfirmTransactionClientboundPacket,
    ) -> InventoryUpdate {
        let prediction = self
            .pickup_predictions
            .remove(&(packet.window_id, packet.action_number));
        self.pending_transactions.retain(|pending| {
            !(pending.window_id == packet.window_id
                && pending.action_number == packet.action_number)
        });

        let mut update = InventoryUpdate::default();

        if packet.accepted {
            update
                .accepted_transactions
                .push((packet.window_id, packet.action_number));
        } else {
            if let Some((window, cursor)) = prediction {
                if packet.window_id == 0 {
                    self.inventory_window = window;
                    self.sync_player_inventory_to_open();
                } else if self
                    .open_window
                    .as_ref()
                    .is_some_and(|open| open.window_id == packet.window_id)
                {
                    self.open_window = Some(window);
                    self.sync_open_player_inventory();
                }
                self.carried_item = cursor;
                self.pickup_predictions
                    .retain(|(window_id, _), _| *window_id != packet.window_id);
                update.touch_window(packet.window_id);
                update.cursor_changed = true;
            }
            update
                .rejected_transactions
                .push((packet.window_id, packet.action_number));
            update
                .outbound_packets
                .push(PlayServerboundPacket::ConfirmTransaction(
                    ConfirmTransactionServerboundPacket {
                        window_id: packet.window_id,
                        action_number: packet.action_number,
                        accepted: true,
                    },
                ));
        }

        update
    }

    pub fn apply_play_packet(&mut self, packet: &PlayClientboundPacket) -> InventoryUpdate {
        match packet {
            PlayClientboundPacket::HeldItemChange(packet) => {
                if (0..=8).contains(&packet.slot) {
                    self.selected_hotbar_slot = packet.slot as u8;
                }
                InventoryUpdate::default()
            }
            PlayClientboundPacket::OpenWindow(packet) => self.apply_open_window(packet),
            PlayClientboundPacket::CloseWindow(packet) => self.apply_close_window(packet),
            PlayClientboundPacket::SetSlot(packet) => self.apply_set_slot(packet),
            PlayClientboundPacket::WindowItems(packet) => self.apply_window_items(packet),
            PlayClientboundPacket::ConfirmTransaction(packet) => {
                self.apply_confirm_transaction(packet)
            }
            _ => InventoryUpdate::default(),
        }
    }

    fn slot_rules(
        &self,
        window_id: u8,
        slot_id: i16,
        item: Option<&rmc_net::codec::play::ItemStack>,
    ) -> (bool, u8) {
        let Some(item) = item else {
            return (true, 64);
        };
        let mut limit = item_stack_limit(item.item_id);
        let valid = if window_id == 0 {
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
            match self
                .open_window
                .as_ref()
                .and_then(|w| w.metadata.as_ref())
                .map(|m| m.inventory_type.as_str())
            {
                Some("minecraft:crafting_table") if slot_id == 0 => false,
                Some("minecraft:furnace" | "minecraft:anvil" | "minecraft:villager")
                    if slot_id == 2 =>
                {
                    false
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
                Some("minecraft:beacon") if slot_id == 0 => {
                    limit = 1;
                    matches!(item.item_id, 264 | 265 | 266 | 388)
                }
                Some("EntityHorse") if slot_id == 0 => {
                    limit = 1;
                    item.item_id == 329
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

    fn sync_open_player_inventory(&mut self) {
        let Some(window) = self.open_window.as_ref() else {
            return;
        };
        if window.metadata.is_none() {
            return;
        }
        let offset = window.player_inventory_offset();
        if window.slots.len() < offset + 36 {
            return;
        }
        for index in 0..36 {
            self.inventory_window
                .set_slot((index + 9) as i16, window.slots[offset + index].clone());
        }
    }

    fn sync_player_inventory_to_open(&mut self) {
        let Some(window) = self.open_window.as_mut() else {
            return;
        };
        if window.metadata.is_none() {
            return;
        }
        let offset = window.player_inventory_offset();
        if window.slots.len() < offset + 36 {
            return;
        }
        for index in 0..36 {
            window.slots[offset + index] = self
                .inventory_window
                .slots
                .get(index + 9)
                .cloned()
                .unwrap_or(None);
        }
    }

    fn slot(&self, window_id: u8, slot_id: i16) -> Option<&Slot> {
        if window_id == 0 {
            self.inventory_window.slot(slot_id)
        } else {
            self.open_window
                .as_ref()
                .filter(|window| window.window_id == window_id)
                .and_then(|window| window.slot(slot_id))
        }
    }

    fn next_action_number(&mut self, window_id: u8) -> i16 {
        let next = self.next_action_numbers.entry(window_id).or_insert(1);
        let action_number = *next;
        *next = next.wrapping_add(1);
        action_number
    }

    fn ensure_open_window(&mut self, window_id: u8) -> &mut ContainerSnapshot {
        if self
            .open_window
            .as_ref()
            .map(|window| window.window_id != window_id)
            .unwrap_or(true)
        {
            self.open_window = Some(ContainerSnapshot {
                window_id,
                slots: Vec::new(),
                metadata: None,
            });
        }

        self.open_window.as_mut().expect("open window must exist")
    }

    fn drop_open_window(&mut self, window_id: u8) {
        if self
            .open_window
            .as_ref()
            .map(|window| window.window_id == window_id)
            .unwrap_or(false)
        {
            self.open_window = None;
        }

        self.pending_transactions
            .retain(|pending| pending.window_id != window_id);
        self.pickup_predictions
            .retain(|(id, _), _| *id != window_id);
    }
}

fn predict_pickup(slot: Slot, cursor: Slot, button: i8, valid: bool, limit: u8) -> (Slot, Slot) {
    if !valid && cursor.is_some() {
        if let (Some(slot), Some(cursor)) = (&slot, &cursor) {
            if slot.item_id == cursor.item_id
                && slot.damage == cursor.damage
                && slot.tags_equal(&cursor)
                && u16::from(slot.count) + u16::from(cursor.count)
                    <= u16::from(item_stack_limit(cursor.item_id))
            {
                let mut cursor = cursor.clone();
                cursor.count += slot.count;
                return (None, Some(cursor));
            }
        }
        return (slot, cursor);
    }
    match (slot, cursor) {
        (None, None) => (None, None),
        (Some(mut stack), None) => {
            let amount = if button == 0 {
                stack.count
            } else {
                stack.count.div_ceil(2)
            };
            let mut cursor = stack.clone();
            cursor.count = amount;
            stack.count -= amount;
            ((stack.count > 0).then_some(stack), Some(cursor))
        }
        (None, Some(mut cursor)) => {
            let mut placed = cursor.clone();
            placed.count = if button == 0 {
                cursor.count.min(limit)
            } else {
                1
            };
            cursor.count -= placed.count;
            (Some(placed), (cursor.count > 0).then_some(cursor))
        }
        (Some(mut slot), Some(mut cursor))
            if slot.item_id == cursor.item_id
                && slot.damage == cursor.damage
                && slot.tags_equal(&cursor) =>
        {
            let capacity = limit.saturating_sub(slot.count);
            let transfer = capacity.min(if button == 0 { cursor.count } else { 1 });
            slot.count += transfer;
            cursor.count -= transfer;
            (Some(slot), (cursor.count > 0).then_some(cursor))
        }
        (slot, cursor) => {
            if cursor.as_ref().is_some_and(|item| item.count > limit) {
                (slot, cursor)
            } else {
                (cursor, slot)
            }
        }
    }
}

fn item_stack_limit(id: i16) -> u8 {
    match id {
        256 | 257 | 258 | 259 | 261 | 267 | 268 | 269 | 270 | 271 | 272 | 273 | 274 | 275 | 276
        | 277 | 278 | 279 | 282 | 283 | 284 | 285 | 286 | 290 | 291 | 292 | 293 | 294 | 298
        | 299 | 300 | 301 | 302 | 303 | 304 | 305 | 306 | 307 | 308 | 309 | 310 | 311 | 312
        | 313 | 314 | 315 | 316 | 317 | 326 | 327 | 328 | 329 | 333 | 335 | 342 | 343 | 346
        | 354 | 355 | 359 | 373 | 386 | 398 | 403 | 407 | 408 | 413 | 417 | 418 | 419 | 422
        | 2256 | 2257 | 2258 | 2259 | 2260 | 2261 | 2262 | 2263 | 2264 | 2265 | 2266 | 2267 => 1,
        323 | 325 | 332 | 344 | 368 | 387 | 416 | 425 => 16,
        _ => 64,
    }
}

#[cfg(test)]
mod tests {
    use super::InventoryState;
    use rmc_net::codec::play::{
        CloseWindowPacket, ConfirmTransactionClientboundPacket, ItemStack, OpenWindowPacket,
        PlayServerboundPacket, SetSlotPacket, WindowItemsPacket,
    };

    #[test]
    fn held_item_change_is_only_sent_on_slot_change() {
        let mut inventory = InventoryState::new();
        assert!(inventory.sync_selected_hotbar_slot(0).is_none());
        assert!(matches!(
            inventory.sync_selected_hotbar_slot(2),
            Some(PlayServerboundPacket::HeldItemChange(_))
        ));
        assert!(inventory.sync_selected_hotbar_slot(2).is_none());
    }

    #[test]
    fn window_items_replace_inventory_window() {
        let mut inventory = InventoryState::new();
        let update = inventory.apply_window_items(&WindowItemsPacket {
            window_id: 0,
            items: vec![Some(ItemStack::simple(276, 1, 0)), None],
        });

        assert_eq!(update.updated_windows, vec![0]);
        assert_eq!(inventory.inventory_window().slots.len(), 2);
    }

    #[test]
    fn open_window_set_slot_and_close_flow() {
        let mut inventory = InventoryState::new();
        let update = inventory.apply_open_window(&OpenWindowPacket {
            window_id: 4,
            inventory_type: "minecraft:chest".to_owned(),
            window_title_json: "{\"text\":\"Loot\"}".to_owned(),
            slot_count: 27,
            entity_id: None,
        });
        assert_eq!(update.updated_windows, vec![4]);
        assert_eq!(
            inventory
                .open_window()
                .and_then(|window| window.metadata.as_ref())
                .map(|metadata| metadata.slot_count),
            Some(27)
        );

        let update = inventory.apply_set_slot(&SetSlotPacket {
            window_id: 4,
            slot_id: 10,
            item: Some(ItemStack::simple(5, 16, 0)),
        });
        assert_eq!(update.updated_windows, vec![4]);
        assert_eq!(
            inventory
                .open_window()
                .and_then(|window| window.slot(10))
                .cloned()
                .flatten(),
            Some(ItemStack::simple(5, 16, 0))
        );

        let packet = inventory
            .close_open_window()
            .expect("close window packet should exist");
        assert!(matches!(
            packet,
            PlayServerboundPacket::CloseWindow(packet) if packet.window_id == 4
        ));

        let update = inventory.apply_close_window(&CloseWindowPacket { window_id: 4 });
        assert_eq!(update.closed_windows, vec![4]);
        assert!(inventory.open_window().is_none());
    }

    #[test]
    fn set_slot_updates_carried_item() {
        let mut inventory = InventoryState::new();
        let update = inventory.apply_set_slot(&SetSlotPacket {
            window_id: -1,
            slot_id: -1,
            item: Some(ItemStack::simple(261, 1, 0)),
        });

        assert!(update.cursor_changed);
        assert_eq!(
            inventory.carried_item().clone(),
            Some(ItemStack::simple(261, 1, 0))
        );
    }

    #[test]
    fn rejected_transaction_emits_vanilla_ack() {
        let mut inventory = InventoryState::new();
        let packet = inventory.queue_pickup_click(0, 36, 0);
        assert!(matches!(packet, PlayServerboundPacket::ClickWindow(_)));

        let update = inventory.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
            window_id: 0,
            action_number: 1,
            accepted: false,
        });

        assert_eq!(update.rejected_transactions, vec![(0, 1)]);
        assert_eq!(update.outbound_packets.len(), 1);
        assert!(matches!(
            &update.outbound_packets[0],
            PlayServerboundPacket::ConfirmTransaction(packet)
                if packet.window_id == 0 && packet.action_number == 1 && packet.accepted
        ));
        assert!(inventory.pending_transactions().is_empty());
    }

    #[test]
    fn action_numbers_increment_per_window() {
        let mut inventory = InventoryState::new();
        let first = inventory.queue_pickup_click(0, 10, 0);
        let second = inventory.queue_pickup_click(0, 11, 0);

        assert!(matches!(
            first,
            PlayServerboundPacket::ClickWindow(packet) if packet.action_number == 1
        ));
        assert!(matches!(
            second,
            PlayServerboundPacket::ClickWindow(packet) if packet.action_number == 2
        ));
    }
}
