mod clash_game;
mod game_mode;

use bfbb::game_interface::dolphin::DolphinInterface;
use bfbb::game_interface::{InterfaceError, InterfaceProvider};
use clash_lib::net::LobbyMessage;
use clash_lib::net::Message;
use spin_sleep::LoopHelper;
use std::sync::mpsc::Receiver;
use tokio::sync::oneshot::error::TryRecvError;
use tracing::instrument;

use crate::net::NetCommandSender;
use crate::{gui::handle::GuiHandle, net::NetCommand};

use self::{clash_game::ClashGame, game_mode::GameMode};

pub type ShutdownSender = tokio::sync::oneshot::Sender<()>;
pub type ShutdownReceiver = tokio::sync::oneshot::Receiver<()>;

/// Entry point for a spectator session.
///
/// This is a temporary hack that is necessary because the GUI relies on the logic thread
/// to update it with new lobby information. Having a dedicated thread that only forwards messages
/// is certainly a bit unecessary but it doesn't really make sense to refactor the GUI to receive
/// network messages directly. The reasoning being that when we implement partial updates the GUI won't
/// know how to actually update the lobby in place from those messages.
pub fn start_spectator(
    mut gui_handle: GuiHandle,
    _network_sender: NetCommandSender,
    logic_receiver: Receiver<Message>,
    mut shutdown_receiver: ShutdownReceiver,
) {
    // Spectator client doesn't need to care about doubling BfBB's framerate
    let mut loop_helper = LoopHelper::builder().build_with_target_rate(60);

    while let Err(TryRecvError::Empty) = shutdown_receiver.try_recv() {
        loop_helper.loop_start();
        while let Ok(msg) = logic_receiver.recv() {
            match msg {
                Message::ConnectionAccept { player_id } => gui_handle.send(player_id),
                Message::GameLobbyInfo { lobby } => gui_handle.send(lobby),
                _ => continue,
            }
        }
        loop_helper.loop_sleep();
    }
}

/// Entry point for lobby logic thread.
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
    #[instrument(skip_all, name = "Logic")]
    fn update(&mut self) {
        let Some(game) = self.game.as_mut() else { return };
        match game.update(&self.network_sender) {
            Err(InterfaceError::Unhooked) => {
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
            Err(InterfaceError::ProcessNotFound | InterfaceError::EmulationNotRunning) => (),
            Err(e) => tracing::error!("{e:?}"),
            Ok(()) => (),
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
