#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
python3 scripts/portable/build-native.py
cargo fmt --check
cargo clippy --locked --all-targets -j "${GLIDE_BUILD_JOBS:-2}" -- -D warnings
cargo test --locked -j "${GLIDE_BUILD_JOBS:-2}"
cargo build --locked --release -j "${GLIDE_BUILD_JOBS:-2}"
python3 scripts/portable/prepare.py
python3 scripts/portable/package.py
