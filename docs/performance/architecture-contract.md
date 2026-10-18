# Performance and scale architecture contract

Accepted by the user after four read-only grill rounds, Q1–Q18. This register is
the implementation contract for the finite Phase 33 tranche and future Phase 34.
The full audit was consolidated in conversation before implementation authorization.
Phase 34 implementation remains separately authorized work.

## Accepted decision register

1. **Workload meaning:** rider totals, online/eligible/assigned/travelling/waiting
   populations and capacity/stops are independent axes. Count generated, offered,
   admitted, assigned, completed and unresolved order identities according to the
   actual lifecycle. Declare arrival processes, policies, budgets, concurrency,
   graph, duration, outcomes and backlog. Separate simulation, API, durable,
   historical and rendering capabilities.
2. **Graph:** profile the lifecycle and ownership first. Preserve dense IDs, CSR,
   pooled directed geometry, full semantic identity and mandatory verification.
   Schema-4 is the baseline; alternate formats need independent versioned approval.
3. **Budgets:** distinguish semantic submissions, actual nested work, resource
   admission/envelopes and wall-clock cancellation. Exhaustion is not infeasibility.
4. **Reuse:** prove exact dependencies, algorithm/tie selection and diagnostic
   provenance. Prepared immutable metadata and bounded decision-local memoization
   precede shared caches. Never bypass current publication authority.
5. **Validation:** proof binds exact immutable contents, graph and time/context,
   not WorldId/version alone. Public arbitrary-world helpers stay defensive;
   corruption remains an error. Persistent incremental validation is separate scope.
6. **Spatial:** preserve eligible-first inclusive radius/top-K, Haversine and
   distance/RiderId ties followed by canonical evaluation ordering. Never query K
   then filter eligibility, retain stale membership, or use renderer positions.
7. **Dispatch:** equivalent changes preserve candidates, canonical budget prefix,
   objectives, ties, protections, diagnostics, isolation and publication rules.
   Lazy generation cannot silently change whole-neighborhood counts. Changed
   screening/pruning/neighborhoods are policy changes needing separate approval.
8. **Resources:** retain single authority and accepted-command ownership. Observe
   pre-admission work, waiters, execution, locks, publication and responses.
   Disconnect does not erase a reserved command, prove noncommit or permit repeats.
9. **Concurrency:** serial authoritative simulation is the reference. Parallel
   independent runs differ from single-world capacity. Pure parallel evaluation
   needs canonical aggregation, prefixes/reductions and schedule equivalence;
   partitioned authority requires a separate architecture decision.
10. **Evidence:** separately measure headless trace, original traffic duplication,
    frames, export, encoding and browser import/seek. Preserve bounded scene-v1,
    original route/context/schedule values and dependency lifetimes. New archives
    or reduced evidence require explicit compatibility/capability decisions.
11. **Retention:** historical orders, transitions, effects, aliases, publications
    and temporary copies are distinct. Checkpoint-v1 requires history from revision
    zero. No silent expiry, truncation or recognition loss. Compaction and future
    retention/privacy changes require separate migration and recovery approval.
12. **Durability:** preserve transaction, original acknowledgement, anchor fsync,
    fencing, recognition and uncertain-COMMIT guarantees. Measure command ACK,
    downstream consumer application and verified restoration separately.
13. **Generation:** use immutable graph-qualified exogenous scenarios, generator
    version/configuration, seeds, checksums and canonical ordering. Valid anchors
    do not imply reachability. Never regenerate engine failures away. Generation,
    load and execution are separate; target size does not raise HTTP limits.
14. **Claims:** match semantic work/evidence and conditions. Retain raw data,
    estimator/denominator/sample limits, unfinished work and uncertainty. Open-loop
    service load differs from sequential clients and logical simulation throughput.
    No service SLO or production claim without representative evidence.
15. **Diagnostics:** preserve canonical expansions/budgets/evidence. Use bounded
    operation-local diagnostics and scoped external observations; measure overhead.
    RSS, payload sizes and actual allocation are different. Diagnostics do not
    become deterministic identity, clock or publication inputs.
16. **Selection:** require a measured constraint, attribution, finite tranche,
    semantic tests and matched before/after evidence. Retain tradeoffs and negative
    results. A microbenchmark gain alone does not prove end-to-end benefit.
17. **Overload:** qualify refusal, waiting, incomplete search, staleness, uncertainty
    and uncontrolled failure per boundary. Harness termination is not graceful
    engine rejection. Use isolated disposable state and explicit stop conditions.
