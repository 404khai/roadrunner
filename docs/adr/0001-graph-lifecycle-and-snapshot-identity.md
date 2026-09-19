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

## Relationship to other ADRs

ADR 0002 defines snapshot-owned segments and edges. ADR 0005 defines
deterministic finalization. ADR 0007 defines validated snapshot loading.
