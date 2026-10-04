use std::collections::BTreeMap;

use roadrunner_core::geo::Coordinate;
use serde::Serialize;

use crate::{DispatchInstant, RiderId};

macro_rules! identity {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(u64);
        impl $name {
            /// Creates an identity from its canonical integer value.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }
            /// Returns its canonical integer value.
            #[must_use]
            pub const fn value(self) -> u64 {
                self.0
            }
        }
    };
}
identity!(OrderId, "Stable fulfillment request identity.");
identity!(DecisionId, "Caller-supplied immutable decision identity.");
identity!(WorldVersion, "Whole-world optimistic concurrency version.");
identity!(
    CapacityUnits,
    "Non-negative normalized reference-parcel slots, never active-order count."
);

/// Stable fulfillment request facts, separate from readiness and execution.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Order {
    /// Request identity.
    pub id: OrderId,
    /// Authoritative pickup location, independent of graph projection.
    pub pickup: Coordinate,
    /// Authoritative dropoff location.
    pub dropoff: Coordinate,
    /// Creation instant.
    pub created_at: DispatchInstant,
    /// Completed-dropoff commitment, soft-observed by both dispatch strategies.
    pub deadline: Option<DispatchInstant>,
    /// Scalar onboard demand.
    pub demand: CapacityUnits,
}

/// Readiness prediction and observation are distinct facts.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct OrderReadiness {
    /// Mutable estimate; Phase 14 uses this only while no actual ready event exists.
    pub expected_at: Option<DispatchInstant>,
    /// Actual ready event; not overwritten by a prediction.
    pub observed_at: Option<DispatchInstant>,
}

/// Fulfillment state is authoritative for custody and remaining-stop semantics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum FulfillmentState {
    /// No pickup completed; responsibility may still be unassigned.
    AwaitingPickup,
    /// Onboard custody forbids responsibility transfer.
    PickedUp {
        /// Custody holder.
        rider: RiderId,
        /// Observed pickup completion.
        at: DispatchInstant,
    },
    /// Completed dropoff, with no active responsibility or remaining stops.
    Delivered {
        /// Rider that completed fulfillment.
        rider: RiderId,
        /// Historical completed pickup, preserved after custody is released.
        picked_up_at: DispatchInstant,
        /// Observed completed dropoff.
        at: DispatchInstant,
    },
}

/// Stable rider identity and supported capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RiderProfile {
    /// Identity shared with the spatial projection.
    pub id: RiderId,
    /// Exact supported compiled routing profile; no silent substitution.
    pub routing_profile: String,
    /// Maximum scalar onboard demand.
    pub max_capacity: CapacityUnits,
}

/// Operational availability independent from plan or custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Availability {
    /// Operationally available; existing work still prevents Basic Dispatch eligibility.
    Available,
    /// Offline, paused, or otherwise not available for new assignment.
    Unavailable,
}

/// Mutable authoritative position and operational availability.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RiderState {
    /// Validated geographic location.
    pub coordinate: Coordinate,
    /// Operational availability.
    pub availability: Availability,
}

/// Current active responsibility, distinct from decision evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CommittedAssignment {
    /// Request being fulfilled.
    pub order: OrderId,
    /// Responsible rider.
    pub rider: RiderId,
}

/// Logical remaining work; location is resolved through the referenced request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Stop {
    /// Complete pickup and establish onboard custody.
    Pickup(OrderId),
    /// Complete dropoff and release onboard custody.
    Dropoff(OrderId),
}

impl Stop {
    /// Returns the referenced order.
    #[must_use]
    pub const fn order(self) -> OrderId {
        match self {
            Self::Pickup(id) | Self::Dropoff(id) => id,
        }
    }
}

/// Ordered remaining logical work, with no two-stop or single-order type limit.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct RiderPlan {
    /// Pending work in execution order; completed stops are removed.
    pub stops: Vec<Stop>,
}

/// Value input to a world; published state is validated and exposed read-only.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorldData {
    /// Stable requests.
    pub orders: BTreeMap<OrderId, Order>,
    /// Mutable readiness keyed by request identity.
    pub readiness: BTreeMap<OrderId, OrderReadiness>,
    /// Execution and custody facts.
    pub fulfillment: BTreeMap<OrderId, FulfillmentState>,
    /// Rider capabilities.
    pub profiles: BTreeMap<RiderId, RiderProfile>,
    /// Authoritative operational state.
    pub riders: BTreeMap<RiderId, RiderState>,
    /// Active responsibility keyed by order.
    pub assignments: BTreeMap<OrderId, CommittedAssignment>,
    /// Effective remaining plan keyed by rider.
    pub plans: BTreeMap<RiderId, RiderPlan>,
}
