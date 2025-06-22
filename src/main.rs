mod models;
mod audio_api;
mod routes;
mod steam;
mod dev;
mod app_state;
pub mod schema;
mod command_parser;
mod backends;
mod battery_monitor;
mod overlay;
mod logging;
mod adb;

use self::models::*;
use crate::app_state::AppState;
use crate::audio_api::{DeviceChangeEvent, PipeWireManager};
use crate::backends::BackendType;
use crate::battery_monitor::BatteryMonitor;
use crate::overlay::WlxOverlayManager;
use crate::steam::launcher::{CompatLauncher, ProcessHandle};
use axum::http::{header, HeaderValue};
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use steam::steam_interface::SteamInterface;
use tokio::sync::{broadcast, Mutex};
use ts_rs::TS;
use crate::adb::device_manager::DeviceManager;

include!(concat!(env!("OUT_DIR"), "/frontend_assets.rs"));

pub type StdMutex<T> = std::sync::Mutex<T>;
pub type TokioMutex<T> = tokio::sync::Mutex<T>;

#[derive(Debug, Serialize, TS)]
#[ts(export, export_to = "rust_bindings.ts")]
#[serde(rename_all = "camelCase")]
pub struct GameSession {
    game: Game,
    #[serde(skip)]
    process_handle: ProcessHandle,
    start_time_epoch: u64,
    vr_device_serial: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let steam_api = SteamInterface::new();
    let launcher = Arc::new(CompatLauncher::new());

    let (ws_tx, _) = broadcast::channel::<String>(100);
    let audio_api = PipeWireManager::new();
    let mut device_changes = audio_api.subscribe_to_changes();

    let ws_tx_clone = ws_tx.clone();
    tokio::spawn(async move {
        while let Ok(event) = device_changes.recv().await {
            let message = match event {
                DeviceChangeEvent::DefaultInputChanged(device) =>
                    format!("default_input_changed:{}", serde_json::to_string(&device).unwrap()),
                DeviceChangeEvent::DefaultOutputChanged(device) =>
                    format!("default_output_changed:{}", serde_json::to_string(&device).unwrap()),
            };
            let _ = ws_tx_clone.send(message);
        }
    });

    let device_manager = Arc::new(Mutex::new(DeviceManager::new()?));
    let ws_tx_clone = ws_tx.clone();
    let app_state = Arc::new(Mutex::new(AppState {
        audio_api,
        steam_api,
        launcher: launcher.clone(),
        active_game_session: None,
        sock_tx: ws_tx,
        active_backend: None,
        device_manager: device_manager.clone(),
        backend_type: BackendType::Unknown,
        battery_monitor: BatteryMonitor::new(ws_tx_clone, device_manager.clone()),
        overlay_manager: WlxOverlayManager::new(),
        log_session: None,
        launch_requests: HashSet::new(),
    }));


    launcher.set_app_state_async(app_state.clone()).await;

    let app = Router::new()
        .route("/api/games", get(routes::games::list_games))
        .route("/api/games/{game_id}", get(routes::games::get_game_info))
        .route("/api/games/{game_id}/cover", get(routes::games::get_game_cover))
        .route("/api/games/{game_id}/launch", post(routes::game_state::launch_game_async))
        .route("/api/games/active", get(routes::game_state::get_active_game))
        .route("/api/games/active/kill", post(routes::game_state::kill_active_game))
        .route("/api/games/reload_backend", post(routes::game_state::reload_backend))
        .route("/api/audio/{endpoint}", get(routes::audio::get_audio_endpoints))
        .route("/api/audio/{endpoint}/{endpoint_id}/default", post(routes::audio::set_default_audio_endpoint))
        .route("/api/sock", get(routes::sock::sock_state_handler))
        .route("/api/device/battery", get(routes::device::get_battery_status))
        .route("/api/debug/agent", get(routes::debug::get_user_agent))
        .route("/{path}", get(routes::frontend::get_frontend_asset))
        .fallback(get(routes::frontend::get_frontend_asset))
        .layer(tower_http::set_header::SetResponseHeaderLayer::appending(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        ))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;
    axum::serve(listener, app).await?;

    Ok(())
}