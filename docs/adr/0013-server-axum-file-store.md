# 0013: Rust axum server with a file store

Status: accepted
Date: 2026-09-20

## Context

With the server reduced to a blob store, its language was free. Options were a Cloudflare Worker
with KV, a small Python service, or Rust.

## Decision

Rust with `axum`, one binary, blobs as files with atomic rename, bearer token from the
environment. The server links `vpet-core` to validate blobs on PUT.

## Consequences

- One language across the repo; the server can run the core for validation and, later, for a
  rendered status frame.
- Needs a box to run on (Fly, a VPS, a Pi) rather than an edge platform. Acceptable for one user.
- No database to operate.
