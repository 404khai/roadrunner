# Runtime boundary foundation

Status: pre-Phase-20 implementation. HTTP, credentials, durable storage and event
transport are future phases. The approved [Q1–Q33 audit](runtime-boundary-audit.md)
is normative. The [completion report](pre-phase-20-remediation-completion.md)
records verification and remaining limits.

## Authority and dependency boundary

`roadrunner-dispatch::OperationalState` is one logical, volatile authority. It
jointly contains `World`, effective plan revisions, active execution, adopted
contexts, checked allocators, recovery cooldown, applied-effect recognition,
committed transition history and publication provenance. Its `World` is private;
read-only `Deref` provides existing query APIs. There is no mutable dereference,
public writable clone, or external replacement operation. Detached
`OperationalPlanningSnapshot` provides coherent read/evaluation access only.

Every mutation stages a complete private copy, validates domain and active-action
invariants, derives changed plan revisions, checks counter overflow, increments
one operational revision, records the committed transition, and replaces the
complete authority under exclusive mutable access. Failure discards staging.
Duplicate recognition may add an alias recognition record without advancing the
domain revision; it never reapplies custody, fulfillment or plan effects.

This exclusive publication boundary implements the initial logical single writer.
A future application coordinator must retain one authority per namespace and
serialize access; independent processes cannot share a volatile authority safely.
This library establishes in-memory atomic visibility under Rust's access rules,
not database transactions, fencing, crash durability or network authorization.

Dispatch owns transition validity and explicit state/time inputs. Simulation owns
its clock, heap, synthetic observations, randomness and metrics. Adapters must
validate observation trust; an ETA or timer cannot prove physical completion.
`World` remains a trusted standalone compatibility/evaluator library. An operational
state cannot expose its mutable World. Compatibility publication methods on
`OperationalState` reject unless established with `for_simulation` at bootstrap;
there is no operation to turn an existing operational authority into that mode.

## Identity, revision and allocation

| Type | Scope and meaning |
|---|---|
| `OperationalWorldNamespace` | Immutable 128-bit namespace; OS entropy for new live lifetime, explicit deterministic fixture namespace |
| `OrderId`, `RiderId` | Existing typed u64 local IDs; durable/public meaning requires namespace |
| `CommandId`, `DecisionAttemptId`, `DomainEventId`, `ExternalObservationId` | Distinct typed local identities; command registry and transport allocation are future work |
| `OperationalRevision` | Namespace + monotonic whole-authority number |
| `PlanRevision` | Monotonic logical remaining-plan sequence per rider |
| `ActionId` | Immutable started logical action, namespace scoped |
| `ScheduleGeneration` | Scheduled attempt for the same action; rescheduling does not allocate a new ActionId |
| `AppliedExecutionEffectId` | Recognized effect identity; also recognize action + effect across distinct IDs |
| `AdoptedContextIdentity` | Immutable semantic content reference by named category |
| `AdoptedContextRevision` | Monotonic adoption order, including A → B → A |

`create_order` and `register_rider` allocate canonical local IDs atomically above
existing IDs. Overflow fails without publication or wrapping. `reference` encodes
namespace/type/u64 as a lossless string; adapters must validate resource kind and
authorize it. This is an encoding foundation, not authentication. New volatile
lifetimes use `fresh`; `new` is bootstrap, not a durable restoration API. Phase 21
must restore all counters, revisions, recognition and execution before writes.

Plan progression/replacement changes PlanRevision when logical stops change.
Traffic/ETA-only reevaluation does not. A scheduled unstarted action must supply
its expected PlanRevision. Completion of a preserved started action checks active
ActionId, stage and generation, without requiring its originating PlanRevision
to equal the current effective plan revision.

## Shared execution

