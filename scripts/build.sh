#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release -j "${GLIDE_BUILD_JOBS:-3}"
cargo test --locked -j "${GLIDE_BUILD_JOBS:-3}"
