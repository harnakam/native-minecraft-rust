use rmc_game::usability::UsabilityState;
use rmc_net::codec::play::{
    ChatMessagePacket, DisplayScoreboardPacket, ItemStack, OpenWindowPacket, PlayClientboundPacket,
    PlayServerboundPacket, PlayerListEntry, PlayerListItemAction, PlayerListItemPacket,
    ScoreboardObjectiveMode, ScoreboardObjectivePacket, SetSlotPacket, SoundEffectPacket,
    TeamAction, TeamsPacket, UpdateScoreAction, UpdateScorePacket,
};
use rmc_ui::PvPHud;
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Debug)]
pub struct UsabilityCliOptions {
    pub trace_path: Option<String>,
    pub send_chat: Option<String>,
    pub chat_json: Option<String>,
    pub chat_position: i8,
    pub tab_uuid: Option<[u8; 16]>,
    pub tab_name: Option<String>,
    pub tab_display_json: Option<String>,
    pub tab_ping: i32,
    pub tab_game_mode: i32,
    pub tab_remove: bool,
    pub scoreboard_objective: Option<String>,
    pub scoreboard_title: String,
    pub scoreboard_render_type: String,
    pub scoreboard_remove: bool,
    pub display_slot: i8,
    pub score_entry: Option<String>,
    pub score_objective: Option<String>,
    pub score_value: i32,
    pub score_remove: bool,
    pub team_name: Option<String>,
    pub team_display: String,
    pub team_prefix: String,
    pub team_suffix: String,
    pub team_visibility: String,
    pub team_color: i8,
    pub team_player: Option<String>,
    pub team_remove: bool,
    pub open_window_id: Option<u8>,
    pub open_window_type: String,
    pub open_window_title: String,
    pub open_window_slots: u8,
    pub set_slot_window_id: Option<i8>,
    pub set_slot_id: Option<i16>,
    pub set_slot_item_id: Option<i16>,
    pub set_slot_item_count: u8,
    pub set_slot_item_damage: i16,
    pub server_close_window_id: Option<u8>,
    pub client_close_window: bool,
    pub locale: Option<String>,
    pub view_distance: Option<i8>,
    pub chat_visibility: Option<u8>,
    pub disable_chat_colors: bool,
    pub mute_audio: bool,
    pub audio_volume: Option<f32>,
    pub sound_name: Option<String>,
    pub sound_x: f64,
    pub sound_y: f64,
    pub sound_z: f64,
    pub sound_volume: f32,
    pub sound_pitch: u8,
}

