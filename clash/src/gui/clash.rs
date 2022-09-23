use std::error::Error;
use std::rc::Rc;
use std::sync::mpsc::Receiver;

use clash_lib::lobby::NetworkedLobby;
use clash_lib::net::Message;
use clash_lib::PlayerId;
use eframe::egui::{
    Area, Context, FontData, FontDefinitions, Label, RichText, Style, TextStyle, TopBottomPanel,
};
use eframe::epaint::{Color32, FontFamily, FontId, Pos2};
use eframe::{App, CreationContext, Frame};

use crate::gui::lobby::Game;
use crate::gui::main_menu::MainMenu;
use crate::gui::state::{Screen, State};
use crate::gui::PADDING;

pub struct Clash {
    state: Rc<State>,
    game_screen: Game,
    main_menu: MainMenu,

    error_receiver: Receiver<Box<dyn Error + Send>>,
    error_queue: Vec<Box<dyn Error>>,
}

impl Clash {
    pub fn new(
        cc: &CreationContext<'_>,
        gui_receiver: Receiver<(PlayerId, NetworkedLobby)>,
        error_receiver: Receiver<Box<dyn Error + Send>>,
        network_sender: tokio::sync::mpsc::Sender<Message>,
    ) -> Self {
        Self::setup(&cc.egui_ctx);

        let state = Rc::new(State::new(&cc.egui_ctx));
        Self {
            error_receiver,
            state: state.clone(),
            game_screen: Game::new(state.clone(), gui_receiver, network_sender.clone()),
            main_menu: MainMenu::new(state, network_sender),
            error_queue: Vec::new(),
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

        while let Ok(e) = self.error_receiver.try_recv() {
            self.error_queue.push(e);
        }
        TopBottomPanel::bottom("errors").show(ctx, |ui| {
            if let Some(e) = self.error_queue.get(0) {
                ui.add(Label::new(
                    RichText::new(format!("Error!: {e}")).color(Color32::DARK_RED),
                ));
                if ui.button("OK").clicked() {
                    self.error_queue.remove(0);
                }
            }
        });

        match self.state.screen.get() {
            Screen::MainMenu(_) => self.main_menu.update(ctx, frame),
            Screen::Lobby => self.game_screen.update(ctx, frame),
        }

        version_ui.show(ctx, |ui| {
            ui.small(crate::VERSION);
        });
    }
}
