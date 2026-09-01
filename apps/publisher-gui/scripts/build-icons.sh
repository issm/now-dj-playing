#!/usr/bin/env bash
# publisher-gui アプリアイコンの生成
#
# assets/app-icon-master.png (元画像) から macOS 向けの 1024x1024 ソース画像を作り、
# tauri CLI で src-tauri/icons/ 配下のアイコン一式を生成する。
#
# 元画像は squircle（角丸正方形）デザイン + 透過背景。
# macOS のアイコンは 1024 キャンバスに対して本体 824 程度の余白を持たせるのが標準なので、
# 824x824 に縮小して 1024 キャンバスの中央に配置する。
#
# 必要なコマンド:
#   - magick (ImageMagick 7)
#   - cargo-tauri (cargo install tauri-cli)
set -euo pipefail

cd "$(dirname "$0")/.."

MASTER="assets/app-icon-master.png"
SOURCE="assets/app-icon-1024.png"

if [[ ! -f "$MASTER" ]]; then
  echo "元画像が見つかりません: $MASTER" >&2
  exit 1
fi

magick "$MASTER" \
  -resize 824x824 \
  -background none -gravity center -extent 1024x1024 \
  -define png:color-type=6 \
  "$SOURCE"

echo "ソース画像を生成しました: $SOURCE"

cargo tauri icon "$SOURCE"

# 本アプリは macOS 専用のため、モバイル向けと未参照の生成物は破棄する。
# (iOS 向けは gen/ 配下に出力され、gen/ は .gitignore 済み)
rm -rf src-tauri/icons/android
rm -f src-tauri/icons/64x64.png

echo "アイコン一式を生成しました: src-tauri/icons/"
