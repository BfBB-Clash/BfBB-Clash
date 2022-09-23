use bfbb::Level;
use clash_lib::{
    lobby::LobbyOptions,
    net::{Item, Message},
    player::PlayerOptions,
    LobbyId, PlayerId,
};
use tokio::sync::{broadcast, mpsc, oneshot};

use super::lobby_actor::LobbyMessage;
use super::LobbyError;

#[derive(Debug)]
pub struct LobbyHandleProvider {
    pub(super) sender: mpsc::Sender<LobbyMessage>,
    pub(super) lobby_id: LobbyId,
}

impl LobbyHandleProvider {
    /// Adds a new player to this lobby. If there is currently no host, they will become it.
    pub fn get_handle(&self, player_id: PlayerId) -> LobbyHandle {
        LobbyHandle {
            sender: self.sender.clone(),
            lobby_id: self.lobby_id,
            player_id,
        }
    }
}

/// Automatically removes player from lobby when dropped.
#[derive(Clone, Debug)]
pub struct LobbyHandle {
    sender: mpsc::Sender<LobbyMessage>,
    lobby_id: LobbyId,
    player_id: PlayerId,
}

impl LobbyHandle {
    async fn execute<T>(
        &self,
        msg: LobbyMessage,
        rx: oneshot::Receiver<Result<T, LobbyError>>,
    ) -> Result<T, LobbyError> {
        // Ignore first error, if there is an error, rx.await will fail aswell since it's sender
        // will have been dropped
        let _ = self.sender.send(msg).await;
        rx.await.unwrap_or(Err(LobbyError::HandleInvalid))
    }

    pub async fn start_game(&self) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::StartGame {
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
        let msg = LobbyMessage::AddPlayer {
            respond_to: tx,
            id: self.player_id,
        };
        self.execute(msg, rx).await.map(|recv| {
            log::info!(
                "Player {:#X} has joined lobby {:#X}",
                self.player_id,
                self.lobby_id,
            );
            recv
        })
    }

    // Removes a player from the lobby, if it exists, returning the number of player's remaining
    pub async fn rem_player(&self) -> Result<(), LobbyError> {
        // TODO: Do this with self.execute somehow?
        self.sender
            .send(LobbyMessage::RemovePlayer { id: self.player_id })
            .await
            .map_err(|_| LobbyError::HandleInvalid)
    }

    pub async fn set_player_options(&self, options: PlayerOptions) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetPlayerOptions {
            respond_to: tx,
            id: self.player_id,
            options,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_player_can_start(&self, can_start: bool) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetPlayerCanStart {
            respond_to: tx,
            id: self.player_id,
            can_start,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_player_level(&self, level: Option<Level>) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetPlayerLevel {
            respond_to: tx,
            id: self.player_id,
            level,
        };
        self.execute(msg, rx).await
    }

    pub async fn player_collected_item(&self, item: Item) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::PlayerCollectedItem {
            respond_to: tx,
            id: self.player_id,
            item,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_game_options(&self, options: LobbyOptions) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetGameOptions {
            respond_to: tx,
            id: self.player_id,
            options,
        };
        self.execute(msg, rx).await
    }
}

#[cfg(test)]
mod test {
    use bfbb::{Level, Spatula};
    use clash_lib::{lobby::LobbyOptions, net::Item, player::PlayerOptions};
    use tokio::sync::mpsc;

    use crate::lobby::{lobby_actor::LobbyMessage, LobbyError};

    use super::{LobbyHandle, LobbyHandleProvider};

    fn setup() -> (mpsc::Receiver<LobbyMessage>, LobbyHandle) {
        let (tx, rx) = mpsc::channel(2);
        let handle = LobbyHandle {
            sender: tx,
            lobby_id: 0,
            player_id: 123,
        };
        (rx, handle)
    }

    #[test]
    fn lobby_provider_provides_new_handle() {
        let handle_provider = LobbyHandleProvider {
            sender: mpsc::channel(2).0,
            lobby_id: 0,
        };

        let handle = handle_provider.get_handle(123);
        assert_eq!(handle.player_id, 123);
    }

    #[tokio::test]
    async fn start_game() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(
                m,
                LobbyMessage::StartGame {
                    respond_to: _,
                    id: 123
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
                LobbyMessage::AddPlayer {
                    respond_to: _,
                    id: 123
                }
            ));
        });
        let _ = handle.join_lobby().await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn rem_player() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(m, LobbyMessage::RemovePlayer { id: 123 }));
        });
        let _ = handle.rem_player().await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn set_player_options() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            if let LobbyMessage::SetPlayerOptions {
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
                LobbyMessage::SetPlayerLevel {
                    respond_to: _,
                    id: 123,
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
                LobbyMessage::PlayerCollectedItem {
                    respond_to: _,
                    id: 123,
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
            if let LobbyMessage::SetGameOptions {
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
