use freya::engine::prelude::SkColor;
use freya::prelude::*;

use crate::{
  app_state::SharedAppState,
  components::user_context_menu_item::UserContextMenuItem,
  config::save_config,
  user::{User, UserVoiceState},
  util::{
    colors::{self, parse_hex},
    image::avatar_image,
  },
};

static DEAFENED_SVG: &[u8] = include_bytes!("../../assets/deafened.svg");
static MUTED_SVG: &[u8] = include_bytes!("../../assets/muted.svg");
static STREAMING_SVG: &[u8] = include_bytes!("../../assets/streaming.svg");
static CAMERA_SVG: &[u8] = include_bytes!("../../assets/camera.svg");

#[derive(PartialEq)]
struct AvatarIcon {
  user: User,
}

impl Component for AvatarIcon {
  fn render(&self) -> impl IntoElement {
    let (url, border) = avatar_url_and_border(&self.user);
    rect()
      .width(Size::px(35_f32))
      .height(Size::px(35_f32))
      .margin(Gaps::new(0., 6., 0., 8.))
      .corner_radius(CornerRadius::new_all(25.))
      .child(
        avatar_image(&url, border)
          .width(Size::fill())
          .height(Size::fill()),
      )
  }
}

struct UserLabel {
  user: User,
  background: Option<String>,
  is_speaking: bool,
}

impl PartialEq for UserLabel {
  fn eq(&self, other: &Self) -> bool {
    self.user == other.user && self.background == other.background && self.is_speaking == other.is_speaking
  }
}

impl Component for UserLabel {
  fn render(&self) -> impl IntoElement {
    let user = &self.user;
    let is_muted = user.voice_state == UserVoiceState::Muted;
    let is_deafened = user.voice_state == UserVoiceState::Deafened;

    rect()
      .direction(Direction::Horizontal)
      .main_align(Alignment::Center)
      .cross_align(Alignment::Center)
      .height(Size::percent(70_f32))
      .background(
        self
          .background
          .as_deref()
          .and_then(parse_hex)
          .unwrap_or(colors::DARKISH_BLUE),
      )
      .corner_radius(CornerRadius::new_all(8.))
      .margin(Gaps::new(0., 6., 0., 6.))
      .border(Border::new().fill(if self.is_speaking { colors::GREEN } else { colors::TRANSPARENT }).width(1.))
      .child(
        rect().padding(Gaps::new_all(4.)).child(
          label()
            .font_size(12.)
            .color(Color::WHITE)
            .padding(Gaps::new(10., 12., 10., 12.))
            .margin(Gaps::new(0., 6., 0., 6.))
            .text(user.name.clone()),
        ),
      )
      .maybe(is_muted, |el| {
        el.child(
          svg(MUTED_SVG)
            .width(Size::px(16_f32))
            .height(Size::px(16_f32))
            .margin(Gaps::new(0., 6., 0., 2.)),
        )
      })
      .maybe(is_deafened, |el| {
        el.child(
          svg(DEAFENED_SVG)
            .width(Size::px(16_f32))
            .height(Size::px(16_f32))
            .margin(Gaps::new(0., 6., 0., 2.)),
        )
      })
      .maybe(user.streaming, |el| {
        el.child(
          svg(STREAMING_SVG)
            .width(Size::px(16_f32))
            .height(Size::px(16_f32))
            .margin(Gaps::new(0., 6., 0., 0.)),
        )
      })
      .maybe(user.camera, |el| {
        el.child(
          svg(CAMERA_SVG)
            .width(Size::px(16_f32))
            .height(Size::px(16_f32))
            .margin(Gaps::new(0., 6., 0., 0.)),
        )
      })
  }
}

pub struct UserRow {
  pub user: User,
  pub is_right_aligned: bool,
  pub is_open: bool,
  pub is_voice_semitransparent: bool,
  pub can_context_menu: bool,
  pub background: Option<String>,
  pub shared: SharedAppState,
  pub x_mult: i32,
  pub y_mult: i32,
}

impl PartialEq for UserRow {
  fn eq(&self, other: &Self) -> bool {
    self.user == other.user
      && self.is_right_aligned == other.is_right_aligned
      && self.is_open == other.is_open
      && self.is_voice_semitransparent == other.is_voice_semitransparent
      && self.can_context_menu == other.can_context_menu
      && self.background == other.background
  }
}

