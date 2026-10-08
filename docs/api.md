# Roadrunner API contracts

The current repository exposes library and CLI APIs. HTTP endpoints remain Phase 20.

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
for verified results. Phase 19 recovery remains deferred.
