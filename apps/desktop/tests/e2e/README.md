# Library V0 E2E Acceptance

This directory contains the Library V0 product-acceptance E2E lab for the desktop Electron app.

## What It Proves

The Library V0 acceptance gate proves the covered contract for generated local library sources:

- the built app launches with isolated user data;
- a generated music folder can be admitted, scanned, browsed, filtered, removed, and kept removed;
- search respects the active source or folder scope;
- an admitted source persists across graceful restart;
- a missing/offline admitted source is presented honestly and can recover after restore;
- removing and re-adding the exact same path does not leave duplicate active sources.

## What It Does Not Prove

The gate does not prove waveform, Prepared Room, adapters, playlists, crates, collection authority, or broad source-health cockpit behavior. It does not prove real media compatibility. Fixtures intentionally use generated tiny WAV files and companion text/image files.

## Running Scenarios

From `apps/desktop`:

```bash
pnpm run test:e2e:library-v0
pnpm run test:e2e:library-v0:repeat
pnpm run verify:e2e:library-v0
```

Individual scenarios can be run by tag:

```bash
pnpm run test:e2e -- --grep @library-v0:launch
pnpm run test:e2e -- --grep @library-v0:golden-smoke
pnpm run test:e2e -- --grep @library-v0:scoped-search
pnpm run test:e2e -- --grep @library-v0:restart-persistence
pnpm run test:e2e -- --grep @library-v0:missing-source-health
pnpm run test:e2e -- --grep @library-v0:remove-readd-freshness
```

`verify:e2e:library-v0` builds the desktop app and boundary binary before running the acceptance gate. The Library V0 gate is intentionally not part of normal `verify` yet.

## Scenario Registry

Scenario metadata lives under `support/scenarios`:

- `libraryV0Scenarios.ts` defines stable scenario ids, invariant summaries, fixture requirements, expected failure evidence, and gate membership.
- `fixtureCatalog.ts` documents generated source fixtures.
- `acceptanceTags.ts` centralizes Playwright grep tags.

Keep this registry descriptive. Do not turn it into a second test framework.

## Diagnostics

Acceptance specs run through `LibraryV0AcceptanceRun`. On failure it attaches:

- `library-v0-diagnostics.json`;
- `library-v0-failure-classification.md`;
- the existing Electron fixture attachments for stdout, stderr, main/renderer console, page errors, window placement, shutdown logs, screenshot, and trace where available.

Diagnostics use public preload APIs and role-based visible surfaces. They do not inspect Vue internals and do not take broad DOM snapshots.

The runtime bundle includes local roots, host status, persisted view state, source lifecycle/integrity/activity/maintenance snapshots for admitted test sources, user-data path, fixture paths, visible Library title, visible contents rows, source status text/actions, and search query when the search field is open.

## Failure Classification

`LibraryFailureClassifier` is deliberately rule-based. It classifies failures as:

- `product-blocker`;
- `harness/setup-issue`;
- `undefined-product-contract`;
- `likely-flake/timing-issue`.

The classifier uses scenario id, last phase, error shape, runtime probe availability, and simple UI/runtime contradiction checks. It is a triage aid, not product truth.

## Fixtures

Generated fixtures are created under each Playwright test output directory:

- Source A: root audio, nested audio, descendant-only audio, empty folder, and mixed companion files.
- Source B: one audio file for cross-source search isolation.

Missing/offline simulation uses `moveSourceOffline` and `restoreOfflineSource` from `filesystem.fixture.ts`. Do not add real media binaries. Keep cleanup safe after partial failures.

## Locator Policy

Tests should use the canonical support path:

- `support/domain`;
- `support/screens`;
- `support/locators`.

Prefer roles, labels, and visible product text. Do not add CSS or XPath selectors. `data-testid` is allowed only when a product surface has no stable accessible role/name and the hook represents a durable product-testing affordance. No Library V0 acceptance hook currently uses `data-testid`.

## Window Placement

The Electron harness applies deterministic window placement and attaches placement logs on failure. Use the existing placement support and environment variables rather than per-spec window manipulation.

## Adding A Scenario

1. Add metadata to `support/scenarios/libraryV0Scenarios.ts`.
2. Use generated fixtures or add a deterministic helper in `fixtures/filesystem.fixture.ts`.
3. Express user actions through `LibraryV0` domain helpers or screen objects.
4. Wrap the spec in `createLibraryV0AcceptanceRun`.
5. Assert product-visible behavior and public runtime evidence without coupling to Vue internals.
6. Add only bounded diagnostics that reduce failure interpretation time.

If behavior is not defined by the product contract, stop and document the contract decision needed before encoding a permissive assertion.
