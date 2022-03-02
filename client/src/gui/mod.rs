mod game_menu;
mod player_widget;

use self::{game_menu::GameMenu, player_widget::PlayerUi};
use crate::game::GameState;
use clash::{room::Room, spatula::Spatula};
use std::sync::mpsc::Receiver;

use eframe::egui::{
    Align, Button, CentralPanel, Checkbox, Color32, Context, FontData, FontDefinitions, FontFamily,
    Layout, SidePanel, Style, TextEdit, TextStyle, TopBottomPanel, Ui,
};
use eframe::epaint::FontId;
use eframe::epi::{App, Frame, Storage};
use eframe::{run_native, NativeOptions};

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
    lab_door_string: String,
    lab_door_num: Option<u8>,
    game_active: bool,
    game_state: GameState,
    receiver: Receiver<GuiMessage>,
}

impl Clash {
    fn new(receiver: Receiver<GuiMessage>) -> Self {
        Self {
            state: Menu::Main,
            name: Default::default(),
            lobby_id: Default::default(),
            lab_door_string: Default::default(),
            lab_door_num: None,
            game_active: false,
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

    fn paint_options(&mut self, ui: &mut Ui) {
        ui.heading("Lobby Options");
        ui.separator();

        ui.add(Checkbox::new(
            &mut self.game_state.options.ng_plus,
            "New Game+",
        ))
        .on_hover_text(
            "All players start the game with the Bubble Bowl and Cruise Missile unlocked.",
        );

        if ui
            .horizontal(|ui| {
                if self.lab_door_num.is_none() {
                    ui.style_mut().visuals.override_text_color = Some(Color32::DARK_RED);
                }
                ui.label("Lab Door Cost: ");
                ui.text_edit_singleline(&mut self.lab_door_string)
            })
            .inner
            .changed()
        {
            // Validate input
            self.lab_door_num = self
                .lab_door_string
                .parse::<u8>()
                .ok()
                .filter(|&n| n > 0 && n <= 82);
        }

        let mut start_game_response = ui
            .add_enabled(
                self.game_state.can_start() && self.lab_door_num.is_some(),
                Button::new("Start Game"),
            )
            .on_hover_text("Starts a new game for all connected players.");

        // We unfortunately have to check these conditions twice since we need the Response to add the
        // tooltips but need to enable/disable the button before we can get the response
        if !self.game_state.can_start() {
            start_game_response =
                start_game_response.on_disabled_hover_text("All players must be on the Main Menu.")
        }

        if self.lab_door_num.is_none() {
            start_game_response = start_game_response
                .on_disabled_hover_text("'Lab Door Cost' must be a number from 1-82");
        }

        if start_game_response.clicked() {
            // TODO: Send a message to the network thread to start the game.
            self.game_active = true;
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
                    if self.game_active {
                        ui.add(GameMenu::new(&self.game_state));
                    } else {
                        // TODO: Restrict these to the host
                        self.paint_options(ui);
                    }
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
