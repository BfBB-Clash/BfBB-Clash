mod clash_game;
mod game_mode;

use bfbb::game_interface::dolphin::DolphinInterface;
use bfbb::game_interface::{InterfaceError, InterfaceProvider};
use clash_lib::net::LobbyMessage;
use clash_lib::net::Message;
use spin_sleep::LoopHelper;
use std::sync::mpsc::Receiver;
use tokio::sync::oneshot::error::TryRecvError;

use crate::net::NetCommandSender;
use crate::{gui::handle::GuiHandle, net::NetCommand};

use self::{clash_game::ClashGame, game_mode::GameMode};

pub type ShutdownSender = tokio::sync::oneshot::Sender<()>;
pub type ShutdownReceiver = tokio::sync::oneshot::Receiver<()>;

pub fn start_game(
    gui_handle: GuiHandle,
    network_sender: NetCommandSender,
    logic_receiver: Receiver<Message>,
    mut shutdown_receiver: ShutdownReceiver,
) {
    let mut loop_helper = LoopHelper::builder()
        .report_interval_s(0.5)
        .build_with_target_rate(126);

    let mut logic: Logic<DolphinInterface> = Logic {
        gui_handle,
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

struct Logic<I> {
    gui_handle: GuiHandle,
    network_sender: NetCommandSender,
    logic_receiver: Receiver<Message>,
    game: Option<ClashGame<I>>,
}

impl<I: InterfaceProvider> Logic<I> {
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
                    self.game = Some(ClashGame::new(I::default(), player_id));
                    self.gui_handle.send(player_id);
                    continue;
                }
                Message::GameLobbyInfo { lobby } => {
                    if let Some(g) = self.game.as_mut() {
                        g.update_lobby(lobby, &mut self.gui_handle);
                    }
                    continue;
                }
                Message::Lobby(m) => m,
                _ => continue,
            };

            self.game
                .as_mut()
                .expect("Tried to process a LobbyAction without having a gamemode setup")
                .message(action, &mut self.gui_handle);
        }
        Ok(())
    }
}
