#!/usr/bin/env bash
# Build the WASM engine into web/pkg. Re-extract assets with:
#   python3 -I tools/unity4/extract.py <merged Unity Data dir> web/assets
set -euo pipefail
cd "$(dirname "$0")"
cargo build --manifest-path engine/Cargo.toml --release --target wasm32-unknown-unknown --lib
wasm-bindgen --target web --out-dir web/pkg --no-typescript \
  engine/target/wasm32-unknown-unknown/release/squarelights.wasm
ls -la web/pkg
