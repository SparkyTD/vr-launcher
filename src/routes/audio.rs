use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use serde::Deserialize;
use crate::app_state::AppStateWrapper;

pub async fn get_audio_endpoints(
    State(app_state): State<AppStateWrapper>,
    Path(endpoint): Path<String>
) -> impl IntoResponse {
    let app_state = app_state.lock().await;
    let mut devices = match endpoint.as_str() {
        "inputs" => app_state.audio_api.get_input_devices(),
        "outputs" => app_state.audio_api.get_output_devices(),
        _ => panic!("Bad request"),
    }.into_iter().collect::<Vec<_>>();

    devices.sort_by_key(|d| d.name.clone());

    Json(devices)
}

pub async fn set_default_audio_endpoint(
    State(app_state): State<AppStateWrapper>,
    Path((endpoint, endpoint_id)): Path<(String, u32)>
) -> impl IntoResponse {
    let app_state = app_state.lock().await;
    let devices = match endpoint.as_str() {
        "inputs" => app_state.audio_api.get_input_devices(),
        "outputs" => app_state.audio_api.get_output_devices(),
        _ => panic!("Bad request"),
    };

    match devices.iter().find(|d| d.id == endpoint_id) {
        Some(device) => {
            match endpoint.as_str() {
                "inputs" => app_state.audio_api.set_default_input_device(device),
                "outputs" => app_state.audio_api.set_default_output_device(device),
                _ => panic!("Bad request"),
            }
            StatusCode::NO_CONTENT.into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn set_audio_endpoint_volume(
    State(app_state): State<AppStateWrapper>,
    Path(endpoint_id): Path<u32>,
    Json(payload): Json<AudioVolumeControl>
) -> impl IntoResponse {
    let app_state = app_state.lock().await;
    let mut all_devices = app_state.audio_api.get_input_devices().clone();
    all_devices.extend(app_state.audio_api.get_output_devices());

    let device = all_devices.iter().find(|d| d.id == endpoint_id);
    if let Some(device) = device {
        app_state.audio_api.set_device_volume(device, payload.volume, payload.muted);
    }

    StatusCode::OK.into_response()
}

#[derive(Deserialize, Debug)]
pub struct AudioVolumeControl {
    volume: u8,
    muted: bool,
}