# Pre-Phase-20 Prerequisite Remediation — Completion Report

Executive verdict: **PASS — READY FOR PHASE 20**.

All 18 prerequisite gates pass within the documented foundation and named temporal
policy. Phase 20 is **not started or authorized by this report**. HTTP endpoints,
credentials, databases, brokers and distributed services were not introduced.
Explicit user confirmation and separate Phase 20 authorization remain required.

The work is isolated on `feat/pre-phase-20-remediation`, based on Phase 19 commit
`4271ddd`. The original checkout and unrelated work were preserved. The signed
[Q1–Q33 audit](runtime-boundary-audit.md) remains normative; no architectural
amendment or weakened accepted-service invariant was needed.

## 1. Implemented scope and source changes

- **Core graph:** `graph/network.rs`, `artifact.rs`, `error.rs`, `mod.rs`, graph
  tests. Every builder/modifier seals canonical compiled content; every operational
  load verifies it. Historical inspection preserves original claims.
- **OSM:** `compile.rs`, `provenance.rs`, `snapshot.rs`. Compiler v4 binds compiled
  provenance and its semantic version/configuration; bundles verify these bindings.
- **Dispatch:** new `operational.rs`, `temporal.rs`, `history.rs`; updated domain,
  routing, pooling, World and exports. Namespace/revisions, private coherent state,
  shared execution, once-only effects, adoption and certified publication are
  reusable library foundations. Existing evaluator/optimizer objectives remain intact.
- **Simulation:** engine uses OperationalState and shared domain transitions/frozen
  projection; ExecutedLeg aliases the shared pinned-leg type. Driver clock, heap,
  observations, seeded randomness and metrics remain in simulation.
- **Evidence/tooling:** focused graph/operational tests, verified traffic fixtures,
  archived-baseline replay comparator, graph/boundary collectors and measured results.
- **Normative docs:** AGENTS, specification, architecture, glossary, API guide,
  crate/root READMEs and seven existing ADR amendments; three cohesive new ADRs
  [0017](adr/0017-operational-authority-and-publication.md),
  [0018](adr/0018-planning-time-and-historical-provenance.md),
  [0019](adr/0019-durable-authority-and-event-contracts.md).

See [the complete implemented API contract](runtime-boundary.md) and Rustdoc.
New regular dispatch dependencies are SHA-256, canonical JSON serialization and
pinned OS entropy support; the workspace has no new infrastructure crate.

## 2. Graph semantic identity and history

Graph artifact **schema 4**, identity domain **graph-semantics.v2**, and OSM
**compiler v4** bind topology, directed edges, geometry, traversal/access/speed,
adjacency, maneuver restrictions, capability/build metadata and required compiled
provenance. Float bits and canonical E7 coordinates avoid ambiguous numeric encoding.
The self digest/compact ID are excluded from hashing. Compiled provenance clears its
self graph reference before its separate digest, avoiding a circular construction.
Compiler semantic version is also bound in canonical build configuration.

GraphBuilder assertions cannot publish arbitrary IDs. Finalization, maneuver
modification and provenance binding reseal. The compact u64 ID derives from the
full digest and cannot establish detached identity alone. Loading recomputes semantic
identity even if an attacker/mistake updates the artifact integrity checksum.

Tests cover reordered canonical construction, roundtrip, geometry, speed, access,
directionality/topology and maneuver changes, false asserted identity, rehashed
semantic corruption and historical schema rejection. Existing OSM source-change,
restriction, provenance-swap and bundle tests pass. Both current Lagos snapshots
were additionally deep-verified by the CLI.

Schema 3 remains **inspection-only, semantically unverified**; operational decoding
rejects it. Original Phase 10/11 fixtures and old benchmark evidence are unchanged.
New `.semantic-v2.json` fixtures are explicit current records, supported by
[verified graph correspondence](evidence/pre20-graph-correspondence.json).
Archived compiler output and current output have equal declared routing-content
fields; compiler-identity metadata differences are recorded, not silently relabeled.
Existing snapshot mismatch tests reject stale anchors. A frozen action retains its
G1 route/digest through G2 adoption and can finish without interpreting G1 IDs on G2.

## 3. Coherent operational authority and publication

