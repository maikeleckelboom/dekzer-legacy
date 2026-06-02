use library_boundary_protocol::{CommandOutcome, CommandRequest};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[allow(clippy::large_enum_variant)]
#[serde(tag = "type", rename_all = "camelCase")]
pub(crate) enum StdioResponseEnvelope {
    Ready {
        server: StdioReadyServer,
    },
    CommandOutcome {
        #[serde(rename = "requestId")]
        request_id: String,
        outcome: CommandOutcome,
    },
    TransportError {
        #[serde(rename = "requestId")]
        request_id: Option<String>,
        error: StdioTransportErrorBody,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdioReadyServer {
    LibraryBoundaryStdio,
}

pub(crate) const READY_ENVELOPE_TYPE: &str = "ready";
pub(crate) const READY_SERVER: &str = "libraryBoundaryStdio";

impl StdioReadyServer {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LibraryBoundaryStdio => READY_SERVER,
        }
    }
}

impl serde::Serialize for StdioReadyServer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for StdioReadyServer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        match value.as_str() {
            READY_SERVER => Ok(Self::LibraryBoundaryStdio),
            _ => Err(serde::de::Error::custom(format!(
                "unknown stdio ready server {value:?}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StdioTransportErrorBody {
    pub(crate) code: StdioTransportErrorCode,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StdioTransportErrorCode {
    InvalidFrame,
    InvalidEnvelope,
    InvalidRequestId,
    InvalidCommandRequest,
    ServerPanic,
    StdinReadFailure,
}

pub(crate) const REMOTE_TRANSPORT_ERROR_CODES: [StdioTransportErrorCode; 6] = [
    StdioTransportErrorCode::InvalidFrame,
    StdioTransportErrorCode::InvalidEnvelope,
    StdioTransportErrorCode::InvalidRequestId,
    StdioTransportErrorCode::InvalidCommandRequest,
    StdioTransportErrorCode::ServerPanic,
    StdioTransportErrorCode::StdinReadFailure,
];

pub(crate) fn remote_transport_error_codes() -> &'static [StdioTransportErrorCode] {
    &REMOTE_TRANSPORT_ERROR_CODES
}

pub(crate) fn ready_envelope_type() -> &'static str {
    READY_ENVELOPE_TYPE
}

pub(crate) fn ready_server_string() -> &'static str {
    READY_SERVER
}

impl StdioTransportErrorCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidFrame => "invalidFrame",
            Self::InvalidEnvelope => "invalidEnvelope",
            Self::InvalidRequestId => "invalidRequestId",
            Self::InvalidCommandRequest => "invalidCommandRequest",
            Self::ServerPanic => "serverPanic",
            Self::StdinReadFailure => "stdinReadFailure",
        }
    }
}

impl serde::Serialize for StdioTransportErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for StdioTransportErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        match value.as_str() {
            "invalidFrame" => Ok(Self::InvalidFrame),
            "invalidEnvelope" => Ok(Self::InvalidEnvelope),
            "invalidRequestId" => Ok(Self::InvalidRequestId),
            "invalidCommandRequest" => Ok(Self::InvalidCommandRequest),
            "serverPanic" => Ok(Self::ServerPanic),
            "stdinReadFailure" => Ok(Self::StdinReadFailure),
            _ => Err(serde::de::Error::custom(format!(
                "unknown stdio transport error code {value:?}"
            ))),
        }
    }
}

pub(crate) fn ready_response() -> StdioResponseEnvelope {
    StdioResponseEnvelope::Ready {
        server: StdioReadyServer::LibraryBoundaryStdio,
    }
}

pub(crate) fn command_outcome_response(
    request_id: String,
    outcome: CommandOutcome,
) -> StdioResponseEnvelope {
    StdioResponseEnvelope::CommandOutcome {
        request_id,
        outcome,
    }
}

pub(crate) fn transport_error_response(
    request_id: Option<String>,
    code: StdioTransportErrorCode,
    message: impl Into<String>,
) -> StdioResponseEnvelope {
    StdioResponseEnvelope::TransportError {
        request_id,
        error: StdioTransportErrorBody {
            code,
            message: message.into(),
        },
    }
}

