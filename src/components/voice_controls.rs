use freya::prelude::*;
use serde_json::Value;

use crate::{
  app_state::{AppState, SharedAppState},
  config::TransportMode,
  user::{User, UserVoiceState},
  util::{bridge::BridgeMessage, colors},
};

static DEAFENED_SVG: &[u8] = include_bytes!("../../assets/deafened.svg");
static DEAFEN_SVG: &[u8] = include_bytes!("../../assets/deafen.svg");
static MUTED_SVG: &[u8] = include_bytes!("../../assets/muted.svg");
static MUTE_SVG: &[u8] = include_bytes!("../../assets/mute.svg");
static DISCONNECT_SVG: &[u8] = include_bytes!("../../assets/disconnect.svg");
static STOP_STREAM_SVG: &[u8] = include_bytes!("../../assets/stopstream.svg");
static SOUNDBOARD_SVG: &[u8] = include_bytes!("../../assets/speaker.svg");
static SETTINGS_SVG: &[u8] = include_bytes!("../../assets/settings.svg");

#[derive(Clone)]
pub struct RedrawSender(pub flume::Sender<()>);

impl PartialEq for RedrawSender {
  fn eq(&self, other: &Self) -> bool {
    std::ptr::eq(&self.0 as *const _, &other.0 as *const _)
  }
}

#[derive(PartialEq)]
struct ControlButton {
  icon: &'static [u8],
  is_red: bool,
  on_click: EventHandler<()>,
}

impl Component for ControlButton {
  fn render(&self) -> impl IntoElement {
    let mut hovered = use_state(|| false);
    let is_red = self.is_red;
    let icon = self.icon;
    let on_click = self.on_click.clone();

    use_drop(move || {
      if *hovered.read() {
        Cursor::set(CursorIcon::default());
      }
    });

    rect()
      .direction(Direction::Vertical)
      .main_align(Alignment::Center)
      .cross_align(Alignment::Center)
      .height(Size::fill())
      .width(Size::percent(20_f32))
      .margin(Gaps::new(2., 2., 2., 2.))
      .padding(Gaps::new_all(4.))
      .corner_radius(CornerRadius::new_all(6.))
      .background(if *hovered.read() {
        if is_red {
          colors::RED_GRAY
        } else {
          colors::LIGHT_GRAY
        }
      } else {
        Color::TRANSPARENT
      })
      .on_press(move |_| on_click.call(()))
      .on_pointer_enter(move |_| {
        *hovered.write() = true;
        Cursor::set(CursorIcon::Pointer);
      })
      .on_pointer_leave(move |_| {
        *hovered.write() = false;
        Cursor::set(CursorIcon::default());
      })
      .child(svg(icon).width(Size::px(18_f32)).height(Size::px(18_f32)))
  }
}

pub struct VoiceControls {
  pub user: User,
  pub app_state: State<AppState>,
  pub soundboard_open: State<bool>,
  pub shared: SharedAppState,
  pub redraw_tx: RedrawSender,
}

impl PartialEq for VoiceControls {
  fn eq(&self, other: &Self) -> bool {
    self.user == other.user
  }
}

impl Component for VoiceControls {
  fn render(&self) -> impl IntoElement {
    let mut app_state = self.app_state;
    let mut soundboard_open = self.soundboard_open;
    let shared = self.shared.clone();
    let redraw_tx = self.redraw_tx.clone();
    let is_muted = self.user.voice_state == UserVoiceState::Muted;
    let is_deafened = self.user.voice_state == UserVoiceState::Deafened;
    let is_streaming = self.user.streaming;

    rect()
      .direction(Direction::Horizontal)
      .main_align(Alignment::Center)
      .cross_align(Alignment::Center)
      .height(Size::auto())
      .max_height(Size::px(40_f32))
      .max_width(Size::px(300_f32))
      .background(colors::DARKER_GRAY)
      .corner_radius(CornerRadius::new_all(8.))
      .child(ControlButton {
        icon: if is_muted || is_deafened {
          MUTED_SVG
        } else {
          MUTE_SVG
        },
        is_red: is_muted || is_deafened,
        on_click: (move |()| {
          app_state.write().send(BridgeMessage {
            cmd: "TOGGLE_MUTE".to_string(),
            data: Value::Null,
          })
        })
        .into(),
      })
      .child(ControlButton {
        icon: if is_deafened {
          DEAFENED_SVG
        } else {
          DEAFEN_SVG
        },
        is_red: is_deafened,
        on_click: (move |()| {
          app_state.write().send(BridgeMessage {
            cmd: "TOGGLE_DEAF".to_string(),
            data: Value::Null,
          })
        })
        .into(),
      })
      .maybe(
        app_state.read().transport_mode == TransportMode::Ipc,
        |el| {
          el.child(ControlButton {
            icon: SOUNDBOARD_SVG,
            is_red: false,
            on_click: (move |()| {
              let is_open = *soundboard_open.read();
              soundboard_open.set(!is_open);
            })
            .into(),
          })
        },
      )
      .child(ControlButton {
        icon: DISCONNECT_SVG,
        is_red: true,
        on_click: (move |()| {
          app_state.write().send(BridgeMessage {
            cmd: "DISCONNECT".to_string(),
            data: Value::Null,
          })
        })
        .into(),
      })
      .maybe(is_streaming, |el| {
        el.child(ControlButton {
          icon: STOP_STREAM_SVG,
          is_red: true,
          on_click: (move |()| {
            app_state.write().send(BridgeMessage {
              cmd: "STOP_STREAM".to_string(),
              data: Value::Null,
            })
          })
          .into(),
        })
      })
      .child(ControlButton {
        icon: SETTINGS_SVG,
        is_red: false,
        on_click: (move |()| {
          crate::configurator::open_configurator(shared.clone(), redraw_tx.0.clone());
        })
        .into(),
      })
  }
}
