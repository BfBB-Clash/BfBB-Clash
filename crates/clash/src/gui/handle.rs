//! A handle for updating the GUI from other threads.
//!
//! This handle holds a copy of the GUI's [`Context`] and will
//! ensure that [`Context::request_repaint`] is called after any message is sent.

use clash_lib::{lobby::NetworkedLobby, PlayerId};
use eframe::egui::Context;

pub(super) type GuiReceiver = std::sync::mpsc::Receiver<GuiMessage>;
pub(super) type GuiSender = std::sync::mpsc::Sender<GuiMessage>;

pub enum GuiMessage {
    LocalPlayer(PlayerId),
    LobbyUpdate(NetworkedLobby),
}

impl From<PlayerId> for GuiMessage {
    fn from(id: PlayerId) -> Self {
        Self::LocalPlayer(id)
    }
}

impl From<NetworkedLobby> for GuiMessage {
    fn from(lobby: NetworkedLobby) -> Self {
        Self::LobbyUpdate(lobby)
    }
}

#[derive(Clone)]
pub struct GuiHandle {
    pub(super) context: Context,
    pub(super) sender: GuiSender,
}

#[cfg(test)]
impl GuiHandle {
    pub fn dummy() -> Self {
        let (sender, _) = std::sync::mpsc::channel();
        Self {
            context: Context::default(),
            sender,
        }
    }
}

impl GuiHandle {
    pub fn send(&mut self, msg: impl Into<GuiMessage>) {
        if let Err(e) = self.sender.send(msg.into()) {
            log::error!("Failed to send message to GUI\n{e:?}");
        }
        self.context.request_repaint();
    }
}
