use clash::spatula::Spatula;

/// Trait for interacting with BfBB in an abstract way.
pub trait GameInterface {
    /// Will start a new game when called. Only works when the player is on the main menu and not in the demo cutscene.
    fn start_new_game(&self);
    /// Sets the player's total spatula count to `value`
    fn set_spatula_count(&self, value: u32);
    /// Marks a spatula as "completed" in the pause menu. This has the effect of giving the player access to the task warp.
    fn mark_task_complete(&self, spatula: Spatula);
}