OperationalState privately owns World, active execution, effective plan revisions,
context adoptions, cooldown, allocators, effect recognition, committed history and
publication records. Read-only snapshots support evaluation outside the writer.
There is no mutable World dereference, public writable clone or externally supplied
state replacement. Standalone World remains trusted compatibility/domain code;
its weaker commit paths cannot mutate an operational authority. Explicit simulation
compatibility can be selected only at bootstrap.

All transitions stage a private complete replacement, validate, derive changed
plan revisions, advance exactly one namespace-bound OperationalRevision and publish
atomically under exclusive mutable access. Readers cannot observe a partially
updated World/action pair. Rejections discard staging. Context adoption participates
in that lane and advances a separate monotonic adoption revision, including ABA.
The committed history identifies predecessor/successor revision, domain versions,
changed plans/execution/context categories, effects and recovery timing.

This proves **volatile in-memory** publication. It does not prove storage transactions,
fencing, distributed writer exclusion or crash durability. Phase 21 must establish
those guarantees using actual storage; none is claimed now.

## 4. Identity and shared execution results

Existing typed local IDs acquire durable meaning with OperationalWorldNamespace.
Fresh live lifetimes use OS entropy; fixtures use explicit deterministic namespaces.
Canonical order/rider allocation checks exhaustion and cannot leak failed creation.
Lossless references are strings. Bootstrap is not a durable restoration API.

PlanRevision, immutable ActionId, ScheduleGeneration and AppliedExecutionEffectId
are distinct. Starting future work requires its applicable plan revision. Rescheduling
changes the attempt/generation, retaining the logical action and original route.
Started frozen actions finish across valid suffix replacement without requiring
originating-plan equality with the newest plan. New departures cannot backdate prior
execution effects or reuse an obsolete adopted graph.

Shared dispatch transitions validate rider/order/ownership, stage, generation,
readiness, service, custody and plan progression. Pickup/dropoff publish all domain,
execution and recognition effects together. Per-order release and capacity-prefix
invariants remain enforced by shared World validation. Pending progression after
completion is valid and next_stop reads the current plan.

Identical effects return AlreadyApplied under the same or distinct input identity.
Conflicting identity/meaning rejects. No duplicate custody, load, fulfillment, stop
advancement or logical revision occurs. Alias recognition may add diagnostic
recognition metadata without pretending a new business transition happened.
The simulator also guards the current stage before driver metrics, preventing
same-current-movement redelivery from adding distance twice.

The new boundary suite includes **17 passing tests**, 24 seeded complete execution
histories, repeated terminal effects, old generations, frozen travel/wait/service,
suffix replacement, graph adoption, stale fleet publication after pickup, competing
assignment publication, every adopted context category and complete-state/history
rollback assertions. The release measurement test runs separately.

## 5. Operational time and temporal applicability

OperationalClock provides authoritative samples in an explicit named time domain.
LogicalOperationalClock enforces monotonic advancement; host-time conversion remains
an adapter responsibility. Evaluation, publication and commit samples are separate.

`static-road-monotone/v1` certifies the **exact original** decision over an explicitly
proved interval. Static road seconds/distance preserve Phase 17 complete-search
ranking; fixed readiness/service produce monotone completion feasibility. The original
winner must remain feasible at the endpoint, with healthy baseline and independently
retained deadline/cumulative terms. Forecast assumptions must remain valid throughout;
observed readiness and explicit legacy readiness ignore unused forecast expiry.
Frozen projected completion must extend through the interval.

Phase 18 certification preserves incumbent feasibility, admitted count and static
road accounting, without inventing a global optimum. Phase 19 preserves recovery
feasibility, locked ownership, original terms and churn accounting. New accepted_at
and recovery cooldown use the **actual commit sample**. Completion references remain
those authentically evaluated at acceptance; neither rewrites nor delay reset them.

Demonstrated outcomes:

