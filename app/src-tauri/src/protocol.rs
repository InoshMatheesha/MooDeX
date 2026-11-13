// protocol.rs - moodex-img://<hash>/<variant>: covers streamed from the cache, no base64 over IPC (spec 10).

use moodex_images::{ImageCache, Variant};
use tauri::http::{header, Request, Response, StatusCode};

fn reply(status: StatusCode, ctype: &str, body: Vec<u8>, cache: bool) -> Response<Vec<u8>> {
    let mut b = Response::builder().status(status).header(header::CONTENT_TYPE, ctype);
    if cache {
        // Content-addressed: a hash never changes meaning, so the browser may keep it forever.
        b = b.header(header::CACHE_CONTROL, "public, max-age=31536000, immutable");
    }
    b.body(body.clone()).unwrap_or_else(|_| Response::new(body))
}

// Path is "/<64-hex hash>/<thumb|hero>".
pub async fn handle(images: ImageCache, req: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = req.uri().path().trim_matches('/').to_string();
    let mut parts = path.split('/');
    let (Some(hash), Some(variant), None) = (parts.next(), parts.next(), parts.next()) else {
        return reply(StatusCode::NOT_FOUND, "text/plain", b"not found".to_vec(), false);
    };
    let Some(variant) = Variant::parse(variant) else {
        return reply(StatusCode::NOT_FOUND, "text/plain", b"unknown variant".to_vec(), false);
    };
    match images.load(hash, variant).await {
        Ok(bytes) => reply(StatusCode::OK, "image/webp", bytes.as_ref().clone(), true),
        Err(moodex_images::ImageError::Unknown(_)) => {
            reply(StatusCode::NOT_FOUND, "text/plain", b"unknown image".to_vec(), false)
        }
        Err(e) => {
            tracing::debug!(%hash, error = %e, "cover unavailable");
            reply(StatusCode::BAD_GATEWAY, "text/plain", e.to_string().into_bytes(), false)
        }
    }
}
