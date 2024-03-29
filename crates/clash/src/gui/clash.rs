use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;

use eframe::egui::{
    self, Button, CentralPanel, Context, FontData, FontDefinitions, Label, Layout, RichText, Style,
    TextStyle, TopBottomPanel, Ui, Window,
};
use eframe::emath::Align;
use eframe::epaint::{Color32, FontFamily, FontId, Vec2};
use eframe::{App, CreationContext, Frame};
use poll_promise::Promise;
use tracing::instrument;

use crate::gui::main_menu::MainMenu;
use crate::gui::state::State;
use crate::gui::PADDING;
use crate::net;

use super::val_text::ValText;
use super::UiExt;

pub struct Clash {
    state: Rc<State>,
    settings_open: bool,
    curr_app: Box<dyn App>,
    displayed_error: Option<anyhow::Error>,
    update_task: Promise<Option<String>>,
    address: ValText<SocketAddr>,
}

impl Clash {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let url_ctx = cc.egui_ctx.clone();
        let update_task = net::spawn_promise(async move {
            let url = net::check_for_updates().await;
            url_ctx.request_repaint();
            url
        });
        Self::setup(&cc.egui_ctx);

        let state = Rc::new(State::new(&cc.egui_ctx));
        Self {
            curr_app: Box::new(MainMenu::new(state.clone())),
            settings_open: false,
            state,
            displayed_error: None,
            update_task,
            address: ValText::with_validator(|s| {
                s.to_socket_addrs().ok().and_then(|mut x| x.next())
            }),
        }
    }

    fn setup(ctx: &Context) {
        let mut font_def = FontDefinitions::default();
        font_def.font_data.insert(
            "spongebob".into(),
            FontData::from_static(include_bytes!("../../fonts/Some.Time.Later.otf")),
        );

        // Prepend our font but leave default fonts intact as fallbacks.
        font_def
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .splice(0..0, ["spongebob".into()]);

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
    #[instrument(skip_all)]
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        TopBottomPanel::bottom("toolbar")
            // Margins look better with a "group" frame
            .frame(egui::Frame::group(&ctx.style()).fill(ctx.style().visuals.window_fill()))
            .show(ctx, |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Override interior padding of button.
                    ui.style_mut().spacing.button_padding = Vec2::ZERO;

                    let settings_button =
                        Button::new(RichText::new("⛭").font(TextStyle::Small.resolve(ui.style())))
                            .frame(false)
                            .small();
                    if ui.add(settings_button).clicked() {
                        self.settings_open = !self.settings_open;
                    }

                    ui.small(crate::VERSION);
                    ui.small("Version");
                });
            });

        if self.displayed_error.is_none() {
            if let Ok(e) = self.state.error_receiver.try_recv() {
                self.displayed_error = Some(e);
            }
        }

        // Note: Closure provided to TopBottomPanel::show needs mutable access to displayed_error so we can't
        // use an if let here that binds a reference to the error here.
        if self.displayed_error.is_some() {
            TopBottomPanel::bottom("errors").show(ctx, |ui| {
                let e = self
                    .displayed_error
                    .as_ref()
                    .expect("We checked for is_some already");
                ui.add(Label::new(
                    RichText::new(format!("Error!: {e}")).color(Color32::DARK_RED),
                ));
                if ui.button("OK").clicked() {
                    self.displayed_error = None;
                }
            });
        }

        if let Some(app) = self.state.get_new_app() {
            self.curr_app = app
        }
        if self.settings_open {
            CentralPanel::default().show(ctx, |ui| self.app_settings(ui));
        } else {
            self.curr_app.update(ctx, frame);
        }

        if let Some(Some(url)) = self.update_task.ready() {
            let mut dismiss = true;
            Window::new("New Update Available")
                .collapsible(false)
                .resizable(false)
                .open(&mut dismiss)
                .show(ctx, |ui| {
                    ui.label("Please visit the download page to get the update.");
                    ui.hyperlink(url);
                });
            if !dismiss {
                self.update_task = Promise::from_ready(None);
            }
        }
    }
}

impl Clash {
    fn app_settings(&mut self, ui: &mut Ui) {
        // FIXME: This will attempt to resolve the address over the network for every character that is typed if
        //        the current string is a valid address (ip:port). ValText either needs to support validating only
        //        on loss of focus or we'll need to handle validation manually here.
        //        Additionally, this will be considered invalid if the host is temporarily unreachable
        ui.add_option("Server Address", &mut self.address, |addr| {
            *net::SERVER_ADDRESS.lock().unwrap() = addr
        });
        ui.add_option(
            "Use icons for spatula tracker",
            self.state.use_icons.get(),
            |use_icons| {
                self.state.use_icons.set(use_icons);
            },
        );
        if ui.button("Close").clicked() {
            self.settings_open = false;
        }
    }
}
