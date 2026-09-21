# 0005: Versioned postcard save with header and CRC

Status: accepted
Date: 2026-09-20

## Context

The blob must be small, `no_std`-encodable on the device, and survive schema changes over years.

## Decision

Header `"VPET" | version u16 | content_hash u32 | len u16`, payload `postcard(SaveVn)`, trailing
CRC32. Each version is a frozen struct; loading converts forward. A newer version than the core
knows returns `VERSION_TOO_NEW` and the host must not overwrite the stored blob.

## Consequences

- About 150 bytes per save; no allocation.
- Postcard is not self-describing: every persisted struct edit needs a version bump, enforced by
  a schema snapshot test.
- Old firmware cannot clobber a newer save.
