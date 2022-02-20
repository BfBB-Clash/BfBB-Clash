use std::{collections::HashSet, sync::mpsc::Sender};

use clash::spatula::Spatula;
use log::info;
use strum::IntoEnumIterator;

use crate::game::GameInterface;

pub struct GameState {
    pub spatulas: HashSet<Spatula>,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            spatulas: HashSet::default(),
        }
    }
}

impl GameState {
    pub fn update<T: GameInterface>(&mut self, game: &T, gui_sender: &mut Sender<Spatula>) {
        if game.is_loading() {
            return;
        }

        let curr_room = game.get_current_level();

        // Check for newly collected spatulas
        for spat in Spatula::iter().filter(|s| s.get_room() == curr_room) {
            if self.spatulas.contains(&spat) {
                continue;
            }
            if spat != Spatula::TheSmallShallRuleOrNot
                && spat != Spatula::KahRahTae
                && game.is_spatula_being_collected(spat)
            {
                self.spatulas.insert(spat);
                let _ = gui_sender.send(spat);
                info!("Collected {spat:?}");
            } else if game.is_task_complete(spat) {
                self.spatulas.insert(spat);
                let _ = gui_sender.send(spat);
                info!("Collected {spat:?}");
            }
        }
    }
}
