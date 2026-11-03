#!/usr/bin/env bash
# viewer-web（ブラウザ版 viewer）のデプロイ（rsync による完全同期）
#
# apps/viewer/dist-web/ の内容を、リモートの ${DEPLOY_DIR}/viewer/ に同期する。
# 手元にないファイルは、リモートからも削除される（--delete）。
#
# 使い方:
#   ./deploy-viewer.sh            # 同期する
#   ./deploy-viewer.sh --dry-run  # 変更内容の確認のみ（リモートは変更しない）
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# 設定読み込み
if [ -f "$SCRIPT_DIR/.env" ]; then
    # shellcheck disable=SC1091
    source "$SCRIPT_DIR/.env"
else
    echo "エラー: $SCRIPT_DIR/.env が見つかりません。.env.example を参考に作成してください。"
    exit 1
fi

SOURCE_DIR="$PROJECT_ROOT/apps/viewer/dist-web"
REMOTE_USER="${DEPLOY_USER:-admin}"
REMOTE_HOST="${DEPLOY_HOST:?DEPLOY_HOST が未設定です}"
REMOTE_DIR="${DEPLOY_DIR:-/opt/ndp-server}"
REMOTE_VIEWER_DIR="${DEPLOY_VIEWER_DIR:-${REMOTE_DIR}/viewer}"
SSH_KEY="${DEPLOY_SSH_KEY:-}"

# 引数
DRY_RUN=0
case "${1:-}" in
    "") ;;
    --dry-run|-n) DRY_RUN=1 ;;
    *)
        echo "使い方: $0 [--dry-run]"
        exit 1
        ;;
esac

# 同期元のチェック
# （空のディレクトリを --delete で同期すると、リモートの内容がすべて消えてしまう）
if [ ! -f "$SOURCE_DIR/index.html" ]; then
    echo "エラー: $SOURCE_DIR/index.html が見つかりません。先に build-viewer.sh を実行してください。"
    exit 1
fi

# 同期先のチェック（--delete で消す範囲を誤らないよう、絶対パスで、少なくとも 2 階層以上を要求する）
case "$REMOTE_VIEWER_DIR" in
    /*/*) ;;
    *)
        echo "エラー: 同期先 '$REMOTE_VIEWER_DIR' が不正です（/opt/ndp-server/viewer のような 2 階層以上の絶対パスにしてください）。"
        exit 1
        ;;
esac

# SSH コマンド（鍵が指定されていれば -i を付ける）
# 配列を使わないのは、macOS 標準の bash 3.2 で、空の配列を set -u の下で展開すると失敗するため
ssh_cmd() {
    if [ -n "$SSH_KEY" ]; then
        ssh -i "$SSH_KEY" "$@"
    else
        ssh "$@"
    fi
}
RSYNC_SSH="ssh"
if [ -n "$SSH_KEY" ]; then
    RSYNC_SSH="ssh -i $SSH_KEY"
fi

echo "==> 同期元: $SOURCE_DIR/"
echo "==> 同期先: ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_VIEWER_DIR}/"
if [ "$DRY_RUN" -eq 1 ]; then
    echo "==> dry-run: リモートは変更しません"
fi

# リモートの準備（rsync の有無の確認と、同期先ディレクトリの作成）
if ! ssh_cmd "${REMOTE_USER}@${REMOTE_HOST}" "command -v rsync >/dev/null"; then
    echo "エラー: リモートに rsync がありません。次を実行してください:"
    echo "  ssh ${REMOTE_USER}@${REMOTE_HOST} sudo apt install -y rsync"
    exit 1
fi
if [ "$DRY_RUN" -eq 0 ]; then
    ssh_cmd "${REMOTE_USER}@${REMOTE_HOST}" "sudo install -d -m 755 '${REMOTE_VIEWER_DIR}'"
fi

# 完全同期（手元にないファイルはリモートからも削除する）
# - リモート側は sudo rsync で書き込む（同期先は root 所有のまま。Caddy は読み取りのみ）
# - -c: ファイル名にハッシュが付く成果物は、内容が同じなら再転送しない
RSYNC_OPTS="-rlcz --delete --itemize-changes"
if [ "$DRY_RUN" -eq 1 ]; then
    RSYNC_OPTS="$RSYNC_OPTS --dry-run"
fi

echo "==> 同期中..."
# shellcheck disable=SC2086
rsync $RSYNC_OPTS \
    -e "$RSYNC_SSH" \
    --rsync-path="sudo rsync" \
    "$SOURCE_DIR/" \
    "${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_VIEWER_DIR}/"

if [ "$DRY_RUN" -eq 1 ]; then
    echo "==> dry-run 完了（上の一覧が、実行時に変更される内容です）"
    exit 0
fi

echo "==> デプロイ完了"

# 配信されているか確認（Caddyfile を更新していないと、ここで失敗する）
if [ -n "${DEPLOY_DOMAIN:-}" ]; then
    URL="https://${DEPLOY_DOMAIN}/v/"
    if curl -fsS -o /dev/null -w "==> 確認: ${URL} -> HTTP %{http_code}\n" "$URL"; then
        :
    else
        echo "警告: ${URL} を取得できませんでした。Caddyfile を更新して reload したか確認してください（README の「viewer-web のデプロイ」を参照）。"
    fi
fi
