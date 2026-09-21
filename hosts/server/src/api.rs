//! The four endpoints from docs/SYNC.md "Server API", plus health. The server never runs the
//! simulation for play (ADR 0007); it links `vpet-core` only to reject blobs that don't load.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use base64::Engine;
use serde::Serialize;
use subtle::ConstantTimeEq;
use tower_http::cors::CorsLayer;
use vpet_core::save::LoadError;
use vpet_core::Cart;

use crate::store::{valid_id, Store};

pub const SIM_NOW_HEADER: &str = "x-sim-now";

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    pub token: Arc<String>,
}

/// The GET body; also the 409 body so a client can adopt the newer save without a second
/// round trip (docs/hosts/server.md).
#[derive(Debug, Serialize)]
pub struct PetBody {
    pub sim_now: u32,
    pub seq: u64,
    pub saved_at: String,
    pub blob_b64: String,
}

#[derive(Debug, Serialize)]
struct SeqBody {
    seq: u64,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

fn error(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(ErrorBody { error: msg.into() })).into_response()
}

fn internal(e: impl std::fmt::Display) -> Response {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("storage error: {e}"),
    )
}

pub fn router(state: AppState, cors_origins: Vec<HeaderValue>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([Method::GET, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::HeaderName::from_static(SIM_NOW_HEADER),
        ]);

    let pets = Router::new()
        .route(
            "/v1/pets/{id}",
            get(get_pet).put(put_pet).delete(delete_pet),
        )
        .route_layer(middleware::from_fn_with_state(state.clone(), auth));

    Router::new()
        .route("/v1/health", get(|| async { "ok" }))
        .merge(pets)
        .layer(cors)
        .with_state(state)
}

/// `Authorization: Bearer <VPET_TOKEN>` on every pets route; constant-time compare.
async fn auth(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let presented = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    let ok = presented.len() == state.token.len()
        && presented.as_bytes().ct_eq(state.token.as_bytes()).into();
    if !ok {
        return error(StatusCode::UNAUTHORIZED, "missing or invalid bearer token");
    }
    next.run(req).await
}

fn pet_body(stored: &crate::store::Stored) -> PetBody {
    PetBody {
        sim_now: stored.meta.sim_now,
        seq: stored.meta.seq,
        saved_at: stored.meta.saved_at.clone(),
        blob_b64: base64::engine::general_purpose::STANDARD.encode(&stored.blob),
    }
}

async fn get_pet(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    if !valid_id(&id) {
        return error(StatusCode::BAD_REQUEST, "invalid pet id");
    }
    match state.store.get(&id).await {
        Ok(Some(stored)) => Json(pet_body(&stored)).into_response(),
        Ok(None) => error(StatusCode::NOT_FOUND, "no such pet"),
        Err(e) => internal(e),
    }
}

async fn put_pet(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !valid_id(&id) {
        return error(StatusCode::BAD_REQUEST, "invalid pet id");
    }
    let Some(declared) = headers
        .get(SIM_NOW_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u32>().ok())
    else {
        return error(
            StatusCode::BAD_REQUEST,
            "missing or invalid X-Sim-Now header",
        );
    };

    // Validate by actually loading it into a throwaway core (docs/SYNC.md).
    let mut cart = Cart::new_uninit();
    if let Err(e) = cart.load(&body) {
        let msg = match e {
            LoadError::VersionTooNew => {
                "blob is from a newer core than this server was built with; the server needs \
                 rebuilding (keep playing locally)"
                    .to_string()
            }
            other => format!("blob rejected: {other:?}"),
        };
        return error(StatusCode::UNPROCESSABLE_ENTITY, msg);
    }
    let insp = cart.inspect();
    if insp.sim_now != declared {
        return error(
            StatusCode::BAD_REQUEST,
            format!(
                "X-Sim-Now ({declared}) does not match the blob's sim_now ({})",
                insp.sim_now
            ),
        );
    }

    let lock = state.store.lock(&id).await;
    let _guard = lock.lock().await;
    match state.store.get(&id).await {
        Ok(Some(existing)) if existing.meta.sim_now > declared => {
            (StatusCode::CONFLICT, Json(pet_body(&existing))).into_response()
        }
        Ok(_) => match state
            .store
            .put(&id, &body, declared, insp.content_hash)
            .await
        {
            Ok(meta) => Json(SeqBody { seq: meta.seq }).into_response(),
            Err(e) => internal(e),
        },
        Err(e) => internal(e),
    }
}

