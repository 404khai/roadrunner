use roadrunner_core::geo::{Meters, Seconds};
use roadrunner_core::graph::{EdgeId, NodeId};
use roadrunner_dispatch::{
    DecisionId, DispatchInstant, OrderId, RiderId, RoutingProvenance, TrafficIdentity,
};
use serde::Serialize;

/// A complete road leg frozen at departure and recorded only after arrival.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExecutedLeg {
    /// Leg origin node.
    pub from: NodeId,
    /// Leg destination node.
    pub to: NodeId,
    /// Explicit logical departure.
    pub departed_at: DispatchInstant,
    /// Road travel duration; excludes stop waiting.
    pub travel: Seconds,
    /// Completed road distance.
    pub distance: Meters,
    /// Path computed by Roadrunner's core routing implementation.
    pub nodes: Vec<NodeId>,
    /// Directed traversals, including legal maneuvers.
    pub edges: Vec<EdgeId>,
    /// Graph/profile/traffic used at departure.
    pub routing: RoutingProvenance,
}

/// Processed domain event; dispatch failure and valid Unassigned are distinct.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SimulationEvent {
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
