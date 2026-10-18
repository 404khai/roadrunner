# Phase 34 workload qualification

This harness investigates coherent **headless engine work**. It does not raise HTTP
or scene-capture limits, animate decorative fleets, change dispatch policy, or
promise city-scale support. See [completion/results](../../docs/phase-34-completion.md).

## Build and reproduce

Requires a POSIX host with process groups, `ps` and `/usr/bin/time`. Darwin and
Linux timer modes are supported; Windows is not a qualified execution environment.

Build from a clean committed checkout before collecting measurements. The source
argument identifies the binary build, not an unrelated collector checkout.

```sh
cargo build --release --locked -p roadrunner-simulation --example phase34_workload
python3 scripts/phase34.py generate benchmarks/phase34/smoke.json /tmp/rr34-smoke \
  --binary target/release/examples/phase34_workload
python3 scripts/phase34.py run /tmp/rr34-smoke /tmp/rr34-smoke-run \
  --binary target/release/examples/phase34_workload \
  --source-revision "$(git rev-parse HEAD)" --seconds 60 --rss-mib 1024 --disk-mib 256
```

Input bundles and output directories must be fresh; existing evidence is never
replaced. Generation materializes all exogenous inputs **before** execution.
`bundle.json` binds exact manifest/scenario/artifact SHA-256 and semantic graph
identity. Execution verifies those bindings and the supported bundle version.

Run the declared suite with one excluded warmup and three measured repetitions:

```sh
python3 scripts/qualify_phase34.py --binary target/release/examples/phase34_workload \
  --source-revision "$(git rev-parse HEAD)" --output /tmp/rr34-qualification \
  --repetitions 3 --seconds 60 --rss-mib 1024 --disk-mib 256
```

These are external qualification envelopes, not latency targets or engine limits.
A failed warmup is retained. A small-case failure stops qualification; a tier
failure stops escalation to larger tiers. B/C inputs are still materialized and
identified even when their execution is skipped. Do not run compilation, tests or
unrelated benchmark jobs during measurement. Host background activity must be disclosed.

For independent routing, add `--routing dijkstra` or `--routing astar`, `--queries
128` and `--concurrency 1` (or 4). The suite runs these as a separate plane. Threads
share one immutable graph, use fixed pickup/dropoff query pairs and return results
in original query order. This is saturated closed-loop core routing, not open-loop
HTTP traffic, dispatch concurrency or multiple authorities. Queries repeat modulo
the scenario's orders; no rider × order Cartesian product is generated. All queries
use free-flow departure zero; traffic-churn simulation results are separate.

An external schema-4 artifact can replace the grid: set the manifest's `grid` to
`null` and pass `generate --artifact path/to/graph.rr-graph`. The copied artifact is
verified and qualified by its actual identity and node count. Dense-node sampling
can generate unreachable pairs; no failures are regenerated away. Graph validity
is not proof of reachability or representativeness.

## Generator contract

`splitmix64-modulo-anchors/v1` uses integer SplitMix64 with a fixed seed. Node
selection is modulo dense graph nodes (small modulo bias is intentional/versioned),
not uniform geographic or population demand. Hotspot mode samples the first four
node IDs; on the grid these are a short street prefix, not a city neighborhood.
IDs follow input index. Equal-time event ordering is still the engine's canonical
serial queue order. Materialized scenario bytes and graph identity define the run.

Manifests independently specify total/initially available riders, scalar capacity,
total generated orders, arrival family, readiness, horizon, geography, policy/schema,
admission/service terms, budgets and traffic interventions. Fixed readiness is a
synthetic input, never physical telemetry. No input is changed based on an outcome.

Workloads include sustained, burst, explicit fast-arrival/drain horizon, simultaneous
mass arrivals, hotspot/dispersed anchors, traffic replacement, history-aged sequences,
and distinct insertion/fleet/recovery/budget-incomplete cases. History-aged cases
exercise retained history while keeping a small fleet; they are not preloaded durable
history images. Tier A/B/C configure 100/1K, 1K/10K, 10K/100K rider/order populations,
all initially available, Basic policy, 256-node bidirectional grids and 0.5 arrivals
per logical second, followed by a 600-second drain horizon. These parameters are one
workload family, not the universal meaning of those tiers. Other policies remain
small qualification cases; no global VRP optimum is claimed.

## Outcomes and measurement boundaries