impl UsabilityCliOptions {
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            trace_path: None,
            send_chat: None,
            chat_json: None,
            chat_position: 0,
            tab_uuid: None,
            tab_name: None,
            tab_display_json: None,
            tab_ping: 0,
            tab_game_mode: 0,
            tab_remove: false,
            scoreboard_objective: None,
            scoreboard_title: "BED WARS".to_owned(),
            scoreboard_render_type: "integer".to_owned(),
            scoreboard_remove: false,
            display_slot: 1,
            score_entry: None,
            score_objective: None,
            score_value: 0,
            score_remove: false,
            team_name: None,
            team_display: String::new(),
            team_prefix: String::new(),
            team_suffix: String::new(),
            team_visibility: "always".to_owned(),
            team_color: -1,
            team_player: None,
            team_remove: false,
            open_window_id: None,
            open_window_type: "minecraft:chest".to_owned(),
            open_window_title: "{\"text\":\"Loot\"}".to_owned(),
            open_window_slots: 27,
            set_slot_window_id: None,
            set_slot_id: None,
            set_slot_item_id: None,
            set_slot_item_count: 1,
            set_slot_item_damage: 0,
            server_close_window_id: None,
            client_close_window: false,
            locale: None,
            view_distance: None,
            chat_visibility: None,
            disable_chat_colors: false,
            mute_audio: false,
            audio_volume: None,
            sound_name: None,
            sound_x: 0.0,
            sound_y: 0.0,
            sound_z: 0.0,
            sound_volume: 1.0,
            sound_pitch: 63,
        };

        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "usability" => {}
                "--help" | "-h" => return Err(Self::usage()),
                "--trace" => options.trace_path = Some(next_value(&mut args, "--trace")?),
                "--send-chat" => options.send_chat = Some(next_value(&mut args, "--send-chat")?),
                "--chat-json" => options.chat_json = Some(next_value(&mut args, "--chat-json")?),
                "--chat-position" => {
                    options.chat_position = next_value(&mut args, "--chat-position")?
                        .parse()
                        .map_err(|_| "invalid --chat-position value".to_owned())?;
                }
                "--tab-uuid" => {
                    options.tab_uuid = Some(parse_uuid_hex(&next_value(&mut args, "--tab-uuid")?)?);
                }
                "--tab-name" => options.tab_name = Some(next_value(&mut args, "--tab-name")?),
                "--tab-display-json" => {
                    options.tab_display_json = Some(next_value(&mut args, "--tab-display-json")?);
                }
                "--tab-ping" => {
                    options.tab_ping = next_value(&mut args, "--tab-ping")?
                        .parse()
                        .map_err(|_| "invalid --tab-ping value".to_owned())?;
                }
                "--tab-gamemode" => {
                    options.tab_game_mode = next_value(&mut args, "--tab-gamemode")?
                        .parse()
                        .map_err(|_| "invalid --tab-gamemode value".to_owned())?;
                }
                "--tab-remove" => options.tab_remove = true,
                "--scoreboard-objective" => {
                    options.scoreboard_objective =
                        Some(next_value(&mut args, "--scoreboard-objective")?);
                }
                "--scoreboard-title" => {
                    options.scoreboard_title = next_value(&mut args, "--scoreboard-title")?;
                }
                "--scoreboard-render-type" => {
                    options.scoreboard_render_type =
                        next_value(&mut args, "--scoreboard-render-type")?;
                }
                "--scoreboard-remove" => options.scoreboard_remove = true,
                "--display-slot" => {
                    options.display_slot = next_value(&mut args, "--display-slot")?
                        .parse()
                        .map_err(|_| "invalid --display-slot value".to_owned())?;
                }
                "--score-entry" => {
                    options.score_entry = Some(next_value(&mut args, "--score-entry")?)
                }
                "--score-objective" => {
                    options.score_objective = Some(next_value(&mut args, "--score-objective")?);
                }
                "--score-value" => {
                    options.score_value = next_value(&mut args, "--score-value")?
                        .parse()
                        .map_err(|_| "invalid --score-value value".to_owned())?;
                }
                "--score-remove" => options.score_remove = true,
                "--team-name" => options.team_name = Some(next_value(&mut args, "--team-name")?),
                "--team-display" => options.team_display = next_value(&mut args, "--team-display")?,
                "--team-prefix" => options.team_prefix = next_value(&mut args, "--team-prefix")?,
                "--team-suffix" => options.team_suffix = next_value(&mut args, "--team-suffix")?,
                "--team-visibility" => {
                    options.team_visibility = next_value(&mut args, "--team-visibility")?;
                }
                "--team-color" => {
                    options.team_color = next_value(&mut args, "--team-color")?
                        .parse()
                        .map_err(|_| "invalid --team-color value".to_owned())?;
                }
                "--team-player" => {
                    options.team_player = Some(next_value(&mut args, "--team-player")?)
                }
                "--team-remove" => options.team_remove = true,
                "--open-window-id" => {
                    options.open_window_id = Some(
                        next_value(&mut args, "--open-window-id")?
                            .parse()
                            .map_err(|_| "invalid --open-window-id value".to_owned())?,
                    );
                }
                "--open-window-type" => {
                    options.open_window_type = next_value(&mut args, "--open-window-type")?;
                }
                "--open-window-title" => {
                    options.open_window_title = next_value(&mut args, "--open-window-title")?;
                }
                "--open-window-slots" => {
                    options.open_window_slots = next_value(&mut args, "--open-window-slots")?
                        .parse()
                        .map_err(|_| "invalid --open-window-slots value".to_owned())?;
                }
                "--set-slot-window-id" => {
                    options.set_slot_window_id = Some(
                        next_value(&mut args, "--set-slot-window-id")?
                            .parse()
                            .map_err(|_| "invalid --set-slot-window-id value".to_owned())?,
                    );
                }
                "--set-slot-id" => {
                    options.set_slot_id = Some(
                        next_value(&mut args, "--set-slot-id")?
                            .parse()
                            .map_err(|_| "invalid --set-slot-id value".to_owned())?,
                    );
                }
                "--set-slot-item-id" => {
                    options.set_slot_item_id = Some(
                        next_value(&mut args, "--set-slot-item-id")?
                            .parse()
                            .map_err(|_| "invalid --set-slot-item-id value".to_owned())?,
                    );
                }
                "--set-slot-item-count" => {
                    options.set_slot_item_count = next_value(&mut args, "--set-slot-item-count")?
                        .parse()
                        .map_err(|_| "invalid --set-slot-item-count value".to_owned())?;
                }
                "--set-slot-item-damage" => {
                    options.set_slot_item_damage = next_value(&mut args, "--set-slot-item-damage")?
                        .parse()
                        .map_err(|_| "invalid --set-slot-item-damage value".to_owned())?;
                }
                "--server-close-window-id" => {
                    options.server_close_window_id = Some(
                        next_value(&mut args, "--server-close-window-id")?
                            .parse()
                            .map_err(|_| "invalid --server-close-window-id value".to_owned())?,
                    );
                }
                "--client-close-window" => options.client_close_window = true,
                "--locale" => options.locale = Some(next_value(&mut args, "--locale")?),
                "--view-distance" => {
                    options.view_distance = Some(
                        next_value(&mut args, "--view-distance")?
                            .parse()
                            .map_err(|_| "invalid --view-distance value".to_owned())?,
                    );
                }
                "--chat-visibility" => {
                    options.chat_visibility = Some(
                        next_value(&mut args, "--chat-visibility")?
                            .parse()
                            .map_err(|_| "invalid --chat-visibility value".to_owned())?,
                    );
                }
                "--disable-chat-colors" => options.disable_chat_colors = true,
                "--mute-audio" => options.mute_audio = true,
                "--audio-volume" => {
                    options.audio_volume = Some(
                        next_value(&mut args, "--audio-volume")?
                            .parse()
                            .map_err(|_| "invalid --audio-volume value".to_owned())?,
                    );
                }
                "--sound-name" => options.sound_name = Some(next_value(&mut args, "--sound-name")?),
                "--sound-x" => {
                    options.sound_x = next_value(&mut args, "--sound-x")?
                        .parse()
                        .map_err(|_| "invalid --sound-x value".to_owned())?;
                }
                "--sound-y" => {
                    options.sound_y = next_value(&mut args, "--sound-y")?
                        .parse()
                        .map_err(|_| "invalid --sound-y value".to_owned())?;
                }
                "--sound-z" => {
                    options.sound_z = next_value(&mut args, "--sound-z")?
                        .parse()
                        .map_err(|_| "invalid --sound-z value".to_owned())?;
                }
                "--sound-volume" => {
                    options.sound_volume = next_value(&mut args, "--sound-volume")?
                        .parse()
                        .map_err(|_| "invalid --sound-volume value".to_owned())?;
                }
                "--sound-pitch" => {
                    options.sound_pitch = next_value(&mut args, "--sound-pitch")?
                        .parse()
                        .map_err(|_| "invalid --sound-pitch value".to_owned())?;
                }
                other => return Err(format!("unknown argument: {other}\n\n{}", Self::usage())),
            }
        }

        if options.tab_name.is_some() && options.tab_uuid.is_none() {
            options.tab_uuid = Some([1; 16]);
        }

        if options.tab_remove && options.tab_uuid.is_none() {
            return Err("--tab-remove requires --tab-uuid".to_owned());
        }

        if options.set_slot_window_id.is_some() ^ options.set_slot_id.is_some() {
            return Err(
                "--set-slot-window-id and --set-slot-id must be provided together".to_owned(),
            );
        }

        Ok(options)
    }

    pub fn usage() -> String {
        [
            "usage:",
            "  rmc-client usability [options]",
            "",
            "options:",
            "  --trace PATH",
            "  --send-chat MESSAGE",
            "  --chat-json JSON [--chat-position N]",
            "  --tab-uuid HEX32 --tab-name NAME [--tab-display-json JSON] [--tab-ping N] [--tab-gamemode N] [--tab-remove]",
            "  --scoreboard-objective NAME [--scoreboard-title TITLE] [--scoreboard-render-type TYPE] [--scoreboard-remove] [--display-slot N]",
            "  --score-entry NAME [--score-objective NAME] [--score-value N] [--score-remove]",
            "  --team-name NAME [--team-display NAME] [--team-prefix TEXT] [--team-suffix TEXT] [--team-player NAME] [--team-remove]",
            "  --open-window-id N [--open-window-type TYPE] [--open-window-title JSON] [--open-window-slots N]",
            "  --set-slot-window-id N --set-slot-id N [--set-slot-item-id ID --set-slot-item-count N --set-slot-item-damage N]",
            "  --server-close-window-id N",
            "  --client-close-window",
            "  --locale LOCALE [--view-distance N] [--chat-visibility N] [--disable-chat-colors]",
            "  --mute-audio | --audio-volume VALUE",
            "  --sound-name NAME [--sound-x X --sound-y Y --sound-z Z --sound-volume V --sound-pitch N]",
        ]
        .join("\n")
    }
}

