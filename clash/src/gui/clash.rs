use std::rc::Rc;

use eframe::egui::{
    Area, Context, FontData, FontDefinitions, Label, RichText, Style, TextStyle, TopBottomPanel,
};
use eframe::epaint::{Color32, FontFamily, FontId, Pos2};
use eframe::{App, CreationContext, Frame};

use crate::gui::main_menu::MainMenu;
use crate::gui::state::State;
use crate::gui::PADDING;

pub struct Clash {
    state: Rc<State>,
    curr_app: Box<dyn App>,
    displayed_error: Option<anyhow::Error>,
}

impl Clash {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        Self::setup(&cc.egui_ctx);

        let state = Rc::new(State::new(&cc.egui_ctx));
        Self {
            curr_app: Box::new(MainMenu::new(state.clone())),
            state,
            displayed_error: None,
        }
    }

    fn setup(ctx: &Context) {
        let mut font_def = FontDefinitions::default();
        font_def.font_data.insert(
            "spongebob".into(),
            FontData::from_static(include_bytes!("../../fonts/Some.Time.Later.otf")),
        );
        font_def
            .families
            .insert(FontFamily::Proportional, vec!["spongebob".into()]);

        ctx.set_fonts(font_def);

        let mut style = Style::default();
        style.spacing.button_padding = (PADDING, PADDING).into();
        style.spacing.item_spacing = (PADDING, PADDING).into();
        style.text_styles.insert(
            TextStyle::Heading,
            FontId {
                size: 42.,
                family: FontFamily::Proportional,
            },
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId {
                size: 32.,
                family: FontFamily::Proportional,
            },
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId {
                size: 18.,
                family: FontFamily::Proportional,
            },
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId {
                size: 40.,
                family: FontFamily::Proportional,
            },
        );

        ctx.set_style(style);
    }
}

impl App for Clash {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        let style = ctx.style();
        let height = ctx.fonts().row_height(&TextStyle::Small.resolve(&style));

        // Paint this at the end so it stays on top
        let version_ui = Area::new("Version")
            // TODO: Find how to not hardcode this
            .fixed_pos(Pos2::new(560., ctx.available_rect().height() - height));

        if self.displayed_error.is_none() {
            if let Ok(e) = self.state.error_receiver.try_recv() {
                self.displayed_error = Some(e);
            }
        }
        TopBottomPanel::bottom("errors").show(ctx, |ui| {
            if let Some(e) = &self.displayed_error {
                ui.add(Label::new(
                    RichText::new(format!("Error!: {e}")).color(Color32::DARK_RED),
                ));
                if ui.button("OK").clicked() {
                    self.displayed_error = None;
                }
            }
        });

        if let Some(app) = self.state.get_new_app() {
            self.curr_app = app
        }
        self.curr_app.update(ctx, frame);

        version_ui.show(ctx, |ui| {
            ui.small(crate::VERSION);
        });
    }
}
