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
    let mut rx = {
        let state = state.lock().await;
        state.sock_tx.subscribe()
    };

    let (mut sender, _) = socket.split();

    // Task to receive broadcast messages and send to client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break; // Client disconnected
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
    }
}