# Phase 26 producer-conformance review evidence

**Verdict: implemented and verified for the finite producer package; ready for producer-conformance review.**
Architecture Gate A and implementation Gate B were explicitly authorized. Gates C (real client
consumption) and D (visual fidelity) remain unexecuted. Phase 27 was not started. Work is on the
app-bound `feat/visualization-evidence-contracts` branch/worktree, based on
`0ef8c93156c03bb46ca7162c103eec4062b96c5f`; the parent checkout was not modified.

The full source-thread texts were recovered with itemId/textOffset pagination through final pages,
not inferred from previews. [Source provenance](phase-26-source/provenance.json) records the five exact
item IDs/positions and captured text hashes. Their historical pending labels are superseded by
[item 122 authorization](phase-26-source/authorization.md).

## Implemented package

* Core `routing/geometry.rs`: independent full-digest gate, directed topology/E7 validation,
  occurrence/segment/orientation association, full point multiplicity, explicit boundaries,
  repeated traversals, no-travel, typed invalid/unavailable/unsupported/limit outcomes and preallocation limits.
* Core `routing/evidence.rs`: scoped path/evaluation identity, full producer-supplied original routing
  context, profile/jurisdiction, typed original objective/elapsed/distance, exact family summaries,
  mandatory-semantics preflight and explicit unavailable cost breakdown/trace capabilities.
* API `routing.rs`/`http.rs`: separate version-1 portable route and alternative inspection endpoints;
  selected-set/member identities and original alternative termination/work; legacy provenance parity
  and independent graph guard. Read authorization, vertex/byte limits, separate export admission,
  worker serialization, chunked body lifetime admission, explicit 413/503, capture-lock timing.
* Dispatch `inspection.rs`: existing basic, insertion, fleet and recovery evidence qualified by input,
  coverage, completion, retention, comparator, quality and recommendation stage. Runtime command
  evidence attaches metadata without replacing original decision/publication evidence. Incomplete
  insertion/fleet/recovery proposals remain suppressed. No optimizer search code was changed.
* API construction (`lib.rs`, `persistence.rs`) initializes export admission in volatile/durable modes.
  No SQL migration, stored-record rewrite, event-recognition change or historical resolver was added.
* [Contract](visualization-contract.md), [ADR 0023](adr/0023-checked-visualization-evidence.md),
  [schema](schemas/route-evidence-v1.schema.json), architecture/API/README/AGENTS reconciliation,
  adversarial tests, independent oracle and reproducible resource collectors.

## Fresh verification

[Structured execution record](evidence/phase-26-verification.json) contains commands, source hashes and
actual test counts. [Claim-linked proof matrix](evidence/phase-26-proof-matrix.json) maps every applicable
§25 geometry/decision/summary/compatibility/resource case to its oracle, fixture, owner and qualification.

| Check | Actual result |
| --- | --- |
| Baseline `cargo test --workspace --all-features --locked` | 286 passed, 0 failed, 13 opt-in ignored |
| Final same workspace command | 303 passed, 0 failed, 13 opt-in ignored |
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | passed |
| `git diff --check` | passed |
| Opt-in PostgreSQL storage tests, serial | 5 passed |
| Opt-in PostgreSQL event tests, serial | 7 passed |
| Real process crash/restoration suite | all 7 explicit cases passed |
| Independent Draft202012 schema/E7/SHA/Haversine oracle | verified |

The workspace's remaining pre-existing ignored dispatch benchmark evidence writer is not claimed run.
The 12 opt-in DB tests were explicitly executed against a dedicated disposable local database, using
PostgreSQL 18.4 / PostGIS 3.6.4. An initial DB invocation omitted the host and failed connection
configuration; the corrected Unix-socket invocation passed. [Actual process evidence](evidence/phase-26-postgres-runtime.json)
records binary/dataset hashes and frozen restoration, recognition, rollback, lost-response and expiry cases.
The schema oracle used jsonschema 4.25.1 in a temporary venv; no Python production dependency was added.

New independent proofs include two different full graph digests sharing dense IDs, literal full
forward/reverse/repeated E7 sequences, exact boundaries and duplicate vertices, actual builder/artifact
roundtrip, production compilation quarantine, explicit no-travel, invalid topology, unavailable graph,
actual single/alternative traffic/time/profile/objective parity, access-context identity distinction,
integer counters above 2^53, hand-traced stale/goal pops, source shortcut/heuristic invocation behavior,
and body/admission/byte bounds. Existing independent Bellman-Ford, insertion/fleet/recovery and accepted
protection oracles were freshly executed; inspected grill coverage was never credited as passing evidence.

