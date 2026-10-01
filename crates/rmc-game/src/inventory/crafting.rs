//! Local crafting matching and result-slot effects, reconstructed from MCP919.
use super::{item_has_subtypes, item_stack_limit, ContainerSnapshot};
use rmc_net::codec::play::{ItemStack, Slot};
use rmc_net::nbt::{self, Tag};
#[path = "banner.rs"]
mod banner;
#[derive(Clone, Copy)]
struct Ingredient {
    id: i16,
    damage: Option<i16>,
}
impl Ingredient {
    fn matches(self, item: &ItemStack) -> bool {
        self.id == item.item_id && self.damage.is_none_or(|d| d == item.damage)
    }
}
struct Recipe {
    order: usize,
    width: usize,
    height: usize,
    inputs: Vec<Option<Ingredient>>,
    output: ItemStack,
}
#[path = "recipe_catalog.rs"]
mod recipe_catalog;
fn recipes() -> &'static [Recipe] {
    static RECIPES: std::sync::OnceLock<Vec<Recipe>> = std::sync::OnceLock::new();
    RECIPES.get_or_init(|| {
        recipe_catalog::ENTRIES
            .iter()
            .map(
                |&(order, width, height, inputs, id, count, damage)| Recipe {
                    order,
                    width,
                    height,
                    inputs: inputs
                        .iter()
                        .map(|&(id, damage)| {
                            if id < 0 {
                                None
                            } else {
                                Some(Ingredient {
                                    id,
                                    damage: (damage != 32767).then_some(damage),
                                })
                            }
                        })
                        .collect(),
                    output: ItemStack::simple(id, count, damage),
                },
            )
            .collect()
    })
}
pub(super) fn grid_width(window: &ContainerSnapshot) -> Option<usize> {
    if window.window_id == 0 {
        Some(2)
    } else if window
        .metadata
        .as_ref()
        .is_some_and(|m| m.inventory_type == "minecraft:crafting_table")
    {
        Some(3)
    } else {
        None
    }
}
fn fireworks_result(cells: &[Slot]) -> Slot {
    const COLORS: [i32; 16] = [
        1973019, 11743532, 3887386, 5320730, 2437522, 8073150, 2651799, 11250603, 4408131,
        14188952, 4312372, 14602026, 6719955, 12801229, 15435844, 15790320,
    ];
    let (mut powder, mut paper, mut stars, mut dyes, mut modifiers, mut shapes) =
        (0, 0, 0, 0, 0, 0);
    for stack in cells.iter().flatten() {
        match stack.item_id {
            289 => powder += 1,
            339 => paper += 1,
            402 => stars += 1,
            351 => dyes += 1,
            348 | 264 => modifiers += 1,
            385 | 288 | 371 | 397 => shapes += 1,
            _ => return None,
        }
    }
    if powder > 3 || paper > 1 {
        return None;
    }
    let key = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
    if powder >= 1 && paper == 1 && dyes + modifiers + shapes == 0 {
        let mut output = ItemStack::simple(401, 1, 0);
        if stars > 0 {
            let explosions: Vec<Tag> = cells
                .iter()
                .flatten()
                .filter(|s| s.item_id == 402)
                .filter_map(|s| s.nbt.as_deref().and_then(|b| nbt::parse(b).ok()))
                .filter_map(|t| match t.get("Explosion") {
                    Some(e @ Tag::Compound(_)) => Some(e.clone()),
                    _ => None,
                })
                .collect();
            let kind = if explosions.is_empty() { 0 } else { 10 };
            let fireworks = Tag::Compound(
                [
                    (key("Flight"), Tag::Byte(powder as i8)),
                    (
                        key("Explosions"),
                        Tag::List {
                            kind,
                            values: explosions,
                        },
                    ),
                ]
                .into(),
            );
            output.nbt =
                Some(nbt::encode(&Tag::Compound([(key("Fireworks"), fireworks)].into())).ok()?);
        }
        return Some(output);
    }
    if powder == 1 && paper == 0 && stars == 0 && dyes > 0 && shapes <= 1 {
        let mut explosion = std::collections::BTreeMap::new();
        let mut colors = Vec::new();
        let mut shape = 0;
        for stack in cells.iter().flatten() {
            match stack.item_id {
                351 => colors.push(COLORS[(stack.damage & 15) as usize]),
                348 => {
                    explosion.insert(key("Flicker"), Tag::Byte(1));
                }
                264 => {
                    explosion.insert(key("Trail"), Tag::Byte(1));
                }
                385 => shape = 1,
                288 => shape = 4,
                371 => shape = 2,
                397 => shape = 3,
                _ => {}
            }
        }
        explosion.insert(key("Type"), Tag::Byte(shape));
        explosion.insert(key("Colors"), Tag::Ints(colors));
        let mut output = ItemStack::simple(402, 1, 0);
        output.nbt = Some(
            nbt::encode(&Tag::Compound(
                [(key("Explosion"), Tag::Compound(explosion))].into(),
            ))
            .ok()?,
        );
        return Some(output);
    }
    if powder == 0 && paper == 0 && stars == 1 && dyes > 0 && modifiers + shapes == 0 {
        let mut output = cells.iter().flatten().find(|s| s.item_id == 402)?.clone();
        let Tag::Compound(mut tags) = nbt::parse(output.nbt.as_deref()?).ok()? else {
            return None;
        };
        let colors = cells
            .iter()
            .flatten()
            .filter(|s| s.item_id == 351)
            .map(|s| COLORS[(s.damage & 15) as usize])
            .collect();
        // getCompoundTag returns a detached empty compound for an absent/wrong type.
        // Java accepts this case but does not attach the newly written fade colors.
        if let Some(Tag::Compound(explosion)) = tags.get_mut(&key("Explosion")) {
            explosion.insert(key("FadeColors"), Tag::Ints(colors));
        }
        output.count = 1;
        output.nbt = Some(nbt::encode(&Tag::Compound(tags)).ok()?);
        return Some(output);
    }
    None
}
fn armor_dye_result(cells: &[Slot]) -> Slot {
    // Sheep dye RGB in dye-damage order, with Java float precision.
    const RGB: [[f32; 3]; 16] = [
        [0.1, 0.1, 0.1],
        [0.6, 0.2, 0.2],
        [0.4, 0.5, 0.2],
        [0.4, 0.3, 0.2],
        [0.2, 0.3, 0.7],
        [0.5, 0.25, 0.7],
        [0.3, 0.5, 0.6],
        [0.6, 0.6, 0.6],
        [0.3, 0.3, 0.3],
        [0.95, 0.5, 0.65],
        [0.5, 0.8, 0.1],
        [0.9, 0.9, 0.2],
        [0.4, 0.6, 0.85],
        [0.7, 0.3, 0.85],
        [0.85, 0.5, 0.2],
        [1.0, 1.0, 1.0],
    ];
    let mut armor = None;
    let mut dyes = 0;
    let mut samples = 0;
    let mut channels = [0i32; 3];
    let mut brightness = 0i32;
    for stack in cells.iter().flatten() {
        if (298..=301).contains(&stack.item_id) {
            if armor.is_some() {
                return None;
            }
            armor = Some(stack.clone());
            if let Some(Tag::Int(color)) = stack
                .nbt
                .as_deref()
                .and_then(|b| nbt::parse(b).ok())
                .and_then(|t| t.get("display").and_then(|d| d.get("color")).cloned())
            {
                let rgb = [(color >> 16) & 255, (color >> 8) & 255, color & 255]
                    .map(|v| v as f32 / 255.0);
                brightness = (brightness as f32 + rgb[0].max(rgb[1].max(rgb[2])) * 255.0) as i32;
                for c in 0..3 {
                    channels[c] = (channels[c] as f32 + rgb[c] * 255.0) as i32;
                }
                samples += 1;
            }
        } else if stack.item_id == 351 {
            let index = if (0..16).contains(&stack.damage) {
                stack.damage as usize
            } else {
                0
            };
            let rgb = RGB[index].map(|v| (v * 255.0) as i32);
            brightness += rgb[0].max(rgb[1].max(rgb[2]));
            for c in 0..3 {
                channels[c] += rgb[c];
            }
            samples += 1;
            dyes += 1;
        } else {
            return None;
        }
    }
    if dyes == 0 {
        return None;
    }
    let mut output = armor?;
    let mean = channels.map(|v| v / samples);
    let intensity = brightness as f32 / samples as f32;
    let peak = mean[0].max(mean[1].max(mean[2])) as f32;
    let normalized = mean.map(|v| (v as f32 * intensity / peak) as i32);
    let color = ((normalized[0] << 8) + normalized[1]) << 8 | normalized[2];
    let mut tags = match output.nbt.as_deref() {
        Some(bytes) => match nbt::parse(bytes).ok()? {
            Tag::Compound(v) => v,
            _ => return None,
        },
        None => Default::default(),
    };
    let display_key: Vec<u16> = "display".encode_utf16().collect();
    let mut display = match tags.remove(&display_key) {
        Some(Tag::Compound(v)) => v,
        _ => Default::default(),
    };
    display.insert("color".encode_utf16().collect(), Tag::Int(color));
    tags.insert(display_key, Tag::Compound(display));
    output.count = 1;
    output.nbt = Some(nbt::encode(&Tag::Compound(tags)).ok()?);
    Some(output)
}
fn clone_result(cells: &[Slot]) -> Slot {
    let original = cells
        .iter()
        .flatten()
        .find(|s| matches!(s.item_id, 358 | 387))?;
    let blank_id = if original.item_id == 358 { 395 } else { 386 };
    let mut originals = 0;
    let mut blanks = 0;
    for stack in cells.iter().flatten() {
        if stack.item_id == original.item_id {
            originals += 1;
        } else if stack.item_id == blank_id {
            blanks += 1;
        } else {
            return None;
        }
    }
    if originals != 1 || blanks == 0 {
        return None;
    }
    if original.item_id == 358 {
        let mut output = ItemStack::simple(358, blanks + 1, original.damage);
        if let Some(Tag::String(name)) = original
            .nbt
            .as_deref()
            .and_then(|b| nbt::parse(b).ok())
            .and_then(|t| t.get("display").and_then(|d| d.get("Name")).cloned())
        {
            let display =
                Tag::Compound([("Name".encode_utf16().collect(), Tag::String(name))].into());
            output.nbt = Some(
                nbt::encode(&Tag::Compound(
                    [("display".encode_utf16().collect(), display)].into(),
                ))
                .ok()?,
            );
        }
        Some(output)
    } else {
        let Tag::Compound(mut tags) = nbt::parse(original.nbt.as_deref()?).ok()? else {
            return None;
        };
        let key: Vec<u16> = "generation".encode_utf16().collect();
        let generation = match tags.get(&key) {
            Some(Tag::Byte(v)) => i32::from(*v),
            Some(Tag::Short(v)) => i32::from(*v),
            Some(Tag::Int(v)) => *v,
            Some(Tag::Long(v)) => *v as i32,
            Some(Tag::Float(v)) => java_floor(f64::from(*v)),
            Some(Tag::Double(v)) => java_floor(*v),
            _ => 0,
        };
        if generation >= 2 {
            return None;
        }
        tags.insert(key, Tag::Int(generation.wrapping_add(1)));
        let mut output = ItemStack::simple(387, blanks, 0);
        output.nbt = Some(nbt::encode(&Tag::Compound(tags)).ok()?);
        Some(output)
    }
}
// NBT numeric getters use MathHelper.floor rather than a truncating cast.
fn java_floor(value: f64) -> i32 {
    let truncated = value as i32;
    if value < f64::from(truncated) {
        truncated.wrapping_sub(1)
    } else {
        truncated
    }
}
fn repair_result(cells: &[Slot]) -> Slot {
    let mut items = cells.iter().flatten();
    let first = items.next()?;
    let second = items.next()?;
    if items.next().is_some()
        || first.item_id != second.item_id
        || first.count != 1
        || second.count != 1
    {
        return None;
    }
    let (_, subtypes, max_damage) = super::item_properties::properties(first.item_id)?;
    if subtypes || max_damage == 0 {
        return None;
    }
    let max = i32::from(max_damage);
    let damage = (i32::from(first.damage) + i32::from(second.damage) - max - max * 5 / 100).max(0);
    // Crafting creates a fresh item and intentionally removes ingredient NBT/enchantments.
    Some(ItemStack::simple(first.item_id, 1, damage as i16))
}

