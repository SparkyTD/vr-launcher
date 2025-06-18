use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use crate::app_state::AppStateWrapper;

pub async fn get_battery_status(State(app_state): State<AppStateWrapper>) -> impl IntoResponse {
    let app_state = app_state.lock().await;
    let current_info = app_state.battery_monitor.get_battery_info_async().await;
    
    match current_info.as_ref() {
        Some(info) => Json(info).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}