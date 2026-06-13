# Canonical topology families

**Governs:** legal workspace topology families, when each is permitted, and what each topology owns.
**Does not govern:** optical law, tonal or radius doctrine, slot-state implementation.
**See also:** `visual-workspace-doctrine.md`, `responsive-topology-matrix.md`.

---

## Status

This document is the canonical topology family registry for Dekzer. It defines the bounded set of named workspace
topology families and the conditions under which each may be used.

A screen proposal must classify against one of these families without argument. A new family requires a canon-level
process.

## Purpose

Define legal layout families. Proposals that cannot classify against one of these families are not ready.

## Mode-native field doctrine

Each mode has a natural instrument. The topology family used must match it.

### Perform mode

Privileges: decks, mixer, immediate browser support, cue, sync, timing, and action readiness. Must feel continuous and
operational. Must not degrade into a dashboard.

### Library mode

Privileges: source and structure navigation, dense track lists, metadata clarity, fast filtering, deliberate loading.
May be denser and more information-forward than perform mode.

### Sleeves and routes mode

Privileges: sleeves as workflow objects, route and transition objects, branching and fallback, commit lanes, explainable
next-step reasoning. This is a planning instrument, not a decorated playlist view.

### Analysis mode

Privileges: alignment, phrase structure, cue certainty, verification state, technical confidence. This is a specialist
field, not the default product face.

### Utility and overlay mode

Supporting instruments only. Sampler tools, preparation popovers, transition editors, and quick utilities remain
subordinate to the active field.

## Focused instrument view

### Purpose

Use for any mode at compact handheld size. Use as the fallback when no multi-region layout can be shown honestly.

### Dominant slot

One instrument surface owns the full active area.

### Support slots

Temporary only: sheets, drawers, step-in views, or bottom action tray.

### Legal uses

- all modes at handheld size
- any mode where only one surface can be shown with credible hierarchy

### Illegal distortions

- persistent side or bottom regions that compete with the instrument
- preserving panel count at the cost of instrument clarity

### Responsive substitution

This is the collapse target for all modes at handheld size and the floor below which no further reduction is possible.

## Split support view

### Purpose

Use for any mode at compact landscape or small tablet size.

### Dominant slot

One dominant field.

### Support slots

One persistent subordinate lane, drawer, or side support region. Inspector or utility as temporary reveal.

### Legal uses

- all modes at compact landscape or small tablet size
- route canvas may keep a shortened commit lane or inspector summary, not both at full size

### Illegal distortions

- more than one major support region remaining persistently visible
- support region matching the dominant field in visual weight

### Responsive substitution

At the lower edge of this class, substitute to focused instrument view.

## Perform bench

### Purpose

Use for perform mode at laptop and wide-monitor sizes.

### Dominant slot

Upper instrument field containing decks, mixer, and immediate action surfaces.

### Support slots

- lower browse or support field
- optional inspector region, typically right-anchored

### Legal uses

- perform mode at laptop size and above
- when the upper instrument field remains clearly dominant over all lower regions

### Illegal distortions

- lower regions expanding to equal the upper instrument field in visual weight
- lower region using card grid instead of continuous surface discipline

### Responsive substitution

- wide monitor: perform bench with persistent inspector or additional support context
- laptop: perform bench as primary composition
- smaller: substitute to split support view

## Library bench

### Purpose

Use for library mode at laptop and wide-monitor sizes.

### Dominant slot

Center table field. Primary working plane for track selection, sorting, and scanning.

### Support slots

- left hierarchy and navigation region
- right details or inspector region

### Legal uses

- library mode at laptop size and above
- when the table is the dominant surface and hierarchy supports filtering without competing

### Illegal distortions

- left hierarchy or right inspector growing to equal the table in visual weight
- table region treated as secondary to navigation
- table styled like a generic admin grid

### Responsive substitution

- wide monitor: library bench with wider details or secondary preview support
- laptop: library bench as primary composition
- smaller: substitute to split support view

## Route canvas

### Purpose

Use for sleeves and routes mode at laptop and wide-monitor sizes.

### Dominant slot

Central graph field.

### Support slots

- side inspector, typically right-anchored
- optional lower commit lane

### Legal uses

- sleeves and routes mode at laptop size and above
- when the graph field maintains clear dominance over inspector and lane

### Illegal distortions

- inspector and commit lane combining to exceed or equal the graph field in visual weight
- commit lane expanding from thin lane to a competing panel
- route canvas used for modes that are not graph-centric

### Responsive substitution

- wide monitor: full route canvas with persistent inspector and commit lane
- laptop: route canvas with inspector and optional commit lane
- smaller: substitute to split support view

## Central operator bench

### Purpose

Use when the mode owns a center-commanded operator surface with flanking supports that does not map honestly to perform
bench or library bench.

### Dominant slot

Center operator surface.

