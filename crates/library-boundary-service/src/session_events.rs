use std::collections::{BTreeMap, VecDeque};
use std::sync::Mutex;

use library_boundary_protocol::{
    LibraryBoundaryEvent, MaintainedSnapshotInvalidation, MaintainedSnapshotRevision,
    MaintainedSnapshotScope, MaintainedSnapshotScopeRevision,
};

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

    pub(crate) fn drain(&self, max_events: usize) -> Vec<LibraryBoundaryEvent> {
        let mut state = self.state.lock().expect("boundary event stream poisoned");
        state.drain(max_events)
    }
}

#[derive(Debug, Default)]
struct LibraryBoundaryEventStreamState {
    observed_revisions: BTreeMap<MaintainedSnapshotScope, MaintainedSnapshotRevision>,
    pending_order: VecDeque<MaintainedSnapshotScope>,
    pending_invalidations: BTreeMap<MaintainedSnapshotScope, MaintainedSnapshotInvalidation>,
}

impl LibraryBoundaryEventStreamState {
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
            events.push(LibraryBoundaryEvent::MaintainedSnapshotInvalidated(
                invalidation,
            ));
        }
        events
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
