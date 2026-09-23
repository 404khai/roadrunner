# Phase 10 traffic scenario

`lagos-marina-severe.json` is a synthetic, deterministic traffic overlay for
`data/fixtures/phase-7/lagos-marina.osm.pbf`. Compile that PBF with source ID
`phase10-lagos-marina` to obtain the graph snapshot digest pinned in the scenario.
The file assigns `severe` congestion only to directed edge 91. The reverse edge,
other edges, and other graph snapshots are unaffected.

See [traffic documentation](../../../docs/traffic.md) for reproduction commands,
selection results, and the scenario schema.
