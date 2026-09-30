use crate::live_runtime::{
    EntityTracker, LiveRuntime, LiveRuntimeConfig, RuntimeActionInput, TargetedEntity,
};
use crate::play_assets::{GameAssets, ImageAsset};
use crate::verification_cli::{hypixel_gate_decision, GateDecision};
use arboard::Clipboard;
use font8x8::UnicodeFonts;
use fontdue::{Font, FontSettings};
use pixels::{Pixels, SurfaceTexture};
use rmc_game::input::{InputFrame, PhysicalInput};
use rmc_game::player::Vec3;
use rmc_game::usability::WindowSnapshot;
use rmc_net::auth::OnlineAccount;
use rmc_net::microsoft::{
    authenticate_with_authorization_input, authenticate_with_refresh_token,
    load_account_sources_from_path, microsoft_login_url, AccountSource, AccountSourceKind,
    MicrosoftSession,
};
use rmc_render::{ChunkMesh, WorldRenderSnapshot};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event::{
    DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
    WindowEvent,
};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{CursorGrabMode, WindowBuilder};

const DEFAULT_WIDTH: u32 = 960;
const DEFAULT_HEIGHT: u32 = 540;
const NEAR_PLANE: f32 = 0.05;
const VERTICAL_FOV_DEGREES: f32 = 70.0;

#[derive(Debug)]
pub struct PlayCliOptions {
    pub server_host: String,
    pub server_port: u16,
    pub offline_username: String,
    pub width: u32,
    pub height: u32,
    pub accounts_path: Option<PathBuf>,
    pub account_name: Option<String>,
    pub auto_play: bool,
    pub force_offline: bool,
    pub duration_secs: Option<u64>,
    pub stop_after_join: bool,
    pub unsafe_hypixel: bool,
}

impl PlayCliOptions {
    pub fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut options = Self {
            server_host: "localhost".to_owned(),
            server_port: 25565,
            offline_username: default_offline_username(),
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            accounts_path: None,
            account_name: None,
            auto_play: false,
            force_offline: false,
            duration_secs: None,
            stop_after_join: false,
            unsafe_hypixel: false,
        };
        let mut args = args.into_iter();

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "play" => {}
                "--help" | "-h" => return Err(Self::usage()),
                "--server" => {
                    let (host, port) = parse_server_address(&next_value(&mut args, "--server")?)?;
                    options.server_host = host;
                    options.server_port = port;
                }
                "--offline-name" => {
                    options.offline_username = next_value(&mut args, "--offline-name")?;
                }
                "--width" => {
                    options.width = next_value(&mut args, "--width")?
                        .parse()
                        .map_err(|_| "invalid --width value".to_owned())?;
                }
                "--height" => {
                    options.height = next_value(&mut args, "--height")?
                        .parse()
                        .map_err(|_| "invalid --height value".to_owned())?;
                }
                "--accounts" => {
                    options.accounts_path =
                        Some(PathBuf::from(next_value(&mut args, "--accounts")?));
                }
                "--account" => {
                    options.account_name = Some(next_value(&mut args, "--account")?);
                }
                "--auto-play" => {
                    options.auto_play = true;
                }
                "--offline" => {
                    options.force_offline = true;
                }
                "--duration-secs" => {
                    options.duration_secs = Some(
                        next_value(&mut args, "--duration-secs")?
                            .parse()
                            .map_err(|_| "invalid --duration-secs value".to_owned())?,
                    );
                }
                "--stop-after-join" => {
                    options.stop_after_join = true;
                }
                "--unsafe-hypixel" => {
                    options.unsafe_hypixel = true;
                }
                other => return Err(format!("unknown argument: {other}\n\n{}", Self::usage())),
            }
        }

        if options.width < 320 || options.height < 180 {
            return Err("window size is too small".to_owned());
        }

        Ok(options)
    }

    pub fn usage() -> String {
        [
            "usage:",
            "  rmc-client play [--server HOST[:PORT]] [--offline-name NAME] [--accounts PATH] [--account NAME] [--width N --height N]",
            "",
            "options:",
            "  --offline-name NAME",
            "  --offline",
            "  --auto-play",
            "  --duration-secs N",
            "  --stop-after-join",
            "  --unsafe-hypixel",
            "",
            "notes:",
            "  Opens a windowed login and play path for private servers first.",
            "  --server defaults to localhost:25565 for fastest bring-up.",
            "  Loads launcher accounts from %APPDATA%\\.minecraft\\launcher_accounts.json when present.",
            "  --accounts can point at a custom JSON file with refresh tokens.",
            "  --offline forces Play Offline even when online accounts are loaded.",
            "  --auto-play starts the selected account immediately.",
            "  If no online account is selected, Play Offline uses --offline-name.",
            "  --duration-secs and --stop-after-join help with automated verification.",
            "  Hypixel is blocked by default until vanilla parity verification is done.",
        ]
        .join("\n")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScreenState {
    Menu,
    Playing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusField {
    None,
    ServerInput,
    OfflineUsernameInput,
    AuthInput,
}

#[derive(Clone, Copy, Debug)]
struct UiRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl UiRect {
    fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x as f32
            && x < (self.x + self.width) as f32
            && y >= self.y as f32
            && y < (self.y + self.height) as f32
    }
}

struct RuntimeInputState {
    held_keys: BTreeSet<PhysicalInput>,
    pressed_keys: Vec<PhysicalInput>,
    released_keys: Vec<PhysicalInput>,
    hotbar_scroll: i8,
    mouse_delta_x: f32,
    mouse_delta_y: f32,
    attack_pressed: bool,
    attack_held: bool,
    use_pressed: bool,
    use_released: bool,
    close_window_pressed: bool,
}

impl RuntimeInputState {
    fn keyboard_input(&mut self, input: PhysicalInput, pressed: bool) {
        if pressed {
            if self.held_keys.insert(input) {
                self.pressed_keys.push(input);
            }
        } else if self.held_keys.remove(&input) {
            self.released_keys.push(input);
        }
    }

    fn mouse_scroll(&mut self, delta: i8) {
        self.hotbar_scroll = self.hotbar_scroll.saturating_add(delta);
    }

    fn consume_frame(&mut self) -> (InputFrame, RuntimeActionInput) {
        let frame = InputFrame {
            pressed_inputs: std::mem::take(&mut self.pressed_keys),
            released_inputs: std::mem::take(&mut self.released_keys),
            mouse_delta_x: std::mem::take(&mut self.mouse_delta_x),
            mouse_delta_y: std::mem::take(&mut self.mouse_delta_y),
            hotbar_scroll: std::mem::take(&mut self.hotbar_scroll),
        };
        let actions = RuntimeActionInput {
            attack_pressed: std::mem::take(&mut self.attack_pressed),
            attack_held: self.attack_held,
            use_pressed: std::mem::take(&mut self.use_pressed),
            use_released: std::mem::take(&mut self.use_released),
            close_window_pressed: std::mem::take(&mut self.close_window_pressed),
        };
        (frame, actions)
    }
}

pub fn run_play_cli(raw_args: Vec<String>) -> Result<(), String> {
    let options = PlayCliOptions::parse(raw_args)?;
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("RustMinecraft")
        .with_inner_size(LogicalSize::new(
            f64::from(options.width),
            f64::from(options.height),
        ))
        .with_min_inner_size(LogicalSize::new(640.0, 360.0))
        .build(&event_loop)
        .map_err(|error| format!("failed to create window: {error}"))?;
    let window_size = window.inner_size();
    let surface = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(options.width, options.height, surface)
        .map_err(|error| format!("failed to create pixel surface: {error}"))?;
    let mut app = PlayApp::new(options);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    if pixels.resize_surface(size.width, size.height).is_err() {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    if pixels
                        .resize_surface(new_inner_size.width, new_inner_size.height)
                        .is_err()
                    {
                        *control_flow = ControlFlow::Exit;
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    app.mouse_position = position.cast::<f32>();
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    app.handle_mouse_input(button, state == ElementState::Pressed, &window);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    app.handle_mouse_wheel(delta);
                }
                WindowEvent::ReceivedCharacter(character) => {
                    app.handle_received_character(character);
                }
                WindowEvent::KeyboardInput { input, .. } => {
                    app.handle_keyboard_input(input, &window);
                }
                _ => {}
            },
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                app.handle_mouse_motion(delta);
            }
            Event::MainEventsCleared => {
                app.update(&window);
                if app.should_exit() {
                    if app.options.auto_play {
                        let healthy = app.runtime.as_ref().is_some_and(|runtime| runtime.summary().joined_game && !runtime.summary().ended_by_eof && runtime.summary().disconnect_reason_json.is_none());
                        if let Some(runtime) = &app.runtime {
                            println!("play_cli: smoke_joined={} loaded_chunks={} ticks={} imported_assets={}", runtime.summary().joined_game, runtime.loaded_chunk_count(), runtime.output().map_or(0, |output| output.total_ticks), app.assets.vanilla_root.is_some());
                        }
                        if !healthy { eprintln!("play_cli: smoke test failed: {}", app.status_line); }
                        *control_flow = ControlFlow::ExitWithCode(if healthy { 0 } else { 1 });
                    } else { *control_flow = ControlFlow::Exit; }
                    return;
                }
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                app.draw(pixels.frame_mut(), app.options.width, app.options.height);
                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });

    #[allow(unreachable_code)]
    Ok(())
}

struct PlayApp {
    options: PlayCliOptions,
    assets: GameAssets,
    screen: ScreenState,
    launcher_accounts_path: Option<PathBuf>,
    clipboard: Option<Clipboard>,
    accounts: Vec<AccountSource>,
    manual_accounts: Vec<AccountSource>,
    selected_account: usize,
    server_input: String,
    offline_username_input: String,
    auth_input: String,
    focus_field: FocusField,
    status_line: String,
    runtime: Option<LiveRuntime>,
    runtime_input: RuntimeInputState,
    mouse_position: PhysicalPosition<f32>,
    last_frame_at: Instant,
    launched_at: Instant,
    mouse_captured: bool,
    modifiers_ctrl: bool,
    modifiers_shift: bool,
    auto_exit_after: Option<Duration>,
    stop_after_join: bool,
    should_exit: bool,
    join_announced: bool,
    chat_open: bool,
    chat_input: String,
    chat_scroll: usize,
    show_tab_overlay: bool,
}

impl PlayApp {
    fn new(options: PlayCliOptions) -> Self {
        let auto_exit_after = options.duration_secs.map(Duration::from_secs);
        let stop_after_join = options.stop_after_join;
        let (assets, asset_notice) = GameAssets::load();
        let server_input = format!("{}:{}", options.server_host, options.server_port);
        let offline_username_input = options.offline_username.clone();
        let mut app = Self {
            options,
            assets,
            screen: ScreenState::Menu,
            launcher_accounts_path: default_launcher_accounts_path(),
            clipboard: Clipboard::new().ok(),
            accounts: Vec::new(),
            manual_accounts: Vec::new(),
            selected_account: 0,
            server_input,
            offline_username_input,
            auth_input: String::new(),
            focus_field: FocusField::None,
            status_line: asset_notice.unwrap_or_default(),
            runtime: None,
            runtime_input: RuntimeInputState {
                held_keys: BTreeSet::new(),
                pressed_keys: Vec::new(),
                released_keys: Vec::new(),
                hotbar_scroll: 0,
                mouse_delta_x: 0.0,
                mouse_delta_y: 0.0,
                attack_pressed: false,
                attack_held: false,
                use_pressed: false,
                use_released: false,
                close_window_pressed: false,
            },
            mouse_position: PhysicalPosition::new(0.0, 0.0),
            last_frame_at: Instant::now(),
            launched_at: Instant::now(),
            mouse_captured: false,
            modifiers_ctrl: false,
            modifiers_shift: false,
            auto_exit_after,
            stop_after_join,
            should_exit: false,
            join_announced: false,
            chat_open: false,
            chat_input: String::new(),
            chat_scroll: 0,
            show_tab_overlay: false,
        };
        app.reload_accounts();
        if let Some(account_name) = app.options.account_name.as_deref() {
            if let Some(index) = app
                .accounts
                .iter()
                .position(|account| account.username == account_name)
            {
                app.selected_account = index;
            } else {
                app.status_line = format!("Account not found: {account_name}");
                if app.options.auto_play {
                    app.should_exit = true;
                }
            }
        }
        if app.options.auto_play && !app.should_exit {
            app.start_play(app.options.force_offline || app.accounts.is_empty());
        }
        app
    }

