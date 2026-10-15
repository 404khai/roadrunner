# Phase 23 ETA dataset

Roadrunner publishes **verified synthetic started-road-leg evidence**. This is an
offline data engineering artifact, not a physical travel-time dataset or an ML model.
The signed [audit](data-ml-readiness-audit.md), [authorization](phase-23-authorization.md)
and [ADR 0020](adr/0020-eta-dataset-evidence-and-synthetic-traces.md) govern the contract.

Build and verify a new version from the root checkout:

```sh
cargo +1.99.0 build -p roadrunner-cli --locked
./target/debug/roadrunner dataset build data/fixtures/phase-23/request.json /tmp/eta-example-v1
./target/debug/roadrunner dataset verify /tmp/eta-example-v1
python3 scripts/verify_eta_dataset.py /tmp/eta-example-v1
```

The build command requires a Git checkout for actual source/configuration capture and
installed `rustc`/`cargo` for version reporting. Run from the root; source scenario paths
are relative to the request document. A source graph artifact path remains relative to
its own scenario document. The library takes already verified graph/scenario values
and has no Git, network, database or wall-clock requirement. Verification of an existing
bundle needs no original file paths or Git checkout, only compatible Rust semantics.
Source reconstruction bytes are included in the bundle; offline Cargo dependency
availability must still be provisioned using the retained lockfile. A vendor archive
of third-party dependencies is not claimed.

A request declares an immutable dataset version, each source's archive-owned run ID,
scenario file, generator/family/base-world/realization/policy ancestry and dependencies,
plus exact run assignments. The sample is [request.json](../data/fixtures/phase-23/request.json).
The exact scenario/graph/result bytes are retained and checked. Every source is replayed
and reconciled before publication, including excluded sources. Corrupt/unverified sources
fail rather than disappear. Changing boundary or evidence produces a new content identity;
use a new version/destination. Overwriting an existing output directory is refused.

`bundle.json` holds the five logical components:

| Component | Fields and contract |
|---|---|
| Source/census | `sources`, `rows[].key`, `rider`; every committed start once |
| Features/B0 | `features`, original route, captured traffic, availability revision |
| Labels | `label.evidence`, primary outcome, optional exact duration or censor bound |
| Lineage/dependencies | source ancestry, effect aliases, complete source schedules, `splits` |
| Manifest/quality | schema/version, retained code/toolchain/patch, quality, evaluation specification |

`manifest.json` separately binds canonical semantic and actual packaging SHA-256.
Core graph decoding verifies schema 4 semantics; byte integrity alone does not confer
modern semantics on legacy artifacts. Source verification requires the original supported
artifact. Standalone feature inspection can report archival loss for optional attributes;
publication cannot assert complete source replay if the required graph is lost.

The unit is a whole started road leg, including zero-distance legs. Identity is
run + operational namespace + ActionId, with RiderId checked. Generations and effect
aliases never add rows. Failed starts, never-started orders and unchosen candidates
are not fabricated traversals. Complete order demand, complete ingress and physical
counterfactual outcomes are not established by this census.

The five primary outcomes reconcile exactly to starts. Labels distinguish logical
start, source arrival and processing sample. Synthetic same-run seconds support
subtraction; they have no civil epoch/timezone. Physical/report clock relationships
are unsupported. Runtime acceptance does not establish physical departure. Synthetic
horizon censoring requires replay-verified exhaustive event processing and the active
pending movement; operational silence cannot supply the same bound. Missing/censored/
disputed intervals have no exact label, never zero or a predicted replacement.

Only original static start inputs reach the feature builder. B0, route distance,
coordinates, directed edge free-flow times and original traffic factors are retained.
Graph road class is `UnsupportedSemantics`; day-of-week is `NotApplicable`.
`ArchivalLoss`, `PredictorTimeAbsent` and unsupported interpretation remain distinct.
No readiness, future traffic, actual delay, latest state, seeds or scenario identities
are default predictors. `original_route`, scoped keys and witnesses serve lineage;
future model code must consume the declared numeric allowlist, not flatten all JSON.

The initial split protocol conservatively groups shared graph/family/world/realization,
replay and explicit protected episodes by connected closure. Exact source assignments
are supplied before evaluating outcomes. With inadequate protected groups, predictive
holdout evaluation is `EvaluationUnavailable`; descriptive dataset publication can pass.
The sample's complete, truncated and replay runs share one group. They do not provide
independent samples, uncertainty estimates, physical generalization or causal policy effects.

The frozen evaluation package declares future equal-action MAE, RMSE, residual sign,
tails, coverage and missingness rules. This phase computes **no baseline error metrics**.
Phase 24 must review this evidence, separately authorize evaluation, respect exclusions
and protected assignments, and report unsupported uncertainty/generalization honestly.

For privacy, only synthetic fixtures belong in this public directory. Bundle source
capture intentionally includes relevant Rust/configuration files, not .t3, environment
variables, credentials or unrelated images. Access to future raw operational locations,
stable IDs and timing needs source-owner controls and minimized exports. There is no
universal retention interval. Removing bytes narrows the reproducibility guarantee.

To reconstruct the extractor independently, write each
`bundle.reproduction.files[path].utf8` into a **new** source directory after verifying
its SHA-256. These are original repository-relative paths; reject absolute or escaping
paths when restoring. Build `cargo +1.99.0 build -p roadrunner-cli --bin roadrunner --locked`
inside that directory, matching the recorded toolchain, then run its binary's
`dataset verify` on the retained bundle. No Git checkout or original scenario path is
needed for verification. The [reproduction evidence](evidence/phase-23-reproduction.json)
records an actual reconstruction/build/verification from those bytes. This reproduces
the extractor and source replay; the entire external dependency cache and existing
repository test-fixture tree are not duplicated inside every dataset bundle.
