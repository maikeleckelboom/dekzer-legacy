# Dekzer visual workspace doctrine

**Governs:** visual product law — shell language, field language, optical rules, review standard.
**Does not govern:** topology substitution, slot representation, or responsive mapping.
**See also:** `topology-families.md`, `topology-modeling-law.md`.

---

## Status

This document is the canonical visual product law for Dekzer. It is the decision authority for all questions about how
the product must look and what it must visually communicate. It does not yield unless a newer canon document explicitly
supersedes it.

## Purpose

This document locks visual product law. It defines shell ownership, field ownership, optical rules, and the review
standard every screen must pass. This is product law, not commentary.

## Core statement

Dekzer uses a contained premium desktop shell around continuous, mode-native instrument fields.

The shell stays stable across the product. The active field changes by mode because different musical tasks are
different instruments. Dekzer uses one shell grammar and multiple field grammars.

## Primary law

The product always reads as one product, one shell, one chrome language, one divider language, one accent logic, one
inspector language.

Inside that shell, the active field changes shape to match the current instrument.

## Decision priority

When laws appear to compete, use this order:

1. instrument clarity
2. structural honesty
3. slot continuity
4. responsive truthfulness
5. shell consistency
6. visual polish

A cleaner-looking screen is still wrong if it hides the active instrument, breaks topology honesty, or introduces false
responsive behavior.

## Layer ownership

### Shell

The shell owns product identity and containment.

Shell includes: app frame, caption and header zone, global status and transport context, module rail navigation when the
shell keeps a durable rail, stable tonal framing, overlay containment language.

Shell feels: premium, calm, deliberate, reliable, desktop-native, custom to Dekzer.

Shell must not feel: like a browser app, like a web dashboard, like a VS Code clone, like a JetBrains clone, like
floating cards on a canvas.

### Main field

The main field owns the active instrument. It is continuous by default — one working surface partitioned by function,
not a pile of widgets.

The field changes structure by mode but keeps the same discipline: shared edges, crisp dividers, restrained depth,
strong role hierarchy, low ornamental noise.

### Secondary utilities

Secondary utilities are contained and subordinate. Inspectors, pickers, dialogs, popovers, side utilities, and transient
tools may use more containment than the main field but must not visually overpower the active instrument.

## Explicit optical spec

### Where radius appears

Radius follows containment. Use radius on: outer shell, dialogs, popovers, command surfaces that float temporarily above
the field, detached utility trays, explicit object cards treated as independent contained entities.

Radius is absent or near-absent on: deck-to-mixer boundaries, primary split boundaries, browser, library, and details
partitions inside one continuous field, table region boundaries, lane partitions inside a continuous performance or
preparation field.

Practical rule: outer shell radius is visible. Overlay radius is visible. Internal field radius is zero or near-zero.

### What dividers look like

Dividers are precise structural cuts, not decorative borders.

Primary characteristics: 1 px optical weight, dark-to-slightly-lighter tonal step, straight geometry, continuous where
structure is continuous, stronger contrast only at major regional boundaries.

Allowed strengths: faint for table cell rhythm, normal for primary field partitioning, elevated only at active resize or
active drop targets.

Forbidden: double borders, border-plus-glow-plus-shadow stacks, embossed chrome lines, many competing divider styles on
one screen, rounded-outline panel borders as default partitioning.

Resize rule: idle boundary stays quiet. Hover may sharpen slightly. Active boundary is crisp and clearly owned. Only
directly affected regions receive companion emphasis.

### How many tonal levels exist

Dekzer uses a small disciplined tonal ladder.

1. window surround and far background
2. shell surface
3. active field base
4. subordinate region tone
5. contained overlay or card tone
6. active selection or armed emphasis

Level 6 is an emphasis state, not a fill system. No screen needs more than five resting tonal planes plus one active
emphasis plane. If more are needed, the structure is wrong.

### When a card is legal

A card is legal only when the thing shown is a contained object, not a structural partition of the active field.

Legal: sleeve objects, route objects, candidate objects, explicit media identity objects, detached suggestions, dialogs,
popovers, temporary compare surfaces.

Conditionally legal: inspector groupings, contained utility trays, onboarding surfaces.

Not legal: default deck regions, default mixer regions, browser, library, and details partitions inside one field, every
module in a performance view.

Hard rule: if the object should read as part of the same working plane, do not card it.

### How quiet the left utility strip is

The left utility strip exists to switch instruments, expose a small number of durable tool families, and provide
orientation. It does not become the dominant visual signature.

Required: narrower than a typical IDE activity rail, darker or lower-contrast than the main field, sparse icon set,
restrained labeling, consistent icon geometry, calm hover and selected states.

Visual rule: the eye lands on the active field before it lands on the left strip. If the strip is the loudest element on
screen, it is wrong.

### How active emphasis behaves

Active emphasis is local, causal, and temporary. It appears where action is happening. It propagates only to directly
affected regions. Unrelated areas stay calm.

Examples: active deck gets modest accent increase and slightly stronger internal contrast. Active route node gets edge
or border emphasis on the selected node and directly related path only. Active resize boundary gets a crisp line and
light companion emphasis on the two negotiated regions.

Forbidden: many simultaneous glowing regions, animated chrome that does not track meaningful state, pulsing accents
across unrelated modules, broad bloom that reduces structural clarity.

Motion rule: transitions confirm state change. They do not perform for attention.

### What first glance must communicate

In the first two seconds, a screen must communicate:

1. what mode or instrument is active
2. what surface currently owns attention
3. what is playing, armed, selected, or live
4. what belongs to the main field versus supporting it
5. where the next action is likely to happen

Failure signs: the eye has no landing point. Cards compete equally with the instrument. Shell chrome is louder than
operational content. Everything looks selected. The screen looks impressive before it looks usable.

## Always, sometimes, never

### Always

- show one clearly dominant instrument
- keep shell ownership distinct from workspace ownership
- let topology explain the screen before styling does
- preserve slot continuity across mode and size changes
- keep support regions visibly subordinate to the dominant field
- use containment only when semantic containment is real
- make responsive substitution preserve instrument truth

### Sometimes

- use contained object cards for sleeves, candidates, compare surfaces, and other true contained objects
- use a horizon lane when the mode genuinely benefits from visible commit, route, or live timeline context
- use a partial-height rail when the upper band truly owns the screen and the termination point is structurally
  meaningful
- widen support context on large displays when it improves clarity without flattening hierarchy
- collapse or park support surfaces when smaller sizes require stronger focus

### Never

- let shell chrome compete with the active instrument
- use card treatment as the default partitioning language of the main field
- preserve desktop parity dishonestly on small screens
- invent new topology slots for every feature request
- allow multiple equal dominant regions on one screen
- use decorative emphasis that does not track meaningful state
- make the left utility strip the loudest element on screen
- produce a screen that looks impressive before it looks usable

## Final review checklist

A screen proposal is not ready until all of the following can be answered clearly:

1. what is the dominant instrument
2. what topology family is active
3. which slots are shell-owned versus workspace-owned
4. which slots are persistent versus summonable
5. what collapses first as width decreases
6. what remains continuous across topology change
7. whether any card treatment is semantically justified
8. whether first glance reveals action readiness

If any answer is vague, the screen is not final.

## See also

- `topology-families.md` — legal topology families and their conditions
- `responsive-topology-matrix.md` — size classes and substitution rules
- `topology-modeling-law.md` — slot representation and continuity law
