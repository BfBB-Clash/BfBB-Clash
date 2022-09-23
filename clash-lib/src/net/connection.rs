use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use std::io::Cursor;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::{io::AsyncReadExt, io::AsyncWriteExt, io::BufWriter, net::TcpStream};

use super::{FrameError, Message};

pub fn from_socket(socket: TcpStream) -> (ConnectionTx, ConnectionRx) {
    let (read_stream, write_stream) = socket.into_split();

    (
        ConnectionTx {
            write_stream: BufWriter::new(write_stream),
        },
        ConnectionRx {
            read_stream,
            buffer: BytesMut::with_capacity(64),
        },
    )
}

#[derive(Debug)]
pub struct ConnectionTx {
    write_stream: BufWriter<OwnedWriteHalf>,
}
pub struct ConnectionRx {
    read_stream: OwnedReadHalf,
    buffer: BytesMut,
}

impl ConnectionTx {
    pub async fn write_frame(&mut self, frame: Message) -> Result<(), FrameError> {
        let mut bytes: Bytes = bincode::serialize(&frame)?.into();
        if bytes.len() > u16::MAX.into() {
            return Err(FrameError::FrameLength);
        }
        let len = bytes.len() as u16;
        let len = len.to_be_bytes();
        self.write_stream.write_all(&len).await?;
        self.write_stream.write_buf(&mut bytes).await?;
        self.write_stream.flush().await?;
        Ok(())
    }
}

impl ConnectionRx {
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
                    return Err(FrameError::ConnectionReset.into());
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
}
