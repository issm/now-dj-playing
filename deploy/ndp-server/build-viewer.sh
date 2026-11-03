#!/usr/bin/env bash
# viewer-web（ブラウザ版 viewer）のビルド
#
# 成果物: apps/viewer/dist-web/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VIEWER_DIR="$PROJECT_ROOT/apps/viewer"

echo "==> ソース: $VIEWER_DIR"

cd "$VIEWER_DIR"

# /v/ の viewer から、同じホストの /api/* を呼ぶ（同一オリジン）ため、サーバー URL は空にする。
# 環境変数は .env.web より優先される
VITE_NDP_SERVER_URL="" yarn build:web

echo "==> ビルド完了: $VIEWER_DIR/dist-web"
ls -lh "$VIEWER_DIR/dist-web"