`next_stop` reads the current remaining plan. `start_action` checks revision,
expected plan, current origin, destination, ownership and route/anchor graph
binding. `delay_action` changes the attempt token/generation and predicted arrival,
retaining ActionId and original route provenance. `frozen_prefix` projects the
shared authoritative arrival/wait/service state; observed readiness is distinct
from a forecast. Missing readiness remains a typed evaluation failure.

`apply_effect` accepts explicit identity, rider, action, generation, effect and
observation instant. Arrival and completion have different recognition scopes.
An identical duplicate returns `AlreadyApplied`, including a distinct input ID
for the same logical effect. Conflicting reuse fails. Completion requires service
stage and service duration; pickup additionally requires observed readiness before
service. Completion publishes fulfillment/custody/plan advancement, active-action
removal, recognition and revision together. Dropoff releases one order. Waiting
and service cannot be abandoned by cancellation/recovery or suffix replacement.

The pinned `FrozenExecutionLeg` retains graph digest, nodes/edges and originating
routing context. Adopting a new graph does not reinterpret it. Active completion
needs no route lookup in the new graph. New planning against incompatible frozen
anchors fails until an explicit valid reanchoring transition is supported; no
implicit graph migration or mid-leg diversion is provided.

## Evaluation and publication time

`OperationalClock` supplies a named time domain and authoritative instant;
dispatch never reads a host clock. `LogicalOperationalClock` is a checked monotonic
implementation for tests/simulation/explicit adapter conversion. A live host-clock
conversion/epoch policy belongs to Phase 20. Evaluation, publication and commit
samples are recorded separately.

Lifecycle:

```text
snapshot → adopt validated immutable context → evaluate insertion/fleet/recovery
         → certify exact decision over an explicit interval
         → exclusive revalidation/staging → publication/commit clock sample
         → complete replacement + provenance
```

Evaluation can run on a detached snapshot outside the writer. Sealed
`EvaluatedOperationalDecision` binds the source operational revision, adopted
references/revisions, time domain and original domain decision. It cannot be
rebound to new authority. `certify` checks the original domain commit contract and
constructs a sealed `TemporalProposal`. `publish_temporal` checks exact current
revision/adoptions, routing provenance, clock domain and interval again, then
checks a second clock sample at the replacement boundary. Expiry during staging
leaves the original complete authority unchanged.

### `static-road-monotone/v1`

The supported operational publication policy uses departure-invariant Roadrunner
routing: free flow or immutable static traffic. There is no universal TTL or
implicit grace period. The caller chooses an explicit endpoint that must be
proved; an arbitrary chosen endpoint does not itself authorize publication.

For fixed stop sequences, fixed readiness instants and deterministic service,
later idle departure yields nondecreasing stop completion. Static road travel and
distance are unchanged. Frozen actions keep their original projected end and must
extend through the certified interval. Applicable forecast assumptions must remain valid throughout it. Observed
readiness and explicit legacy readiness do not depend on an unused forecast.
Endpoint evaluations validate capacity, custody, structural constraints, baseline
health and every hard service protection. New cumulative protection remains bound
to the original evaluated acceptance reference.

For Phase 17, the entire original declared complete search's objective/ranking is
invariant: feasible candidates can become infeasible with time, but previously
infeasible candidates cannot become feasible under this model. Therefore an
original best candidate still feasible at the endpoint remains best. There is no
hidden rebasing or reselection. For Phase 18 the certificate asserts continued
incumbent feasibility and unchanged admitted count/road objective, not a new
arbitrary global optimum. Phase 19 checks recovery feasibility, preserved terms,
ownership locks and static travel/churn applicability.

Time-dependent providers return `UnsupportedTemporalModel` for operational
certification even if a particular profile might admit a narrower proof. This
protects the Phase 17 exhaustive-best claim when departure time could change
ranking. Such routing still works in queries and trusted deterministic simulation.
Other temporal models require separately established proofs; no current-data
substitution or timestamp-equality workaround is provided.

