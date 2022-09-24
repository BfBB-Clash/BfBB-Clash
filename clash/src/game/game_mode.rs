use std::collections::HashSet;

use bfbb::{game_interface::GameInterface, Spatula};
use clash_lib::{lobby::NetworkedLobby, PlayerId};

use crate::net::NetCommandSender;

// TODO: Revisit this when new unique game modes are created.
//  Idea is to allow game mode logic to be implemented by an arbitrary
//  struct with a consistent interface.
pub trait GameMode {
    type Result;

    fn update<G: GameInterface>(
        &mut self,
        interface: &G,
        lobby: &mut NetworkedLobby,
        local_player: PlayerId,
        network_sender: &mut NetCommandSender,
        local_spat_state: &mut HashSet<Spatula>,
    ) -> Self::Result;
}
