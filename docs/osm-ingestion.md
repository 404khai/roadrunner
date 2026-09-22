# OpenStreetMap ingestion

Phases 7 and 7.5 implement this boundary:

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

## Maneuver-aware routing

The delivery-motorcycle compiler resolves applicable, non-conditional node-via
restrictions with exactly one `from` way, one `via` node, and one `to` way. The
supported values are `no_left_turn`, `no_right_turn`, `no_straight_on`,
`no_u_turn`, `only_left_turn`, `only_right_turn`, and `only_straight_on`.
`restriction:motorcycle` takes precedence over a generic `restriction`; a generic
restriction is ignored when `except` includes `motorcycle`, `motor_vehicle`, or
`vehicle`.

Compilation resolves source members to directed graph edges after one-way and
access policy has been applied. A `no_*` relation forbids its resolved edge pair.
An `only_*` relation forbids every other outgoing edge after the incoming edge.
Dijkstra and A* then search `(node, incoming edge)` states, so a cheaper arrival
that cannot make the next turn does not suppress a legal arrival at the same
node. Route reconstruction follows expanded-state predecessors.

Way-via, conditional, malformed, ambiguous, unresolved, and unsupported vehicle
forms remain preserved with explicit provenance statuses. They are not silently
treated as enforced.

## Snapshot trust boundary

A snapshot directory contains `graph.rr-graph`, `graph.rr-provenance`, and
`manifest.json`. Loading rejects noncanonical bytes, unknown fields, invalid
domains, structural inconsistencies, stale metadata, invalid mappings, and
cross-snapshot substitution. Publication validates and deep-verifies a temporary
bundle before atomically renaming the directory.

The authoritative 256-bit `GraphSnapshotDigest` binds source identity, semantic
versions/configuration, canonical graph semantics, provenance, and capabilities.
The 64-bit snapshot ID is a derived in-process convenience only.

Current compiled snapshots report `turn_restrictions_enforced: true`, including
snapshots that contain no applicable restrictions, because the routing engine
enforces the declared supported subset. Graph and provenance schema versions are
part of snapshot identity; older Phase 7 bundles must be rebuilt.
