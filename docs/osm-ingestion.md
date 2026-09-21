# OpenStreetMap ingestion

Phase 7 implements this boundary:

```text
.osm.pbf
  -> osm_normalization_v2
NormalizedOsmDataset
  -> delivery_motorcycle_v2 + ng_v2
FrozenGraph + source provenance + manifest
```

`roadrunner-osm` owns parsing and policy; `roadrunner-core` only sees the frozen
routing graph. The CLI publishes and validates a snapshot bundle:

```bash
roadrunner osm extract input.osm.pbf output.rr-osm --source-id ID
roadrunner osm compile output.rr-osm output-snapshot
roadrunner graph verify output-snapshot --deep
```

## Routing-source contract

The normalized dataset is profile-independent within Roadrunner's versioned
routing-source schema, not a complete OSM mirror. It preserves ordered source
identities, exact E7 coordinates, supported routing node/way tags, relevant
unsupported values, ferry connectors, and retained generic, vehicle-qualified,
and conditional restriction variants. A vehicle profile never decides what
survives extraction. Barrier, access, ford, restriction-via, intersection,
endpoint, and other semantic boundaries become explicit split reasons.

The extractor uses two passes but retains candidate ways and requested node
state in memory. Measurements are published; Roadrunner does not claim
bounded-memory scaling.

## Motorcycle and Nigeria policy

Access, directionality, physical suitability, and speed are separate decisions.
Reason-specific access classes are preserved. General access is routable;
destination, delivery, customer, private, permit, and unknown-explicit classes
are denied unless a request proves the specific supported authorization. There
is no generic contextual bypass. Endpoint-region semantics for destination,
delivery, and customer access remain unsupported and therefore denied.

Directionality has explicit bidirectional, forward-only, reverse-only,
unsupported-dynamic, contradictory, and unknown-explicit outcomes. Missing
directionality uses the documented default; unsupported explicit semantics never
widen connectivity. `oneway:motorcycle` precedes `oneway`; `oneway=-1` uses
original OSM order. Roundabout and circular junction forms have explicit policy.

Speed is derived through road-class default, parsed legal limit, profile maximum,
and surface/tracktype/smoothness constraints. Common paved surfaces are normal,
`unpaved`/`gravel` cap at 20 km/h, `ground`/`dirt` at 15 km/h, and mud, sand, or
unknown explicit physical values are conservatively excluded. The resulting
`free_flow_travel_time` remains deterministic uncongested time, never live ETA.

## Compilation and provenance

Semantic contraction preserves split points and shape coordinates. Physical
length is the Haversine sum over canonical geometry. Ordinary zero-distance OSM
candidates are quarantined. Geometry endpoints/orientation, endpoint lower
bound, reciprocal traversal consistency, and derived travel times are checked.

Canonical provenance maps retained OSM nodes to dense nodes and OSM ways to
ordered physical segments plus forward/reverse edges and actual compiled
access/speed attributes. It is bound to the exact full semantic snapshot digest.

## Snapshot trust boundary

A snapshot directory contains `graph.rr-graph`, `graph.rr-provenance`, and
`manifest.json`. Loading rejects noncanonical bytes, unknown fields, invalid
domains, structural inconsistencies, stale metadata, invalid mappings, and
cross-snapshot substitution. Publication validates and deep-verifies a temporary
bundle before atomically renaming the directory.

The authoritative 256-bit `GraphSnapshotDigest` binds source identity, semantic
versions/configuration, canonical graph semantics, provenance, and capabilities.
The 64-bit snapshot ID is a derived in-process convenience only.

Phase 7 preserves restrictions and resolution provenance but does not enforce
maneuvers. Phase 7 snapshots report `turn_restrictions_enforced: false`.
