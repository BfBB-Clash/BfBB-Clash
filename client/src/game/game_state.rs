use crate::game::{GameInterface, InterfaceResult};
use crate::gui::GuiMessage;
use clash::game_state::GameState;
use clash::{
    protocol::{Item, Message},
    room::Room,
    spatula::Spatula,
};
use log::info;
use std::sync::mpsc::{Receiver, Sender};
use strum::IntoEnumIterator;

pub trait GameStateExt {
    fn update<T: GameInterface>(
        &mut self,
        game: &T,
        gui_sender: &mut Sender<GuiMessage>,
        _network_sender: &mut tokio::sync::mpsc::Sender<Message>,
        logic_receiver: &mut Receiver<Message>,
    ) -> InterfaceResult<()>;

    fn can_start(&self) -> bool;
}

impl GameStateExt for GameState {
    /// Process state updates from the server and report back any actions of the local player
    fn update<T: GameInterface>(
        &mut self,
        game: &T,
        gui_sender: &mut Sender<GuiMessage>,
        _network_sender: &mut tokio::sync::mpsc::Sender<Message>,
        logic_receiver: &mut Receiver<Message>,
    ) -> InterfaceResult<()> {
        while let Ok(m) = logic_receiver.try_recv() {
            if let Message::GameItemCollected {
                auth_id: _,
                item: Item::Spatula(spat),
            } = m
            {
                self.spatulas.insert(spat, None);
                game.mark_task_complete(spat)?;
                let _ = gui_sender.send(GuiMessage::Spatula(spat));
                info!("Collected {spat:?}");
            }
        }
        if game.is_loading()? {
            return Ok(());
        }

        // Set the cost to unlock the lab door
        let room = Some(game.get_current_level()?);
        if self.current_room != room {
            self.current_room = room;
            let _ = gui_sender.send(GuiMessage::Room(room));
        }
        if self.current_room == Some(Room::ChumBucket) {
            game.set_lab_door(self.lobby.options.lab_door_cost.into())?;
        }

        // Check for newly collected spatulas
        for spat in Spatula::iter() {
            // Skip already collected spatulas
            if self.spatulas.contains_key(&spat) {
                continue;
            }

            // Check menu for any potentially missed collection events
            if game.is_task_complete(spat)? {
                self.spatulas.insert(spat, None);
                let _ = gui_sender.send(GuiMessage::Spatula(spat));
                info!("Collected (from menu) {spat:?}");
            }

            // Skip spatulas that aren't in the current room
            if self.current_room != Some(spat.get_room()) {
                continue;
            }

            // Detect spatula collection events
            if spat != Spatula::TheSmallShallRuleOrNot
                && spat != Spatula::KahRahTae
                && game.is_spatula_being_collected(spat)?
            {
                // TODO: Don't make this None.
                self.spatulas.insert(spat, None);
                let _ = gui_sender.send(GuiMessage::Spatula(spat));
                info!("Collected {spat:?}");
            }
        }

        Ok(())
    }

    /// True when all connected players are on the Main Menu
    fn can_start(&self) -> bool {
        // TODO: Solve the "Demo Cutscene" issue. We can probably detect when players are on the autosave preference screen instead.
        self.current_room == Some(Room::MainMenu)
    }
}
