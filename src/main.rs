#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#![allow(clippy::borrow_interior_mutable_const)]
#![allow(clippy::declare_interior_mutable_const)]

// use std::sync::Arc;
use freya::prelude::*;
use gumdrop::Options;
use native_dialog::{MessageDialogBuilder, MessageLevel};
#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;
use winit::{dpi::PhysicalPosition, window::WindowLevel};

use crate::{
  app_state::{AppState, SharedAppState},
  components::{
    MessagesSection, Soundboard, VoiceControls, VoiceSection, voice_controls::RedrawSender,
  },
  config::{is_first_run, load_config, save_config},
  config_watcher::start_config_watcher,
  configurator::{open_configurator, open_configurator_standalone},
  display::{
    save_window_position, specific_monitor_or_primary, update_monitor, window_size_for_display,
  },
  manager::OverlayManager,
  notifications::create_notification_thread,
  payloads::Notification,
  transport::create_transport_thread,
  // updates::maybe_notify_update,
  util::{bridge::BridgeMessage, colors},
};

mod app_state;
mod components;
mod config;
mod config_watcher;
mod configurator;
mod display;
mod ipc;
#[cfg(not(target_os = "macos"))]
mod keys;
mod logger;
mod manager;
mod notifications;
mod payloads;
mod target;
mod transport;
mod updates;
mod user;
mod util;
mod websocket;
mod window;

static TWEMOJI_FONT: &[u8] = include_bytes!("../assets/fonts/Twemoji.ttf");

const GIT_HASH: Option<&str> = option_env!("GIT_HASH");
const APP_NAME: Option<&str> = option_env!("CARGO_PKG_NAME");
const APP_VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const CLIENT_ID: &str = "207646673902501888";

#[derive(Debug, Clone, Options)]
pub struct Args {
  #[options(help = "Display usage information")]
  help: bool,

  #[options(help = "The port to run the websocket server on", default = "6888")]
  port: u16,

  #[options(help = "Print version information")]
  version: bool,

  #[options(help = "Enable various debugging features")]
  debug: bool,

  #[options(help = "Force websocket mode instead of IPC")]
  websocket: bool,

  #[options(help = "Force IPC mode instead of websocket")]
  ipc: bool,

  #[options(help = "Open the configuration window")]
  config: bool,

  #[options(help = "Target application to launch (if any)", free)]
  target: Vec<String>,
}

fn main() {
  let args = Args::parse_args_default_or_exit();

  if args.help_requested() {
    println!("{}", Args::usage());
    std::process::exit(0);
  }

  if args.version {
    println!(
      "{} version {} (rev {})",
      APP_NAME.unwrap_or("Unknown"),
      APP_VERSION.unwrap_or("0.0.0"),
      GIT_HASH.unwrap_or("unknown")
    );
    std::process::exit(0);
  }

  if args.config {
    open_configurator_standalone();
    std::process::exit(0);
  }

  if util::process::is_already_running() {
    MessageDialogBuilder::default()
      .set_level(MessageLevel::Error)
      .set_title("Lumina")
      .set_text("Lumina is already running. Kill the existing process before starting a new one.")
      .alert()
      .show()
      .expect("Failed to show message dialog");
    std::process::exit(0);
  }

  let config = load_config().unwrap_or_default();

  if let Some(software_rendering) = config.software_rendering
    && software_rendering
  {
    unsafe { std::env::set_var("FREYA_RENDERER", "software") };
  }

  let display = specific_monitor_or_primary();
  let monitor_position = (display.x, display.y);

  // Compute the initial window size for the chosen display.
  let window_size = window_size_for_display(&display);

  let initial_position = config
    .window_position
    .map(|(x, y)| PhysicalPosition::new(x, y))
    .unwrap_or_else(|| PhysicalPosition::new(monitor_position.0, monitor_position.1));

  #[cfg(target_os = "linux")]
  {
    let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    if session_type.to_lowercase() == "wayland" {
      warn!(
        "You are using Wayland. Lumina will probably not work correctly unless running with XWayland."
      );
    }
  }

  if !args.target.is_empty() {
    log!("Launching child process: {:?}", args.target);
    target::launch_target(args.target);
  }

  launch(
    LaunchConfig::new()
      .with_font("Twemoji", TWEMOJI_FONT)
      .with_fallback_font("Twemoji")
      .with_tray(
        || {
          use freya::tray::menu::{Menu, MenuItemBuilder};
          let settings_item = MenuItemBuilder::new().id("Settings".into()).text("Settings").enabled(true).build();
          let quit_item = MenuItemBuilder::new().id("Quit".into()).text("Quit").enabled(true).build();
          let menu = Menu::new();
          menu.append(&settings_item).unwrap();
          menu.append(&quit_item).unwrap();
          freya::tray::TrayIconBuilder::new()
            .with_icon(LaunchConfig::tray_icon(include_bytes!("../assets/lumina.png")))
            .with_tooltip("Lumina")
            .with_menu(Box::new(menu))
            .build()
            .unwrap()
        },
        |event, mut ctx| {
          if let freya::tray::TrayEvent::Menu(ref menu_event) = event {
            if menu_event.id == "Settings" {
              let shared = SharedAppState::default();
              if let Some(saved) = load_config() {
                shared.write().unwrap().config = saved;
              }
              ctx.launch_window(
                configurator::configurator_window(shared, flume::unbounded().0),
              );
            } else if menu_event.id == "Quit" {
              std::process::exit(0);
            }
          }
        },
      )
      .with_window(
        WindowConfig::new(app)
          .with_title("lumina")
          .with_icon(LaunchConfig::window_icon(include_bytes!("../assets/lumina.png")))
          .with_decorations(false)
          .with_transparency(true)
          .with_background(Color::TRANSPARENT)
          .with_window_attributes(move |mut w, _event_loop| {
            w = w
              .with_inner_size(window_size)
              .with_resizable(false)
              .with_window_level(WindowLevel::AlwaysOnTop)
              .with_position(initial_position);

            #[cfg(target_os = "windows")]
            {
              w = w.with_skip_taskbar(true);
            }

            #[cfg(target_os = "linux")]
            {
              use winit::platform::wayland::WindowAttributesExtWayland;
              use winit::platform::x11::{WindowAttributesExtX11, WindowType};

              w = WindowAttributesExtX11::with_name(w, "lumina", "lumina")
                .with_x11_window_type(vec![WindowType::Utility])
                .with_override_redirect(true);
              w = WindowAttributesExtWayland::with_name(w, "lumina", "lumina");
            }

            w
          }),
      ),
  );
}

