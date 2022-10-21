mod clash_game;
mod game_mode;

use bfbb::game_interface::InterfaceError;
use clash_lib::net::LobbyMessage;
use clash_lib::{lobby::NetworkedLobby, net::Message, PlayerId};
use eframe::egui::Context;
use spin_sleep::LoopHelper;
use std::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot::error::TryRecvError;

use crate::net::NetCommandSender;
use crate::{gui::GuiSender, net::NetCommand};

use self::{clash_game::ClashGame, game_mode::GameMode};

pub type ShutdownSender = tokio::sync::oneshot::Sender<()>;
pub type ShutdownReceiver = tokio::sync::oneshot::Receiver<()>;

pub fn start_game(
    gui_sender: Sender<(PlayerId, NetworkedLobby)>,
    gui_ctx: eframe::egui::Context,
    network_sender: NetCommandSender,
    logic_receiver: Receiver<Message>,
    mut shutdown_receiver: ShutdownReceiver,
) {
    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);

    let mut logic = Logic {
        gui_ctx,
        gui_sender,
        network_sender,
        logic_receiver,
        game: None,
    };

    while let Err(TryRecvError::Empty) = shutdown_receiver.try_recv() {
        loop_helper.loop_start();

        logic.update_from_network().unwrap();
        logic.update();

        loop_helper.loop_sleep();
    }
}

struct Logic {
    gui_ctx: Context,
    gui_sender: GuiSender,
    network_sender: NetCommandSender,
    logic_receiver: Receiver<Message>,
    game: Option<ClashGame>,
}

impl Logic {
    fn update(&mut self) {
        let game = match self.game.as_mut() {
            Some(it) => it,
            _ => return,
        };
        // TODO: Log non-hooking errors
        if let Err(InterfaceError::Unhooked) = game.update(&self.network_sender) {
            // We lost dolphin
            // Our local state will be updated when the client accepts this message and responds.
            self.network_sender
                .try_send(NetCommand::Send(Message::Lobby(
                    LobbyMessage::GameCurrentLevel { level: None },
                )))
                .unwrap();
            self.network_sender
                .try_send(NetCommand::Send(Message::Lobby(
                    LobbyMessage::PlayerCanStart(false),
                )))
                .unwrap();
        }
    }

    fn update_from_network(&mut self) -> Result<(), InterfaceError> {
        for msg in self.logic_receiver.try_iter() {
            let action = match msg {
                Message::ConnectionAccept { player_id } => {
                    self.game = Some(ClashGame::new(player_id));
                    continue;
                }
                Message::Lobby(m) => m,
                _ => continue,
            };

            self.game
                .as_mut()
                .expect("Tried to process a LobbyAction without having a gamemode setup")
                .message(action, &mut self.gui_sender);

            // Signal the UI to update
            self.gui_ctx.request_repaint();
        }
        Ok(())
    }
}
