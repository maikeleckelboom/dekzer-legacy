---
status: candidate
last-reviewed: 2026-06-13
owner: product-architecture
purpose: Guardrails for concise, context-owned code names.
---

# Code Naming Discipline

Rules:

1. Folder context carries domain meaning.
2. Symbol names carry role, not full path.
3. Do not repeat the folder name in every exported symbol.
4. Use `kind` only for discriminated union tags.
5. Use precise nouns: `state`, `status`, `role`, `phase`, `profile`, `scope`, `operation`, and `action`.
6. Avoid generic controller names when a narrower owner exists, but also avoid absurd long controller names.
7. Prefer `createController` and `useRead` inside focused folders.
8. Avoid brand prefixes and generic UI prefixes unless an external API requires them.
9. Do not keep compatibility aliases after renames unless an external API requires them.
10. Final reports and future prompts must mention naming discipline when adding new modules.

Generated and protocol names should be changed only at their source contract and regenerated.
