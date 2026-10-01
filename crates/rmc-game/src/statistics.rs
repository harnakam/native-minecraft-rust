//! StatFileWriter absolute values and NetHandlerPlayClient notification rules.
use rmc_net::codec::play::{statistic_is_achievement, PlayServerboundPacket, StatisticsPacket};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StatisticsUpdate {
    pub achievement_notifications: Vec<String>,
    pub inventory_hint: bool,
    pub inventory_hint_disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticsState {
    values: BTreeMap<String, i32>,
    loaded: bool,
    show_inventory_hint: bool,
}

impl Default for StatisticsState {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            loaded: false,
            show_inventory_hint: true,
        }
    }
}

impl StatisticsState {
    pub fn value(&self, id: &str) -> i32 {
        self.values.get(id).copied().unwrap_or(0)
    }
    pub fn values(&self) -> &BTreeMap<String, i32> {
        &self.values
    }
    pub fn loaded(&self) -> bool {
        self.loaded
    }
    pub fn show_inventory_hint(&self) -> bool {
        self.show_inventory_hint
    }
    pub fn set_inventory_hint(&mut self, enabled: bool) {
        self.show_inventory_hint = enabled;
    }
    pub fn request_packet(&self) -> PlayServerboundPacket {
        PlayServerboundPacket::ClientStatus(1)
    }

    pub fn achievement_unlocked(&self, id: &str) -> bool {
        crate::achievement_catalog::achievement(id).is_some() && self.value(id) > 0
    }

    pub fn can_unlock_achievement(&self, id: &str) -> bool {
        crate::achievement_catalog::achievement(id).is_some_and(|definition| {
            definition
                .parent
                .is_none_or(|parent| self.achievement_unlocked(parent))
        })
    }

    /// Number of still-locked ancestors before the first unlocked ancestor.
    pub fn achievement_unlock_distance(&self, id: &str) -> Option<usize> {
        let definition = crate::achievement_catalog::achievement(id)?;
        if self.achievement_unlocked(id) {
            return Some(0);
        }
        let mut distance = 0;
        let mut parent = definition.parent;
        while let Some(id) = parent {
            if self.achievement_unlocked(id) {
                break;
            }
            distance += 1;
            parent = crate::achievement_catalog::achievement(id)?.parent;
        }
        Some(distance)
    }

    /// StatFileWriter.increaseStat, with EntityPlayerSP's remote-world gate.
    /// Accepted increments use Java int overflow; server absolute updates can
    /// subsequently replace these local values.
    pub fn increase(&mut self, id: &str, amount: i32, remote_player: bool) -> bool {
        let Some(achievement) = statistic_is_achievement(id) else {
            return false;
        };
        if remote_player && rmc_net::codec::play::statistic_is_independent(id) != Some(true) {
            return false;
        }
        if achievement && !self.can_unlock_achievement(id) {
            return false;
        }
        self.values
            .insert(id.into(), self.value(id).wrapping_add(amount));
        true
    }

