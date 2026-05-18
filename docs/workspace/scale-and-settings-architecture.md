# Dekzer scale and settings architecture

## Status

Accepted working direction.

This document writes down the scale/settings model for Dekzer desktop so the system can grow without hidden coupling.

---

## Core decision

The current direction is correct.

But it is only correct if the boundary stays sharp:

- the **core settings substrate** owns composition, layering, inheritance, persistence, and resolution orchestration
- each **module** owns the meaning of its own settings
- each **instance** may override only within the module's declared contract
- renderers consume **resolved view config**, not raw layered settings

The core must not know what a deck, mixer, browser, waveform, or inspector is.

That is the line.

---

## The three scale families

Dekzer has three distinct scale families.

### 1. Display scale

This is whole-renderer optical scale.

Purpose:

- accessibility
- screen-distance comfort
- presentation mode
- unusual DPI / monitor setups

Canonical law:

```text
scale = 1.2 ^ level
```

The persisted app setting is the zoom **level**, not an arbitrary factor.

```ts
export type AppZoomLevel = -3 | -2 | -1 | 0 | 1 | 2 | 3 | 4 | 5 | 6;
```

Notes:

- `0` is default
- negative values are valid because this is a level model, not a factor model
- UI should usually show a derived percent, not a fake label like `-1x`

Display scale is the outer optical layer only.
It is not the owner of semantic density.

---

### 2. Global UI scale baseline

This is the app-wide semantic density baseline.

```ts
export type UiScalePreset = 'dense' | 'standard' | 'spacious';
```

Purpose:

- table cell density
- tree row density
- inspector spacing rhythm
- compact vs relaxed control sizing
- typography baseline bias for dense vs relaxed reading

This is not "font size only".
This is a surface-density baseline.

Important:

- this baseline is a default suggestion
- modules may honor it strongly, partially, or ignore it
- nothing responds automatically unless it explicitly opts in

---

### 3. Domain/module-local scale

This is module-owned semantic scale.

Examples:

- browser density
- mixer strip density
- deck information emphasis
- waveform zoom
- artwork scale
- inspector compactness

These are not core settings concepts.
These are module concepts.

---

## The ownership law

**The core owns settings composition. Modules own settings meaning.**

This is the sentence to preserve.

The failure modes are:

1. **Domain leakage into core**
  - bad: the shared settings substrate knows what a mixer or deck is

2. **Fake abstraction**
  - bad: the system uses meaningless generic numeric knobs with no semantic owner

The correct middle is:

- generic composition mechanism
- module-specific semantic schemas

---

## Layering model

There are three scopes.

### App scope

Cross-app defaults.

Examples:

- `appZoomLevel`
- `uiScale`

### Module scope

Default settings for a module family.

Examples:

- browser module defaults
- mixer module defaults
- deck module defaults

### Instance scope

Overrides for one realized module instance.

Examples:

- this specific mixer
- this specific browser surface
- this specific inspector

---

## Generic core model

The shared substrate should stay generic.

Example shape:

```ts
export type SettingScope =
  | { kind: 'app' }
  | { kind: 'module'; moduleKey: string }
  | { kind: 'instance'; moduleKey: string; instanceKey: string };

export type SettingNamespace = string;

export type SettingLayer<TSettings> = {
  namespace: SettingNamespace;
  scope: SettingScope;
  values: Partial<TSettings>;
};
```

The core may own:

- scope hierarchy
- namespace registration
- persistence
- override precedence
- migrations / versioning
- validation hooks
- resolution lifecycle

The core must not own:

- browser density meaning
- mixer strip semantics
- deck emphasis modes
- waveform zoom semantics

---

## Module-owned schemas

Modules define their own semantic setting contracts.

Examples:

```ts
export type BrowserAppearanceSettings = {
  density?: 'dense' | 'standard' | 'spacious';
  columnHeaderStyle?: 'compact' | 'standard';
};

export type MixerAppearanceSettings = {
  stripDensity?: 'compact' | 'standard' | 'performance';
  meterLabelVisibility?: 'hidden' | 'compact' | 'full';
};
```

