# ADR 0005: Deterministic Graph Compilation

Status: Accepted

## Context

Dense IDs and equal-cost path tie-breaking become unstable if compilation depends
on hash iteration, parser order, worker scheduling, or filesystem order.

## Decision

Identical declared semantic inputs produce identical canonical graph contents,
dense IDs, geometry order, adjacency order, and routing attributes.

Declared inputs include source content/identity, compiler and schema versions,
normalization version, routing profile, jurisdiction policy, and build
configuration. Finalization assigns IDs from documented stable ordering keys and
sorts outgoing adjacency once. Parallel work must merge deterministically.

Canonical payloads exclude incidental metadata such as timestamps, PIDs, and
temporary paths. A build manifest records semantic inputs, graph statistics, and
integrity hashes. Snapshot identity, source identity, and hashes remain distinct.

All components are retained by default. Connectivity statistics are computed
deterministically; pruning is an explicit build option that changes the artifact.

## Consequences

## Phase 7 remediation amendment (2026-09-21)

Determinism covers canonical normalized bytes, graph bytes, provenance,
manifests, dense IDs, adjacency, geometry, routes, and the full semantic
snapshot digest. Semantic build configuration is identity-bearing; worker
count, temporary paths, timing, and other execution configuration are not.

- Rebuilds are reproducible and benchmark/regression diffs are attributable.
- Equal-cost tie-breaking can safely use deterministic dense IDs.
- Offline sorting and canonicalization consume additional build time and memory.
- Changes to declared semantic inputs may legitimately reassign every dense ID.

## Rejected / deferred alternatives

- Encounter-order ID assignment is rejected.
- Semantic equivalence without canonical ID/byte stability is rejected.
- Stable dense IDs across changed inputs are not promised.

## Relationship to other ADRs

ADR 0001 defines snapshot-local identity. ADR 0007 defines canonical artifact
validation. ADR 0008 applies determinism to source extraction.

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
