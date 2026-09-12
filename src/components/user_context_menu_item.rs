use freya::prelude::*;
use serde_json::json;

use crate::{
  app_state::SharedAppState,
  user::User,
  util::{
    bridge::BridgeMessage,
    colors::{self},
  },
};

pub struct UserContextMenuItem {
  pub user: User,
  pub shared: SharedAppState,
}

impl PartialEq for UserContextMenuItem {
  fn eq(&self, other: &Self) -> bool {
    self.user == other.user
  }
}

impl Component for UserContextMenuItem {
  fn render(&self) -> impl IntoElement {
    let shared = self.shared.clone();
    let user_id = self.user.id.clone();
    let mut slider_value = use_state(|| f64::from(self.user.volume.clamp(0., 200.) / 2.));

    let menu_item_theme = MenuItemThemePartial {
      background: Some(Preference::Specific(colors::DARKISH_GRAY)),
      hover_background: Some(Preference::Specific(colors::DARKISH_GRAY)),
      select_background: Some(Preference::Specific(colors::DARKISH_GRAY)),
      border_fill: Some(Preference::Specific(Color::TRANSPARENT)),
      select_border_fill: Some(Preference::Specific(Color::TRANSPARENT)),
      corner_radius: Some(Preference::Specific(CornerRadius::new_all(8.))),
      color: Some(Preference::Specific(colors::SUPERLIGHT_GRAY)),
    };

    let slider_theme = SliderThemePartial {
      background: Some(Preference::Specific(colors::MUTED_GRAY)),
      thumb_background: Some(Preference::Specific(colors::SUPERLIGHT_GRAY)),
      thumb_inner_background: Some(Preference::Specific(colors::SUPERLIGHT_GRAY)),
      border_fill: Some(Preference::Specific(colors::GRAY)),
    };

    MenuItem::new()
      .theme(menu_item_theme)
      .padding(Gaps::new_all(4.))
      .child(
        rect()
          .direction(Direction::Vertical)
          .cross_align(Alignment::Start)
          .width(Size::px(160_f32))
          .child(
            rect()
              .direction(Direction::Horizontal)
              .main_align(Alignment::SpaceBetween)
              .cross_align(Alignment::Center)
              .width(Size::fill())
              .margin(Gaps::new(0., 0., 8., 0.))
              .child(
                label()
                  .font_size(12.)
                  .color(colors::SUPERLIGHT_GRAY)
                  .text("User Volume"),
              )
              .child(
                label()
                  .font_size(12.)
                  .color(colors::SUPERLIGHT_GRAY)
                  .text(format!("{}%", slider_value.read().round() as u64 * 2)),
              ),
          )
          .child(
            Slider::new(move |value: f64| {
              let volume = (value * 2.0).clamp(0.0, 200.0);
              slider_value.set(value);
              shared.write().unwrap().send(BridgeMessage {
                cmd: "SET_USER_VOLUME".to_string(),
                data: json!({
                  "user_id": user_id,
                  "volume": volume,
                }),
              });
            })
            .theme(slider_theme)
            .value(*slider_value.read())
            .size(Size::fill()),
          ),
      )
  }
}
