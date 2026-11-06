#!/usr/bin/env bash
set -euo pipefail

OUT_PATH="${1:-/tmp/kclip_screenshot.png}"

# Find kclip window in umbriel windows
# Format example: *kclip  kclip  [float 840x276+1500+1800]
GEOM=$(umbriel windows | grep -E "(kclip|\*kclip)" | grep -oE "\[float [0-9]+x[0-9]+\+[0-9]+\+[0-9]+\]" | head -n 1 || true)

if [ -z "$GEOM" ]; then
    echo "kclip window not found in umbriel windows, capturing full screen"
    grim "$OUT_PATH"
else
    # Parse W, H, X, Y
    CLEAN="${GEOM#\[float }"
    CLEAN="${CLEAN%\]}"
    # CLEAN is WxH+X+Y
    W=$(echo "$CLEAN" | cut -d'x' -f1)
    REST=$(echo "$CLEAN" | cut -d'x' -f2)
    H=$(echo "$REST" | cut -d'+' -f1)
    X=$(echo "$REST" | cut -d'+' -f2)
    Y=$(echo "$REST" | cut -d'+' -f3)

    echo "Capturing kclip window at $X,$Y ${W}x${H} to $OUT_PATH"
    grim -g "${X},${Y} ${W}x${H}" "$OUT_PATH"
fi

echo "Saved screenshot to $OUT_PATH"