This is correct ownership.
The browser owns browser semantics.
The mixer owns mixer semantics.

---

## Capability law

Nothing should react to global settings by accident.

A module must explicitly declare what it honors.

Examples of capability categories:

- inherits global UI scale baseline
- supports module-local density preset
- supports instance override
- supports typography override
- supports domain-local zoom

The exact capability model can evolve, but the law is stable:

**opt-in, never ambient coupling**

---

## Resolution law

Raw layered settings are not for renderers.

A module resolver should take layered inputs and produce one resolved config.

Resolution order:

```text
instance override
-> module default
-> app baseline
-> hard product fallback
```

The renderer should consume only resolved values.

Bad:

```ts
if (global.uiScale === 'dense') { ...
}
if (moduleSettings.stripDensity === 'compact') { ...
}
if (instanceSettings?.meterLabelVisibility === 'full') { ...
}
```

scattered through rendering code.

Good:

```ts
export type ResolvedMixerViewConfig = {
  stripWidthPx: number;
  controlHeightPx: number;
  meterLabelVisibility: 'hidden' | 'compact' | 'full';
  typographyScale: number;
};
```

One resolver owns the merge.
The renderer receives resolved config only.

---

## Display settings contract

The cross-app display contract should stay small.

```ts
export type DisplaySettings = {
  appZoomLevel: AppZoomLevel;
  uiScale: UiScalePreset;
};
```

This is the app-owned baseline.

Do not turn it into a giant universal scale object.
Do not make every future module setting part of this type.

---

## What global UI scale may influence

Global UI scale may influence, if a module opts in:

- row height buckets
- cell padding buckets
- tree indent rhythm
- inspector spacing rhythm
- compact control height buckets
- typography baseline bias

Global UI scale must not directly own:

- resize negotiation law
- workspace topology law
- waveform zoom
- mixer strip geometry semantics
- deck-local emphasis semantics
- domain-specific canvas zoom

---

## Domain examples without leaking them into core

This distinction matters.

Correct:

- browser module decides how `dense | standard | spacious` affects row height and cell padding
- mixer module decides how `compact | standard | performance` affects strip width, label visibility, and control spacing
- waveform module decides its own zoom law entirely outside global density

Incorrect:

- core settings substrate hardcodes browser, mixer, deck, and waveform settings as built-in product nouns

---

## Anti-goals

Do not do these.

### 1. One universal scalar

```ts
scale: number
```

This destroys semantics.

### 2. Core registry of domain nouns

```ts
type ModuleSettings = BrowserSettings | MixerSettings | DeckSettings;
```

This couples the foundation to the current product surface list.

### 3. Ambient inheritance

Surfaces should not react to global settings unless they declared that behavior.

### 4. Renderers reading raw settings everywhere

Renderers should receive resolved config, not perform their own inheritance logic.

### 5. Typography standing in for density

Text size alone is not a valid density model for a workstation UI.

---

## Design test

Use this test repeatedly:

**Can a brand new module be added without editing the shared settings substrate?**

If the answer is no, the boundary is wrong.

The correct outcome is:

- define a new namespace
- define a new module schema
- define a resolver
- define realized config/tokens
- done

No core surgery.

---

## Acceptance bar

The architecture is correct if all of the following stay true:

1. The shared substrate is generic and domain-agnostic.
2. Modules own semantic meaning.
3. Instances override only inside module law.
4. Global UI scale is a baseline, not a universal command.
5. Display scale remains separate from semantic density.
6. Renderers consume resolved config only.
7. New modules can be added without editing the substrate core.

---

## Current decision

Proceed with this model.

Harden these statements:

- **Display scale** is outer optical scale.
- **UI scale** is a global semantic density baseline.
- **Modules** own local semantic settings.
- **Instances** own exceptions.
- **The core owns composition. Modules own meaning.**

That is the version worth building.