    fn update(&mut self, window: &winit::window::Window) {
        let now = Instant::now();
        let frame_delta = now.saturating_duration_since(self.last_frame_at);
        self.last_frame_at = now;

        if self
            .auto_exit_after
            .is_some_and(|limit| now.saturating_duration_since(self.launched_at) >= limit)
        {
            self.should_exit = true;
        }

        if let Some(runtime) = &mut self.runtime {
            let (mut frame_input, mut actions) = self.runtime_input.consume_frame();
            if self.chat_open
                || !self.mouse_captured
                || runtime.combat_snapshot().health <= 0.0
                || runtime
                    .usability_snapshot()
                    .is_some_and(|snapshot| snapshot.window.is_some())
            {
                frame_input.pressed_inputs.clear();
                frame_input
                    .released_inputs
                    .extend(std::mem::take(&mut self.runtime_input.held_keys));
                frame_input.mouse_delta_x = 0.0;
                frame_input.mouse_delta_y = 0.0;
                frame_input.hotbar_scroll = 0;
                self.runtime_input.attack_held = false;
                actions.attack_pressed = false;
                actions.attack_held = false;
                actions.use_pressed = false;
            }
            if let Err(error) = runtime.step(frame_delta, &frame_input, &actions) {
                self.status_line = error;
                self.runtime = None;
                self.screen = ScreenState::Menu;
                self.apply_cursor_capture(window, false);
                if self.options.auto_play {
                    self.should_exit = true;
                }
                return;
            }

            if let Some(reason) = runtime.summary().disconnect_reason_json.as_ref() {
                self.status_line = format!("Disconnected: {reason}");
                println!("play_cli: disconnected {reason}");
                if self.options.auto_play {
                    self.should_exit = true;
                }
            }

            if runtime.summary().joined_game && !self.join_announced {
                self.join_announced = true;
                println!(
                    "play_cli: joined_game=true reached_play={} compression_enabled={} encryption_enabled={}",
                    runtime.summary().reached_play,
                    runtime.summary().compression_enabled,
                    runtime.summary().encryption_enabled,
                );
                if self.stop_after_join {
                    self.should_exit = true;
                }
            }

            if runtime.summary().ended_by_eof {
                println!("play_cli: connection closed by eof");
                if self.options.auto_play {
                    self.should_exit = true;
                }
            }

            let captured = runtime
                .output()
                .map(|output| output.render.camera.mouse_captured)
                .unwrap_or(false);
            let ui_blocking = runtime.combat_snapshot().health <= 0.0
                || self.chat_open
                || runtime
                    .usability_snapshot()
                    .and_then(|snapshot| snapshot.window.as_ref())
                    .is_some();
            self.apply_cursor_capture(window, captured && !ui_blocking);
        } else {
            self.runtime_input.consume_frame();
        }
    }

    fn draw(&self, frame: &mut [u8], width: u32, height: u32) {
        match self.screen {
            ScreenState::Menu => self.draw_menu(frame, width, height),
            ScreenState::Playing => self.draw_play(frame, width, height),
        }
    }

    fn draw_menu(&self, frame: &mut [u8], width: u32, height: u32) {
        fill_gradient(frame, width, height, [16, 22, 38], [60, 32, 24]);
        let panel = UiRect {
            x: 34,
            y: 28,
            width: width as i32 - 68,
            height: height as i32 - 56,
        };
        draw_panel(frame, width, height, panel, [18, 20, 29], [255, 120, 54]);
        draw_text_scaled(
            frame,
            width,
            height,
            56,
            46,
            "RustMinecraft",
            [255, 245, 235],
            2,
        );
        draw_text_scaled(
            frame,
            width,
            height,
            56,
            74,
            "Windowed PvP client bring-up",
            [255, 170, 120],
            1,
        );

        let account_panel = UiRect {
            x: 56,
            y: 108,
            width: (width as i32 / 2) - 80,
            height: height as i32 - 180,
        };
        let auth_panel = UiRect {
            x: (width as i32 / 2) + 8,
            y: 108,
            width: (width as i32 / 2) - 64,
            height: height as i32 - 180,
        };
        draw_panel(
            frame,
            width,
            height,
            account_panel,
            [24, 29, 42],
            [93, 172, 255],
        );
        draw_panel(
            frame,
            width,
            height,
            auth_panel,
            [28, 23, 34],
            [255, 98, 81],
        );
        draw_text_scaled(
            frame,
            width,
            height,
            account_panel.x + 16,
            account_panel.y + 14,
            "Accounts",
            [233, 242, 255],
            1,
        );
        draw_text_scaled(
            frame,
            width,
            height,
            auth_panel.x + 16,
            auth_panel.y + 14,
            "Connect / Login",
            [255, 233, 228],
            1,
        );

        let mut row_y = account_panel.y + 40;
        for (index, account) in self.accounts.iter().enumerate().take(8) {
            let row = UiRect {
                x: account_panel.x + 12,
                y: row_y,
                width: account_panel.width - 24,
                height: 28,
            };
            let selected = index == self.selected_account;
            draw_button(
                frame,
                width,
                height,
                row,
                if selected {
                    [57, 111, 170]
                } else {
                    [38, 46, 63]
                },
                if selected {
                    [181, 228, 255]
                } else {
                    [108, 166, 230]
                },
            );
            let kind_label = match account.kind {
                AccountSourceKind::LauncherAccessToken => "launcher",
                AccountSourceKind::RefreshToken => "refresh",
            };
            draw_text_scaled(
                frame,
                width,
                height,
                row.x + 10,
                row.y + 6,
                &format!("{} [{}]", account.username, kind_label),
                [245, 248, 255],
                1,
            );
            row_y += 34;
        }

        if self.accounts.is_empty() {
            draw_text_scaled(
                frame,
                width,
                height,
                account_panel.x + 16,
                account_panel.y + 50,
                "No usable accounts loaded",
                [180, 190, 205],
                1,
            );
        }

        let server_input = self.server_input_rect(width as i32, height as i32);
        let offline_input = self.offline_username_rect(width as i32, height as i32);
        let reload = self.reload_button_rect(width as i32, height as i32);
        let play_online = self.play_online_button_rect(width as i32, height as i32);
        let play_offline = self.play_offline_button_rect(width as i32, height as i32);
        let browser = self.browser_button_rect(width as i32, height as i32);
        let paste = self.paste_button_rect(width as i32, height as i32);
        let signin = self.signin_button_rect(width as i32, height as i32);
        let input = self.auth_input_rect(width as i32, height as i32);

        draw_text_scaled(
            frame,
            width,
            height,
            auth_panel.x + 16,
            auth_panel.y + 42,
            "Server",
            [255, 221, 210],
            1,
        );
        draw_input(
            frame,
            width,
            height,
            server_input,
            self.focus_field == FocusField::ServerInput,
            &self.server_input,
            "localhost:25565",
        );
        draw_text_scaled(
            frame,
            width,
            height,
            auth_panel.x + 16,
            auth_panel.y + 94,
            "Offline Username",
            [255, 221, 210],
            1,
        );
        draw_input(
            frame,
            width,
            height,
            offline_input,
            self.focus_field == FocusField::OfflineUsernameInput,
            &self.offline_username_input,
            "Player",
        );

        draw_button(frame, width, height, reload, [41, 65, 83], [122, 227, 255]);
        draw_button(
            frame,
            width,
            height,
            play_offline,
            [39, 66, 40],
            [150, 255, 155],
        );
        draw_button(
            frame,
            width,
            height,
            play_online,
            [86, 43, 33],
            [255, 175, 98],
        );
        draw_text_scaled(
            frame,
            width,
            height,
            reload.x + 14,
            reload.y + 9,
            "Reload",
            [235, 250, 255],
            1,
        );
        draw_text_scaled(
            frame,
            width,
            height,
            play_offline.x + 9,
            play_offline.y + 9,
            "Play Offline",
            [238, 255, 238],
            1,
        );
        draw_text_scaled(
            frame,
            width,
            height,
            play_online.x + 11,
            play_online.y + 9,
            "Play Online",
            [255, 244, 236],
            1,
        );

        draw_text_scaled(
            frame,
            width,
            height,
            auth_panel.x + 16,
            auth_panel.y + 182,
            "Microsoft Login",
            [255, 221, 210],
            1,
        );
        draw_button(frame, width, height, browser, [70, 37, 50], [255, 111, 160]);
        draw_button(frame, width, height, paste, [40, 38, 55], [155, 161, 255]);
        draw_button(frame, width, height, signin, [48, 59, 32], [160, 255, 128]);
        draw_input(
            frame,
            width,
            height,
            input,
            self.focus_field == FocusField::AuthInput,
            &self.auth_input,
            "code / callback URL / refresh token",
        );
        draw_text_scaled(
            frame,
            width,
            height,
            browser.x + 10,
            browser.y + 9,
            "Open Login",
            [255, 239, 246],
            1,
        );
        draw_text_scaled(
            frame,
            width,
            height,
            paste.x + 20,
            paste.y + 9,
            "Paste",
            [236, 239, 255],
            1,
        );
        draw_text_scaled(
            frame,
            width,
            height,
            signin.x + 18,
            signin.y + 9,
            "Sign In",
            [244, 255, 240],
            1,
        );

        draw_text_box(
            frame,
            width,
            height,
            auth_panel.x + 16,
            auth_panel.y + 304,
            auth_panel.width - 32,
            &[
                "Play Offline is the fastest path for localhost or a private server.",
                "Play Online uses the selected launcher or refresh-token account.",
                "Paste the callback URL or raw code, then Sign In if needed.",
            ],
            [205, 212, 224],
        );

        if !self.status_line.is_empty() {
            draw_text_box(
                frame,
                width,
                height,
                56,
                height as i32 - 54,
                width as i32 - 112,
                &[self.status_line.as_str()],
                [255, 222, 198],
            );
        }
    }

