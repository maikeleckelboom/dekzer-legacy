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
pub struct ReadLibraryBoundaryEventsAfterRequest {
    /// The last event sequence the caller has already observed.
    /// `None` means start from the beginning.
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
pub struct ReadLibraryBoundaryEventsAfterReply {
    pub events: Vec<LibraryBoundaryEvent>,
    /// The cursor the consumer should pass as `lastSeenEventSequence` in
    /// the next read-after call.  It is the event sequence of the last
    /// event delivered in this reply, or the previously supplied
    /// `lastSeenEventSequence` when no new events were returned, or
    /// `None` when no events have been published yet.
    pub latest_event_sequence: Option<i64>,
    /// The earliest sequence still retained in the event ring buffer.
    /// If the caller asks for events after a sequence lower than this,
    /// some events have been compacted away.
    pub earliest_retained_sequence: Option<i64>,
    /// True when the caller's `lastSeenEventSequence` is older than the
    /// earliest retained event, meaning events were compacted away.
    pub gap_detected: bool,
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
    ReadAfter(ReadLibraryBoundaryEventsAfterReply),
}

#[cfg(test)]
mod tests {
    use super::{LibraryBoundaryEventStreamCommand, ReadLibraryBoundaryEventsAfterRequest};

    #[test]
    fn read_after_with_cursor_requests_next_events() {
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
        }
    }

    #[test]
    fn read_after_with_none_sequence_starts_from_beginning() {
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
        }
    }
}
