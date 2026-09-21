# ADR 0002: Road Segments, Directed Edges, and Geometry

Status: Accepted

## Context

OSM shape nodes describe geometry, not necessarily routing decisions. Storing
every shape point as a graph node inflates search state, while storing a geometry
vector on each directed edge duplicates bidirectional roads.

## Decision

Routing topology, physical road segments, geometry, and provenance are distinct.

- Routing nodes exist at intersections, branches, routable endpoints, barriers,
  restriction `via` points, graph boundaries, and routing-attribute transitions.
- Intermediate non-decision shape nodes are contracted.
- `RoadSegment` represents a physical normalized corridor between routing nodes.
- `DirectedEdge` represents one potentially permitted traversal of a segment.
- Geometry is stored once in a snapshot-owned contiguous point pool. A segment
  references a range; an edge references the segment plus orientation.
- Bidirectional roads normally compile to two directed edges. Static supported
  directionality is represented by edge existence, not a `one_way` flag.
- Directional access, speed, time, and traffic may differ between reciprocal
  edges.

Source identity determines connectivity. Equal coordinates and geometric line
crossings never create connections by themselves. Coordinate-based repair is a
separate, versioned future stage.

All routable connected components are retained by default. Optional pruning must
be explicit, deterministic, versioned, and reported.

## Consequences

## Phase 7 remediation amendment (2026-09-21)

OSM compilation quarantines ordinary zero-distance physical candidates and
validates canonical endpoint orientation, positive geometry-derived distance,
the endpoint-geodesic lower bound, and reciprocal traversal consistency.
Routing identity remains source-topological: coordinate equality never merges
distinct OSM nodes.

- Search traverses routing choices rather than drawing points.
- GeoJSON can reproduce full geometry without polluting the hot path.
- Alternative-route comparison can distinguish physical, directional, and
  geometric overlap.
- Segment incidents and directional traffic have appropriate identity scopes.
- Roundabouts compile as directed cycles split at relevant connections.

## Rejected / deferred alternatives

- One routing node per OSM shape node is rejected.
- Geometry owned independently by every edge is rejected.
- Connectivity inferred from coordinate equality is rejected.
- Turn-expanded graphs and topology repair are deferred.

## Relationship to other ADRs

ADR 0001 scopes all identities to a snapshot. ADR 0006 defines canonical
geometry and distance. ADR 0008 defines source-topology extraction.
