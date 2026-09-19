# OpenStreetMap ingestion

Phase 7 implements the accepted source boundary from ADR 0008:

```text
.osm.pbf
  -> two-pass extraction
NormalizedOsmDataset artifact
  -> delivery_motorcycle_v1 + ng_v1
FrozenGraph artifact + build manifest
```

The implementation lives in `roadrunner-osm`, keeping source parsing and policy
out of `roadrunner-core`. The CLI exposes the two boundaries separately:

```bash
cargo run -p roadrunner-cli -- osm extract \
  input.osm.pbf output.rr-osm \
  --source-id dataset-name-and-version

cargo run -p roadrunner-cli -- osm compile \
  output.rr-osm output.rr-graph manifest.json
```

## Normalized dataset contract

The extractor first reads candidate ways and restriction relations, then reads
the PBF again to retain only referenced node coordinates. Its independent
artifact envelope contains a schema version, canonical JSON payload, and SHA-256
integrity value. It records a caller-declared source identity, exact PBF hash and
size, normalization version, ordered node references, selected structured tags,
unsupported routing-relevant values, relation members, and topology split
points. Roadrunner routing IDs are not assigned at this stage.

Canonical source coordinates must be exactly representable as WGS 84 E7. An
input containing finer precision is rejected instead of rounded. Published OSM
identifiers must be positive. Missing referenced coordinates, duplicate source
objects, non-canonical artifacts, and integrity failures are rejected.

Candidate ways are those with `highway=*` and at least two node references. The
structured v1 subset is:

- `highway`, `service`, and `junction`;
- `oneway` and `oneway:motorcycle`;
- `access`, `vehicle`, `motor_vehicle`, `motorcycle`, and directional motorcycle access;
- `maxspeed`, motorcycle-specific speed, and directional speed variants.

Conditional access/speed values and selected physical attributes such as
surface, track type, width, bridge, tunnel, ford, lanes, and toll are retained as
unsupported values. They do not silently change v1 routing.

Restriction relations retain ordered members, roles, `restriction` or
`restriction:motorcycle`, `except`, and unsupported values. Node-via members are
split points. No restriction is enforced in Phase 7.

## `delivery_motorcycle_v1` and `ng_v1`

Compilation contracts ways at endpoints, shared source nodes, and restriction
via nodes. Dense graph IDs follow stable OSM way/node ordering. Static oneway
rules become directed-edge existence. `oneway:motorcycle` overrides general
`oneway`; roundabouts and motorways default to forward-only when no explicit
tag overrides them.

The initial profile supports motorway through residential/service/track road
classes with conservative deterministic free-flow defaults. Normally unsupported
classes become routable only with explicit motorcycle permission. Numeric km/h,
`km/h`, `kph`, and `mph` speed values are parsed; symbolic or compound values
remain observable but fall back to the class default. Effective speed is capped
by a parsed legal limit.

| `highway` value | Default km/h |
| --- | ---: |
| `motorway` | 80 |
| `motorway_link`, `trunk` | 60 |
| `trunk_link`, `primary` | 50 |
| `primary_link`, `secondary` | 45 |
| `secondary_link`, `tertiary` | 40 |
| `tertiary_link`, `unclassified`, `road` | 35 |
| `residential` | 30 |
| `service`, `track` | 20 |
| `living_street` | 15 |

Directional speed precedence is motorcycle-direction, motorcycle, general
direction, then general `maxspeed`. Access precedence is motorcycle-direction,
motorcycle, motor vehicle, vehicle, then general access.

Static `no`, agricultural, and forestry access excludes a traversal. Private,
destination, delivery, customer, permit, and unrecognized explicit access
compile as `Contextual`, leaving request authorization to traversal evaluation.
Absent access and explicit yes/permissive/designated/official values compile as
general access.

These tables are intentionally narrow and versioned. Changes require a new
profile or jurisdiction policy identifier when graph semantics change.

## Reproducibility and diagnostics

The build manifest records source and normalized hashes, compiler and policy
versions, build configuration, graph artifact hash, graph counts, weak-component
sizes, and the explicit `turn_restrictions_enforced: false` boundary. All weak
components are retained. The first real fixture and its complete provenance are
documented under `data/fixtures/phase-7/`.

Phase 7 does not include maneuver-aware state, turn-restriction enforcement,
reference-engine route comparison, topology repair, conditional restrictions,
or full OSM tag coverage. Those remain Phase 7.5 or later work.
