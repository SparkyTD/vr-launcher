use axum::extract::ws::Message;
use axum::extract::{State, WebSocketUpgrade};
use axum::extract::ws::WebSocket;
use axum::response::Response;
use futures_util::SinkExt;
use futures_util::stream::StreamExt;
use crate::app_state::AppStateWrapper;

pub async fn sock_state_handler(ws: WebSocketUpgrade, State(state): State<AppStateWrapper>) -> Response {
    ws.on_upgrade(|socket| sock_state(socket, state))
}

async fn sock_state(socket: WebSocket, state: AppStateWrapper) {
    let (mut data_rx, mut stop_rx) = {
        let state = state.lock().await;
        let result = (state.sock_tx.subscribe(), state.socket_stop_tx.subscribe());
        drop(state);
        
        result
    };

    let (mut sender, _) = socket.split();

    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = stop_rx.recv() => {
                    println!("A socket handler thread has received an interrupt signal");
                    break;
                }
                message_result = data_rx.recv() => {
                    match message_result {
                        Ok(message) => {
                            if sender.send(Message::Text(message.into())).await.is_err() {
                                break; // Client disconnected
                            }
                        },
                        Err(err) => {
                            eprintln!("An error occurred while reading message from socket: {}", err);
                        },
                    }
                }
            }
        }

        println!("  >> [SOCK] Task exiting");
    });

    tokio::select! {
        _ = send_task => {},
    }
}