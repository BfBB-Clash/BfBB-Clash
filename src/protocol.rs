use tokio::{io::AsyncReadExt, io::AsyncWriteExt, net::TcpStream};

// TODO: Take more advantage of the type system (e.g. Client/Server messages)
#[derive(Debug)]
pub enum Message {
    ConnectionAccept {
        auth_id: u32,
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
        options: Options,
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

#[derive(Debug)]
pub enum Item {
    Spatula,
    Fuse,
}

#[derive(Debug)]
pub enum Room {}



pub struct Connection {
    socket: TcpStream,
}

impl Connection {
    pub fn new(socket: TcpStream) -> Self {
        Self { socket }
    }

    pub async fn read_frame(&mut self) -> Message {
        todo!()
    }

    pub async fn write_frame(&mut self, frame: Message) {
        todo!()
    }
}
