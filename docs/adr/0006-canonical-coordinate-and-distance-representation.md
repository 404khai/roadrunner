# ADR 0006: Canonical Coordinates and Segment Distance

Status: Accepted

## Context

Raw `f64` coordinates are convenient for calculations but are ambiguous as the
canonical representation of millions of geometry points. Contracted geometry
also means endpoint Haversine distance is not the physical road length.

## Decision

Canonical source and graph geometry uses fixed-point WGS 84 coordinates with a
schema-declared scale. The initial standard-OSM contract is E7 signed integers
when Phase 7 confirms exact source representability. Unsupported finer precision
is detected and rejected rather than silently rounded.

Validated floating-point `Coordinate` values remain the public/calculation
boundary. Haversine and accumulated routing values use validated `f64`.

`RoadSegment.distance` is derived deterministically after geometry
canonicalization:

```text
sum Haversine(point[i], point[i + 1])
```

Callers cannot supply an independent frozen-graph distance. Reciprocal directed
edges share segment distance. The compiler validates the endpoint-geodesic lower
bound using one documented tolerance policy. Zero-length segments are classified
explicitly and never merged merely by coordinate equality.

## Consequences

- Geometry has exact canonical equality and compact storage.
- Stored distance can be reproduced from the artifact.
- Distance-model or coordinate-scale changes are versioned semantic changes.
- Spatial indexes may derive other representations without becoming authoritative.

## Rejected / deferred alternatives

- Canonical raw `f64` geometry is rejected.
- Endpoint-only distance for curved segments is rejected.
- Caller-supplied physical distance is rejected.
- Projected coordinates as authoritative graph geometry are rejected.
- Compressing cost and time values to `f32` is deferred pending measurements.

## Relationship to other ADRs

ADR 0002 owns geometry through `RoadSegment`. ADR 0004 uses the distance
invariant. ADR 0007 verifies stored numeric structure.
