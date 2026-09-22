# Phase 8: reference routing comparison

This experiment runs Roadrunner and [OSRM 5.27.1](https://github.com/Project-OSRM/osrm-backend) on the **same committed OSM PBF**, then compares a pinned corpus of source OSM node pairs. The result is a route-validity diagnostic, not a speed benchmark or proof of equivalent routing policy.

## Reproduce

Requirements: Rust toolchain from `rust-toolchain.toml`, Python 3.9+, and a running Docker daemon. The script uses only the Python standard library. From the repository root:

```sh
python3 benchmarks/reference-comparison/run.py \
  --output benchmarks/reference-comparison/results/lagos-marina-osrm-car.json
```

The script validates the graph snapshot, checks the exact PBF hash, builds OSRM's stock `car.lua` profile from the same PBF, runs `osrm-contract` and a short-lived local `osrm-routed` server, queries every pair, and stops the container. It pins the OSRM image by digest. The machine-readable result includes the engine configuration, graph identity, dataset and corpus hashes, per-query status, distance, travel time, OSRM snap offsets, and geometry differences. The existing CLI export can be inspected independently with:

```sh
roadrunner route corpus <snapshot-directory> data/fixtures/phase-7/route-corpus.json
```

The nine cases include short, medium, and longer local routes; same-node and disconnected pairs; one-way forward and reverse traversals; parallel one-way segments; and paved/unpaved paths. Endpoints are OSM node identities resolved through Roadrunner's validated provenance. OSRM receives their WGS 84 coordinates in longitude,latitude order. This avoids selecting unrelated nodes by proximity; the result also records where OSRM snapped each endpoint.

## Metrics and interpretation

- **Routing failure:** Roadrunner `NoRoute` or OSRM `NoRoute` is recorded as unreachable. Unexpected OSRM codes are `reference_error`, not silently counted as unreachable. Status agreement is reported separately from route metrics.
- **Distance and duration:** signed Roadrunner minus OSRM values, in meters and seconds. Medians use absolute differences over pairs reachable in both engines. A difference is diagnostic and is not an error threshold.
- **Geometry:** each full route polyline is sampled at intervals of at most 10 m. For each sample, the distance to the other polyline is measured; the union of both directions yields symmetric mean and maximum deviation. A local planar projection is used because the fixture spans a small area. This measures spatial overlap, not traversal direction, and is sensitive to endpoint snapping. It is not an exact path-identity test.

The result at [`results/lagos-marina-osrm-car.json`](results/lagos-marina-osrm-car.json) reports 9/9 reachability agreements and 7 routes reachable in both engines. Median absolute distance difference is 0.343 m; median absolute duration difference is 5.734 s; median symmetric maximum geometry deviation is 0.065 m. The full per-query values are the evidence for these summaries.

The near-identical geometries in this corpus suggest both engines chose the same local corridors. The time differences are expected because Roadrunner's `delivery_motorcycle_v2` policy and OSRM's stock `car.lua` use different access and speed rules. OSRM's [profile is applied during extraction](https://github.com/Project-OSRM/osrm-backend/blob/master/docs/profiles.md), so the `driving` URL label does not harmonize those rules. OSRM also includes its own turn-cost model. The 0–0.071 m geometry deviations are consistent with snapping and coordinate rounding, though the comparison alone does not prove their exact cause.

This corpus is one tiny Lagos Marina extract (58 Roadrunner nodes, 114 directed edges) and contains no restriction relations. It does not validate Phase 7.5 turn restriction agreement, larger networks, other regions, traffic, or production ETA accuracy. No latency or throughput claims are made from it.

[OSRM route API documentation](https://github.com/Project-OSRM/osrm-backend/blob/master/docs/http.md) defines the returned distance, duration, GeoJSON geometry, and snapped waypoints used here.
