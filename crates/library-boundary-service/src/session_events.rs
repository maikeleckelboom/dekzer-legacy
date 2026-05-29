use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use library_boundary_protocol::{
    LibraryBoundaryEvent, MaintainedSnapshotEvent, MaintainedSnapshotInvalidation,
    MaintainedSnapshotRevision, MaintainedSnapshotScope, MaintainedSnapshotScopeRevision,
    ScanRunPhase, SourceScanEvent, SourceScanEventKind,
};

const MAX_STORED_EVENTS: usize = 256;

#[derive(Debug, Default)]
pub(crate) struct LibraryBoundaryEventStream {
    state: Mutex<LibraryBoundaryEventStreamState>,
}

impl LibraryBoundaryEventStream {
    pub(crate) fn new<I>(initial_revisions: I) -> Self
    where
        I: IntoIterator<Item = MaintainedSnapshotScopeRevision>,
    {
        let mut observed_revisions = BTreeMap::new();
        for revision in initial_revisions {
            observed_revisions.insert(revision.scope, revision.revision);
        }

        Self {
            state: Mutex::new(LibraryBoundaryEventStreamState {
                observed_revisions,
                ..LibraryBoundaryEventStreamState::default()
            }),
        }
    }

    pub(crate) fn publish_revisions<I>(&self, revisions: I)
    where
        I: IntoIterator<Item = MaintainedSnapshotScopeRevision>,
    {
        let mut state = self.state.lock().expect("boundary event stream poisoned");
        for revision in revisions {
            state.publish_revision(revision);
        }
    }

    pub(crate) fn publish_scan_event(
        &self,
        kind: SourceScanEventKind,
        root_id: i64,
        scan_run_id: i64,
        phase: ScanRunPhase,
        directories_visited: usize,
        files_visited: usize,
        files_discovered: usize,
        media_candidates: usize,
        queued_work_items: usize,
        detail: Option<String>,
    ) {
        let mut state = self.state.lock().expect("boundary event stream poisoned");
        state.publish_scan_event(
            kind,
            root_id,
            scan_run_id,
            phase,
            directories_visited,
            files_visited,
            files_discovered,
            media_candidates,
            queued_work_items,
            detail,
        );
    }

    pub(crate) fn read_after(
        &self,
        last_seen_event_sequence: Option<i64>,
        max_events: usize,
    ) -> (Vec<LibraryBoundaryEvent>, Option<i64>, Option<i64>, bool) {
        let state = self.state.lock().expect("boundary event stream poisoned");
        state.read_after(last_seen_event_sequence, max_events)
    }
}

#[derive(Debug, Default)]
struct LibraryBoundaryEventStreamState {
    next_sequence: i64,
    latest_published_sequence: Option<i64>,
    stored_events: VecDeque<LibraryBoundaryEvent>,
    observed_revisions: BTreeMap<MaintainedSnapshotScope, MaintainedSnapshotRevision>,
}

impl LibraryBoundaryEventStreamState {
    fn next_sequence(&mut self) -> i64 {
        let seq = self.next_sequence;
        self.next_sequence += 1;
        seq
    }

    fn push_event(&mut self, event: LibraryBoundaryEvent) {
        self.stored_events.push_back(event);
        if self.stored_events.len() > MAX_STORED_EVENTS {
            self.stored_events.pop_front();
        }
    }

    fn current_time_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }

    fn publish_scan_event(
        &mut self,
        kind: SourceScanEventKind,
        root_id: i64,
        scan_run_id: i64,
        phase: ScanRunPhase,
        directories_visited: usize,
        files_visited: usize,
        files_discovered: usize,
        media_candidates: usize,
        queued_work_items: usize,
        detail: Option<String>,
    ) {
        let sequence = self.next_sequence();
        let event = LibraryBoundaryEvent::SourceScanEvent(SourceScanEvent {
            event_sequence: sequence,
            occurred_at_ms: Self::current_time_ms(),
            kind,
            root_id,
            scan_run_id,
            phase,
            directories_visited,
            files_visited,
            files_discovered,
            media_candidates,
            queued_work_items,
            detail,
        });
        self.latest_published_sequence = Some(sequence);
        self.push_event(event);
    }

    fn publish_revision(&mut self, revision: MaintainedSnapshotScopeRevision) {
        let observed = self
            .observed_revisions
            .entry(revision.scope)
            .or_insert(MaintainedSnapshotRevision::new(0));
        if *observed >= revision.revision {
            return;
        }

        *observed = revision.revision;
        self.publish_invalidation(MaintainedSnapshotInvalidation {
            scope: revision.scope,
            revision: Some(revision.revision),
        });
    }

    fn publish_invalidation(&mut self, invalidation: MaintainedSnapshotInvalidation) {
        let sequence = self.next_sequence();
        let event = LibraryBoundaryEvent::MaintainedSnapshotInvalidated(
            MaintainedSnapshotEvent {
                event_sequence: sequence,
                occurred_at_ms: Self::current_time_ms(),
                invalidation,
            },
        );
        self.latest_published_sequence = Some(sequence);
        self.push_event(event);
    }

    fn read_after(
        &self,
        last_seen_event_sequence: Option<i64>,
        max_events: usize,
    ) -> (Vec<LibraryBoundaryEvent>, Option<i64>, Option<i64>, bool) {
        let start_from = match last_seen_event_sequence {
            Some(seq) => seq + 1,
            None => 0,
        };

        let earliest_retained = self
            .stored_events
            .front()
            .map(|e| Self::event_sequence(e));

        let gap_detected = match (last_seen_event_sequence, earliest_retained) {
            (Some(last_seen), Some(earliest)) => last_seen + 1 < earliest,
            _ => false,
        };

        let events: Vec<LibraryBoundaryEvent> = self
            .stored_events
            .iter()
            .filter(|e| Self::event_sequence(e) >= start_from)
            .take(max_events)
            .cloned()
            .collect();

        let latest = events
            .last()
            .map(|e| Self::event_sequence(e))
            .or(last_seen_event_sequence)
            .or(self.latest_published_sequence);

        (events, latest, earliest_retained, gap_detected)
    }

    fn event_sequence(event: &LibraryBoundaryEvent) -> i64 {
        match event {
            LibraryBoundaryEvent::SourceScanEvent(e) => e.event_sequence,
            LibraryBoundaryEvent::MaintainedSnapshotInvalidated(e) => e.event_sequence,
        }
    }
}
