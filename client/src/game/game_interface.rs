use clash::{room::Room, spatula::Spatula};

/// Trait for interacting with BfBB in an abstract way.
pub trait GameInterface {
    /// True if the game is currently in a loading screen.
    fn is_loading(&self) -> bool;

    /// Will start a new game when called. Only works when the player is on the main menu and not in the demo cutscene.
    fn start_new_game(&self);

    /// Get the level that the player is currently in
    fn get_current_level(&self) -> Room;

    /// Gets the player's total spatula count
    fn get_spatula_count(&self) -> u32;

    /// Sets the player's total spatula count to `value`
    fn set_spatula_count(&self, value: u32);

    /// Marks a spatula as "completed" in the pause menu. This has the effect of giving the player access to the task warp.
    fn mark_task_complete(&self, spatula: Spatula);

    /// True when `spatula` is shown as gold in the pause menu.
    /// Calling this from outside the level that `spatula` is in is undefined behavior.
    fn is_task_complete(&self, spatula: Spatula) -> bool;

    /// Collect a spatula in the world. This only removes the entity, it will not complete the task or increment the spatula
    /// counter. Calling this from outside the level that `spatula` is in is undefined behavior.
    fn collect_spatula(&self, spatula: Spatula);

    /// True when `spatula`'s collected animation is playing
    /// Calling this from outside the level that `spatula` is in is undefined behavior.
    fn is_spatula_being_collected(&self, spatula: Spatula) -> bool;

    /// Changes the number of spatulas required to enter the Chum Bucket Lab. Calling this from outside of the Chum Bucket
    /// is undefined behavior.
    fn set_lab_door(&self, value: u32);
}