    fn draw_play(&self, frame: &mut [u8], width: u32, height: u32) {
        fill_gradient(frame, width, height, [92, 132, 182], [21, 24, 31]);

        if let Some(runtime) = &self.runtime {
            if let (Some(output), Some(world_render)) = (runtime.output(), runtime.world_render()) {
                let daylight = runtime.daylight(output.render.interpolation_alpha);
                if let Some(light) = daylight {
                    let scale = |color: [u8; 3]| {
                        color.map(|channel| (f32::from(channel) * light.sky_color_multiplier) as u8)
                    };
                    fill_gradient(
                        frame,
                        width,
                        height,
                        scale([92, 132, 182]),
                        scale([21, 24, 31]),
                    );
                }
                let mut depth = vec![f32::INFINITY; (width * height) as usize];
                render_world_meshes(
                    frame,
                    &mut depth,
                    width,
                    height,
                    output.render.camera.position,
                    output.render.camera.yaw,
                    output.render.camera.pitch,
                    world_render,
                    &self.assets,
                    daylight.map_or(0, |d| d.skylight_subtracted),
                );
                render_tracked_players(
                    frame,
                    &mut depth,
                    width,
                    height,
                    output.render.camera.position,
                    output.render.camera.yaw,
                    output.render.camera.pitch,
                    runtime.entity_tracker(),
                    runtime.targeted_entity(),
                    runtime.is_spectator(),
                    |name| runtime.can_see_friendly_invisible(name),
                );
                draw_border_warning(
                    frame,
                    width,
                    height,
                    runtime.border_warning_strength(),
                    self.assets.vignette.as_ref(),
                );
                if let Some(icons) = self.assets.icons.as_ref() {
                    draw_sprite_region(
                        frame,
                        width,
                        height,
                        icons,
                        UiRect {
                            x: width as i32 / 2 - 7,
                            y: height as i32 / 2 - 7,
                            width: 16,
                            height: 16,
                        },
                        UiRect {
                            x: 0,
                            y: 0,
                            width: 16,
                            height: 16,
                        },
                        [255, 255, 255],
                        1.0,
                    );
                } else {
                    draw_crosshair(frame, width, height, [255, 245, 233]);
                }
                draw_hotbar_overlay(frame, width, height, runtime, &self.assets);
                if self.show_tab_overlay {
                    draw_tab_overlay(frame, width, height, runtime);
                }
                if let Some(snapshot) = runtime.usability_snapshot() {
                    if runtime.is_survival_or_adventure() {
                        draw_experience_overlay(
                            frame,
                            width,
                            height,
                            snapshot.experience,
                            &self.assets,
                        );
                    }

                    if let Some(window) = snapshot.window.as_ref() {
                        draw_window_overlay(frame, width, height, window);
                        if let Some(item) = window.carried_item.as_ref() {
                            draw_text_scaled(
                                frame,
                                width,
                                height,
                                self.mouse_position.x as i32 + 8,
                                self.mouse_position.y as i32 + 8,
                                &format!("{} x{}", item.item_id, item.count),
                                [255, 255, 255],
                                1,
                            );
                        }
                    }
                }
                if self.chat_open {
                    if let Some(snapshot) = runtime.usability_snapshot() {
                        draw_chat_history(
                            frame,
                            width,
                            height,
                            &snapshot.chat_lines,
                            self.chat_scroll,
                        );
                    }
                    draw_chat_input_overlay(frame, width, height, &self.chat_input);
                }
                draw_runtime_overlay(frame, width, height, runtime);
                let combat = runtime.combat_snapshot();
                if combat.health <= 0.0 {
                    for pixel in frame.chunks_exact_mut(4) {
                        pixel[0] = (pixel[0] as u16 / 2 + 64).min(255) as u8;
                        pixel[1] /= 2;
                        pixel[2] /= 2;
                    }
                    draw_text_scaled(
                        frame,
                        width,
                        height,
                        width as i32 / 2 - 72,
                        height as i32 / 4,
                        "You died!",
                        [255, 255, 255],
                        2,
                    );
                    if !combat.hardcore {
                        let ready = combat.death_ticks >= 20 && !combat.respawn_requested;
                        let rect = death_button_rect(width as i32, height as i32, 0);
                        draw_button(
                            frame,
                            width,
                            height,
                            rect,
                            if ready { [58, 64, 72] } else { [32, 32, 32] },
                            [150, 150, 150],
                        );
                        draw_text_scaled(
                            frame,
                            width,
                            height,
                            rect.x + 12,
                            rect.y + 8,
                            if combat.respawn_requested {
                                "Waiting for server..."
                            } else {
                                "Respawn [Enter]"
                            },
                            if ready {
                                [255, 255, 255]
                            } else {
                                [140, 140, 140]
                            },
                            1,
                        );
                    }
                    let rect = death_button_rect(width as i32, height as i32, 1);
                    draw_button(frame, width, height, rect, [58, 64, 72], [150, 150, 150]);
                    draw_text_scaled(
                        frame,
                        width,
                        height,
                        rect.x + 12,
                        rect.y + 8,
                        "Leave server [Esc]",
                        [255, 255, 255],
                        1,
                    );
                    if !self.status_line.is_empty() {
                        draw_text_scaled(
                            frame,
                            width,
                            height,
                            12,
                            height as i32 - 24,
                            &self.status_line,
                            [255, 210, 180],
                            1,
                        );
                    }
                }
                return;
            }
        }

        draw_text_scaled(
            frame,
            width,
            height,
            24,
            24,
            "Connecting...",
            [255, 244, 236],
            2,
        );
        if !self.status_line.is_empty() {
            draw_text_box(
                frame,
                width,
                height,
                24,
                72,
                width as i32 - 48,
                &[self.status_line.as_str()],
                [255, 222, 198],
            );
        }
    }

    fn handle_mouse_input(
        &mut self,
        button: MouseButton,
        pressed: bool,
        window: &winit::window::Window,
    ) {
        match self.screen {
            ScreenState::Menu => {
                if pressed && button == MouseButton::Left {
                    self.handle_menu_click();
                }
            }
            ScreenState::Playing => {
                if self
                    .runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.combat_snapshot().health <= 0.0)
                {
                    if pressed && button == MouseButton::Left {
                        let width = self.options.width as i32;
                        let height = self.options.height as i32;
                        if death_button_rect(width, height, 0)
                            .contains(self.mouse_position.x, self.mouse_position.y)
                        {
                            if let Some(runtime) = self.runtime.as_mut() {
                                if let Err(error) = runtime.request_respawn() {
                                    self.status_line = error;
                                }
                            }
                        } else if death_button_rect(width, height, 1)
                            .contains(self.mouse_position.x, self.mouse_position.y)
                        {
                            self.runtime = None;
                            self.screen = ScreenState::Menu;
                            self.apply_cursor_capture(window, false);
                        }
                    }
                    return;
                }
                if pressed
                    && matches!(
                        button,
                        MouseButton::Left | MouseButton::Right | MouseButton::Middle
                    )
                    && !self.mouse_captured
                {
                    if let Some((window_id, slot_id)) =
                        self.window_slot_at(self.mouse_position.x, self.mouse_position.y)
                    {
                        if let Some(runtime) = &mut self.runtime {
                            let button_id = if button == MouseButton::Right { 1 } else { 0 };
                            let result = if button == MouseButton::Middle {
                                if slot_id >= 0 {
                                    runtime.clone_window_slot(window_id, slot_id)
                                } else {
                                    Ok(())
                                }
                            } else if self.modifiers_shift && slot_id >= 0 {
                                runtime.transfer_window_slot(window_id, slot_id, button_id)
                            } else {
                                runtime.click_window_slot(window_id, slot_id, button_id)
                            };
                            if let Err(error) = result {
                                self.status_line = error;
                            }
                        }
                        return;
                    }
                    if self.window_is_open() {
                        return;
                    }
                }

                match button {
                    MouseButton::Left => {
                        self.runtime_input.attack_held = pressed;
                        if pressed {
                            self.runtime_input.attack_pressed = true;
                        }
                    }
                    MouseButton::Right => {
                        if pressed {
                            self.runtime_input.use_pressed = true;
                        } else {
                            self.runtime_input.use_released = true;
                        }
                    }
                    MouseButton::Middle => {
                        if pressed {
                            self.runtime_input.close_window_pressed = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if self.screen == ScreenState::Playing
            && pressed
            && !self.mouse_captured
            && !self.chat_open
            && self
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.usability_snapshot())
                .and_then(|snapshot| snapshot.window.as_ref())
                .is_none()
        {
            self.apply_cursor_capture(window, true);
        }
    }

    fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.screen == ScreenState::Playing && self.chat_open {
            let y = match delta {
                MouseScrollDelta::LineDelta(_, y) => y,
                MouseScrollDelta::PixelDelta(p) => p.y as f32,
            };
            let count = self
                .runtime
                .as_ref()
                .and_then(|r| r.usability_snapshot())
                .map_or(0, |s| s.chat_lines.len());
            let step = if self.modifiers_shift { 1 } else { 7 };
            if y.is_finite() && y != 0.0 {
                self.chat_scroll = (self.chat_scroll as i64 + i64::from(y.signum() as i8) * step)
                    .clamp(0, count.saturating_sub(8) as i64)
                    as usize;
            }
            return;
        }
        if self.screen != ScreenState::Playing
            || self.chat_open
            || !self.mouse_captured
            || self.runtime.as_ref().is_some_and(|runtime| {
                runtime.combat_snapshot().health <= 0.0
                    || runtime
                        .usability_snapshot()
                        .is_some_and(|snapshot| snapshot.window.is_some())
            })
        {
            return;
        }

        let delta = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(position) => (position.y as f32 / 24.0).clamp(-3.0, 3.0),
        };

        if delta.abs() >= f32::EPSILON {
            if let Some(runtime) = self.runtime.as_mut() {
                if runtime.is_spectator() {
                    runtime.adjust_spectator_fly_speed(delta.signum() as i8);
                    return;
                }
            }
            self.runtime_input.mouse_scroll(delta.signum() as i8);
        }
    }

    fn handle_received_character(&mut self, character: char) {
        if character.is_control() {
            return;
        }

        match self.screen {
            ScreenState::Menu => match self.focus_field {
                FocusField::ServerInput => self.server_input.push(character),
                FocusField::OfflineUsernameInput => self.offline_username_input.push(character),
                FocusField::AuthInput => self.auth_input.push(character),
                FocusField::None => {}
            },
            ScreenState::Playing => {
                if self.chat_open {
                    self.chat_input.push(character);
                }
            }
        }
    }

    fn handle_keyboard_input(&mut self, input: KeyboardInput, window: &winit::window::Window) {
        if let Some(key) = input.virtual_keycode {
            let pressed = input.state == ElementState::Pressed;

            if key == VirtualKeyCode::LShift || key == VirtualKeyCode::RShift {
                self.modifiers_shift = pressed;
            }

            if key == VirtualKeyCode::LControl || key == VirtualKeyCode::RControl {
                self.modifiers_ctrl = pressed;
            }

            match self.screen {
                ScreenState::Menu => self.handle_menu_key(key, pressed),
                ScreenState::Playing => self.handle_play_key(key, pressed, window),
            }
        }
    }

    fn handle_menu_key(&mut self, key: VirtualKeyCode, pressed: bool) {
        if !pressed {
            return;
        }

        match key {
            VirtualKeyCode::Tab => {
                self.focus_field = match self.focus_field {
                    FocusField::None => FocusField::ServerInput,
                    FocusField::ServerInput => FocusField::OfflineUsernameInput,
                    FocusField::OfflineUsernameInput => FocusField::AuthInput,
                    FocusField::AuthInput => FocusField::None,
                };
            }
            VirtualKeyCode::Back => match self.focus_field {
                FocusField::ServerInput => {
                    self.server_input.pop();
                }
                FocusField::OfflineUsernameInput => {
                    self.offline_username_input.pop();
                }
                FocusField::AuthInput => {
                    self.auth_input.pop();
                }
                FocusField::None => {}
            },
            VirtualKeyCode::Return => {
                if self.focus_field == FocusField::AuthInput && !self.auth_input.trim().is_empty() {
                    self.sign_in_from_input();
                } else {
                    self.start_play(self.accounts.is_empty());
                }
            }
            VirtualKeyCode::Up => {
                if self.selected_account > 0 {
                    self.selected_account -= 1;
                }
            }
            VirtualKeyCode::Down => {
                if self.selected_account + 1 < self.accounts.len() {
                    self.selected_account += 1;
                }
            }
            VirtualKeyCode::V if self.modifiers_ctrl => self.paste_clipboard(),
            VirtualKeyCode::F5 => self.reload_accounts(),
            _ => {}
        }
    }

    fn handle_play_key(
        &mut self,
        key: VirtualKeyCode,
        pressed: bool,
        window: &winit::window::Window,
    ) {
        if self
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.combat_snapshot().health <= 0.0)
        {
            if pressed && key == VirtualKeyCode::Return {
                if let Some(runtime) = self.runtime.as_mut() {
                    if let Err(error) = runtime.request_respawn() {
                        self.status_line = error;
                    }
                }
            } else if pressed && key == VirtualKeyCode::Escape {
                self.runtime = None;
                self.screen = ScreenState::Menu;
                self.apply_cursor_capture(window, false);
            }
            return;
        }
        if self.chat_open {
            if !pressed {
                return;
            }

            match key {
                VirtualKeyCode::Escape => {
                    self.chat_open = false;
                    self.chat_input.clear();
                }
                VirtualKeyCode::Back => {
                    self.chat_input.pop();
                }
                VirtualKeyCode::Return => {
                    self.send_chat_input();
                }
                VirtualKeyCode::V if self.modifiers_ctrl => self.paste_chat_from_clipboard(),
                _ => {}
            }
            return;
        }

        if key == VirtualKeyCode::Tab {
            self.show_tab_overlay = pressed;
            return;
        }

        if pressed {
            if key == VirtualKeyCode::T {
                self.chat_open = true;
                self.chat_scroll = 0;
                self.chat_input.clear();
                self.apply_cursor_capture(window, false);
                return;
            }
            if key == VirtualKeyCode::Slash {
                self.chat_open = true;
                self.chat_scroll = 0;
                self.chat_input = "/".to_owned();
                self.apply_cursor_capture(window, false);
                return;
            }
            if matches!(key, VirtualKeyCode::E | VirtualKeyCode::Escape) && self.window_is_open() {
                if let Some(runtime) = &mut self.runtime {
                    if let Err(error) = runtime.close_open_window() {
                        self.status_line = error;
                    }
                }
                self.apply_cursor_capture(window, true);
                return;
            }
            if key == VirtualKeyCode::E {
                if let Some(runtime) = &mut self.runtime {
                    if let Err(error) = runtime.open_player_inventory() {
                        self.status_line = error;
                    }
                }
                self.apply_cursor_capture(window, false);
                return;
            }
            if key == VirtualKeyCode::Escape && self.mouse_captured {
                self.apply_cursor_capture(window, false);
                return;
            }
            if key == VirtualKeyCode::Escape && !self.mouse_captured {
                self.screen = ScreenState::Menu;
                self.runtime = None;
                self.apply_cursor_capture(window, false);
                self.chat_open = false;
                self.show_tab_overlay = false;
                return;
            }
        }

        if self.window_is_open() {
            if pressed {
                if key == VirtualKeyCode::Q {
                    if let Some((window_id, slot_id)) =
                        self.window_slot_at(self.mouse_position.x, self.mouse_position.y)
                    {
                        if slot_id >= 0 {
                            if let Some(runtime) = &mut self.runtime {
                                if let Err(error) = runtime.throw_window_slot(
                                    window_id,
                                    slot_id,
                                    self.modifiers_ctrl,
                                ) {
                                    self.status_line = error;
                                }
                            }
                        }
                    }
                    return;
                }
                let hotbar = match key {
                    VirtualKeyCode::Key1 => Some(0),
                    VirtualKeyCode::Key2 => Some(1),
                    VirtualKeyCode::Key3 => Some(2),
                    VirtualKeyCode::Key4 => Some(3),
                    VirtualKeyCode::Key5 => Some(4),
                    VirtualKeyCode::Key6 => Some(5),
                    VirtualKeyCode::Key7 => Some(6),
                    VirtualKeyCode::Key8 => Some(7),
                    VirtualKeyCode::Key9 => Some(8),
                    _ => None,
                };
                if let (Some(hotbar), Some((window_id, slot_id))) = (
                    hotbar,
                    self.window_slot_at(self.mouse_position.x, self.mouse_position.y),
                ) {
                    if slot_id >= 0 {
                        if let Some(runtime) = &mut self.runtime {
                            if let Err(error) =
                                runtime.swap_window_slot_with_hotbar(window_id, slot_id, hotbar)
                            {
                                self.status_line = error;
                            }
                        }
                    }
                }
            }
            return;
        }
        if let Some(input) = map_virtual_key(key) {
            self.runtime_input.keyboard_input(input, pressed);
        }
    }

    fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        if self.screen == ScreenState::Playing && self.mouse_captured {
            self.runtime_input.mouse_delta_x += delta.0 as f32;
            self.runtime_input.mouse_delta_y += delta.1 as f32;
        }
    }

    fn handle_menu_click(&mut self) {
        let width = self.options.width as i32;
        let height = self.options.height as i32;
        let mouse_x = self.mouse_position.x;
        let mouse_y = self.mouse_position.y;

        if self
            .reload_button_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.reload_accounts();
            return;
        }
        if self
            .play_online_button_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.start_play(false);
            return;
        }
        if self
            .play_offline_button_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.start_play(true);
            return;
        }
        if self
            .browser_button_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.open_browser_login();
            return;
        }
        if self
            .paste_button_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.paste_clipboard();
            return;
        }
        if self
            .signin_button_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.sign_in_from_input();
            return;
        }
        if self
            .server_input_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.focus_field = FocusField::ServerInput;
            return;
        }
        if self
            .offline_username_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.focus_field = FocusField::OfflineUsernameInput;
            return;
        }
        if self
            .auth_input_rect(width, height)
            .contains(mouse_x, mouse_y)
        {
            self.focus_field = FocusField::AuthInput;
            return;
        }

        let account_panel = UiRect {
            x: 56,
            y: 108,
            width: (width / 2) - 80,
            height: height - 180,
        };
        let mut row_y = account_panel.y + 40;
        for index in 0..self.accounts.len().min(8) {
            let row = UiRect {
                x: account_panel.x + 12,
                y: row_y,
                width: account_panel.width - 24,
                height: 28,
            };
            if row.contains(mouse_x, mouse_y) {
                self.selected_account = index;
                return;
            }
            row_y += 34;
        }

        self.focus_field = FocusField::None;
    }

    fn reload_accounts(&mut self) {
        let mut sources = Vec::new();

        if let Some(path) = self
            .launcher_accounts_path
            .as_ref()
            .filter(|path| path.exists())
        {
            match load_account_sources_from_path(path) {
                Ok(mut loaded) => sources.append(&mut loaded),
                Err(error) => {
                    self.status_line = format!("Failed to load launcher accounts: {error:?}");
                }
            }
        }

        if let Some(path) = self
            .options
            .accounts_path
            .as_ref()
            .filter(|path| path.exists())
        {
            match load_account_sources_from_path(path) {
                Ok(mut loaded) => sources.append(&mut loaded),
                Err(error) => {
                    self.status_line = format!("Failed to load custom accounts: {error:?}");
                }
            }
        }

        sources.extend(self.manual_accounts.iter().cloned());
        self.accounts = dedup_accounts(sources);
        if self.selected_account >= self.accounts.len() {
            self.selected_account = self.accounts.len().saturating_sub(1);
        }
        if self.status_line.is_empty() {
            self.status_line = if self.accounts.is_empty() {
                "Loaded 0 online accounts. Play Offline is ready.".to_owned()
            } else {
                format!("Loaded {} account sources", self.accounts.len())
            };
        }
    }

    fn open_browser_login(&mut self) {
        match webbrowser::open(&microsoft_login_url()) {
            Ok(_) => {
                self.status_line =
                    "Browser login opened. Paste the callback URL or code, then Sign In."
                        .to_owned();
            }
            Err(error) => {
                self.status_line = format!("Failed to open browser: {error}");
            }
        }
    }

    fn paste_clipboard(&mut self) {
        match self
            .clipboard
            .as_mut()
            .and_then(|clipboard| clipboard.get_text().ok())
        {
            Some(text) => match self.focus_field {
                FocusField::ServerInput => {
                    self.server_input = text;
                    self.status_line = "Pasted clipboard into server field".to_owned();
                }
                FocusField::OfflineUsernameInput => {
                    self.offline_username_input = text;
                    self.status_line = "Pasted clipboard into offline username field".to_owned();
                }
                FocusField::AuthInput | FocusField::None => {
                    self.auth_input = text;
                    self.focus_field = FocusField::AuthInput;
                    self.status_line = "Pasted clipboard into login field".to_owned();
                }
            },
            None => {
                self.status_line = "Clipboard is unavailable".to_owned();
            }
        }
    }

    fn sign_in_from_input(&mut self) {
        let input = self.auth_input.trim();
        if input.is_empty() {
            self.status_line = "Login input is empty".to_owned();
            return;
        }

        let session = if input.starts_with("M.") {
            authenticate_with_refresh_token(input)
        } else {
            authenticate_with_authorization_input(input)
        };

        match session {
            Ok(session) => {
                self.store_manual_session(session);
                self.status_line = format!(
                    "Authenticated as {}. Press Play to connect.",
                    self.manual_accounts
                        .last()
                        .map(|account| account.username.as_str())
                        .unwrap_or("player")
                );
                self.auth_input.clear();
                self.reload_accounts();
            }
            Err(error) => {
                self.status_line = format!("Login failed: {error:?}");
            }
        }
    }

    fn start_play(&mut self, offline: bool) {
        let server = self.server_input.trim();
        if server.is_empty() {
            self.status_line = "Server field is empty".to_owned();
            return;
        }
        let (server_host, server_port) = match parse_server_address(server) {
            Ok(value) => value,
            Err(error) => {
                self.status_line = error;
                return;
            }
        };

        if !self.options.unsafe_hypixel {
            match hypixel_gate_decision(&server_host) {
                GateDecision::Allowed => {}
                GateDecision::Blocked(reason) => {
                    self.status_line = reason;
                    return;
                }
            }
        }
        let mut config = if offline {
            let username = self.offline_username_input.trim();
            if username.is_empty() {
                self.status_line = "Offline username is empty".to_owned();
                return;
            }
            LiveRuntimeConfig::offline(username)
        } else {
            let selected = self.accounts.get(self.selected_account).cloned();
            let account = match selected {
                Some(source) => match self.resolve_account_source(source) {
                    Ok(account) => account,
                    Err(error) => {
                        self.status_line = error;
                        return;
                    }
                },
                None => {
                    self.status_line =
                        "No online account selected. Use Play Offline or sign in first.".to_owned();
                    return;
                }
            };
            LiveRuntimeConfig::hypixel(account)
        };
        config.server_host = server_host;
        config.server_port = server_port;
        println!(
            "play_cli: connecting to {}:{} as {} mode={}",
            config.server_host,
            config.server_port,
            config.username(),
            if offline { "offline" } else { "online" },
        );

        match LiveRuntime::connect(config) {
            Ok(runtime) => {
                self.status_line.clear();
                self.runtime = Some(runtime);
                self.screen = ScreenState::Playing;
                self.join_announced = false;
                self.chat_open = false;
                self.chat_input.clear();
                self.show_tab_overlay = false;
            }
            Err(error) => {
                self.status_line = error;
                if self.options.auto_play {
                    self.should_exit = true;
                }
            }
        }
    }

    fn resolve_account_source(&mut self, source: AccountSource) -> Result<OnlineAccount, String> {
        if let Some(refresh_token) = source.refresh_token.as_deref() {
            let session = authenticate_with_refresh_token(refresh_token)
                .map_err(|error| format!("refresh-token login failed: {error:?}"))?;
            let account = session.account.clone();
            self.store_manual_session(session);
            self.reload_accounts();
            return Ok(account);
        }

        source
            .online_account()
            .ok_or_else(|| "selected account has no usable access token".to_owned())
    }

    fn store_manual_session(&mut self, session: MicrosoftSession) {
        let source = AccountSource {
            source_label: "manual".to_owned(),
            username: session.account.username.clone(),
            profile_id: Some(session.account.profile_id.clone()),
            access_token: Some(session.account.access_token.clone()),
            refresh_token: Some(session.refresh_token),
            kind: AccountSourceKind::RefreshToken,
        };

        self.manual_accounts
            .retain(|entry| entry.profile_id != source.profile_id);
        self.manual_accounts.push(source);
        self.selected_account = self.manual_accounts.len().saturating_sub(1);
    }

    fn apply_cursor_capture(&mut self, window: &winit::window::Window, captured: bool) {
        if self.mouse_captured == captured {
            return;
        }

        self.mouse_captured = captured;
        let _ = window.set_cursor_grab(if captured {
            CursorGrabMode::Confined
        } else {
            CursorGrabMode::None
        });
        window.set_cursor_visible(!captured);
    }

    fn reload_button_rect(&self, width: i32, height: i32) -> UiRect {
        UiRect {
            x: width - 364,
            y: height - 104,
            width: 92,
            height: 34,
        }
    }

    fn play_offline_button_rect(&self, width: i32, height: i32) -> UiRect {
        UiRect {
            x: width - 260,
            y: height - 104,
            width: 92,
            height: 34,
        }
    }

    fn play_online_button_rect(&self, width: i32, height: i32) -> UiRect {
        UiRect {
            x: width - 156,
            y: height - 104,
            width: 92,
            height: 34,
        }
    }

    fn server_input_rect(&self, width: i32, _height: i32) -> UiRect {
        UiRect {
            x: (width / 2) + 24,
            y: 170,
            width: (width / 2) - 96,
            height: 36,
        }
    }

    fn offline_username_rect(&self, width: i32, _height: i32) -> UiRect {
        UiRect {
            x: (width / 2) + 24,
            y: 222,
            width: (width / 2) - 96,
            height: 36,
        }
    }

    fn auth_input_rect(&self, width: i32, _height: i32) -> UiRect {
        UiRect {
            x: (width / 2) + 24,
            y: 310,
            width: (width / 2) - 96,
            height: 36,
        }
    }

    fn browser_button_rect(&self, width: i32, _height: i32) -> UiRect {
        UiRect {
            x: (width / 2) + 24,
            y: 362,
            width: 124,
            height: 34,
        }
    }

    fn paste_button_rect(&self, width: i32, _height: i32) -> UiRect {
        UiRect {
            x: (width / 2) + 162,
            y: 362,
            width: 90,
            height: 34,
        }
    }

    fn signin_button_rect(&self, width: i32, _height: i32) -> UiRect {
        UiRect {
            x: (width / 2) + 266,
            y: 362,
            width: 108,
            height: 34,
        }
    }

    fn window_is_open(&self) -> bool {
        self.runtime
            .as_ref()
            .and_then(|runtime| runtime.usability_snapshot())
            .and_then(|snapshot| snapshot.window.as_ref())
            .is_some()
    }

    fn window_slot_at(&self, mouse_x: f32, mouse_y: f32) -> Option<(u8, i16)> {
        let window = self
            .runtime
            .as_ref()
            .and_then(|runtime| runtime.usability_snapshot())
            .and_then(|snapshot| snapshot.window.as_ref())?;
        let layout = layout_window_snapshot(
            window,
            self.options.width as i32,
            self.options.height as i32,
        );

        let outside = !layout.panel.contains(mouse_x, mouse_y);
        layout
            .slots
            .into_iter()
            .find(|(rect, _)| rect.contains(mouse_x, mouse_y))
            .map(|(_, slot_id)| (window.window_id, slot_id))
            .or_else(|| outside.then_some((window.window_id, -999)))
    }

    fn send_chat_input(&mut self) {
        let message = self.chat_input.trim();
        if message.is_empty() {
            self.chat_open = false;
            self.chat_input.clear();
            return;
        }

        if let Some(runtime) = &mut self.runtime {
            if let Err(error) = runtime.send_chat_message(message) {
                self.status_line = error;
                return;
            }
        }

        self.chat_open = false;
        self.chat_input.clear();
    }

    fn paste_chat_from_clipboard(&mut self) {
        match self
            .clipboard
            .as_mut()
            .and_then(|clipboard| clipboard.get_text().ok())
        {
            Some(text) => self.chat_input.push_str(&text),
            None => self.status_line = "Clipboard is unavailable".to_owned(),
        }
    }

    fn should_exit(&self) -> bool {
        self.should_exit
    }
}