### Support slots

- left flanking support
- right flanking support or inspector

### Legal uses

- when the active instrument is genuinely center-led with equally flanked context
- when neither perform bench nor library bench captures the instrument correctly

### Illegal distortions

- three equal-weight panels without a clear dominant surface
- using this family as a convenient three-column default rather than a mode-honest choice

### Responsive substitution

- smaller sizes: substitute to split support view with the center instrument preserved

## Analysis bench

### Purpose

Use for analysis mode at laptop and wide-monitor sizes.

### Dominant slot

Waveform field.

### Support slots

- subordinate data region or inspector
- optional comparison surface at wide-monitor sizes

### Legal uses

- analysis mode at laptop size and above
- when waveform or verification work is the primary instrument

### Illegal distortions

- support regions growing to compete with the waveform field
- analysis bench used as the default product face rather than a specialist mode
- comparison arrangements that destroy dominance hierarchy

### Responsive substitution

- wide monitor: analysis bench with comparison or expanded verification support
- laptop: waveform-centric field with table or inspector support
- smaller: substitute to split support view

## Authority stack workbench

### Purpose

Use when the screen must keep a visible authority stack on the left while granting one large dominant field to the
active instrument.

Appropriate when the user simultaneously needs: durable orientation through a left authority region, one clearly
dominant working surface, contextual support modules near that field, a visible horizon lane, and a contained object
row.

This is not the default topology for every screen. It is valid for complex preparation, hybrid perform-plus-library
work, and authority-heavy workbench screens.

### Shell behavior

The shell wraps the entire topology. Titlebar, caption controls, and outer shell framing continue across the full
composition. They do not stop at the dominant field. The module rail and all internal regions live inside the shell.

The horizon lane is not shell chrome. It is a topology-owned workspace pane when it carries mode-owned operational
content.

### Dominant slot

Upper-right primary field. Owns first-glance attention. May host performance surface, waveform field, route or canvas
field, hybrid preparation surface, or dense operator surface. Must remain visually dominant over the left authority
stack and lower object row.

### Support slots

- module rail: narrow persistent navigation or tool access
- authority upper: left-side primary authority region
- authority lower: left-side secondary authority or library region
- mid support left: contextual support under the dominant field
- mid support center: contextual support under the dominant field
- mid support right: broader support or inspector-adjacent surface under the dominant field
- horizon lane: thin full-width commit, route, status, or action lane
- object row: multi-object contained row for candidates, sleeves, suggestions, or secondary modules

Support slots are not equal peers to the dominant slot.

### Visual priority rules

Correct read order:

1. dominant upper-right field
2. currently active left authority region
3. horizon lane if it carries live or commit consequence
4. mid support row
5. lower contained object row
6. module rail

If the eye lands first on the module rail or the object row, the hierarchy is wrong.

### Containment rules

- dominant upper-right field remains continuous
- left authority stack may be partitioned but reads as a coherent authority column
- mid support row stays quieter than the dominant field
- horizon lane remains thin and linear
- object row may use contained object cards because it hosts contained objects, not main field partitions

### Legal uses

- perform-plus-library workbenches where the upper-right instrument remains dominant
- preparation screens with a strong left authority stack and a large active work surface
- route or transition workflows needing a visible horizon lane and lower candidate lane
- hybrid screens where authority, active field, and object candidates must coexist without destroying dominance

### Illegal distortions

- left authority stack becomes visually equal to the dominant field
- object row treated as a structural field partition instead of contained secondary objects
- horizon lane expanded until it competes with the dominant field
- every support slot given equal visual weight
- shell titlebar visually stops at one region instead of wrapping the whole composition
- used for simple screens that would be clearer as perform bench, library bench, or route canvas

### Responsive substitution

This topology is primarily a laptop and wide-monitor family.

- smaller laptop widths: collapse or reduce the object row into a summonable tray first
- compact tablet widths: collapse or convert one authority region into a drawer
- handheld: do not preserve this topology; substitute to focused instrument view

### Future variant

A partial-height module rail is legal when the rail belongs only to the upper instrument or authority band and
terminates on a meaningful structural boundary. The shell still wraps the full composition.

## Partial-height rail variant

A module rail does not need to span the full shell height to be valid.

Use when: the rail belongs primarily to the upper instrument band, the lower data region should read as more continuous
and less shell-framed, and the active mode is strongly top-band or instrument-led.

Rules: termination point must align with a meaningful regional boundary. Must not look accidentally cut off. Lower
regions must retain orientation and recovery paths. Rail must remain quieter than the active instrument.

This is a deliberate shell variant, not a visual flourish.

## See also

- `visual-workspace-doctrine.md` — visual product law governing all families
- `responsive-topology-matrix.md` — which family to use at each size and mode
- `topology-modeling-law.md` — how topology slots are represented and kept continuous
