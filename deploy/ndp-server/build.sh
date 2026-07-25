#!/usr/bin/env bash
# ndp-server クロスコンパイル (cargo-zigbuild + musl)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SERVER_DIR="$PROJECT_ROOT/apps/server"
TARGET="x86_64-unknown-linux-musl"
OUTPUT_DIR="$SCRIPT_DIR/dist"

echo "==> ビルドターゲット: $TARGET"
echo "==> ソース: $SERVER_DIR"

# rustup ターゲット追加（未追加の場合のみ）
rustup target add "$TARGET" 2>/dev/null || true

# ビルド
cd "$SERVER_DIR"
cargo zigbuild --release --target "$TARGET"

# 成果物をコピー
mkdir -p "$OUTPUT_DIR"
cp "$SERVER_DIR/target/$TARGET/release/ndp-server" "$OUTPUT_DIR/ndp-server"

echo "==> ビルド完了: $OUTPUT_DIR/ndp-server"
ls -lh "$OUTPUT_DIR/ndp-server"
