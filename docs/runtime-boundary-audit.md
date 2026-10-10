# Pre-Phase-20 Runtime Boundary Architecture Audit

Status: confirmed by the user, 2026-10-10. Normative for prerequisite remediation
and Phases 20–22. The grill is closed. Implementation authorization currently
covers prerequisites only. Verdict: READY AFTER SPECIFIED PRE-PHASE-20 REMEDIATIONS.

## Accepted decision register

1. One coherent logical operational authority covers domain, execution and correctness metadata; independently sourced planning inputs use validated immutable references. Projections cannot override facts.
2. Dispatch owns execution validity, including action/stage/owner/custody/frozen/generation checks and same-effect idempotency; adapters establish observation authority.
3. Server assembles contexts and revalidates authority at publication; clients supply intent. No weaker operational publication bypass.
4. Shared domain transitions, separate simulation/operational drivers; predicted timers never prove physical completion. Preserve Phase 15–19 declared simulation behavior.
5. Durable identity is immutable world namespace plus typed local integer. Server allocation is checked; public references are opaque lossless strings. Volatile discarded lifetimes change namespace; durable restore preserves it.
6. Namespace-bound monotonic whole-operational revision advances atomically for meaningful authoritative changes, not duplicate/rejected no-effect operations. External identities, adoption revisions and typed time remain distinct.
7. Separate PlanRevision, ActionId, ScheduleGeneration and AppliedExecutionEffectId. Frozen actions survive suffix replacement and rescheduling; unstarted superseded work cannot execute.
8. Universal semantic graph identity and verified loading precede Phase 20. Historical identities are never silently relabeled; frozen routes keep their original snapshot.
9. Versioned history preserves actual terms and immutable policy/configuration, prediction, algorithm and graph provenance. Inspection differs from recomputation; missing inputs produce typed unavailable outcomes.
10. One publication lane per namespace; expensive evaluation outside it. All authoritative mutations/context adoption participate; read-only routing does not hold the writer.
11. All authoritative transition effects, recognition, revision and required delivery intent publish atomically. Pending progression is valid independently. Database commit becomes the durable linearization point in Phase 21.
12. Current external input means runtime-adopted immutable reference plus adoption revision. Remote receipt is distinct. Detect ABA adoption; identity equality does not establish freshness.
13. Preserve execution and reject stale proposals wholly. No rebase/partial salvage. Reevaluate as a new evidenced attempt; transport retries resolve original outcomes before business reexecution.
14. Phase 20 exposes operational queries/commands, outcome lookup and isolated simulation. Dispatch evaluates and attempts commit. Creation does not assign; recovery does not silently admit. No detached proposals or coordinate snapping.
15. Mandatory HTTP key scoped by namespace/stable authenticated principal binds canonical client meaning, not server context. Atomic reservation; effects and terminal success together; lifetime recognition. Command ledger is not domain revision.
16. Versioned result envelopes separate HTTP status, semantic outcome and publication. 200 can be noncommit, 202 pending, 409 conflict. Lost volatile namespace is not proven noncommit.
17. Authenticate/authorize capability, namespace and resource. Preserve observation source/identity/times/ordering. Authentication alone does not prove physical truth. Explicit limited local mode; credential mechanism deferred.
18. Fail closed unless temporal validity establishes required feasibility/selection guarantees at publication. Justified intervals permitted; no universal TTL or timestamp-equality requirement. Mechanism must allow useful nonzero computation time.
19. Durable transactional current state is authority; immutable history explains it. Events are not default recovery truth. Full event sourcing deferred.
20. Relational operational facts/relationships and versioned immutable evidence. Constraints defend local integrity; dispatch validates whole-state invariants. No independently editable derived truth or generic repository framework.
21. Storage namespace gate checks fence/revision/context/prior state/time transactionally. Authority transfer uses the same exclusion; obsolete writers cannot publish. Election and isolation implementation remain separate choices.
22. Restore/validate complete authority before writes. Preserve frozen work; resume only safe progression. Prove nonpublication before InterruptedBeforePublication; reconcile unknown outcomes. Critical missing artifacts block writes; optional replay absence is narrower.
23. Declared durability/retention/migration/restore/clone contracts. Lifetime recognition survives ordinary restart. No silent acknowledged-history rollback or two writable copies. New namespace alone cannot undo external effects. Offline migrations preserve meaning.
24. Distinct domain event, integration event, observation, command and simulation categories. Only validated atomic transitions establish operational truth.
25. Transactional immutable outbox obligation with originating commit revision/ordinal. At-least-once delivery reuses identity/payload. Transport acknowledgement is not consumer application. Obligation in 21, delivery in 22; broker optional.
26. Stable consumer/source/message inbox fingerprint and outcome atomically accompany effects. Domain effect/observation/command/business uniqueness remain separate. Acknowledge after durable outcome/reconciliation; remote effects need separate guarantees.
27. Type-specific causality/source ordering, not timestamp last-write-wins. Defer gaps, recognize duplicates, reject superseded attempts, quarantine with evidence and authorized resolution. Global revisions are not contiguous consumer sequence numbers.
28. Separate historical inspection, operational recovery, deterministic decision replay and event-processing replay. Isolate replay/suppress external effects; use recorded order/time/compatible semantics, never modern substitute inputs.
29. Controlled concurrent histories/fault injection inspect complete authority and legal sequential publication explanation, not only final state/response counts. Actual storage/process tests required for durable guarantees.
30. Separately authorized pre-20 gates: graph identity/history; authority/shared transitions; namespace/revisions/generations/effect recognition; envelopes; usable clock/time validity; no bypass; reconciled docs; regression/replay.
31. Phase 20 gate: complete authorized operational API, volatile reservation/idempotency, contexts/publication/time, race/lost-response tests, isolated simulation, routing identity, measured evidence. No durable or multi-writer claim.
32. Phase 21 gate: real-storage atomicity/exclusion/fencing/recognition, frozen restoration, outbox preservation, crash recovery, meaning-preserving migrations and safe restoration/cloning, measured evidence. No broker requirement.
33. Phase 22 gate: durable outbox redelivery/inbox/effects, ordering/quarantine/reconciliation, crash recovery, isolated replay/observability/measured delivery. No exactly-once end-to-end claim.

