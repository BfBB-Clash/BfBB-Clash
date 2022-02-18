use std::collections::HashSet;

use clash::spatula::Spatula;
use log::info;
use strum::IntoEnumIterator;

use crate::game::GameInterface;

pub struct GameState<T: GameInterface> {
    interface: T,
    spatulas: HashSet<Spatula>,
}

impl<T: GameInterface> GameState<T> {
    pub fn new(interface: T) -> Self {
        Self {
            interface,
            spatulas: HashSet::default(),
        }
    }

    pub fn update(&mut self) {
        let game = &self.interface;
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
                && self.interface.is_spatula_being_collected(spat)
            {
                self.spatulas.insert(spat);
                info!("Collected {spat:?}");
            } else if self.interface.is_task_complete(spat) {
                self.spatulas.insert(spat);
                info!("Collected {spat:?}");
            }
        }
    }
}
