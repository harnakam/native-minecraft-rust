//! Banner recipe matching and pattern NBT, reconstructed from MCP919.
use super::{ItemStack, Slot};
use rmc_net::nbt::{self, Tag};
use std::collections::BTreeMap;
enum Pattern {
    Mask([u8; 9]),
    Item(i16, i16),
}
const PATTERNS: &[(&str, Pattern)] = &[
    ("bl", Pattern::Mask(*b"      #  ")),
    ("br", Pattern::Mask(*b"        #")),
    ("tl", Pattern::Mask(*b"#        ")),
    ("tr", Pattern::Mask(*b"  #      ")),
    ("bs", Pattern::Mask(*b"      ###")),
    ("ts", Pattern::Mask(*b"###      ")),
    ("ls", Pattern::Mask(*b"#  #  #  ")),
    ("rs", Pattern::Mask(*b"  #  #  #")),
    ("cs", Pattern::Mask(*b" #  #  # ")),
    ("ms", Pattern::Mask(*b"   ###   ")),
    ("drs", Pattern::Mask(*b"#   #   #")),
    ("dls", Pattern::Mask(*b"  # # #  ")),
    ("ss", Pattern::Mask(*b"# ## #   ")),
    ("cr", Pattern::Mask(*b"# # # # #")),
    ("sc", Pattern::Mask(*b" # ### # ")),
    ("bt", Pattern::Mask(*b"    # # #")),
    ("tt", Pattern::Mask(*b"# # #    ")),
    ("bts", Pattern::Mask(*b"   # # # ")),
    ("tts", Pattern::Mask(*b" # # #   ")),
    ("ld", Pattern::Mask(*b"## #     ")),
    ("rd", Pattern::Mask(*b"     # ##")),
    ("lud", Pattern::Mask(*b"   #  ## ")),
    ("rud", Pattern::Mask(*b" ##  #   ")),
    ("mc", Pattern::Mask(*b"    #    ")),
    ("mr", Pattern::Mask(*b" # # # # ")),
    ("vh", Pattern::Mask(*b"## ## ## ")),
    ("hh", Pattern::Mask(*b"######   ")),
    ("vhr", Pattern::Mask(*b" ## ## ##")),
    ("hhb", Pattern::Mask(*b"   ######")),
    ("bo", Pattern::Mask(*b"#### ####")),
    ("cbo", Pattern::Item(106, 0)),
    ("cre", Pattern::Item(397, 4)),
    ("gra", Pattern::Mask(*b"# # #  # ")),
    ("gru", Pattern::Mask(*b" #  # # #")),
    ("bri", Pattern::Item(45, 0)),
    ("sku", Pattern::Item(397, 1)),
    ("flo", Pattern::Item(38, 8)),
    ("moj", Pattern::Item(322, 1)),
];
fn key(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn tags(stack: &ItemStack) -> Option<Tag> {
    stack.nbt.as_deref().and_then(|b| nbt::parse(b).ok())
}
pub(super) fn pattern_count(stack: &ItemStack) -> usize {
    match tags(stack).and_then(|t| {
        t.get("BlockEntityTag")
            .and_then(|b| b.get("Patterns"))
            .cloned()
    }) {
        Some(Tag::List { kind: 10, values }) => values.len(),
        _ => 0,
    }
}
fn base(stack: &ItemStack) -> i32 {
    let value =
        tags(stack).and_then(|t| t.get("BlockEntityTag").and_then(|b| b.get("Base")).cloned());
    match value {
        Some(Tag::Byte(v)) => v as i32,
        Some(Tag::Short(v)) => v as i32,
        Some(Tag::Int(v)) => v,
        Some(Tag::Long(v)) => v as i32,
        Some(Tag::Float(v)) => super::java_floor(v as f64),
        Some(Tag::Double(v)) => super::java_floor(v),
        Some(_) => 0,
        None => stack.damage as i32,
    }
}
pub(super) fn duplicate(cells: &[Slot]) -> Slot {
    let stacks: Vec<_> = cells.iter().flatten().collect();
    if stacks.len() != 2
        || stacks.iter().any(|s| s.item_id != 425)
        || base(stacks[0]) != base(stacks[1])
    {
        return None;
    }
    let patterned: Vec<_> = stacks.iter().filter(|s| pattern_count(s) > 0).collect();
    if patterned.len() != 1 {
        return None;
    }
    let mut out = (*patterned[0]).clone();
    out.count = 1;
    Some(out)
}
pub(super) fn add_pattern(cells: &[Slot]) -> Slot {
    let banners: Vec<_> = cells
        .iter()
        .flatten()
        .filter(|s| s.item_id == 425)
        .collect();
    if banners.len() != 1 || pattern_count(banners[0]) >= 6 {
        return None;
    }
    let mut chosen = None;
    for &(id, ref pattern) in PATTERNS {
        let matched = match pattern {
            Pattern::Item(item, damage) => {
                let mut tokens = 0;
                let mut dyes = 0;
                let mut valid = true;
                for s in cells.iter().flatten().filter(|s| s.item_id != 425) {
                    if s.item_id == 351 {
                        dyes += 1;
                    } else if s.item_id == *item && s.damage == *damage {
                        tokens += 1;
                    } else {
                        valid = false;
                    }
                }
                valid && tokens == 1 && dyes <= 1
            }
            Pattern::Mask(mask) => {
                if cells.len() != 9 {
                    false
                } else {
                    let mut color = None;
                    cells.iter().zip(mask).all(|(cell, &mark)| match cell {
                        Some(s) if s.item_id != 425 => {
                            if s.item_id != 351
                                || mark == b' '
                                || color.is_some_and(|c| c != s.damage)
                            {
                                false
                            } else {
                                color = Some(s.damage);
                                true
                            }
                        }
                        _ => mark == b' ',
                    })
                }
            }
        };
        if matched {
            chosen = Some(id);
            break;
        }
    }
    let pattern = chosen?;
    let color = cells
        .iter()
        .flatten()
        .find(|s| s.item_id == 351)
        .map_or(0, |s| s.damage as i32);
    let mut out = banners[0].clone();
    out.count = 1;
    let mut root = match tags(&out) {
        Some(Tag::Compound(v)) => v,
        None => BTreeMap::new(),
        _ => return None,
    };
    let mut block = match root.remove(&key("BlockEntityTag")) {
        Some(Tag::Compound(v)) => v,
        _ => BTreeMap::new(),
    };
    let entry = Tag::Compound(
        [
            (key("Pattern"), Tag::String(key(pattern))),
            (key("Color"), Tag::Int(color)),
        ]
        .into(),
    );
    // A wrong nonempty list subtype produces a detached getTagList result in Java.
    match block.get_mut(&key("Patterns")) {
        Some(Tag::List { kind, values }) => {
            if *kind == 10 || *kind == 0 {
                *kind = 10;
                values.push(entry);
            }
        }
        _ => {
            block.insert(
                key("Patterns"),
                Tag::List {
                    kind: 10,
                    values: vec![entry],
                },
            );
        }
    }
    root.insert(key("BlockEntityTag"), Tag::Compound(block));
    out.nbt = Some(nbt::encode(&Tag::Compound(root)).ok()?);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn banner(patterns: Tag) -> ItemStack {
        let mut s = ItemStack::simple(425, 1, 4);
        s.nbt = Some(
            nbt::encode(&Tag::Compound(
                [(
                    key("BlockEntityTag"),
                    Tag::Compound([(key("Patterns"), patterns)].into()),
                )]
                .into(),
            ))
            .unwrap(),
        );
        s
    }
    #[test]
    fn material_pattern_can_omit_dye_and_keeps_zero_color() {
        let out = add_pattern(&[
            Some(ItemStack::simple(425, 1, 4)),
            Some(ItemStack::simple(106, 1, 0)),
            None,
            None,
        ])
        .unwrap();
        let tag = tags(&out).unwrap();
        let p = tag
            .get("BlockEntityTag")
            .unwrap()
            .get("Patterns")
            .unwrap()
            .list()
            .unwrap();
        assert_eq!(p[0].get("Pattern"), Some(&Tag::String(key("cbo"))));
        assert_eq!(p[0].get("Color"), Some(&Tag::Int(0)));
    }
    #[test]
    fn full_patterns_reject_addition_and_mistyped_lists_remain_unchanged() {
        let six = banner(Tag::List {
            kind: 10,
            values: vec![Tag::Compound(BTreeMap::new()); 6],
        });
        assert!(
            add_pattern(&[Some(six), Some(ItemStack::simple(106, 1, 0)), None, None]).is_none()
        );
        for values in [vec![], vec![Tag::String(key("bad"))]] {
            let original = banner(Tag::List { kind: 8, values });
            let out = add_pattern(&[
                Some(original.clone()),
                Some(ItemStack::simple(106, 1, 0)),
                None,
                None,
            ])
            .unwrap();
            assert!(out.tags_equal(&original));
        }
    }
}
