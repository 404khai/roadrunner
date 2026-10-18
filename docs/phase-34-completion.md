# Phase 34 — staged scale investigation

Phase 34 implements the separately authorized [workload plan](performance/phase34-plan.md)
under the accepted [Q1–Q18 contract](performance/architecture-contract.md). It delivers
reproducible scenario generation, headless execution, independent concurrent routing,
bounded external stress execution and honest outcome accounting. It introduces no
engine optimization or new supported-capacity promise.

## Implementation

- `scripts/phase34.py`: immutable graph-qualified generation and isolated execution;
  external wall/RSS/output envelopes, sampled resource records and classified stops.
- `scripts/qualify_phase34.py`: small workloads before A/B/C escalation, warmup and
  repetitions, preserved failed runs and a stop on unsafe/failed tier escalation.
- `scripts/summarize_phase34.py`: matched-source/configuration/hardware summaries,
  deterministic-result checks, raw repeated-run timing, queue-action bucket quantiles,
  completed throughput and visible incomplete outcomes.
- `phase34_workload` example: real engine policies and full evidence serialization;
  independent Dijkstra/A* query threads share an immutable graph and aggregate in
  canonical query order. No parallel authoritative simulation is introduced.
- `simulate_observed`: optional diagnostic observer after serial queue boundaries.
  Only new event records and constant-time cardinalities are borrowed; no frames or
  world copies. Normal simulation has no diagnostic timers/callbacks. Tests compare
  complete serialized results across observation modes and all supported policy families.

[Manifests and reproduction](../benchmarks/phase34/README.md) define all workload and
measurement semantics, including population/arrival distinctions and current gaps.

## Qualification evidence

The [retained evidence index](../benchmarks/results/phase34/index.json),
[environment](../benchmarks/results/phase34/environment.json) and raw run directories
contain 68 executions, full canonical outputs pooled by exact SHA-256, every input
bundle, progress logs, OS resource records and incomplete outcomes. Pooling/gzip is
lossless benchmark packaging; it changes no historical artifact or retention format.
Binary source is `2293f7e2`, release Rust 1.96.0, Apple M3/16 GiB, Python 3.9.6.
The desktop host was not exclusive. No builds/tests ran during timed collection.
There is no cold-cache or production-host claim.

Headless cases have one excluded successful warmup and three measured repetitions;
the failed Tier B warmup remains visible. Routing has three repetitions with no
excluded warmup. All completed repetitions of each deterministic case match full
result hashes. Observed/unobserved smoke results also match.

### Staged population outcomes

- **Tier A: 100 riders / 1,000 orders.** All three measured runs registered,
  assigned and delivered all 1,000 identities, with zero outstanding work. Engine
  execution was 27.435 / 25.598 / 23.464 seconds (median 25.598 s); serialization was
  2.562 / 1.776 / 1.563 seconds (median 1.776 s). Median completed orders per engine
  execution second was 39.066; this is synthetic compute throughput, not service ingress.
  Coarse OS whole-process durations were 30.03 / 27.40 / 25.05 s. OS peak RSS was
  128,024,576 / 163,708,928 / 163,594,240 bytes. These are process measurements,
  not allocation counts. Every full result is exactly 147,094,085 bytes.
- The workload uses a 256-node/960-directed-edge bidirectional grid, Basic dispatch,
  100 initially available capacity-one riders, one arrival every two logical seconds,
  fixed three-second readiness, dispersed node anchors, no traffic changes and a
  2,600-second horizon. All 100 riders recorded positive responsibility exposure during the run.
  Peak outstanding work was 12, peak pending work one; 8,000
  queue entries were processed. This does not qualify 100 simultaneously travelling
  riders, complex fleet optimization, a real city graph or a service SLO.
- **Tier B: 1,000 riders / 10,000 orders.** The external 60-second envelope stopped
  the warmup at 60.218 collector-observed seconds. The last flushed boundary, at
  logical second 504, reported 253 created and 243 delivered identities, 10 outstanding,
  no pending orders, and 19,504 queued entries (mostly future arrivals). It was observed
  at 56.286 execution seconds and may lag the kill. There is no final result or inferred
  final admission/completion total. This is a qualified harness timeout, not graceful
  engine rejection or proof that Tier B is impossible under other resources.
- **Tier C: 10,000 riders / 100,000 orders.** Inputs were fully materialized and
  checksummed. Execution was skipped because Tier B failed the declared envelope.
  No capacity result is claimed.

### Backlog, history, planning and routing

The two-rider fast-arrival/drain case peaked at 24 outstanding/22 pending orders and
completed all 24 by the horizon. The deliberately budget-incomplete insertion case
retained eight outstanding orders and published zero assignments. Insertion, greedy
fleet and traffic-triggered recovery cases completed their eight-order inputs; their
small fixtures do not establish larger planning capacity or VRP optimality.

