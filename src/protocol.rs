use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use thiserror::Error;
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, io::BufWriter, net::TcpStream};

use crate::lobby::{LobbyOptions, SharedLobby};
use crate::player::PlayerOptions;
use crate::room::Room;
use crate::spatula::Spatula;

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
        room: Option<Room>,
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
    Spatula(Spatula),
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

    pub async fn read_frame(&mut self) -> Result<Option<Message>> {
        loop {
            if let Some(frame) = self.parse_frame()? {
                return Ok(Some(frame));
            }

            if self.read_stream.read_buf(&mut self.buffer).await? == 0 {
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

    fn parse_frame(&mut self) -> Result<Option<Message>, tokio::io::Error> {
        // Use a Cursor to avoid advancing the internal cursor of self.buffer
        let mut buf = Cursor::new(&self.buffer[..]);

        if self.buffer.len() < 2 {
            return Ok(None);
        }

        // Check if the buffer contains the full message yet
        let message_len = buf.get_u16().into();
        if self.buffer.remaining() < message_len + std::mem::size_of::<u16>() {
            return Ok(None);
        }

        // Consume the frame from the buffer and deserialize a message
        self.buffer.advance(std::mem::size_of::<u16>());
        let message = bincode::deserialize::<Message>(&self.buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        self.buffer.advance(message_len);

        Ok(Some(message))
    }

    pub async fn write_frame(&mut self, frame: Message) -> Result<()> {
        let mut bytes: Bytes = bincode::serialize(&frame)?.into();
        if bytes.len() > u16::MAX.into() {
            return Err(Error::FrameLength.into());
        }
        let len = bytes.len() as u16;
        let len = len.to_be_bytes();
        self.write_stream.write(&len).await?;
        self.write_stream.write_buf(&mut bytes).await?;
        self.write_stream.flush().await?;
        Ok(())
    }
}
