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