## Boundaries, evidence and implementation order

Core owns graph/routing/cost, dispatch owns validity/planning/operational facts,
simulation owns synthetic observations/logical queue/metrics, future adapters
own external trust and infrastructure. No HTTP, database, credential or broker
implementation is authorized by this prerequisite stage.

The inspected baseline has WorldData domain state but private simulator active
stops/cooldowns. World publication is serial in-memory only. Graph metadata
hashing omits compiled content; maneuver modification retains digest; core
artifact checksum does not prove semantic identity. Fleet/recovery commits
protect trusted explicit contexts but do not establish an external authority.
These are prerequisite defects, not reasons to invalidate Phase 19's declared
simulator evidence.

Dependency order: reconcile contracts → universal graph identity → operational
identity/revision → coherent execution/effect publication → temporal applicability
and versioned evidence → Phase 15–19 replay/regression → prerequisite report.

Every prerequisite gate requires concrete PASS evidence. Incomplete gate means
REMEDIATION INCOMPLETE. A material architectural amendment requires user
confirmation. Stop after prerequisite report; Phase 20 requires separate approval.

## Future phase contracts and deferred choices

Phase 20 is one volatile authority with authorized intent/outcome lookup. Phase 21
makes that authority durable through transactional state, fencing, recognition,
restoration and immutable obligations. Phase 22 delivers/processes messages
without repeating effects. Database/isolation, credentials, delivery technology,
retention implementation and disaster-recovery operations remain implementation
design choices constrained by this register.

Deferred: fine-grained/multi-writer publication, detached optimizers, snapping,
full event sourcing/arbitrary historical reconstruction, indefinite historic
implementation support, automatic failover, online mixed writers, mandatory
brokers, global ordering, parallel consumers, distributed provider transactions,
and exactly-once remote effects.

Required failure evidence: competing assignments/plans, execution versus search,
recovery versus insertion, context/time expiry, lost responses/ambiguous commit,
duplicate effects/messages/business references, superseded actions, transaction
rollback, publisher/consumer crash, dependency gaps/quarantine, migration and
unsafe restoration. Record initial state, identities, controlled interleaving,
fault point, outcomes, legal committed history and final invariants.

Benchmarks record hardware/dataset hashes/size/configuration/build/run count/raw
and median/p95/p99 where applicable, deterministic work, and measured resources
or explicitly unmeasured values. Never invent SLOs or numbers.