pub fn run_usability_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = UsabilityCliOptions::parse(raw_args)?;
    let mut state = UsabilityState::new();
    let mut trace_lines = Vec::new();
    let mut outbound_packets = Vec::new();

    apply_settings(&options, &mut state, &mut outbound_packets);
    apply_inbound_packets(&options, &mut state, &mut trace_lines);

    if options.client_close_window {
        if let Some(packet) = state.close_open_window() {
            outbound_packets.push(packet);
        }
    }

    if let Some(message) = &options.send_chat {
        outbound_packets.push(state.send_chat_message(message));
    }

    let snapshot = state.snapshot();
    let hud = PvPHud::from_snapshot(&snapshot);

    println!("chat_lines={}", snapshot.chat_lines.len());
    println!("tab_entries={}", snapshot.tab_list.len());
    println!("sidebar_visible={}", snapshot.sidebar.is_some());
    println!("window_open={}", snapshot.window.is_some());
    println!("recent_sounds={}", snapshot.recent_sounds.len());

    for line in hud.render_lines() {
        println!("{line}");
        trace_lines.push(line);
    }

    for (index, packet) in outbound_packets.iter().enumerate() {
        let line = render_outbound_packet(index as u64 + 1, packet);
        println!("{line}");
        trace_lines.push(line);
    }

    if let Some(trace_path) = &options.trace_path {
        write_lines(trace_path, &trace_lines)?;
    }

    Ok(())
}