fn default_launcher_accounts_path() -> Option<PathBuf> {
    env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .map(|path| path.join(".minecraft").join("launcher_accounts.json"))
        .filter(|path| path.exists())
}

fn default_offline_username() -> String {
    env::var("USERNAME")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Player".to_owned())
}

fn dedup_accounts(accounts: Vec<AccountSource>) -> Vec<AccountSource> {
    let mut deduped = BTreeMap::new();

    for account in accounts {
        let key = account
            .profile_id
            .clone()
            .unwrap_or_else(|| format!("{}:{}", account.source_label, account.username));
        deduped.insert(key, account);
    }

    deduped.into_values().collect()
}

fn map_virtual_key(key: VirtualKeyCode) -> Option<PhysicalInput> {
    match key {
        VirtualKeyCode::W => Some(PhysicalInput::KeyW),
        VirtualKeyCode::A => Some(PhysicalInput::KeyA),
        VirtualKeyCode::S => Some(PhysicalInput::KeyS),
        VirtualKeyCode::D => Some(PhysicalInput::KeyD),
        VirtualKeyCode::Space => Some(PhysicalInput::Space),
        VirtualKeyCode::LShift => Some(PhysicalInput::LeftShift),
        VirtualKeyCode::LControl => Some(PhysicalInput::LeftControl),
        VirtualKeyCode::Key1 => Some(PhysicalInput::Digit1),
        VirtualKeyCode::Key2 => Some(PhysicalInput::Digit2),
        VirtualKeyCode::Key3 => Some(PhysicalInput::Digit3),
        VirtualKeyCode::Key4 => Some(PhysicalInput::Digit4),
        VirtualKeyCode::Key5 => Some(PhysicalInput::Digit5),
        VirtualKeyCode::Key6 => Some(PhysicalInput::Digit6),
        VirtualKeyCode::Key7 => Some(PhysicalInput::Digit7),
        VirtualKeyCode::Key8 => Some(PhysicalInput::Digit8),
        VirtualKeyCode::Key9 => Some(PhysicalInput::Digit9),
        VirtualKeyCode::Escape => Some(PhysicalInput::Escape),
        _ => None,
    }
}

fn parse_server_address(value: &str) -> Result<(String, u16), String> {
    if let Some((host, port)) = value.rsplit_once(':') {
        if let Ok(port) = port.parse() {
            return Ok((host.to_owned(), port));
        }
    }

    Ok((value.to_owned(), 25565))
}

fn next_value<I>(args: &mut I, flag: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn fill_gradient(frame: &mut [u8], width: u32, height: u32, top: [u8; 3], bottom: [u8; 3]) {
    for y in 0..height {
        let t = y as f32 / height.max(1) as f32;
        let color = [
            lerp_u8(top[0], bottom[0], t),
            lerp_u8(top[1], bottom[1], t),
            lerp_u8(top[2], bottom[2], t),
        ];
        for x in 0..width {
            put_pixel(frame, width, height, x as i32, y as i32, color);
        }
    }
}

fn draw_panel(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: UiRect,
    fill: [u8; 3],
    stroke: [u8; 3],
) {
    draw_rect(frame, width, height, rect, fill);
    draw_rect_outline(frame, width, height, rect, stroke);
}

fn draw_button(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: UiRect,
    fill: [u8; 3],
    stroke: [u8; 3],
) {
    draw_rect(frame, width, height, rect, fill);
    draw_rect_outline(frame, width, height, rect, stroke);
}

fn draw_input(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: UiRect,
    focused: bool,
    value: &str,
    placeholder: &str,
) {
    draw_rect(frame, width, height, rect, [23, 24, 33]);
    draw_rect_outline(
        frame,
        width,
        height,
        rect,
        if focused {
            [255, 214, 161]
        } else {
            [94, 102, 122]
        },
    );
    if value.is_empty() {
        draw_text_scaled(
            frame,
            width,
            height,
            rect.x + 10,
            rect.y + 12,
            placeholder,
            [140, 145, 160],
            1,
        );
    } else {
        draw_text_scaled(
            frame,
            width,
            height,
            rect.x + 10,
            rect.y + 12,
            &truncate_text(value, ((rect.width - 16) / 8) as usize),
            [244, 244, 244],
            1,
        );
    }
}

fn draw_text_box(
    frame: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    box_width: i32,
    lines: &[&str],
    color: [u8; 3],
) {
    let rect = UiRect {
        x,
        y,
        width: box_width,
        height: 18 + (lines.len() as i32 * 12),
    };
    draw_rect(frame, width, height, rect, [18, 19, 28]);
    draw_rect_outline(frame, width, height, rect, [84, 86, 105]);
    for (index, line) in lines.iter().enumerate() {
        draw_text_scaled(
            frame,
            width,
            height,
            x + 10,
            y + 8 + (index as i32 * 12),
            line,
            color,
            1,
        );
    }
}

fn draw_crosshair(frame: &mut [u8], width: u32, height: u32, color: [u8; 3]) {
    let cx = width as i32 / 2;
    let cy = height as i32 / 2;
    for offset in -6i32..=6i32 {
        if offset.abs() > 1 {
            put_pixel(frame, width, height, cx + offset, cy, color);
            put_pixel(frame, width, height, cx, cy + offset, color);
        }
    }
}

fn draw_runtime_overlay(frame: &mut [u8], width: u32, height: u32, runtime: &LiveRuntime) {
    let mut lines = Vec::new();
    lines.push(format!("player={}", runtime.username()));
    lines.push(format!("joined={}", runtime.summary().joined_game));
    if let Some(output) = runtime.output() {
        lines.push(format!(
            "xyz={:.2} {:.2} {:.2}",
            output.render.camera.position.x,
            output.render.camera.position.y,
            output.render.camera.position.z
        ));
        lines.push(format!(
            "yaw/pitch={:.1} {:.1}",
            output.render.camera.yaw, output.render.camera.pitch
        ));
    }
    let combat = runtime.combat_snapshot();
    lines.push(format!(
        "health={:.1} food={}",
        combat.health, combat.food_level
    ));
    if let Some(target) = runtime.targeted_entity() {
        lines.push(format!(
            "target={} dist={:.2}",
            target.entity_id, target.distance
        ));
    } else {
        lines.push("target=none".to_owned());
    }
    if let Some(snapshot) = runtime.usability_snapshot() {
        lines.push(format!(
            "chat={} tab={} sidebar={}",
            snapshot.chat_lines.len(),
            snapshot.tab_list.len(),
            snapshot.sidebar.is_some()
        ));
    }

    let rect = UiRect {
        x: 12,
        y: 12,
        width: 240,
        height: 16 + (lines.len() as i32 * 12),
    };
    draw_rect(frame, width, height, rect, [16, 18, 24]);
    draw_rect_outline(frame, width, height, rect, [105, 114, 140]);
    for (index, line) in lines.iter().enumerate() {
        draw_text_scaled(
            frame,
            width,
            height,
            rect.x + 8,
            rect.y + 6 + (index as i32 * 12),
            line,
            [245, 245, 245],
            1,
        );
    }

    if let Some(hud) = runtime.hud() {
        let mut y = height as i32 - 72;
        for line in hud.render_lines().into_iter().take(6) {
            draw_text_scaled(frame, width, height, 14, y, &line, [255, 232, 214], 1);
            y += 12;
        }
    }
}

#[derive(Clone, Debug)]
struct WindowLayout {
    panel: UiRect,
    slots: Vec<(UiRect, i16)>,
}

fn draw_border_warning(
    frame: &mut [u8],
    width: u32,
    height: u32,
    strength: f32,
    vignette: Option<&ImageAsset>,
) {
    if !strength.is_finite() || strength <= 0.0 {
        return;
    }
    let edge_width = (width.min(height) as f32 * 0.15).max(1.0);
    for y in 0..height {
        for x in 0..width {
            let distance = x.min(width - 1 - x).min(y.min(height - 1 - y)) as f32;
            let tint = ((1.0 - distance / edge_width).max(0.0) * strength).clamp(0.0, 1.0);
            let index = ((y * width + x) * 4) as usize;
            let mask = vignette.map(|image| {
                image.sample_linear_repeat(
                    (x as f32 + 0.5) / width as f32,
                    (y as f32 + 0.5) / height as f32,
                )
            });
            for channel in [1, 2] {
                let source = mask.map_or(tint, |sample| sample[channel] * strength);
                // GuiIngame uses ZERO, ONE_MINUS_SRC_COLOR for border vignette.
                frame[index + channel] =
                    (f32::from(frame[index + channel]) * (1.0 - source).clamp(0.0, 1.0)) as u8;
            }
        }
    }
}

fn draw_experience_overlay(
    frame: &mut [u8],
    width: u32,
    height: u32,
    xp: rmc_game::usability::Experience,
    assets: &GameAssets,
) {
    let x = width as i32 / 2 - 91;
    let y = height as i32 - 29;
    let filled = (xp.progress * 183.0) as i32;
    let cap = if xp.level >= 30 {
        112_i32.wrapping_add(xp.level.wrapping_sub(30).wrapping_mul(9))
    } else if xp.level >= 15 {
        37_i32.wrapping_add(xp.level.wrapping_sub(15).wrapping_mul(5))
    } else {
        7_i32.wrapping_add(xp.level.wrapping_mul(2))
    };
    if cap > 0 {
        for (bar_width, texture_y, color) in [(182, 64, [30, 45, 15]), (filled, 69, [128, 190, 35])]
        {
            if bar_width <= 0 {
                continue;
            }
            let rect = UiRect {
                x,
                y,
                width: bar_width,
                height: 5,
            };
            if let Some(icons) = assets.icons.as_ref() {
                draw_sprite_region(
                    frame,
                    width,
                    height,
                    icons,
                    rect,
                    UiRect {
                        x: 0,
                        y: texture_y,
                        width: bar_width,
                        height: 5,
                    },
                    [255, 255, 255],
                    1.0,
                );
            } else {
                draw_rect(frame, width, height, rect, color);
            }
        }
    }
    if xp.level > 0 {
        let label = xp.level.to_string();
        let label_x = width as i32 / 2 - label.len() as i32 * 3;
        let label_y = height as i32 - 35;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            draw_text_scaled(
                frame,
                width,
                height,
                label_x + dx,
                label_y + dy,
                &label,
                [0, 0, 0],
                1,
            );
        }
        draw_text_scaled(
            frame,
            width,
            height,
            label_x,
            label_y,
            &label,
            [128, 255, 32],
            1,
        );
    }
}

