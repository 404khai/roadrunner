# Phase 20 completion

Implemented after the merged pre-20 gates (`681cd64`) and explicit Phase 20 authorization.
The new `roadrunner-api` workspace crate provides Axum/Tokio HTTP composition, generated
OpenAPI 3.1 and vendored Swagger UI. The [run guide](../crates/roadrunner-api/README.md)
is the complete wire/trust/time/outcome contract.

## Delivered boundary

The original nine endpoints are implemented: route, alternatives, order creation/query,
rider creation/location, dispatch, delivery query and simulation. The approved expanded
runtime boundary also has state/rider queries, fleet admission, recovery, cancellation,
readiness/forecast updates, static traffic adoption, explicit action start/wait/service/
arrival/completion/delay and principal-scoped command lookup. There are 24 documented
paths; `/v1/orders` and `/v1/riders` each share creation and resource-query families.

The API depends on core, dispatch and simulation. It does not change domain APIs,
algorithms, cost semantics, graph artifacts or shared simulator behavior. Read-only routes
use Roadrunner's Dijkstra/A*, bounded alternatives use its Yen implementation, and full
segment geometry honors traversal orientation. Explicit graph digest/node/coordinate
projections reject old snapshots and coordinate mismatches. No snapping, external
routing service, authentication bypass or operational World replacement is introduced.

One coordinator mutex jointly contains authoritative OperationalState, anchors, selected
server policies/forecasts, current static traffic, command reservations/results and source
observation recognition/order. Evaluation captures detached state and pinned inputs;
search/certification and route-to-next-stop run outside that writer on bounded blocking
workers. Exact source/adoption/clock/proof checks precede dispatch publication. Business
effects and terminal command recognition are visible together under the writer.

Bearer credentials bind stable principals, separate read/write/simulation capabilities,
observation source and optional resource allowlists. Lookup checks original resource scope
as well as principal identity; a narrower credential cannot retrieve a broad command result.
The binary intentionally exposes only explicit trusted-local loopback mode. It is a local
driver, not a physical-device attestation system. An embedding host provisions actual trust.

Mandatory namespace/principal/key recognition binds parsed canonical intent, including
normalized defaults, sorted traffic/fleet intent and versioned simulation normalization.
The spawned worker owns an accepted reservation even if its response waiter disconnects.
Pending retries return the same command identity; terminal retries return the original
status/result. Conflicting key/source identity reuse rejects. Saturation refuses new work
before reservation while existing recognition remains available.

The clock samples UNIX origin once and advances by monotonic elapsed time. Evaluation
and commit timestamps stay distinct. The local driver requests a two-second endpoint,
which dispatch must prove under `static-road-monotone/v1`; it is not an unconditional TTL.
Stale/expired proposals reject wholly. Forecasts never become physical observations and
timers never complete service. Started actions and accepted terms remain domain-owned.

Observation source/resource sequences begin at 1, detect gaps/regression/supersession,
reject future instants, and retain receipt separately. Same-source identical identity is
recognized independently of command keys. Domain action-effect recognition additionally
handles identical effects across different commands and observation identities. Arrival,
service and pickup/dropoff completion use shared OperationalState transitions. Generation
checks preserve delayed/frozen work. Cancellation refuses custody/started obligations.

Simulation runs existing schemas 1–4 on the pinned graph in an isolated simulation
authority and independent clock, with no live-world/execution/publication mutation.

## Verification and controlled histories

The tests verify these initial-state/interleaving/fault scenarios:

- Empty fixture namespace `[1;16]`, clock 100: simultaneous same-key rider creation has
  one command identity, one rider and one committed transition. Meaning conflict rejects.
- Lost response: a clock gate pauses a worker after reservation; abort the response waiter;
  release it; retry/lookup resolves the committed original result with one rider/history effect.
- Competing assignments: two detached certified evaluations adopt equal content at different
  operational revisions. The older one rejects; exactly one assignment/publication remains.
- Observation versus search: pause after certification, publish a trusted offline observation,
  then attempt the old certificate. Reject with no assignment and unchanged post-observation
  state/revision. Repeat with traffic adoption; post-adoption state/contexts remain intact.
- Time: certify at 100, publish at 101 and retain distinct evaluation/commit samples. A later
  certificate expires before publication; world, execution, publications and revision stay exact.
- Execution: create without admission, dispatch, observe readiness, start/pin action, observe
  arrival, start configured service, observe pickup/dropoff completion. Identical completion
  across a distinct command/observation identity is recognized without another business revision.
- Capability/resource/source rejection, future/gapped/conflicting observations, graph mismatch,
  disconnected routing, reversed intermediate geometry, Dijkstra/A* cost equality, fleet/recovery/
  cancellation/traffic, worker refusal, source/principal lookup, and simulation isolation.
- OpenAPI resolves all component references and serves typed request/route/envelope schemas;
  vendored Swagger HTML serves successfully. The native collaborative browser loaded the
  release server's Swagger UI, displayed OAS 3.1 and operations, with no console errors.
- The process entry point rejects implicit trust, non-loopback binds and missing credentials.

Validation commands:

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
cargo +1.99.0 build -p roadrunner-api --release --locked
python3 scripts/collect_http_api_benchmark.py
```

The [verification record](evidence/phase-20-verification.json) records passing workspace checks: 233 tests, including 15 adapter tests. The existing ignored operational
measurement test retains its explicit opt-in status; no tests were disabled or converted
to ignored tests. Existing CI discovers this workspace member without a new workflow.

## Measured evidence

[Raw HTTP measurements](../benchmarks/results/phase-20-http.json) record hardware,
release compiler/binary digest, graph file/hash/semantic digest, size, request pair,
configuration, warmups, 30 runs per workload, raw latency, median/p95/p99 and route
expanded states. The collector starts a fresh loopback authority and checks that
recognized creation retries produce exactly the original results and one effect.

These are client + HTTP + JSON + adapter measurements on a 58-node/114-edge fixture,
with one tiny routing pair and sequential requests. CPU usage and peak memory are
explicitly unmeasured. They establish a reproducible adapter baseline, not route
scalability, production throughput, an optimization claim or an SLO.

## Declared limits

This phase is volatile and single-process. Discarded namespace outcomes are unknown;
lookup requires the original namespace and never represents old work as proven noncommit.
Internal worker failure retains pending/unknown recognition and poisoned authority fails
closed. No database, fencing, restart restore, outbox/inbox, broker, automatic repair or
external-effect exactly-once guarantee is claimed. Phases 21/22 remain separate.

The live graph stays fixed. Operational routing/publication uses free flow or explicitly
adopted static traffic. Time-dependent operational certificates and unknown active-prefix
proof retain prerequisite applicability limits; the API does not invent a proof. General
source reconciliation/quarantine and physical source integration are not broker services
in this adapter. Historical inspection returns recorded library evidence; recomputation
still requires retained compatible artifacts. Opaque resource references and string
revisions are the command interface; nested u64 domain evidence must not be round-tripped
through JavaScript numeric identities. Full visualization contracts remain Phase 26.
