# roadrunner-api

Phase 20 Axum/Tokio adapter over Roadrunner's graph, dispatch and simulation libraries.
It owns one volatile operational namespace and a serialized publication lane. Algorithms,
execution validity and temporal certification remain in `roadrunner-dispatch`/core.

## Run locally

From the repository root:

```bash
export ROADRUNNER_API_TOKEN="$(openssl rand -hex 32)"
cargo +1.99.0 run -p roadrunner-api -- --trusted-local \
  data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph
```

Open **http://127.0.0.1:3000/swagger-ui/**. The UI and assets are vendored; it does
not need a CDN. Use **Authorize** with the token, then `GET /v1/state` to discover
the namespace, graph digest, clock and existing references. The OpenAPI 3.1 document
is at `/openapi.json`. Export it without a graph, credential or running listener:

```bash
cargo +1.99.0 run -p roadrunner-api -- --openapi > /tmp/roadrunner-openapi.json
```

`ROADRUNNER_API_BIND` defaults to `127.0.0.1:3000`. This entry point rejects
non-loopback addresses and requires explicit `--trusted-local`. Its one stable
principal is `local-operator`, with read/write/simulation permissions and trusted
observation source `trusted-local`. This is an explicitly trusted experiment driver;
it is not a physical device verifier or a deployment credential service.

The embedding `Api` library accepts stable principals, separate read/write/simulation
capabilities, credential-bound observation sources and optional resource allowlists.
Namespace-wide creation, snapshot, planning, traffic and simulation reject scoped
credentials. A host application owns provisioning, credential rotation and real
source validation before granting observation capability.

## Route example

The committed semantic-v2 Lagos fixture has an edge from node 0 to node 1:

```bash
curl -s http://127.0.0.1:3000/v1/state \
  -H "Authorization: Bearer $ROADRUNNER_API_TOKEN"
```

Copy `graph.snapshot_digest` into both explicit locations:

```json
{
  "origin": {
    "graph_digest": "<graph.snapshot_digest>",
    "node": 0,
    "coordinate": {"latitude": 6.5232933, "longitude": 3.3801305}
  },
  "destination": {
    "graph_digest": "<graph.snapshot_digest>",
    "node": 1,
    "coordinate": {"latitude": 6.5216648, "longitude": 3.3807667}
  },
  "algorithm": "astar",
  "objective": "travel_time",
  "departure_time": "2026-10-10T09:00:00Z"
}
```

POST to `/v1/routes`. Algorithms are `dijkstra` and `astar`; objectives are
`distance`, `travel_time`, and `traffic_aware` (requires a traffic adoption).
`/v1/routes/alternatives` accepts `alternatives` from 1 through 5 and returns bounded
Yen enumeration with diversity/termination evidence. Its algorithm is always Yen; omit `algorithm` or use `dijkstra` for its inner searches.
Routes contain full directed segment geometry as GeoJSON `[longitude, latitude]`,
node/edge identities, cost units, travel seconds, expanded states and graph identity.
There is no coordinate snapping or off-road connector; coordinates must exactly
match the specified graph node. A zero-length route has two identical GeoJSON positions.

## Commands and execution

Every command requires `Authorization: Bearer ...`, `Idempotency-Key`, and the
current `/v1/state` `namespace` in its JSON body. Creation allocates lossless opaque
references and never assigns automatically. Policy `local` selects the server-owned
`local-soft-forecast/v1` policy: soft deadline, no cumulative delay cap, zero service,
and observed readiness or a valid forecast. Explicit capacity/demand must be positive.
Expected readiness and its validity endpoint must be supplied together; omitting
both leaves readiness unavailable until a supported observation is accepted.

1. `POST /v1/riders` with `location`, `capacity`, `available`.
2. `POST /v1/orders` with `pickup`, `dropoff`, `demand`, `policy` and optional forecast/deadline.
3. `POST /v1/dispatch` with the returned opaque `order` reference. The server assembles
   forecasts, policies, projections, routing and clock; evaluates on a detached snapshot;
   certifies the exact result; revalidates and attempts publication. Fleet admission is
   `/v1/dispatch/fleet`; existing-commitment recovery is `/v1/dispatch/recovery`.
4. `POST /v1/riders/{id}/actions/start` with the current decimal-string `plan_revision`.
   The route is computed outside the writer, then pinned only if the revision still matches.
5. Submit credential-bound `arrival` observations using returned `action` and `generation`,
   a namespace-qualified `effect_id` (`<namespace-token>:effect:<u64>`) and source evidence.
   Observation source allocates its own non-reused effect identities; conflicting reuse rejects.
6. Observe order readiness through `/v1/orders/{id}/ready`. Then issue `actions/service`.
   The server selects the accepted service duration. A timer or forecast never completes pickup.
7. Submit `completion` observation, then repeat start/arrival/service/completion for dropoff.

