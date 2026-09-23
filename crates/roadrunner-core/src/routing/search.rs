use crate::cost::{
    RouteCost, RoutingContext, SearchCapability, TraversalEvaluation, TraversalEvaluator,
    TraversalState,
};
use crate::geo::{Meters, Seconds};
use crate::graph::{EdgeId, FrozenGraph, NodeId};

use super::result::RouteMetrics;
use super::{RouteEndpoint, RouteResult, RoutingAlgorithm, RoutingError};

#[derive(Debug, Clone, Copy)]
pub(super) struct Label {
    pub(super) objective: RouteCost,
    pub(super) elapsed: Seconds,
}

pub(super) fn index(id: NodeId) -> usize {
    id.value() as usize
}

pub(super) fn validate_request(
    graph: &FrozenGraph,
    source: NodeId,
    destination: NodeId,
    evaluator: &dyn TraversalEvaluator,
) -> Result<(), RoutingError> {
    if graph.node(source).is_none() {
        return Err(RoutingError::NodeNotFound {
            endpoint: RouteEndpoint::Source,
            node_id: source,
        });
    }
    if graph.node(destination).is_none() {
        return Err(RoutingError::NodeNotFound {
            endpoint: RouteEndpoint::Destination,
            node_id: destination,
        });
    }
    if !evaluator.supports_graph(graph) {
        return Err(RoutingError::EvaluatorGraphMismatch);
    }
    if matches!(
        evaluator.capability(),
        SearchCapability::RequiresExpandedState | SearchCapability::Unsupported
    ) {
        return Err(RoutingError::UnsupportedCapability {
            capability: evaluator.capability(),
        });
    }
    if evaluator.capability() == SearchCapability::FifoEarliestArrival
        && evaluator.kind() != crate::cost::CostKind::TravelTime
    {
        return Err(RoutingError::UnsupportedCapability {
            capability: evaluator.capability(),
        });
    }
    Ok(())
}

pub(super) fn evaluate(
    graph: &FrozenGraph,
    evaluator: &dyn TraversalEvaluator,
    edge: &crate::graph::DirectedEdge,
    current: Label,
    context: RoutingContext,
) -> Result<Option<Label>, RoutingError> {
    let segment = graph
        .segment(edge.segment())
        .ok_or(RoutingError::MissingRouteEdge { edge_id: edge.id() })?;
    let result = evaluator
        .evaluate(
            edge,
            segment,
            TraversalState::new(current.elapsed),
            &context,
        )
        .map_err(|source| RoutingError::TraversalEvaluation {
            edge_id: edge.id(),
            source,
        })?;
    let TraversalEvaluation::Traversable {
        objective_cost,
        travel_time,
    } = result
    else {
        return Ok(None);
    };
    if objective_cost.kind() != evaluator.kind() {
        return Err(RoutingError::CostKindMismatch {
            edge_id: edge.id(),
            expected: evaluator.kind(),
            actual: objective_cost.kind(),
        });
    }
    if evaluator.capability() == SearchCapability::FifoEarliestArrival
        && objective_cost
            .value()
            .total_cmp(&travel_time.value())
            .is_ne()
    {
        return Err(RoutingError::FifoObjectiveMismatch { edge_id: edge.id() });
    }
    let objective = current
        .objective
        .checked_add(objective_cost)
        .map_err(|source| RoutingError::CostAccumulation {
            edge_id: edge.id(),
            source,
        })?;
    let elapsed = current.elapsed.checked_add(travel_time).map_err(|source| {
        RoutingError::TimeAccumulation {
            edge_id: edge.id(),
            source,
        }
    })?;
    Ok(Some(Label { objective, elapsed }))
}

