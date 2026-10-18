# Phase 34 entry and workload plan

The user explicitly authorized Phase 34 after reviewing Phase 33. The headless
workload implementation and staged outcomes are recorded in
[completion](../phase-34-completion.md). The requirements below remain the broader
qualification plan; unqualified environments are listed explicitly in completion.
The targets are generated populations, not supported capacity or completion promises:

- A: 100 riders / 1,000 orders.
- B: 1,000 riders / 10,000 orders.
- C: 10,000 riders / 100,000 orders.

Begin with correctness fixtures and intermediate qualification. Test sustained,
burst, accumulation/drain, hotspot/dispersed, traffic-churn, planning-complexity,
mass-arrival and history-aged families. Vary graph size/topology independently.

The immutable manifest must record generator/schema/configuration versions, seeds,
graph/artifact digest, input checksum, canonical ordering, anchor-selection and
geographic distributions, initial rider populations/capacity, order arrivals,
pickup/dropoff, readiness/service, traffic/interventions, policy/budgets and horizon.
Hardware and toolchain belong in measurement metadata, not exogenous input identity.
Materialize generation outside primary execution timing and measure its load/memory
separately. Preserve declared unreachable/infeasible requests and actual engine results.

The Phase 33 grid manifests qualify narrow probes, not this generator or a city model.
Use exact graph anchors; uniform node sampling is not uniform population demand.
Do not silently extend the four legacy policy comparison harness to modern planners.

Measure six planes separately: core routing, dispatch, operational API, headless
simulation, durability/restoration/event replay, and visualization. The ordinary HTTP
simulation cap remains 10K orders/riders and scenes remain bounded to 32 riders/64
orders/2K queue entries; larger tests need a qualified headless harness, not raised caps.

Record offered/admitted/completed/rejected/noncommitted/pending/uncertain identities,
activity/stop/capacity populations, denominators, raw operation latency, peak/backlog
age and depth, drain time, CPU/memory, actual route/evaluator/submission work, evidence
age/composition and bytes. Reconcile counts by lifecycle rather than one artificial
conservation equation. Completion-duration percentiles retain unfinished populations.

Service tests need independently scheduled offered demand and client scheduling lag;
closed-loop concurrency-one latency is separate. Logical simulation time/wall time
is separate from sustained ingress throughput. Cache modes, process/filesystem/DB
warmth, storage/fsync/anchor configuration and instrumentation scope are declared.

History-aged tests hold active populations approximately fixed while completed orders,
effects/aliases, transitions, publications and original contexts grow. No expiry or
compaction is authorized. Durable command ACK, downstream drain and verified serving
readiness after restart are different measurements. Use disposable databases, source
artifacts and independent anchors; include accepted pending/uncertain work recovery.

Predeclare harness duration/resource stop boundaries from smaller qualification;
measure host safety without inventing engine graceful rejection. Stop escalation on
semantic failure, unsafe resource exposure or unobservable outcomes. Preserve raw
failure artifacts and reconcile accepted commands after recovery where supported.

Entry requires a reviewed finite Phase 33 completion package, qualified graph/input
availability, metric definitions, suitable resource tools, safe workload harness and
correctness proof plan. It does not require Tier C success. A failed tier with correct
accounting and identified constraints is useful evidence; new remediation needs scoped
review rather than arbitrary rewrites.
