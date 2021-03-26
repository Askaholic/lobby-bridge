use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use log::*;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::select;
use tokio_tungstenite::WebSocketStream;
use tungstenite::error::Result as WsResult;
use tungstenite::Message;

use std::fmt::Display;

mod config;

#[tokio::main]
async fn main() {
    env_logger::init();
    config::init();

    let addr = &*config::BIND_ADDR;
    let listener = TcpListener::bind(&addr).await.expect("Can't listen");
    info!("Listening on: wss://{}", &addr);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }
}

async fn accept_connection(stream: TcpStream) {
    let addr = stream.peer_addr().unwrap();
    info!("New connection from: {}", &addr);

    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws_stream) => ws_stream,
        Err(e) => {
            info!("{} - Failed websocket handshake: {}", &addr, e);
            return;
        }
    };
    info!("{} - Established websocket connection", &addr);

    let lb_stream = match TcpStream::connect(&*config::LOBBY_ADDR).await {
        Ok(lb_stream) => lb_stream,
        Err(e) => {
            error!("{} - Failed to establish lobby connection: {}", &addr, e);
            return;
        }
    };
    debug!("{} - Established lobby connection", &addr);

    handle_connection(ws_stream, lb_stream).await;

    info!("{} - Disconnected", &addr);
}

/// Proxy data between the websocket client and lobby server
async fn handle_connection(ws_stream: WebSocketStream<TcpStream>, mut lb_conn: TcpStream) {
    let addr = ws_stream.get_ref().peer_addr().unwrap();
    let (ws_writer, ws_reader) = ws_stream.split();
    let (lb_reader, lb_writer) = lb_conn.split();

    if let Err(e) = select!(
        e = read_from_websocket(ws_reader, lb_writer, &addr) => e,
        e = read_from_lobby(lb_reader, ws_writer, &addr) => e
    ) {
        info!("{} - Encountered error: {}", &addr, e);
    }
}

/// Read text messages from the websocket and forward it to the lobby connection on a new line.
async fn read_from_websocket(
    mut reader: SplitStream<WebSocketStream<TcpStream>>,
    mut writer: impl AsyncWrite + Unpin,
    addr: &impl Display,
) -> WsResult<()> {
    while let Some(msg) = reader.next().await {
        let msg = msg?;

        if msg.is_text() {
            let mut msg_text = msg.into_text().unwrap();
            msg_text.push('\n');

            debug!("{} - WebSocket >> {}", &addr, msg_text.trim());
            writer.write_all(msg_text.as_bytes()).await?
        } else if msg.is_close() {
            break;
        } else {
            debug!("Socket sent unsupported message {:?}", msg);
        }
    }

    Ok(())
}

/// Read lines from the lobby server and forward them to the websocket as text messages.
async fn read_from_lobby(
    reader: impl AsyncRead + Unpin,
    mut writer: SplitSink<WebSocketStream<TcpStream>, Message>,
    addr: &impl Display,
) -> WsResult<()> {
    let mut reader = BufReader::new(reader);
    loop {
        let mut msg = String::new();
        reader.read_line(&mut msg).await?;

        if msg.is_empty() {
            break;
        }
        // Trim off the trailing newline
        msg.pop();

        debug!("{} - Lobby >> {}", addr, &msg);
        writer.send(Message::Text(msg)).await?;
    }

    Ok(())
}
