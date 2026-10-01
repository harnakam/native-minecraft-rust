//! Chat, scoreboard, tab-list, settings, and audio state for M5 usability.

use crate::inventory::{InventoryState, InventoryUpdate};
use rmc_net::codec::play::{
    ChatMessagePacket, ChatMessageServerboundPacket, ClientSettingsPacket, DisplayScoreboardPacket,
    PlayClientboundPacket, PlayServerboundPacket, PlayerListItemAction, PlayerListItemPacket,
    ScoreboardObjectiveMode, ScoreboardObjectivePacket, SoundEffectPacket, TeamAction, TeamsPacket,
    UpdateScoreAction, UpdateScorePacket,
};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_CHAT_LINES: usize = 100;
pub const MAX_AUDIO_CUES: usize = 16;
pub const SIDEBAR_DISPLAY_SLOT: u8 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatLine {
    pub age_ticks: u64,
    pub message_json: String,
    pub position: i8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionBarMessage {
    pub message_json: String,
    pub remaining_ticks: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AudioCueSnapshot {
    pub sound_name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabListEntrySnapshot {
    pub uuid: [u8; 16],
    pub name: String,
    pub display_name_json: Option<String>,
    pub team_formatted_name: String,
    pub tab_score: Option<(i32, String)>,
    pub latency: i32,
    pub game_mode: i32,
    pub property_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SidebarLine {
    pub entry_name: String,
    pub rendered_name: String,
    pub value: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SidebarSnapshot {
    pub objective_name: String,
    pub title: String,
    pub render_type: String,
    pub lines: Vec<SidebarLine>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowSnapshot {
    pub properties: BTreeMap<i16, i16>,
    pub window_id: u8,
    pub inventory_type: String,
    pub title_json: String,
    pub slot_count: usize,
    pub slots: Vec<rmc_net::codec::play::Slot>,
    pub carried_item: rmc_net::codec::play::Slot,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClientSettingsState {
    pub locale: String,
    pub view_distance: i8,
    pub chat_visibility: u8,
    pub chat_colors: bool,
    pub displayed_skin_parts: u8,
    pub audio_enabled: bool,
    pub audio_volume: f32,
}

impl Default for ClientSettingsState {
    fn default() -> Self {
        let packet = ClientSettingsPacket::vanilla_headless_defaults();
        Self {
            locale: packet.locale,
            view_distance: packet.view_distance,
            chat_visibility: packet.chat_visibility,
            chat_colors: packet.chat_colors,
            displayed_skin_parts: packet.displayed_skin_parts,
            audio_enabled: true,
            audio_volume: 1.0,
        }
    }
}

impl ClientSettingsState {
    pub fn to_packet(&self) -> ClientSettingsPacket {
        ClientSettingsPacket {
            locale: self.locale.clone(),
            view_distance: self.view_distance,
            chat_visibility: self.chat_visibility,
            chat_colors: self.chat_colors,
            displayed_skin_parts: self.displayed_skin_parts,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Experience {
    pub progress: f32,
    pub level: i32,
    pub total: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UsabilitySnapshot {
    pub experience: Experience,
    pub chat_lines: Vec<ChatLine>,
    pub action_bar: Option<ActionBarMessage>,
    pub title: crate::title::TitleState,
    pub sidebar: Option<SidebarSnapshot>,
    pub tab_list: Vec<TabListEntrySnapshot>,
    pub tab_header_json: String,
    pub tab_footer_json: String,
    pub window: Option<WindowSnapshot>,
    pub recent_sounds: Vec<AudioCueSnapshot>,
    pub settings: ClientSettingsState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UsabilityUpdate {
    pub statistics: Option<crate::statistics::StatisticsUpdate>,
    pub outbound_packets: Vec<PlayServerboundPacket>,
    pub inventory: InventoryUpdate,
    pub chat_updated: bool,
    pub scoreboard_updated: bool,
    pub tab_list_updated: bool,
    pub audio_updated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TabListEntryState {
    uuid: [u8; 16],
    name: String,
    display_name_json: Option<String>,
    latency: i32,
    game_mode: i32,
    property_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObjectiveState {
    display_name: String,
    render_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
struct TeamState {
    display_name: String,
    prefix: String,
    suffix: String,
    friendly_flags: u8,
    name_tag_visibility: String,
    color: i8,
    players: BTreeSet<String>,
}

pub struct UsabilityState {
    statistics: crate::statistics::StatisticsState,
    experience: Experience,
    inventory: InventoryState,
    settings: ClientSettingsState,
    chat_lines: Vec<ChatLine>,
    action_bar: Option<ActionBarMessage>,
    title: crate::title::TitleState,
    tab_list: BTreeMap<[u8; 16], TabListEntryState>,
    tab_header_json: String,
    tab_footer_json: String,
    objectives: BTreeMap<String, ObjectiveState>,
    display_slots: BTreeMap<u8, String>,
    scores: BTreeMap<(String, String), i32>,
    teams: BTreeMap<String, TeamState>,
    player_teams: BTreeMap<String, String>,
    recent_sounds: Vec<AudioCueSnapshot>,
}

impl Default for UsabilityState {
    fn default() -> Self {
        Self::new()
    }
}

impl UsabilityState {
    pub fn advance_chat_ticks(&mut self, ticks: usize) {
        self.title.advance(ticks);
        if let Some(message) = &mut self.action_bar {
            message.remaining_ticks = message.remaining_ticks.saturating_sub(ticks.min(255) as u8);
            if message.remaining_ticks == 0 {
                self.action_bar = None;
            }
        }
        for line in &mut self.chat_lines {
            line.age_ticks = line.age_ticks.saturating_add(ticks as u64);
        }
    }

    pub fn reset_experience(&mut self) {
        self.experience = Experience::default();
    }

    pub fn player_name(&self, uuid: &[u8; 16]) -> Option<&str> {
        self.tab_list.get(uuid).map(|p| p.name.as_str())
    }

    pub fn friendly_invisibles_visible(&self, viewer: &str, target: &str) -> bool {
        let Some(team) = self.player_teams.get(target) else {
            return false;
        };
        self.player_teams.get(viewer) == Some(team)
            && self
                .teams
                .get(team)
                .is_some_and(|t| t.friendly_flags & 2 != 0)
    }
    pub fn new() -> Self {
        Self {
            statistics: Default::default(),
            experience: Experience::default(),
            inventory: InventoryState::new(),
            settings: ClientSettingsState::default(),
            chat_lines: Vec::new(),
            action_bar: None,
            title: Default::default(),
            tab_list: BTreeMap::new(),
            tab_header_json: String::new(),
            tab_footer_json: String::new(),
            objectives: BTreeMap::new(),
            display_slots: BTreeMap::new(),
            scores: BTreeMap::new(),
            teams: BTreeMap::new(),
            player_teams: BTreeMap::new(),
            recent_sounds: Vec::new(),
        }
    }

    pub fn statistics(&self) -> &crate::statistics::StatisticsState {
        &self.statistics
    }

    pub fn statistics_mut(&mut self) -> &mut crate::statistics::StatisticsState {
        &mut self.statistics
    }

    pub fn inventory(&self) -> &InventoryState {
        &self.inventory
    }

    pub fn inventory_mut(&mut self) -> &mut InventoryState {
        &mut self.inventory
    }

    pub fn settings(&self) -> &ClientSettingsState {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut ClientSettingsState {
        &mut self.settings
    }

    pub fn send_chat_message(&self, message: &str) -> PlayServerboundPacket {
        PlayServerboundPacket::ChatMessage(ChatMessageServerboundPacket::vanilla(message))
    }

    pub fn sync_settings_packet(&self) -> PlayServerboundPacket {
        PlayServerboundPacket::ClientSettings(self.settings.to_packet())
    }

    pub fn close_open_window(&mut self) -> Option<PlayServerboundPacket> {
        self.inventory.close_open_window()
    }

    pub fn apply_play_packet(&mut self, packet: &PlayClientboundPacket) -> UsabilityUpdate {
        let inventory = self.inventory.apply_play_packet(packet);
        let mut update = UsabilityUpdate {
            outbound_packets: inventory.outbound_packets.clone(),
            inventory,
            ..UsabilityUpdate::default()
        };

        match packet {
            PlayClientboundPacket::Statistics(packet) => {
                update.statistics = Some(self.statistics.receive(packet));
            }
            PlayClientboundPacket::PlayerListHeaderFooter(packet) => {
                self.tab_header_json.clone_from(&packet.header_json);
                self.tab_footer_json.clone_from(&packet.footer_json);
                update.tab_list_updated = true;
            }
            PlayClientboundPacket::Title(packet) => self.title.receive(packet),
            PlayClientboundPacket::SetExperience(packet) => {
                self.experience = Experience {
                    progress: packet.progress,
                    level: packet.level,
                    total: packet.total,
                };
            }
            PlayClientboundPacket::ChatMessage(packet) => {
                self.push_chat(packet);
                update.chat_updated = true;
            }
            PlayClientboundPacket::SoundEffect(packet) => {
                update.audio_updated = self.push_sound(packet);
            }
            PlayClientboundPacket::PlayerListItem(packet) => {
                self.apply_player_list(packet);
                update.tab_list_updated = true;
            }
            PlayClientboundPacket::ScoreboardObjective(packet) => {
                self.apply_scoreboard_objective(packet);
                update.scoreboard_updated = true;
            }
            PlayClientboundPacket::UpdateScore(packet) => {
                self.apply_score(packet);
                update.scoreboard_updated = true;
            }
            PlayClientboundPacket::DisplayScoreboard(packet) => {
                self.apply_display_scoreboard(packet);
                update.scoreboard_updated = true;
            }
            PlayClientboundPacket::Teams(packet) => {
                self.apply_team(packet);
                update.scoreboard_updated = true;
                update.tab_list_updated = true;
            }
            _ => {}
        }

        update
    }

    pub fn snapshot(&self) -> UsabilitySnapshot {
        let mut tab_list = self
            .tab_list
            .values()
            .map(|entry| TabListEntrySnapshot {
                uuid: entry.uuid,
                name: entry.name.clone(),
                display_name_json: entry.display_name_json.clone(),
                team_formatted_name: self.rendered_score_name(&entry.name),
                tab_score: self.display_slots.get(&0).and_then(|objective| {
                    self.objectives.get(objective).map(|state| {
                        (
                            self.scores
                                .get(&(objective.clone(), entry.name.clone()))
                                .copied()
                                .unwrap_or(0),
                            state.render_type.clone(),
                        )
                    })
                }),
                latency: entry.latency,
                game_mode: entry.game_mode,
                property_count: entry.property_count,
            })
            .collect::<Vec<_>>();
        tab_list.sort_by(|left, right| {
            (left.game_mode == 3)
                .cmp(&(right.game_mode == 3))
                .then_with(|| {
                    self.player_teams
                        .get(&left.name)
                        .map_or("", String::as_str)
                        .encode_utf16()
                        .cmp(
                            self.player_teams
                                .get(&right.name)
                                .map_or("", String::as_str)
                                .encode_utf16(),
                        )
                })
                .then_with(|| left.name.encode_utf16().cmp(right.name.encode_utf16()))
        });

        let window = self.inventory.open_window().map(|window| WindowSnapshot {
            properties: window.properties.clone(),
            window_id: window.window_id,
            inventory_type: window
                .metadata
                .as_ref()
                .map(|metadata| metadata.inventory_type.clone())
                .unwrap_or_else(|| "minecraft:inventory".into()),
            title_json: window
                .metadata
                .as_ref()
                .map(|metadata| metadata.window_title_json.clone())
                .unwrap_or_else(|| "{\"text\":\"Inventory\"}".into()),
            slot_count: window.player_inventory_offset(),
            slots: window.slots.clone(),
            carried_item: self.inventory.carried_item().clone(),
        });

        UsabilitySnapshot {
            experience: self.experience,
            chat_lines: self.chat_lines.clone(),
            action_bar: self.action_bar.clone(),
            title: self.title.clone(),
            sidebar: self.sidebar_snapshot(),
            tab_list,
            tab_header_json: self.tab_header_json.clone(),
            tab_footer_json: self.tab_footer_json.clone(),
            window,
            recent_sounds: self.recent_sounds.clone(),
            settings: self.settings.clone(),
        }
    }

    fn push_chat(&mut self, packet: &ChatMessagePacket) {
        if packet.position == 2 {
            self.action_bar = Some(ActionBarMessage {
                message_json: packet.message_json.clone(),
                remaining_ticks: 60,
            });
            return;
        }
        self.chat_lines.push(ChatLine {
            age_ticks: 0,
            message_json: packet.message_json.clone(),
            position: packet.position,
        });

        if self.chat_lines.len() > MAX_CHAT_LINES {
            self.chat_lines
                .drain(0..self.chat_lines.len() - MAX_CHAT_LINES);
        }
    }

    fn push_sound(&mut self, packet: &SoundEffectPacket) -> bool {
        if !self.settings.audio_enabled || self.settings.audio_volume <= 0.0 {
            return false;
        }

        self.recent_sounds.push(AudioCueSnapshot {
            sound_name: packet.sound_name.clone(),
            x: packet.x(),
            y: packet.y(),
            z: packet.z(),
            volume: packet.volume * self.settings.audio_volume,
            pitch: packet.pitch_value(),
        });

        if self.recent_sounds.len() > MAX_AUDIO_CUES {
            self.recent_sounds
                .drain(0..self.recent_sounds.len() - MAX_AUDIO_CUES);
        }

        true
    }

    fn apply_player_list(&mut self, packet: &PlayerListItemPacket) {
        match packet.action {
            PlayerListItemAction::AddPlayer => {
                for entry in &packet.entries {
                    self.tab_list.insert(
                        entry.uuid,
                        TabListEntryState {
                            uuid: entry.uuid,
                            name: entry
                                .name
                                .clone()
                                .unwrap_or_else(|| format_uuid(entry.uuid)),
                            display_name_json: entry.display_name_json.clone(),
                            latency: entry.latency.unwrap_or_default(),
                            game_mode: entry.game_mode.unwrap_or_default(),
                            property_count: entry.properties.len(),
                        },
                    );
                }
            }
            PlayerListItemAction::UpdateGameMode => {
                for entry in &packet.entries {
                    self.entry_for_uuid(entry.uuid).game_mode = entry.game_mode.unwrap_or_default();
                }
            }
            PlayerListItemAction::UpdateLatency => {
                for entry in &packet.entries {
                    self.entry_for_uuid(entry.uuid).latency = entry.latency.unwrap_or_default();
                }
            }
            PlayerListItemAction::UpdateDisplayName => {
                for entry in &packet.entries {
                    self.entry_for_uuid(entry.uuid).display_name_json =
                        entry.display_name_json.clone();
                }
            }
            PlayerListItemAction::RemovePlayer => {
                for entry in &packet.entries {
                    self.tab_list.remove(&entry.uuid);
                }
            }
        }
    }

    fn apply_scoreboard_objective(&mut self, packet: &ScoreboardObjectivePacket) {
        match packet.mode {
            ScoreboardObjectiveMode::Create | ScoreboardObjectiveMode::Update => {
                self.objectives.insert(
                    packet.objective_name.clone(),
                    ObjectiveState {
                        display_name: packet.objective_value.clone(),
                        render_type: packet.render_type.clone(),
                    },
                );
            }
            ScoreboardObjectiveMode::Remove => {
                self.objectives.remove(&packet.objective_name);
                self.display_slots
                    .retain(|_, objective| objective.as_str() != packet.objective_name);
                self.scores
                    .retain(|(objective, _), _| objective != &packet.objective_name);
            }
        }
    }

    fn apply_score(&mut self, packet: &UpdateScorePacket) {
        let key = (packet.objective_name.clone(), packet.score_name.clone());

        if packet.action == UpdateScoreAction::Remove {
            self.scores.remove(&key);
        } else {
            self.scores.insert(key, packet.value);
        }
    }

    fn apply_display_scoreboard(&mut self, packet: &DisplayScoreboardPacket) {
        if packet.score_name.is_empty() {
            self.display_slots.remove(&(packet.position as u8));
        } else {
            self.display_slots
                .insert(packet.position as u8, packet.score_name.clone());
        }
    }

    fn apply_team(&mut self, packet: &TeamsPacket) {
        match packet.action {
            TeamAction::Create => {
                self.teams.insert(
                    packet.name.clone(),
                    TeamState {
                        display_name: packet.display_name.clone(),
                        prefix: packet.prefix.clone(),
                        suffix: packet.suffix.clone(),
                        friendly_flags: packet.friendly_flags,
                        name_tag_visibility: packet.name_tag_visibility.clone(),
                        color: packet.color,
                        players: BTreeSet::new(),
                    },
                );
                self.assign_players_to_team(&packet.name, &packet.players);
            }
            TeamAction::Remove => {
                if let Some(team) = self.teams.remove(&packet.name) {
                    for player in team.players {
                        self.player_teams.remove(&player);
                    }
                }
            }
            TeamAction::Update => {
                let team = self.teams.entry(packet.name.clone()).or_default();
                team.display_name = packet.display_name.clone();
                team.prefix = packet.prefix.clone();
                team.suffix = packet.suffix.clone();
                team.friendly_flags = packet.friendly_flags;
                team.name_tag_visibility = packet.name_tag_visibility.clone();
                team.color = packet.color;
            }
            TeamAction::AddPlayers => {
                self.assign_players_to_team(&packet.name, &packet.players);
            }
            TeamAction::RemovePlayers => {
                self.remove_players_from_team(&packet.name, &packet.players);
            }
        }
    }

    fn sidebar_snapshot(&self) -> Option<SidebarSnapshot> {
        let objective_name = self.display_slots.get(&SIDEBAR_DISPLAY_SLOT)?.clone();
        let objective = self.objectives.get(&objective_name)?;
        let mut lines = self
            .scores
            .iter()
            .filter_map(|((score_objective, entry_name), value)| {
                if score_objective == &objective_name {
                    Some(SidebarLine {
                        entry_name: entry_name.clone(),
                        rendered_name: self.rendered_score_name(entry_name),
                        value: *value,
                    })
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        lines.sort_by(|left, right| {
            right
                .value
                .cmp(&left.value)
                .then(left.entry_name.cmp(&right.entry_name))
        });
        lines.truncate(15);

        Some(SidebarSnapshot {
            objective_name,
            title: objective.display_name.clone(),
            render_type: objective.render_type.clone(),
            lines,
        })
    }

    fn rendered_score_name(&self, entry_name: &str) -> String {
        self.player_teams
            .get(entry_name)
            .and_then(|team_name| self.teams.get(team_name))
            .map(|team| format!("{}{}{}", team.prefix, entry_name, team.suffix))
            .unwrap_or_else(|| entry_name.to_owned())
    }

    fn assign_players_to_team(&mut self, team_name: &str, players: &[String]) {
        for player in players {
            if let Some(previous_team) = self
                .player_teams
                .insert(player.clone(), team_name.to_owned())
            {
                if previous_team != team_name {
                    if let Some(previous) = self.teams.get_mut(&previous_team) {
                        previous.players.remove(player);
                    }
                }
            }

            self.teams
                .entry(team_name.to_owned())
                .or_default()
                .players
                .insert(player.clone());
        }
    }

    fn remove_players_from_team(&mut self, team_name: &str, players: &[String]) {
        if let Some(team) = self.teams.get_mut(team_name) {
            for player in players {
                team.players.remove(player);

                if self
                    .player_teams
                    .get(player)
                    .map(|assigned| assigned == team_name)
                    .unwrap_or(false)
                {
                    self.player_teams.remove(player);
                }
            }
        }
    }

    fn entry_for_uuid(&mut self, uuid: [u8; 16]) -> &mut TabListEntryState {
        self.tab_list
            .entry(uuid)
            .or_insert_with(|| TabListEntryState {
                uuid,
                name: format_uuid(uuid),
                display_name_json: None,
                latency: 0,
                game_mode: 0,
                property_count: 0,
            })
    }
}

fn format_uuid(uuid: [u8; 16]) -> String {
    uuid.iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::{UsabilityState, MAX_AUDIO_CUES, MAX_CHAT_LINES, SIDEBAR_DISPLAY_SLOT};
    use rmc_net::codec::play::{
        ChatMessagePacket, DisplayScoreboardPacket, PlayClientboundPacket, PlayerListEntry,
        PlayerListItemAction, PlayerListItemPacket, ScoreboardObjectiveMode,
        ScoreboardObjectivePacket, SoundEffectPacket, TeamAction, TeamsPacket, UpdateScoreAction,
        UpdateScorePacket,
    };

    #[test]
    fn action_bar_replaces_previous_message_expires_and_never_enters_chat_history() {
        let mut state = UsabilityState::new();
        for text in ["first", "second"] {
            state.apply_play_packet(&PlayClientboundPacket::ChatMessage(ChatMessagePacket {
                message_json: format!("{{\"text\":\"{text}\"}}"),
                position: 2,
            }));
            assert!(state.snapshot().chat_lines.is_empty());
            assert_eq!(
                state
                    .snapshot()
                    .action_bar
                    .as_ref()
                    .unwrap()
                    .remaining_ticks,
                60
            );
            state.advance_chat_ticks(10);
        }
        assert!(state
            .snapshot()
            .action_bar
            .as_ref()
            .unwrap()
            .message_json
            .contains("second"));
        state.advance_chat_ticks(49);
        assert_eq!(
            state
                .snapshot()
                .action_bar
                .as_ref()
                .unwrap()
                .remaining_ticks,
            1
        );
        state.advance_chat_ticks(1);
        assert!(state.snapshot().action_bar.is_none());
    }

    #[test]
    fn chat_age_uses_simulation_ticks_and_new_messages_start_at_zero() {
        let mut state = UsabilityState::new();
        let packet = ChatMessagePacket {
            message_json: "{\"text\":\"hello\"}".into(),
            position: 0,
        };
        state.apply_play_packet(&PlayClientboundPacket::ChatMessage(packet.clone()));
        state.advance_chat_ticks(190);
        state.apply_play_packet(&PlayClientboundPacket::ChatMessage(packet));
        let snapshot = state.snapshot();
        assert_eq!(snapshot.chat_lines[0].age_ticks, 190);
        assert_eq!(snapshot.chat_lines[1].age_ticks, 0);
        state.advance_chat_ticks(10);
        assert_eq!(state.snapshot().chat_lines[0].age_ticks, 200);
        assert_eq!(state.snapshot().chat_lines.len(), 2);
    }

    #[test]
    fn chat_is_capped_and_outbound_chat_is_truncated() {
        let mut state = UsabilityState::new();

        for index in 0..(MAX_CHAT_LINES + 5) {
            state.apply_play_packet(&PlayClientboundPacket::ChatMessage(ChatMessagePacket {
                message_json: format!("{{\"text\":\"line{index}\"}}"),
                position: 0,
            }));
        }

        assert_eq!(state.snapshot().chat_lines.len(), MAX_CHAT_LINES);

        let packet = state.send_chat_message(&"x".repeat(140));
        assert!(matches!(
            packet,
            rmc_net::codec::play::PlayServerboundPacket::ChatMessage(packet)
                if packet.message.chars().count() == 100
        ));
    }

    #[test]
    fn sidebar_uses_team_formatting() {
        let mut state = UsabilityState::new();
        state.apply_play_packet(&PlayClientboundPacket::ScoreboardObjective(
            ScoreboardObjectivePacket {
                objective_name: "bw".to_owned(),
                mode: ScoreboardObjectiveMode::Create,
                objective_value: "BED WARS".to_owned(),
                render_type: "integer".to_owned(),
            },
        ));
        state.apply_play_packet(&PlayClientboundPacket::DisplayScoreboard(
            DisplayScoreboardPacket {
                position: SIDEBAR_DISPLAY_SLOT as i8,
                score_name: "bw".to_owned(),
            },
        ));
        state.apply_play_packet(&PlayClientboundPacket::Teams(TeamsPacket {
            name: "red".to_owned(),
            action: TeamAction::Create,
            display_name: "Red".to_owned(),
            prefix: "[R] ".to_owned(),
            suffix: String::new(),
            friendly_flags: 0,
            name_tag_visibility: "always".to_owned(),
            color: 12,
            players: vec!["Rush".to_owned()],
        }));
        state.apply_play_packet(&PlayClientboundPacket::UpdateScore(UpdateScorePacket {
            score_name: "Rush".to_owned(),
            action: UpdateScoreAction::Change,
            objective_name: "bw".to_owned(),
            value: 12,
        }));

        let snapshot = state.snapshot();
        assert_eq!(
            snapshot
                .sidebar
                .as_ref()
                .map(|sidebar| sidebar.title.clone()),
            Some("BED WARS".to_owned())
        );
        assert_eq!(
            snapshot
                .sidebar
                .as_ref()
                .and_then(|sidebar| sidebar.lines.first())
                .map(|line| line.rendered_name.clone()),
            Some("[R] Rush".to_owned())
        );
    }

    #[test]
    fn tab_score_tracks_display_slot_zero_updates_and_objective_removal() {
        let mut state = UsabilityState::new();
        state.apply_play_packet(&PlayClientboundPacket::PlayerListItem(
            PlayerListItemPacket {
                action: PlayerListItemAction::AddPlayer,
                entries: vec![PlayerListEntry {
                    uuid: [1; 16],
                    name: Some("Alex".into()),
                    properties: vec![],
                    game_mode: Some(0),
                    latency: Some(1),
                    display_name_json: None,
                }],
            },
        ));
        let mut objective = ScoreboardObjectivePacket {
            objective_name: "points".into(),
            mode: ScoreboardObjectiveMode::Create,
            objective_value: "Points".into(),
            render_type: "integer".into(),
        };
        state.apply_play_packet(&PlayClientboundPacket::ScoreboardObjective(
            objective.clone(),
        ));
        state.apply_play_packet(&PlayClientboundPacket::DisplayScoreboard(
            DisplayScoreboardPacket {
                position: 0,
                score_name: "points".into(),
            },
        ));
        assert_eq!(
            state.snapshot().tab_list[0].tab_score,
            Some((0, "integer".into()))
        );
        state.apply_play_packet(&PlayClientboundPacket::UpdateScore(UpdateScorePacket {
            score_name: "Alex".into(),
            action: UpdateScoreAction::Change,
            objective_name: "points".into(),
            value: -42,
        }));
        assert_eq!(
            state.snapshot().tab_list[0].tab_score,
            Some((-42, "integer".into()))
        );
        objective.mode = ScoreboardObjectiveMode::Remove;
        state.apply_play_packet(&PlayClientboundPacket::ScoreboardObjective(objective));
        assert_eq!(state.snapshot().tab_list[0].tab_score, None);
    }

    #[test]
    fn tab_team_name_updates_from_received_team_packets() {
        let mut state = UsabilityState::new();
        state.apply_play_packet(&PlayClientboundPacket::PlayerListItem(
            PlayerListItemPacket {
                action: PlayerListItemAction::AddPlayer,
                entries: vec![PlayerListEntry {
                    uuid: [1; 16],
                    name: Some("Alex".into()),
                    properties: vec![],
                    game_mode: Some(0),
                    latency: Some(0),
                    display_name_json: None,
                }],
            },
        ));
        let mut team = TeamsPacket {
            name: "red".into(),
            action: TeamAction::Create,
            display_name: "Red".into(),
            prefix: "[R] ".into(),
            suffix: "!".into(),
            friendly_flags: 0,
            name_tag_visibility: "always".into(),
            color: 12,
            players: vec!["Alex".into()],
        };
        assert!(
            state
                .apply_play_packet(&PlayClientboundPacket::Teams(team.clone()))
                .tab_list_updated
        );
        assert_eq!(
            state.snapshot().tab_list[0].team_formatted_name,
            "[R] Alex!"
        );
        team.action = TeamAction::Update;
        team.prefix = "[Red] ".into();
        assert!(
            state
                .apply_play_packet(&PlayClientboundPacket::Teams(team.clone()))
                .tab_list_updated
        );
        assert_eq!(
            state.snapshot().tab_list[0].team_formatted_name,
            "[Red] Alex!"
        );
        team.action = TeamAction::RemovePlayers;
        state.apply_play_packet(&PlayClientboundPacket::Teams(team));
        assert_eq!(state.snapshot().tab_list[0].team_formatted_name, "Alex");
    }

    #[test]
    fn tab_order_places_spectators_last_then_team_then_java_name() {
        let mut state = UsabilityState::new();
        for (id, name, mode, team) in [
            (1, "Zed", 0, "a"),
            (2, "Alpha", 0, "b"),
            (3, "Aaron", 3, ""),
            (4, "Beta", 0, ""),
        ] {
            state.tab_list.insert(
                [id; 16],
                super::TabListEntryState {
                    uuid: [id; 16],
                    name: name.into(),
                    game_mode: mode,
                    latency: 0,
                    display_name_json: None,
                    property_count: 0,
                },
            );
            if !team.is_empty() {
                state.player_teams.insert(name.into(), team.into());
            }
        }
        assert_eq!(
            state
                .snapshot()
                .tab_list
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Beta", "Zed", "Alpha", "Aaron"]
        );
    }

    #[test]
    fn tab_list_add_update_and_remove_flow() {
        let mut state = UsabilityState::new();
        state.apply_play_packet(&PlayClientboundPacket::PlayerListItem(
            PlayerListItemPacket {
                action: PlayerListItemAction::AddPlayer,
                entries: vec![PlayerListEntry {
                    uuid: [2; 16],
                    name: Some("Rush".to_owned()),
                    properties: Vec::new(),
                    game_mode: Some(1),
                    latency: Some(45),
                    display_name_json: None,
                }],
            },
        ));
        state.apply_play_packet(&PlayClientboundPacket::PlayerListItem(
            PlayerListItemPacket {
                action: PlayerListItemAction::UpdateLatency,
                entries: vec![PlayerListEntry {
                    uuid: [2; 16],
                    name: None,
                    properties: Vec::new(),
                    game_mode: None,
                    latency: Some(12),
                    display_name_json: None,
                }],
            },
        ));

        assert_eq!(state.snapshot().tab_list[0].latency, 12);

        state.apply_play_packet(&PlayClientboundPacket::PlayerListItem(
            PlayerListItemPacket {
                action: PlayerListItemAction::RemovePlayer,
                entries: vec![PlayerListEntry {
                    uuid: [2; 16],
                    name: None,
                    properties: Vec::new(),
                    game_mode: None,
                    latency: None,
                    display_name_json: None,
                }],
            },
        ));

        assert!(state.snapshot().tab_list.is_empty());
    }

    #[test]
    fn audio_respects_local_settings() {
        let mut state = UsabilityState::new();
        state.settings_mut().audio_enabled = false;

        let updated =
            state.apply_play_packet(&PlayClientboundPacket::SoundEffect(SoundEffectPacket {
                sound_name: "note.pling".to_owned(),
                effect_position_x: 8,
                effect_position_y: 16,
                effect_position_z: 24,
                volume: 1.0,
                pitch: 63,
            }));
        assert!(!updated.audio_updated);
        assert!(state.snapshot().recent_sounds.is_empty());

        state.settings_mut().audio_enabled = true;
        state.settings_mut().audio_volume = 0.5;
        for _ in 0..(MAX_AUDIO_CUES + 2) {
            state.apply_play_packet(&PlayClientboundPacket::SoundEffect(SoundEffectPacket {
                sound_name: "note.pling".to_owned(),
                effect_position_x: 8,
                effect_position_y: 16,
                effect_position_z: 24,
                volume: 1.0,
                pitch: 63,
            }));
        }

        let snapshot = state.snapshot();
        assert_eq!(snapshot.recent_sounds.len(), MAX_AUDIO_CUES);
        assert_eq!(snapshot.recent_sounds[0].volume, 0.5);
    }
}