The maneuver fixture has four graph nodes and five accepted incoming-qualified state pops. Equal-objective
A* candidates do not replace labels or acquire fictional heuristic calls. Current supported built-in
heuristic policies are consistent: a repeated-node reopen fixture is not applicable to those policies.
No inconsistent policy/new model was invented to manufacture history. A* still has no permanent closed
set, and summaries never claim unique/settled nodes. A future admissible inconsistent policy needs its
own proof and repeated-processing fixture. Detailed numeric g/h/f/frontier history remains deferred.

## Findings and disposition

| Finding | Reproduction / reachability | Disposition / owner |
| --- | --- | --- |
| R1 builder vs artifact zero distance | Confirmed actual builder -> freeze -> encode -> decoder rejects `ordinary road segment distance must be positive`; normalized-artifact compiler quarantine freshly verified | **Contained**, not silently fixed globally: checked export rejects ordinary zero-length traversal; no-travel accepted. Foundational builder/artifact harmonization remains graph-maintenance work requiring its own scope |
| R2 missing independent digest guard | Established precondition gap; no current wrong-graph API output was demonstrated | Resolved for reusable checked producer and guarded legacy API helper; storage's single-graph helper is not promoted to a general resolver |
| R3 display flattening loses boundaries | Confirmed serializer behavior | New canonical representation preserves all E7 vertices/occurrences; legacy display line retained |
| R4 single/alternative provenance asymmetry | Confirmed against captured producer inputs | Resolved in portable evidence and additive legacy alternative objective/traffic fields |
| R5 exact summary evidence | Required proof gap | Fresh exact fixtures and independent existing work/quality tests; consistent-policy qualification above |
| R6 full `/v1/state` lock/payload risk | Existing producer unchanged | Documented later fleet-view risk; no snapshot redesign claimed |
| R7 mixed-graph historical resolution | Durable adapter still single-graph | Portable exports are embedded; missing originals unavailable. Resolver service remains conditional/deferred |
| R8–R11 stream/seek/guidance/scene/access lifecycle gaps | Existing capability limits | No unsupported upgrade; later authorized producers/clients own these |
| R12 stale no-durability architecture prose | Confirmed documentation contradiction | Reconciled with actual optional persistence; history not rewritten |

None required changing routing costs, objectives, heuristic proof contracts, accepted protections,
frozen execution, publication rules or recognition semantics. No tracked parent changes or user reference
assets were altered, and no assets, UI, PR or external publication were created.

## Compatibility and readiness

Legacy requests/route fields remain. Alternative objective/traffic and command inspection fields are
additive; strict response parsers must explicitly accept these documented extensions. New canonical
output uses separate endpoints and schema/identity versions. No old record is relabelled or backfilled.
Unsupported required geometry/authority/objective meanings reject; optional extensions may degrade.
JSON integer preservation is explicit; browser parsers must avoid number rounding above their exact range.

| Layer | Implementation / conformance | Capability |
| --- | --- | --- |
| Route contract, directed geometry, scoped identity | implemented / verified | supported within declared node-anchored limits |
| Cost/objective observability | implemented / verified | supported source metrics; breakdown unavailable |
| Existing decisions / selection | implemented / verified | retained granularity; no candidate archive inferred |
| Search summaries | implemented / verified with stated policy scope | summaries supported; detailed traces deferred |
| Movement/timeline compatibility | documented; extra producer out of scope | existing endpoint/execution facts; interpolation deferred |
| Guidance | out of scope | unavailable |
| Coherent whole-state viewing | existing producer unchanged | limited; payload/lock risk remains |
| Incremental streaming/resume | out of scope | deferred; no certified cursor cut |
| Historical inspection | existing original records only | limited single-graph dependency availability |
| Deterministic movement seeking | out of scope | deferred |
| 2D renderer data | new portable evidence verified | data supported; real client gate unexecuted |
| 3D scene data | geographic evidence only | elevation/alignment/scene pipelines deferred |
| Motorcycle assets | reference-only | models/rigs/customization deferred |
| Phase26 producer package | implemented, tested, documented | ready for conformance review |

Phase27 owns map foundation and initial route reveal; 28 alternative comparison; 29 graph/summary
inspection; 30 courier movement/assets; 31 discovery/selection; 32 supported historical presentation.
Cinematic reveal is result presentation, never journey/search progress. Geographic pulses need original
proven center/radius/Haversine units/eligible population/limit; final candidate order is not discovery
history. Missing ordered leg road geometry cannot become straight connectors. Guidance, telemetry,
continuous interpolation, detailed tracing, streaming/scoped projections and general resolvers require
later explicit scope. The addendum's actual visual, reduced-motion, mode-switch and performance tests
remain unexecuted future client obligations.

