use clash::spatula::Spatula;
use eframe::{egui::CentralPanel, epi::App, run_native, NativeOptions};
use egui::{
    Align, Color32, FontData, FontDefinitions, FontFamily, Layout, SidePanel, TopBottomPanel,
};
use strum::IntoEnumIterator;

const BORDER: f32 = 32.;
const PADDING: f32 = 8.;

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
}

impl Default for Clash {
    fn default() -> Self {
        Self {
            state: Menu::Main,
            name: Default::default(),
            lobby_id: Default::default(),
        }
    }
}

impl App for Clash {
    fn setup(
        &mut self,
        ctx: &egui::CtxRef,
        _frame: &eframe::epi::Frame,
        _storage: Option<&dyn eframe::epi::Storage>,
    ) {
        let mut font_def = FontDefinitions::default();
        font_def.font_data.insert(
            "spongebob".into(),
            // TODO: include_bytes is platform specific, this will not compile on linux.
            FontData::from_static(include_bytes!("..\\..\\fonts\\Some.Time.Later.otf")),
        );
        font_def
            .fonts_for_family
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "spongebob".into());

        font_def.family_and_size.insert(
            eframe::egui::TextStyle::Heading,
            (FontFamily::Proportional, 42.),
        );
        font_def.family_and_size.insert(
            eframe::egui::TextStyle::Body,
            (FontFamily::Proportional, 32.),
        );
        font_def.family_and_size.insert(
            eframe::egui::TextStyle::Button,
            (FontFamily::Proportional, 40.),
        );

        ctx.set_fonts(font_def);
    }

    fn update(&mut self, ctx: &eframe::egui::CtxRef, frame: &eframe::epi::Frame) {
        match self.state {
            Menu::Main => {
                CentralPanel::default().show(ctx, |ui| {
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            ui.add_space(BORDER);
                            ui.label("Battle for Bikini Bottom");
                            ui.add_space(PADDING);
                            ui.heading("CLASH!");
                        });
                    });
                    ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
                        ui.add_space(BORDER);
                        if ui.button("Quit").clicked() {
                            frame.quit();
                        }
                        ui.add_space(PADDING);
                        if ui.button("Join Game").clicked() {
                            self.state = Menu::Join;
                        }
                        ui.add_space(PADDING);
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
                    ui.horizontal(|ui| {
                        ui.label("Name: ");
                        ui.add_space(PADDING);
                        ui.text_edit_singleline(&mut self.name);
                    });
                    ui.add_space(PADDING);
                    ui.add_enabled_ui(!self.name.is_empty(), |ui| {
                        if ui.button("Host Game").clicked() {
                            self.state = Menu::Game;
                        }
                    });
                    ui.add_space(PADDING);
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
                        ui.label("Name: ");
                        ui.add_space(PADDING);
                        ui.text_edit_singleline(&mut self.name);
                    });
                    ui.add_space(PADDING);
                    ui.horizontal(|ui| {
                        ui.label("Lobby ID: ");
                        ui.add_space(PADDING);
                        ui.text_edit_singleline(&mut self.lobby_id);
                    });
                    ui.add_space(PADDING);
                    ui.add_enabled_ui(!self.name.is_empty() && !self.lobby_id.is_empty(), |ui| {
                        if ui.button("Join Game").clicked() {
                            self.state = Menu::Game;
                        }
                    });
                    ui.add_space(PADDING);
                    if ui.button("Back").clicked() {
                        self.state = Menu::Main;
                    }
                    ui.add_space(BORDER);
                });
            }
            Menu::Game => {
                SidePanel::left("Player List")
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(&self.name);
                        ui.set_min_size((100., 0.).into());
                        ui.set_max_size((100., 0.).into());
                    });
                CentralPanel::default().show(ctx, |ui| {
                    for spat in Spatula::iter() {
                        let (y, x) = spat.into();
                        ui.painter().circle_filled(
                            (x as f32 * 40. + 140., y as f32 * 40. + 20.).into(),
                            16.,
                            Color32::from_rgb(50, 50, 50),
                        )
                    }
                });
            }
        }
    }

    fn name(&self) -> &str {
        "BfBB Clash"
    }
}

pub fn run() {
    let window_options = NativeOptions {
        initial_window_size: Some((600., 720.).into()),
        resizable: false,
        ..Default::default()
    };

    run_native(Box::new(Clash::default()), window_options);
}
