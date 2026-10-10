# ADR 0001: Graph Lifecycle and Snapshot Identity

Status: Accepted

## Context

The Phase 1–6 graph is a mutable collection of hash maps. That representation is
useful for construction, but it is too allocation-heavy and weakly scoped for a
published road graph containing millions of elements. Dense identifiers also
cannot be treated as durable identities across rebuilds.

## Decision

Roadrunner separates graph construction from routing:

```text
mutable GraphBuilder
        -> validate and deterministically finalize
FrozenGraph(GraphSnapshotId)
```

`FrozenGraph` owns contiguous, indexable routing storage and is immutable after
publication. Topology changes create a new snapshot. `NodeId`, `RoadSegmentId`,
and `EdgeId` are dense identities scoped to one snapshot.

An object already tied to a `FrozenGraph` may use bare dense IDs. Every durable,
cached, serialized, or otherwise detached reference carries `GraphSnapshotId`.
Cross-snapshot correlation uses separate provenance or stable source keys.

`GraphSnapshotId`, source identity, build version, and integrity hash remain
distinct concepts until their generation rules are deliberately specified.

## Consequences

- Routing state can use indexed vectors rather than hash maps.
- Graph snapshots can be shared safely by concurrent readers.
- Persisted routes, overlays, simulations, replay, and visualization requests
  cannot silently apply old dense IDs to a new graph.
- Live topology mutation is replaced by snapshot rebuild and publication.
- Old snapshots must remain addressable when historical replay requires them.

## Rejected / deferred alternatives

- A long-lived mutable hash-map graph is rejected.
- Raw OSM IDs as routing identities are rejected.
- Globally stable dense IDs across rebuilds are rejected.
- The exact CSR layout and `GraphSnapshotId` generation are deferred.

## Phase 7 remediation amendment (2026-09-21)

The authoritative durable identity is now a full SHA-256
`GraphSnapshotDigest` over declared semantic metadata and canonical graph and
provenance semantics. The 64-bit ID is derived convenience only. Provenance,
manifests, durable routes, replay, and benchmark evidence bind to the full
digest; compact-ID resolution must also compare that digest.

## Relationship to other ADRs

ADR 0002 defines snapshot-owned segments and edges. ADR 0005 defines
deterministic finalization. ADR 0007 defines validated snapshot loading.

## Pre-Phase-20 implementation amendment (2026-10-10)

Graph artifact schema 4 and OSM compiler v4 now compute and verify the full
canonical compiled semantic identity, including required provenance and maneuver
restrictions. Builder-supplied IDs/digests are assertions, never publication authority.
Immutable modifiers reseal; the compact ID is derived from the full SHA-256 digest.
The self identity is excluded from hashing. Provenance has a separate canonical
binding with its self graph-digest excluded. Bundle loading verifies that binding.
Schema 3 claims remain inspection-only/unverified and cannot become operational
routing snapshots by silent relabeling. Historical fixtures/evidence remain intact;
explicit semantic-v2 fixtures record verified content correspondence separately.
See [ADR 0018](0018-planning-time-and-historical-provenance.md) and the
[implemented identity contract](../runtime-boundary.md).
