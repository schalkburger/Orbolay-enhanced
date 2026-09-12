use freya::prelude::*;

use crate::util::colors::{parse_hex, to_argb_hex, TRANSPARENT};

#[derive(PartialEq)]
pub struct ColorInputControl {
  initial: Option<String>,
  on_change: EventHandler<String>,
}

impl ColorInputControl {
  pub fn new(initial: Option<String>, on_change: EventHandler<String>) -> Self {
    Self { initial, on_change }
  }
}

impl Component for ColorInputControl {
  fn render(&self) -> impl IntoElement {
    let on_change = self.on_change.clone();
    let initial_color = self
      .initial
      .as_deref()
      .and_then(parse_hex)
      .unwrap_or(TRANSPARENT);
    let mut color = use_state(|| initial_color);

    let current = color();
    ColorPicker::new(move |c| {
      color.set(c);
      on_change.call(to_argb_hex(c));
    })
    .value(current)
    .width(Size::px(180_f32))
  }
}
