use bfbb::Level;
use clash_lib::{
    lobby::LobbyOptions,
    net::{Item, Message},
    player::PlayerOptions,
    PlayerId,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use super::LobbyError;
use super::{lobby_actor::LobbyAction, LobbyResult};

#[derive(Clone, Debug)]
pub struct LobbyHandleProvider {
    pub(super) sender: mpsc::WeakSender<LobbyAction>,
}

impl LobbyHandleProvider {
    pub fn into_handle(self, player_id: impl Into<PlayerId>) -> LobbyResult<LobbyHandle> {
        Ok(LobbyHandle {
            sender: self.sender.upgrade().ok_or(LobbyError::HandleInvalid)?,
            player_id: player_id.into(),
        })
    }

    pub async fn spectate(&self) -> LobbyResult<broadcast::Receiver<Message>> {
        let (tx, rx) = oneshot::channel();
        let sender = self.sender.upgrade().ok_or(LobbyError::HandleInvalid)?;
        let _ = sender
            .send(LobbyAction::AddSpectator { respond_to: tx })
            .await;
        rx.await.map_err(|_| LobbyError::HandleInvalid)
    }
}

#[derive(Debug)]
pub struct LobbyHandle {
    pub(super) sender: mpsc::Sender<LobbyAction>,
    pub(super) player_id: PlayerId,
}

impl LobbyHandle {
    async fn execute<T>(
        &self,
        msg: LobbyAction,
        rx: oneshot::Receiver<Result<T, LobbyError>>,
    ) -> Result<T, LobbyError> {
        // Ignore first error, if there is an error, rx.await will fail as well since it's sender
        // will have been dropped
        let _ = self.sender.send(msg).await;
        rx.await.unwrap_or(Err(LobbyError::HandleInvalid))
    }

    pub async fn reset_lobby(&self) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::ResetLobby {
            respond_to: tx,
            id: self.player_id,
        };
        self.execute(msg, rx).await
    }

    pub async fn start_game(&self) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::StartGame {
            respond_to: tx,
            id: self.player_id,
        };
        self.execute(msg, rx).await
    }

    /// Adds a new player to this lobby. If there is currently no host, they will become it.
    ///
    /// TODO: Would be nice to not have to manually call this. Since it's async we can't
    /// currently do this in the object constructor without holding a reference to the LobbyHandleProvider
    /// across an await boundary.
    pub async fn join_lobby(&self) -> Result<broadcast::Receiver<Message>, LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::AddPlayer {
            respond_to: tx,
            id: self.player_id,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_player_options(&self, options: PlayerOptions) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::SetPlayerOptions {
            respond_to: tx,
            id: self.player_id,
            options,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_player_can_start(&self, can_start: bool) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::SetPlayerCanStart {
            respond_to: tx,
            id: self.player_id,
            can_start,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_player_level(&self, level: Option<Level>) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::SetPlayerLevel {
            respond_to: tx,
            id: self.player_id,
            level,
        };
        self.execute(msg, rx).await
    }

    pub async fn player_collected_item(&self, item: Item) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::PlayerCollectedItem {
            respond_to: tx,
            id: self.player_id,
            item,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_game_options(&self, options: LobbyOptions) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyAction::SetGameOptions {
            respond_to: tx,
            id: self.player_id,
            options,
        };
        self.execute(msg, rx).await
    }
}

impl Drop for LobbyHandle {
    fn drop(&mut self) {
        let tx = self.sender.clone();
        let id = self.player_id;
        tokio::spawn(async move {
            if let Err(e) = tx.send(LobbyAction::RemovePlayer { id }).await {
                tracing::warn!(%e, "Failed to remove player from their lobby.");
            }
        });
    }
}

#[cfg(test)]
mod test {
    use bfbb::{Level, Spatula};
    use clash_lib::{lobby::LobbyOptions, net::Item, player::PlayerOptions, PlayerId};
    use tokio::sync::mpsc;

    use crate::lobby::{lobby_actor::LobbyAction, LobbyError};

    use super::{LobbyHandle, LobbyHandleProvider};

    fn setup() -> (mpsc::Receiver<LobbyAction>, LobbyHandle) {
        let (tx, rx) = mpsc::channel(2);
        let handle = LobbyHandle {
            sender: tx,
            player_id: 123.into(),
        };
        (rx, handle)
    }

    #[tokio::test]
    async fn lobby_provider_provides_new_handle() {
        let (tx, _rx) = mpsc::channel(2);
        let handle_provider = LobbyHandleProvider {
            sender: tx.downgrade(),
        };

        let handle = handle_provider.into_handle(123).unwrap();
        assert_eq!(handle.player_id, 123);
    }

    #[tokio::test]
    async fn reset_lobby() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            let LobbyAction::ResetLobby { respond_to: _, id } = m else {
                panic!("Incorrect LobbyAction produced");
            };
            assert_eq!(id, 123);
        });
        let _ = handle.reset_lobby().await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn start_game() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(
                m,
                LobbyAction::StartGame {
                    respond_to: _,
                    id: PlayerId(123),
                }
            ));
        });
        let _ = handle.start_game().await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn add_player() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(
                m,
                LobbyAction::AddPlayer {
                    respond_to: _,
                    id: PlayerId(123)
                }
            ));
        });
        let _ = handle.join_lobby().await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn rem_player_on_drop() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(m, LobbyAction::RemovePlayer { id: PlayerId(123) }));
        });
        drop(handle);
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn set_player_options() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            if let LobbyAction::SetPlayerOptions {
                respond_to: _,
                id,
                options,
            } = m
            {
                assert_eq!(id, 123);
                assert_eq!(
                    options,
                    PlayerOptions {
                        name: "tester".to_owned(),
                        ..Default::default()
                    }
                );
            } else {
                panic!("Incorrect message was sent");
            }
        });
        let _ = handle
            .set_player_options(PlayerOptions {
                name: "tester".to_owned(),
                ..Default::default()
            })
            .await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn set_player_level() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(
                m,
                LobbyAction::SetPlayerLevel {
                    respond_to: _,
                    id: PlayerId(123),
                    level: Some(Level::MainMenu)
                }
            ));
        });
        let _ = handle.set_player_level(Some(Level::MainMenu)).await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn player_collected_item() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(
                m,
                LobbyAction::PlayerCollectedItem {
                    respond_to: _,
                    id: PlayerId(123),
                    item: Item::Spatula(Spatula::OnTopOfThePineapple)
                }
            ));
        });
        let _ = handle
            .player_collected_item(Item::Spatula(Spatula::OnTopOfThePineapple))
            .await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn set_game_options() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            if let LobbyAction::SetGameOptions {
                respond_to: _,
                id,
                options,
            } = m
            {
                assert_eq!(id, 123);
                assert_eq!(
                    options,
                    LobbyOptions {
                        lab_door_cost: 69,
                        ..Default::default()
                    }
                );
            } else {
                panic!("Incorrect message was sent");
            }
        });
        let _ = handle
            .set_game_options(LobbyOptions {
                lab_door_cost: 69,
                ..Default::default()
            })
            .await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn invalid_handle() {
        let (mut rx, handle) = setup();

        // Ensure that an action performed on a closed lobby will result in a `HandleInvalid` error.
        rx.close();
        assert_eq!(handle.start_game().await, Err(LobbyError::HandleInvalid));
        drop(rx);
        assert_eq!(handle.start_game().await, Err(LobbyError::HandleInvalid));
    }
}
