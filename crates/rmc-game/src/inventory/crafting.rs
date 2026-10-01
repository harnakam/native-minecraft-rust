//! Local crafting matching and result-slot effects, reconstructed from MCP919.
use super::{item_has_subtypes, item_stack_limit, ContainerSnapshot};
use rmc_net::codec::play::{ItemStack, Slot};
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
    width: usize,
    height: usize,
    inputs: Vec<Option<Ingredient>>,
    output: ItemStack,
}
fn add(
    recipes: &mut Vec<Recipe>,
    rows: &[&str],
    keys: &[(char, i16, Option<i16>)],
    id: i16,
    count: u8,
    damage: i16,
) {
    recipes.push(Recipe {
        width: rows[0].len(),
        height: rows.len(),
        inputs: rows
            .iter()
            .flat_map(|row| {
                row.chars().map(|ch| {
                    keys.iter().find(|k| k.0 == ch).map(|k| Ingredient {
                        id: k.1,
                        damage: k.2,
                    })
                })
            })
            .collect(),
        output: ItemStack::simple(id, count, damage),
    });
}
fn shapeless(
    recipes: &mut Vec<Recipe>,
    inputs: &[(i16, Option<i16>)],
    id: i16,
    count: u8,
    damage: i16,
) {
    recipes.push(Recipe {
        width: 0,
        height: 0,
        inputs: inputs
            .iter()
            .map(|&(id, damage)| Some(Ingredient { id, damage }))
            .collect(),
        output: ItemStack::simple(id, count, damage),
    });
}
fn recipes() -> &'static [Recipe] {
    static RECIPES: std::sync::OnceLock<Vec<Recipe>> = std::sync::OnceLock::new();
    RECIPES.get_or_init(|| {
        let mut r = Vec::new();
        for damage in 0..4 {
            add(&mut r, &["#"], &[('#', 17, Some(damage))], 5, 4, damage);
        }
        for damage in 0..2 {
            add(
                &mut r,
                &["#"],
                &[('#', 162, Some(damage))],
                5,
                4,
                damage + 4,
            );
        }
        add(&mut r, &["#", "#"], &[('#', 5, None)], 280, 4, 0);
        add(&mut r, &["##", "##"], &[('#', 5, None)], 58, 1, 0);
        add(&mut r, &["###", "# #", "###"], &[('#', 5, None)], 54, 1, 0);
        add(&mut r, &["###", "# #", "###"], &[('#', 4, None)], 61, 1, 0);
        add(&mut r, &["###", "###"], &[('#', 5, None)], 96, 2, 0);
        add(&mut r, &["##"], &[('#', 5, None)], 72, 1, 0);
        add(&mut r, &["#"], &[('#', 5, None)], 143, 1, 0);
        add(&mut r, &["##"], &[('#', 1, Some(0))], 70, 1, 0);
        add(&mut r, &["#"], &[('#', 1, Some(0))], 77, 1, 0);
        add(&mut r, &["###"], &[('#', 296, Some(0))], 297, 1, 0);
        add(
            &mut r,
            &["#", "X"],
            &[('#', 263, Some(0)), ('X', 280, Some(0))],
            50,
            4,
            0,
        );
        add(
            &mut r,
            &["#", "X"],
            &[('#', 263, Some(1)), ('X', 280, Some(0))],
            50,
            4,
            0,
        );
        add(
            &mut r,
            &["#", "X"],
            &[('#', 331, Some(0)), ('X', 280, Some(0))],
            76,
            1,
            0,
        );
        add(
            &mut r,
            &["###", "XYX", "ZZZ"],
            &[
                ('#', 335, Some(0)),
                ('X', 353, Some(0)),
                ('Y', 344, Some(0)),
                ('Z', 296, Some(0)),
            ],
            354,
            1,
            0,
        );
        for (material, pick, axe, shovel, hoe, sword) in [
            (5, 270, 271, 269, 290, 268),
            (4, 274, 275, 273, 291, 272),
            (265, 257, 258, 256, 292, 267),
            (264, 278, 279, 277, 293, 276),
            (266, 285, 286, 284, 294, 283),
        ] {
            let keys = [
                ('#', material, if material >= 256 { Some(0) } else { None }),
                ('X', 280, Some(0)),
            ];
            add(&mut r, &["###", " X ", " X "], &keys, pick, 1, 0);
            add(&mut r, &["##", "#X", " X"], &keys, axe, 1, 0);
            add(&mut r, &["#", "X", "X"], &keys, shovel, 1, 0);
            add(&mut r, &["##", " X", " X"], &keys, hoe, 1, 0);
            add(&mut r, &["#", "#", "X"], &keys, sword, 1, 0);
        }
        for (material, base) in [(334, 298), (265, 306), (264, 310), (266, 314)] {
            let keys = [('#', material, if material >= 256 { Some(0) } else { None })];
            add(&mut r, &["###", "# #"], &keys, base, 1, 0);
            add(&mut r, &["# #", "###", "###"], &keys, base + 1, 1, 0);
            add(&mut r, &["###", "# #", "# #"], &keys, base + 2, 1, 0);
            add(&mut r, &["# #", "# #"], &keys, base + 3, 1, 0);
        }
        shapeless(&mut r, &[(338, Some(0))], 353, 1, 0);
        shapeless(&mut r, &[(39, None), (40, None), (281, Some(0))], 282, 1, 0);
        shapeless(
            &mut r,
            &[
                (339, Some(0)),
                (339, Some(0)),
                (339, Some(0)),
                (334, Some(0)),
            ],
            340,
            1,
            0,
        );
        shapeless(
            &mut r,
            &[(375, Some(0)), (353, Some(0)), (39, None)],
            376,
            1,
            0,
        );
        shapeless(&mut r, &[(369, Some(0))], 377, 2, 0);
        shapeless(&mut r, &[(360, Some(0))], 362, 1, 0);
        shapeless(&mut r, &[(86, None)], 361, 4, 0);
        shapeless(&mut r, &[(352, Some(0))], 351, 3, 15);
        for damage in 0..16 {
            shapeless(
                &mut r,
                &[(35, Some(0)), (351, Some(damage))],
                35,
                1,
                15 - damage,
            );
        }
        for (material, damage, block) in [
            (265, 0, 42),
            (266, 0, 41),
            (264, 0, 57),
            (388, 0, 133),
            (351, 4, 22),
            (331, 0, 152),
            (263, 0, 173),
        ] {
            add(
                &mut r,
                &["###", "###", "###"],
                &[('#', material, Some(damage))],
                block,
                1,
                0,
            );
            add(&mut r, &["#"], &[('#', block, None)], material, 9, damage);
        }
        // Shaped recipes precede shapeless recipes, with larger recipe sizes first.
        r.sort_by_key(|recipe| (recipe.width == 0, std::cmp::Reverse(recipe.inputs.len())));
        r
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
pub(super) fn result(window: &ContainerSnapshot) -> Slot {
    let width = grid_width(window)?;
    if window.slots.len() < 1 + width * width {
        return None;
    }
    for recipe in recipes() {
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
    None
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
    for slot in 1..=width * width {
        let Some(mut stack) = window.slots[slot].take() else {
            continue;
        };
        let remainder =
            matches!(stack.item_id, 326 | 327 | 335).then(|| ItemStack::simple(325, 1, 0));
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
                        s.item_id == 325
                            && s.tags_equal(&remainder)
                            && s.damage == 0
                            && s.count < item_stack_limit(325)
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
    #[ignore = "requires locally executed MCP919 CraftingProbe fixtures"]
    fn local_java_crafting_fixtures_match() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/crafting-java-oracle.log");
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
        assert!(count >= 200);
        println!(
            "{count} local Java crafting cases match; {} authored recipes registered",
            recipes().len()
        );
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
