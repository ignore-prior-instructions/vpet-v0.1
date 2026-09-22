//! `vpet-server`: a single binary that stores one save blob per pet and resolves conflicts by
//! the blob's own simulated time (docs/SYNC.md, docs/hosts/server.md, ADR 0007, ADR 0013). It
//! links `vpet-core` only to validate blobs; it never runs the simulation for play.

mod api;
mod store;

use std::sync::Arc;

use axum::http::HeaderValue;

/// Origins allowed by default: the published web host and the Vite dev server
/// (docs/hosts/server.md "CORS"). Override with `VPET_CORS_ORIGINS` (comma-separated).
const DEFAULT_ORIGINS: &str = "https://ignore-prior-instructions.github.io,http://localhost:5173";

#[tokio::main]
async fn main() {
    let token = match std::env::var("VPET_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            eprintln!("VPET_TOKEN must be set to a non-empty bearer token (docs/hosts/server.md)");
            std::process::exit(2);
        }
    };
    let data = std::env::var("VPET_DATA").unwrap_or_else(|_| "./data".to_string());
    let bind = std::env::var("VPET_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let origins: Vec<HeaderValue> = std::env::var("VPET_CORS_ORIGINS")
        .unwrap_or_else(|_| DEFAULT_ORIGINS.to_string())
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| HeaderValue::from_str(s).ok())
        .collect();

    let state = api::AppState {
        store: Arc::new(store::Store::new(&data)),
        token: Arc::new(token),
    };
    let app = api::router(state, origins);

    let listener = match tokio::net::TcpListener::bind(&bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind {bind}: {e}");
            std::process::exit(1);
        }
    };
    eprintln!("vpet-server listening on {bind}, data in {data}");
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
