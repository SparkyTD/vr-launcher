use crate::app_state::AppStateWrapper;
use crate::backends::BackendType;
use crate::models::{establish_connection, Game};
use crate::schema::games::dsl::games;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct LaunchQuery {
    idem_token: String,
}

pub async fn launch_game_async(
    State(app_state): State<AppStateWrapper>,
    Path(game_id): Path<String>,
    query: Query<LaunchQuery>,
) -> impl IntoResponse + Send {
    let mut app_state = app_state.lock().await;
    if app_state.launch_requests.contains(&query.idem_token) {
        return StatusCode::NO_CONTENT.into_response();
    }
    app_state.launch_requests.insert(query.idem_token.clone());

    let connection = &mut establish_connection();
    let mut result = games
        .select(Game::as_select())
        .find(game_id)
        .load(connection)
        .expect("Error loading games");

    println!("[Axum/HTTP] Handling launch request");

    match result.pop() {
        Some(game) => {
            match app_state.launch_game_async(game).await {
                Ok(_) => {
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::empty())
                        .unwrap()
                }
                Err(error) => {
                    println!("{:?}", error);
                    Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(error.to_string()))
                        .unwrap()
                },
            }
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn kill_active_game(
    State(app_state): State<AppStateWrapper>,
) -> impl IntoResponse {
    let mut app_state = app_state.lock().await;
    match &mut app_state.active_game_session {
        Some(_) => {
            match app_state.kill_active_game() {
                Ok(_) => StatusCode::NO_CONTENT.into_response(),
                Err(error) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(error.to_string()))
                    .unwrap(),
            }
        }
        None => StatusCode::NO_CONTENT.into_response()
    }
}

pub async fn get_active_game(State(_app_state): State<AppStateWrapper>) -> impl IntoResponse {
    let app_state = _app_state.lock().await;
    match &app_state.active_game_session {
        Some(game_session) => format!("{}", serde_json::to_string(&game_session).unwrap()),
        None => "{}".into(),
        /*None => format!("{}", serde_json::to_string(&crate::GameSession {
            start_time_epoch: 0,
            process_handle: crate::steam::launcher::ProcessHandle::null(),
            game: Game {
                id: "01JXVDSX3DZ4TACG949D1Z14XW".into(),
                steam_app_id: Some(450390),
                title: "Test Game".into(),
                vr_backend: "wivrn".into(),
                command_line: None,
                proton_version: None,
                cover: None,
                total_playtime_sec: 0,
            },
            vr_device_serial: "1WMHHA67UU2191".into(),
        }).unwrap())*/
    }
}

pub async fn reload_backend(State(app_state): State<AppStateWrapper>) -> impl IntoResponse {
    let mut app_state = app_state.lock().await;

    if app_state.active_game_session.is_none() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from("No active game session found"))
            .unwrap();
    }

    let device_manager = app_state.device_manager.clone();

    match &app_state.backend_type {
        BackendType::WiVRn | BackendType::Envision => {
            match app_state.active_backend.as_mut() {
                Some(backend) => {
                    match backend.reconnect_async(device_manager).await {
                        Ok(_) => StatusCode::OK.into_response(),
                        Err(error) => Response::builder()
                            .status(StatusCode::OK)
                            .body(Body::from(error.to_string()))
                            .unwrap(),
                    }
                }
                None => Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::from("There is no active backend"))
                    .unwrap(),
            }
        }
        _ => todo!("Reloading this backend is not implemented: {:?}", app_state.backend_type),
    }
}