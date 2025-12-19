use axum::body::Body;
use axum::http::{StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use crate::{get_asset, BundledContent};

pub async fn get_frontend_asset(uri: Uri) -> impl IntoResponse {
    let path = uri.path();

    let asset: Option<BundledContent> = match path.trim_start_matches("/") {
        "/" | "" => get_asset("index.html"),
        "favicon.ico" => get_favicon(),
        path => get_asset(&path),
    };

    match asset {
        None => StatusCode::NOT_FOUND.into_response(),
        Some(asset) => {
            // Create response with proper MIME type
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", &asset.mime_type)
                .body(Body::from(asset.data.clone()))
                .unwrap();

            response
        }
    }
}

fn get_favicon() -> Option<BundledContent> {
    let tray_icon_bytes = include_bytes!("../../icon.png");
    Some(BundledContent {
        data: tray_icon_bytes.into(),
        mime_type: "image/png".into(),
    })
}