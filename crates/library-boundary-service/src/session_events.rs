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

    pub(crate) fn drain(&self, max_events: usize) -> Vec<LibraryBoundaryEvent> {
        let mut state = self.state.lock().expect("boundary event stream poisoned");
        state.drain(max_events)
    }

    pub(crate) fn read_after(
        &self,
        last_seen_event_sequence: Option<i64>,
        max_events: usize,
    ) -> (Vec<LibraryBoundaryEvent>, Option<i64>) {
        let state = self.state.lock().expect("boundary event stream poisoned");
        state.read_after(last_seen_event_sequence, max_events)
    }
}

#[derive(Debug, Default)]
struct LibraryBoundaryEventStreamState {
    next_sequence: i64,
    stored_events: VecDeque<LibraryBoundaryEvent>,
    observed_revisions: BTreeMap<MaintainedSnapshotScope, MaintainedSnapshotRevision>,
    pending_order: VecDeque<MaintainedSnapshotScope>,
    pending_invalidations: BTreeMap<MaintainedSnapshotScope, MaintainedSnapshotInvalidation>,
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
        let event = LibraryBoundaryEvent::SourceScanEvent(SourceScanEvent {
            event_sequence: self.next_sequence(),
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
        if let Some(pending) = self.pending_invalidations.get_mut(&invalidation.scope) {
            pending.revision = coalesce_revision(pending.revision, invalidation.revision);
            return;
        }

        self.pending_order.push_back(invalidation.scope);
        self.pending_invalidations
            .insert(invalidation.scope, invalidation);
    }

    fn drain(&mut self, max_events: usize) -> Vec<LibraryBoundaryEvent> {
        let mut events = Vec::with_capacity(max_events.min(self.pending_order.len()));

        while events.len() < max_events {
            let Some(scope) = self.pending_order.pop_front() else {
                break;
            };
            let Some(invalidation) = self.pending_invalidations.remove(&scope) else {
                continue;
            };

            let sequence = self.next_sequence();
            let event = LibraryBoundaryEvent::MaintainedSnapshotInvalidated(
                MaintainedSnapshotEvent {
                    event_sequence: sequence,
                    occurred_at_ms: Self::current_time_ms(),
                    invalidation,
                },
            );
            self.push_event(event.clone());
            events.push(event);
        }
        events
    }

    fn read_after(
        &self,
        last_seen_event_sequence: Option<i64>,
        max_events: usize,
    ) -> (Vec<LibraryBoundaryEvent>, Option<i64>) {
        let start_from = match last_seen_event_sequence {
            Some(seq) => seq + 1,
            None => {
                let latest = self
                    .stored_events
                    .back()
                    .map(|e| Self::event_sequence(e));
                return (Vec::new(), latest);
            }
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
            .or(last_seen_event_sequence);

        (events, latest)
    }

    fn event_sequence(event: &LibraryBoundaryEvent) -> i64 {
        match event {
            LibraryBoundaryEvent::SourceScanEvent(e) => e.event_sequence,
            LibraryBoundaryEvent::MaintainedSnapshotInvalidated(e) => e.event_sequence,
        }
    }
}

fn coalesce_revision(
    left: Option<MaintainedSnapshotRevision>,
    right: Option<MaintainedSnapshotRevision>,
) -> Option<MaintainedSnapshotRevision> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (None, _) | (_, None) => None,
    }
}
