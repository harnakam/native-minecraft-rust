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
                slots: vec![None; 46],
                metadata: None,
            },
            open_window: None,
            carried_item: None,
            pending_transactions: Vec::new(),
            next_action_numbers: BTreeMap::new(),
            pickup_predictions: BTreeMap::new(),
        }
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
        self.open_window.as_ref()
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
            if matches!(button, 0 | 1) && slot_id >= 0 && before.slot(slot_id).is_some() {
                self.pickup_predictions
                    .insert((window_id, click.action_number), (before, previous_cursor));
                let (slot, cursor) = predict_pickup(clicked_item, self.carried_item.take(), button);
                self.carried_item = cursor;
                if window_id == 0 {
                    self.inventory_window.set_slot(slot_id, slot);
                } else if let Some(window) = &mut self.open_window {
                    window.set_slot(slot_id, slot);
                }
            }
        }
        packet
    }

    pub fn apply_open_window(&mut self, packet: &OpenWindowPacket) -> InventoryUpdate {
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
                self.inventory_window
                    .set_slot(packet.slot_id, packet.item.clone());
                update.touch_window(0);
            }
            0 => {
                self.inventory_window
                    .set_slot(packet.slot_id, packet.item.clone());
                update.touch_window(0);
            }
            positive if positive > 0 => {
                let window_id = positive as u8;
                self.ensure_open_window(window_id)
                    .set_slot(packet.slot_id, packet.item.clone());
                update.touch_window(window_id);
            }
            _ => {}
        }

        update
    }

    pub fn apply_window_items(&mut self, packet: &WindowItemsPacket) -> InventoryUpdate {
        if packet.window_id == 0 {
            self.inventory_window.slots = packet.items.clone();
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

        let mut update = InventoryUpdate::default();
        update.touch_window(packet.window_id);
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
                } else if self
                    .open_window
                    .as_ref()
                    .is_some_and(|open| open.window_id == packet.window_id)
                {
                    self.open_window = Some(window);
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
        *next = next.saturating_add(1);
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

fn predict_pickup(slot: Slot, cursor: Slot, button: i8) -> (Slot, Slot) {
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
                cursor.count.min(item_stack_limit(cursor.item_id))
            } else {
                1
            };
            cursor.count -= placed.count;
            (Some(placed), (cursor.count > 0).then_some(cursor))
        }
        (Some(mut slot), Some(mut cursor))
            if slot.item_id == cursor.item_id
                && slot.damage == cursor.damage
                && slot.nbt == cursor.nbt =>
        {
            let capacity = item_stack_limit(slot.item_id).saturating_sub(slot.count);
            let transfer = capacity.min(if button == 0 { cursor.count } else { 1 });
            slot.count += transfer;
            cursor.count -= transfer;
            (Some(slot), (cursor.count > 0).then_some(cursor))
        }
        (slot, cursor) => (cursor, slot),
    }
}

fn item_stack_limit(id: i16) -> u8 {
    match id {
        256..=259
        | 261
        | 267..=279
        | 282..=286
        | 290..=294
        | 298..=317
        | 326..=329
        | 333
        | 335
        | 346
        | 354
        | 355
        | 359
        | 373
        | 386
        | 398
        | 403
        | 407
        | 408
        | 413
        | 417..=419
        | 422
        | 2256..=2267 => 1,
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
