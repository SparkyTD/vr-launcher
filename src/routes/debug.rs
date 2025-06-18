use axum::response::IntoResponse;
use axum_extra::headers::UserAgent;
use axum_extra::TypedHeader;

pub async fn get_user_agent(user_agent: Option<TypedHeader<UserAgent>>) -> impl IntoResponse {
    format!("{:?}", user_agent)
}