pub(super) fn result(window: &ContainerSnapshot) -> Slot {
    let width = grid_width(window)?;
    if window.slots.len() < 1 + width * width {
        return None;
    }
    let cells = &window.slots[1..=width * width];
    let cloning = clone_result(cells);
    // Retain the reference registry positions across static and dynamic recipes.
    // Position 72 is map extension, pending world map-data integration.
    let dynamic = [
        (0, armor_dye_result(cells)),
        (1, fireworks_result(cells)),
        (2, banner::add_pattern(cells)),
        (70, cloning.clone().filter(|s| s.item_id == 387)),
        (71, cloning.filter(|s| s.item_id == 358)),
        (216, repair_result(cells)),
        (280, banner::duplicate(cells)),
    ]
    .into_iter()
    .find_map(|(order, output)| output.map(|s| (order, s)));
    for recipe in recipes() {
        if dynamic
            .as_ref()
            .is_some_and(|(order, _)| *order < recipe.order)
        {
            return dynamic.map(|(_, s)| s);
        }
        if recipe.width == 0 {
            let mut remaining = recipe.inputs.clone();
            let mut matches = true;
            for item in window.slots[1..=width * width].iter().flatten() {
                if let Some(index) = remaining
                    .iter()
                    .position(|ingredient| ingredient.is_some_and(|i| i.matches(item)))
                {
                    remaining.remove(index);
                } else {
                    matches = false;
                    break;
                }
            }
            if matches && remaining.is_empty() {
                return Some(recipe.output.clone());
            }
            continue;
        }
        for ox in 0..=3 - recipe.width {
            for oy in 0..=3 - recipe.height {
                for mirror in [true, false] {
                    let mut matches = true;
                    for x in 0..3 {
                        for y in 0..3 {
                            let ingredient = if x >= ox
                                && y >= oy
                                && x < ox + recipe.width
                                && y < oy + recipe.height
                            {
                                let rx = if mirror {
                                    recipe.width - 1 - (x - ox)
                                } else {
                                    x - ox
                                };
                                recipe.inputs[(y - oy) * recipe.width + rx]
                            } else {
                                None
                            };
                            let actual = if x < width && y < width {
                                window.slots[1 + y * width + x].as_ref()
                            } else {
                                None
                            };
                            if !match (ingredient, actual) {
                                (None, None) => true,
                                (Some(i), Some(a)) => i.matches(a),
                                _ => false,
                            } {
                                matches = false;
                            }
                        }
                    }
                    if matches {
                        return Some(recipe.output.clone());
                    }
                }
            }
        }
    }
    dynamic.map(|(_, s)| s)
}
pub(super) fn refresh(window: &mut ContainerSnapshot) {
    if grid_width(window).is_some() && !window.slots.is_empty() {
        window.slots[0] = result(window);
    }
}
pub(super) fn consume(window: &mut ContainerSnapshot) {
    let Some(width) = grid_width(window) else {
        return;
    };
    let cloning_banner = banner::duplicate(&window.slots[1..=width * width]).is_some();
    let cloning_book =
        clone_result(&window.slots[1..=width * width]).is_some_and(|s| s.item_id == 387);
    for slot in 1..=width * width {
        if cloning_book
            && window.slots[slot]
                .as_ref()
                .is_some_and(|s| s.item_id == 387)
        {
            continue;
        }
        let Some(mut stack) = window.slots[slot].take() else {
            continue;
        };
        let remainder = if cloning_banner && banner::pattern_count(&stack) > 0 {
            let mut original = stack.clone();
            original.count = 1;
            Some(original)
        } else {
            matches!(stack.item_id, 326 | 327 | 335).then(|| ItemStack::simple(325, 1, 0))
        };
        stack.count = stack.count.saturating_sub(1);
        window.slots[slot] = (stack.count > 0).then_some(stack);
        if let Some(remainder) = remainder {
            if window.slots[slot].is_none() {
                window.slots[slot] = Some(remainder);
            } else {
                // InventoryPlayer searches hotbar before main storage for spare container items.
                let offset = window.player_inventory_offset();
                let indices: Vec<usize> = (offset + 27..offset + 36)
                    .chain(offset..offset + 27)
                    .filter(|&i| i < window.slots.len())
                    .collect();
                if let Some(&index) = indices.iter().find(|&&i| {
                    window.slots[i].as_ref().is_some_and(|s| {
                        s.item_id == remainder.item_id
                            && s.tags_equal(&remainder)
                            && s.damage == remainder.damage
                            && s.count < item_stack_limit(remainder.item_id)
                    })
                }) {
                    window.slots[index].as_mut().unwrap().count += 1;
                } else if let Some(&index) = indices.iter().find(|&&i| window.slots[i].is_none()) {
                    window.slots[index] = Some(remainder);
                }
                // A full inventory leaves the remainder to the authoritative server's drop/spawn.
            }
        }
    }
    refresh(window);
}

