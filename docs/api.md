# Roadrunner API contracts

Phase 20 exposes Axum HTTP endpoints alongside the existing library and CLI APIs.
See the [HTTP run guide and contracts](../crates/roadrunner-api/README.md) and
[completion evidence](phase-20-completion.md). Generated OpenAPI is served at
`/openapi.json`; vendored Swagger UI is at `/swagger-ui/`.

## Phase 17 dispatch library

See the full [multi-order input/evidence/publication contract](multi-order.md).

| API / type | Responsibility |
| --- | --- |
| `OrderPolicy`, `ReadinessForecast` | Explicit versioned admission, service and prediction validity inputs |
| `project_execution` / `ExecutionProjection` / `FrozenPrefix` | Validate immutable active execution and expose post-prefix custody/load/anchor/suffix |
| `evaluate_whole_plan` / `WholePlanEvaluation` | Authoritative propagated road/readiness/service/load/completion and protections |
| `insert_order` / `InsertionDecision` | Read-only complete eligible-fleet order-preserving single-order insertion |
| `PoolingContext::new` | Bind exact logical time, world-independent policies/predictions, execution and routing inputs |
| `World::commit_insertion` | Exact stale checking and atomic new assignment, replacement plan and accepted terms |
| `World::pickup` / `World::deliver` | Shared custody, fulfillment, plan advancement and per-order release |

Whole-plan evaluation distinguishes typed `PoolingError` from `InsertionRejection`.
Completed search distinguishes `BestInsertion`, `NoFeasibleInsertion` and
`SearchIncomplete`. Only complete BestInsertion decisions contain publishable proposals.
Inputs and accepted/provisional state cannot be conflated; callers rebuild current
context from their authoritative sources before commit. World identity/version covers
all domain mutations, while complete context equality covers external prediction,
policy, graph/traffic/profile, projection/execution and clock changes.

## CLI and scenario contracts

`roadrunner simulate <scenario.json> [--json]` loads the existing graph/scenario document.
Schema 1 preserves historical single-order semantics. Schema 2 requires scenario_id,
`dispatch.kind = multi_order`, work_budget, forecast_validity_seconds and every order's
explicit admission policy. No hard or cumulative numeric default is introduced.

Successful JSON contains semantic insertion evidence, proposed terms/plans/assignments,
commit status/version, final state, predicted/actual outcomes and realized protection
comparisons. Readable output includes search coverage, work, typed rejections and the
chosen plan/road deltas. Missing required forecasts produce typed incomplete-input nonpublication records while
existing execution continues; observations may allow a later fresh admission. Invalid
policies or structural corruption produce a nonzero exit without a success artifact. See [runnable examples](../data/fixtures/phase-17/README.md).

## Phase 18 fleet batch extension

Joint new-order allocation and editable suffix resequencing preserve committed owners,
accepted terms and frozen execution. Dispatch uses `optimize_fleet`, `FleetInputs`,
`FleetContext`, `evaluate_batch_plan`, and atomic `World::commit_fleet`. Simulation schema
3 selects `fleet_batch` with a named deterministic algorithm. Explicit unknown-input and
baseline-breach isolation differ from Phase 17 complete-input insertion. See
[fleet optimization](fleet-optimization.md) for API, objective, neighborhood, coverage,
execution, oracle and measurement contracts, and [completion](phase-18-completion.md)
for verified results. Phase 19 committed recovery is implemented with
`RecoveryContext`, `RecoveryPolicy`, `recover_fleet`, `World::commit_recovery` and
validated cancellation; see [dynamic recovery](dynamic-redispatch.md).

## Operational prerequisite library boundary

Standalone `World` APIs above are trusted compatibility/domain APIs; they do not
establish an external operational authority. `OperationalState` privately contains
World and active execution. Its read-only `snapshot` is the detached planning
boundary. `create_order`/`register_rider` allocate checked namespace-scoped identities.
Shared execution uses `next_stop`, `start_action` with expected PlanRevision,
`delay_action`, `frozen_prefix`, readiness/wait/service transitions and `apply_effect`
with distinct action/generation/effect identity. Identical duplicate effects return
AlreadyApplied without another domain revision; conflicting reuse rejects.

Operational publication uses `adopt_planning_context`, `evaluate_insertion` /
`evaluate_fleet` / `evaluate_recovery`, `certify`, then `publish_temporal`. Sealed
records bind exact operational/adoption revisions, pinned routing and named time
semantics. `static-road-monotone/v1` proves an explicit interval; it supports
nonzero elapsed evaluation time and rejects unproved time-dependent selection.
All transition effects publish in one in-memory replacement. Actual accepted_at
and recovery cooldown use the commit sample; original completion references remain
unchanged. `PublicationProvenance` includes separate original decision and actual
commitment records. No mutable World or weaker legacy operational commit is exposed.

Simulation declares `runtime_semantics = shared-execution/v2` separately from its
scenario schema. Historical graph schema 3 claims are inspection-only; current
schema 4 loaders recompute the canonical semantic identity. See the complete
[identity/time/compatibility API contract](runtime-boundary.md).

## Phase 20 HTTP interface

