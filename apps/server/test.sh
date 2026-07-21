#!/bin/bash
# ndp-server の簡易動作テスト
# 前提: サーバが localhost:8080 で起動していること

set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
PASS=0
FAIL=0

green() { printf "\033[32m%s\033[0m\n" "$1"; }
red() { printf "\033[31m%s\033[0m\n" "$1"; }
dim() { printf "\033[2m%s\033[0m\n" "$1"; }

assert_status() {
  local label="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    green "  ✓ $label (HTTP $actual)"
    PASS=$((PASS + 1))
  else
    red "  ✗ $label (expected $expected, got $actual)"
    FAIL=$((FAIL + 1))
  fi
}

show_body() {
  echo "$1" | jq . 2>/dev/null || echo "$1"
  echo
}

echo "=== ndp-server テスト ($BASE_URL) ==="
echo

# --- Health ---
echo "[1] GET /health"
RESP=$(curl -s -w "\n%{http_code}" "$BASE_URL/health")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | sed '$d')
assert_status "ヘルスチェック" "200" "$STATUS"
show_body "$BODY"

# --- セッション作成 ---
echo "[2] POST /api/sessions/create"
RESP=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/sessions/create" \
  -H 'Content-Type: application/json' \
  -d '{"event_name": "Test Event"}')
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | sed '$d')
assert_status "セッション作成" "201" "$STATUS"
show_body "$BODY"

SESSION_ID=$(echo "$BODY" | jq -r '.session_id')
CODE=$(echo "$BODY" | jq -r '.code')
VIEWER_TOKEN=$(echo "$BODY" | jq -r '.viewer_token')

# --- セッション参加 ---
echo "[3] POST /api/sessions/join"
RESP=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/sessions/join" \
  -H 'Content-Type: application/json' \
  -d "{\"code\": \"$CODE\", \"dj_name\": \"DJ-Test\"}")
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | sed '$d')
assert_status "セッション参加" "200" "$STATUS"
show_body "$BODY"

PUBLISHER_TOKEN=$(echo "$BODY" | jq -r '.token')
PUBLISHER_ID=$(echo "$BODY" | jq -r '.publisher_id')

# --- 無効コードで参加 ---
echo "[4] POST /api/sessions/join (無効コード)"
RESP=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/sessions/join" \
  -H 'Content-Type: application/json' \
  -d '{"code": "000000", "dj_name": "DJ-Invalid"}')
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | sed '$d')
assert_status "無効コード → 404" "404" "$STATUS"
show_body "$BODY"

# --- Publish ---
echo "[5] POST /api/publish"
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/publish" \
  -H 'Content-Type: application/json' \
  -H "Authorization: Bearer $PUBLISHER_TOKEN" \
  -d '{
    "title": "Test Track",
    "artist": "Test Artist",
    "album": "Test Album",
    "comment": "House / 128BPM",
    "updated_at": "2026-07-20T15:30:00+09:00"
  }')
assert_status "楽曲送信" "204" "$STATUS"
dim "  (No Content)"
echo

# --- Publish (認証なし) ---
echo "[6] POST /api/publish (認証なし)"
RESP=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/publish" \
  -H 'Content-Type: application/json' \
  -d '{"title":"X","artist":"Y","updated_at":"2026-07-20T00:00:00+09:00"}')
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | sed '$d')
assert_status "認証なし → 401" "401" "$STATUS"
show_body "$BODY"

# --- SSE (途中接続で last_track 取得) ---
echo "[7] GET /api/sessions/{id}/stream (SSE 途中接続)"
SSE_RESP=$(curl -s -m 2 \
  -H "Authorization: Bearer $VIEWER_TOKEN" \
  "$BASE_URL/api/sessions/$SESSION_ID/stream" 2>/dev/null || true)

if echo "$SSE_RESP" | grep -q "track_changed"; then
  green "  ✓ SSE で track_changed を受信"
  PASS=$((PASS + 1))
else
  red "  ✗ SSE で track_changed が受信できなかった"
  FAIL=$((FAIL + 1))
fi
dim "  $SSE_RESP"
echo

# --- SSE (無効トークン) ---
echo "[8] GET /api/sessions/{id}/stream (無効トークン)"
RESP=$(curl -s -w "\n%{http_code}" -m 2 \
  -H "Authorization: Bearer invalid_token" \
  "$BASE_URL/api/sessions/$SESSION_ID/stream" 2>/dev/null || true)
STATUS=$(echo "$RESP" | tail -1)
BODY=$(echo "$RESP" | sed '$d')
assert_status "SSE 無効トークン → 401" "401" "$STATUS"
show_body "$BODY"

# --- 結果 ---
echo "=== 結果: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
