# Phase 7 real OSM fixture

`lagos-marina.osm.pbf` is a deliberately small, versioned real-road fixture for
the first OSM ingestion and compilation evidence. It is test data, not a claim
that this bounding box represents Lagos or production-scale routing.

## Provenance

- Source: OpenStreetMap API 0.6 `map` response
- Bounding box: `3.3780,6.5230,3.3810,6.5260` (`minlon,minlat,maxlon,maxlat`)
- Retrieved: 2026-09-19
- Request: `https://api.openstreetmap.org/api/0.6/map?bbox=3.3780,6.5230,3.3810,6.5260`
- Source XML SHA-256: `042309ff17a5e838381749358844d9a9274c628e7837357b8714801f6727a6e6`
- PBF conversion: pyosmium 4.3.1 `SimpleWriter`, preserving source elements
- PBF SHA-256: `f1f0f4e7abfcc9396c6b67bbd08b720a0481bf69a636d6c07c50980ea15b8687`
- Data copyright: OpenStreetMap contributors
- License: [Open Data Commons Open Database License 1.0](https://opendatacommons.org/licenses/odbl/1-0/)

The committed PBF is the authoritative test input. Tests do not contact the
OpenStreetMap API.
