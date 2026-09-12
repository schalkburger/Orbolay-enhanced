use freya::prelude::*;

#[derive(PartialEq)]
pub struct SliderControl {
  value: f64,
  min: f64,
  max: f64,
  step: f64,
  on_change: EventHandler<String>,
}

impl SliderControl {
  pub fn new(value: f64, min: f64, max: f64, step: f64, on_change: EventHandler<String>) -> Self {
    Self {
      value,
      min,
      max,
      step,
      on_change,
    }
  }
}

impl Component for SliderControl {
  fn render(&self) -> impl IntoElement {
    let min = self.min;
    let max = self.max;
    let step = self.step;
    let on_change = self.on_change.clone();

    // Map value from [min, max] to [0.0, 100.0] for Slider
    let percentage = if (max - min).abs() > f64::EPSILON {
      ((self.value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
    } else {
      0.0
    };

    let on_moved = move |per: f64| {
      // Map percentage back to value
      let raw = min + (per / 100.0) * (max - min);
      // Snap to step
      let snapped = (raw / step).round() * step;
      let clamped = snapped.clamp(min, max);
      on_change.call(format!("{:.0}", clamped));
    };

    rect()
      .direction(Direction::Horizontal)
      .cross_align(Alignment::Center)
      // .width(Size::fill())
      .width(Size::percent(60_f32))
      .child(
        label()
          .text(format!("{:.0}", self.value))
          .color(Color::WHITE)
          .font_size(12.)
          .width(Size::px(35_f32))
          .text_align(TextAlign::Left)
          .line_height(1.0)
          .padding(Gaps::new(0., 8., 0., 8.))
          .margin(Gaps::new(0., 8., 4., 8.)),
      )
      .child(Slider::new(on_moved).value(percentage))
  }
}
