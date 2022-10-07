use eframe::{
    egui::{Response, Ui, Widget, WidgetText},
    epaint::Color32,
};

use super::{val_text::ValText, UiExt};

pub struct OptionEditor<'a, In: ?Sized, Out> {
    label: WidgetText,
    input: &'a mut In,
    on_changed: Box<dyn FnMut(&Out) + 'a>,
}

impl<'a, In: ?Sized, Out> OptionEditor<'a, In, Out> {
    pub fn new(
        text: impl Into<WidgetText>,
        input: &'a mut In,
        changed: impl FnMut(&Out) + 'a,
    ) -> Self {
        Self {
            label: text.into(),
            input,
            on_changed: Box::new(changed),
        }
    }
}

impl<'a, T> Widget for OptionEditor<'a, ValText<T>, T> {
    fn ui(mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            if !self.input.is_valid() {
                ui.style_mut().visuals.override_text_color = Some(Color32::DARK_RED);
            }
            ui.label(self.label);
            if ui.text_edit_singleline(self.input).changed() && self.input.is_valid() {
                if let Some(x) = self.input.get_val() {
                    (self.on_changed)(x);
                }
            }
        })
        .response
    }
}

impl<'a> Widget for OptionEditor<'a, bool, bool> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        ui.horizontal(|ui| {
            if ui.checkbox(self.input, self.label).changed() {
                (self.on_changed)(self.input);
            }
        })
        .response
    }
}

impl<'a, T: Copy> Widget for OptionEditor<'a, [ValText<T>], (usize, T)> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        ui.collapsing(self.label, |ui| {
            for (i, input) in self.input.iter_mut().enumerate() {
                ui.add_option(format!("Tier {}", i + 1), input, |&x| {
                    (self.on_changed)(&(i, x))
                });
            }
        })
        .header_response
    }
}
