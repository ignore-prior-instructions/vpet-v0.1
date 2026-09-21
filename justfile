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

art-check:
    cd tools/spritekit && uv run spritekit validate --all
    cd tools/spritekit && uv run spritekit compile --check

golden:
    cargo run -p vpet-cli -- replay tests/golden/*.vlog

bless:
    cargo run -p vpet-cli -- replay --bless tests/golden/*.vlog

# Typecheck and production-build the browser host. Needs `just wasm` first so vpet.wasm exists
# under hosts/web/public/ (the build copies it into dist/).
web-check: wasm
    cd hosts/web && npm ci && npm run typecheck && npm run build

# docs/TESTING.md "Web smoke": load the page, press A/B/C, screenshot the canvas. Runs against
# the production build web-check just produced.
web-e2e: web-check
    cd hosts/web && npx playwright install --with-deps chromium && npm run test:e2e

ci: fmt clippy nofloat test size imports golden art-check web-e2e
