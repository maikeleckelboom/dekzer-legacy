use thiserror::Error;

pub type ProtocolResult<T> = Result<T, ProtocolError>;

#[derive(
    Debug,
    Error,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum ProtocolError {
    #[error("{detail}")]
    InvalidRequest { detail: String },
    #[error("{detail}")]
    DurableStoreFailure { detail: String },
    #[error("{detail}")]
    HostFailure { detail: String },
}

impl ProtocolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest { .. } => "INVALID_REQUEST",
            Self::DurableStoreFailure { .. } => "DURABLE_STORE_FAILURE",
            Self::HostFailure { .. } => "HOST_FAILURE",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProtocolError;

    #[test]
    fn maintained_errors_preserve_stable_codes() {
        let invalid = ProtocolError::InvalidRequest {
            detail: "missing query".to_string(),
        };
        let store = ProtocolError::DurableStoreFailure {
            detail: "sqlite busy".to_string(),
        };
        let host = ProtocolError::HostFailure {
            detail: "thread pool unavailable".to_string(),
        };

        assert_eq!(invalid.code(), "INVALID_REQUEST");
        assert_eq!(store.code(), "DURABLE_STORE_FAILURE");
        assert_eq!(host.code(), "HOST_FAILURE");
    }
}
