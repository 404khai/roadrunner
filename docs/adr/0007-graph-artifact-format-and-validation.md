# ADR 0007: Graph Artifact Validation Boundary

Status: Accepted

## Context

Graph validity is relational. Successful deserialization alone cannot establish
safe offsets, references, canonical ordering, geometry ranges, or agreement with
the build manifest.

## Decision

`FrozenGraph` cannot be directly deserialized or publicly constructed. Only
`GraphBuilder::finalize` and a validating artifact loader may create it:

```text
artifact bytes -> UnvalidatedGraphArtifact -> validate -> FrozenGraph
```

Mandatory loading validates framing, schema compatibility, integrity, integer
ranges, dense identity, adjacency offsets, segment/edge/geometry references,
orientation, numeric invariants, canonical ordering, capabilities, and
manifest/payload agreement. Unknown schemas and noncanonical artifacts are
rejected.

Expensive recomputation such as all component analysis or every geometry distance
belongs to optional deep verification when integrity plus structural validation
already protects normal loading. Artifact publication uses temporary output and
atomic publication so partial files never appear valid.

## Consequences

## Phase 7 remediation amendment (2026-09-21)

The publication unit is a canonical graph/provenance/manifest snapshot bundle.
Every load validates framing, domain values, unique traversals, contiguous
geometry, canonical CSR adjacency, manifest agreement, provenance ranges and
ordering, hashes, and snapshot binding. Publication additionally runs the deep
verifier and atomically renames the complete directory.

- Routing code can trust `FrozenGraph` invariants in its hot path.
- Corrupt, truncated, or incompatible artifacts fail explicitly.
- Loading has validation cost that must be benchmarked.
- Decoder and validator fuzzing becomes valuable later.

## Rejected / deferred alternatives

- Deriving public deserialization directly on `FrozenGraph` is rejected.
- Repairing or canonicalizing malformed artifacts during load is rejected.
- mmap, zero-copy access, signed artifacts, and unsafe fast paths are deferred.

## Relationship to other ADRs

ADR 0005 defines canonical output. ADR 0001 defines the trusted snapshot.
ADR 0010 defines load/validation benchmarks.
