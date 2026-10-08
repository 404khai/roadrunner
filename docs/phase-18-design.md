# Phase 18 implementation contract

Status: implementation authorized 2026-10-08. ADR 0016 remains normative.

Optimize a canonical current unassigned batch jointly with editable suffixes. Existing
assignments, custody, accepted terms, and active execution remain fixed. Phase 19
reassignment/recovery is excluded. Reuse Phase 17 timing, projection, service/readiness,
capacity, and accepted protection validation.

Initial algorithm: deterministic multi-start greedy construction followed by strict
best-improvement local search. Neighborhoods admit an unplaced request, relocate any
editable stop within its owner plan preserving precedence, relocate a newly proposed
pickup/dropoff pair across riders, and displace one newly proposed request while admitting
an unplaced request, then reinsert the displaced request. Committed orders never change
riders. No weighted objective, demand reward, workload limit, host clock, or optimizer
framework. Compare admitted count, remaining evaluable fleet road seconds, distance,
canonical rider/semantic-stop identity. Isolated riders are constant and excluded
identically from comparisons; unknown road contributions are never fabricated.

A deterministic budget counts complete fleet submissions to the evaluator. Baseline
health/input checks are outside this budget. Exhaustion yields SearchIncomplete and no
proposal/commit. Completion means the declared multi-start construction and local
neighborhood are covered and converged, not globally optimal arbitrary VRP. Independent
tiny exact enumeration reports admission and road-objective quality without reusing
production generation/ranking. Greedy and local-only comparisons establish measured
quality and expose local optimum limitations.

Isolate accepted-hard-breached riders unchanged. Isolate required unavailable prediction
or unroutable-baseline riders and unavailable new-order predictions; retain explicit
incomplete INPUT coverage. Structural corruption, missing/unsupported policy, or invalid
projection is an error rather than isolation. Solve the healthy evaluable remainder.
Exact context binds the whole world identity/version, clock, graph/traffic/profile,
anchors, policies, forecasts, frozen projections, isolation facts, optimizer and budget.
Publish all new assignments, replacement plans and new authentic acceptance terms in one
validated world transition; any stale context rejects everything with zero mutation.

Simulation schema 3 selects fleet_batch dispatch. Equal-time creation/ready events form
a current batch before one queued fleet decision; each successful fleet proposal commits
once and execution reads current plans after finishing active actions. Planning admitted
orders and realized delivered orders remain distinct. Evidence includes expected/proposed
state, isolated inputs/riders, baseline/chosen timing, signed customer impacts, canonical
batch, work/neighborhood/rejection/termination coverage, and publication. External
benchmarks own machine/compiler/dataset hashes and raw timing samples.