## Measured resources and reproducibility

[Structured samples](../benchmarks/results/2026-10-16-phase26-visualization.json) retain raw timings, hardware,
graph/dataset/binary identities, configuration, runs, median/p95/p99, payloads and process resource output.

Hardware: Apple M3, 8 logical CPUs, macOS-27.0-arm64-arm-64bit;
rustc 1.96.0 (ac68faa20 2026-05-25), optimized release. Deterministic synthetic chain workloads; three warmups and
31 measurements per core/HTTP case. Build/routing are excluded from core export timings.

| Exported E7 vertices | Payload bytes | Checked export ms median / p95 / p99 | JSON encoding ms median / p95 / p99 |
| --- | --- | --- | --- |
| 400 | 31,439 | 0.0053 / 0.0066 / 0.0072 | 0.0315 / 0.0328 / 0.0338 |
| 16,000 | 776,758 | 0.0401 / 0.0481 / 0.0658 | 0.6877 / 0.7258 / 0.8015 |
| 160,000 | 7,138,966 | 0.2517 / 0.4151 / 2.0145 | 6.5592 / 18.8643 / 27.6525 |

HTTP uses the 5,001-node / 5,000-edge / 160,000-vertex graph, new loopback urllib connections and
4 configured workers. It measures routing, worker JSON construction/encoding and transfer, excluding
client JSON parsing. Static routes are followed by an adopted 1,000-edge traffic overlay at multiplier 1.5.

| HTTP operation | ms median / p95 / p99 |
| --- | --- |
| dijkstra | 72.0916 / 94.6833 / 148.2543 |
| astar | 86.9663 / 215.7477 / 222.6552 |
| astar_traffic_1000_overrides | 87.7443 / 132.3533 / 134.9771 |
| authority_without_export | 0.2487 / 0.7750 / 1.5138 |
| authority_with_export | 0.4230 / 0.6656 / 1.0436 |

Capture debug instrumentation recorded 103 context captures. Lock hold
median/p95/p99: 83/5,500/33,541 ns; lock wait:
125/375/625 ns. This is the brief traffic-context clone, including
free-flow and overlay calls; it is not whole-state snapshot construction. No JSON or transfer occurs under that lock.

Whole-example process peak resident size: 281,362,432 bytes (268.33 MiB),
measured by macOS `/usr/bin/time -l`. This includes graph building, routing, export samples and graph-artifact
encoding; it is **not** exporter-only allocation or API RSS. CPU/process timing is retained verbatim in JSON.

The concurrent authority comparison completed 1 large export(s) while creating 31 riders;
it tests this declared overlap only, not saturated sustained export throughput or causal latency attribution.
Client/server share the host. Final samples were collected after conformance processes finished.
No production latency/memory/fleet-size guarantee follows from these results.


Reproduce after building:

```sh
cargo build --release --locked -p roadrunner-api -p roadrunner-core --bins --examples
python3 scripts/collect_visualization_benchmark.py --output /tmp/phase26-visualization.json
python3 -m venv /tmp/phase26-schema
/tmp/phase26-schema/bin/pip install jsonschema==4.25.1
/tmp/phase26-schema/bin/python scripts/verify_visualization_contract.py --output /tmp/phase26-oracle.json
```

Output limits are admission choices, not benchmark-derived scale claims: 20,000 occurrences and 200,000
vertices per route; at most five returned routes; 32 MiB response; active export slots equal configured
workers. Explicit saturation/oversize tests pass. Slow-body tests verify a held response retains only
export admission, with operational workers available, and drop releases admission. Allocation failure
is not claimed recoverable. Existing full-state snapshot growth/lock cost, real fleet concurrency,
rendering, physical telemetry, provider scenes and detailed tracing remain unmeasured/out of scope.

## CI toolchain compatibility correction

CI's Rust 1.99 introduced `clippy::assert_is_empty`, which rejected two test assertions that passed
original local Rust 1.96 verification. Both now use equality assertions showing the actual collection;
no lint is suppressed and production behavior is unchanged. The full workspace/all-target/all-feature
Clippy command passes on 1.99, as do the affected 23 routing and 4 visualization API tests.
[Supplemental evidence](evidence/phase-26-clippy-compatibility.json) records the corrected source hashes
and actual commands; the original verification artifact retains its original implementation evidence.
