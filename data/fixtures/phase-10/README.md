# Phase 10 traffic scenario

`lagos-marina-severe.semantic-v2.json` is a synthetic, deterministic traffic overlay for
`data/fixtures/phase-7/lagos-marina.osm.pbf`. Compile that PBF with source ID
`phase10-lagos-marina` with the current compiler v4 to obtain the semantic digest pinned in this scenario.
The file assigns `severe` congestion only to directed edge 91. The reverse edge,
other edges, and other graph snapshots are unaffected.

See [traffic documentation](../../../docs/traffic.md) for reproduction commands,
selection results, and the scenario schema.

The original `.json` fixture is preserved as historical evidence under the older
compiler/graph identity contract. Its digest is not relabeled or accepted against
new graphs. The `.semantic-v2.json` fixture uses verified schema 4 graph identity;
[explicit correspondence](../../../docs/evidence/pre20-graph-correspondence.json)
records identical topology/geometry/traversal/maneuver content and the changed
compiler-identity metadata. Use the current fixture for the documented commands.