| Scenario | Verified result |
|---|---|
| Insertion evaluated at 0, certified through 2, commits at 1 | PASS; original completion reference 30 retained, accepted_at 1 |
| Fleet incumbent and recovery with nonzero delay | PASS; original plans/terms retained; recovery cooldown stamped at actual commit 2 |
| Hard deadline or cumulative protection violated at endpoint | Reject; no semantic mutation |
| Required forecast expires | Typed prediction failure; no publication |
| Unused forecast expires after observed readiness | Publication permitted; observation remains authoritative |
| Graph/traffic/readiness/profile/policy adoption or ABA replacement | Stale; zero proposed mutation |
| Pickup advances after fleet evaluation | Execution preserved; entire old fleet proposal stale |
| Clock expires between staging and replacement | Complete rollback, including terms/history/publication evidence |
| FIFO traffic changes exhaustive winner with no revision change | Winner demonstrably changes; obsolete claim cannot certify |
| Old generation or conflicting effect reuse | Reject; custody/plan/revision unchanged |

No universal TTL or timestamp-equality rule was introduced. The initial proof supports
free-flow/immutable static traffic and explicit evaluable active-prefix projections.
Time-dependent certification returns UnsupportedTemporalModel. Unknown active-prefix
certification is an explicit input/context limitation; it never fabricates readiness
or abandons existing work. Existing simulator Phase 18 isolation remains intact.
Broader operational proofs are extensions, not silently supported configurations.

## 6. Historical envelopes and compatibility

HistoricalRecord schema 1 distinguishes Command, Decision, Commitment and Execution
records, source revision, policy semantics/configuration digest, algorithm semantics,
prediction/routing context and time. Command registry/durable retention are future work.
PublicationProvenance stores the original decision separately from actual committed
World/execution/plan/cooldown facts and certified endpoint predictions.

Legacy Phase 13–16 soft deadlines, zero service and historic readiness behavior remain
explicit. No historic accepted reference, detour bound, acceptance timestamp or hard
policy was invented. Existing accepted references/terms remain immutable across all
new operations. Historical recomputation requires retained original artifacts and
compatible semantics; typed unavailable reasons prohibit substituting today's inputs.

Simulation evidence declares **shared-execution/v2** separately from scenario schema.
[Replay comparison](evidence/pre20-replay.json) covers **38 Phase 15–19 fixtures**
against an independently built archived `4271ddd` CLI, plus exact current replay.
Every other semantic field matches: timing, waiting/service, route paths, responsibility,
accepted values, custody/fulfillment, work/coverage/objectives, busy occupancy and metrics.
The only declared changes are graph/derived traffic identity, the version field, and
frozen logical ActionId representation after rescheduling. Exact changed paths and
original/current output hashes are recorded. Golden outputs were not blindly replaced.

Independent oracle evidence still passes: Phase 17 32 seeded repeated-insertion worlds,
Phase 18 72 tiny exact comparisons plus a declared 2-second local-minimum gap fixture,
and Phase 19 **32/32** seeded single-order recovery matches. Heuristic output is not
misrepresented as arbitrary global optimization.

## 7. Commands executed and outcomes

