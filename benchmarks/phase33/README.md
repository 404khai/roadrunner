# Bounded Phase 33 probes

These are versioned **profiling inputs**, not a Phase 34 city generator or capacity
claim. JSON is the exact manifest; each run records its checksum and semantic graph
identity. Deterministic synthetic grids have 1m-scale E7 spacing, bidirectional
orthogonal roads, no prohibited turns but maneuver-capable graph metadata. They are
controlled topology, not real street/demand representativeness. The pinned artifact
case loads the existing 58-node Phase 10 road fixture and queries its first traversal.

- Graph: clone, encode, decode+verify on a 1,024-node grid.
- Routing: 25 repeated prepared A*/Dijkstra calls on 256/1,024-node grids and the
  pinned road artifact. Preparation is separate. Expanded-state counters are original.
- Candidates: 64 fresh riders, or 128 riders with 256 retained delivered orders;
  five exhaustive and five spatial eligible/build/query calls, plus isolated
  validation/snapshot timings. Actual eligibility and Basic Dispatch policy apply.
- Simulation: 1/2 riders and 16/32/128 timed orders, or a bounded 8-order scene.
  Seed 33, fixed readiness at creation, 100 logical seconds between arrivals, real
  engine-selected assignments and full retained outcomes. Churn replaces edge 0's
  factor at 150s. The manifest/code jointly define generation; no failures are filtered.

The same generated scenario bytes and complete serialized SimulationResult are hashed
before/after; simulation load and output serialization are separately timed. Routing
and candidate probes hash their deterministic outputs. Every repetition must agree.

Probe input guards (grid side 2..64, riders 1..256, orders <=256, retained orders
<=1,024, repetitions 1..100) are harness safety boundaries, not new engine limits.
The collector's 120s per-process timeout is an external failure boundary; timeout or
nonzero exit aborts collection instead of producing a successful partial artifact.

## Reproduction

Use Rust 1.99.0 for the retained measurements. Build outside measurement. Source
revisions must identify the binary's build, not just the collector checkout.

```sh
cargo +1.99.0 build --release -p roadrunner-simulation --example phase33_probe
python3 scripts/collect_phase33.py \
  --binary target/release/examples/phase33_probe --source-revision "$(git rev-parse HEAD)" \
  --runs 7 --output /tmp/phase33-candidate.json
```

To reproduce the pre-optimization control, create a separate checkout at `d265c05`,
build that same example there, then invoke the **current collector** from this
checkout with that binary and `--source-revision d265c05`. Keep compilation/checks out
of collected intervals. Validate comparisons:

```sh
python3 scripts/compare_phase33.py benchmarks/results/phase33-control.json \
  benchmarks/results/phase33-candidate.json --output /tmp/phase33-comparison.json
python3 -m unittest discover -s scripts -p test_phase33.py -v
```

Canonical measurements are `phase33-control.json` and `phase33-candidate.json`;
`phase33-comparison.json` derives its values from those raw samples. The earlier
`phase33-baseline.json` and validation-only pass record tranche selection/intermediate
experiments. Their `baseline_revision` means collector HEAD; their tracked diff is
collection-checkout state, not a substitute for the copied binary identity. Control
and candidate records explicitly separate binary source and collector revisions.

## Measurement semantics

Outer clocks record stage elapsed ns, never domain time or canonical evidence. No
profiling observer is added to production routing/dispatch hot loops. The diagnostic
flag controls a post-execution traffic-buffer inventory; ordinary library calls have
no such inventory. Modes in `../phase33-modes/` generate identical semantic results.

`traffic_vector_capacity_bytes` counts unique live vector addresses and element
capacity × element size. It is an exact payload-capacity inventory, **not** total
allocator bytes, RSS, or logical output size; Arc/allocator headers and other objects
are excluded. Process RSS includes graph preparation, state, encoding, result bytes
and diagnostics. Coarse process user/system seconds are retained where the OS timer
reports them; this is not per-query CPU utilization or allocator attribution.

One excluded process warmup per case; no graph/result cache; process/filesystem/OS
caches are not cleared. No cold-cache claim. The host was not an exclusive benchmark
machine; background activity and sample variation limit interpretation. Quantiles
are descriptive interpolation of seven stage/process observations, not p95/p99 of
individual service requests. Counter names preserve existing semantics.

No new database/anchor timing, global allocation profiler, request admission,
cooperative cancellation, scratch reuse, spatial persistence, shared route cache,
archive, retention expiry, worker parallelism or dispatch policy is delivered here.
These remain explicit scope/measurement gaps, not implicit completeness claims.
