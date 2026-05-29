use crate::events::LibraryBoundaryEvent;

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLibraryBoundaryEventsRequest {
    pub max_events: usize,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLibraryBoundaryEventsAfterRequest {
    /// The last event sequence the caller has already observed.
    /// `None` means start from the current tail.
    pub last_seen_event_sequence: Option<i64>,
    pub max_events: usize,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLibraryBoundaryEventsReply {
    pub events: Vec<LibraryBoundaryEvent>,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct ReadLibraryBoundaryEventsAfterReply {
    pub events: Vec<LibraryBoundaryEvent>,
    /// The highest event sequence included in this reply,
    /// so the caller can store it for the next read-after call.
    pub latest_event_sequence: Option<i64>,
}

#[derive(
    Debug,
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
pub enum LibraryBoundaryEventStreamCommand {
    ReadPending(ReadLibraryBoundaryEventsRequest),
    ReadAfter(ReadLibraryBoundaryEventsAfterRequest),
}

#[derive(
    Debug,
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
pub enum LibraryBoundaryEventStreamReply {
    ReadPending(ReadLibraryBoundaryEventsReply),
    ReadAfter(ReadLibraryBoundaryEventsAfterReply),
}

#[cfg(test)]
mod tests {
    use super::{
        LibraryBoundaryEventStreamCommand, ReadLibraryBoundaryEventsAfterRequest,
        ReadLibraryBoundaryEventsRequest,
    };

    #[test]
    fn library_boundary_event_stream_supports_both_read_pending_and_read_after() {
        let pending =
            LibraryBoundaryEventStreamCommand::ReadPending(ReadLibraryBoundaryEventsRequest {
                max_events: 16,
            });

        match pending {
            LibraryBoundaryEventStreamCommand::ReadPending(request) => {
                assert_eq!(request.max_events, 16);
            }
            _ => panic!("expected read pending"),
        }

        let after =
            LibraryBoundaryEventStreamCommand::ReadAfter(ReadLibraryBoundaryEventsAfterRequest {
                last_seen_event_sequence: Some(42),
                max_events: 16,
            });

        match after {
            LibraryBoundaryEventStreamCommand::ReadAfter(request) => {
                assert_eq!(request.last_seen_event_sequence, Some(42));
                assert_eq!(request.max_events, 16);
            }
            _ => panic!("expected read after"),
        }
    }

    #[test]
    fn read_after_with_none_sequence_starts_from_tail() {
        let after =
            LibraryBoundaryEventStreamCommand::ReadAfter(ReadLibraryBoundaryEventsAfterRequest {
                last_seen_event_sequence: None,
                max_events: 8,
            });

        match after {
            LibraryBoundaryEventStreamCommand::ReadAfter(request) => {
                assert_eq!(request.last_seen_event_sequence, None);
                assert_eq!(request.max_events, 8);
            }
            _ => panic!("expected read after"),
        }
    }
}