async fn delete_pet(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    if !valid_id(&id) {
        return error(StatusCode::BAD_REQUEST, "invalid pet id");
    }
    let lock = state.store.lock(&id).await;
    let _guard = lock.lock().await;
    match state.store.delete(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => internal(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    const TOKEN: &str = "test-token";

    fn app() -> (Router, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            store: Arc::new(Store::new(dir.path())),
            token: Arc::new(TOKEN.to_string()),
        };
        (
            router(
                state,
                vec![HeaderValue::from_static("http://localhost:5173")],
            ),
            dir,
        )
    }

    /// A real save from the real core, `secs` after reset.
    fn blob_at(secs: u64) -> (Vec<u8>, u32) {
        let mut cart = Cart::new_uninit();
        cart.reset(1_700_000_000_000, 42);
        cart.update(1_700_000_000_000 + secs * 1000, 0);
        let mut scratch = [0u8; 512];
        let mut out = [0u8; 512];
        let len = cart.save(&mut scratch, &mut out).unwrap();
        (out[..len].to_vec(), cart.inspect().sim_now)
    }

    fn put(id: &str, blob: &[u8], sim_now: u32, token: Option<&str>) -> Request<Body> {
        let mut b = Request::builder()
            .method("PUT")
            .uri(format!("/v1/pets/{id}"))
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .header(SIM_NOW_HEADER, sim_now.to_string());
        if let Some(t) = token {
            b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        b.body(Body::from(blob.to_vec())).unwrap()
    }

    fn get(id: &str, token: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().uri(format!("/v1/pets/{id}"));
        if let Some(t) = token {
            b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        b.body(Body::empty()).unwrap()
    }

    async fn json(resp: Response) -> serde_json::Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn health_is_open() {
        let (app, _d) = app();
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/v1/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn pets_routes_need_the_token() {
        let (app, _d) = app();
        let resp = app.clone().oneshot(get("default", None)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let resp = app.oneshot(get("default", Some("wrong"))).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn put_then_get_round_trips() {
        let (app, _d) = app();
        let (blob, sim_now) = blob_at(10);
        let resp = app
            .clone()
            .oneshot(put("default", &blob, sim_now, Some(TOKEN)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(json(resp).await["seq"], 1);

        let resp = app.oneshot(get("default", Some(TOKEN))).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = json(resp).await;
        assert_eq!(body["sim_now"], sim_now);
        assert_eq!(body["seq"], 1);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(body["blob_b64"].as_str().unwrap())
            .unwrap();
        assert_eq!(decoded, blob);
    }

    #[tokio::test]
    async fn stale_put_is_a_conflict_carrying_the_newer_blob() {
        let (app, _d) = app();
        let (newer, newer_now) = blob_at(600);
        let (older, older_now) = blob_at(10);
        assert!(newer_now > older_now);
        let resp = app
            .clone()
            .oneshot(put("default", &newer, newer_now, Some(TOKEN)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let resp = app
            .clone()
            .oneshot(put("default", &older, older_now, Some(TOKEN)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
        let body = json(resp).await;
        assert_eq!(body["sim_now"], newer_now);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(body["blob_b64"].as_str().unwrap())
            .unwrap();
        assert_eq!(decoded, newer);

        // An equal sim_now is accepted: last writer wins, seq breaks the tie.
        let resp = app
            .oneshot(put("default", &newer, newer_now, Some(TOKEN)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(json(resp).await["seq"], 2);
    }

    #[tokio::test]
    async fn bad_blobs_and_bad_headers_are_rejected_without_writing() {
        let (app, _d) = app();
        let resp = app
            .clone()
            .oneshot(put("default", b"not a blob", 5, Some(TOKEN)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let (blob, sim_now) = blob_at(10);
        let resp = app
            .clone()
            .oneshot(put("default", &blob, sim_now + 1, Some(TOKEN)))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/v1/pets/default")
                    .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                    .body(Body::from(blob.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST); // no X-Sim-Now

        let resp = app
            .clone()
            .oneshot(put("..%2Fetc", &blob, sim_now, Some(TOKEN)))
            .await
            .unwrap();
        assert!(resp.status().is_client_error());

        let resp = app.oneshot(get("default", Some(TOKEN))).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND); // nothing was written
    }

    #[tokio::test]
    async fn delete_then_get_is_404() {
        let (app, _d) = app();
        let (blob, sim_now) = blob_at(10);
        app.clone()
            .oneshot(put("default", &blob, sim_now, Some(TOKEN)))
            .await
            .unwrap();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/v1/pets/default")
                    .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
        let resp = app.oneshot(get("default", Some(TOKEN))).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
