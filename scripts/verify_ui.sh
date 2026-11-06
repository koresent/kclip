#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SCREENSHOT_PATH="${1:-/tmp/kclip_verified.png}"

echo "=== kclip Visual & Function Verification ==="

# 1. Check daemon status
echo "1. Checking daemon status..."
if ! kclip status >/dev/null 2>&1; then
    echo "Starting kclip daemon via systemd..."
    systemctl --user start kclip.service
    sleep 0.5
fi
kclip status

# 2. Feed a sample image and color if needed (optional)
if [ "${FEED_SAMPLE:-0}" = "1" ]; then
    echo "2. Feeding sample color and code..."
    wl-copy "#34c759"
    sleep 0.2
    wl-copy "pub fn kclip_style() -> bool { true }"
    sleep 0.3
fi

# 3. Launch UI briefly and take screenshot
echo "3. Launching kclip UI..."
kclip &
KCLIP_PID=$!
sleep 0.8

echo "4. Capturing window screenshot..."
"$SCRIPT_DIR/capture_window.sh" "$SCREENSHOT_PATH"

echo "5. Dismissing UI..."
kill $KCLIP_PID || true

echo "=== Verification complete: Screenshot saved to $SCREENSHOT_PATH ==="
