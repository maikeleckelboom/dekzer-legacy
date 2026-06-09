# Floating Docking Law

**Status:** Accepted
**Domain:** Workspace / Layout Runtime
**Relates to:** ADR-layout-session-law, ADR-layout-runtime-placement

---

## Context

During architectural review of the workspace resize model, the question arose of how Dekzer should position itself
relative to the full-spectrum docking models found in professional creative tools. This ADR records the resulting
room-level law governing topology mutation, floating surfaces, and docking posture.

Dekzer is a performance instrument. That single fact is the dominant forcing function for every decision in this
document.

---

## External Behavior Reference

Adobe's workspace system (Premiere Pro, After Effects, Audition, Bridge) is the strongest external behavior reference
for shared-boundary resize interaction. Its broader panel docking, floating, and workspace mutation model is not adopted
as a room-level law for Dekzer. Adobe is evidence for specific interaction behavior, not architectural authority.

---

## The Core Distinction

Adobe's resize behavior and Adobe's docking culture are separable.

**Resize behavior (adopted in principle):** boundaries shared by multiple surfaces co-negotiate resize under dominance
and participation rules. The specifics are governed by ADR-layout-runtime-placement.

**Docking culture (rejected at room level):** ambient drop zones, free-form panel relocation, casual undocking into
floating windows, topology mutation as a performance-surface affordance. This model is correct for Adobe's product. It
is wrong for a performance instrument.

---

## Non-Goals

This ADR does not define:

- Solver weighting, locality attenuation, or participation economic tuning (see ADR-layout-runtime-placement)
- Local component docking contracts (deferred; bounded by room law here)
- Multi-window or multi-display infrastructure
- The detailed UX structure of topology editing
- Invariants owned by other ADRs, even when referenced here

---

## Definitions

- **Room-level:** pertaining to the workspace topology that governs the performance room as a whole, as distinct from a
  single component's internal contract.
- **Topology mutation:** any structural change to slots, hosts, their relationships, or their participation rules within
  the room-level workspace model.
- **Realization:** the rendered UI projection of runtime/authored workspace state. Realization reflects topology; it
  does not define it.
- **Workspace command:** an explicit declared operation that mutates topology through known slot/host targets, as
  opposed to an implicit affordance such as a drop zone.

---

## Decision

### Room-Level Law

**Floating panels are not first-class in Dekzer.**

The performance workspace is a stable, authored instrument room. All room-level surfaces are governed by the authored
topology and runtime workspace model. They resize, respond to visibility policy, and negotiate space according to
topology ownership rules. The room remains one coherent authored structure.

**Ambient free-form docking is rejected as a room-level workspace law.**

Drop zones, drag-anywhere-dock-anywhere affordances, and casual panel relocation are not part of the performance room's
surface law. This is not a default preference that can be relaxed over time. It is a product law grounded in the
instrument-room contract. It does not prohibit bounded local internal docking inside a component contract (see below).

**Topology mutations require explicit intent.**

Structural changes operate on known slots and hosts through declared workspace commands. They are not triggered by
generic drag-over affordances. No ambient topology mutation is available during performance interaction.

**Topology mutation is separated from performance interaction.**

Topology mutation commands are unavailable during performance interaction. The concrete UX realization of this
separation — whether a literal edit mode, a locked workspace state, permission-gated commands, or edit-only surfaces —
is defined in ADR-layout-session-law. This ADR establishes only the separation requirement.

### Room-Level Positive Commitments

- Topology-native shared-boundary resize co-negotiation under dominance and participation law (see
  ADR-layout-runtime-placement for solver specifics)
- Persistent authored workspaces are the stable baseline. Workspace persistence derives from declared authored topology
  and explicitly committed runtime workspace state, not from incidental drag mutation replay.
- Explicit workspace commands for slot moves, swaps, and structural edits
- Workspace recovery targets a known committed configuration.

### Future Local Docking (Bounded and Optional)

