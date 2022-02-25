use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, Sender},
};

use clash::{
    protocol::{Item, Message},
    spatula::Spatula,
};
use log::info;
use strum::{EnumCount, IntoEnumIterator};

use crate::game::GameInterface;

pub struct GameState {
    pub spatulas: HashMap<Spatula, Option<usize>>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            spatulas: HashMap::with_capacity(Spatula::COUNT),
        }
    }
}

impl GameState {
    pub fn update<T: GameInterface>(
        &mut self,
        game: &T,
        gui_sender: &mut Sender<Spatula>,
        _network_sender: &mut tokio::sync::mpsc::Sender<Message>,
        logic_receiver: &mut Receiver<Message>,
    ) {
        while let Ok(m) = logic_receiver.try_recv() {
            if let Message::GameItemCollected {
                auth_id: _,
                item: Item::Spatula(spat),
            } = m
            {
                self.spatulas.insert(spat, None);
                game.mark_task_complete(spat);
                let _ = gui_sender.send(spat);
                info!("Collected {spat:?}");
            }
        }
        if game.is_loading() {
            return;
        }

        let curr_room = game.get_current_level();

        // Check for newly collected spatulas
        for spat in Spatula::iter().filter(|s| s.get_room() == curr_room) {
            if self.spatulas.contains_key(&spat) {
                continue;
            }
            if spat != Spatula::TheSmallShallRuleOrNot
                && spat != Spatula::KahRahTae
                && game.is_spatula_being_collected(spat)
            {
                // TODO: Don't make this None.
                self.spatulas.insert(spat, None);
                let _ = gui_sender.send(spat);
                info!("Collected {spat:?}");
            } else if game.is_task_complete(spat) {
                self.spatulas.insert(spat, None);
                let _ = gui_sender.send(spat);
                info!("Collected {spat:?}");
            }
        }
    }
}
