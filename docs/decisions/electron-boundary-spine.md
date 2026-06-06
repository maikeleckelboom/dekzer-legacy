## Implementation sequence

Do not mix this work with browse-policy omission metadata.

Do not mix this work with source-root admission/default discovery.

Recommended sequence:

1. Finish or merge source-add activation if already green.
2. Finish browse-policy omission metadata inventory and implement that slice only if the inventory proves the boolean
   can be computed honestly.
3. Consolidate the Electron library command/event spine.
4. Implement source-root admission and default music source discovery on top of the spine.
5. Introduce resource-plane helpers only when a real resource payload requires them.

The spine should happen before source-root admission/default discovery because source admission adds new command and
event surface. Adding that surface before the spine increases IPC entropy.

The spine should not block small already-green product repairs, but it should block larger new boundary surfaces.

## First spine implementation slice

The first implementation slice should be structural, not semantic.

In scope:

* central channel owner for library IPC;
* plane-separated channel constants;
* one explicit library command registry;
* typed command handler wrapper;
* product result versus host failure separation;
* renderer boundary client normalization;
* preload listener cleanup API;
* event emit helpers;
* generated contract type imports;
* validation hooks;
* tests and source guards.

Out of scope:

* source-root admission;
* default music source discovery;
* browse-policy omission metadata;
* playable-media policy;
* source scan behavior changes;
* tree behavior changes;
* contents row fields;
* waveform/resource payload implementation;
* Exclave integration;
* new schema generation system;
* broad preload rewrite;
* app shell redesign.

The first slice may move existing handlers into the spine, but it must not change product behavior unless a behavior is
already a boundary bug.

## File ownership target

Use existing files if they already own these roles. Do not create parallel paths.

Target ownership shape:

### Channel owner

Owns:

* ControlPlane constants;
* PublicationPlane constants;
* future ResourcePlane constants;
* channel naming convention;
* no duplicate channel strings.

Does not own:

* handler logic;
* emit logic;
* product behavior.

### Command registry

Owns:

* registering all library control-plane commands;
* explicit allowlist;
* dependency injection through function signature;
* request decode hook;
* service call handoff;
* product result normalization;
* unexpected exception normalization;
* host failure logging;
* response validation hook.

Does not own:

* source admission policy;
* scan lifecycle;
* selection behavior;
* contents filtering;
* renderer projection.

### Event emit helpers

Own:

* publication event send calls;
* publication channel constants;
* event payload typing;
* optional event validation;
* delivery target policy.

Do not own:

* event meaning;
* refresh scheduling;
* renderer invalidation policy;
* scan state authority.

### Preload boundary

Owns:

* safe command invocation wrappers;
* safe listener registration;
* cleanup-returning listener functions;
* no raw ipcRenderer exposure.

Does not own:

* renderer feature policy;
* library state;
* selection;
* contents projection.

### Renderer boundary client

Owns:

* feature-facing typed command API;
* transport failure normalization;
* product result forwarding;
* event payload ingress normalization.

Does not own:

* source facts;
* contents filtering;
* scan meaning;
* authority decisions.

## Result model

The spine must distinguish three operational outcomes.

### Product success

The service ran and returned a value.

Example:

* contents read succeeded;
* source registered;
* scan start accepted.

### Product failure

The service ran and returned a domain failure.

Example:

* source already exists;
* invalid cursor;
* policy conflict;
* source not found;
* scan already running.

Product failures are not thrown as transport errors. Renderer feature code handles them as normal product outcomes.

### Host or transport failure

The boundary failed.

Example:

* handler crashed;
* channel missing;
* preload API missing;
* validation failed;
* serialization failed;
* invoke rejected before a product result existed.

Host and transport failures are not LibraryError values.

Renderer feature code must not confuse these with product errors.

## Runtime validation policy

The generated boundary contract package is the source of compile-time types.

If generated runtime validators already exist, the spine uses them.

If generated runtime validators do not exist, the spine still introduces named validation hooks and documents the gap.

Do not invent a parallel local schema system in the first spine slice.

Required validation hook names should make direction clear:

* decode command request at main ingress;
* validate command response at main egress;
* decode command response at renderer boundary ingress;
* decode publication event at renderer boundary ingress.

