use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, io::BufWriter, net::TcpStream};

use crate::room::Room;
use crate::lobby::LobbyOptions;
use crate::player::

// TODO: Take more advantage of the type system (e.g. Client/Server messages)
#[derive(Debug, Deserialize, Serialize)]
pub enum Message {
    ConnectionAccept {
        auth_id: u32,
    },
    PlayerOptions {
        auth_id: u32,
        options: PlayerOptions,
    },
    GameHost {
        auth_id: u32,
        lobby_id: u32,
    },
    GameJoin {
        auth_id: u32,
        lobby_id: u32,
    },
    GameOptions {
        auth_id: u32,
        lobby_id: u32,
        options: LobbyOptions,
    },
    GameLobbyInfo {
        auth_id: u32,
        lobby_id: u32,
    },
    GameBegin {
        auth_id: u32,
        lobby_id: u32,
    },
    GameCurrentRoom {
        auth_id: u32,
        lobby_id: u32,
        room: Room,
    },
    GameForceWarp {
        auth_id: u32,
        lobby_id: u32,
        room: Room,
    },
    GameItemCollected {
        auth_id: u32,
        lobby_id: u32,
        item: Item,
    },
    GameEnd {
        auth_id: u32,
        lobby_id: u32,
    },
    GameLeave {
        auth_id: u32,
        lobby_id: u32,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Item {
    Spatula,
    Fuse,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Frame exceeded max length")]
    FrameLength,
    #[error("Connection reset by peer")]
    ConnectionReset,
}

#[derive(Debug)]
pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(socket),
            buffer: BytesMut::with_capacity(64),
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<Message>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            if self.stream.read_buf(&mut self.buffer).await? == 0 {
                if self.buffer.is_empty() {
                    // Remote closed Connection
                    return Ok(None);
                } else {
                    // Connection closed while still sending data
                    return Err(Error::ConnectionReset.into());
                }
            }
        }
    }

    fn parse_frame(&mut self) -> Result<Option<Message>> {
        if self.buffer.len() < 2 {
            return Ok(None);
        }

        let len = self.buffer.get_u16().into();
        if self.buffer.remaining() < len {
            return Ok(None);
        }

        let message = bincode::deserialize::<Message>(&self.buffer)?;
        self.buffer.advance(len);
        Ok(Some(message))
    }

    pub async fn write_frame(&mut self, frame: Message) -> Result<()> {
        let mut bytes: Bytes = bincode::serialize(&frame)?.into();
        if bytes.len() > u16::MAX.into() {
            return Err(Error::FrameLength.into());
        }
        let len: u16 = bytes.len() as u16;
        let mut len = len.to_be_bytes();
        self.stream.write(&mut len).await?;
        self.stream.write_buf(&mut bytes).await?;
        self.stream.flush().await?;
        Ok(())
    }
}
