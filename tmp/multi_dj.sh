#!/bin/bash
set -euo pipefail

BASE_URL="http://localhost:8080"
CODE="${1:?セッションコードを第1引数で指定してください: bash tmp/multi_dj.sh <code>}"

# DJ-A join
JOIN_A=$(curl -s -X POST "$BASE_URL/api/sessions/join" -H "Content-Type: application/json" -d "{\"code\":\"$CODE\",\"dj_name\":\"DJ そのいち\"}")
echo "join A: $JOIN_A"
SID=$(echo "$JOIN_A" | jq -r '.session_id')
PT_A=$(echo "$JOIN_A" | jq -r '.token')

sleep 2

# DJ-B join
JOIN_B=$(curl -s -X POST "$BASE_URL/api/sessions/join" -H "Content-Type: application/json" -d "{\"code\":\"$CODE\",\"dj_name\":\"DJ そのに\"}")
echo "join B: $JOIN_B"
PT_B=$(echo "$JOIN_B" | jq -r '.token')

sleep 2
exit 0

# DJ-A publish
curl -s -o /dev/null -w "publish A status: %{http_code}\n" -X POST "$BASE_URL/api/sessions/$SID/publish" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PT_A" \
  -d "{\"title\":\"Track 1\",\"artist\":\"Artist A\",\"updated_at\":\"2026-07-22T00:00:00+09:00\"}"

sleep 2

# DJ-B publish
curl -s -o /dev/null -w "publish B status: %{http_code}\n" -X POST "$BASE_URL/api/sessions/$SID/publish" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PT_B" \
  -d "{\"title\":\"Track 2\",\"artist\":\"Artist B\",\"updated_at\":\"2026-07-22T00:01:00+09:00\"}"

sleep 2

# DJ-A publish
curl -s -o /dev/null -w "publish A status: %{http_code}\n" -X POST "$BASE_URL/api/sessions/$SID/publish" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PT_A" \
  -d "{\"title\":\"Track 3\",\"artist\":\"Artist C\",\"updated_at\":\"2026-07-22T00:00:00+09:00\"}"

sleep 2

# DJ-B publish
curl -s -o /dev/null -w "publish B status: %{http_code}\n" -X POST "$BASE_URL/api/sessions/$SID/publish" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $PT_B" \
  -d "{\"title\":\"Track 4\",\"artist\":\"Artist D\",\"updated_at\":\"2026-07-22T00:01:00+09:00\"}"