pub(crate) fn deserialize_request_frame(
    line: &str,
) -> Result<(String, CommandRequest), Box<StdioResponseEnvelope>> {
    let value = serde_json::from_str::<serde_json::Value>(line).map_err(|error| {
        Box::new(transport_error_response(
            None,
            StdioTransportErrorCode::InvalidFrame,
            format!("invalid JSON frame: {error}"),
        ))
    })?;

    let request_id = request_id_from_value(&value);
    let Some(object) = value.as_object() else {
        return Err(Box::new(transport_error_response(
            request_id,
            StdioTransportErrorCode::InvalidEnvelope,
            "stdio frame must be a JSON object",
        )));
    };

    if !matches!(object.get("type"), Some(serde_json::Value::String(kind)) if kind == "command") {
        return Err(Box::new(transport_error_response(
            request_id,
            StdioTransportErrorCode::InvalidEnvelope,
            "stdio frame type must be command",
        )));
    }

    let Some(request_id) = request_id else {
        return Err(Box::new(transport_error_response(
            None,
            StdioTransportErrorCode::InvalidRequestId,
            "stdio command envelope requestId must be a non-empty string",
        )));
    };

    let Some(request_value) = object.get("request") else {
        return Err(Box::new(transport_error_response(
            Some(request_id),
            StdioTransportErrorCode::InvalidCommandRequest,
            "stdio command envelope request field is required",
        )));
    };

    let request =
        serde_json::from_value::<CommandRequest>(request_value.clone()).map_err(|error| {
            Box::new(transport_error_response(
                Some(request_id.clone()),
                StdioTransportErrorCode::InvalidCommandRequest,
                format!("invalid boundary command request: {error}"),
            ))
        })?;

    Ok((request_id, request))
}

pub(crate) fn serialize_response_frame(
    response: &StdioResponseEnvelope,
) -> Result<String, serde_json::Error> {
    serde_json::to_string(response)
}

