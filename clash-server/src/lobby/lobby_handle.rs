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

#[derive(Clone, Debug)]
pub struct LobbyHandle {
    pub(super) sender: mpsc::Sender<LobbyMessage>,
    pub(super) lobby_id: LobbyId,
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

    pub fn get_lobby_id(&self) -> LobbyId {
        self.lobby_id
    }

    pub async fn start_game(&self, player_id: PlayerId) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::StartGame {
            respond_to: tx,
            id: player_id,
        };
        self.execute(msg, rx).await
    }

    /// Adds a new player to this lobby. If there is currently no host, they will become it.
    pub async fn add_player(
        &self,
        player_id: PlayerId,
    ) -> Result<broadcast::Receiver<Message>, LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::AddPlayer {
            respond_to: tx,
            id: player_id,
        };
        self.execute(msg, rx).await
    }

    // Removes a player from the lobby, if it exists, returning the number of player's remaining
    pub async fn rem_player(&self, player_id: PlayerId) -> Result<(), LobbyError> {
        // TODO: Do this with self.execute somehow?
        self.sender
            .send(LobbyMessage::RemovePlayer { id: player_id })
            .await
            .map_err(|_| LobbyError::HandleInvalid)
    }

    pub async fn set_player_options(
        &self,
        player_id: PlayerId,
        options: PlayerOptions,
    ) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetPlayerOptions {
            respond_to: tx,
            id: player_id,
            options,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_player_can_start(
        &self,
        player_id: PlayerId,
        can_start: bool,
    ) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetPlayerCanStart {
            respond_to: tx,
            id: player_id,
            can_start,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_player_level(
        &self,
        player_id: PlayerId,
        level: Option<Level>,
    ) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetPlayerLevel {
            respond_to: tx,
            id: player_id,
            level,
        };
        self.execute(msg, rx).await
    }

    pub async fn player_collected_item(
        &self,
        player_id: PlayerId,
        item: Item,
    ) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::PlayerCollectedItem {
            respond_to: tx,
            id: player_id,
            item,
        };
        self.execute(msg, rx).await
    }

    pub async fn set_game_options(
        &self,
        player_id: PlayerId,
        options: LobbyOptions,
    ) -> Result<(), LobbyError> {
        let (tx, rx) = oneshot::channel();
        let msg = LobbyMessage::SetGameOptions {
            respond_to: tx,
            id: player_id,
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

    use super::LobbyHandle;

    fn setup() -> (mpsc::Receiver<LobbyMessage>, LobbyHandle) {
        let (tx, rx) = mpsc::channel(2);
        let handle = LobbyHandle {
            sender: tx,
            lobby_id: 0,
        };
        (rx, handle)
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
                    id: 1234
                }
            ));
        });
        let _ = handle.start_game(1234).await;
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
                    id: 0x12123434
                }
            ));
        });
        let _ = handle.add_player(0x12123434).await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn rem_player() {
        let (mut rx, handle) = setup();
        let actor = tokio::spawn(async move {
            let m = rx.recv().await.unwrap();
            assert!(matches!(m, LobbyMessage::RemovePlayer { id: 0x13371337 }));
        });
        let _ = handle.rem_player(0x13371337).await;
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
                assert_eq!(id, 0x12123434);
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
            .set_player_options(
                0x12123434,
                PlayerOptions {
                    name: "tester".to_owned(),
                    ..Default::default()
                },
            )
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
                    id: 0x12123434,
                    level: Some(Level::MainMenu)
                }
            ));
        });
        let _ = handle
            .set_player_level(0x12123434, Some(Level::MainMenu))
            .await;
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
                    id: 0x12123434,
                    item: Item::Spatula(Spatula::OnTopOfThePineapple)
                }
            ));
        });
        let _ = handle
            .player_collected_item(0x12123434, Item::Spatula(Spatula::OnTopOfThePineapple))
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
                assert_eq!(id, 0x12123434);
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
            .set_game_options(
                0x12123434,
                LobbyOptions {
                    lab_door_cost: 69,
                    ..Default::default()
                },
            )
            .await;
        actor.await.unwrap();
    }

    #[tokio::test]
    async fn invalid_handle() {
        let (mut rx, handle) = setup();

        // Ensure that an action performed on a closed lobby will result in a `HandleInvalid` error.
        rx.close();
        assert_eq!(handle.start_game(0).await, Err(LobbyError::HandleInvalid));
        drop(rx);
        assert_eq!(handle.start_game(0).await, Err(LobbyError::HandleInvalid));
    }
}
