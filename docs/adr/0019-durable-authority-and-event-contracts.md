# ADR 0019: Durable authority and event delivery contracts

Status: Accepted future contract (audit Q14–Q17, Q19–Q29, Q31–Q33).
Implementation: NOT implemented by prerequisite remediation.

## Decision

Phase 20 exposes authorized intent, server contexts, serialized publication,
namespace/principal-scoped mandatory idempotency keys, canonical payload conflict
checking and versioned outcomes. Creation is separate from admission; recovery does
not silently admit. Reserve a command atomically, recognize original outcome on
transport retry and reconcile unknown outcome before a new business attempt.
Recognition lasts for the living volatile namespace; discarded state cannot prove
old commands uncommitted. No restart-durable or multi-writer guarantee yet.

Phase 21 makes transactional current state authoritative, with immutable explanatory
history. One transaction checks writer fence, expected revision/adoptions/prior state
and temporal applicability, then publishes all domain/execution/terms/effect/command
results, revision and required outbox obligations. Database commit is the durable
linearization point; external delivery occurs outside it. Restore and validate the
entire authority, including frozen work and recognition, before allowing writes.
Actual storage/process tests must prove rollback, exclusion, fencing and restart.
Offline migrations preserve meaning; unsupported/corrupt state fails closed.
Backup rollback, writable cloning, retention and acknowledged durability require
explicit policies. Database/isolation/schema technologies remain implementation
choices constrained by these guarantees, despite roadmap stack suggestions.

Phase 22 delivers immutable outbox messages at least once and transactionally
recognizes inbox outcomes plus business effects. Message duplication and domain
effect duplication are separate scopes. Acknowledge after durable terminal outcome;
never equate delivery with committed effect. Typed source/causal ordering replaces
global timestamp last-write-wins. Gaps defer; poison/conflicting inputs quarantine
with identity, payload, attempt history and authorized reconciliation. Replay uses
an isolated authority with external effects suppressed and compatible recorded inputs.

## Gates and alternatives

Controlled interleavings must establish a legal sequential committed history, not
only plausible final state. Inject failures before/after commit and lost acknowledgements;
prove same-effect safety across distinct commands/messages. Real storage and selected
transport tests are mandatory in their phases. Working endpoints or a queue alone
are insufficient.

Full event sourcing, generic repositories, mandatory brokers, global message order,
parallel consumers, distributed provider transactions, exactly-once transport/remote
effects and automatic failover are deferred. Small modular-monolith adapters are the
initial architecture. See the [approved audit](../runtime-boundary-audit.md) and
[phase contracts](../runtime-boundary.md); no future guarantee is asserted as delivered.
