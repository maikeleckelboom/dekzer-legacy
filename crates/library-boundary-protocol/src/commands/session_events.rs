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
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
#[ts(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum LibraryBoundaryEventStreamCommand {
    ReadPending(ReadLibraryBoundaryEventsRequest),
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
}

#[cfg(test)]
mod tests {
    use super::{LibraryBoundaryEventStreamCommand, ReadLibraryBoundaryEventsRequest};

    #[test]
    fn library_boundary_event_stream_is_a_single_pull_boundary() {
        let command =
            LibraryBoundaryEventStreamCommand::ReadPending(ReadLibraryBoundaryEventsRequest {
                max_events: 16,
            });

        match command {
            LibraryBoundaryEventStreamCommand::ReadPending(request) => {
                assert_eq!(request.max_events, 16);
            }
        }
    }
}
