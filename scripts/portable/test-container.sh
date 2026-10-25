#!/usr/bin/env bash
# Execute inside a clean distro container with only desktop runtime/test tools.
set -euo pipefail
cd /tmp
GLIDE_TEST_DIR=$(mktemp -d /tmp/glide-portable.XXXXXX)
trap 'rm -rf "$GLIDE_TEST_DIR"' EXIT
cd "$GLIDE_TEST_DIR"
"$1" --appimage-extract > /dev/null
GLIDE_TEST_APP="$GLIDE_TEST_DIR/squashfs-root/AppRun"
"$GLIDE_TEST_APP" --version
python3 /work/tests/network.py "$GLIDE_TEST_APP"
python3 /work/tests/settings.py "$GLIDE_TEST_APP"
dbus-run-session -- xvfb-run -a python3 /work/tests/portable-smoke.py "$GLIDE_TEST_APP"
