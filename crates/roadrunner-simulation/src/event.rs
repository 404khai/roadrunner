use roadrunner_dispatch::{DecisionId, DispatchInstant, OrderId, RiderId, TrafficIdentity};
use serde::Serialize;

/// Shared pinned execution leg; simulation records it after actual model arrival.
pub type ExecutedLeg = roadrunner_dispatch::FrozenExecutionLeg;

/// Processed domain event; dispatch failure and valid Unassigned are distinct.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SimulationEvent {
    /// Committed-work recovery result, including keep/incomplete/failure.
    RecoveryPlanned {
        /// Semantic logical-time trigger.
        trigger: String,
        /// Atomic publication result.
        committed: bool,
        /// Typed failure or termination text.
        result: String,
    },
    /// External dynamic change and explicit application/refusal outcome.
    DynamicChanged {
        /// Exact configured input.
        change: crate::DynamicChange,
        /// No implicit cancellation or execution abandonment.
        applied: bool,
        /// Human-readable typed refusal/application reason.
        reason: String,
    },
    /// One fleet decision may admit many requests through one atomic publication.
    FleetPlanned {
        /// Canonical original batch, including isolated requests.
        batch: Vec<OrderId>,
        /// Newly committed request-to-rider ownership.
        admitted: Vec<roadrunner_dispatch::CommittedAssignment>,
        /// Exact joint publication occurred.
        committed: bool,
    },
    /// Shared order registration completed.
    OrderCreated {
        /// Registered identity.
        order: OrderId,
    },
    /// Actual readiness observed, including already-ready stock received at creation.
    OrderReady {
        /// Request identity.
        order: OrderId,
        /// Actual fact timestamp, which may precede event receipt.
        ready_at: DispatchInstant,
    },
    /// Exact decision committed through World.
    RiderAssigned {
        /// Request identity.
        order: OrderId,
        /// Responsible rider.
        rider: RiderId,
        /// Immutable decision identity in the result.
        decision: DecisionId,
    },
    /// Valid completed evaluation found no feasible eligible rider; request remains pending.
    DispatchUnassigned {
        /// Pending request identity.
        order: OrderId,
        /// Completed Unassigned decision.
        decision: DecisionId,
    },
    /// Required readiness input prevented this admission; existing execution continues.
    DispatchPredictionFailed {
        /// New request whose admission was blocked.
        order: OrderId,
        /// Request with unavailable prediction (may be existing committed work).
        unavailable_order: OrderId,
    },
    /// Rider reached a road-leg endpoint; no interpolated positions are invented.
    RiderMoved {
        /// Request being executed.
        order: OrderId,
        /// Moving rider.
        rider: RiderId,
        /// Completed movement.
        leg: ExecutedLeg,
    },
    /// Arrival at store/restaurant; readiness may require additional waiting.
    RiderArrivedPickup {
        /// Awaiting request.
        order: OrderId,
        /// Arriving rider.
        rider: RiderId,
    },
    /// Shared pickup established custody and advanced remaining work.
    OrderPickedUp {
        /// Picked-up request.
        order: OrderId,
        /// Custody holder.
        rider: RiderId,
    },
    /// Shared delivery released custody and active responsibility.
    OrderDelivered {
        /// Completed request.
        order: OrderId,
        /// Completing rider.
        rider: RiderId,
    },
    /// A new complete static overlay became active for new departures.
    TrafficChanged {
        /// Exact new traffic identity.
        traffic: TrafficIdentity,
    },
}

/// Event trace entry ordered by (logical instant, monotonic sequence).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RecordedEvent {
    /// Logical time of the processed event.
    pub at: DispatchInstant,
    /// Stable queue insertion identity, never a wall-clock value.
    pub sequence: u64,
    /// Completed event and its source identities.
    pub event: SimulationEvent,
}
