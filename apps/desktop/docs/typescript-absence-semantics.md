# Desktop TypeScript Absence Semantics

Dekzer-owned desktop TypeScript uses `undefined` and omitted optional properties for absence.

Keep `null` at external boundaries where it is part of the contract: generated protocol types,
database-shaped rows, JSON payloads, DOM and Electron APIs, and third-party compatibility. Boundary
mapping code should consume nullable protocol values and normalize them before returning
renderer-facing desktop DTOs.

Tests may use `null` when they model protocol fixtures, generated contracts, DOM/Electron behavior,
or renderer-visible boundary contracts. Generated contracts keep their generated shape.

For app-owned unions, prefer discriminants over paired nullable fields. For example, a hierarchy
directory node owns `sourceDirectoryId`, while a file node owns `sourceFileId`; neither shape carries
the other ID as `null`.
