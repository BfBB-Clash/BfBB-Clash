use bfbb::game_interface::InterfaceResult;
use clash_lib::{lobby::NetworkedLobby, net::LobbyMessage};

use crate::{gui::handle::GuiHandle, net::NetCommandSender};

// TODO: Revisit this when new unique game modes are created.
//  Idea is to allow game mode logic to be implemented by an arbitrary
//  struct with a consistent interface.
pub trait GameMode {
    fn update(&mut self, network_sender: &NetCommandSender) -> InterfaceResult<()>;

    fn message(&mut self, message: LobbyMessage, gui_sender: &mut GuiHandle);

    fn update_lobby(&mut self, new_lobby: NetworkedLobby, gui_sender: &mut GuiHandle);
}
