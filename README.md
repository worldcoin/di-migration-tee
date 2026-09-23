## Deep Identifier Migration 
This repo contains the implementation of the TEE-based migration from iris code-based PCPs to DeepIdentifier-based ones

## Migration API

Run `cargo run -p migration-api`. `HTTP_ADDR` defaults to `0.0.0.0:8080`.
Set `DYNAMODB_TABLE_NAME` and configure an AWS region and credentials using the AWS SDK's standard environment or profile settings. The credentials need `dynamodb:DescribeTable` permission on the table.
Set `SQS_QUEUE_URL` to the queue URL. The credentials also need `sqs:GetQueueAttributes` permission on the queue.

`GET /healtz` checks the API process. `GET /readyz` describes the configured DynamoDB table and reads the SQS queue attributes, returning 503 if either request fails or times out.
