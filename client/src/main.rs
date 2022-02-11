use clash::protocol::Connection;
use tokio::net::TcpStream;

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .format_level(true)
        .format_module_path(true)
        .format_target(false)
        .format_indent(Some(4))
        .format_timestamp_secs()
        .filter_level(log::LevelFilter::Warn)
        .parse_env("CLASH_LOG")
        .init();
    let sock = TcpStream::connect("127.0.0.1:42932").await.unwrap();
    let mut conn = Connection::new(sock);

    conn.write_frame(clash::protocol::Message::GameHost {
        auth_id: 1,
        lobby_id: 2,
    })
    .await
    .unwrap();
}
