use crate::game::{GameInterface, InterfaceResult};
use crate::gui::GuiMessage;
use clash::{
    lobby::LobbyOptions,
    protocol::{Item, Message},
    room::Room,
    spatula::Spatula,
};
use log::info;
use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};
use strum::{EnumCount, IntoEnumIterator};

pub struct GameState {
    pub options: LobbyOptions,
    pub spatulas: HashMap<Spatula, Option<usize>>,
    pub current_room: Option<Room>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            options: LobbyOptions::default(),
            spatulas: HashMap::with_capacity(Spatula::COUNT),
            current_room: None,
        }
    }
}

impl GameState {
    pub fn update<T: GameInterface>(
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
            game.set_lab_door(self.options.lab_door_cost.into())?;
        }

        // Check for newly collected spatulas
        for spat in Spatula::iter().filter(|s| Some(s.get_room()) == self.current_room) {
            if self.spatulas.contains_key(&spat) {
                continue;
            }
            if spat != Spatula::TheSmallShallRuleOrNot
                && spat != Spatula::KahRahTae
                && game.is_spatula_being_collected(spat)?
            {
                // TODO: Don't make this None.
                self.spatulas.insert(spat, None);
                let _ = gui_sender.send(GuiMessage::Spatula(spat));
                info!("Collected {spat:?}");
            } else if game.is_task_complete(spat)? {
                self.spatulas.insert(spat, None);
                let _ = gui_sender.send(GuiMessage::Spatula(spat));
                info!("Collected {spat:?}");
            }
        }

        Ok(())
    }
}
