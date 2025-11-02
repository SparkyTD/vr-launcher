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
use crate::adb::device_manager::DeviceManager;
use crate::app_state::AppState;
use crate::audio_api::{DeviceChangeEvent, PipeWireManager};
use crate::backends::BackendType;
use crate::battery_monitor::BatteryMonitor;
use crate::overlay::WlxOverlayManager;
use crate::steam::launcher::{CompatLauncher, ProcessHandle};
use axum::http::{header, HeaderValue};
use axum::routing::{get, post};
use axum::Router;
use image::ImageFormat;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use steam::steam_interface::SteamInterface;
use tokio::signal;
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use tray_icon::menu::{MenuEvent, MenuId, MenuItemBuilder};
use tray_icon::{Icon, TrayIconBuilder};
use ts_rs::TS;

include!(concat!(env!("OUT_DIR"), "/bundled_assets.rs"));

pub type StdMutex<T> = std::sync::Mutex<T>;
pub type TokioMutex<T> = Mutex<T>;

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
    println!("Launcher Process ID: {}", std::process::id());

    /*let instance = SingleInstance::new("vr-launcher-instance")?;
    if !instance.is_single() {
        return Err(anyhow::anyhow!("Another instance of this application is already running"));
    }*/

    let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
    let tray_icon_bytes = include_bytes!("../icon.png");
    let tray_icon_image = image::ImageReader::with_format(std::io::Cursor::new(tray_icon_bytes), ImageFormat::Png)
        .decode()?
        .into_rgba8();
    let (width, height) = tray_icon_image.dimensions();
    thread::spawn(move || {
        use tray_icon::menu::Menu;

        gtk::init().unwrap();
        let tray_menu = Menu::new();
        tray_menu.append(&MenuItemBuilder::new()
            .id(MenuId("exit".into()))
            .text("Exit")
            .enabled(true)
            .build()).unwrap();
        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_icon(Icon::from_rgba(tray_icon_image.into_raw(), width, height).unwrap())
            .build()
            .unwrap();

        let menu_channel = MenuEvent::receiver();

        thread::spawn(move || {
            loop {
                if let Ok(event) = menu_channel.try_recv() {
                    match event.id {
                        MenuId(id) if id == "exit" => {
                            _ = shutdown_tx.blocking_send(());
                            break;
                        }
                        _ => continue,
                    }
                }

                thread::sleep(std::time::Duration::from_millis(100));
            }
        });

        gtk::main();
    });

    let steam_api = SteamInterface::new();
    let launcher = Arc::new(CompatLauncher::new());

    let (sock_tx, _) = broadcast::channel::<String>(100);
    let audio_api = PipeWireManager::new()?;
    let mut device_changes = audio_api.subscribe_to_changes();

    let ws_tx_clone = sock_tx.clone();
    let (audio_monitor_stop_tx, _) = broadcast::channel::<()>(1);
    let mut audio_monitor_stop_rx = audio_monitor_stop_tx.subscribe();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = audio_monitor_stop_rx.recv() => {
                    println!("Audio monitor has received an interrupt signal");
                    break;
                }
                event_result = device_changes.recv() => {
                    match event_result {
                        Ok(event) => {
                            let message = match event {
                                DeviceChangeEvent::DefaultInputChanged(device) =>
                                    format!("default_input_changed:{}", serde_json::to_string(&device).unwrap()),
                                DeviceChangeEvent::DefaultOutputChanged(device) =>
                                    format!("default_output_changed:{}", serde_json::to_string(&device).unwrap()),
                                DeviceChangeEvent::VolumeMuteChanged(device) =>
                                    format!("volume_mute_changed:{}", serde_json::to_string(&device).unwrap()),
                            };
                            let _ = ws_tx_clone.send(message);
                        }
                        Err(e) => {
                            eprintln!("Error getting audio device changes: {}", e);
                        }
                    }
                }
            }
        }
        println!("  >> [AUDIO_MON] Task exiting");
    });

    let (socket_stop_tx, _) = broadcast::channel::<()>(1);
    let (bat_mon_stop_tx, _) = broadcast::channel::<()>(1);
    let (device_mon_stop_tx, _) = broadcast::channel::<()>(1);
    let device_manager = Arc::new(Mutex::new(DeviceManager::new(device_mon_stop_tx.clone())?));
    let ws_tx_clone = sock_tx.clone();
    let app_state = Arc::new(Mutex::new(AppState {
        audio_api,
        steam_api,
        launcher: launcher.clone(),
        active_game_session: None,
        sock_tx,
        socket_stop_tx: socket_stop_tx.clone(),
        active_backend: None,
        device_manager: device_manager.clone(),
        backend_type: BackendType::Unknown,
        battery_monitor: BatteryMonitor::new(ws_tx_clone, device_manager.clone(), bat_mon_stop_tx.clone()),
        overlay_manager: WlxOverlayManager::new(),
        log_session: None,
        launch_requests: HashSet::new(),
    }));

    launcher.set_app_state_async(app_state.clone()).await;

    let app_state_clone = app_state.clone();
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
        .route("/api/audio/device/{endpoint_id}/volume", post(routes::audio::set_audio_endpoint_volume))
        .route("/api/sock", get(routes::sock::sock_state_handler))
        .route("/api/device/battery", get(routes::device::get_battery_status))
        .route("/api/debug/agent", get(routes::debug::get_user_agent))
        .route("/{path}", get(routes::frontend::get_frontend_asset))
        .fallback(get(routes::frontend::get_frontend_asset))
        .layer(tower_http::set_header::SetResponseHeaderLayer::appending(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        ))
        .layer(CorsLayer::very_permissive())
        .with_state(app_state);

    let shutdown_signal = async move {
        tokio::select! {
            _ = signal::ctrl_c() => {
                println!("Received Ctrl+C signal, shutting down gracefully...");
            }
            _ = shutdown_rx.recv() => {
                println!("Shutting down...");
            }
        }
    };

    let listen_address = "0.0.0.0:3001";
    println!("Listening on http://{}/", listen_address);
    let listener = tokio::net::TcpListener::bind(listen_address).await?;
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal);

    match server.await {
        Ok(_) => println!("The web listener has stopped"),
        Err(e) => println!("The server failed to start: {}", e),
    }

    let mut app_state = app_state_clone.lock().await;
    app_state.shutdown_async().await?;
    _ = audio_monitor_stop_tx.send(());
    _ = bat_mon_stop_tx.send(());
    _ = device_mon_stop_tx.send(());
    _ = socket_stop_tx.send(());

    #[cfg(debug_assertions)]
    {
        println!("Sent all interrupt signals, waiting for 1 second...");
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let handle = tokio::runtime::Handle::current();
        println!("Active tokio tasks: {}", handle.metrics().num_alive_tasks());
    }

    Ok(())
}