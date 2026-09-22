# See docs/ROADMAP.md and docs/TESTING.md. Run `just ci` for everything CI checks.

# vpet-abi is excluded from the workspace's default-members (see root Cargo.toml) because it
# only makes sense built for wasm32-unknown-unknown; these recipes build/test/lint it
# explicitly for that target where relevant.

test:
    cargo test

clippy:
    cargo clippy --all-targets -- -D warnings
    cargo clippy -p vpet-abi --target wasm32-unknown-unknown -- -D warnings

fmt:
    cargo fmt --all -- --check

nofloat:
    ! grep -rnE '\bf(32|64)\b' crates/vpet-core/src

wasm:
    cargo build -p vpet-abi --release --target wasm32-unknown-unknown
    mkdir -p target/dist hosts/web/public
    wasm-opt -Oz -o target/dist/vpet.wasm target/wasm32-unknown-unknown/release/vpet_abi.wasm
    cp target/dist/vpet.wasm hosts/web/public/vpet.wasm

size: wasm
    @len=$(wc -c < target/dist/vpet.wasm); \
    echo "vpet.wasm: $len bytes (limit 65536)"; \
    test "$len" -le 65536

imports: wasm
    @n=$(wasm-tools print target/dist/vpet.wasm | grep -c '(import' || true); \
    echo "imports: $n (must be 0)"; \
    test "$n" -eq 0

# docs/CONTENT.md "Hot preview without rebuilding the core": a separate wasm build with the
# `dev-overrides` feature (pose/global override overlay, `dev_*` exports), used only by
# `hosts/web`'s `/dev/sprites` page. Debug build for fast iteration; no size gate; never copied
# into `hosts/web/public/vpet.wasm` (the shipped artifact `just wasm` produces) or checked by
# `imports`/`size` -- this build is expected to have imports and be well over the 64 KB budget.
wasm-dev:
    cargo build -p vpet-abi --features dev-overrides --target wasm32-unknown-unknown
    mkdir -p hosts/web/public
    cp target/wasm32-unknown-unknown/debug/vpet_abi.wasm hosts/web/public/vpet-dev.wasm

# docs/hosts/server.md: the blob store, locally, with a throwaway token (override VPET_TOKEN).
server:
    VPET_TOKEN="${VPET_TOKEN:-dev}" cargo run -p vpet-server

art-check:
    cd tools/spritekit && uv run spritekit validate --all
    cd tools/spritekit && uv run spritekit compile --check

golden:
    cargo run -p vpet-cli -- replay tests/golden/*.vlog

bless:
    cargo run -p vpet-cli -- replay --bless tests/golden/*.vlog

# docs/TESTING.md "Cross-host parity": every .vlog through the native Cart and through the
# shipped vpet.wasm on the wasmi interpreter; flags, frames, blobs and Inspect must match.
parity: wasm
    cargo run -p vpet-cli --features parity -- parity --wasm target/dist/vpet.wasm tests/golden/*.vlog

# Typecheck, unit-test (docs/CONTENT.md's grid.ts/grid.py parser-parity test, against
# tests/fixtures/packed.json -- regenerate with `spritekit dev-fixture` after changing either
# parser), and production-build the browser host. Needs `just wasm` first so vpet.wasm exists
# under hosts/web/public/ (the build copies it into dist/).
web-check: wasm
    cd hosts/web && npm ci && npm run typecheck && npm run test:unit && npm run build

# docs/TESTING.md "Web smoke": load the page, press A/B/C, screenshot the canvas. Runs against
# the production build web-check just produced.
web-e2e: web-check
    cargo build -p vpet-server
    cd hosts/web && npx playwright install --with-deps chromium && npm run test:e2e

ci: fmt clippy nofloat test size imports golden parity art-check web-e2e
