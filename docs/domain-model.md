# Domain model

This explains the conceptual layers Dekzer uses to get from a file on a disk to something a DJ can trust, and what each
layer is and is not allowed to claim.

The schema and the tests own column-level truth. Table names appear here as implementation evidence, not as a second
schema definition.

## The progression

```text
Source                                  implemented
  -> source-file occurrence             implemented
    -> observation                      implemented
      -> exact-byte attachment          implemented
        -> playable media               implemented
          -> media candidate            not implemented
            -> track candidate          implemented for the whole-file media case
              -> user decision          implemented
                -> canonical track      not implemented
                  -> preparation        not implemented
                    -> workflow         future
```

Everything through user decision exists in the current baseline, except for the media-candidate layer. The current
track-candidate implementation covers only the whole-file media case. Canonical track identity, preparation, and
workflow remain unimplemented, and are named here because the layers under them were shaped to leave room for them.
The workflow layer belongs to [the vision](vision/spatial-performance-memory.md).

## Why the layers exist

The whole model follows from one observation: **a filesystem path is an occurrence, not an identity.**

A path records that something was seen at a location at a time. It says nothing durable. Drives get unplugged and
remounted under different letters. Folders get reorganized. The same bytes appear in three places because someone
copied a folder to a backup drive. A file gets replaced in place with a different encode and keeps its name.

If a path is treated as identity, all of that becomes data loss or silent corruption. So Dekzer separates the question
"where was this seen" from "what bytes are these" from "what music is this", and refuses to let an answer to one
migrate into another without a recorded reason.

## The layers

### Source

Where music may live. A registered source has locators, durable state, scan state, and lifecycle. Source locations are
registered subpaths within a source.

Owns: whether a source is known, reachable, scanning, blocked, or missing, and what has been observed under it.

Does not own: the identity of anything inside it.

A source that becomes unreachable stays registered and visible as unavailable. It is not deleted and its contents are
not forgotten. When the path returns, it recovers.

### Source-file occurrence

One observed file at one relative path under one source, with presence, size, and modification time. Presence is
`present`, `missing`, or `removed`.

Owns: durable source-relative inventory and path-derived file kind and class.

Does not own: playability, byte identity, or track identity. Path-derived classification is provisional. A `.wav`
extension is a hint about what a file claims to be, not evidence about what it contains.

### Observation

Evidence about one occurrence, bound to a basis. The basis is the file state the evidence was taken against: source,
relative path, size, modification time, presence.

Observations currently carry a BLAKE3 content hash and audio probe results such as container, codec, duration, sample
rate, channels, and bit depth. Each accepted observation cites the work artifact that produced it, so every piece of
evidence can be traced to the run that generated it and the adapter version that ran.

Owns: what was learned by reading or probing bytes, and under what conditions.

Does not own: product row admission or user-facing identity.

**Staleness is computed, never stored.** There is no `is_stale` flag on an observation or a link. Validity is
determined by comparing current observation state against the recorded basis. A stored flag is a cached answer, and
cached answers go wrong quietly when the thing they describe changes and nothing updates them. See
[the evidence and decisions ADR](decisions/0002-evidence-separated-from-decisions.md).

### Exact-byte attachment

A durable content identity keyed by BLAKE3 hash, with links from the occurrences that carry those bytes. One
occurrence has at most one current attachment link. The link carries interpretation context such as file kind. The
attachment carries only content identity.

Owns: the relation "these occurrences contain identical bytes".

Does not own: track identity, path proximity guesses, or any claim that identical bytes should become one product
track. Two files with the same hash are the same bytes. That is all it means. Whether they should be treated as one
item is a separate question that requires a decision.

When a file's hash changes, the old link is deleted and a new link to the correct attachment is inserted. The old
attachment row survives, because other occurrences may still reference it.

This layer is also what makes duplicate detection, relocation detection, offline occurrences, and backup copies one
substrate rather than four. They are filter views over the same occurrence model: several current occurrences of one
attachment, or one occurrence gone stale while another became current, or a known occurrence whose source is offline.

### Playable media

A promoted record for an attachment that has current byte identity and current audio probe evidence. V0 is audio-only.
The name avoids "audio" so the record can later admit video deliberately rather than by accident.

Owns: evidence-backed playability.

Does not own: canonical track identity or contents authority. Promotion requires both attachment identity and probe
evidence, because a hash alone cannot distinguish a playable file from a corrupt container with the right extension.

### Media candidate (not implemented)

A playable or interpretable unit derived from attachment and probe evidence. This layer does not exist yet, and it is
named here because the layers around it only make sense with the gap visible.

Playable media answers "is this file playable". That is a per-file question, and it holds as long as one file is one
musical unit. Real collections break that assumption in both directions. A single FLAC with a CUE sheet is one file
holding many units, and the CUE is a second file that describes them without containing audio. A multi-disc rip is
many files that belong to one release. A long DJ mix with an index is one file that a user may want to address by
subrange.

