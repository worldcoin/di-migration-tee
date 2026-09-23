## Deep Identifier Migration 
This repo contains the implementation of the TEE-based migration from iris code-based PCPs to DeepIdentifier-based ones

## Migration API

Run `cargo run -p migration-api`. `HTTP_ADDR` defaults to `0.0.0.0:8080`.
Set `DYNAMODB_TABLE_NAME` and configure an AWS region and credentials using the AWS SDK's standard environment or profile settings. The credentials need `dynamodb:DescribeTable` permission on the table.

`GET /healtz` checks the API process. `GET /readyz` describes the configured DynamoDB table and returns 503 if the request fails or times out.