fn request_id_from_value(value: &serde_json::Value) -> Option<String> {
    value
        .get("requestId")
        .and_then(serde_json::Value::as_str)
        .filter(|request_id| !request_id.trim().is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use library_boundary_protocol::{
        CommandOutcome, CommandReply, CommandRequest, CommandSuccessEnvelope, LibraryRootCommand,
        LibraryRootReply, RegisterLocalRootReply, RegisterLocalRootRequest,
    };
    use serde_json::json;

    use super::{
        StdioResponseEnvelope, StdioTransportErrorCode, command_outcome_response,
        deserialize_request_frame, ready_response, serialize_response_frame,
        transport_error_response,
    };

    fn register_root_request() -> CommandRequest {
        CommandRequest::LibraryRoots(LibraryRootCommand::RegisterLocalRoot(
            RegisterLocalRootRequest {
                absolute_path: "C:/Music".to_string(),
            },
        ))
    }

    #[test]
    fn ready_response_serializes_as_stable_compact_line() {
        let line = serialize_response_frame(&ready_response()).expect("serialize response");

        assert!(!line.contains('\n'));
        assert_eq!(line, r#"{"type":"ready","server":"libraryBoundaryStdio"}"#);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&line).expect("parse response"),
            json!({
                "type": "ready",
                "server": "libraryBoundaryStdio"
            })
        );
    }

    #[test]
    fn valid_command_envelope_deserializes() {
        let line = serde_json::to_string(&json!({
            "type": "command",
            "requestId": "request-1",
            "request": {
                "type": "libraryRoots",
                "payload": {
                    "type": "registerLocalRoot",
                    "payload": {
                        "absolutePath": "C:/Music"
                    }
                }
            }
        }))
        .expect("serialize test line");

        let (request_id, request) = deserialize_request_frame(&line).expect("deserialize request");

        assert_eq!(request_id, "request-1");
        assert_eq!(request, register_root_request());
    }

    #[test]
    fn command_outcome_response_serializes_same_request_id_as_single_line() {
        let response = command_outcome_response(
            "request-1".to_string(),
            CommandOutcome::Success(CommandSuccessEnvelope {
                reply: CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(
                    RegisterLocalRootReply {
                        root_id: 7,
                        canonical_path: "C:/Music".to_string(),
                    },
                )),
            }),
        );

        let line = serialize_response_frame(&response).expect("serialize response");

        assert!(!line.contains('\n'));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&line).expect("parse response"),
            json!({
                "type": "commandOutcome",
                "requestId": "request-1",
                "outcome": {
                    "type": "success",
                    "payload": {
                        "reply": {
                            "type": "libraryRoots",
                            "payload": {
                                "type": "registerLocalRoot",
                                "payload": {
                                    "rootId": "7",
                                    "canonicalPath": "C:/Music"
                                }
                            }
                        }
                    }
                }
            })
        );
    }

    #[test]
    fn transport_error_response_serializes_null_request_id() {
        let response =
            transport_error_response(None, StdioTransportErrorCode::InvalidFrame, "bad frame");

        let line = serialize_response_frame(&response).expect("serialize response");

        assert!(!line.contains('\n'));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&line).expect("parse response"),
            json!({
                "type": "transportError",
                "requestId": null,
                "error": {
                    "code": "invalidFrame",
                    "message": "bad frame"
                }
            })
        );
    }

    #[test]
    fn transport_error_response_serializes_request_id() {
        let response = transport_error_response(
            Some("request-1".to_string()),
            StdioTransportErrorCode::InvalidEnvelope,
            "bad envelope",
        );

        let line = serialize_response_frame(&response).expect("serialize response");

        assert!(!line.contains('\n'));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&line).expect("parse response"),
            json!({
                "type": "transportError",
                "requestId": "request-1",
                "error": {
                    "code": "invalidEnvelope",
                    "message": "bad envelope"
                }
            })
        );
    }

    #[test]
    fn every_remote_transport_error_code_serializes_to_stable_string() {
        let expected = [
            (StdioTransportErrorCode::InvalidFrame, "invalidFrame"),
            (StdioTransportErrorCode::InvalidEnvelope, "invalidEnvelope"),
            (
                StdioTransportErrorCode::InvalidRequestId,
                "invalidRequestId",
            ),
            (
                StdioTransportErrorCode::InvalidCommandRequest,
                "invalidCommandRequest",
            ),
            (StdioTransportErrorCode::ServerPanic, "serverPanic"),
            (
                StdioTransportErrorCode::StdinReadFailure,
                "stdinReadFailure",
            ),
        ];

        assert_eq!(
            super::remote_transport_error_codes(),
            expected.map(|(code, _)| code)
        );
        for (code, stable_string) in expected {
            assert_eq!(code.as_str(), stable_string);
            assert_eq!(
                serde_json::to_value(code).expect("serialize code"),
                json!(stable_string)
            );
            assert_eq!(
                serde_json::from_value::<StdioTransportErrorCode>(json!(stable_string))
                    .expect("deserialize code"),
                code
            );
        }
    }

    #[test]
    fn invalid_json_yields_transport_error() {
        let response = *deserialize_request_frame("{not-json").expect_err("invalid JSON rejected");

        assert!(matches!(
            response,
            StdioResponseEnvelope::TransportError {
                request_id: None,
                error
            } if error.code == StdioTransportErrorCode::InvalidFrame
        ));
    }

    #[test]
    fn missing_request_id_yields_transport_error() {
        let line = serde_json::to_string(&json!({
            "type": "command",
            "request": {
                "type": "libraryRoots",
                "payload": {
                    "type": "registerLocalRoot",
                    "payload": {
                        "absolutePath": "C:/Music"
                    }
                }
            }
        }))
        .expect("serialize test line");

        let response = *deserialize_request_frame(&line).expect_err("missing requestId rejected");

        assert!(matches!(
            response,
            StdioResponseEnvelope::TransportError {
                request_id: None,
                error
            } if error.code == StdioTransportErrorCode::InvalidRequestId
        ));
    }

    #[test]
    fn invalid_request_id_yields_transport_error() {
        let line = serde_json::to_string(&json!({
            "type": "command",
            "requestId": "",
            "request": {
                "type": "libraryRoots",
                "payload": {
                    "type": "registerLocalRoot",
                    "payload": {
                        "absolutePath": "C:/Music"
                    }
                }
            }
        }))
        .expect("serialize test line");

        let response = *deserialize_request_frame(&line).expect_err("invalid requestId rejected");

        assert!(matches!(
            response,
            StdioResponseEnvelope::TransportError {
                request_id: None,
                error
            } if error.code == StdioTransportErrorCode::InvalidRequestId
        ));
    }
}
