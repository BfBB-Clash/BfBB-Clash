mod game_menu;
mod player_widget;

use self::{game_menu::GameMenu, player_widget::PlayerUi};
use crate::game::GameState;
use clash::{room::Room, spatula::Spatula};
use eframe::egui::{
    Align, Color32, Context, FontData, FontDefinitions, FontFamily, Layout, SidePanel, Style,
    TextEdit, TextStyle, TopBottomPanel,
};
use eframe::epaint::FontId;
use eframe::epi::{Frame, Storage};
use eframe::{egui::CentralPanel, epi::App, run_native, NativeOptions};
use std::sync::mpsc::Receiver;

const BORDER: f32 = 32.;
const PADDING: f32 = 8.;

pub enum GuiMessage {
    Spatula(Spatula),
    Room(Option<Room>),
}

pub enum Menu {
    Main,
    Host,
    Join,
    Game,
}

pub struct Clash {
    state: Menu,
    name: String,
    lobby_id: String,
    game_state: GameState,
    receiver: Receiver<GuiMessage>,
}

impl Clash {
    fn new(receiver: Receiver<GuiMessage>) -> Self {
        Self {
            state: Menu::Main,
            name: Default::default(),
            lobby_id: Default::default(),
            game_state: GameState::default(),
            receiver,
        }
    }

    fn process_messages(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message {
                GuiMessage::Spatula(s) => {
                    self.game_state.spatulas.insert(s, None);
                }
                GuiMessage::Room(r) => {
                    self.game_state.current_room = r;
                }
            }
        }
    }
}

impl App for Clash {
    fn setup(&mut self, ctx: &Context, _frame: &Frame, _storage: Option<&dyn Storage>) {
        let mut font_def = FontDefinitions::default();
        font_def.font_data.insert(
            "spongebob".into(),
            // TODO: include_bytes is platform specific, this will not compile on linux.
            FontData::from_static(include_bytes!("..\\..\\fonts\\Some.Time.Later.otf")),
        );
        font_def
            .families
            .insert(FontFamily::Proportional, vec!["spongebob".into()]);

        ctx.set_fonts(font_def);

        //
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

    fn update(&mut self, ctx: &Context, frame: &Frame) {
        match self.state {
            Menu::Main => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            ui.add_space(BORDER);
                            ui.label("Battle for Bikini Bottom");
                            ui.heading("CLASH!");
                        });
                    });
                    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                        ui.add_space(BORDER);
                        if ui.button("Quit").clicked() {
                            frame.quit();
                        }
                        if ui.button("Join Game").clicked() {
                            self.state = Menu::Join;
                        }
                        if ui.button("Host Game").clicked() {
                            self.state = Menu::Host;
                        }
                    });
                });
            }
            Menu::Host => {
                TopBottomPanel::top("Title").show(ctx, |ui| {
                    ui.vertical_centered(|ui| ui.label("Host Game"));
                });
                TopBottomPanel::bottom("Join Panel").show(ctx, |ui| {
                    ui.add(TextEdit::singleline(&mut self.name).hint_text("Name"));
                    ui.add_enabled_ui(!self.name.is_empty(), |ui| {
                        if ui.button("Host Game").clicked() {
                            self.state = Menu::Game;
                        }
                    });
                    if ui.button("Back").clicked() {
                        self.state = Menu::Main;
                    }
                    ui.add_space(BORDER);
                });
            }
            Menu::Join => {
                TopBottomPanel::top("Title").show(ctx, |ui| {
                    ui.label("Join Game");
                });
                TopBottomPanel::bottom("Host Panel").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut self.name).hint_text("Name"));
                    });
                    ui.horizontal(|ui| {
                        ui.add(TextEdit::singleline(&mut self.lobby_id).hint_text("Lobby ID"));
                    });
                    ui.add_enabled_ui(!self.name.is_empty() && !self.lobby_id.is_empty(), |ui| {
                        if ui.button("Join Game").clicked() {
                            self.state = Menu::Game;
                        }
                    });
                    if ui.button("Back").clicked() {
                        self.state = Menu::Main;
                    }
                    ui.add_space(BORDER);
                });
            }
            Menu::Game => {
                // Continuously repaint
                ctx.request_repaint();

                // Receive gamestate updates
                self.process_messages();

                SidePanel::left("Player List")
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.add_space(PADDING);
                        ui.add(PlayerUi::new(
                            self.name.as_str(),
                            self.game_state.spatulas.len() as u32,
                            self.game_state.current_room,
                            Color32::from_rgb(100, 120, 180),
                        ));
                        ui.add(PlayerUi::new(
                            "Not Square",
                            4,
                            None,
                            Color32::from_rgb(180, 100, 120),
                        ));
                        ui.add(PlayerUi::new(
                            "Not Square",
                            4,
                            None,
                            Color32::from_rgb(180, 100, 120),
                        ));
                        ui.add(PlayerUi::new(
                            "Not Square",
                            4,
                            None,
                            Color32::from_rgb(180, 100, 120),
                        ));
                        ui.add(PlayerUi::new(
                            "Not Square",
                            4,
                            None,
                            Color32::from_rgb(180, 100, 120),
                        ));
                        ui.add(PlayerUi::new(
                            "Not Square",
                            4,
                            None,
                            Color32::from_rgb(180, 100, 120),
                        ));
                    });
                CentralPanel::default().show(ctx, |ui| {
                    ui.add(GameMenu::new(&self.game_state));
                });
            }
        }
    }

    fn name(&self) -> &str {
        "BfBB Clash"
    }
}

pub fn run(gui_receiver: Receiver<GuiMessage>) {
    let window_options = NativeOptions {
        initial_window_size: Some((600., 720.).into()),
        resizable: true,
        ..Default::default()
    };

    run_native(Box::new(Clash::new(gui_receiver)), window_options);
}
