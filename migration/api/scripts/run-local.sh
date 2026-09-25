#!/usr/bin/env bash
# Runs the migration API against the LocalStack stack from docker-compose.yml.
set -euo pipefail

cd "$(dirname "$0")/.."

export AWS_REGION=us-east-1
export AWS_ACCESS_KEY_ID=test
export AWS_SECRET_ACCESS_KEY=test
export AWS_ENDPOINT_URL=http://localhost:4566

export HTTP_ADDR=127.0.0.1:8080
export DYNAMODB_TABLE_NAME=di-migration
export SQS_QUEUE_URL=http://localhost:4566/000000000000/di-migration
export PCP_BUCKET=di-migration-pcp
export S3_FORCE_PATH_STYLE=true
export ENCLAVE_ID=local-stub-enclave
# LocalStack has no Nitro enclave to attest; never set this outside local runs.
export STUB_ATTESTATION=true

exec cargo run -p migration-api "$@"
