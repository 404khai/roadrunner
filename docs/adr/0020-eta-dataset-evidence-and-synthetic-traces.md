# ADR 0020: Started-leg dataset evidence and complete synthetic sources

Status: Accepted implementation of the signed [Q1–Q43 audit](../data-ml-readiness-audit.md)
and [Phase 23 authorization](../phase-23-authorization.md). The audit's original
confirmation language is preserved verbatim as historical provenance; it is signed
off by the subsequent authorization. Source thread: `67a08df0-4aab-4932-9af2-443c26d06e69`,
audit position 485, authorization position 487. The later root-checkout branch and PR
instructions govern implementation delivery.

## Decision

Implement a small offline Rust crate, `roadrunner-dataset`, depending on simulation,
dispatch and core. No database/HTTP/event dependencies flow into routing. The first
publishable cohort is verified synthetic evidence. Operational exports and physical
clock conversion are unsupported, rather than inferred from durable acceptance.

The simulator adds `synthetic-started-actions/v1` without changing existing fields
or event processing. It synchronously captures each successful `start_action`, its
original `ActiveExecution`, revision and full static traffic factors. Successful
delays record generations and original action/route identity. The final trace retains
read-only authority effects/history/action allocator, active work and every pending
queue entry. The allocator is an independent denominator: fresh simulations allocate
contiguous ActionIds from zero inside successful staged transactions. Rejected starts
cannot increment it. Completed movements are not used to create identities.

`simulate_with_run_identity` accepts an archive-owned run token. Ordinary `simulate`
uses a graph/input content-qualified replay token after normalization of unordered
rider/order/override inputs. Repeated deterministic invocations are the same replay,
not independent realizations. Separate captured runs qualify repeated fixture namespaces.

Source capture retains graph artifact bytes, complete scenario and result bytes,
SHA-256 identities, run scope and hierarchical lineage. Each source has its own
inclusive horizon and final operational revision. Publication reruns the retained
scenario using the verified original graph, compares every recorded result field,
then reconciles starts, generations, ownership, effects and pending work. Replay is
new instrumented synthetic evidence. It does not backfill nonexistent old trace fields.
This verification is deliberately expensive and offline; no throughput claim applies.

One canonical row is `(run, namespace, ActionId)` with RiderId consistency. B0 remains
the original frozen route's elapsed travel estimate. Labels use accepted synthetic
arrival minus logical start, without requiring pickup/service/delivery completion.
Pending, RightCensored, MissingArrival and DisputedOrInvalid remain explicit states.
Only verified synthetic complete coverage plus active/pending evidence permits the
synthetic horizon lower bound. This is not physical censoring. Unknown/incompatible
clock intervals get no exact duration. Credible disputes create an ineligible new
assessment version; rejected corrections never replace the accepted endpoint.

Feature extraction accepts only `StartedAction` and an optional original graph.
It cannot read scenario latent readiness/delay scripts, outcomes, final state or
rescheduling. Capture at successful start establishes availability for static,
departure-invariant inputs; commit revision is the ordering witness, not a fabricated
universal timestamp. Retrospective archival time is not predictor knowledge. B0,
distance, coordinates, directed traversals, free-flow times and original traffic are
supported. Road class is unsupported by current graph bytes; civil day-of-week is
not applicable to run seconds. Archival loss differs from predictor-time absence.
IDs, route/context hashes, seed, family, policy and source names remain metadata.
No rolling aggregates or fitted preprocessing are implemented.

Five logical components share one JSON bundle: source/census, features/B0, labels,
lineage/dependencies and manifest/quality. Canonical sorted JSON keys, roundtrip
binary64 and lossless u64 preserve existing numeric semantics. Semantic and pretty
packaging digests differ. Actual Rust source/config bytes, Cargo.lock, toolchain,
base revision and patch bytes are retained, not only hashes. New publication uses
fsynced staging and rename; an existing dataset destination is refused. Readers
verify packaging, semantic identity, source replay and a full derived-component rebuild.

Splits require exact whole-source assignments and connected dependency closure over
shared graph, family, base world, realization, replay and explicit episode/coupling
references. This conservative initial protocol supports descriptive or unseen synthetic
component claims. It does not promise within-graph independence. Missing/extra assignments,
unexplained exclusions or crossed protected dependencies fail. Insufficient train,
validation and protected test groups yield `EvaluationUnavailable`. Excluded sources
remain in the census. No random rows or percentages manufacture a holdout.

## Consequences and limits

Publish synthetic engineering evidence with explicit lack of physical validity.
The representative replay/horizon runs share one protected group. Their row count
cannot support independent confidence intervals or causal dispatch claims. Unknown
ingress/potential demand is not zero. Selected starts are not all candidate outcomes.

Raw synthetic bundle inputs are safe for repository publication; operational sources
would require separately established source, clock, privacy, retention and historical
capture contracts. Do not put production trajectories, credentials or private data in
this public example. Authorized removal narrows reconstruction/reproduction claims.

The frozen Phase 24 package records endpoint/B0/eligibility/split and metric definitions;
no MAE/RMSE evaluator or result is introduced. Phase 24 requires completion review and
separate authorization. Whole-route duration is the later research granularity; no
edge supervision or learned A* change is implied by the roadmap's old interface sketch.
