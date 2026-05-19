// Root lifecycle remains schema-backed substrate for future device/root work,
// but it is not part of the maintained boundary protocol center.
#![allow(dead_code)]

mod lifecycle;

pub(crate) use lifecycle::{
    ApplyRootChangedInput, ApplyRootEjectCancelledInput, ApplyRootMountedInput,
    ApplyRootUnmountPendingInput, ApplyRootUnmountRequestedInput, ApplyRootUnmountedInput,
    RegisterRemovableRootInput, ResolvedRoot, RootStatusDelta,
};
pub(crate) use lifecycle::{CanonicalSourcePath, SourceLifecycleTx};
#[cfg(test)]
pub(crate) use lifecycle::{RootIdentityKind, RootMountStatus};