`run.json` retains binary/source/harness identities, compiler, host, command, cache
mode, resource samples, stop configuration, outcome and final report when available.
`resources.txt` preserves OS CPU/RSS observations. `progress.jsonl` flushes the first
and every hundredth completed queue boundary; a failed run's last observation can lag
active computation. `report.json` is written only after successful full serialization.
`result.json` retains all canonical evidence. Results are never compacted/truncated.

- Generated orders are scenario population; arrivals beyond the horizon stay uncreated.
- Offered arrivals are configured creation times within the horizon. Successful
  registrations are `created_orders`; accepted dispatch responsibility is
  `assigned_orders` (distinct ever-assigned identities), not number of attempts.
- Delivered, cancelled, pending/unassigned, assigned unfinished and outstanding counts
  retain the engine's existing definitions. Attempts and events are different counters.
- Final lifecycle equations are checked; failure never invents final admission or
  completion totals. The last observed boundary is explicitly partial evidence.
- Queue depth includes scheduled future arrivals. Pending orders and all outstanding
  deliveries are separate. Peaks are evaluated at every completed boundary, including
  the initial pre-pop queue depth. Sampled backlog traces do not imply per-frame writes.
- `riders_with_work` includes travel/wait/service; waiting is separately observed.
  Initial availability is not current eligibility or travelling population. Exact
  travel/service/eligible distributions and nested route counts are unavailable here.
- Queue-action processing histograms use bounded base-2 nanosecond buckets. Their
  p50/p95/p99 are **bucket upper bounds**, not exact quantiles or HTTP latency. They
  exclude observer callbacks but include action processing/capture/recording and
  preserve algorithm counters. Canonical SimulationResult contains no wall timing.
- Execution includes initialization/finish; graph/artifact loading precedes it.
  Serialization is separate. Coarse OS whole-process duration includes load, execution, encoding
  and startup; collector-observed wall also includes polling/bookkeeping overhead. Generation/load/serialization must not be confused with engine time.
- Independent routing retains exact query timings, success/error outcomes and complete
  algorithm-specific outputs. Graph loading and heuristic preparation precede timed
  routing. No time-dependent/cache/shared-service claim follows from this workload.
- Fresh processes use uncontrolled warm OS/filesystem state; no cold-cache claim.
  No route-result cache exists. Quantiles across three repetitions are descriptive,
  not service tails. Throughput accompanies outcomes, backlog and incomplete runs.

The observer is opt-in and read-only, uses constant-time engine cardinalities and
only newly emitted records, and produces no full-world clones or movement sampling.
`--unobserved` removes callbacks/timers; those diagnostic counts are unavailable,
not zero work. Full-result hashes must match observed runs. Instrumentation overhead
is measured by matched observed/unobserved repetitions and disclosed as noisy.

Safety monitors sample isolated process-group RSS/output bytes every ~100 ms; OS
scheduling and monitor cost can delay observations. They can overshoot caps and do
not provide application-wide OOM handling or precise allocation measurements. SIGKILL
terminates only the disposable benchmark process group. Nonzero engine exit,
wall timeout, sampled-memory stop, output stop and completed execution are distinct.
A killed run can have accepted synthetic work but no valid final artifact; do not
interpret it as rejection, infeasibility or complete restoration evidence.

## Retained evidence

[Result index](../results/phase34/index.json) links raw runs and complete outputs.
[Environment](../results/phase34/environment.json) declares hardware/toolchain and
collection scope. Headless warmups are excluded from measured distributions;
independent routing has three measured repetitions with no excluded warmup.
The older raw `qualification.json` uses `delivered_per_wall_second` for collector
wall, including polling/bookkeeping; `index.json` names this denominator explicitly
and separately reports completed orders per engine execution second.

`scripts/archive_phase34.py SOURCE FRESH_DESTINATION` retains every raw run and
input, verifies complete result/scenario checksums and losslessly gzip-compresses
identical JSON bytes once by digest. This is benchmark packaging, not a new historical
archive or truncated record. To inspect a referenced JSON object:

```sh
gzip -dc benchmarks/results/phase34/objects/OBJECT_SHA.json.gz > /tmp/full-result.json
```

Do not load every large result into a browser or a single JSON parser merely to verify
checksums; the CI contract decompresses/hashes bounded chunks. Reproduction regenerates
input bundles from the committed manifests with their exact graph/generator settings.