Validation failure becomes host failure, not product failure.

## Plane enforcement

Plane membership decides the Electron primitive.

ControlPlane:

* may use invoke and handle;
* must not use send and on as the primary command path.

PublicationPlane:

* may use send and on;
* listener APIs must return cleanup;
* must not use invoke and handle as the primary event path.

ResourcePlane:

* reserved for future handle-oriented payload access;
* must not be prematurely modeled as a generic event stream;
* must not dump large payloads into command replies.

A channel constant must not belong to more than one plane.

## Source guards

The implementation must include source guards or equivalent tests for these conditions:

* no raw library ipcMain.handle outside the command registry;
* no raw library webContents.send outside publication emit helpers;
* no raw ipcRenderer.on exposed to renderer feature code;
* no library channel string literals outside the channel owner and tests;
* no PublicationPlane channel used with invoke or handle;
* no ControlPlane channel used with send or on;
* generated contract types are imported rather than redeclared locally;
* command registry does not contain product policy;
* event emit helpers do not contain renderer refresh policy.

These guards are allowed to be pragmatic source-search tests if TypeScript cannot enforce the rule cleanly yet.

## Acceptance bar

The first spine slice is accepted only if:

* all existing library commands remain callable;
* all existing library publication events still reach the renderer;
* product errors are returned as product failures;
* unexpected handler exceptions become host failures;
* renderer invoke rejection becomes host failure;
* listener cleanup removes exactly the registered listener;
* raw library IPC strings are centralized;
* command and publication channels are separated by plane;
* no product behavior moved into the registry;
* no generated boundary types are duplicated;
* validation hooks exist even if runtime validators are not yet generated;
* targeted tests pass;
* full verification passes.

Manual smoke must confirm:

* add source still works;
* scan start still works;
* scan events still update visible state;
* tree expansion still works;
* contents reads still work;
* remove source still works;
* no renderer listener leak is visible through repeated mount/unmount or app navigation;
* no product error appears as an unhandled exception in renderer logs.

## Rejection cases

Reject the implementation if any of these are true:

* command registry becomes a product service;
* source admission is implemented inside the spine slice;
* scan lifecycle policy moves into command registration;
* renderer selection behavior changes accidentally;
* publication events are routed through invoke;
* commands are routed through event listeners;
* raw library IPC strings remain scattered;
* feature code catches raw Electron invoke errors directly;
* product errors are thrown instead of returned;
* host failures are encoded as LibraryError;
* listener APIs do not return cleanup;
* runtime validation is spread into feature components;
* local request/response types duplicate generated contract types;
* ResourcePlane is implemented prematurely without a real resource payload.

## Naming guidance

Use product and plane names that describe ownership.

Good names:

* ControlPlane
* PublicationPlane
* ResourcePlane
* registerLibraryCommands
* emitLibraryBrowserInvalidation
* emitLibraryScanEvent
* createLibraryCommandHandler
* normalizeHostInvokeFailure

Avoid names that imply too much:

* ipcRouter
* messageBus
* eventHub
* commandFramework
* libraryCore
* boundaryMagic
* universalHandler

The spine is explicit infrastructure, not a framework.

## Long-term direction

The Electron boundary spine is the bridge between today’s Dekzer desktop app and the future Exclave-compatible
architecture.

It should make future work easier without forcing premature integration.

Future work should be able to add:

* source-root admission commands;
* default music source discovery commands;
* waveform resource handles;
* artwork resource handles;
* analysis blob resource handles;
* warm runtime publication events;
* hot realtime bindings;
* stricter generated runtime validators;

without inventing new IPC patterns.

The long-term target is not fewer boundaries.

The target is boundaries that are explicit, typed, testable, and owned.

## Summary

Dekzer’s Electron boundary spine exists to prevent local-first desktop complexity from becoming hidden renderer
authority.

The spine must make these facts structurally true:

* commands are commands;
* publications are publications;
* resources are resources;
* product errors are not host failures;
* host failures are not product errors;
* generated contracts are the type source;
* listener cleanup is mandatory;
* channel ownership is centralized;
* plane membership determines transport primitive;
* product authority stays outside the IPC spine.

This is core architecture.
