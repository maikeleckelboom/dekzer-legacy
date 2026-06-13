use std::io::{self, BufRead, Write};
use std::panic::{self, AssertUnwindSafe};

use library_boundary_service::LibraryBoundaryService;

use crate::envelope::{
    StdioResponseEnvelope, StdioTransportErrorCode, command_outcome_response,
    deserialize_request_frame, ready_response, serialize_response_frame, transport_error_response,
};

#[derive(Debug)]
pub(crate) enum ServerLoopError {
    StdinReadFailure(io::Error),
    StdoutWriteFailure(io::Error),
    ResponseSerialization(serde_json::Error),
}

impl ServerLoopError {
    pub(crate) fn render_lines(&self) -> Vec<String> {
        vec![
            format!("[library-boundary-stdio] detail: {self}"),
            "[library-boundary-stdio] error: stdio server loop failed".to_string(),
        ]
    }
}

impl std::fmt::Display for ServerLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StdinReadFailure(error) => {
                write!(f, "stdin read failure: {error}")
            }
            Self::StdoutWriteFailure(error) => {
                write!(f, "stdout write failure: {error}")
            }
            Self::ResponseSerialization(error) => {
                write!(f, "stdio response serialization failure: {error}")
            }
        }
    }
}

pub(crate) fn run_server_loop<R, W>(
    service: &LibraryBoundaryService,
    mut reader: R,
    mut writer: W,
) -> Result<(), ServerLoopError>
where
    R: BufRead,
    W: Write,
{
    write_response_frame(&mut writer, &ready_response())?;

    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(error) => {
                let response = transport_error_response(
                    None,
                    StdioTransportErrorCode::StdinReadFailure,
                    format!("failed to read stdin frame: {error}"),
                );
                write_response_frame(&mut writer, &response)?;
                return Err(ServerLoopError::StdinReadFailure(error));
            }
        }

        let frame = line.trim_end_matches(&['\r', '\n'][..]);
        if frame.trim().is_empty() {
            continue;
        }

        let response = handle_frame(service, frame);
        write_response_frame(&mut writer, &response)?;
    }
}

pub(crate) fn handle_frame(service: &LibraryBoundaryService, frame: &str) -> StdioResponseEnvelope {
    match deserialize_request_frame(frame) {
        Ok((request_id, request)) => {
            match panic::catch_unwind(AssertUnwindSafe(|| service.handle_command(request))) {
                Ok(outcome) => command_outcome_response(request_id, outcome),
                Err(_) => transport_error_response(
                    Some(request_id),
                    StdioTransportErrorCode::ServerPanic,
                    "library boundary service panicked while handling command",
                ),
            }
        }
        Err(error_response) => *error_response,
    }
}

