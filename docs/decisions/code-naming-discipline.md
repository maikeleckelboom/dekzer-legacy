# Code naming discipline

Status: Accepted
Date: 2026-06-13

Folder context carries domain meaning, so symbol names carry role rather than full path. This is the one naming
convention that spans every module and therefore cannot live next to any single implementation.

1. Do not repeat the folder name in every exported symbol.
2. Use `kind` only for discriminated union tags.
3. Prefer precise nouns: `state`, `status`, `role`, `phase`, `profile`, `scope`, `operation`, `action`.
4. Avoid a generic controller name when a narrower owner exists, without swinging to an absurdly long one.
5. Prefer `createController` and `useRead` inside focused folders.
6. Avoid brand prefixes and generic UI prefixes unless an external API requires them.
7. Do not keep compatibility aliases after a rename unless an external API requires them.

Rust files use `snake_case`. TypeScript and Vue files use `camelCase`. Documentation filenames use `kebab-case`. SQL
tables and durable enum values use `snake_case` and full durable vocabulary.

Generated and protocol names change at their source contract and are regenerated, never edited in the generated
output. See [the boundary contract decision](0001-rust-owned-boundary-contract.md).
