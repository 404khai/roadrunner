# Phase 13 completion

Date: 2026-10-04
Status: COMPLETE — Basic Dispatch

The pre-Phase-13 remediation already implemented the narrow assignment workflow.
This closeout reuses those APIs and adds an executable example and dispatch usage
documentation. No routing, domain, ranking, or commit implementation is duplicated.
The [remediation report](pre-phase-13-dispatch-remediation-report.md) retains the
original architecture entry-gate evidence; [dispatch.md](dispatch.md) documents the
completed phase's public behavior and how to run it.

## Requirement review

| Phase 13 requirement | Implemented behavior | Evidence |
| --- | --- | --- |
| Find nearby available riders | Exhaustive eligible fleet or spatial radius/limit shortlist with explicit coverage | `generate_candidates`; eligibility and coverage fixtures in `entry_gate.rs` |
| Calculate rider → pickup ETA | Road travel through pinned `RouteProvider` | Routed-best-versus-nearest fixture; runnable example |
| Calculate pickup → customer ETA | Road leg departs at predicted pickup departure | Propagated-departure and FIFO traffic fixtures |
| Score candidates | Exact pickup travel + delivery travel, finite ordering | `BaselineStrategy`; exact ties, permutation, and sub-epsilon difference fixture |
| Select best rider | Minimum among feasible evaluated riders; lower identity on ties | Immutable assigned proposal and canonical selection evidence |
| Explain assignment | Per-candidate timing/distance, score contributions or rejection, coverage, provenance, and tie reason | Serializable `DecisionEvidence`; example JSON |
| Apply assignment coherently | Explicit atomic commit with stale/prior-state checks | Commit fixtures and world unit regressions |
| Preserve execution rules | Shared readiness, pickup, delivery, custody, and remaining-plan transitions | Shared-transition fixture; runnable example through completed delivery |

The initial score is road travel only, with explicit zero waiting/service and
soft-observed deadlines. Complete coverage means the Basic Dispatch eligible fleet,
not busy riders or a fleet-wide optimization problem. Spatial coverage remains
potentially incomplete. Readiness-aware dispatch is not part of this phase.

## Validation

Closeout validation passed:

| Command | Result |
| --- | --- |
| `cargo fmt --all --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace` | PASS — 96 tests; doc tests pass |
| `cargo run -p roadrunner-dispatch --example basic_dispatch` | PASS — selection and execution assertions pass; JSON output parses |
| `git diff --check` | PASS |

Relative file links in both new dispatch documents resolve. The example asserts that exhaustive
selection chooses rider 2 while spatial screening chooses rider 9, then executes the
exhaustive plan and validates the resulting delivered state, empty assignment/plan,
and zero custody-derived load. These are synthetic correctness observations, not
performance measurements.

No new benchmark claim is made. Existing rider-index tests and the benchmark target
remain available in `roadrunner-dispatch`; strategy comparisons belong to Phase 16.

Phase 14 and subsequent phases remain deferred.
