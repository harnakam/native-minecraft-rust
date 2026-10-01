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