    pub fn receive(&mut self, packet: &StatisticsPacket) -> StatisticsUpdate {
        let mut update = StatisticsUpdate::default();
        let mut positive_achievement = false;
        for (id, value) in &packet.values {
            let Some(achievement) = statistic_is_achievement(id) else {
                continue;
            };
            if achievement && *value > 0 {
                positive_achievement = true;
                // Java checks precisely zero, not <= zero. Initial response is silent.
                if self.loaded && self.value(id) == 0 {
                    update.achievement_notifications.push(id.clone());
                    if id == "achievement.openInventory" {
                        self.show_inventory_hint = false;
                        update.inventory_hint_disabled = true;
                    }
                }
            }
            self.values.insert(id.clone(), *value);
        }
        update.inventory_hint = !self.loaded && !positive_achievement && self.show_inventory_hint;
        self.loaded = true;
        update
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn packet(values: &[(&str, i32)]) -> StatisticsPacket {
        StatisticsPacket {
            values: values
                .iter()
                .map(|(id, value)| ((*id).into(), *value))
                .collect(),
        }
    }
    #[test]
    fn parent_gates_remote_independence_and_java_overflow() {
        let mut state = StatisticsState::default();
        assert_eq!(
            state.achievement_unlock_distance("achievement.buildPickaxe"),
            Some(3)
        );
        assert!(!state.can_unlock_achievement("achievement.mineWood"));
        assert!(!state.increase("achievement.mineWood", 1, false));
        assert!(state.increase("achievement.openInventory", 1, true));
        assert!(state.can_unlock_achievement("achievement.mineWood"));
        assert!(!state.increase("achievement.mineWood", 1, true));
        assert!(state.increase("achievement.mineWood", 1, false));
        assert_eq!(
            state.achievement_unlock_distance("achievement.buildPickaxe"),
            Some(1)
        );
        assert_eq!(
            state.achievement_unlock_distance("achievement.mineWood"),
            Some(0)
        );
        assert_eq!(state.achievement_unlock_distance("unknown"), None);
        state.receive(&packet(&[("stat.jump", i32::MAX)]));
        assert!(state.increase("stat.jump", 1, true));
        assert_eq!(state.value("stat.jump"), i32::MIN);
        state.receive(&packet(&[("stat.jump", 2)]));
        assert_eq!(state.value("stat.jump"), 2);
        assert!(!state.increase("stat.craftItem.minecraft.stick", 1, true));
        assert!(!state.increase("unknown", 1, false));
    }

    #[test]
    #[ignore = "requires local MCP919 achievement rules Java oracle"]
    fn local_java_achievement_rules_match() {
        use crate::achievement_catalog::ACHIEVEMENTS;
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tmp/achievement-rules-java-oracle.log");
        let oracle = std::fs::read_to_string(path).unwrap();
        let mut rules = 0;
        let mut increments = 0;
        let mut definitions = 0;
        let mut independent = 0;
        for line in oracle.lines() {
            let fields: Vec<_> = line.split('|').collect();
            match fields[0] {
                "INDEPENDENT" => {
                    assert_eq!(
                        rmc_net::codec::play::statistic_is_independent(fields[1]),
                        Some(fields[2] == "true")
                    );
                    independent += 1;
                }
                "META" => {
                    let definition = &ACHIEVEMENTS[definitions];
                    assert_eq!(definition.id, fields[1]);
                    assert_eq!(definition.parent.unwrap_or("-"), fields[2]);
                    assert_eq!(definition.column, fields[3].parse::<i32>().unwrap());
                    assert_eq!(definition.row, fields[4].parse::<i32>().unwrap());
                    assert_eq!(definition.item_id, fields[5].parse::<i16>().unwrap());
                    assert_eq!(definition.damage, fields[6].parse::<i16>().unwrap());
                    assert_eq!(definition.special, fields[7] == "true");
                    definitions += 1;
                }
                "RULE" => {
                    let mask: usize = fields[1].parse().unwrap();
                    let mut state = StatisticsState::default();
                    for (index, definition) in ACHIEVEMENTS.iter().enumerate() {
                        state
                            .values
                            .insert(definition.id.into(), ((mask >> (index % 8)) & 1) as i32);
                    }
                    let id = fields[2];
                    assert_eq!(
                        state.achievement_unlocked(id),
                        fields[3] == "true",
                        "{line}"
                    );
                    assert_eq!(
                        state.can_unlock_achievement(id),
                        fields[4] == "true",
                        "{line}"
                    );
                    assert_eq!(
                        state.achievement_unlock_distance(id),
                        Some(fields[5].parse().unwrap()),
                        "{line}"
                    );
                    rules += 1;
                }
                "ADD" => {
                    let mut state = StatisticsState::default();
                    state
                        .values
                        .insert(fields[1].into(), fields[2].parse().unwrap());
                    state.increase(fields[1], fields[3].parse().unwrap(), fields[4] == "1");
                    assert_eq!(
                        state.value(fields[1]),
                        fields[5].parse::<i32>().unwrap(),
                        "{line}"
                    );
                    increments += 1;
                }
                _ => {}
            }
        }
        assert_eq!(definitions, 34);
        assert_eq!(independent, 891);
        assert_eq!(rules, 8704);
        assert_eq!(increments, 16038);
        println!("{rules} achievement dependency cases and {increments} statistic increment cases match MCP919");
    }
    #[test]
    fn initial_response_hint_absolute_updates_and_achievement_edges() {
        let mut state = StatisticsState::default();
        assert_eq!(state.value("stat.leaveGame"), 0);
        assert_eq!(
            state.request_packet(),
            PlayServerboundPacket::ClientStatus(1)
        );
        assert!(
            state
                .receive(&packet(&[("stat.leaveGame", 9)]))
                .inventory_hint
        );
        assert!(state.loaded());
        let update = state.receive(&packet(&[("achievement.openInventory", 1)]));
        assert_eq!(
            update.achievement_notifications,
            ["achievement.openInventory"]
        );
        assert!(update.inventory_hint_disabled);
        assert!(!state.show_inventory_hint());
        assert!(state
            .receive(&packet(&[("achievement.openInventory", 2)]))
            .achievement_notifications
            .is_empty());
        state.receive(&packet(&[("stat.leaveGame", -4), ("unknown.stat", 1)]));
        assert_eq!(state.value("stat.leaveGame"), -4);
        assert_eq!(state.value("unknown.stat"), 0);
        assert_eq!(state.value("achievement.openInventory"), 2);
        state.receive(&packet(&[("achievement.openInventory", 0)]));
        assert_eq!(
            state
                .receive(&packet(&[("achievement.openInventory", 1)]))
                .achievement_notifications
                .len(),
            1
        );
    }
    #[test]
    fn initial_unlocked_achievements_are_silent_and_negative_previous_values_are_not_zero() {
        let mut state = StatisticsState::default();
        let first = state.receive(&packet(&[
            ("achievement.openInventory", 1),
            ("achievement.mineWood", -1),
        ]));
        assert!(!first.inventory_hint);
        assert!(first.achievement_notifications.is_empty());
        assert!(state.show_inventory_hint());
        assert!(state
            .receive(&packet(&[("achievement.mineWood", 1)]))
            .achievement_notifications
            .is_empty());
        let mut disabled = StatisticsState::default();
        disabled.set_inventory_hint(false);
        assert!(
            !disabled
                .receive(&StatisticsPacket::default())
                .inventory_hint
        );
    }
}
