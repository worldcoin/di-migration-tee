#!/usr/bin/env bash
# Runs inside the LocalStack container once it is ready.
set -euo pipefail

awslocal s3api create-bucket --bucket di-migration-pcp

awslocal dynamodb create-table \
  --table-name di-migration \
  --attribute-definitions AttributeName=migration_id,AttributeType=S \
  --key-schema AttributeName=migration_id,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST

awslocal sqs create-queue --queue-name di-migration
