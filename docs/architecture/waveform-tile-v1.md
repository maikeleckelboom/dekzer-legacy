# Waveform Tile V1

**Status:** proposal, not implemented
**Baseline:** Dekzer dev branch, user-reported latest commit `fe5468555825c505a5954ae33d7a09a5ba63890e`
**Provenance:** Authored from the waveform/stems R&D thread and follow-up review corrections.
**Review note:** Do not treat this as implementation proof. It is an architecture contract candidate for human review.

## Purpose

Define the proposed production V1 binary tile container for waveform artifact payloads.

Waveform tiles are file-backed binary data. SQLite stores artifact status, ownership, manifest references, and small metadata. SQLite must not store full waveform payloads.

## Tile ownership

A tile belongs to:

- one waveform artifact,
- one basis hash,
- one column schema key,
- one LOD,
- one tile index.

The tile does not contain raw filesystem paths. Paths are resolved by the artifact store, not by renderers.

## Tile column count

Recommended V1 tile size:

    tile_column_count = 4096

Full tile payload sizes:

| Schema        | Column bytes | Full tile payload |
| ------------- | -----------: | ----------------: |
| mix LOD0/1/2  |           32 |     131,072 bytes |
| mix LOD3/4    |           52 |     212,992 bytes |
| stem LOD0/1/2 |           20 |      81,920 bytes |
| stem LOD3/4   |           40 |     163,840 bytes |

These sizes keep normal tile fetches small enough for bounded worker delivery while avoiding excessive file fragmentation.

## Tile header

Header size:

    160 bytes

Payload starts at byte 160.

All numeric fields are little-endian. All multi-byte numeric fields are aligned to 2-byte or 4-byte boundaries. There are no odd-offset i16/u16 fields.

| Offset | Field            | Type   | Bytes | Meaning                                     |
| -----: | ---------------- | ------ | ----: | ------------------------------------------- |
|      0 | magic            | u8[4]  |     4 | ASCII `DZWT`                                |
|      4 | version          | u16    |     2 | Tile format version, V1 = 1                 |
|      6 | header_bytes     | u16    |     2 | Must be 160                                 |
|      8 | artifact_id_hash | u8[32] |    32 | Hash of artifact identity string            |
|     40 | basis_hash       | u8[32] |    32 | Canonical basis hash                        |
|     72 | payload_hash     | u8[32] |    32 | SHA-256 of payload bytes only               |
|    104 | manifest_hash    | u8[32] |    32 | Hash of immutable manifest sidecar          |
|    136 | lod              | u16    |     2 | LOD number 0..4                             |
|    138 | schema_id        | u16    |     2 | Numeric schema id for column schema key     |
|    140 | column_bytes     | u16    |     2 | Column byte width for this LOD/schema       |
|    142 | compression_kind | u16    |     2 | 0 = none for V1                             |
|    144 | bucket_samples   | u32    |     4 | Samples per column at this LOD              |
|    148 | tile_index       | u32    |     4 | Zero-based tile index                       |
|    152 | column_start     | u32    |     4 | First column index represented by this tile |
|    156 | column_count     | u32    |     4 | Number of columns in this tile              |

Payload byte length is derived:

    payload_bytes = column_count * column_bytes

The payload hash covers payload bytes only, not the header.

## Schema ids

V1 schema ids:

| schema_id | schema key                   |
| --------: | ---------------------------- |
|         1 | `mix_spectral_microtrace_v1` |
|         2 | `stem_amp_microtrace_v1`     |

## Compression

V1 uses uncompressed payloads:

    compression_kind = 0

Compression may be added later only if:

- frame-time decompression is not required,
- worker decode remains bounded,
- tile payload hash semantics remain clear,
- random-access performance is measured.

## Manifest relationship

The immutable waveform manifest sidecar owns the logical artifact index:

- artifact id
- basis hash
- target identity
- schema key
- track/component sample count
- LOD summaries
- tile count per LOD
- tile_column_count
- tile store uri or opaque descriptor
- content hashes
- diagnostics

SQLite stores a queryable projection of the manifest and artifact status. It must not duplicate every tile descriptor as competing authority unless cache eviction explicitly needs a small per-tile state table.

## Cache state

Per-tile cache state is optional and exists only for eviction and integrity state:

- ready
- evicted
- missing
- corrupt
- pinned

Do not update `last_accessed_at` in SQLite per render frame. Renderer/backend tile access recency must be tracked in memory and flushed coarsely only when needed for eviction or diagnostics.

## Range calculation

Given a sample window:

    first_column = floor(sample_start / bucket_samples)
    last_column = ceil(sample_end / bucket_samples) - 1

Given a column index:

    tile_index = floor(column_index / tile_column_count)
    column_in_tile = column_index % tile_column_count
    byte_offset_in_payload = column_in_tile * column_bytes

Reads are bounded by:

- max total bytes,
- max total tiles,
- allowed artifact id/basis hash,
- visible or prefetched surface reason.

## Failure handling

Tile read may return missing ranges instead of throwing when a tile is not currently available:

- `not_generated`
- `evicted`
- `corrupt`
- `over_budget`
- `basis_mismatch`
- `artifact_not_ready`

Renderer behavior for missing tiles is defined in waveform rendering and delivery docs. The frame path must never wait for a missing tile.

Corruption recovery is tile-scoped by default. A corrupt tile invalidates that tile's cache/state and may trigger tile
regeneration or refetch. It must not invalidate the whole artifact unless manifest hashes, payload hashes, or broader
artifact integrity checks prove damage beyond the single tile.

Descriptor and token validation for tile reads is owned by `waveform-delivery-v1.md`. Tile headers validate artifact,
basis, manifest, LOD, schema, and payload integrity; delivery descriptors validate whether the renderer is allowed to
request the tile at all.

## Acceptance criteria

- Payload begins at byte 160.
- Header fields are alignment-safe.
- Payload byte length equals column_count × column_bytes.
- Payload hash covers payload only.
- Header contains no raw filesystem path.
- Tile lookup is through manifest/artifact store, not arbitrary renderer file access.
- LOD4 tiles are cache/on-demand by default.
