//! HUD, chat, menus, and container UI.

use rmc_game::usability::UsabilitySnapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CrosshairHud {
    pub visible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HotbarSlotHud {
    pub index: u8,
    pub label: String,
    pub selected: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HotbarHud {
    pub selected_slot: u8,
    pub slots: Vec<HotbarSlotHud>,
}

impl HotbarHud {
    pub fn vanilla(selected_slot: u8) -> Self {
        let slots = (0..9)
            .map(|index| HotbarSlotHud {
                index,
                label: format!("slot{}", index + 1),
                selected: index == selected_slot,
            })
            .collect();

        Self {
            selected_slot,
            slots,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimalHud {
    pub crosshair: CrosshairHud,
    pub hotbar: HotbarHud,
}

impl MinimalHud {
    pub fn vanilla(selected_slot: u8) -> Self {
        Self {
            crosshair: CrosshairHud { visible: true },
            hotbar: HotbarHud::vanilla(selected_slot),
        }
    }

    pub fn render_lines(&self) -> Vec<String> {
        let selected = self
            .hotbar
            .slots
            .iter()
            .map(|slot| {
                if slot.selected {
                    format!("[{}]", slot.index + 1)
                } else {
                    format!(" {} ", slot.index + 1)
                }
            })
            .collect::<Vec<_>>()
            .join(" ");

        vec![
            format!("crosshair={}", self.crosshair.visible),
            format!("hotbar={selected}"),
        ]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PvPHud {
    pub chat_lines: Vec<String>,
    pub scoreboard_lines: Vec<String>,
    pub tab_lines: Vec<String>,
    pub window_lines: Vec<String>,
    pub audio_lines: Vec<String>,
    pub settings_lines: Vec<String>,
}

impl PvPHud {
    pub fn from_snapshot(snapshot: &UsabilitySnapshot) -> Self {
        let chat_lines = snapshot
            .chat_lines
            .iter()
            .rev()
            .take(5)
            .map(|line| format!("chat[{}]={}", line.position, line.message_json))
            .collect::<Vec<_>>();

        let scoreboard_lines = snapshot
            .sidebar
            .as_ref()
            .map(|sidebar| {
                let mut lines = vec![format!(
                    "scoreboard={} render_type={}",
                    sidebar.title, sidebar.render_type
                )];
                lines.extend(
                    sidebar
                        .lines
                        .iter()
                        .map(|line| format!("score={} {}", line.value, line.rendered_name)),
                );
                lines
            })
            .unwrap_or_else(|| vec!["scoreboard=hidden".to_owned()]);

        let tab_lines = snapshot
            .tab_list
            .iter()
            .take(10)
            .map(|entry| {
                let display_name = entry
                    .display_name_json
                    .as_ref()
                    .map(|display| format!(" display={display}"))
                    .unwrap_or_default();
                format!(
                    "tab={} ping={} gamemode={} properties={}{}",
                    entry.name, entry.latency, entry.game_mode, entry.property_count, display_name
                )
            })
            .collect();

        let window_lines =
            snapshot
                .window
                .as_ref()
                .map(|window| {
                    let mut lines = vec![format!(
                        "window id={} type={} title={} carried={}",
                        window.window_id,
                        window.inventory_type,
                        window.title_json,
                        slot_label(&window.carried_item)
                    )];
                    lines.extend(window.slots.iter().enumerate().filter_map(
                        |(slot_index, slot)| {
                            slot.as_ref()
                                .map(|_| format!("slot[{slot_index}]={}", slot_label(slot)))
                        },
                    ));
                    lines
                })
                .unwrap_or_else(|| vec!["window=closed".to_owned()]);

        let audio_lines = snapshot
            .recent_sounds
            .iter()
            .rev()
            .take(5)
            .map(|sound| {
                format!(
                    "sound={} x={:.3} y={:.3} z={:.3} volume={:.3} pitch={:.3}",
                    sound.sound_name, sound.x, sound.y, sound.z, sound.volume, sound.pitch
                )
            })
            .collect();

        let settings_lines = vec![
            format!("locale={}", snapshot.settings.locale),
            format!("view_distance={}", snapshot.settings.view_distance),
            format!("chat_visibility={}", snapshot.settings.chat_visibility),
            format!("chat_colors={}", snapshot.settings.chat_colors),
            format!("skin_parts={}", snapshot.settings.displayed_skin_parts),
            format!("audio_enabled={}", snapshot.settings.audio_enabled),
            format!("audio_volume={:.3}", snapshot.settings.audio_volume),
        ];

        Self {
            chat_lines,
            scoreboard_lines,
            tab_lines,
            window_lines,
            audio_lines,
            settings_lines,
        }
    }

    pub fn render_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        lines.extend(self.settings_lines.iter().cloned());
        lines.extend(self.chat_lines.iter().cloned());
        lines.extend(self.scoreboard_lines.iter().cloned());
        lines.extend(self.tab_lines.iter().cloned());
        lines.extend(self.window_lines.iter().cloned());
        lines.extend(self.audio_lines.iter().cloned());
        lines
    }
}

fn slot_label(slot: &Option<rmc_net::codec::play::ItemStack>) -> String {
    slot.as_ref()
        .map(|item| format!("{}:{}:{}", item.item_id, item.count, item.damage))
        .unwrap_or_else(|| "none".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{MinimalHud, PvPHud};
    use rmc_game::usability::{
        AudioCueSnapshot, ChatLine, ClientSettingsState, SidebarLine, SidebarSnapshot,
        TabListEntrySnapshot, UsabilitySnapshot, WindowSnapshot,
    };

    #[test]
    fn marks_selected_hotbar_slot() {
        let hud = MinimalHud::vanilla(2);
        assert!(hud.hotbar.slots[2].selected);
        assert!(!hud.hotbar.slots[1].selected);
    }

    #[test]
    fn renders_pvp_overlay_lines() {
        let hud = PvPHud::from_snapshot(&UsabilitySnapshot {
            action_bar: None,
            title: Default::default(),
            experience: Default::default(),
            chat_lines: vec![ChatLine {
                age_ticks: 0,
                message_json: "{\"text\":\"Queue popped\"}".to_owned(),
                position: 1,
            }],
            sidebar: Some(SidebarSnapshot {
                objective_name: "bw".to_owned(),
                title: "BED WARS".to_owned(),
                render_type: "integer".to_owned(),
                lines: vec![SidebarLine {
                    entry_name: "Rush".to_owned(),
                    rendered_name: "[R] Rush".to_owned(),
                    value: 12,
                }],
            }),
            tab_list: vec![TabListEntrySnapshot {
                uuid: [1; 16],
                name: "Rush".to_owned(),
                display_name_json: Some("{\"text\":\"[MVP+] Rush\"}".to_owned()),
                latency: 32,
                game_mode: 1,
                property_count: 1,
            }],
            window: Some(WindowSnapshot {
                properties: Default::default(),
                window_id: 4,
                inventory_type: "minecraft:chest".to_owned(),
                title_json: "{\"text\":\"Loot\"}".to_owned(),
                slot_count: 2,
                slots: vec![
                    Some(rmc_net::codec::play::ItemStack::simple(5, 16, 0)),
                    None,
                ],
                carried_item: None,
            }),
            recent_sounds: vec![AudioCueSnapshot {
                sound_name: "note.pling".to_owned(),
                x: 1.0,
                y: 2.0,
                z: 3.0,
                volume: 0.5,
                pitch: 1.0,
            }],
            settings: ClientSettingsState::default(),
        });

        let lines = hud.render_lines();
        assert!(lines.iter().any(|line| line.contains("BED WARS")));
        assert!(lines.iter().any(|line| line.contains("tab=Rush")));
        assert!(lines.iter().any(|line| line.contains("window id=4")));
        assert!(lines.iter().any(|line| line.contains("sound=note.pling")));
    }
}
