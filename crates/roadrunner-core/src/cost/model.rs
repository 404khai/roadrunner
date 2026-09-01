use crate::graph::Edge;

use super::{CostError, CostKind, RouteCost, RoutingContext};

/// Evaluates a directed edge under a selected routing objective.
pub trait CostModel: Send + Sync {
    /// Returns the semantic kind produced by this model.
    fn kind(&self) -> CostKind;

    /// Evaluates an edge in an immutable routing context.
    ///
    /// # Errors
    ///
    /// Returns [`CostError`] when the model cannot produce a valid non-negative,
    /// finite cost. The Phase 4 models operate on validated edge attributes and
    /// therefore always succeed.
    fn edge_cost(&self, edge: &Edge, context: &RoutingContext) -> Result<RouteCost, CostError>;
}

/// Selects edge distance in meters as route cost.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DistanceCost;

impl CostModel for DistanceCost {
    fn kind(&self) -> CostKind {
        CostKind::Distance
    }

    fn edge_cost(&self, edge: &Edge, _context: &RoutingContext) -> Result<RouteCost, CostError> {
        Ok(RouteCost::from_distance(edge.distance()))
    }
}

/// Selects edge base travel time in seconds as route cost.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TravelTimeCost;

impl CostModel for TravelTimeCost {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }

    fn edge_cost(&self, edge: &Edge, _context: &RoutingContext) -> Result<RouteCost, CostError> {
        Ok(RouteCost::from_travel_time(edge.base_travel_time()))
    }
}

#[cfg(test)]
mod tests {
    use crate::geo::{Meters, Seconds};
    use crate::graph::{EdgeId, NodeId};

    use super::*;

    fn edge() -> Edge {
        let distance = Meters::new(1_250.0);
        let travel_time = Seconds::new(180.0);
        let (Ok(distance), Ok(travel_time)) = (distance, travel_time) else {
            panic!("expected valid edge measurements");
        };
        Edge::new(
            EdgeId::new(1),
            NodeId::new(10),
            NodeId::new(11),
            distance,
            travel_time,
        )
    }

    #[test]
    fn distance_model_selects_only_edge_distance() {
        let model = DistanceCost;
        let edge = edge();

        assert_eq!(model.kind(), CostKind::Distance);
        assert_eq!(
            model.edge_cost(&edge, &RoutingContext::new()),
            Ok(RouteCost::from_distance(edge.distance()))
        );
    }

    #[test]
    fn travel_time_model_selects_only_base_travel_time() {
        let model = TravelTimeCost;
        let edge = edge();

        assert_eq!(model.kind(), CostKind::TravelTime);
        assert_eq!(
            model.edge_cost(&edge, &RoutingContext::new()),
            Ok(RouteCost::from_travel_time(edge.base_travel_time()))
        );
    }
}
