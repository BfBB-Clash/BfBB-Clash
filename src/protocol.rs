use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, io::BufWriter, net::TcpStream};

use crate::lobby::{LobbyOptions, SharedLobby};
use crate::player::PlayerOptions;
use crate::room::Room;

// TODO: Take more advantage of the type system (e.g. Client/Server messages)
#[derive(Debug, Deserialize, Serialize, Clone)]
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
        options: LobbyOptions,
    },
    GameLobbyInfo {
        auth_id: u32,
        lobby: SharedLobby,
    },
    GameBegin {
        auth_id: u32,
    },
    GameCurrentRoom {
        auth_id: u32,
        room: Room,
    },
    GameForceWarp {
        auth_id: u32,
        room: Room,
    },
    GameItemCollected {
        auth_id: u32,
        item: Item,
    },
    GameEnd {
        auth_id: u32,
    },
    GameLeave {
        auth_id: u32,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Item {
    Spatula,
    Fuse,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Frame exceeded max length")]
    FrameLength,
    #[error("Full frame is not available yet.")]
    FrameIncomplete,
    #[error("Connection reset by peer")]
    ConnectionReset,
}

#[derive(Debug)]
pub struct Connection<'a> {
    read_stream: ReadHalf<'a>,
    write_stream: BufWriter<WriteHalf<'a>>,
    buffer: BytesMut,
}

impl<'a> Connection<'a> {
    pub fn new(socket: &'a mut TcpStream) -> Self {
        let (read_stream, write_stream) = socket.split();
        Self {
            read_stream,
            write_stream: BufWriter::new(write_stream),
            buffer: BytesMut::with_capacity(64),
        }
    }

    pub fn read_frame(&mut self) -> Result<Option<Message>, tokio::io::Error> {
        if let Some(frame) = self.parse_frame()? {
            return Ok(Some(frame));
        }
        if self.read_stream.try_read(&mut self.buffer)? == 0 {
            if self.buffer.is_empty() {
                // Remote closed Connection
                return Ok(None);
            } else {
                // Connection closed while still sending data
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    Error::ConnectionReset,
                ));
            }
        }

        todo!()
    }

    fn parse_frame(&mut self) -> Result<Option<Message>, tokio::io::Error> {
        if self.buffer.len() < 2 {
            return Ok(None);
        }

        let len = self.buffer.get_u16().into();
        if self.buffer.remaining() < len {
            return Ok(None);
        }

        let message = bincode::deserialize::<Message>(&self.buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
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
        self.write_stream.write(&mut len).await?;
        self.write_stream.write_buf(&mut bytes).await?;
        self.write_stream.flush().await?;
        Ok(())
    }
}