18. **Gates:** finite Phase 33 completion includes baselines, attribution, verified
    changes, overload findings, remaining constraints and Phase 34 plan. Separate
    authorization starts Phase 34; neither phase licenses new search policy,
    archives, truncation, weakened durability or multiple authorities.

## Current cost model and evidence boundaries

`FrozenGraph` uses dense vectors, CSR and pooled geometry (`graph/network.rs`).
Its storage grows with nodes + segments + directed edges + points + maneuver pairs.
Build/encode/decode peaks differ from retained graph storage; durable startup clones
owned graphs (`api/persistence.rs::open_postgres`).

Nontrivial Dijkstra/A* allocate node-sized or incoming-edge-state-sized arrays.
Admitted concurrency multiplies scratch; heap growth is additional. Maneuver
transitions depend on incoming/outgoing degree products. API travel-time A* prepares
bounds across edges; dispatch CoreRouteProvider uses Dijkstra. Yen retains full path
prefixes. No new route cache or search budget is delivered by Phase 33.

Snapshot construction validates world/anchors/context. Public basic eligibility
continues to validate arbitrary WorldData. Candidate construction uses that exact
snapshot proof after Phase 33; order, rider and commit checks still apply.
Spatial trees remain rebuilt immutable eligible-population trees.

Insertion has quadratic placement counts in editable stops, with propagated leg
routing. Fleet reevaluates nonisolated plans; recovery eagerly materializes complete
candidate fleets before submission limits. World copies and retained evidence add
cost beyond route calls. Insertion, fleet healthy-remainder and recovery publication
rules differ; global VRP optimality is not promised.

Simulation is a stable event heap, not per-frame authoritative movement. Ordinary
runs retain starts, schedules, effects, decisions and history. Original started-action
traffic remains logically A×E entries; Phase 33 shares physical storage within an
unchanged context. Scene capture still clones worlds per processed queue boundary.
The finite result/JSON/browser representation can dominate peak memory even after
internal sharing. No historical evidence or wire fields are omitted.

Durable publication stages full images, writes provisional/final relations, commits
SQL, then persists an independent acknowledgement anchor. History/recognition growth,
SQL/lock/anchor work and replay/restoration require distinct aged-state measurements.
Existing PostgreSQL/event baselines are tiny sequential fixtures, not capacity proof.

## Correctness and proof matrix

- Graph: identity, IDs, orientation, maneuver legality, artifact/restore verification.
- Routing: independent small oracle, algorithm-specific paths/ties, traffic/FIFO,
  errors, original counters. Existing `routing_correctness` tests apply.
- Eligibility: defensive helper oracle, corrupt worlds, equal numeric identities,
  changed graph/context and assignment-only eligibility changes.
- Spatial: linear eligible-first oracle, inclusive boundaries, ties and geography.
- Planning: independent small enumeration/ranking, exact prefix/budgets, protections,
  isolation, incomplete outcomes, and diagnostic meanings; existing fleet/pooling/
  recovery tests remain required.
- Sharing: exact serialized full-result hashes, original contexts under churn,
  independent mutable artifact edits, dependency lifetime and array round-trip.
- Concurrency: worker/completion-order perturbation before any parallelism change.
- Durability: real SQL tests plus process crash/acknowledgement/restore scenarios.
- Replay: source order, causal backwards seek, accepted arrival/completion and
  explicit unavailable geometry; scene-v1/Web tests remain required.

## Overload qualification matrix

- Command saturation: `workers_busy`, no new reservation; retries retain recognition.
- Route saturation: awaits permit; no newly bounded waiting room or cancellation.
- Pre-admission DB/blocking work: remains outside universal worker accounting.
- Candidate memory: eager recovery remains; no universal byte/CPU bound.
- Stale planning: existing source/context fencing rejects publication.
- PostgreSQL: 5s lock/30s statement timeouts are not total request deadlines.
- COMMIT/anchor uncertainty: writer fails closed; reconcile, never infer noncommit.
- Exports: existing admission and byte caps; intermediate allocations remain.
- Slow/dropped body: export permit lifetime differs from CPU worker lifetime.
- Consumer gaps/poison: existing defer/quarantine/backoff/reconciliation semantics.
- Process/OOM: graceful global OOM rejection is unproven; external kills are failures.

These are current contracts and source findings, not new stress-test results.
Future overload tests must record trigger, actual outcome, held resources,
observability, recovery, guarantees and unsupported behavior for each row.