impl Component for UserRow {
  fn render(&self) -> impl IntoElement {
    let is_right_aligned = self.is_right_aligned;
    let is_speaking = self.user.voice_state == UserVoiceState::Speaking;

    let opacity = if !is_speaking && (self.is_voice_semitransparent && !self.is_open) {
      0.5_f32
    } else {
      1.0
    };

    let label = UserLabel {
      user: self.user.clone(),
      background: self.background.clone(),
      is_speaking,
    };
    let icon = AvatarIcon {
      user: self.user.clone(),
    };

    let shared_down = self.shared.clone();
    let shared_move = self.shared.clone();
    let x_mult = self.x_mult;
    let y_mult = self.y_mult;

    let drag_state: State<Option<(f64, f64, i32, i32)>> = use_state(|| None);
    let mut drag_down = drag_state;
    let drag_move = drag_state;
    let mut drag_press = drag_state;

    let row = rect()
      .direction(Direction::Horizontal)
      .main_align(if is_right_aligned { Alignment::End } else { Alignment::Start })
      .cross_align(Alignment::Center)
      .width(Size::px(200_f32))
      //  .width(Size::fill())
      .height(Size::px(50_f32))
      .padding(Gaps::new_all(0.3))
      .margin(Gaps::new(2.0, 0.0, 2.0, 2.0))
      .corner_radius(CornerRadius::new_all(6.))
      .background(colors::TRANSPARENT)
      .opacity(opacity)
      .maybe(self.can_context_menu, |el| {
        let user = self.user.clone();
        let shared = self.shared.clone();
        el.on_secondary_down(move |e: Event<PressEventData>| {
          ContextMenu::open_from_event(
            &e,
            Menu::new()
              .theme(MenuContainerThemePartial {
                background: Some(Preference::Specific(colors::DARKISH_GRAY)),
                padding: Some(Preference::Specific(Gaps::new_all(6.))),
                shadow: Some(Preference::Specific(colors::TRANSPARENT_GRAY)),
                border_fill: Some(Preference::Specific(colors::MUTED_GRAY)),
                corner_radius: Some(Preference::Specific(CornerRadius::new_all(8.))),
              })
              .child(UserContextMenuItem {
                user: user.clone(),
                shared: shared.clone(),
              }),
          );
        })
      })
      .on_pointer_down(move |e: Event<PointerEventData>| {
        let location = e.global_location();
        let state = shared_down.read().unwrap();
        let init_x = state.config.user_offset_x;
        let init_y = state.config.user_offset_y;
        drop(state);
        *drag_down.write() = Some((location.x, location.y, init_x, init_y));
      })
      .on_global_pointer_move(move |e: Event<PointerEventData>| {
        if let Some((start_x, start_y, init_x, init_y)) = *drag_move.read() {
          let location = e.global_location();
          let dx = ((location.x - start_x) as i32) * x_mult;
          let dy = ((location.y - start_y) as i32) * y_mult;
          let new_x = init_x + dx;
          let new_y = init_y + dy;
          let mut state = shared_move.write().unwrap();
          state.config.user_offset_x = new_x;
          state.config.user_offset_y = new_y;
          let config = state.config.clone();
          drop(state);
          save_config(&config);
        }
      })
      .on_global_pointer_press(move |_e: Event<PointerEventData>| {
        *drag_press.write() = None;
      });

    if is_right_aligned {
      row.child(label).child(icon)
    } else {
      row.child(icon).child(label)
    }
  }
}

fn avatar_url_and_border(user: &User) -> (String, Option<SkColor>) {
  let border_color = match user.voice_state {
    UserVoiceState::Speaking => Some(SkColor::from_rgb(47, 186, 139)),
    UserVoiceState::Deafened | UserVoiceState::Muted => Some(SkColor::from_rgb(218, 62, 68)),
    _ => None,
  };

  let url = if user.avatar.is_empty() {
    String::new()
  } else {
    format!(
      "https://cdn.discordapp.com/avatars/{}/{}.png?size=80",
      user.id, user.avatar
    )
  };

  (url, border_color)
}