fn apply_settings(
    options: &UsabilityCliOptions,
    state: &mut UsabilityState,
    outbound_packets: &mut Vec<PlayServerboundPacket>,
) {
    let mut network_changed = false;
    {
        let settings = state.settings_mut();

        if let Some(locale) = &options.locale {
            settings.locale = locale.clone();
            network_changed = true;
        }

        if let Some(view_distance) = options.view_distance {
            settings.view_distance = view_distance;
            network_changed = true;
        }

        if let Some(chat_visibility) = options.chat_visibility {
            settings.chat_visibility = chat_visibility;
            network_changed = true;
        }

        if options.disable_chat_colors {
            settings.chat_colors = false;
            network_changed = true;
        }

        if options.mute_audio {
            settings.audio_enabled = false;
        }

        if let Some(audio_volume) = options.audio_volume {
            settings.audio_volume = audio_volume;
        }
    }

    if network_changed {
        outbound_packets.push(state.sync_settings_packet());
    }
}

fn apply_inbound_packets(
    options: &UsabilityCliOptions,
    state: &mut UsabilityState,
    trace_lines: &mut Vec<String>,
) {
    if let Some(chat_json) = &options.chat_json {
        let packet = PlayClientboundPacket::ChatMessage(ChatMessagePacket {
            message_json: chat_json.clone(),
            position: options.chat_position,
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }

    if let Some(window_id) = options.open_window_id {
        let packet = PlayClientboundPacket::OpenWindow(OpenWindowPacket {
            window_id,
            inventory_type: options.open_window_type.clone(),
            window_title_json: options.open_window_title.clone(),
            slot_count: options.open_window_slots,
            entity_id: None,
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }

    if let (Some(window_id), Some(slot_id)) = (options.set_slot_window_id, options.set_slot_id) {
        let packet = PlayClientboundPacket::SetSlot(SetSlotPacket {
            window_id,
            slot_id,
            item: options.set_slot_item_id.map(|item_id| {
                ItemStack::simple(
                    item_id,
                    options.set_slot_item_count,
                    options.set_slot_item_damage,
                )
            }),
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }

    if let Some(window_id) = options.server_close_window_id {
        let packet = PlayClientboundPacket::CloseWindow(rmc_net::codec::play::CloseWindowPacket {
            window_id,
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }

    if let Some(tab_uuid) = options.tab_uuid {
        let action = if options.tab_remove {
            PlayerListItemAction::RemovePlayer
        } else {
            PlayerListItemAction::AddPlayer
        };
        let packet = PlayClientboundPacket::PlayerListItem(PlayerListItemPacket {
            action,
            entries: vec![PlayerListEntry {
                uuid: tab_uuid,
                name: options.tab_name.clone(),
                properties: Vec::new(),
                game_mode: if options.tab_remove {
                    None
                } else {
                    Some(options.tab_game_mode)
                },
                latency: if options.tab_remove {
                    None
                } else {
                    Some(options.tab_ping)
                },
                display_name_json: if options.tab_remove {
                    None
                } else {
                    options.tab_display_json.clone()
                },
            }],
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }

    if let Some(objective_name) = &options.scoreboard_objective {
        let mode = if options.scoreboard_remove {
            ScoreboardObjectiveMode::Remove
        } else {
            ScoreboardObjectiveMode::Create
        };
        let packet = PlayClientboundPacket::ScoreboardObjective(ScoreboardObjectivePacket {
            objective_name: objective_name.clone(),
            mode,
            objective_value: options.scoreboard_title.clone(),
            render_type: options.scoreboard_render_type.clone(),
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);

        if !options.scoreboard_remove {
            let packet = PlayClientboundPacket::DisplayScoreboard(DisplayScoreboardPacket {
                position: options.display_slot,
                score_name: objective_name.clone(),
            });
            trace_lines.push(render_inbound_packet(&packet));
            state.apply_play_packet(&packet);
        }
    }

    if let Some(score_entry) = &options.score_entry {
        let objective_name = options
            .score_objective
            .clone()
            .or_else(|| options.scoreboard_objective.clone())
            .unwrap_or_else(|| "sidebar".to_owned());
        let packet = PlayClientboundPacket::UpdateScore(UpdateScorePacket {
            score_name: score_entry.clone(),
            action: if options.score_remove {
                UpdateScoreAction::Remove
            } else {
                UpdateScoreAction::Change
            },
            objective_name,
            value: options.score_value,
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }

    if let Some(team_name) = &options.team_name {
        let packet = PlayClientboundPacket::Teams(TeamsPacket {
            name: team_name.clone(),
            action: if options.team_remove {
                TeamAction::Remove
            } else {
                TeamAction::Create
            },
            display_name: options.team_display.clone(),
            prefix: options.team_prefix.clone(),
            suffix: options.team_suffix.clone(),
            friendly_flags: 0,
            name_tag_visibility: options.team_visibility.clone(),
            color: options.team_color,
            players: options.team_player.clone().into_iter().collect(),
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }

    if let Some(sound_name) = &options.sound_name {
        let packet = PlayClientboundPacket::SoundEffect(SoundEffectPacket {
            sound_name: sound_name.clone(),
            effect_position_x: (options.sound_x * 8.0) as i32,
            effect_position_y: (options.sound_y * 8.0) as i32,
            effect_position_z: (options.sound_z * 8.0) as i32,
            volume: options.sound_volume,
            pitch: options.sound_pitch,
        });
        trace_lines.push(render_inbound_packet(&packet));
        state.apply_play_packet(&packet);
    }
}

fn render_inbound_packet(packet: &PlayClientboundPacket) -> String {
    match packet {
        PlayClientboundPacket::ChatMessage(packet) => {
            format!(
                "inbound chat position={} json={}",
                packet.position, packet.message_json
            )
        }
        PlayClientboundPacket::OpenWindow(packet) => format!(
            "inbound open_window id={} type={} title={} slots={}",
            packet.window_id, packet.inventory_type, packet.window_title_json, packet.slot_count
        ),
        PlayClientboundPacket::CloseWindow(packet) => {
            format!("inbound close_window id={}", packet.window_id)
        }
        PlayClientboundPacket::SetSlot(packet) => format!(
            "inbound set_slot window_id={} slot_id={} item={}",
            packet.window_id,
            packet.slot_id,
            packet
                .item
                .as_ref()
                .map(|item| format!("{}:{}:{}", item.item_id, item.count, item.damage))
                .unwrap_or_else(|| "none".to_owned())
        ),
        PlayClientboundPacket::PlayerListItem(packet) => format!(
            "inbound tab action={:?} entries={}",
            packet.action,
            packet.entries.len()
        ),
        PlayClientboundPacket::ScoreboardObjective(packet) => format!(
            "inbound scoreboard_objective name={} mode={:?} title={}",
            packet.objective_name, packet.mode, packet.objective_value
        ),
        PlayClientboundPacket::UpdateScore(packet) => format!(
            "inbound update_score entry={} objective={} action={:?} value={}",
            packet.score_name, packet.objective_name, packet.action, packet.value
        ),
        PlayClientboundPacket::DisplayScoreboard(packet) => format!(
            "inbound display_scoreboard slot={} objective={}",
            packet.position, packet.score_name
        ),
        PlayClientboundPacket::Teams(packet) => format!(
            "inbound team name={} action={:?} players={}",
            packet.name,
            packet.action,
            packet.players.len()
        ),
        PlayClientboundPacket::SoundEffect(packet) => format!(
            "inbound sound name={} x={:.3} y={:.3} z={:.3} volume={:.3} pitch={:.3}",
            packet.sound_name,
            packet.x(),
            packet.y(),
            packet.z(),
            packet.volume,
            packet.pitch_value()
        ),
        _ => "inbound unsupported".to_owned(),
    }
}

fn render_outbound_packet(index: u64, packet: &PlayServerboundPacket) -> String {
    match packet {
        PlayServerboundPacket::ChatMessage(packet) => {
            format!("outbound[{index}] packet=ChatMessage message={}", packet.message)
        }
        PlayServerboundPacket::CloseWindow(packet) => {
            format!("outbound[{index}] packet=CloseWindow window_id={}", packet.window_id)
        }
        PlayServerboundPacket::ClientSettings(packet) => format!(
            "outbound[{index}] packet=ClientSettings locale={} view_distance={} chat_visibility={} chat_colors={} displayed_skin_parts={}",
            packet.locale,
            packet.view_distance,
            packet.chat_visibility,
            packet.chat_colors,
            packet.displayed_skin_parts
        ),
        other => format!("outbound[{index}] packet={other:?}"),
    }
}

fn parse_uuid_hex(value: &str) -> Result<[u8; 16], String> {
    let compact = value.replace('-', "");

    if compact.len() != 32 {
        return Err("uuid must be 32 hex chars".to_owned());
    }

    let mut uuid = [0; 16];

    for (index, chunk) in compact.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        uuid[index] = (high << 4) | low;
    }

    Ok(uuid)
}

fn hex_value(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("uuid contains non-hex characters".to_owned()),
    }
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn write_lines(path: &str, lines: &[String]) -> Result<(), String> {
    let file = File::create(path).map_err(|error| format!("failed to create {path}: {error}"))?;
    let mut writer = BufWriter::new(file);

    for line in lines {
        writeln!(writer, "{line}").map_err(|error| format!("failed to write {path}: {error}"))?;
    }

    writer
        .flush()
        .map_err(|error| format!("failed to flush {path}: {error}"))
}
