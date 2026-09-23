use crate::geo::Seconds;
use crate::graph::{AccessClass, DirectedEdge, FrozenGraph, RoadSegment};

use super::{CostError, CostKind, RouteCost, RoutingContext};

/// Search contract required by a traversal evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchCapability {
    /// Static, non-negative, additive scalar routing.
    StaticNonNegative,
    /// FIFO time-dependent earliest-arrival routing.
    FifoEarliestArrival,
    /// The evaluator requires search state not supported by node-state routing.
    RequiresExpandedState,
    /// The evaluator cannot provide a correctness contract supported by this engine.
    Unsupported,
}

/// Named heuristic policies whose lower-bound contracts an evaluator may support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeuristicPolicy {
    /// The always-safe zero lower bound.
    Zero,
    /// Endpoint Haversine distance for a physical-distance objective.
    DistanceHaversine,
    /// Endpoint Haversine distance divided by a validated maximum speed.
    TravelTimeHaversine,
}

/// Search-label information available while evaluating an outgoing edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraversalState {
    elapsed_travel_time: Seconds,
}

impl TraversalState {
    /// Creates state for the supplied elapsed travel time.
    #[must_use]
    pub const fn new(elapsed_travel_time: Seconds) -> Self {
        Self {
            elapsed_travel_time,
        }
    }
    /// Returns elapsed time since request departure.
    #[must_use]
    pub const fn elapsed_travel_time(self) -> Seconds {
        self.elapsed_travel_time
    }
}

/// Result of evaluating one outgoing directed edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraversalEvaluation {
    /// The edge may be traversed and contributes objective cost and elapsed time.
    Traversable {
        /// Contribution to the selected optimization objective.
        objective_cost: RouteCost,
        /// Physical/logical time elapsed while traversing the edge.
        travel_time: Seconds,
    },
    /// The edge is unavailable for this request without constituting an error.
    Forbidden,
}

/// Evaluates directed traversals under an objective and request context.
pub trait TraversalEvaluator: Send + Sync {
    /// Returns the semantic objective kind.
    fn kind(&self) -> CostKind;
    /// Returns the search contract required by this evaluator.
    fn capability(&self) -> SearchCapability;
    /// Returns whether this evaluator belongs to the requested graph snapshot.
    fn supports_graph(&self, _graph: &FrozenGraph) -> bool {
        true
    }
    /// Declares whether this evaluator preserves a named heuristic's lower bound.
    ///
    /// Custom evaluators are conservative by default and accept only zero.
    fn supports_heuristic(&self, policy: HeuristicPolicy) -> bool {
        policy == HeuristicPolicy::Zero
    }
    /// Evaluates one edge transition.
    ///
    /// # Errors
    ///
    /// Returns [`CostError`] when evaluation cannot produce valid values.
    fn evaluate(
        &self,
        edge: &DirectedEdge,
        segment: &RoadSegment,
        state: TraversalState,
        context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError>;
}

fn access_allowed(access: AccessClass, context: &RoutingContext) -> bool {
    match access {
        AccessClass::General => true,
        AccessClass::Private => context.has_private_access(),
        AccessClass::PermitRequired => context.has_permit_access(),
        AccessClass::Destination
        | AccessClass::Delivery
        | AccessClass::Customers
        | AccessClass::UnknownExplicit => false,
    }
}

/// Static physical-distance objective.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DistanceCost;

impl TraversalEvaluator for DistanceCost {
    fn kind(&self) -> CostKind {
        CostKind::Distance
    }
    fn capability(&self) -> SearchCapability {
        SearchCapability::StaticNonNegative
    }
    fn supports_heuristic(&self, policy: HeuristicPolicy) -> bool {
        matches!(
            policy,
            HeuristicPolicy::Zero | HeuristicPolicy::DistanceHaversine
        )
    }
    fn evaluate(
        &self,
        edge: &DirectedEdge,
        segment: &RoadSegment,
        _state: TraversalState,
        context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError> {
        if !access_allowed(edge.properties().access(), context) {
            return Ok(TraversalEvaluation::Forbidden);
        }
        Ok(TraversalEvaluation::Traversable {
            objective_cost: RouteCost::from_distance(segment.distance()),
            travel_time: edge.free_flow_travel_time(),
        })
    }
}

/// Static free-flow travel-time objective.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TravelTimeCost;

impl TraversalEvaluator for TravelTimeCost {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }
    fn capability(&self) -> SearchCapability {
        SearchCapability::StaticNonNegative
    }
    fn supports_heuristic(&self, policy: HeuristicPolicy) -> bool {
        matches!(
            policy,
            HeuristicPolicy::Zero | HeuristicPolicy::TravelTimeHaversine
        )
    }
    fn evaluate(
        &self,
        edge: &DirectedEdge,
        _segment: &RoadSegment,
        _state: TraversalState,
        context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError> {
        if !access_allowed(edge.properties().access(), context) {
            return Ok(TraversalEvaluation::Forbidden);
        }
        let travel_time = edge.free_flow_travel_time();
        Ok(TraversalEvaluation::Traversable {
            objective_cost: RouteCost::from_travel_time(travel_time),
            travel_time,
        })
    }
}