pub(super) fn reconstruct_route(
    graph: &FrozenGraph,
    algorithm: RoutingAlgorithm,
    source: NodeId,
    destination: NodeId,
    predecessors: &[Option<EdgeId>],
    label: Label,
    expanded_states: usize,
) -> Result<RouteResult, RoutingError> {
    let mut path = vec![destination];
    let mut edges = Vec::new();
    let mut cursor = destination;
    while cursor != source {
        if path.len() > graph.node_count() {
            return Err(RoutingError::PredecessorCycle);
        }
        let edge_id = predecessors
            .get(index(cursor))
            .copied()
            .flatten()
            .ok_or(RoutingError::MissingPredecessor { node_id: cursor })?;
        let edge = graph
            .edge(edge_id)
            .ok_or(RoutingError::MissingRouteEdge { edge_id })?;
        if edge.to() != cursor {
            return Err(RoutingError::InvalidPredecessorEdge {
                edge_id,
                expected_to: cursor,
                actual_to: edge.to(),
            });
        }
        edges.push(edge_id);
        cursor = edge.from();
        path.push(cursor);
    }
    path.reverse();
    edges.reverse();
    let mut total_distance = Meters::ZERO;
    for edge_id in &edges {
        let edge = graph
            .edge(*edge_id)
            .ok_or(RoutingError::MissingRouteEdge { edge_id: *edge_id })?;
        let segment = graph
            .segment(edge.segment())
            .ok_or(RoutingError::MissingRouteEdge { edge_id: *edge_id })?;
        total_distance = total_distance
            .checked_add(segment.distance())
            .map_err(|source| RoutingError::DistanceAccumulation {
                edge_id: *edge_id,
                source,
            })?;
    }
    Ok(RouteResult::new(
        graph.snapshot_id(),
        graph.metadata().snapshot_digest().to_owned(),
        algorithm,
        path,
        edges,
        RouteMetrics {
            total_distance,
            total_cost: label.objective,
            elapsed_travel_time: label.elapsed,
            expanded_states,
        },
    ))
}

pub(super) fn expanded_index(incoming: EdgeId) -> usize {
    incoming.value() as usize + 1
}

pub(super) fn reconstruct_expanded_route(
    graph: &FrozenGraph,
    algorithm: RoutingAlgorithm,
    source: NodeId,
    destination_state: usize,
    predecessors: &[Option<usize>],
    label: Label,
    expanded_states: usize,
) -> Result<RouteResult, RoutingError> {
    let mut edges = Vec::new();
    let mut state = destination_state;
    while state != 0 {
        if edges.len() > graph.edge_count() {
            return Err(RoutingError::PredecessorCycle);
        }
        let edge_id =
            EdgeId::new(u32::try_from(state - 1).map_err(|_| RoutingError::PredecessorCycle)?);
        edges.push(edge_id);
        state =
            predecessors
                .get(state)
                .copied()
                .flatten()
                .ok_or(RoutingError::MissingPredecessor {
                    node_id: graph
                        .edge(edge_id)
                        .ok_or(RoutingError::MissingRouteEdge { edge_id })?
                        .from(),
                })?;
    }
    edges.reverse();
    let mut path = Vec::with_capacity(edges.len() + 1);
    path.push(source);
    let mut total_distance = Meters::ZERO;
    for &edge_id in &edges {
        let edge = graph
            .edge(edge_id)
            .ok_or(RoutingError::MissingRouteEdge { edge_id })?;
        path.push(edge.to());
        let segment = graph
            .segment(edge.segment())
            .ok_or(RoutingError::MissingRouteEdge { edge_id })?;
        total_distance = total_distance
            .checked_add(segment.distance())
            .map_err(|source| RoutingError::DistanceAccumulation { edge_id, source })?;
    }
    Ok(RouteResult::new(
        graph.snapshot_id(),
        graph.metadata().snapshot_digest().to_owned(),
        algorithm,
        path,
        edges,
        RouteMetrics {
            total_distance,
            total_cost: label.objective,
            elapsed_travel_time: label.elapsed,
            expanded_states,
        },
    ))
}