All commands below succeeded. Reviewable logs and hashes are in
[evidence/pre20-verification](evidence/pre20-verification/summary.json).

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.99.0 test --workspace --locked
cargo +1.99.0 build --workspace --release --locked
RUSTDOCFLAGS='-D warnings' cargo +1.99.0 doc --workspace --no-deps --locked
cargo +1.99.0 test -p roadrunner-dispatch --test fleet --test recovery -- --nocapture
cargo +1.99.0 bench -p roadrunner-core --bench graph_lifecycle --locked -- --noplot --warm-up-time 1 --measurement-time 2 --sample-size 20
python3 scripts/collect_graph_identity_benchmark.py --output benchmarks/results/pre20/graph-identity.json
python3 scripts/collect_operational_boundary_benchmark.py --output benchmarks/results/pre20/operational-boundary.json
python3 scripts/verify_pre20_replay.py --baseline /tmp/roadrunner-baseline-target/debug/roadrunner --binary target/release/roadrunner --output docs/evidence/pre20-replay.json
python3 scripts/collect_simulation_benchmark.py --runs 11 --output benchmarks/results/pre20/phase-15.json
python3 scripts/collect_dispatch_strategy_benchmark.py --runs 11 --output benchmarks/results/pre20/phase-16.json
python3 scripts/collect_multi_order_benchmark.py --runs 11 --output benchmarks/results/pre20/phase-17.json
python3 scripts/collect_fleet_benchmark.py --runs 11 --oracle-log /tmp/rr-pre20-oracles.log --output benchmarks/results/pre20/phase-18.json
python3 scripts/collect_recovery_benchmark.py --runs 11 --oracle-log /tmp/rr-pre20-oracles.log --output benchmarks/results/pre20/phase-19.json
```

Workspace result: **217 passed, 0 failed**; one intentionally ignored
measurement test was separately executed successfully in release mode. Debug workspace/
CLI builds, both deep graph CLI checks, focused graph/operational tests, Python syntax
checks and `git diff --check` also succeeded.

Archived baseline construction used `git archive 4271ddd` in `/tmp`, then Rust 1.99
CLI build with its own manifest, Cargo.lock and target directory. The parent checkout
was not reset or modified. Temporary snapshot paths in logs can be regenerated from
the declared PBF/source IDs and current/archived CLI commands.

## 8. Measured performance

Environment: **Apple M3, arm64 macOS 27.0, 16 GiB RAM**, Rust 1.99.0 release builds,
Cargo.lock/default target features. Hardware, input/definition/binary hashes, raw
samples, median/p95/p99, build/configuration and deterministic work are recorded in
[benchmarks/results/pre20](../benchmarks/results/pre20/operational-boundary.json).
Measurements ran without concurrent workspace validation. Process memory is
**unmeasured**; hardware RAM is not a process-memory measurement.

Boundary fixture: 3 nodes, 4 directed edges, 2 riders, 3 orders, 2 complete candidate
submissions/run; one warmup and **101** measured runs. Setup/assertions are outside
separate evaluation/certification/publication timers.

| Operation | Median µs | p95 µs | p99 µs |
|---|---:|---:|---:|
| Evaluate exhaustive insertion | 24.875 | 36.25 | 70.583 |
| Certify interval applicability | 26.084 | 52.709 | 117.5 |
| Stage/publish complete authority and evidence | 86.041 | 131.375 | 198.25 |

[Graph lifecycle evidence](../benchmarks/results/pre20/graph-identity.json) covers
1,000/10,000 nodes, 2,994/29,994 directed edges, eight operations/configurations,
20 Criterion samples each, one-second warmup/two-second target measurement.
Measured finalization medians are 6.354/67.181 ms;
verified-load medians 17.686/187.086 ms.
The 10,000-node artifact is 13,766,640 bytes. Criterion extended the slower load's
measurement duration to obtain all 20 samples; the raw log records that explicitly.

Scenario collectors cover Phase 15, four Phase 16 strategies, 11 Phase 17 scenarios,
36 Phase 18 configurations and 13 Phase 19 scenarios, **11 measured exact replays per
configuration** after one warmup. This includes process startup/load/JSON overhead.
Individual medians/p95/p99 and work counts are recorded, not inferred from the tiny
boundary test. No throughput, scale, latency SLO or universal optimizer-quality gate
is claimed; these measurements expose copying/hashing cost for later profiling.

## 9. Prerequisite gates

Every PASS below is scoped to the implemented foundation; future durability/events
are excluded. Evidence refers to current code plus successful logs/scenarios above.

| Required gate | Result | Concrete evidence |
|---|---|---|
| Universal verified semantic graph identity | PASS | Builder/modifier sealing, schema4 recomputation, compiler/provenance binding; core/OSM/CLI graph tests |
| Historical graph-identity compatibility | PASS | Inspection-only schema3, unchanged originals, explicit semantic-v2 correspondence |
| Coherent domain/execution operational authority | PASS | Private OperationalState World/execution/adoptions/cooldown; complete-state validation/history tests |
| Shared validated execution transitions | PASS | Simulation calls shared action/wait/service/effect/projection/next-stop methods; 38 baseline comparisons |
| No weaker operational mutation bypass | PASS | No DerefMut/public writable clone; operational legacy commits reject; sealed publication |
| Namespace-scoped durable identity foundations | PASS | Immutable namespace, checked order/rider allocation, distinct typed scopes and lossless references; exhaustion tests |
| Whole-operational revision semantics | PASS | Exactly-one transaction/history advancement; stale/rejected/duplicate tests |
| Adopted-context identity/revision semantics | PASS | Atomic context adoption, category hashes, ABA and all-category stale tests |
| Separate plan/action/schedule/effect identities | PASS | Expected future PlanRevision; immutable started ActionId; reschedule generation; effect recognition |
| Same-effect idempotency | PASS | Same/distinct IDs, conflicting reuse, generated pickup/dropoff duplicates and movement metric guard |
| Frozen-action preservation | PASS | Travel/wait/service and suffix-rewrite/G1-to-G2 tests; locked cancellation/recovery regression |
| Atomic complete in-memory operational transitions | PASS | Staging/replacement; committed history, competing decisions and expiry-after-staging full rollback |
| Versioned command/decision/commitment provenance envelopes | PASS | HistoricalRecord schema1, separate original decision and actual committed facts; actual-time assertions |
| Explicit authoritative operational clock | PASS | OperationalClock/monotonic named logical implementation; separate samples/domain/epoch and reverse-clock tests |
| Usable temporal validity for intended commands | PASS | Delayed insertion/fleet/recovery succeed under named static policy; hard/expiry/ranking-change tests reject |
| Phase 15–19 regression/replay equivalence | PASS | 217 workspace passes; 38 baseline semantic comparisons/exact replays; preserved independent oracles |
| Normative documentation/ADR reconciliation | PASS | Signed Q1–Q33 register, ADR0017–19 and amendments, operational spec/API and implemented-vs-future scope |
| Builds, format, lint and applicable tests passing | PASS | fmt, strict all-target Clippy, workspace/release/doc builds/tests; executed benchmark measurement |

## 10. Limitations, risks and phase ownership

| Risk / limitation | Severity | Mitigation and owner |
|---|---|---|
| Volatile authority cannot reconcile a discarded namespace | High for an operational deployment | Phase20 must disclose; Phase21 implements durable results/restoration/fencing |
| Time-dependent or unevaluable-active temporal proof unsupported | Medium applicability limit | Typed fail-closed result; static/evaluable model usable now; extend only with explicit proof |
| Whole-state/history/evidence copying and unbounded lifetime recognition | Medium performance/memory risk | Measured foundation; profile meaningful Phase20 workloads before finer concurrency/retention changes |
| Graph hashing/verified-load cost grows with artifact size | Medium performance risk | Measured 1K/10K cases; retain verified immutable graphs, profile without weakening identity |
| Historical schema3 identity cannot establish routing authority | Medium compatibility limit | Preserve inspection; explicit correspondence only; never silently relabel |
| Live observations and clock conversion need a trusted adapter | High if externally exposed prematurely | Phase20 authorization/source/ordering/clock gates; no HTTP exposure in this stage |
| Artifact/history retention and backup safety not implemented | Future durable risk | Phase21 explicit storage, migration, retention and restore/clone contract |

Current certification intentionally does not support every time-dependent profile or
Phase 18 unknown-active isolation case. The underlying Phase 13–19 simulator contracts
remain equivalent. This initial policy fulfills the accepted requirement for a usable,
defensible mechanism per publishing command; it makes no universal applicability claim.
Graph reanchoring is explicit future work: frozen G1 execution remains pinned and future
G2 planning cannot reuse bare G1 IDs. No ownership/custody handoff was added.

Infrastructure choices still open: HTTP/auth mechanisms and live-clock conversion in
Phase20; storage/schema/isolation/fencing/retention in Phase21; delivery/consumer/retry/
quarantine implementation in Phase22. Full event sourcing, mandatory brokers, global
ordering, fine-grained multi-writer publication, automatic failover, exactly-once remote
effects and arbitrary historical reconstruction remain deferred under the audit.

**Proposed architectural amendments requiring approval: none.**

## 11. Readiness and stop

**READY FOR PHASE 20** means the prerequisite foundation is verified and ready for
separately authorized Phase20 implementation. It does not mean the operational API,
production observation authority or any durable/event guarantee already exists.

Pre-stage entry was the signed Q1–Q33 audit plus separate remediation authorization.
Exit is all 18 PASS gates above and this report. The next entry condition is explicit
user confirmation of this report and Phase20 authorization. Phase21 requires completed
Phase20 plus selected transactional storage/fencing/retention design. Phase22 requires
completed Phase21 plus explicit message, ordering, retry/quarantine and replay contracts.

Work stops here. Confirm the prerequisite report before any Phase20 work begins.
