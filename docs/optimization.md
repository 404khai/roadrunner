# Dispatch optimization and performance engineering

Roadrunner has two distinct optimization concerns: selecting feasible delivery plans
and improving the implementation's resource use. Faster code must preserve the former's
objectives and publication rules.

## Delivery planning

```text
coherent world + policy + readiness + execution evidence
                           |
                 eligible rider candidates
                           |
                whole-plan insertion / fleet search
                           |
             feasibility + lexicographic comparison
                           |
           revalidate CURRENT authority -> atomic publication
```

[Basic/preparation dispatch](dispatch.md) evaluates pickup, readiness, service and
delivery rather than treating straight-line proximity as completion quality.
[Insertion](multi-order.md) adds pickup/dropoff positions to the editable suffix,
reevaluating the complete plan with propagated times and capacity at each stop.
For s editable stops, insertion positions grow quadratically; the complete-plan
checks invoke routing repeatedly, so position count is not total computational cost.

[Fleet search](fleet-optimization.md) adds deterministic greedy, multi-start and local
improvement under explicit work budgets. Its objective prioritizes newly admitted
order count, then remaining evaluable road time, then distance, then canonical fleet
identity. Existing obligations are constraints, not scored rewards. This is heuristic
planning, not an exact global VRP solver. [Recovery](dynamic-redispatch.md) additionally
preserves custody, frozen execution, accepted protections, churn/cooldown constraints
and explicit eligibility for reassignment. A stale proposal cannot publish solely
because its earlier evaluation was feasible.

## Computational contracts

Semantic submission budgets, actual route/evaluator calls, resource admission,
wall-clock deadlines and cooperative cancellation are separate. Candidate construction,
cloning and nested routing can happen outside counted submissions. Exhaustion does
not prove infeasibility or authorize prohibited incomplete incumbents. Accepted
operational commands survive client disconnect according to their recognition and
durability contracts; disposable read-only computation has different ownership.

[The performance audit](performance/architecture-contract.md) and [Phase 33 evidence](phase-33-completion.md)
record measurement and the finite optimization tranche. [Phase 34](phase-34-completion.md)
records workload qualification and failure evidence. Use [benchmarks](benchmarks.md)
for actual results; confirmed implementation patterns alone are not measured rankings.

Changes require matched inputs/revisions, correctness oracles, attributable before/after
measurements and visible regressions. Latency must be accompanied by throughput,
backlog and memory. Timing thresholds are not correctness tests. Cache hits cannot
fabricate search work, and parallel evaluation cannot alter deterministic objective
ordering. No unsafe Rust, distributed authority or policy rewrite is implied by a
performance investigation.