fn draw_hotbar_overlay(
    frame: &mut [u8],
    width: u32,
    height: u32,
    runtime: &LiveRuntime,
    assets: &GameAssets,
) {
    let selected_slot = runtime
        .output()
        .map(|output| output.render.hud.hotbar.selected_slot)
        .unwrap_or(0);
    let base_x = width as i32 / 2 - 91;
    let base_y = height as i32 - 27;

    if let Some(widgets) = assets.widgets.as_ref() {
        draw_sprite_region(
            frame,
            width,
            height,
            widgets,
            UiRect {
                x: base_x,
                y: base_y,
                width: 182,
                height: 22,
            },
            UiRect {
                x: 0,
                y: 0,
                width: 182,
                height: 22,
            },
            [255, 255, 255],
            1.0,
        );
        draw_sprite_region(
            frame,
            width,
            height,
            widgets,
            UiRect {
                x: base_x - 1 + i32::from(selected_slot) * 20,
                y: base_y - 1,
                width: 24,
                height: 24,
            },
            UiRect {
                x: 0,
                y: 22,
                width: 24,
                height: 24,
            },
            [255, 255, 255],
            1.0,
        );
    } else {
        let bar = UiRect {
            x: base_x,
            y: base_y,
            width: 182,
            height: 22,
        };
        draw_rect(frame, width, height, bar, [20, 20, 20]);
        draw_rect_outline(frame, width, height, bar, [188, 188, 188]);
        let selected = UiRect {
            x: base_x + i32::from(selected_slot) * 20,
            y: base_y,
            width: 22,
            height: 22,
        };
        draw_rect_outline(frame, width, height, selected, [255, 255, 160]);
    }

    for slot in 0..9 {
        draw_text_scaled(
            frame,
            width,
            height,
            base_x + 8 + slot * 20,
            base_y + 7,
            &(slot + 1).to_string(),
            [238, 238, 238],
            1,
        );
    }
}

fn draw_tab_overlay(frame: &mut [u8], width: u32, height: u32, runtime: &LiveRuntime) {
    let Some(snapshot) = runtime.usability_snapshot() else {
        return;
    };

    let entries = snapshot.tab_list.iter().take(20).collect::<Vec<_>>();
    if entries.is_empty() {
        return;
    }

    let rect = UiRect {
        x: width as i32 / 2 - 180,
        y: 16,
        width: 360,
        height: 22 + entries.len() as i32 * 11,
    };
    draw_rect(frame, width, height, rect, [18, 22, 28]);
    draw_rect_outline(frame, width, height, rect, [98, 108, 126]);
    draw_text_scaled(
        frame,
        width,
        height,
        rect.x + 8,
        rect.y + 7,
        "Players",
        [255, 240, 226],
        1,
    );

    for (index, entry) in entries.iter().enumerate() {
        draw_text_scaled(
            frame,
            width,
            height,
            rect.x + 8,
            rect.y + 20 + index as i32 * 11,
            &truncate_text(&entry.name, 28),
            [236, 236, 236],
            1,
        );
        draw_text_scaled(
            frame,
            width,
            height,
            rect.x + rect.width - 60,
            rect.y + 20 + index as i32 * 11,
            &format!("{}ms", entry.latency),
            [172, 214, 255],
            1,
        );
    }
}

fn draw_window_overlay(frame: &mut [u8], width: u32, height: u32, window: &WindowSnapshot) {
    let layout = layout_window_snapshot(window, width as i32, height as i32);
    draw_rect(frame, width, height, layout.panel, [28, 24, 20]);
    draw_rect_outline(frame, width, height, layout.panel, [165, 126, 88]);
    draw_text_scaled(
        frame,
        width,
        height,
        layout.panel.x + 10,
        layout.panel.y + 8,
        &truncate_text(&window.title_json, 34),
        [255, 236, 214],
        1,
    );

    if window.inventory_type == "minecraft:furnace" {
        let property = |id| i32::from(*window.properties.get(&id).unwrap_or(&0));
        let burn_total = if property(1) == 0 { 200 } else { property(1) };
        let burn = (property(0) * 60 / burn_total).clamp(0, 60);
        let cook = if property(3) == 0 {
            0
        } else {
            (property(2) * 60 / property(3)).clamp(0, 60)
        };
        for (x, label, amount, color) in [
            (layout.panel.x + 10, "FUEL", burn, [244, 144, 40]),
            (layout.panel.x + 100, "COOK", cook, [235, 222, 150]),
        ] {
            draw_text_scaled(
                frame,
                width,
                height,
                x,
                layout.panel.y + 44,
                label,
                [230, 230, 230],
                1,
            );
            draw_rect(
                frame,
                width,
                height,
                UiRect {
                    x,
                    y: layout.panel.y + 55,
                    width: 60,
                    height: 4,
                },
                [60, 60, 60],
            );
            draw_rect(
                frame,
                width,
                height,
                UiRect {
                    x,
                    y: layout.panel.y + 55,
                    width: amount,
                    height: 4,
                },
                color,
            );
        }
    }

    for (rect, slot_id) in layout.slots {
        draw_rect(frame, width, height, rect, [34, 37, 46]);
        draw_rect_outline(frame, width, height, rect, [92, 99, 116]);
        if let Some(item) = window
            .slots
            .get(slot_id as usize)
            .and_then(|slot| slot.as_ref())
        {
            draw_text_scaled(
                frame,
                width,
                height,
                rect.x + 2,
                rect.y + 4,
                &truncate_text(&item.item_id.to_string(), 3),
                [246, 246, 246],
                1,
            );
            if item.count > 1 {
                draw_text_scaled(
                    frame,
                    width,
                    height,
                    rect.x + 2,
                    rect.y + 12,
                    &item.count.to_string(),
                    [255, 221, 120],
                    1,
                );
            }
        }
    }
}

fn death_button_rect(width: i32, height: i32, index: i32) -> UiRect {
    UiRect {
        x: width / 2 - 100,
        y: height / 4 + 72 + index * 28,
        width: 200,
        height: 24,
    }
}

fn chat_component_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Array(values) => values.iter().map(chat_component_text).collect(),
        serde_json::Value::Object(object) => {
            let mut text = object
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            if let Some(key) = object.get("translate").and_then(|v| v.as_str()) {
                let args: Vec<String> = object
                    .get("with")
                    .and_then(|v| v.as_array())
                    .map(|values| values.iter().map(chat_component_text).collect())
                    .unwrap_or_default();
                text = match (key, args.as_slice()) {
                    ("chat.type.text", [name, message]) => format!("<{name}> {message}"),
                    ("chat.type.announcement", [name, message]) => format!("[{name}] {message}"),
                    _ => format!("{} {}", key, args.join(" ")),
                };
            }
            if let Some(extra) = object.get("extra") {
                text.push_str(&chat_component_text(extra));
            }
            text
        }
        _ => String::new(),
    }
}

fn draw_chat_history(
    frame: &mut [u8],
    width: u32,
    height: u32,
    lines: &[rmc_game::usability::ChatLine],
    scroll: usize,
) {
    let offset = scroll.min(lines.len().saturating_sub(8));
    for (row, line) in lines.iter().rev().skip(offset).take(8).enumerate() {
        let y = height as i32 - 50 - row as i32 * 12;
        draw_rect(
            frame,
            width,
            height,
            UiRect {
                x: 8,
                y: y - 2,
                width: (width as i32 - 16).min(480),
                height: 12,
            },
            [10, 10, 10],
        );
        let text = serde_json::from_str(&line.message_json)
            .map(|v| chat_component_text(&v))
            .unwrap_or_else(|_| line.message_json.clone());
        draw_text_scaled(
            frame,
            width,
            height,
            12,
            y,
            &truncate_text(&text, ((width.saturating_sub(24)).min(470) / 6) as usize),
            [255, 255, 255],
            1,
        );
    }
}