A component may define bounded internal attach/detach or local dock behavior only inside its own contract. Such behavior
does not modify room-level workspace topology unless promoted through explicit workspace commands. Local internal
docking does not automatically participate in room-level resize negotiation unless explicitly bridged by the host
contract. This capability is deferred and introduced only where product value is unambiguous. It is never a
justification to weaken the room-level law.

---

## Rationale

**Against first-class floating panels:**

A live operator depends on the room being exactly where it was. Floating panels destabilize spatial memory, introduce
unpredictable focus and z-order behavior under keyboard shortcuts and MIDI bindings, create hard reconnect and restore
problems during display changes, and make it harder to answer the question "what is the layout right now?" These are not
acceptable tradeoffs for a live system.

**Against adopting Adobe's docking culture:**

Adobe optimizes for maximal layout flexibility in a general-purpose creative workbench. Dekzer optimizes for
repeatability, fast recovery, and stable performance state. Those are different product contracts. Borrowing Adobe's
resize co-negotiation behavior is correct. Borrowing its workspace mutation philosophy would compromise the
instrument-room contract and introduce exactly the ambient structural mutability that the topology ownership model is
designed to prevent.

---

## Invariants

1. **No first-class floating panels at room level.** The performance workspace does not support ambient panel undocking.

2. **Ambient drop zones are absent during performance interaction.** Drag-over docking affordances are not active during
   performance interaction.

3. **Topology mutations are explicit operations.** Structural changes operate on known slots and hosts through declared
   workspace commands, not generic drop targets.

4. **Topology mutation is separated from performance interaction.** The concrete UX form of this separation is defined
   in ADR-layout-session-law.

5. **Resize co-negotiation law is topology-native.** Resize ownership is topological, not geometric or DOM-derived. The
   touched boundary is dominant, and negotiated participation is resolved by runtime/topology rather than by renderer
   adjacency. Detailed negotiation law is defined in ADR-layout-runtime-placement.

6. **Local component docking is a bounded future capability, not a room-level law.** If introduced, it stays fully
   inside the declaring component's contract, does not modify room-level topology without explicit workspace commands,
   and does not participate in room-level resize negotiation unless explicitly bridged by the host contract.

7. **Runtime/topology owns structural presence and visibility policy.** Whether a surface is part of room-level
   workspace topology, and the policy by which it is shown, suppressed, or restored, are topology/runtime decisions.
   These are not determined by DOM position, incidental layout state, or realization-layer mutation.

8. **Runtime/topology owns participation eligibility for resize and topology operations.** Eligibility to co-negotiate a
   boundary or to be the target of a topology command is topology-resolved. Rendered affordances may reflect eligibility
   but do not establish it.

9. **Realization is a projection, not a source of authority.** Topology state is projected into realization. Realization
   is never read back as workspace authority. DOM structure, visual adjacency, and rendered drop affordances do not
   determine slot presence, participation eligibility, or workspace structure.

10. **Workspace persistence is explicit and recoverable.** Persisted workspace state derives from declared authored
    topology and explicitly committed runtime workspace state, not from incidental drag mutation history or realization
    residue. Workspace recovery targets a known committed configuration.

---

## Consequences

- The workspace layout engine does not implement ambient room-level docking drop zones as part of the baseline workspace
  contract.
- Room-level floating-window orchestration is not part of the baseline workspace contract. Future multi-window work, if
  any, must clear a separate product and recovery bar before it is introduced.
- Topology-edit operations are first-class workspace commands on hosts and slots, not drag affordances.
- Workspace persistence derives from declared authored topology and explicitly committed runtime workspace state. Drift
  through incidental drag mutation does not accumulate into saved layout state.
- When local component docking is introduced, it is scoped inside a component host and designed against that component's
  specific contract. It is not promoted to a generic room-level capability and does not automatically inherit room-level
  resize negotiation.
- Adobe remains a valid behavior reference for shared-boundary resize interaction only. Its panel-placement philosophy
  is not an adoption target.
