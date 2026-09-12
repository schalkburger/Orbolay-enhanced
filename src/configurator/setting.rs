use freya::prelude::*;

#[cfg(not(target_os = "macos"))]
use rdev::Key;

use crate::{
  configurator::{
    color_input::ColorInputControl, dropdown::DropdownControl, input::InputControl,
    slider::SliderControl, toggle::ToggleControl,
  },
  util::colors::MUTED_GRAY,
};

#[cfg(not(target_os = "macos"))]
use crate::configurator::keybind::KeybindControl;

#[derive(PartialEq)]
pub enum SettingChange {
  Value(String),
  #[cfg(not(target_os = "macos"))]
  Keys(Vec<Key>),
}

#[derive(PartialEq)]
pub struct SettingRow {
  pub name: String,
  pub description: Option<String>,
  pub kind: SettingKind,
  pub on_change: EventHandler<SettingChange>,
  pub disabled: bool,
}

#[derive(PartialEq)]
pub enum SettingKind {
  Toggle(bool),
  Dropdown(Vec<String>, Option<String>),
  Input(Option<String>),
  ColorInput(Option<String>),
  Slider {
    value: f64,
    min: f64,
    max: f64,
    step: f64,
  },
  #[cfg(not(target_os = "macos"))]
  Keybind(Option<Vec<Key>>),
}

impl Component for SettingRow {
  fn render(&self) -> impl IntoElement {
    let name = self.name.clone();
    let description = self.description.clone();

    let oc_toggle = self.on_change.clone();
    let oc_dropdown = self.on_change.clone();
    let oc_input = self.on_change.clone();
    let oc_color = self.on_change.clone();
    let oc_slider = self.on_change.clone();
    #[cfg(not(target_os = "macos"))]
    let oc_keybind = self.on_change.clone();

    let toggle_initial = match &self.kind {
      SettingKind::Toggle(b) => Some(*b),
      _ => None,
    };
    let dropdown_data = match &self.kind {
      SettingKind::Dropdown(opts, initial) => Some((opts.clone(), initial.clone())),
      _ => None,
    };
    let input_initial = match &self.kind {
      SettingKind::Input(initial) => Some(initial.clone()),
      _ => None,
    };
    let color_initial = match &self.kind {
      SettingKind::ColorInput(initial) => Some(initial.clone()),
      _ => None,
    };
    let slider_data = match &self.kind {
      SettingKind::Slider { value, min, max, step } => Some((*value, *min, *max, *step)),
      _ => None,
    };
    #[cfg(not(target_os = "macos"))]
    let keybind_initial = match &self.kind {
      SettingKind::Keybind(initial) => Some(initial.clone()),
      _ => None,
    };

    rect()
      .direction(Direction::Vertical)
      .width(Size::fill())
      .padding(Gaps::new(10., 12., 10., 12.))
      .opacity(if self.disabled { 0.4 } else { 1.0_f32 })
      .child({
        let is_color = matches!(&self.kind, SettingKind::ColorInput(_));

        let control = rect()
          .direction(if is_color { Direction::Vertical } else { Direction::Horizontal })
          .main_align(if is_color { Alignment::SpaceAround } else { Alignment::SpaceBetween })
          .cross_align(if is_color { Alignment::Start } else { Alignment::Center })
          .width(Size::fill())
          .child(label().text(name).color(Color::WHITE).font_size(12.).padding(Gaps::new(0., 0., 8., 0.)).margin(Gaps::new(0., 0., 8., 0.)))
          .map(toggle_initial, move |el, initial| {
            el.child(ToggleControl::new(
              initial,
              EventHandler::new(move |v: String| oc_toggle.call(SettingChange::Value(v))),
            ))
          })
          .map(dropdown_data, move |el, (opts, initial)| {
            el.child(DropdownControl::new(
              opts,
              initial,
              EventHandler::new(move |v: String| oc_dropdown.call(SettingChange::Value(v))),
            ))
          })
          .map(input_initial, move |el, initial| {
            el.child(InputControl::new(
              initial,
              EventHandler::new(move |v: String| oc_input.call(SettingChange::Value(v))),
            ))
          })
          .map(slider_data, move |el, (value, min, max, step)| {
            el.child(SliderControl::new(
              value,
              min,
              max,
              step,
              EventHandler::new(move |v: String| oc_slider.call(SettingChange::Value(v))),
            ))
          })
          .map(color_initial, move |el, initial| {
            el.margin(Gaps::new(0., 0., 4., 0.))
              .child(ColorInputControl::new(
                initial,
                EventHandler::new(move |v: String| oc_color.call(SettingChange::Value(v))),
              ))
          });

        #[cfg(not(target_os = "macos"))]
        let control = control.map(keybind_initial, move |el, initial| {
          el.child(KeybindControl::new(
            initial,
            EventHandler::new(move |keys: Vec<Key>| oc_keybind.call(SettingChange::Keys(keys))),
          ))
        });

        control
      })
      .map(description, |el, desc| {
        el.child(
          label()
            .text(desc)
            .color(MUTED_GRAY)
            .font_size(12.)
            .margin(Gaps::new(4., 0., 0., 0.)),
        )
      })
  }
}
