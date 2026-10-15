# ETA dataset evidence

[Phase 23 guide](../../docs/eta-dataset.md) describes schema, lifecycle, point-in-time
availability, source reproduction and exact split contracts.

`phase23-representative-v1/` is an actual generated, verified synthetic evidence bundle,
not hand-authored labels. Sources are a completed scripted road-delay run, its truncated
horizon descendant and a deterministic replay under separate run identity. All share
ancestry, graph and a protected dependency group. Physical labels and independent
predictive holdout evaluation are unavailable.

Use `roadrunner dataset verify <directory>` and
`python3 scripts/verify_eta_dataset.py <directory>` to validate retained evidence.
The request is `data/fixtures/phase-23/request.json`. Rebuild into a new destination;
existing published versions are immutable. Rebuilding under changed source/code/toolchain
produces a different content identity even when label values coincide.

Only public synthetic data is authorized here. Do not add credentials, private source
records or operational trajectories. Retain source bytes for as long as this dataset's
reconstruction claims are required; removal must explicitly narrow those claims.