None of those can be modelled honestly as ordinary playable-file rows. Forcing them into one row per file either
invents units that no file contains or collapses units that a user needs to address separately.

The future layer would own audio and video file candidates, CUE document candidates, CUE-derived split-track
candidates, and multi-file association candidates, each carrying the evidence and confidence behind it. A CUE-to-audio
association must remain an evidence-backed candidate with provenance and confidence. Filename or path proximity alone
must not make it canonical.

Two ordering consequences follow. Acoustic fingerprinting needs decoded audio, so it runs over media candidates rather
than over raw attachments, and it produces its own evidence rather than extending the attachment layer. And CUE parse
schema should follow an audit of real collection material rather than the specification alone, because the failure
modes in real archives are wrong filename casing, missing referenced files, embedded sheets, hidden pregaps, and
non-standard encodings.

### Track candidate

A grouping of playable media that share exact current content evidence, with full provenance back to the occurrences,
attachments, and observations that produced the grouping.

The current implementation reaches only the whole-file media case. It groups playable media by exact BLAKE3 content
evidence, so a candidate may cover several occurrences or playable-media records that share the same bytes. The
limitation is not how many files a candidate spans, it is that each interpreted musical unit currently corresponds to
an entire file. When the media candidate layer above exists, track candidates would be produced from media candidates
rather than directly from playable media, and the current exact-content grouping becomes one case of that.

Owns: the reversible claim "these look equivalent by exact content evidence".

Does not own: canonical track identity, semantic recording matching, metadata reconciliation, units that a file does
not directly contain, or grouping the renderer may invent.

### User decision

A separate record over a candidate: accepted, rejected, deferred, or superseded, with a recorded source and basis.

System maintenance may accept active exact-content candidates under a system decision source. Explicit user commands
accept, reject, or defer under a user decision source. Precedence is backend-owned: a current user decision beats a
system decision, and a current user reject or defer blocks a system accept from taking effect.

Decision evidence is a snapshot of copied provenance identifiers. It does not cascade from live candidate, source, or
attachment rows, so a decision remains readable as the judgment it was at the time it was made, even after the evidence
underneath changes.

## Evidence and decisions

The single most important distinction in the model, and the one most likely to be eroded by a convenient shortcut.

**Evidence** is what the system observed or inferred. Hashes, probes, imported values from other applications,
analyzer output, and candidate groupings are all evidence, regardless of how confident they are.

**A decision** is what a person or an explicit policy chose. Accept, reject, defer, ignore, override, prefer, pin,
unpin, merge, split.

They never share a row. A decision references the evidence and basis it acted on, carries provenance, and survives
recomputation of the evidence beneath it. When evidence changes under an existing decision, the conflict stays
inspectable until it is explicitly resolved rather than being silently reconciled.

Two rules follow:

**Machine recomputation does not overwrite a user decision.** A rescan that produces new evidence may make a decision
stale. It may not reverse it.

**No automatic cleanup, merge, removal, or forgetting.** Dekzer may surface evidence that the same bytes exist in four
places or that content appears to have moved. It never acts on that. Occurrence views are diagnostic evidence, not
action recommendations, and there is no code path that deletes, merges, or forgets a user's files or records on its own
initiative.

## Identity channels

Three kinds of sameness, deliberately not connected.

| Channel           | Basis                | Status          | Means                                   |
| ----------------- | -------------------- | --------------- | --------------------------------------- |
| Byte identity     | BLAKE3 attachment    | Implemented     | Identical bytes                         |
| Acoustic identity | Fingerprint evidence | Not implemented | Same audio, possibly different encoding |
| Track identity    | Canonical track      | Not implemented | Same musical item                       |

Acoustic fingerprinting, when it exists, is one future input to track identity rather than a channel that resolves it.
It runs over media candidates rather than raw attachments, because it needs decoded audio and because the unit it
should fingerprint is not always a whole file. It produces its own evidence and does not extend the attachment layer.

## Language

Vocabulary is load-bearing here, because the wrong word makes a false claim look like a normal description.

| Do not say                             | Say                                         |
| -------------------------------------- | ------------------------------------------- |
| same track                             | same content, same bytes, same attachment   |
| duplicate song                         | same content evidence in multiple locations |
| duplicate track                        | describe the occurrences                    |
| track moved                            | attachment found at a new source-file path  |
| safe to delete                         | nothing, and never recommend deletion       |
| cleanup, remove duplicates, auto-merge | nothing, these are not system actions       |

## What no current layer may claim

- That a path is a durable identity
- That identical bytes are the same musical item
- That a file extension proves content
- That an imported value from another application is native authority
- That absent rows prove an empty scope, which belongs to
  [the library browser](library-browser.md)
- That any current record is a canonical track

The last one is worth stating plainly. Nothing in the current baseline is a canonical track. Candidates and decisions
are a foundation for that layer, deliberately built to be reversible, and they do not substitute for it.