fn app() -> impl IntoElement {
  let args = Args::parse_args_default_or_exit();
  let mut app_state = use_state(AppState::new);
  let mut soundboard_open = use_state(|| false);

  let (shared, redraw_tx) = use_hook(move || {
    let (ws_sender, ws_receiver) = flume::unbounded::<BridgeMessage>();
    let (redraw_tx, redraw_rx) = flume::unbounded::<()>();
    #[cfg(not(target_os = "macos"))]
    let (keybind_tx, keybind_rx) = flume::unbounded::<keys::KeyEvent>();

    app_state.write().ws_sender = Some(ws_sender.clone());

    // Shared state for background threads
    let mut initial = AppState::new();
    initial.ws_sender = Some(ws_sender);

    if let Some(saved) = load_config() {
      initial.config = saved;
    }

    let shared: SharedAppState = std::sync::Arc::new(std::sync::RwLock::new(initial));

    if !args.debug {
      window::set_clickable(false);
    }

    #[cfg(not(target_os = "macos"))]
    keys::watch_keybinds(shared.clone(), keybind_tx);

    create_transport_thread(shared.clone(), redraw_tx.clone(), args, ws_receiver);
    create_notification_thread(shared.clone(), redraw_tx.clone());

    shared.write().unwrap().notify(Notification {
      title: format!(
        "Lumina v{}",
        APP_VERSION.unwrap_or("0.0.0"),
      ),
      body: String::new(),
      icon: String::new(),
      timestamp: Some(chrono::Utc::now().timestamp()),
      timeout_secs: 3,
      guild_id: None,
      channel_id: None,
      message_id: None,
      actions: None,
    });

    // sync SharedAppState -> AppState on every redraw signal
    let shared_sync = shared.clone();
    spawn_forever(async move {
      while let Ok(()) = redraw_rx.recv_async().await {
        let synced = shared_sync.read().unwrap().clone();
        let ws_sender = app_state.read().ws_sender.clone();
        let is_open = app_state.read().is_open;
        *app_state.write() = AppState {
          ws_sender,
          is_open,
          ..synced
        };

        update_monitor();
      }
    });

    // Both of these must happen before shared/redraw_tx are moved into the keybind handler
    if is_first_run() {
      open_configurator(shared.clone(), redraw_tx.clone());
      redraw_tx.send(()).ok();

      // Write the config regardless so we don't trigger this in the future
      {
        let state = shared.read().unwrap();
        save_config(&state.config);
      }
    }

    start_config_watcher(shared.clone(), redraw_tx.clone());
    // maybe_notify_update(shared.clone());

    // Clone for returning to render scope before they're moved into the keybind handler
    let render_shared = shared.clone();
    let render_redraw_tx = RedrawSender(redraw_tx.clone());

    #[cfg(not(target_os = "macos"))]
    spawn_forever(async move {
      while let Ok(event) = keybind_rx.recv_async().await {
        match event {
          keys::KeyEvent::ToggleOverlay => {
            let mut state = app_state.write();
            state.is_open = !state.is_open;
          }
          keys::KeyEvent::CloseOverlay => {
            app_state.write().is_open = false;
          }
          keys::KeyEvent::OpenConfigurator if app_state.read().is_open => {
            open_configurator(shared.clone(), redraw_tx.clone());
          }
          keys::KeyEvent::OpenConfigurator => {}
        }
      }
    });

    (render_shared, render_redraw_tx)
  });

  // Sync is_open -> cursor hit-test, and close soundboard when overlay closes
  use_side_effect(move || {
    let is_open = app_state.read().is_open;
    if !is_open {
      soundboard_open.set(false);
      save_window_position();
    }
    window::set_clickable(is_open);
  });

  let state = app_state.read();
  let voice_users = state.voice_users.clone();
  let messages = state.messages.clone();
  let is_open = state.is_open;
  let is_censor = state.is_censor;
  let config = state.config.clone();
  let current_user = state
    .voice_users
    .iter()
    .find(|u| u.id == state.user_id)
    .cloned();
  drop(state);

  let bg_app_state = app_state;
  let mut bg_soundboard_open = soundboard_open;

  rect()
    .width(Size::fill())
    .height(Size::fill())
    // Background overlay
    .child(
      rect()
        .position(Position::new_absolute().top(0.).left(0.))
        .background(if is_open {
          colors::TRANSPARENT_GRAY
        } else {
          Color::TRANSPARENT
        })
        .width(Size::fill())
        .height(Size::fill())
        .on_press(move |_| {
          OverlayManager::close(bg_app_state);
        }),
    )
    // Soundboard backdrop (catches clicks to dismiss)
    .maybe(*bg_soundboard_open.read(), |el| {
      el.child(
        rect()
          .position(Position::new_absolute().top(0.).left(0.))
          .width(Size::fill())
          .height(Size::fill())
          .on_press(move |_| bg_soundboard_open.set(false)),
      )
    })
    // Soundboard popup
    .maybe(current_user.is_some(), |el| {
      el.child(
        rect()
          .position(Position::new_absolute().top(0.).left(0.))
          .direction(Direction::Vertical)
          .main_align(Alignment::End)
          .cross_align(Alignment::Center)
          .height(Size::percent(90_f32))
          .width(Size::fill())
          .opacity(if *soundboard_open.read() { 1.0_f32 } else { 0.0 })
          .child(Soundboard { app_state }),
      )
    })
    // Voice users
    .child(VoiceSection {
      voice_users,
      is_open,
      is_censor,
      user_alignment: config
        .user_alignment
        .clone()
        .unwrap_or_else(|| "topleft".into()),
      user_offset_x: config.user_offset_x,
      user_offset_y: config.user_offset_y,
      display_voice_members: config.display_voice_members.clone().unwrap_or_default(),
      user_row_background: config.user_row_background.clone(),
      shared: shared.clone(),
    })
    // Voice controls (top center)
    .maybe(is_open && current_user.is_some(), |el| {
      let controls_y = config
        .controls_position
        .as_deref()
        .unwrap_or("top");
      let pos = if controls_y == "bottom" {
        Position::new_absolute().bottom(50.).left(0.)
      } else {
        Position::new_absolute().top(5.).left(0.)
      };
      el.child(
        rect()
          .position(pos)
          .width(Size::fill())
          .height(Size::auto())
          .cross_align(Alignment::Center)
          .child(VoiceControls {
            user: current_user.unwrap(),
            app_state,
            soundboard_open,
            shared: shared.clone(),
            redraw_tx: redraw_tx.clone(),
          }),
      )
    })
    // Messages
    .maybe(config.enable_message_notifications, |el| {
      el.child(MessagesSection {
        messages,
        is_open,
        is_censor,
        message_alignment: config
          .message_alignment
          .clone()
          .unwrap_or_else(|| "topright".into()),
        message_offset_x: config.message_offset_x,
        message_offset_y: config.message_offset_y,
        messages_semitransparent: config.messages_semitransparent,
        app_state,
      })
    })
}
