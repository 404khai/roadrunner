# Phase 18 fleet scenarios (schema 3)

Each JSON document is self-contained with an inline graph, named/versioned scenario,
explicit per-order service/readiness/protection policies and `fleet_batch` dispatch.
Run from the repository root:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-18/joint-pooling.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-18/greedy-trap.json --json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-18/budget-exhaustion.json --json
```

| Fixture | Declared purpose |
| --- | --- |
| joint-pooling | Equal-time A/B form one batch and commit together; both execute |
| greedy-trap | Greedy admits one; local ejection/multi-start admit both on two riders |
| frozen-wait / frozen-service | New batch preserves active pickup timing and identity |
| capacity-prefix | Capacity remains hard at every stop; sequential service may still admit work |
| hard-deadline | No candidate-induced hard completion violation |
| cumulative-repeat | Repeated batches retain original authentic acceptance reference |
| baseline-breach | Baseline hard-breached rider isolated unchanged |
| input-isolation | Unknown forecast isolated; later observation permits a fresh admission |
| complete-infeasibility | No admission over the completed declared search; no global VRP proof |
| budget-exhaustion | SearchIncomplete, no assignment/plan/terms publication |
| realized-violation | Admission prediction feasible but actual readiness causes realized violation |

Committed custody/resequencing and the 32s-versus-30s local minimum use dispatch test
fixtures because runtime scenarios construct fresh valid worlds rather than importing
already-picked-up state. See [fleet contracts](../../../docs/fleet-optimization.md).

The benchmark collector compares `Greedy`, `LocalSearch`, and `MultiStartLocal` on all
fixtures with identical exogenous inputs. Algorithm choice is explicit in scenario
JSON (these names are case-sensitive). Required isolated input coverage may be incomplete
while a healthy remainder publishes. Complete budget coverage always remains mandatory.
