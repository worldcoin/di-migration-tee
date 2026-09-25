#!/usr/bin/env bash
# Calls POST /v1/init-migration and uploads a test payload to the presigned URL it returns.
set -euo pipefail

API_URL=${API_URL:-http://127.0.0.1:8080}
SUB=${SUB:-local-test-sub}

response=$(curl -fsS -X POST "${API_URL}/v1/init-migration" \
  -H 'Content-Type: application/json' \
  -d "$(jq -nc --arg sub "${SUB}" '{sub: $sub}')")
echo "${response}" | (jq . 2>/dev/null || cat)

presigned_url=$(echo "${response}" | jq -r .presigned_url)
upload_status=$(curl -sS -o /dev/null -w '%{http_code}' -X PUT --data-binary 'test-pcp' "${presigned_url}")
echo "presigned PUT -> ${upload_status}"
[[ ${upload_status} == 200 ]]