The matched two-rider history inputs retain 88 versus 723 history records for eight
versus 64 orders; full result size grows from 202,565 to 1,616,672 bytes. This is
retained-history/output growth evidence, not an isolated allocator attribution or
PostgreSQL-aged-state result.

Independent routing uses 128 fixed/repeated pairs on the 64-node smoke graph with
one/four threads. All 12 runs completed every query, with identical algorithm-specific
results across concurrency; A*/Dijkstra objective equality is also tested. Exact query
p50/p95/p99 and throughput appear in the index beside each run. Queries are tiny and
repeat modulo 12 orders; their favorable rates are not city-network or HTTP capacity.
Queue-action processing distributions for real dispatch/planning scenarios use bounded
log2 buckets with explicit upper-bound quantiles, separate from route-request timings.

Smoke observer-on execution times were 2.681/2.716/2.648 ms; observer-off times were
3.407/2.605/2.687 ms. Host/sample noise prevents a defensible causal overhead estimate.
Normal execution does not acquire diagnostic clocks; this does not prove zero overhead
for every added branch. Collector wall time includes polling/bookkeeping and must not
be interpreted as precise process runtime, especially for millisecond fixtures.

### Measured limits and remediation scope

Increasing configured population from A to B on this workload exposes a substantial
compute-throughput limit under the declared envelope. Candidate eligibility scans,
per-candidate route calculations, world/history copying and validation are plausible
source-grounded contributors; these runs do **not** quantitatively rank them. A future
remediation brief needs stage/route/allocation attribution before choosing a rewrite.
The 147 MB Tier A result also makes unchanged full-evidence serialization/storage an
explicit cost. No evidence truncation, changed search policy, new cache, persistent
index or performance rewrite was introduced to make a tier pass.

## Six measurement planes

| Plane | Delivered/qualified boundary |
| --- | --- |
| Core routing | Separate fixed-query Dijkstra/A* runs, serial/concurrent, exact per-query times and full outcomes; no HTTP scheduling/admission claim |
| Dispatch | Real Basic/insertion/fleet/recovery inside coherent scenarios, explicit budgets and queue-action processing buckets; nested routing/allocator attribution remains unavailable |
| Operational API | No new open-loop business-command load generator; no orders/sec service or publication-capacity claim. Existing HTTP caps, fencing and recognition remain |
| Synthetic simulation | Headless generation, stable serial event execution, activity/backlog/queue traces, full retained evidence and external resource stops |
| Persistence/replay | In-memory retained trace/history and serialization growth exercised; PostgreSQL throughput, history-aged SQL restoration and independent anchor fault envelopes remain unqualified. Existing Phase 33 recovery evidence is not relabelled as scale evidence |
| Visualization | Scene/UI limits unchanged and measured separately by existing suites; no thousands-of-markers or browser capacity claim |

## Correctness and limitations

Canonical domain evidence, directed paths, graph/context identity, budget semantics,
protection constraints, incomplete-publication rules and durable single-writer authority
remain unchanged. Source data/scenarios are not regenerated to improve outcomes.
Failure records retain partial observations without inventing final accounting.

Qualified failures expose remediation candidates; they do not authorize spatial-index,
routing scratch, dispatch-neighborhood, retention or persistence rewrites. Real city
networks, exogenous demand realism, open-loop operational load, PostgreSQL contention,
allocator attribution, exact eligibility/travel/service distributions and browser scale
need separately scoped qualification. No service SLO, graceful OOM guarantee, complete
history after a killed run or global VRP optimum is established.

## Verification and readiness verdict

Rust workspace: 321 passed, zero failed, 13 intentionally ignored (12 disposable
PostgreSQL tests and the existing external-collector timing test). Workspace
all-target/all-feature Clippy with warnings denied and rustfmt pass. Phase 34 Python
contracts: 12 passed (including the built release harness); three retained Phase 33
comparison contracts also pass. These cover deterministic generation, corrupted inputs, observed/unobserved
full-result equivalence, policy-specific incomplete publication, concurrent routing,
cost equality, external wall/memory/output stops and lossless retained evidence.
CI exercises those contracts with a built debug harness, without performance thresholds.
Existing durable/browser correctness CI remains enabled; no new large durable or
browser qualification is asserted.

The headless scale investigation is reviewable and reproducible. Tier A qualifies only
its stated synthetic workload; Tier B is resource-incomplete and Tier C unexecuted.
Further scale claims require additional workload/environment evidence. Remediation
and independent operational/durable/rendering qualification remain explicitly scoped
follow-up work, not silently completed phases or an automatic optimization mandate.
