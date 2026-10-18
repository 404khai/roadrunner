# Versioned comparison suite (Phase 37)

[`manifest.json`](manifest.json) defines five comparisons using existing engine
algorithms and pinned repository inputs. Paths resolve from the repository root.
The collector never downloads datasets or builds executables.

| Comparison | Qualified inputs and variants | Measurement |
| --- | --- | --- |
| Dijkstra vs A* | 58-node, 114-directed-edge Lagos Marina bundle; same OSM endpoints, motorcycle free-flow travel-time objective | Individual in-process calls including scratch initialization and route reconstruction; A* heuristic prepared separately |
| Static vs traffic-aware | Same graph/endpoints; free-flow Dijkstra vs static-traffic Dijkstra, with one pinned severe edge override | Individual calls plus both routes evaluated under the **same** traffic snapshot |
| Nearest-rider vs pickup ETA | Phase 16 paired-strategies graph, seed, riders, arrivals, readiness and horizon; `nearest_rider` vs `lowest_pickup_eta` | Whole CLI process and full simulation outcomes |
| Basic vs preparation-aware | Same Phase 16 input; `basic` vs `preparation_aware`, waiting penalty 1 | Whole CLI process and full simulation outcomes |
| Single-order vs multi-order | Phase 17 successful-pooling graph, seed, rider capacity, orders and horizon; legacy `basic` vs exhaustive `multi_order`, submission budget 1,000 and forecast validity 1,000s | Whole CLI process, outcomes and insertion submissions; distinct accepted-term contracts below |

## Reproduce without another target directory

From the repository root, reuse the existing debug cache with incremental compilation
disabled (appropriate for the storage-constrained development checkout):

```bash
CARGO_INCREMENTAL=0 cargo build --locked -p roadrunner-cli --bin roadrunner --example routing_comparison
python3 scripts/benchmark_suite.py collect \
  --binary target/debug/roadrunner \
  --routing-binary target/debug/examples/routing_comparison \
  --build-profile debug \
  --build-command 'CARGO_INCREMENTAL=0 cargo build --locked -p roadrunner-cli --bin roadrunner --example routing_comparison' \
  --runs 11 --timeout 30 --output /tmp/roadrunner-comparisons
python3 scripts/benchmark_suite.py verify /tmp/roadrunner-comparisons
```

The output directory **must not already exist**. Build commands/profiles are
operator declarations, not values inferred from arbitrary executable bytes. Binary
hashes, source revision, collector/probe hashes, rustc version, lockfile, dataset
hashes, hardware and Python version are recorded separately. Build from the recorded
revision/profile/toolchain to reproduce the same source; elapsed samples will vary.
For optimized measurements, explicitly build **both** binaries with `--release` and
change both paths, profile and recorded command. Do not compare debug and release
samples. No build, download or worktree is implicit in collection.

[Retained debug result](../results/phase37/developer-debug/index.json) is a small-fixture
engineering baseline, not a release-speed or supported-scale result.

## Pairing and correctness

Each producer performs one excluded warmup and is then replayed for every measured
sample. Simulation JSON must match its warmup canonically, including all original
domain evidence; routing evidence must match after removing only timing/order
fields. Variant order reverses on alternating rounds to reduce fixed-order bias.
This does not eliminate OS scheduling, thermal or cache effects.

Routing loads/deep-verifies the bundle and graph-qualified traffic before timing.
The probe measures heuristic preparation separately, excludes it from A* search
samples, and warms each query once within the process. There is no route cache.
The OS filesystem cache is uncontrolled; these are not cold-cache measurements.
Expanded-state semantics remain the core's non-stale-pop convention.
A*/Dijkstra free-flow costs must agree within 1e-9 absolute/1e-12 relative tolerance;
identical traversal paths are not required. Traffic-route objective must equal its
reevaluated traffic time and cannot exceed the free-flow route's reevaluated time.
The pinned fixture must actually exercise a path change.

Simulation variants preserve the same exogenous inputs. The single/multi family
**explicitly** adapts schema 2 to schema 1 and removes `admission` terms for the
legacy variant, because the existing legacy producer rejects those terms. The
pooled variant retains all original terms. This is a behavioral policy comparison,
not a search-implementation speedup or proof of equal protection constraints.
The exact materialized inputs are retained and independently checked against that
sole declared adaptation. No domain implementation is altered for the benchmark.

Accounting derives created, assigned, completed and unfinished populations from
unique order identities and retained milestones, then checks existing summaries.
Rider identities, seeds, horizons and paired graph digests are checked. Incomplete
insertion evidence cannot publish a proposal. These fixtures have no cancellations;
the verifier refuses unsupported cancellation accounting rather than inventing a
universal conservation formula. Full traces retain failed/unassigned decisions,
future events, capacities, traffic, policy evidence and accepted effects.

## Results and limits

`index.json` retains raw nanosecond samples, run count, median, p95 and p99 using
linear interpolation, independently per variant. Percentiles from 11 samples are
descriptive tails, not established p99 service latency. JSON replay files and each
materialized simulation input are SHA-256 bound. Repository input hashes and
semantic checks can be reverified without executing the engine; actual replay
requires a fresh collection. Verification is not a signature or an independent
proof of every engine algorithm.

Routing timings exclude graph loading, verification, traffic construction and JSON
serialization. A* preparation has its own raw distribution. Simulation timings
include process startup, inline graph construction, validation, execution,
serialization and capture. Never compare those boundaries as if both were route
latency. Rates include completed orders and events per wall second, simulation
seconds per wall second, and final outstanding work; they describe synthetic whole
runs, not live service admission/dispatch throughput. Arrival rates, deadlines,
traffic, budgets, rider capabilities and duration remain in the retained inputs.

A process timeout/nonzero result or semantic mismatch fails collection and writes
an incomplete index/failure record. No failed run becomes a successful zero-work
sample. The harness timeout is an external process control, not engine admission,
cancellation, OOM or graceful-overload behavior. There are no timing pass/fail gates.
Memory/RSS, allocations, CPU utilization, confidence intervals, concurrent service
backlog, PostgreSQL, restoration, historical replay and frontend rendering are
unmeasured here. Larger-network and fleet experiments remain separately qualified
[Phase 33](../phase33/README.md) / [Phase 34](../phase34/README.md) evidence.

## CI contracts

```bash
python3 -m unittest discover -s scripts -p test_benchmark_suite.py -v
```

CI builds the CLI/probe and exercises a two-sample smoke collection, replay, matching,
identity/count accounting, overwrite refusal, hash tampering, changed inputs and
external timeout classification. Ordinary CI checks correctness; controlled
measurement runs substantiate performance claims.
