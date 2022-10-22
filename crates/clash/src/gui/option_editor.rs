use eframe::{
    egui::{Checkbox, Response, TextEdit, Ui, Widget, WidgetText},
    epaint::Color32,
};

use super::val_text::ValText;

pub struct OptionEditor<'a, In, Out> {
    label: WidgetText,
    input: In,
    enabled: bool,
    on_changed: Box<dyn FnMut(Out) + 'a>,
}

impl<'a, In, Out> OptionEditor<'a, In, Out> {
    pub fn new(text: impl Into<WidgetText>, input: In, changed: impl FnMut(Out) + 'a) -> Self {
        Self {
            label: text.into(),
            input,
            enabled: true,
            on_changed: Box::new(changed),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<'a, T: Copy> Widget for OptionEditor<'a, &'a mut ValText<T>, T> {
    fn ui(mut self, ui: &mut Ui) -> eframe::egui::Response {
        ui.horizontal(|ui| {
            if !self.input.is_valid() {
                ui.style_mut().visuals.override_text_color = Some(Color32::DARK_RED);
            }
            ui.label(self.label);
            let res = ui.add_enabled(self.enabled, TextEdit::singleline(self.input));
            if res.changed() && self.input.is_valid() {
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
            if ui
                .add_enabled(self.enabled, Checkbox::new(&mut self.input, self.label))
                .changed()
            {
                (self.on_changed)(self.input);
            }
        })
        .response
    }
}

impl<'a, T: Copy> Widget for OptionEditor<'a, &'a mut [ValText<T>], (usize, T)> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        ui.collapsing(self.label, |ui| {
            for (i, input) in self.input.iter_mut().enumerate() {
                ui.add(
                    OptionEditor::new(format!("Tier {}", i + 1), input, |x| {
                        (self.on_changed)((i, x))
                    })
                    .enabled(self.enabled),
                );
            }
        })
        .header_response
    }
}