Published new accepted terms retain the original completion reference and policy;
only their new `accepted_at` is stamped with the actual commit sample. Previously
accepted terms are never reset. If that stamp makes acceptance structurally invalid
(for example the reference already precedes commit), publication fails entirely.

## Context and historical evidence

Context adoption hashes canonical graph/traffic/readiness/profile/policy content.
Readiness forecasts, policy configurations and prediction/service/optimizer
identities are included. Adoption revision independently detects ABA. The private
original decision also retains epoch, anchors, projections, work budget and all
search evidence. Server assembly and adapter trust remain Phase 20 responsibilities;
public client objects cannot establish authority simply by asserting these fields.

`HistoricalRecord<T>` schema 1 separates kind, source OperationalRevision, policy
semantics, configuration digest, algorithm semantics, prediction identity,
evaluation and commit instant, and recorded payload. `PublicationProvenance`
records the certified endpoint, clock samples, adoption evidence, exact original
decision and actual committed facts. Recovery cooldown starts at actual commit. These are in-memory envelopes, not a durable command ledger or promise
to retain old graph/algorithm artifacts forever. Inspection uses recorded facts;
recomputation requires retained artifacts and compatible historical implementations.
Typed unavailable reasons distinguish missing artifacts/unsupported semantics/
missing authentic acceptance references. Phase 13–16 commitments stay soft legacy
semantics without invented references or cumulative bounds.

## Verified graph identities

Core graph artifact schema 4 uses `roadrunner.graph-semantics.v2` canonical hashing.
The hash covers compiled topology, directed attributes, E7 geometry, float bit
representations, adjacency, maneuver restrictions, capabilities and build metadata,
plus an optional bound provenance digest. OSM build configuration explicitly
binds compiler semantics v4 as well as the package/compiler metadata. Its own asserted digest/compact ID are
excluded to avoid a cycle. Finalization and every immutable modifier seal a new
verified identity; compact u64 ID derives from the full digest and cannot replace
it in detached references.

OSM compiler v4 additionally binds canonical compiled provenance, with its
self graph-digest field excluded under `roadrunner.compiled-provenance.v1`.
Bundle loading checks that binding as well as artifact integrity and structural
validation. Standalone schema 3 claims are inspection-only through
`inspect_graph_identity`; they cannot be loaded as operational graphs. Historical
fixture identifiers remain unchanged. New explicitly named semantic-v2 traffic
fixtures and [verified correspondence](evidence/pre20-graph-correspondence.json)
record equal routing content, without pretending historical decisions originally
used the new digest.

## Verification commands

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.99.0 test --workspace --locked
cargo +1.99.0 build --workspace --release --locked
cargo +1.99.0 test -p roadrunner-core --test semantic_identity --locked
cargo +1.99.0 test -p roadrunner-dispatch --test operational --locked
cargo +1.99.0 test -p roadrunner-dispatch --release --test operational --locked -- --ignored --nocapture
python3 scripts/verify_pre20_replay.py --baseline /path/to/archived/roadrunner --binary target/release/roadrunner --output docs/evidence/pre20-replay.json
python3 scripts/collect_operational_boundary_benchmark.py --output benchmarks/results/pre20/operational-boundary.json
```

The archived replay binary must be built from the recorded baseline commit, not
from current sources. Regression comparison explicitly lists changed identity
paths and compares every other semantic field; current replay is exact.

## Supported proof limits

Operational certification currently requires explicit evaluable active-prefix
projections. Unevaluable active work is preserved; no frozen departure is invented.
The domain/simulator Phase 18 input-isolation contract remains intact, but the initial
operational certificate does not yet prove publication around such unknown active
prefixes. It returns a typed input/context limitation. Complete/evaluable static-road
fleet decisions are supported. Extending certification to isolated unknown active
work requires preserving the unchanged actor and its unknown health explicitly.
No guarantee of all routing profiles or all input-isolation cases is asserted by
this initial proof policy. This is a declared applicability limit, not business
infeasibility or permission to abandon execution.