Phase 20 adds authorized routing and operational snapshot queries, creation,
lifecycle/assignment/fleet/recovery/cancellation commands, supported observations,
command-outcome lookup and isolated simulation. Creation remains separate from
admission. Clients provide intent, not authoritative snapshots, custody, accepted
terms or clocks. Routing references are snapshot qualified; no hidden snapping.

The coordinator reserves namespace/principal-scoped idempotency keys against
canonical intent, assembles server contexts, evaluates outside the writer and
atomically publishes or returns a typed noncommit/pending result. Same-key different
meaning conflicts; same-key retries return original status/results. Logical execution
effects remain once-only across distinct commands. Recognition lasts for the living
volatile namespace; discarded authority cannot establish durable historical outcomes.
The API implements bearer capability authorization, source-bound observation ordering,
and a UNIX-origin monotonic host clock. Its explicit local entry point grants observation
trust only to the trusted local driver. Phase 21/22 durability, outbox/inbox, fencing and recovery are
separate gates. See [approved Q1–Q33](runtime-boundary-audit.md) and ADRs 0017–0019.

## Phase 21 durable adapter

[Persistent mode](persistence.md) preserves the namespace, original command responses,
observations/effect recognition, frozen execution, counters, authentic terms and history.
Database COMMIT is authoritative; acknowledgement additionally requires the independent
fsynced head watermark. A fenced/failed writer cannot serve cached operational state.
Successful route queries return `route_records` and retain full original provenance;
simulation scenarios/results are recorded in isolation. Migrations, restoration and
unknown-outcome semantics are detailed in the durable guide. HTTP DTOs and domain
algorithms remain independently owned; no client state restoration endpoint is added.


## Phase 26 portable inspection

`POST /v1/visualization/routes` and `/v1/visualization/routes/alternatives` use the existing
strict route request and bearer read capability. Their version-1 responses embed checked E7 route
evidence rather than the legacy display GeoJSON. See the [producer contract](visualization-contract.md)
and [route schema](schemas/route-evidence-v1.schema.json) for identities, units, exact boundaries,
summary/termination scopes and directional compatibility. Saturation returns 503 and geometry/byte
limits return 413 without changing authoritative routing or returning silently partial geometry.
These endpoints do not record durable routes. Legacy alternative responses now include objective
and applicable traffic digest, matching single-route provenance. Planning command evidence additionally
contains versioned `inspection` metadata; separate original publication evidence establishes commitment.

## Phase 27 graph-anchor selection

`GET /v1/visualization/anchors?offset=0` requires the existing routing read capability
and returns version 1 immutable graph metadata, total node/edge counts and up to 500
exact `{node, coordinate}` anchors in dense node order. `offset` is a nonnegative
index, `next_offset` is null on the last page, and out-of-range offsets reject. Its
selection semantics are `explicit_graph_node_no_snapping/v1`. Pages refer to the
process's fixed full graph identity, never a mutable operational snapshot. The
bounded slice is serialized without taking the operational coordinator lock. No
edges, operational entities, search traces, snapping or routing changes are added.
The [web client](../web/README.md) uses this catalog and the unchanged Phase 26
portable single-route endpoint; it does not fetch `/v1/state`.

## Phase 28 alternative comparison and selected-segment inspection

The web client now consumes `POST /v1/visualization/routes/alternatives` with
`algorithm: dijkstra` and `alternatives: 2..5` (including the primary). The actual
returned algorithm is Yen; A* alternative enumeration is unsupported. The endpoint
continues to require read capability and never publishes an operational plan.
Its primary is a legal **loopless** optimum, not a claim about unrestricted walks.
Fewer routes, cost limits, exhausted enumeration and partial budget results are
rendered with their original semantics.

Each member may additionally expose the independently versioned, bounded
`segment_inspection` described in the [visualization contract](visualization-contract.md#phase-28-selected-path-inspection-extension).
It uses captured static inputs for the returned path, with checked sums and no new
routing/search semantics, world mutation or historical trace retention. Existing
portable route identities, legacy responses and single-route responses are unchanged.

## Phase 29 bounded graph inspection

`GET /v1/visualization/graph` exports independently versioned directed graph pages
under the existing read capability. It requires the full graph digest; optional
node/edge offsets and a static objective select the capture. Limits are 500 node-page
entries plus edge endpoints (900 nodes total), 200 edges, 20,000 directed vertices,
and 4 MiB encoded output. Pages stop at edge boundaries; oversized single geometry
rejects rather than clips. Existing worker/body-admission limits apply.

Exact E7 geometry, base time, adopted traffic factor, default-access evaluation and
provenance are exposed. A standalone edge cost is not incoming-maneuver or route
feasibility. Road class/search membership remain explicitly unavailable. The endpoint
never records routes or changes graph/dispatch semantics. See the full
[debugger wire contract and limitations](graph-debugger.md#http-contract).

## Bounded delivery simulation view (Phase 30)

`POST /v1/visualization/deliveries` accepts the strict existing SimulationScenario with
exact server graph digest and requires namespace-wide read plus simulation capability.
It exports an isolated real engine run with coherent queue-boundary frames, original
checked frozen paths, schedule changes, accepted effects and bounded graph geometry.
It is read-only with respect to the live world and requires no command idempotency key.
Version/identity, admission limits, errors, traffic interpretation and motion boundaries
are specified in [live delivery visualization](live-delivery-visualization.md).
