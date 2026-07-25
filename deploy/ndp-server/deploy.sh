#!/usr/bin/env bash
# ndp-server デプロイ（scp 転送 + systemd 再起動）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# 設定読み込み
if [ -f "$SCRIPT_DIR/.env" ]; then
    # shellcheck disable=SC1091
    source "$SCRIPT_DIR/.env"
else
    echo "エラー: $SCRIPT_DIR/.env が見つかりません。.env.example を参考に作成してください。"
    exit 1
fi

BINARY="$SCRIPT_DIR/dist/ndp-server"
REMOTE_USER="${DEPLOY_USER:-admin}"
REMOTE_HOST="${DEPLOY_HOST:?DEPLOY_HOST が未設定です}"
REMOTE_DIR="${DEPLOY_DIR:-/opt/ndp-server}"
SSH_KEY="${DEPLOY_SSH_KEY:-}"
SERVICE_NAME="ndp-server"

# SSH オプション構築
SSH_OPTS=()
if [ -n "$SSH_KEY" ]; then
    SSH_OPTS+=(-i "$SSH_KEY")
fi

# バイナリ存在チェック
if [ ! -f "$BINARY" ]; then
    echo "エラー: $BINARY が見つかりません。先に build.sh を実行してください。"
    exit 1
fi

echo "==> デプロイ先: ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_DIR}"

# バイナリ転送
echo "==> バイナリ転送中..."
scp "${SSH_OPTS[@]}" "$BINARY" "${REMOTE_USER}@${REMOTE_HOST}:/tmp/ndp-server"

# リモートで配置 + サービス再起動
echo "==> サービス再起動中..."
ssh "${SSH_OPTS[@]}" "${REMOTE_USER}@${REMOTE_HOST}" bash -s <<EOF
set -euo pipefail
sudo mv /tmp/ndp-server ${REMOTE_DIR}/ndp-server
sudo chmod +x ${REMOTE_DIR}/ndp-server
sudo systemctl restart ${SERVICE_NAME}
echo "==> デプロイ完了"
sudo systemctl status ${SERVICE_NAME} --no-pager
EOF
