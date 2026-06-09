# Responsive topology matrix

**Governs:** size classes, topology substitution by mode and size, collapse order.
**Does not govern:** topology family definitions, optical law, or slot representation.
**See also:** `topology-families.md`.

---

## Status

This document is the canonical responsive topology matrix for Dekzer. It defines size classes, the mode-to-size topology
mapping, and substitution rules.

## Purpose

Define which topology family is active at each size class and mode, and how the product collapses as screens get
smaller. Responsiveness is not a cosmetic shrink pass. It is a controlled topology substitution system.

## Core responsive law

Responsiveness chooses between canonical topology families. It does not compress one desktop layout until it becomes
illegible.

At every size class, the product must preserve: clear active instrument ownership, stable shell identity, a legible
action horizon, inspectability, credible input targets, predictable mode switching.

## Size classes

### Compact handheld

For phones and very narrow portrait surfaces.

Allows: one dominant active field, optional bottom action tray or mode tray, secondary surfaces as sheets, drawers, or
step-in views.

Forbids: multiple persistent regions, inspector as a docked pane, side rails, full desktop field parity.

This is a focused instrument view, not a compressed desktop.

### Compact landscape and small tablet

For larger phones in landscape and small tablets.

Allows: one dominant field, one subordinate lane or drawer or side support region, temporary inspector or utility
reveal.

Forbids: more than one major support region remaining persistently visible, route canvas at full size with both
inspector and commit lane simultaneously, perform views showing every support surface at once.

### Laptop and standard desktop

For laptop screens and ordinary desktop windows. The minimum class where the full product feels native.

Allows: perform bench, library bench, route canvas, authority stack workbench, central operator bench, analysis bench.
One persistent support region plus an optional secondary lower lane when the mode genuinely requires it.

Forbids: gratuitous fourth or fifth equal-weight region. Lower support zones must remain subordinate to the dominant
field.

This is the primary truth class for day-to-day use.

### Wide monitor and large desktop

For large monitors and wide desktop windows.

Allows: additional support surfaces where they increase clarity. Inspectors may widen. Compare surfaces may appear.
Lower lanes may remain visible more often.

Forbids: using extra width to fill the screen with equally loud modules. The active instrument must still win at first
glance.

## Mode × size-class topology matrix

| Mode                   | Compact handheld          | Compact landscape / small tablet             | Laptop / standard desktop                       | Wide monitor / large desktop                                      |
| ---------------------- | ------------------------- | -------------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------- |
| Perform                | focused instrument view   | split support view                           | perform bench                                   | perform bench with persistent inspector or secondary support      |
| Library                | focused instrument view   | split support view                           | library bench                                   | library bench with wider details or secondary preview support     |
| Sleeves and routes     | focused instrument view   | split support view                           | route canvas                                    | route canvas with persistent inspector and commit lane            |
| Analysis               | focused instrument view   | split support view                           | analysis bench                                  | analysis bench with comparison or expanded verification support   |
| Utilities and overlays | sheets, drawers, popovers | split support view or contained utility tray | contained utility tray or docked support region | contained utility tray, docked support region, or compare surface |

### Reading the matrix

For each screen, answer explicitly:

1. what mode owns the screen
2. what size class is active
3. what topology family is being used
4. what slot is the dominant instrument
5. what regions are support only

If those answers are not obvious, the screen is not ready.

## Responsive topology mapping by mode

### Perform mode

- handheld: one deck or one active perform surface, browser or mixer support summoned
- small tablet: deck-focused field plus compact support lane
- laptop: perform bench
- wide monitor: perform bench with persistent inspector or additional support context

### Library mode

- handheld: focused list with staged detail drill-in
- small tablet: navigation plus list, details summoned
- laptop: library bench
- wide monitor: library bench with wider details or secondary preview and metadata support

### Sleeves and routes mode

- handheld: route stepper or focused node view, not full freeform canvas
- small tablet: reduced graph plus inspector summary or commit lane
- laptop: route canvas with inspector and optional commit lane
- wide monitor: full route canvas with persistent inspector and commit lane, plus secondary context where justified

### Analysis mode

- handheld: focused waveform task view
- small tablet: waveform plus compact metadata or verification support
- laptop: waveform-centric field with table or inspector support
- wide monitor: expanded waveform comparison or multi-track verification field

## Rules for topology substitution

- Moving down in size class reduces simultaneous persistent regions.
- Moving up in size class reveals more support context, not more equal-weight modules.
- Mode changes may switch topology family, but shell law stays stable.
- Topology substitution must preserve slot identity and continuity law.
- A mode may only use a topology family when that family honestly matches the instrument.

## Collapse order principle

When reducing size class, collapse in this order:

1. secondary object hosts (object row, candidate surfaces) collapse first
2. non-essential lower support surfaces collapse before persistent side support
3. support surfaces convert to summonable before the dominant field is compressed below credible use
4. authority context collapses only after non-essential support, unless the mode is not authority-led
5. the dominant instrument field is the last region to be compromised

Small screens preserve focus. Large screens preserve context. This order is not reversible.

Support becomes summonable before the dominant instrument becomes dishonest.

## Rejection rules

Reject any responsive design that:

- scales the desktop down until targets and hierarchy fail
- preserves too many simultaneous equal-weight regions on small screens
- hides critical operational state without a clear recovery path
- turns the product into a tab maze with no dominant instrument
- uses wide screens to add decorative modules with no ownership
- relies on hover as the only access path to any mode
- allows desktop-only chrome assumptions to leak into handheld classes
- swaps to a topology family that does not honestly match the mode's instrument

## See also

- `topology-families.md` — definitions of each named topology family
- `visual-workspace-doctrine.md` — visual law governing all topology substitutions
- `topology-modeling-law.md` — slot identity and continuity during substitution
