use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Media type for the compressed PCP sent in, and the migrated PCP that comes back.
pub const MIGRATION_CONTENT_TYPE: &str = "application/octet-stream";

/// Maximum compressed request body.
///
/// Bounds what the host buffers before it knows whether the payload is even well-formed. The
/// decompressed ceiling is the enclave's, and lives with the vsock contract.
pub const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;

/// `POST /v1/migrations` response. The migration runs in the background; poll for the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationAccepted {
    /// Identifies the migration for `GET /v1/migrations/{id}`.
    pub id: Uuid,
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::MigrationAccepted;

    #[test]
    fn the_response_keeps_its_wire_name() {
        let id = Uuid::nil();
        let body = MigrationAccepted { id };
        let json = serde_json::json!({ "id": id });

        assert_eq!(serde_json::to_value(&body).expect("should serialize"), json);
        assert_eq!(
            serde_json::from_value::<MigrationAccepted>(json).expect("should deserialize"),
            body
        );
    }
}