fn draw_chat_input_overlay(frame: &mut [u8], width: u32, height: u32, value: &str) {
    let rect = UiRect {
        x: 8,
        y: height as i32 - 34,
        width: width as i32 - 16,
        height: 20,
    };
    draw_rect(frame, width, height, rect, [10, 10, 10]);
    draw_rect_outline(frame, width, height, rect, [112, 112, 112]);
    draw_text_scaled(
        frame,
        width,
        height,
        rect.x + 6,
        rect.y + 6,
        &format!(
            "> {}",
            truncate_text(value, ((rect.width - 16) / 8) as usize)
        ),
        [255, 255, 255],
        1,
    );
}

fn layout_window_snapshot(
    window: &WindowSnapshot,
    screen_width: i32,
    screen_height: i32,
) -> WindowLayout {
    if window.window_id == 0 && window.inventory_type == "minecraft:inventory" {
        let panel = UiRect {
            x: screen_width / 2 - 88,
            y: screen_height / 2 - 83,
            width: 176,
            height: 166,
        };
        let slots = (0..window.slots.len().min(45))
            .map(|slot| {
                let (x, y) = match slot {
                    0 => (144, 36),
                    1..=4 => (
                        88 + ((slot - 1) % 2) as i32 * 18,
                        26 + ((slot - 1) / 2) as i32 * 18,
                    ),
                    5..=8 => (8, 8 + (slot - 5) as i32 * 18),
                    9..=35 => (
                        8 + ((slot - 9) % 9) as i32 * 18,
                        84 + ((slot - 9) / 9) as i32 * 18,
                    ),
                    _ => (8 + (slot - 36) as i32 * 18, 142),
                };
                (
                    UiRect {
                        x: panel.x + x,
                        y: panel.y + y,
                        width: 16,
                        height: 16,
                    },
                    slot as i16,
                )
            })
            .collect();
        return WindowLayout { panel, slots };
    }
    let total_slots = window.slots.len().min(90);
    let columns = 9usize;
    let top_rows = usize::from(window.slot_count).div_ceil(columns).max(1);
    let player_rows = total_slots
        .saturating_sub(usize::from(window.slot_count))
        .div_ceil(columns);
    let panel_width = columns as i32 * 20 + 20;
    let panel_height =
        (top_rows as i32 + player_rows as i32) * 20 + 36 + if player_rows > 0 { 20 } else { 0 };
    let panel = UiRect {
        x: screen_width / 2 - panel_width / 2,
        y: screen_height / 2 - panel_height / 2,
        width: panel_width,
        height: panel_height,
    };

    let mut slots = Vec::new();
    for slot_id in 0..total_slots {
        let slot_id_i16 = slot_id as i16;
        let row = slot_id / columns;
        let column = if slot_id < window.slot_count {
            slot_id % columns
        } else {
            (slot_id - window.slot_count) % columns
        };
        let y_offset = if slot_id < usize::from(window.slot_count) {
            row as i32
        } else {
            top_rows as i32 + 1 + (slot_id - usize::from(window.slot_count)) as i32 / columns as i32
        };
        let rect = UiRect {
            x: panel.x + 10 + column as i32 * 20,
            y: panel.y + 22 + y_offset * 20,
            width: 18,
            height: 18,
        };
        slots.push((rect, slot_id_i16));
    }

    WindowLayout { panel, slots }
}

fn draw_sprite_region(
    frame: &mut [u8],
    width: u32,
    height: u32,
    image: &ImageAsset,
    dest: UiRect,
    source: UiRect,
    tint: [u8; 3],
    alpha: f32,
) {
    if dest.width <= 0 || dest.height <= 0 || source.width <= 0 || source.height <= 0 {
        return;
    }

    for dy in 0..dest.height {
        for dx in 0..dest.width {
            let u = dx as f32 / dest.width as f32;
            let v = dy as f32 / dest.height as f32;
            let sx = source.x + (u * source.width as f32).floor() as i32;
            let sy = source.y + (v * source.height as f32).floor() as i32;
            if sx < 0 || sy < 0 || sx >= image.width() as i32 || sy >= image.height() as i32 {
                continue;
            }
            let sample = image.pixel(sx as u32, sy as u32);
            blend_pixel(
                frame,
                width,
                height,
                dest.x + dx,
                dest.y + dy,
                [
                    ((sample[0] as f32) * (tint[0] as f32 / 255.0)) as u8,
                    ((sample[1] as f32) * (tint[1] as f32 / 255.0)) as u8,
                    ((sample[2] as f32) * (tint[2] as f32 / 255.0)) as u8,
                    ((sample[3] as f32) * alpha.clamp(0.0, 1.0)) as u8,
                ],
            );
        }
    }
}

fn render_world_meshes(
    frame: &mut [u8],
    depth: &mut [f32],
    width: u32,
    height: u32,
    camera_position: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,
    world_render: &WorldRenderSnapshot,
    assets: &GameAssets,
    skylight_subtracted: u8,
) {
    let basis = camera_basis(camera_yaw, camera_pitch);
    for mesh in &world_render.chunk_meshes {
        render_chunk_mesh(
            frame,
            depth,
            width,
            height,
            camera_position,
            basis,
            mesh,
            assets,
            skylight_subtracted,
        );
    }
}

fn render_tracked_players(
    frame: &mut [u8],
    depth: &mut [f32],
    width: u32,
    height: u32,
    camera_position: Vec3,
    camera_yaw: f32,
    camera_pitch: f32,
    entities: &EntityTracker,
    targeted_entity: Option<TargetedEntity>,
    spectator: bool,
    friendly_invisible: impl Fn(&str) -> bool,
) {
    let basis = camera_basis(camera_yaw, camera_pitch);
    for entity in entities.players() {
        if entity.flag(5)
            && !spectator
            && !entity
                .profile_name
                .as_deref()
                .is_some_and(&friendly_invisible)
        {
            continue;
        }
        let highlighted = targeted_entity
            .map(|target| target.entity_id == entity.entity_id)
            .unwrap_or(false);
        render_box(
            frame,
            depth,
            width,
            height,
            camera_position,
            basis,
            Vec3::new(
                entity.position.x - 0.3,
                entity.position.y,
                entity.position.z - 0.3,
            ),
            Vec3::new(
                entity.position.x + 0.3,
                entity.position.y + 1.8,
                entity.position.z + 0.3,
            ),
            if highlighted {
                [255, 214, 92]
            } else {
                [255, 92, 92]
            },
        );
    }
}

fn render_chunk_mesh(
    frame: &mut [u8],
    depth: &mut [f32],
    width: u32,
    height: u32,
    camera_position: Vec3,
    basis: CameraBasis,
    mesh: &ChunkMesh,
    assets: &GameAssets,
    skylight_subtracted: u8,
) {
    for triangle in mesh.indices.chunks_exact(3) {
        let a = &mesh.vertices[triangle[0] as usize];
        let b = &mesh.vertices[triangle[1] as usize];
        let c = &mesh.vertices[triangle[2] as usize];
        let texture = assets.block_texture(a.block_state_id, a.face);
        let light = light_factor(
            a.normal,
            a.sky_light.saturating_sub(skylight_subtracted),
            a.block_light,
        );
        let fallback = fallback_block_color(a.block_state_id, light);
        let projected = [
            project_point(camera_position, basis, a.position, a.uv),
            project_point(camera_position, basis, b.position, b.uv),
            project_point(camera_position, basis, c.position, c.uv),
        ];
        draw_projected_triangle(
            frame, depth, width, height, projected, texture, light, fallback,
        );
    }
}

fn render_box(
    frame: &mut [u8],
    depth: &mut [f32],
    width: u32,
    height: u32,
    camera_position: Vec3,
    basis: CameraBasis,
    min: Vec3,
    max: Vec3,
    color: [u8; 3],
) {
    let corners = [
        [min.x as f32, min.y as f32, min.z as f32],
        [max.x as f32, min.y as f32, min.z as f32],
        [max.x as f32, max.y as f32, min.z as f32],
        [min.x as f32, max.y as f32, min.z as f32],
        [min.x as f32, min.y as f32, max.z as f32],
        [max.x as f32, min.y as f32, max.z as f32],
        [max.x as f32, max.y as f32, max.z as f32],
        [min.x as f32, max.y as f32, max.z as f32],
    ];
    let triangles = [
        (0usize, 1usize, 2usize),
        (0, 2, 3),
        (5, 4, 7),
        (5, 7, 6),
        (4, 0, 3),
        (4, 3, 7),
        (1, 5, 6),
        (1, 6, 2),
        (3, 2, 6),
        (3, 6, 7),
        (4, 5, 1),
        (4, 1, 0),
    ];
    for (a, b, c) in triangles {
        draw_projected_triangle(
            frame,
            depth,
            width,
            height,
            [
                project_point(camera_position, basis, corners[a], [0.0, 0.0]),
                project_point(camera_position, basis, corners[b], [0.0, 0.0]),
                project_point(camera_position, basis, corners[c], [0.0, 0.0]),
            ],
            None,
            1.0,
            color,
        );
    }
}

#[derive(Clone, Copy)]
struct CameraBasis {
    forward: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
}

#[derive(Clone, Copy)]
struct ProjectedPoint {
    screen_x: f32,
    screen_y: f32,
    depth: f32,
    u: f32,
    v: f32,
    visible: bool,
}

fn camera_basis(yaw: f32, pitch: f32) -> CameraBasis {
    let yaw = yaw.to_radians();
    let pitch = pitch.to_radians();
    let pitch_cos = pitch.cos();
    let forward = normalize([-yaw.sin() * pitch_cos, -pitch.sin(), yaw.cos() * pitch_cos]);
    let right = normalize([forward[2], 0.0, -forward[0]]);
    let up = normalize(cross(right, forward));
    CameraBasis { forward, right, up }
}

fn project_point(
    camera_position: Vec3,
    basis: CameraBasis,
    position: [f32; 3],
    uv: [f32; 2],
) -> ProjectedPoint {
    let relative = [
        position[0] - camera_position.x as f32,
        position[1] - camera_position.y as f32,
        position[2] - camera_position.z as f32,
    ];
    let view_x = dot(relative, basis.right);
    let view_y = dot(relative, basis.up);
    let view_z = dot(relative, basis.forward);

    if view_z <= NEAR_PLANE {
        return ProjectedPoint {
            screen_x: 0.0,
            screen_y: 0.0,
            depth: view_z,
            u: uv[0],
            v: uv[1],
            visible: false,
        };
    }

    let focal = 1.0 / (VERTICAL_FOV_DEGREES.to_radians() * 0.5).tan();
    ProjectedPoint {
        screen_x: view_x * focal / view_z,
        screen_y: view_y * focal / view_z,
        depth: view_z,
        u: uv[0],
        v: uv[1],
        visible: true,
    }
}

