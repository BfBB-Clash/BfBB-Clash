mod clash_game;
mod game_mode;

use bfbb::game_interface::game_var::GameVarMut;
use bfbb::game_interface::{dolphin::Dolphin, InterfaceError};
use bfbb::{EnumCount, Spatula};
use clash_lib::net::LobbyMessage;
use clash_lib::{lobby::NetworkedLobby, net::Message, PlayerId};
use eframe::egui::Context;
use spin_sleep::LoopHelper;
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use tokio::sync::oneshot::error::TryRecvError;

use crate::gui::GuiSender;
use crate::net::{NetCommand, NetCommandSender};

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

    // TODO: Report hooking errors to user/stdout
    let interface = Dolphin::default();
    let mut game = ClashGame;
    // TODO: Avoid starting this thread until we have our player_id, then change our receiver to
    // take `LobbyMessage`s
    let player_id = 0;
    let lobby = None;
    // Not sure if this is the best approach, the idea is that it would be faster
    // to store a local state of what we as a client have collected rather
    // than searching to see if we've collected it.
    let local_spat_state = HashSet::<Spatula>::with_capacity(Spatula::COUNT);

    let mut logic = Logic {
        gui_ctx,
        gui_sender,
        dol: interface,
        network_sender,
        logic_receiver,
        lobby,
        player_id,
        local_spat_state,
    };

    while let Err(TryRecvError::Empty) = shutdown_receiver.try_recv() {
        loop_helper.loop_start();

        logic.update_from_network().unwrap();
        logic.update(&mut game);

        loop_helper.loop_start_s();
    }
}

struct Logic {
    gui_ctx: Context,
    gui_sender: GuiSender,
    // TODO: probably be generic over a new `GameInterfaceProvider`
    dol: Dolphin,
    network_sender: NetCommandSender,
    logic_receiver: Receiver<Message>,
    lobby: Option<NetworkedLobby>,
    player_id: PlayerId,
    local_spat_state: HashSet<Spatula>,
}

impl Logic {
    fn update<G: GameMode>(&mut self, game: &mut G) {
        let lobby = match self.lobby.as_mut() {
            Some(it) => it,
            _ => return,
        };

        let res = self.dol.with_interface(|i| {
            game.update(
                i,
                lobby,
                self.player_id,
                &mut self.network_sender,
                &mut self.local_spat_state,
            )
        });
        if let Err(InterfaceError::Unhooked) = res {
            // We lost dolphin
            if let Some(local_player) = lobby.players.get_mut(&self.player_id) {
                if local_player.current_level != None {
                    local_player.current_level = None;
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
        }
    }

    fn update_from_network(&mut self) -> Result<(), InterfaceError> {
        for msg in self.logic_receiver.try_iter() {
            let action = match msg {
                Message::ConnectionAccept { player_id: id } => {
                    self.player_id = id;
                    continue;
                }
                Message::Lobby(m) => m,
                _ => continue,
            };

            match action {
                LobbyMessage::GameBegin => {
                    self.local_spat_state.clear();
                    let _ = self.dol.with_interface(|i| i.start_new_game());
                    let lobby = self
                        .lobby
                        .as_mut()
                        .expect("Tried to begin game without being in a lobby");

                    let _ = self
                        .dol
                        .with_interface(|i| i.powers.start_with_powers(lobby.options.ng_plus));
                    self.gui_sender
                        .send((self.player_id, lobby.clone()))
                        .expect("GUI has crashed and so will we");
                }
                LobbyMessage::GameLobbyInfo { lobby: new_lobby } => {
                    // This could fail if the user is restarting dolphin, but that will desync a lot of other things as well
                    // so it's fine to just wait for a future lobby update to correct the issue
                    let _ = self.dol.with_interface(|i| {
                        i.spatula_count
                            .set(new_lobby.game_state.spatulas.len() as u32)
                    });
                    self.lobby = Some(new_lobby.clone());
                    self.gui_sender
                        .send((self.player_id, new_lobby))
                        .expect("GUI has crashed and so will we");
                }
                // We're not yet doing partial updates
                LobbyMessage::PlayerOptions { options: _ } => todo!(),
                LobbyMessage::GameOptions { options: _ } => todo!(),
                LobbyMessage::GameEnd => todo!(),
                LobbyMessage::PlayerCanStart(_) => todo!(),
                LobbyMessage::GameCurrentLevel { level: _ } => todo!(),
                LobbyMessage::GameItemCollected { item: _ } => todo!(),
            }

            // Signal the UI to update
            self.gui_ctx.request_repaint();
        }
        Ok(())
    }
}
