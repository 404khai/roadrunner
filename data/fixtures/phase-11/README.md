# Phase 11 time-dependent traffic scenario

`lagos-marina-profile.semantic-v2.json` is a synthetic, deterministic directed-edge profile
for the committed Phase 7 Lagos Marina PBF. Its graph digest requires source ID
`phase11-lagos-marina`. See [time-dependent routing](../../../docs/time-dependent-routing.md)
for profile semantics and reproduction commands.

The original `.json` fixture is preserved as historical evidence under the older
compiler/graph identity contract. Its digest is not relabeled or accepted against
new graphs. The `.semantic-v2.json` fixture uses verified schema 4 graph identity;
[explicit correspondence](../../../docs/evidence/pre20-graph-correspondence.json)
records identical topology/geometry/traversal/maneuver content and the changed
compiler-identity metadata. Use the current fixture for the documented commands.

## Published snapshot portability

`snapshot.semantic-v2/` retains the exact schema 4 graph, compiled provenance and
build manifest paired with the overlay. Loading verifies semantic identity and
artifact integrity; the CLI tests also perform deep graph verification. Native
floating-point Haversine results can vary across platforms, so rebuilding from the
same PBF/source ID need not reproduce every compiled bit or the pinned digest.
A new build requires its own explicitly authored overlay; this fixture is not rebound.
