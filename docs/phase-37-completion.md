# Phase 37 — Reproducible comparison suite

The [versioned suite](../benchmarks/suite/README.md) unifies all five Phase 37
comparisons without changing domain algorithms, introducing dependencies or creating
a worktree. The CLI probe and standard-library Python collector compose existing
verified graph, routing and simulation producers.

## Delivered

- Dijkstra vs graph-qualified prepared A*, with individual call timing, complete
  routes, canonical expansions, objective equivalence and separate preparation cost.
- Free-flow vs traffic-aware Dijkstra with both paths reevaluated under the identical
  pinned traffic snapshot; the fixture must exercise a real path change.
- Nearest-rider vs lowest-pickup-ETA selection on identical Phase 16 inputs.
- Basic vs preparation-aware dispatch with waiting weight 1 on identical inputs.
- Single-order basic vs exhaustive multi-order insertion on Phase 17 inputs, with
  the explicit legacy schema/admission adaptation required by existing producers.
  This compares policies with different accepted-term contracts, not equivalent
  optimization implementations or universal solution quality.
- Raw per-variant timing samples, median/p95/p99, input and binary fingerprints,
  source/toolchain/hardware metadata, deterministic replay, matched materialized
  inputs, simulation throughput rates and independently reconciled order outcomes.
- Existing-target refusal and failed-run qualification; timeouts cannot masquerade
  as successful engine refusal or zero-work results.
- CI smoke collection and nine contract tests, with no timing thresholds.

## Evidence and reproduction

[Retained run](../benchmarks/results/phase37/developer-debug/index.json) records 11
measured samples per variant after excluded warmups. Its exact source revision,
operator-declared build command, rustc, executable hashes, graph identity, scenario
seeds and hardware are retained in that file. Result JSON and complete materialized
inputs are adjacent. The collector reverses variant order on successive rounds.

The [suite methodology](../benchmarks/suite/README.md) documents all reproduction
commands and measurement boundaries. The retained run uses the existing **debug**
build on a developer workstation without CPU pinning/isolation or controlled OS
caches. It is a correctness-qualified development baseline, not an optimized-speed,
production SLO, service-throughput or large-scale claim. Process rates describe
headless synthetic executions only. Memory, allocator and CPU-utilization
measurements are explicitly unavailable.

The Phase 16 case retains future/uncreated orders and unreachable outstanding work;
no aggregate “processed orders” counter replaces its lifecycle evidence. The pooled
case retains all insertion evidence, budgets and accepted effects. No global VRP
optimality or equal protection constraints are inferred across different policies.

## Verification

```bash
cargo fmt --package roadrunner-cli -- --check
CARGO_INCREMENTAL=0 cargo +1.99.0 clippy -p roadrunner-cli --all-targets --locked -- -D warnings
python3 -m unittest discover -s scripts -p test_benchmark_suite.py -v
python3 scripts/benchmark_suite.py verify benchmarks/results/phase37/developer-debug
```

The new probe compiles with existing dependencies. Nine tests exercise a real
bounded suite and adversarial artifacts. The Phase 36 workflow tests also passed
after the CI-specific Clippy assertion fix. Full workspace/web/PostgreSQL
regression verification is delegated to the PR's existing required CI jobs;
no additional full target/release build was created locally. User deployment/API/web
changes remain outside this branch's commits.

## Remaining limits

One small road-network query, two small simulation fixtures, prepared A* (not its
preparation charged to each call), sequential execution, uncontrolled background
load, and descriptive small-sample tail statistics. The verifier checks hashes,
matching, summary calculations and selected domain invariants; it is not an
independent proof of all routing/dispatch correctness. Current inputs/lockfile must
match recorded fingerprints to verify a historical result; use its source revision
when repository fixtures change. Engine tests and larger qualified Phase 33/34
workloads remain essential. This phase does not authorize another optimizer rewrite,
new dispatch semantics or relaxed evidence/durability guarantees.