`GET /v1/riders/{id}` exposes current plan revision/execution; `GET /v1/orders/{id}`
shows fulfillment, readiness and the opaque `assigned_rider` reference. `GET /v1/deliveries/{id}` uses the **order reference**
and exposes authentic accepted terms plus current fulfillment/assignment. Published planning results include opaque order/rider `assignments` references. Nested domain decision/history evidence retains the library JSON contract; clients should use these opaque references and decimal revision strings for commands rather than round-tripping nested u64 identities through JavaScript numbers.
`actions/wait` preserves readiness waiting; `actions/delay` accepts observed road delay
and advances generation. Stale action/generation observations reject. Cancellation
uses `/v1/orders/{id}/cancel` and respects custody and started-action protection.
Forecast PATCH updates prediction only; readiness POST is separately authorized observation.

An observation supplies `source`, `observation_id`, contiguous `sequence` beginning
at 1 per `(source, resource)`, and `observed_at_seconds`. Source must match the
credential. Future observations, gaps, superseded sequences, time regression and
conflicting identity reuse reject without effects. Reuse of identical source identity
returns the original observation evidence. The domain also recognizes identical
logical action effects across different observations/commands. Receive time is recorded
separately. Gaps require caller reconciliation; this volatile adapter has no inbox broker
or quarantine service. Authentic physical source integration remains the embedding driver's duty.

## Clock and outcomes

The live clock samples UNIX seconds once and adds monotonic elapsed time throughout
this namespace lifetime. Numeric operational times use this advertised time domain;
RFC 3339 route departure timestamps convert to nonnegative UNIX seconds. Simulation
uses its own independent logical epoch. Clients never set the operational clock.

Command recognition is scoped to `(namespace, stable principal, key)`. Parsed intent
is serialized canonically; traffic overrides and fleet order lists are sorted, simulation
inputs normalize through the versioned scenario type, omitted request defaults normalize,
and server-assembled contexts/timestamps are excluded from intent. Same-key differing
meaning returns 409. Recognition lasts until this volatile namespace is discarded.

Versioned envelopes separate HTTP status, semantic outcome, `published`, immutable
result and decimal operational revision. HTTP 200 may be `noncommit` (no feasible
admission, incomplete search, recovery keep, cancellation refusal); 202 means `pending`.
`published` records business effects, while context adoptions can advance revision even
when a planning attempt does not publish assignments. Rejections retain original code/status.
Malformed JSON, authorization, namespace/key conflicts and capacity refusal occur before
reservation and return `{code,message}` errors. A saturated worker pool returns 503
without reservation; retry the same key. Existing command recognition does not need a worker slot.

After an uncertain/lost response:

```bash
curl -s http://127.0.0.1:3000/v1/commands/your-original-key \
  -H "Authorization: Bearer $ROADRUNNER_API_TOKEN" \
  -H "X-Roadrunner-Namespace: <original namespace>"
```

The lookup is principal-scoped and requires the original namespace. A discarded
namespace returns `namespace_unavailable`, not proof of noncommit. Internal worker
failure leaves a pending/unknown record; a poisoned authority fails closed. There is
no promise of durable recognition, automatic repair, multi-process publication or restart recovery.
Do not blindly reissue unknown business work in a new namespace.

The local driver uses four workers, a 2 MiB request limit, 50,000 dispatch work units
and a two-second **candidate proof endpoint**, not a universal freshness TTL. Dispatch
must establish `static-road-monotone/v1` feasibility/selection over that interval.
Static free-flow and immutable static traffic are supported. Time-dependent operational
certification and unknown active-prefix proof remain rejected, matching the existing domain
limits. Expensive routing/evaluation/simulation runs on blocking workers outside the writer.
The live graph remains fixed; traffic adoption is explicit. No algorithm is substituted silently.

## Simulation and validation

`POST /v1/simulations` wraps `{namespace, scenario}`. `scenario` is the existing versioned
`SimulationScenario` contract, with required `graph_snapshot_digest` equal to the server graph.
It supports scenarios 1–4 over that graph, at most 10,000 riders/orders, and creates a separate
simulation authority. It does not mutate live world, clock, execution or publication history.
See [simulation contracts](../../docs/simulation.md) and versioned fixtures. The OpenAPI
schema describes scenario/dispatch/order inputs; advanced dynamic event forms follow that library contract.

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
cargo +1.99.0 build -p roadrunner-api --release --locked
python3 scripts/collect_http_api_benchmark.py
```

The HTTP tests include retries/conflicts, principal scoping, observed execution,
source ordering, simulation isolation, OpenAPI/Swagger, and lost-response injection.
Controlled coordinator tests hold certified proposals across competing adoption,
observation and time-expiry interleavings and inspect complete domain state/history.
Measurements are tiny loopback adapter baselines, not scalability or production SLO claims.
