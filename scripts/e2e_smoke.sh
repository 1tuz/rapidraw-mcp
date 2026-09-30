#!/usr/bin/env bash
# Smoke-test rapidraw-mcp against a live RapidRAW External Control API.
# Skips gracefully when RapidRAW is not running.
set -euo pipefail

API_URL="${RAPIDRAW_API_URL:-http://127.0.0.1:17355}"
STATUS_URL="${API_URL%/}/v1/status"

echo "e2e_smoke: probing ${STATUS_URL}"

HTTP_CODE="$(
  if [[ -n "${RAPIDRAW_API_TOKEN:-}" ]]; then
    curl -sS -o /tmp/rapidraw-e2e-status.json -w '%{http_code}' \
      --connect-timeout 2 --max-time 5 \
      -H "Authorization: Bearer ${RAPIDRAW_API_TOKEN}" \
      "${STATUS_URL}" || true
  else
    curl -sS -o /tmp/rapidraw-e2e-status.json -w '%{http_code}' \
      --connect-timeout 2 --max-time 5 \
      "${STATUS_URL}" || true
  fi
)"

if [[ "${HTTP_CODE}" != "200" ]]; then
  echo "SKIP: RapidRAW not reachable (HTTP ${HTTP_CODE:-none}). Enable External Control in Settings."
  exit 0
fi

if ! python3 -c 'import json,sys; d=json.load(open("/tmp/rapidraw-e2e-status.json")); sys.exit(0 if d.get("ok") else 1)'; then
  echo "SKIP: status response not ok=true"
  cat /tmp/rapidraw-e2e-status.json
  exit 0
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${ROOT}/target/release/rapidraw-mcp"
if [[ ! -x "${BIN}" ]]; then
  echo "building release binary…"
  (cd "${ROOT}" && cargo build --release)
fi

# Minimal MCP initialize handshake over stdio
INIT=$(cat <<'EOF'
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"e2e-smoke","version":"0.0.1"}}}
EOF
)

LIST=$(cat <<'EOF'
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
EOF
)

OUT="$(printf '%s\n%s\n' "${INIT}" "${LIST}" | RAPIDRAW_API_URL="${API_URL}" RAPIDRAW_API_TOKEN="${RAPIDRAW_API_TOKEN:-}" "${BIN}" 2>/tmp/rapidraw-mcp-e2e.err || true)"

if ! echo "${OUT}" | grep -q 'rapidraw_get_status'; then
  echo "FAIL: tools/list did not include rapidraw_get_status"
  echo "--- stdout ---"; echo "${OUT}"
  echo "--- stderr ---"; cat /tmp/rapidraw-mcp-e2e.err
  exit 1
fi

echo "OK: RapidRAW up; MCP tools/list contains rapidraw_get_status"
exit 0
