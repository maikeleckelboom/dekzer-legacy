# Rust owns the generated boundary contract

Status: Accepted
Date: 2026-08-05

## Context

Dekzer's durable state lives in Rust and SQLite. Its interface lives in TypeScript and Vue. Every command, reply,
event, and error that can reach a user crosses that line, and both sides have to agree on the shape of all of it.

Two languages describing the same protocol independently drift. Not dramatically, and not at first. A field becomes
optional on one side, an enum gains a variant the other does not handle, a numeric identifier is widened past what a
JavaScript number represents safely. Each of these compiles cleanly on both sides. They surface as a malformed row, a
projection that silently drops a state, or a runtime error in the renderer with no useful stack.

The problem is worse than ordinary API drift because the failure is asymmetric. Rust tests pass. TypeScript type
checking passes. Nothing fails until the two processes actually talk to each other with the specific value that
exposes the gap.

## Decision

The Rust boundary protocol crate is the single source of truth for cross-language shapes. The TypeScript contract
package and its JSON Schema are generated from it and committed to the repository.

`cargo run -p xtask -- export-boundary-contract` and `export-stdio-transport-contract` regenerate the output. The
matching `check-boundary-contract` and `check-stdio-transport-contract` commands compare the current Rust source
against what is committed and fail on any difference.

Both checks run inside `pnpm verify`. A protocol change that does not regenerate the contract fails the build.

The generated package is not hand-edited. Consumers import from it rather than redeclaring shapes locally, and the test
suite includes a source guard against local redeclaration.

The readiness envelope for the stdio transport is exported separately from the main protocol, because process startup
and protocol negotiation are a different concern from the command surface and change on a different schedule.

## Consequences

Protocol changes become a two-step edit: change the Rust source, run export, commit both. This is friction, and it is
the point. The friction lands on the person making the change rather than on whoever finds the mismatch later.

TypeScript cannot describe a shape that Rust does not produce. When the renderer needs a field, that need has to be
expressed in the protocol crate, which puts the decision where the data actually originates.

Generated output is committed rather than built on demand. This makes the diff for a protocol change reviewable, and
lets TypeScript-only work proceed without a Rust build for typechecking, though Cargo is still required for
`pnpm verify` because the contract checks and stdio tests need it.

The check is a byte comparison, so formatting changes in the generator show up as failures. That is a small
maintenance cost paid in exchange for the check being unambiguous.

## Rejected alternatives

**Hand-maintained TypeScript types.** The cheapest option and the one this decision exists to prevent. It relies on
whoever changes the Rust side remembering to update the TypeScript side, and provides no signal when they do not.

**Generating at build time instead of committing.** Removes the two-step edit, but makes protocol changes invisible in
review and requires a working Rust toolchain for any TypeScript typecheck. The reviewability of a committed diff was
worth more than the convenience.

**A neutral schema language such as Protobuf or OpenAPI as the source, generating both sides.** Defensible, and it
would remove Rust's privileged position. Rejected because it introduces a third artifact that neither side's compiler
checks natively, and because Rust already holds the domain vocabulary. Making the language that owns the meaning also
own the wire format keeps the number of places a shape is defined at one.

**Runtime validation instead of generated types.** Not an alternative so much as a different layer. Validation hooks
exist at the boundary and catch what static types cannot, such as malformed JSON arriving from a mismatched binary.
Neither replaces the other.

## Implementation evidence

- Protocol source: `crates/library-boundary-protocol/src/`
- Export and check commands: `crates/xtask/src/commands/`
- Generated output: `packages/library-boundary-contract/index.ts`,
  `packages/library-boundary-contract/boundary-contract.schema.json`,
  `packages/library-boundary-contract/manifest.json`
- Wired into verification: the `library:contract:check` and `library:stdio:contract:check` steps of the root
  `verify` script