pub(super) fn transfer_output(window: &mut ContainerSnapshot) -> Slot {
    let offset = window.player_inventory_offset();
    if window.slots.len() < offset + 36 {
        return None;
    }
    let mut returned = None;
    while let Some(output) = window.slots[0].clone() {
        let mut stack = output.clone();
        let indices: Vec<usize> = (offset..offset + 36).rev().collect();
        for &index in &indices {
            if let Some(target) = window.slots[index].as_mut() {
                if target.item_id == stack.item_id
                    && (!item_has_subtypes(stack.item_id) || target.damage == stack.damage)
                    && target.tags_equal(&stack)
                {
                    let amount = stack
                        .count
                        .min(item_stack_limit(stack.item_id).saturating_sub(target.count));
                    target.count += amount;
                    stack.count -= amount;
                }
            }
            if stack.count == 0 {
                break;
            }
        }
        if stack.count > 0 {
            if let Some(&index) = indices.iter().find(|&&i| window.slots[i].is_none()) {
                window.slots[index] = Some(stack.clone());
                stack.count = 0;
            }
        }
        if stack.count == output.count {
            break;
        }
        if returned.is_none() {
            returned = Some(output.clone());
        }
        consume(window);
        if window.slots[0]
            .as_ref()
            .is_none_or(|next| next.item_id != output.item_id)
        {
            break;
        }
    }
    returned
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::InventoryState;
    use rmc_net::codec::play::{
        ConfirmTransactionClientboundPacket, PlayServerboundPacket, SetSlotPacket,
    };
    #[test]
    fn workbench_number_key_result_updates_hotbar_alias_and_rolls_back() {
        let mut state = InventoryState::new();
        let mut window = ContainerSnapshot {
            window_id: 1,
            slots: vec![None; 46],
            properties: Default::default(),
            metadata: Some(super::super::ContainerMetadata {
                inventory_type: "minecraft:crafting_table".into(),
                window_title_json: "{}".into(),
                slot_count: 10,
                entity_id: None,
            }),
        };
        window.slots[1] = Some(ItemStack::simple(5, 1, 0));
        window.slots[4] = Some(ItemStack::simple(5, 1, 0));
        refresh(&mut window);
        let before = window.clone();
        let player_before = state.inventory_window.clone();
        state.open_window = Some(window);
        let PlayServerboundPacket::ClickWindow(click) = state.queue_hotbar_swap(1, 0, 0).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            state.inventory_window.slots[36],
            Some(ItemStack::simple(280, 4, 0))
        );
        assert_eq!(
            state.open_window.as_ref().unwrap().slots[37],
            state.inventory_window.slots[36]
        );
        for slot in [0, 1, 4] {
            assert!(state.open_window.as_ref().unwrap().slots[slot].is_none());
        }
        state.apply_confirm_transaction(
            &rmc_net::codec::play::ConfirmTransactionClientboundPacket {
                window_id: 1,
                action_number: click.action_number,
                accepted: false,
            },
        );
        assert_eq!(state.open_window, Some(before));
        assert_eq!(state.inventory_window, player_before);
    }
    #[test]
    fn result_number_key_moves_output_rehomes_hotbar_and_restores_rejection() {
        for occupied in [false, true] {
            let mut state = InventoryState::new();
            state.inventory_window.slots[1] = Some(ItemStack::simple(5, 1, 0));
            state.inventory_window.slots[3] = Some(ItemStack::simple(5, 1, 0));
            if occupied {
                state.inventory_window.slots[36] = Some(ItemStack::simple(1, 12, 0));
            }
            refresh(&mut state.inventory_window);
            let before = state.inventory_window.clone();
            let PlayServerboundPacket::ClickWindow(click) =
                state.queue_hotbar_swap(0, 0, 0).unwrap()
            else {
                panic!()
            };
            assert_eq!((click.mode, click.button, click.clicked_item), (2, 0, None));
            assert_eq!(
                state.inventory_window.slots[36],
                Some(ItemStack::simple(280, 4, 0))
            );
            assert!(state.inventory_window.slots[0..4]
                .iter()
                .all(Option::is_none));
            if occupied {
                assert_eq!(
                    state.inventory_window.slots[37],
                    Some(ItemStack::simple(1, 12, 0))
                );
            }
            state.apply_confirm_transaction(
                &rmc_net::codec::play::ConfirmTransactionClientboundPacket {
                    window_id: 0,
                    action_number: click.action_number,
                    accepted: false,
                },
            );
            assert_eq!(state.inventory_window, before);
        }
    }
    #[test]
    fn full_storage_blocks_occupied_hotbar_result_and_input_swap_recalculates() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(5, 1, 0));
        state.inventory_window.slots[3] = Some(ItemStack::simple(5, 1, 0));
        for slot in 9..45 {
            state.inventory_window.slots[slot] = Some(ItemStack::simple(1, 64, 0));
        }
        refresh(&mut state.inventory_window);
        let before = state.inventory_window.clone();
        state.queue_hotbar_swap(0, 0, 0).unwrap();
        assert_eq!(state.inventory_window, before);
        state.inventory_window.slots[36] = None;
        state.queue_hotbar_swap(0, 1, 0).unwrap();
        assert_eq!(
            state.inventory_window.slots[36],
            Some(ItemStack::simple(5, 1, 0))
        );
        assert!(state.inventory_window.slots[1].is_none());
        assert_eq!(
            state.inventory_window.slots[0],
            Some(ItemStack::simple(143, 1, 0))
        );
    }
    #[test]
    fn workbench_result_throw_consumes_and_rejection_restores_both_views() {
        let mut state = InventoryState::new();
        let mut window = ContainerSnapshot {
            window_id: 1,
            slots: vec![None; 46],
            properties: Default::default(),
            metadata: Some(super::super::ContainerMetadata {
                inventory_type: "minecraft:crafting_table".into(),
                window_title_json: "{}".into(),
                slot_count: 10,
                entity_id: None,
            }),
        };
        window.slots[1] = Some(ItemStack::simple(5, 1, 0));
        window.slots[4] = Some(ItemStack::simple(5, 1, 0));
        refresh(&mut window);
        let before = window.clone();
        let player_before = state.inventory_window.clone();
        state.open_window = Some(window);
        let PlayServerboundPacket::ClickWindow(click) =
            state.queue_throw_click(1, 0, false).unwrap()
        else {
            panic!()
        };
        for slot in [0, 1, 4] {
            assert!(state.open_window.as_ref().unwrap().slots[slot].is_none());
        }
        state.apply_confirm_transaction(
            &rmc_net::codec::play::ConfirmTransactionClientboundPacket {
                window_id: 1,
                action_number: click.action_number,
                accepted: false,
            },
        );
        assert_eq!(state.open_window, Some(before));
        assert_eq!(state.inventory_window, player_before);
    }
    #[test]
    fn throwing_result_takes_whole_output_consumes_once_and_reject_restores() {
        for whole in [false, true] {
            let mut state = InventoryState::new();
            state.inventory_window.slots[1] = Some(ItemStack::simple(5, 2, 0));
            state.inventory_window.slots[3] = Some(ItemStack::simple(5, 2, 0));
            refresh(&mut state.inventory_window);
            let before = state.inventory_window.clone();
            let PlayServerboundPacket::ClickWindow(click) =
                state.queue_throw_click(0, 0, whole).unwrap()
            else {
                panic!()
            };
            assert_eq!(
                (click.mode, click.button, click.clicked_item),
                (4, i8::from(whole), None)
            );
            assert_eq!(state.inventory_window.slots[1].as_ref().unwrap().count, 1);
            assert_eq!(state.inventory_window.slots[3].as_ref().unwrap().count, 1);
            assert_eq!(
                state.inventory_window.slots[0],
                Some(ItemStack::simple(280, 4, 0))
            );
            assert!(state.carried_item.is_none());
            state.apply_confirm_transaction(
                &rmc_net::codec::play::ConfirmTransactionClientboundPacket {
                    window_id: 0,
                    action_number: click.action_number,
                    accepted: false,
                },
            );
            assert_eq!(state.inventory_window, before);
        }
    }
    #[test]
    fn throwing_crafting_input_refreshes_result_and_carried_stack_blocks_it() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(5, 1, 0));
        state.inventory_window.slots[3] = Some(ItemStack::simple(5, 1, 0));
        refresh(&mut state.inventory_window);
        state.carried_item = Some(ItemStack::simple(1, 1, 0));
        let before = state.inventory_window.clone();
        state.queue_throw_click(0, 1, false).unwrap();
        assert_eq!(state.inventory_window, before);
        state.carried_item = None;
        state.queue_throw_click(0, 1, false).unwrap();
        assert_eq!(
            state.inventory_window.slots[0],
            Some(ItemStack::simple(143, 1, 0))
        );
    }
    #[test]
    fn armor_dye_preserves_tags_damage_and_consumes_ingredients() {
        let mut state = InventoryState::new();
        let mut armor = ItemStack::simple(299, 1, 17);
        let tag = Tag::Compound([("custom".encode_utf16().collect(), Tag::Int(42))].into());
        armor.nbt = Some(nbt::encode(&tag).unwrap());
        state.inventory_window.slots[1] = Some(armor);
        state.inventory_window.slots[4] = Some(ItemStack::simple(351, 3, 1));
        refresh(&mut state.inventory_window);
        let output = state.inventory_window.slots[0].as_ref().unwrap();
        assert_eq!((output.item_id, output.count, output.damage), (299, 1, 17));
        let tags = nbt::parse(output.nbt.as_ref().unwrap()).unwrap();
        assert_eq!(tags.get("custom"), Some(&Tag::Int(42)));
        assert_eq!(
            tags.get("display").unwrap().get("color"),
            Some(&Tag::Int(0x993333))
        );
        consume(&mut state.inventory_window);
        assert!(state.inventory_window.slots[1].is_none());
        assert_eq!(state.inventory_window.slots[4].as_ref().unwrap().count, 2);
        assert!(state.inventory_window.slots[0].is_none());
        assert!(armor_dye_result(&[
            Some(ItemStack::simple(307, 1, 0)),
            Some(ItemStack::simple(351, 1, 1))
        ])
        .is_none());
        assert!(armor_dye_result(&[Some(ItemStack::simple(299, 1, 0))]).is_none());
    }
    #[test]
    fn cloning_preserves_book_tags_and_original_but_only_map_name() {
        let tags = Tag::Compound(
            [
                ("generation".encode_utf16().collect(), Tag::Int(0)),
                (
                    "title".encode_utf16().collect(),
                    Tag::String("原本\0😀".encode_utf16().collect()),
                ),
                (
                    "display".encode_utf16().collect(),
                    Tag::Compound(
                        [(
                            "Name".encode_utf16().collect(),
                            Tag::String("地図".encode_utf16().collect()),
                        )]
                        .into(),
                    ),
                ),
            ]
            .into(),
        );
        let mut book = ItemStack::simple(387, 1, 0);
        book.nbt = Some(nbt::encode(&tags).unwrap());
        let mut window = InventoryState::new().inventory_window;
        window.slots = vec![None; 45];
        window.slots[1] = Some(book.clone());
        window.slots[2] = Some(ItemStack::simple(386, 2, 0));
        window.slots[4] = Some(ItemStack::simple(386, 1, 0));
        let output = result(&window).unwrap();
        assert_eq!(output.count, 2);
        let output_tags = nbt::parse(output.nbt.as_ref().unwrap()).unwrap();
        assert_eq!(output_tags.get("generation"), Some(&Tag::Int(1)));
        assert_eq!(output_tags.get("title"), tags.get("title"));
        consume(&mut window);
        assert_eq!(window.slots[1], Some(book));
        assert_eq!(window.slots[2].as_ref().unwrap().count, 1);
        assert!(window.slots[4].is_none());
        let mut map = ItemStack::simple(358, 1, 7);
        map.nbt = Some(nbt::encode(&tags).unwrap());
        window.slots[1] = Some(map);
        window.slots[2] = Some(ItemStack::simple(395, 8, 0));
        let output = result(&window).unwrap();
        assert_eq!((output.item_id, output.count, output.damage), (358, 2, 7));
        let output_tags = nbt::parse(output.nbt.as_ref().unwrap()).unwrap();
        assert!(output_tags.get("title").is_none());
        assert_eq!(output_tags.get("display"), tags.get("display"));
        consume(&mut window);
        assert!(window.slots[1].is_none());
        assert_eq!(window.slots[2].as_ref().unwrap().count, 7);
    }
    #[test]
    fn second_generation_book_and_mixed_clone_inputs_are_rejected() {
        let mut book = ItemStack::simple(387, 1, 0);
        book.nbt = Some(
            nbt::encode(&Tag::Compound(
                [("generation".encode_utf16().collect(), Tag::Int(2))].into(),
            ))
            .unwrap(),
        );
        assert!(clone_result(&[Some(book.clone()), Some(ItemStack::simple(386, 1, 0))]).is_none());
        assert!(clone_result(&[Some(book), Some(ItemStack::simple(395, 1, 0))]).is_none());
        assert!(clone_result(&[
            Some(ItemStack::simple(358, 1, 0)),
            Some(ItemStack::simple(358, 1, 0)),
            Some(ItemStack::simple(395, 1, 0))
        ])
        .is_none());
    }
    #[test]
    fn fireworks_star_fade_and_rocket_use_exact_nbt_and_consume_once() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(289, 2, 0));
        state.inventory_window.slots[2] = Some(ItemStack::simple(351, 1, 1));
        state.inventory_window.slots[3] = Some(ItemStack::simple(264, 1, 0));
        state.inventory_window.slots[4] = Some(ItemStack::simple(348, 1, 0));
        let star = result(&state.inventory_window).unwrap();
        let tag = nbt::parse(star.nbt.as_ref().unwrap()).unwrap();
        let explosion = tag.get("Explosion").unwrap();
        assert_eq!(explosion.get("Trail"), Some(&Tag::Byte(1)));
        assert_eq!(explosion.get("Flicker"), Some(&Tag::Byte(1)));
        assert_eq!(explosion.get("Colors"), Some(&Tag::Ints(vec![11743532])));
        consume(&mut state.inventory_window);
        assert_eq!(state.inventory_window.slots[1].as_ref().unwrap().count, 1);
        assert!(state.inventory_window.slots[2..5]
            .iter()
            .all(Option::is_none));
        state.inventory_window.slots[1] = Some(star);
        state.inventory_window.slots[2] = Some(ItemStack::simple(351, 1, 15));
        let faded = result(&state.inventory_window).unwrap();
        let faded_tag = nbt::parse(faded.nbt.as_ref().unwrap()).unwrap();
        assert_eq!(
            faded_tag.get("Explosion").unwrap().get("FadeColors"),
            Some(&Tag::Ints(vec![15790320]))
        );
        state.inventory_window.slots[1] = Some(faded);
        state.inventory_window.slots[2] = Some(ItemStack::simple(289, 1, 0));
        state.inventory_window.slots[3] = Some(ItemStack::simple(339, 1, 0));
        let rocket = result(&state.inventory_window).unwrap();
        assert_eq!((rocket.item_id, rocket.count), (401, 1));
        let tag = nbt::parse(rocket.nbt.as_ref().unwrap()).unwrap();
        let fireworks = tag.get("Fireworks").unwrap();
        assert_eq!(fireworks.get("Flight"), Some(&Tag::Byte(1)));
        assert_eq!(
            fireworks.get("Explosions").unwrap().list().unwrap(),
            &[faded_tag.get("Explosion").unwrap().clone()]
        );
        state.inventory_window.slots[1] = None;
        assert_eq!(
            result(&state.inventory_window),
            Some(ItemStack::simple(401, 1, 0))
        );
    }
    #[test]
    #[ignore = "requires locally executed MCP919 FireworksProbe fixtures"]
    fn local_java_fireworks_fixtures_match() {
        compare_java_nbt_recipe_fixtures("fireworks-java-oracle.log", 9816);
    }
    #[test]
    #[ignore = "requires locally executed MCP919 BannerProbe fixtures"]
    fn local_java_banner_fixtures_match() {
        compare_java_nbt_recipe_fixtures("banner-java-oracle.log", 12832);
    }
    #[test]
    fn banner_duplication_returns_patterned_original_and_consumes_blank() {
        let mut state = InventoryState::new();
        let mut original = ItemStack::simple(425, 1, 2);
        original.nbt = Some(
            nbt::encode(&Tag::Compound(
                [(
                    "BlockEntityTag".encode_utf16().collect(),
                    Tag::Compound(
                        [(
                            "Patterns".encode_utf16().collect(),
                            Tag::List {
                                kind: 10,
                                values: vec![Tag::Compound(
                                    [(
                                        "Pattern".encode_utf16().collect(),
                                        Tag::String("cre".encode_utf16().collect()),
                                    )]
                                    .into(),
                                )],
                            },
                        )]
                        .into(),
                    ),
                )]
                .into(),
            ))
            .unwrap(),
        );
        state.inventory_window.slots[1] = Some(original.clone());
        state.inventory_window.slots[4] = Some(ItemStack::simple(425, 2, 2));
        refresh(&mut state.inventory_window);
        assert_eq!(state.inventory_window.slots[0], Some(original.clone()));
        consume(&mut state.inventory_window);
        assert_eq!(state.inventory_window.slots[1], Some(original));
        assert_eq!(state.inventory_window.slots[4].as_ref().unwrap().count, 1);
    }
    fn compare_java_nbt_recipe_fixtures(file: &str, expected_count: usize) {
        fn stack(text: &str) -> Slot {
            if text == "~" {
                return None;
            }
            let v: Vec<_> = text.split(':').collect();
            let mut s = ItemStack::simple(
                v[0].parse().unwrap(),
                v[1].parse().unwrap(),
                v[2].parse().unwrap(),
            );
            if v[3] != "-" {
                s.nbt = Some(
                    (0..v[3].len())
                        .step_by(2)
                        .map(|i| u8::from_str_radix(&v[3][i..i + 2], 16).unwrap())
                        .collect(),
                );
            }
            Some(s)
        }
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tmp/{file}")),
        )
        .unwrap();
        let mut count = 0;
        for line in text.lines().filter(|l| l.starts_with("CLONE|")) {
            let parts: Vec<_> = line.split('|').collect();
            let mut window = InventoryState::new().inventory_window;
            window.window_id = 1;
            window.slots = vec![None; 46];
            window.metadata = Some(super::super::ContainerMetadata {
                inventory_type: "minecraft:crafting_table".into(),
                window_title_json: "{}".into(),
                slot_count: 10,
                entity_id: None,
            });
            for (i, cell) in parts[1].split(';').enumerate() {
                window.slots[i + 1] = stack(cell);
            }
            let actual = result(&window);
            let expected = stack(parts[2]);
            match (actual, expected) {
                (Some(a), Some(e)) => {
                    assert_eq!(
                        (a.item_id, a.count, a.damage),
                        (e.item_id, e.count, e.damage),
                        "{line}"
                    );
                    assert!(a.tags_equal(&e), "{line}");
                }
                (a, e) => assert_eq!(a, e, "{line}"),
            }
            count += 1;
        }
        assert_eq!(count, expected_count);
        println!("{count} MCP919 NBT recipe cases match: {file}");
    }
    #[test]
    #[ignore = "requires locally executed MCP919 ArmorDyeProbe fixtures"]
    fn local_java_armor_dye_fixtures_match() {
        compare_java_nbt_recipe_fixtures("armor-dye-java-oracle.log", 8550);
    }
    #[test]
    #[ignore = "requires locally executed MCP919 CloningProbe fixtures"]
    fn local_java_cloning_fixtures_match() {
        compare_java_nbt_recipe_fixtures("cloning-java-oracle.log", 480);
    }
    fn compare_java_static_fixtures(file: &str, expected_count: usize) {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tmp/{file}"));
        let text = std::fs::read_to_string(path).unwrap();
        fn stack(text: &str) -> Slot {
            if text == "~" {
                return None;
            }
            let values: Vec<i16> = text.split(':').map(|v| v.parse().unwrap()).collect();
            Some(ItemStack::simple(values[0], values[1] as u8, values[2]))
        }
        let mut count = 0;
        for line in text.lines().filter(|l| l.starts_with("CASE|")) {
            let parts: Vec<&str> = line.split('|').collect();
            let width: usize = parts[1].parse().unwrap();
            let mut window = InventoryState::new().inventory_window;
            if width == 3 {
                window.window_id = 1;
                window.slots = vec![None; 46];
                window.metadata = Some(super::super::ContainerMetadata {
                    inventory_type: "minecraft:crafting_table".into(),
                    window_title_json: "{}".into(),
                    slot_count: 10,
                    entity_id: None,
                });
            }
            for (i, cell) in parts[2].split(';').enumerate() {
                window.slots[i + 1] = stack(cell);
            }
            assert_eq!(result(&window), stack(parts[3]), "{line}");
            count += 1;
        }
        assert_eq!(count, expected_count);
        println!(
            "{count} local Java crafting cases match; {} static recipes registered",
            recipes().len()
        );
    }
    #[test]
    #[ignore = "requires locally executed MCP919 CraftingProbe fixtures"]
    fn local_java_crafting_fixtures_match() {
        compare_java_static_fixtures("crafting-java-oracle.log", 2036);
    }
    #[test]
    #[ignore = "requires locally executed MCP919 StaticRecipeProbe fixtures"]
    fn local_java_static_catalog_fixtures_match() {
        compare_java_static_fixtures("static-recipes-java-oracle.log", 14336);
    }
    #[test]
    fn repair_combines_durability_with_bonus_and_discards_nbt() {
        let mut state = InventoryState::new();
        let mut a = ItemStack::simple(278, 1, 1500);
        a.nbt = Some(vec![10, 0, 0, 0]);
        state.inventory_window.slots[1] = Some(a);
        state.inventory_window.slots[4] = Some(ItemStack::simple(278, 1, 1400));
        assert_eq!(
            result(&state.inventory_window),
            Some(ItemStack::simple(278, 1, 1261))
        );
        state.inventory_window.slots[1].as_mut().unwrap().damage = 10;
        assert_eq!(
            result(&state.inventory_window),
            Some(ItemStack::simple(278, 1, 0))
        );
        state.inventory_window.slots[4].as_mut().unwrap().count = 2;
        assert_eq!(result(&state.inventory_window), None);
        state.inventory_window.slots[4].as_mut().unwrap().count = 1;
        state.inventory_window.slots[2] = Some(ItemStack::simple(280, 1, 0));
        assert_eq!(result(&state.inventory_window), None);
    }
    #[test]
    fn shaped_offsets_metadata_and_extra_items() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[2] = Some(ItemStack::simple(5, 3, 2));
        state.inventory_window.slots[4] = Some(ItemStack::simple(5, 2, 5));
        assert_eq!(
            result(&state.inventory_window),
            Some(ItemStack::simple(280, 4, 0))
        );
        state.inventory_window.slots[1] = Some(ItemStack::simple(1, 1, 0));
        assert_eq!(result(&state.inventory_window), None);
        state.inventory_window.slots[1] = None;
        state.inventory_window.slots[2] = Some(ItemStack::simple(17, 1, 1));
        state.inventory_window.slots[4] = None;
        assert_eq!(
            result(&state.inventory_window),
            Some(ItemStack::simple(5, 4, 1))
        );
        state.inventory_window.slots[2].as_mut().unwrap().damage = 5;
        assert_eq!(result(&state.inventory_window), None);
    }
    #[test]
    fn shapeless_multiset_counts_duplicates_and_ignores_stack_quantity() {
        let mut state = InventoryState::new();
        for (slot, id) in [(1, 339), (2, 334), (3, 339), (4, 339)] {
            state.inventory_window.slots[slot] = Some(ItemStack::simple(id, 20, 0));
        }
        assert_eq!(
            result(&state.inventory_window),
            Some(ItemStack::simple(340, 1, 0))
        );
        state.inventory_window.slots[4] = None;
        assert_eq!(result(&state.inventory_window), None);
    }
    #[test]
    fn result_right_click_takes_whole_output_consumes_once_and_rolls_back() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(5, 3, 0));
        state.inventory_window.slots[3] = Some(ItemStack::simple(5, 2, 0));
        refresh(&mut state.inventory_window);
        let before = state.inventory_window.clone();
        let packet = state.queue_pickup_click(0, 0, 1);
        assert_eq!(state.carried_item, Some(ItemStack::simple(280, 4, 0)));
        assert_eq!(state.inventory_window.slots[1].as_ref().unwrap().count, 2);
        assert_eq!(state.inventory_window.slots[3].as_ref().unwrap().count, 1);
        assert_eq!(
            state.inventory_window.slots[0],
            Some(ItemStack::simple(280, 4, 0))
        );
        let PlayServerboundPacket::ClickWindow(click) = packet else {
            panic!()
        };
        state.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
            window_id: 0,
            action_number: click.action_number,
            accepted: false,
        });
        assert_eq!(state.inventory_window, before);
        assert_eq!(state.carried_item, None);
    }
    #[test]
    fn full_cursor_does_not_consume_and_input_click_refreshes_result() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(17, 2, 0));
        refresh(&mut state.inventory_window);
        state.carried_item = Some(ItemStack::simple(5, 63, 0));
        let before = state.inventory_window.clone();
        state.queue_pickup_click(0, 0, 0);
        assert_eq!(state.inventory_window, before);
        state.carried_item = None;
        state.queue_pickup_click(0, 1, 0);
        assert_eq!(state.inventory_window.slots[0], None);
        state.apply_set_slot(&SetSlotPacket {
            window_id: -1,
            slot_id: -1,
            item: Some(ItemStack::simple(17, 2, 0)),
        });
        state.queue_pickup_click(0, 2, 1);
        assert_eq!(
            state.inventory_window.slots[0],
            Some(ItemStack::simple(5, 4, 0))
        );
    }
    #[test]
    fn shift_crafts_until_materials_exhaust_and_restores_on_rejection() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(5, 3, 0));
        state.inventory_window.slots[3] = Some(ItemStack::simple(5, 3, 0));
        refresh(&mut state.inventory_window);
        let before = state.inventory_window.clone();
        let packet = state.queue_transfer_click(0, 0, 0).unwrap();
        assert_eq!(
            state.inventory_window.slots[44],
            Some(ItemStack::simple(280, 12, 0))
        );
        assert!(state.inventory_window.slots[0..5]
            .iter()
            .all(Option::is_none));
        let PlayServerboundPacket::ClickWindow(click) = packet else {
            panic!()
        };
        assert_eq!(click.clicked_item, Some(ItemStack::simple(280, 4, 0)));
        state.apply_confirm_transaction(&ConfirmTransactionClientboundPacket {
            window_id: 0,
            action_number: click.action_number,
            accepted: false,
        });
        assert_eq!(state.inventory_window, before);
    }
    #[test]
    fn shifting_material_out_of_matrix_invalidates_result() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(17, 3, 0));
        refresh(&mut state.inventory_window);
        assert_eq!(
            state.inventory_window.slots[0],
            Some(ItemStack::simple(5, 4, 0))
        );
        state.queue_transfer_click(0, 1, 0).unwrap();
        assert_eq!(state.inventory_window.slots[0], None);
        assert_eq!(state.inventory_window.slots[1], None);
        assert_eq!(
            state.inventory_window.slots[9],
            Some(ItemStack::simple(17, 3, 0))
        );
    }
    #[test]
    fn full_inventory_shift_does_not_consume() {
        let mut state = InventoryState::new();
        state.inventory_window.slots[1] = Some(ItemStack::simple(17, 3, 0));
        for slot in 9..45 {
            state.inventory_window.slots[slot] = Some(ItemStack::simple(1, 64, 0));
        }
        refresh(&mut state.inventory_window);
        let before = state.inventory_window.clone();
        let packet = state.queue_transfer_click(0, 0, 0).unwrap();
        assert_eq!(state.inventory_window, before);
        let PlayServerboundPacket::ClickWindow(click) = packet else {
            panic!()
        };
        assert_eq!(click.clicked_item, None);
    }
    #[test]
    fn cake_consumption_returns_three_buckets() {
        let mut window = ContainerSnapshot {
            window_id: 1,
            slots: vec![None; 46],
            properties: Default::default(),
            metadata: Some(super::super::ContainerMetadata {
                inventory_type: "minecraft:crafting_table".into(),
                window_title_json: "{}".into(),
                slot_count: 10,
                entity_id: None,
            }),
        };
        for (slot, id) in [
            (1, 335),
            (2, 335),
            (3, 335),
            (4, 353),
            (5, 344),
            (6, 353),
            (7, 296),
            (8, 296),
            (9, 296),
        ] {
            window.slots[slot] = Some(ItemStack::simple(id, 1, 0));
        }
        assert_eq!(result(&window), Some(ItemStack::simple(354, 1, 0)));
        consume(&mut window);
        for slot in 1..4 {
            assert_eq!(window.slots[slot], Some(ItemStack::simple(325, 1, 0)));
        }
        assert!(window.slots[4..10].iter().all(Option::is_none));
        assert_eq!(window.slots[0], None);
    }
}
