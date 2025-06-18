use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper};
use crate::models::{establish_connection, Game};
use crate::schema::games::dsl::*;

pub async fn list_games() -> Json<Vec<Game>> {
    let connection = &mut establish_connection();
    let results = games
        .select(Game::as_select())
        .load(connection)
        .expect("Error loading games");

    Json(results)
}

pub async fn get_game_info(Path(game_id): Path<String>) -> impl IntoResponse {
    let connection = &mut establish_connection();
    let result = games
        .select(Game::as_select())
        .find(game_id)
        .load(connection)
        .expect("Error loading games");

    match result.first() {
        Some(game) => Json(game).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

pub async fn get_game_cover(Path(game_id): Path<String>) -> impl IntoResponse {
    let connection = &mut establish_connection();
    let result = games
        .select(Game::as_select())
        .find(game_id)
        .load(connection)
        .expect("Error loading games");

    match result.first().and_then(|g| g.cover.as_ref()) {
        Some(cover_data) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "image/jpg")
            .header(header::CONTENT_LENGTH, cover_data.len())
            .body(Body::from(cover_data.clone()))
            .unwrap()
            .into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}