fn draw_projected_triangle(
    frame: &mut [u8],
    depth_buffer: &mut [f32],
    width: u32,
    height: u32,
    mut projected: [ProjectedPoint; 3],
    texture: Option<&ImageAsset>,
    light: f32,
    fallback: [u8; 3],
) {
    if !projected.iter().all(|vertex| vertex.visible) {
        return;
    }

    let half_width = width as f32 * 0.5;
    let half_height = height as f32 * 0.5;
    for vertex in &mut projected {
        vertex.screen_x = half_width + vertex.screen_x * half_height;
        vertex.screen_y = half_height - vertex.screen_y * half_height;
    }

    let area = edge(
        projected[0],
        projected[1],
        projected[2].screen_x,
        projected[2].screen_y,
    );
    if area.abs() <= f32::EPSILON {
        return;
    }

    let min_x = projected
        .iter()
        .map(|vertex| vertex.screen_x.floor() as i32)
        .min()
        .unwrap_or_default()
        .clamp(0, width as i32 - 1);
    let max_x = projected
        .iter()
        .map(|vertex| vertex.screen_x.ceil() as i32)
        .max()
        .unwrap_or_default()
        .clamp(0, width as i32 - 1);
    let min_y = projected
        .iter()
        .map(|vertex| vertex.screen_y.floor() as i32)
        .min()
        .unwrap_or_default()
        .clamp(0, height as i32 - 1);
    let max_y = projected
        .iter()
        .map(|vertex| vertex.screen_y.ceil() as i32)
        .max()
        .unwrap_or_default()
        .clamp(0, height as i32 - 1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let sample_x = x as f32 + 0.5;
            let sample_y = y as f32 + 0.5;
            let w0 = edge(projected[1], projected[2], sample_x, sample_y) / area;
            let w1 = edge(projected[2], projected[0], sample_x, sample_y) / area;
            let w2 = edge(projected[0], projected[1], sample_x, sample_y) / area;

            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }

            let depth = projected[0].depth * w0 + projected[1].depth * w1 + projected[2].depth * w2;
            let index = (y as u32 * width + x as u32) as usize;

            if depth < depth_buffer[index] {
                depth_buffer[index] = depth;
                let u = projected[0].u * w0 + projected[1].u * w1 + projected[2].u * w2;
                let v = projected[0].v * w0 + projected[1].v * w1 + projected[2].v * w2;
                let color = texture
                    .map(|texture| shade_sample(texture.sample_repeat(u, v), light))
                    .unwrap_or(fallback);
                put_pixel(frame, width, height, x, y, color);
            }
        }
    }
}

fn edge(a: ProjectedPoint, b: ProjectedPoint, x: f32, y: f32) -> f32 {
    (x - a.screen_x) * (b.screen_y - a.screen_y) - (y - a.screen_y) * (b.screen_x - a.screen_x)
}

fn light_factor(normal: [f32; 3], sky_light: u8, block_light: u8) -> f32 {
    (0.32
        + normal[1].max(0.0) * 0.28
        + (sky_light as f32 / 15.0) * 0.28
        + (block_light as f32 / 15.0) * 0.12)
        .clamp(0.15, 1.0)
}

fn fallback_block_color(block_state_id: u16, light: f32) -> [u8; 3] {
    let hash = u32::from(block_state_id).wrapping_mul(0x45d9f3b);
    let base = [
        60 + ((hash >> 16) & 0x7f) as u8,
        70 + ((hash >> 8) & 0x6f) as u8,
        80 + (hash & 0x5f) as u8,
    ];
    [
        (base[0] as f32 * light) as u8,
        (base[1] as f32 * light) as u8,
        (base[2] as f32 * light) as u8,
    ]
}

fn shade_sample(sample: [u8; 4], light: f32) -> [u8; 3] {
    [
        ((sample[0] as f32) * light) as u8,
        ((sample[1] as f32) * light) as u8,
        ((sample[2] as f32) * light) as u8,
    ]
}

fn draw_rect(frame: &mut [u8], width: u32, height: u32, rect: UiRect, color: [u8; 3]) {
    for y in rect.y.max(0)..(rect.y + rect.height).min(height as i32) {
        for x in rect.x.max(0)..(rect.x + rect.width).min(width as i32) {
            put_pixel(frame, width, height, x, y, color);
        }
    }
}

fn draw_rect_outline(frame: &mut [u8], width: u32, height: u32, rect: UiRect, color: [u8; 3]) {
    for x in rect.x..(rect.x + rect.width) {
        put_pixel(frame, width, height, x, rect.y, color);
        put_pixel(frame, width, height, x, rect.y + rect.height - 1, color);
    }
    for y in rect.y..(rect.y + rect.height) {
        put_pixel(frame, width, height, rect.x, y, color);
        put_pixel(frame, width, height, rect.x + rect.width - 1, y, color);
    }
}

fn draw_text_scaled(
    frame: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 3],
    scale: i32,
) {
    if let Some(font) = ui_font() {
        draw_text_fontdue(
            frame,
            width,
            height,
            x,
            y,
            text,
            color,
            scale.max(1) as f32 * 14.0,
            font,
        );
        return;
    }

    let scale = scale.max(1);
    let mut cursor_x = x;
    for character in text.chars() {
        if let Some(glyph) = font8x8::BASIC_FONTS.get(character) {
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..8 {
                    if (bits >> column) & 1 == 0 {
                        continue;
                    }
                    for dy in 0..scale {
                        for dx in 0..scale {
                            put_pixel(
                                frame,
                                width,
                                height,
                                cursor_x + ((7 - column) * scale) + dx,
                                y + (row as i32 * scale) + dy,
                                color,
                            );
                        }
                    }
                }
            }
        }
        cursor_x += 8 * scale;
    }
}

fn draw_text_fontdue(
    frame: &mut [u8],
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    text: &str,
    color: [u8; 3],
    px: f32,
    font: &Font,
) {
    let mut cursor_x = x as f32;
    let baseline_y = y as f32 + px;

    for character in text.chars() {
        if character == '\n' {
            cursor_x = x as f32;
            continue;
        }

        let (metrics, bitmap) = font.rasterize(character, px);
        let start_x = cursor_x + metrics.xmin as f32;
        let start_y = baseline_y + metrics.ymin as f32;

        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let coverage = bitmap[row * metrics.width + column];
                if coverage == 0 {
                    continue;
                }

                blend_pixel(
                    frame,
                    width,
                    height,
                    start_x as i32 + column as i32,
                    start_y as i32 + row as i32,
                    [color[0], color[1], color[2], coverage],
                );
            }
        }

        cursor_x += metrics.advance_width;
    }
}

fn ui_font() -> Option<&'static Font> {
    static UI_FONT: OnceLock<Option<Font>> = OnceLock::new();
    UI_FONT.get_or_init(load_ui_font).as_ref()
}

fn load_ui_font() -> Option<Font> {
    for candidate in ui_font_candidates() {
        let Ok(bytes) = fs::read(candidate) else {
            continue;
        };
        if let Ok(font) = Font::from_bytes(bytes, FontSettings::default()) {
            return Some(font);
        }
    }
    None
}

fn ui_font_candidates() -> Vec<PathBuf> {
    let windir = env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_owned());
    let fonts = PathBuf::from(windir).join("Fonts");
    vec![
        fonts.join("segoeui.ttf"),
        fonts.join("tahoma.ttf"),
        fonts.join("arial.ttf"),
    ]
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }

    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

fn put_pixel(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 3]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }

    let index = ((y as u32 * width + x as u32) * 4) as usize;
    frame[index] = color[0];
    frame[index + 1] = color[1];
    frame[index + 2] = color[2];
    frame[index + 3] = 0xff;
}

fn blend_pixel(frame: &mut [u8], width: u32, height: u32, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
        return;
    }

    let alpha = color[3] as f32 / 255.0;
    if alpha <= 0.0 {
        return;
    }

    let index = ((y as u32 * width + x as u32) * 4) as usize;
    let inv_alpha = 1.0 - alpha;
    frame[index] = (frame[index] as f32 * inv_alpha + color[0] as f32 * alpha) as u8;
    frame[index + 1] = (frame[index + 1] as f32 * inv_alpha + color[1] as f32 * alpha) as u8;
    frame[index + 2] = (frame[index + 2] as f32 * inv_alpha + color[2] as f32 * alpha) as u8;
    frame[index + 3] = 0xff;
}

fn lerp_u8(start: u8, end: u8, t: f32) -> u8 {
    (start as f32 + (end as f32 - start as f32) * t) as u8
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(vector: [f32; 3]) -> [f32; 3] {
    let length = dot(vector, vector).sqrt();
    if length <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [vector[0] / length, vector[1] / length, vector[2] / length]
    }
}

#[cfg(test)]
mod inventory_layout_tests {
    use super::*;

    #[test]
    fn received_chat_components_preserve_player_message_and_extra_text() {
        let value = serde_json::json!({"translate":"chat.type.text","with":[{"text":"Alex"},{"text":"hello","extra":[{"text":" world"}]}]});
        assert_eq!(chat_component_text(&value), "<Alex> hello world");
        assert_eq!(
            chat_component_text(&serde_json::json!([{"text":"a"},"b"])),
            "ab"
        );
    }

    #[test]
    fn border_warning_uses_loaded_texture_color_in_destination_blend() {
        let path =
            std::env::temp_dir().join(format!("rmc-vignette-test-{}.png", std::process::id()));
        image::RgbaImage::from_raw(2, 1, vec![255, 255, 128, 255, 0, 0, 0, 255])
            .unwrap()
            .save(&path)
            .unwrap();
        let texture = ImageAsset::load(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        let mut frame = vec![200; 8];
        draw_border_warning(&mut frame, 2, 1, 0.5, Some(&texture));
        assert_eq!(frame, vec![200, 100, 149, 200, 200, 200, 200, 200]);
    }

    #[test]
    fn border_warning_tints_edges_and_leaves_center_unchanged() {
        let mut frame = vec![200; 100 * 100 * 4];
        draw_border_warning(&mut frame, 100, 100, 0.5, None);
        assert_eq!(&frame[0..4], &[200, 100, 100, 200]);
        let center = (50 * 100 + 50) * 4;
        assert_eq!(&frame[center..center + 4], &[200; 4]);
        let unchanged = frame.clone();
        draw_border_warning(&mut frame, 100, 100, 0.0, None);
        assert_eq!(frame, unchanged);
    }

    #[test]
    fn experience_bar_renders_source_width_and_vertical_position() {
        let mut frame = vec![0; 320 * 200 * 4];
        draw_experience_overlay(
            &mut frame,
            320,
            200,
            rmc_game::usability::Experience {
                progress: 0.999,
                level: 7,
                total: 0,
            },
            &GameAssets::default(),
        );
        let pixel = |x: usize, y: usize| &frame[(y * 320 + x) * 4..(y * 320 + x) * 4 + 3];
        assert_eq!(pixel(69, 170), [0, 0, 0]);
        assert_eq!(pixel(69, 171), [128, 190, 35]);
        assert_eq!(pixel(250, 175), [128, 190, 35]);
        assert_eq!(pixel(251, 175), [0, 0, 0]);
        assert_eq!(pixel(69, 176), [0, 0, 0]);
    }

    #[test]
    fn every_slot_is_unique_and_inside_the_clickable_panel() {
        for (id, kind, count, total) in [
            (0, "minecraft:inventory", 9, 45),
            (4, "minecraft:chest", 27, 63),
            (5, "minecraft:chest", 54, 90),
            (6, "minecraft:furnace", 3, 39),
        ] {
            let window = WindowSnapshot {
                properties: Default::default(),
                window_id: id,
                inventory_type: kind.into(),
                title_json: "{}".into(),
                slot_count: count,
                slots: vec![None; total],
                carried_item: None,
            };
            let layout = layout_window_snapshot(&window, 640, 480);
            assert_eq!(layout.slots.len(), total);
            for (index, (rect, slot)) in layout.slots.iter().enumerate() {
                assert_eq!(*slot as usize, index);
                assert!(layout.panel.contains(rect.x as f32, rect.y as f32));
                assert!(layout.panel.contains(
                    (rect.x + rect.width - 1) as f32,
                    (rect.y + rect.height - 1) as f32
                ));
                assert_eq!(
                    layout
                        .slots
                        .iter()
                        .filter(|(other, _)| other.contains(
                            (rect.x + rect.width / 2) as f32,
                            (rect.y + rect.height / 2) as f32
                        ))
                        .count(),
                    1
                );
            }
        }
    }
}
