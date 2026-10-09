# Dekzer legacy documentation

**Development concluded on 9 October 2026.** Read the [closure and handoff](legacy-closure.md)
before the historical V0 product, architecture, and roadmap documents below.

Start with the [root README](../README.md) for what Dekzer is and how to run it.

## Documents

- [legacy-closure.md](legacy-closure.md) records the project conclusion, reusable lessons,
  known limitations, and archival handoff.
- [product.md](product.md) covers what V0 does, the principles that decide current work, and what is deliberately not
  being built.
- [architecture.md](architecture.md) covers how the Vue, Electron, and Rust layers fit together and who owns what.
- [domain-model.md](domain-model.md) covers how a file becomes evidence, and why identity, evidence, and decisions stay
  separate. Read it before touching schema or identity code.
- [library-browser.md](library-browser.md) covers readiness, coverage, selection, and refresh, and the rules that keep
  the browser from claiming more than it knows.
- [roadmap.md](roadmap.md) covers what works now, what comes next, what is blocked, and what is deferred on purpose.
- [development.md](development.md) covers requirements, commands, verification, and storage reset.
- [vision/spatial-performance-memory.md](vision/spatial-performance-memory.md) describes the long-term product thesis.
  Future direction, none of it implemented.

## Decisions

Records for durable choices that are non-obvious from the code and would otherwise be reversed by accident.

- [0001 Rust owns the generated boundary contract](decisions/0001-rust-owned-boundary-contract.md)
- [0002 Evidence and user decisions are separate records](decisions/0002-evidence-separated-from-decisions.md)
- [0003 Background work is bounded-count maintenance with durable provenance](decisions/0003-bounded-work-items.md)
- [Code naming discipline](decisions/code-naming-discipline.md)

## Elsewhere

Schema truth is in `crates/library-store-sqlite/migrations/`. Protocol truth is in
`crates/library-boundary-protocol/`. Behaviour is in the test suites. Development history is in Git.
