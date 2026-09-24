use std::time::Duration;

use anyhow::Context;
use aws_sdk_dynamodb::Client;

const READINESS_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone)]
pub struct Db {
    client: Client,
    table_name: String,
}

impl Db {
    pub fn new(client: Client, table_name: String) -> Self {
        Self { client, table_name }
    }

    pub async fn check_ready(&self) -> anyhow::Result<()> {
        tokio::time::timeout(
            READINESS_TIMEOUT,
            self.client
                .describe_table()
                .table_name(&self.table_name)
                .send(),
        )
        .await
        .context("DynamoDB DescribeTable timed out")?
        .context("DynamoDB DescribeTable failed")?;

        Ok(())
    }
}
