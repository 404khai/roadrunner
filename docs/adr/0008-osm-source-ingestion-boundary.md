# ADR 0008: OSM Source Ingestion Boundary

Status: Accepted

## Context

Feeding decoded PBF objects directly into a graph builder couples parsing,
source normalization, profile policy, contraction, and dense-ID assignment. It
also pressures a one-pass parser to retain excessive node state.

## Decision

Phase 7 introduces a deterministic, versioned, profile-independent routing-source
artifact:

```text
.osm.pbf
    -> staged extraction
NormalizedOsmDataset
    -> delivery_motorcycle_v1 + jurisdiction policy
GraphBuilder
    -> FrozenGraph
```

The dataset is not a full OSM mirror and not a compiled vehicle graph. It retains
potentially routing-relevant ways, required source nodes and coordinates, ordered
references, relevant relations, structured source values, selected unsupported
raw values, and provenance. It assigns no Roadrunner routing IDs.

Extraction may use multiple source passes, bounded temporary files, or external
sorting. Its canonical artifact has an independent schema, normalization version,
source identity, and integrity hash.

The first compiled profile is `delivery_motorcycle_v1` combined with a separately
versioned jurisdiction/default-access policy (`ng_v1` initially). Static
directionality compiles into directed-edge existence. Contextual access is
normalized for request-time evaluation. Source identity, not geometry, defines
topology.

Phase 7 preserves turn-restriction members and split points but declares that
restrictions are not enforced. Phase 7.5 adds maneuver-aware state and an initial
motorcycle-relevant node-via `no_*`/`only_*` subset before serious reference-route
validation.

## Consequences

- Parsing, profile compilation, and routing can be tested independently.
- Additional profiles can reuse source extraction.
- Unsupported tags and restrictions remain observable.
- Phase 7 has a larger but cleaner artifact pipeline.

## Rejected / deferred alternatives

- Direct PBF-to-`GraphBuilder` ingestion is rejected.
- A vague universal "drivable" profile is rejected.
- Full OSM tag coverage, topology repair, conditional restrictions, and complete
  maneuver enforcement are deferred.

## Relationship to other ADRs

ADR 0002 defines normalization output. ADR 0005 governs reproducibility. ADR 0006
defines source-coordinate canonicalization. Phase 7.5 follows this ADR.
