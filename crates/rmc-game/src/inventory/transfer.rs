//! Destination rules from the individual MCP919 container implementations.
use super::ContainerSnapshot;
use rmc_net::codec::play::ItemStack;

pub(super) fn destination(
    window: &ContainerSnapshot,
    slot: usize,
    stack: &ItemStack,
) -> Result<(usize, usize, bool), &'static str> {
    let kind = window
        .metadata
        .as_ref()
        .ok_or("Missing container metadata")?
        .inventory_type
        .as_str();
    let offset = window.player_inventory_offset();
    if window.slots.len() != offset + 36 {
        return Err("Incomplete container inventory");
    }
    match kind {
        "minecraft:furnace" => {
            if slot == 2 {
                Ok((3, 39, true))
            } else if slot < 2 {
                Ok((3, 39, false))
            } else if super::furnace::smelting_result(stack).is_some() {
                Ok((0, 1, false))
            } else if super::furnace::fuel_ticks(stack.item_id) > 0 {
                Ok((1, 2, false))
            } else if slot < 30 {
                Ok((30, 39, false))
            } else {
                Ok((3, 30, false))
            }
        }
        "minecraft:container"
        | "minecraft:chest"
        | "minecraft:hopper"
        | "minecraft:dispenser"
        | "minecraft:dropper" => {
            if slot < offset {
                Ok((offset, window.slots.len(), true))
            } else {
                Ok((0, offset, false))
            }
        }
        "minecraft:beacon" => {
            if slot == 0 {
                Ok((1, 37, true))
            } else if window.slots[0].is_none()
                && stack.count == 1
                && matches!(stack.item_id, 264 | 265 | 266 | 388)
            {
                Ok((0, 1, false))
            } else if slot < 28 {
                Ok((28, 37, false))
            } else {
                Ok((1, 28, false))
            }
        }
        _ => Err("This container requires a specialized transfer algorithm"),
    }
}