pub(crate) fn write_response_frame<W>(
    writer: &mut W,
    response: &StdioResponseEnvelope,
) -> Result<(), ServerLoopError>
where
    W: Write,
{
    let line =
        serialize_response_frame(response).map_err(ServerLoopError::ResponseSerialization)?;
    writer
        .write_all(line.as_bytes())
        .map_err(ServerLoopError::StdoutWriteFailure)?;
    writer
        .write_all(b"\n")
        .map_err(ServerLoopError::StdoutWriteFailure)?;
    writer.flush().map_err(ServerLoopError::StdoutWriteFailure)
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Cursor};

    use library_boundary_protocol::{
        CommandReply, CommandRequest, LibraryRootCommand, LibraryRootReply,
        RegisterLocalRootRequest,
    };
    use library_boundary_service::{LibraryBoundaryService, LibraryStoreContext, StoreEnvironment};
    use serde_json::json;
    use tempfile::TempDir;

    use super::run_server_loop;

    fn open_service() -> (TempDir, LibraryBoundaryService) {
        let tempdir = TempDir::new().expect("create tempdir");
        let service = LibraryBoundaryService::open(LibraryStoreContext {
            user_data_path: tempdir.path().to_string_lossy().into_owned(),
            environment: StoreEnvironment::Development,
        })
        .expect("open service");

        (tempdir, service)
    }

    #[test]
    fn valid_register_root_command_reaches_real_boundary_service() {
        let (tempdir, service) = open_service();
        let source_root = tempdir.path().join("source-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let request = CommandRequest::LibraryRoots(LibraryRootCommand::RegisterLocalRoot(
            RegisterLocalRootRequest {
                requested_path: source_root.to_string_lossy().into_owned(),
            },
        ));
        let input = format!(
            "{}\n",
            serde_json::to_string(&json!({
                "type": "command",
                "requestId": "request-1",
                "request": request
            }))
            .expect("serialize request")
        );
        let mut output = Vec::new();

        run_server_loop(&service, BufReader::new(Cursor::new(input)), &mut output)
            .expect("run server loop");

        let line = String::from_utf8(output).expect("utf8 output");
        let lines = line.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let ready = serde_json::from_str::<serde_json::Value>(lines[0]).expect("parse ready");
        assert_eq!(
            ready,
            json!({
                "type": "ready",
                "server": "libraryBoundaryStdio"
            })
        );
        let response = serde_json::from_str::<serde_json::Value>(lines[1]).expect("parse response");
        assert_eq!(response.pointer("/type"), Some(&json!("commandOutcome")));
        assert_eq!(response.pointer("/requestId"), Some(&json!("request-1")));
        assert_eq!(
            response.pointer("/outcome/type"),
            Some(&json!("success")),
            "real service returns success outcome"
        );
        assert!(
            response
                .pointer("/outcome/payload/reply/payload/payload/payload/rootId")
                .and_then(serde_json::Value::as_str)
                .is_some()
        );

        let outcome = serde_json::from_value::<library_boundary_protocol::CommandOutcome>(
            response
                .get("outcome")
                .expect("outcome field exists")
                .clone(),
        )
        .expect("deserialize outcome");
        let library_boundary_protocol::CommandOutcome::Success(envelope) = outcome else {
            panic!("expected success outcome");
        };
        assert!(matches!(
            envelope.reply,
            CommandReply::LibraryRoots(LibraryRootReply::RegisterLocalRoot(_))
        ));
    }

    #[test]
    fn blank_lines_are_ignored() {
        let (_tempdir, service) = open_service();
        let input = "\n\r\n   \n".to_string();
        let mut output = Vec::new();

        run_server_loop(&service, BufReader::new(Cursor::new(input)), &mut output)
            .expect("run server loop");

        let text = String::from_utf8(output).expect("utf8 output");
        assert_eq!(
            text,
            "{\"type\":\"ready\",\"server\":\"libraryBoundaryStdio\"}\n"
        );
    }

    #[test]
    fn compact_stdout_response_is_single_json_object_per_line() {
        let (tempdir, service) = open_service();
        let source_root = tempdir.path().join("source-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let request = CommandRequest::LibraryRoots(LibraryRootCommand::RegisterLocalRoot(
            RegisterLocalRootRequest {
                requested_path: source_root.to_string_lossy().into_owned(),
            },
        ));
        let input = format!(
            "{}\n",
            serde_json::to_string(&json!({
                "type": "command",
                "requestId": "request-1",
                "request": request
            }))
            .expect("serialize request")
        );
        let mut output = Vec::new();

        run_server_loop(&service, BufReader::new(Cursor::new(input)), &mut output)
            .expect("run server loop");

        let text = String::from_utf8(output).expect("utf8 output");
        assert_eq!(text.lines().count(), 2);
        assert!(text.ends_with('\n'));
        for line in text.lines() {
            assert!(serde_json::from_str::<serde_json::Value>(line).is_ok());
        }
    }

    #[test]
    fn server_loop_writes_ready_before_command_response() {
        let (tempdir, service) = open_service();
        let source_root = tempdir.path().join("source-root");
        std::fs::create_dir_all(&source_root).expect("create source root");
        let request = CommandRequest::LibraryRoots(LibraryRootCommand::RegisterLocalRoot(
            RegisterLocalRootRequest {
                requested_path: source_root.to_string_lossy().into_owned(),
            },
        ));
        let input = format!(
            "{}\n",
            serde_json::to_string(&json!({
                "type": "command",
                "requestId": "request-1",
                "request": request
            }))
            .expect("serialize request")
        );
        let mut output = Vec::new();

        run_server_loop(&service, BufReader::new(Cursor::new(input)), &mut output)
            .expect("run server loop");

        let text = String::from_utf8(output).expect("utf8 output");
        let lines = text.lines().collect::<Vec<_>>();
        assert_eq!(
            lines.first(),
            Some(&r#"{"type":"ready","server":"libraryBoundaryStdio"}"#)
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(lines[1]).expect("parse response")["type"],
            json!("commandOutcome")
        );
    }
}
