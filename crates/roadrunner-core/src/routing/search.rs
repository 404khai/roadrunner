use std::collections::HashMap;

use crate::cost::{CostKind, CostModel, RouteCost, RoutingContext};
use crate::geo::Meters;
use crate::graph::{Edge, EdgeId, Graph, NodeId};

use super::{RouteEndpoint, RouteResult, RoutingAlgorithm, RoutingError};

pub(super) fn validate_endpoints(
    graph: &Graph,
    source: NodeId,
    destination: NodeId,
) -> Result<(), RoutingError> {
    if !graph.contains_node(source) {
        return Err(RoutingError::NodeNotFound {
            endpoint: RouteEndpoint::Source,
            node_id: source,
        });
    }
    if !graph.contains_node(destination) {
        return Err(RoutingError::NodeNotFound {
            endpoint: RouteEndpoint::Destination,
            node_id: destination,
        });
    }
    Ok(())
}

pub(super) fn evaluate_edge_cost(
    cost_model: &dyn CostModel,
    edge: &Edge,
    context: RoutingContext,
    expected_kind: CostKind,
) -> Result<RouteCost, RoutingError> {
    let edge_cost =
        cost_model
            .edge_cost(edge, &context)
            .map_err(|source| RoutingError::CostEvaluation {
                edge_id: edge.id(),
                source,
            })?;
    if edge_cost.kind() != expected_kind {
        return Err(RoutingError::CostKindMismatch {
            edge_id: edge.id(),
            expected: expected_kind,
            actual: edge_cost.kind(),
        });
    }
    Ok(edge_cost)
}

pub(super) fn sorted_outgoing(graph: &Graph, node_id: NodeId) -> Result<Vec<&Edge>, RoutingError> {
    let mut outgoing: Vec<&Edge> = graph
        .neighbors(node_id)
        .map_err(|source| RoutingError::Graph { source })?
        .iter()
        .collect();
    outgoing.sort_unstable_by(|left, right| {
        left.to()
            .cmp(&right.to())
            .then_with(|| left.id().cmp(&right.id()))
    });
    Ok(outgoing)
}

pub(super) fn reconstruct_route(
    graph: &Graph,
    algorithm: RoutingAlgorithm,
    source: NodeId,
    destination: NodeId,
    predecessors: &HashMap<NodeId, EdgeId>,
    total_cost: RouteCost,
    visited_nodes: usize,
) -> Result<RouteResult, RoutingError> {
    let mut path = vec![destination];
    let mut edges = Vec::new();
    let mut total_distance = Meters::ZERO;
    let mut cursor = destination;

    while cursor != source {
        if path.len() > graph.node_count() {
            return Err(RoutingError::PredecessorCycle);
        }
        let edge_id = predecessors
            .get(&cursor)
            .copied()
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

        total_distance = total_distance
            .checked_add(edge.distance())
            .map_err(|source| RoutingError::DistanceAccumulation { edge_id, source })?;
        edges.push(edge_id);
        cursor = edge.from();
        path.push(cursor);
    }

    path.reverse();
    edges.reverse();
    Ok(RouteResult::new(
        algorithm,
        path,
        edges,
        total_distance,
        total_cost,
        visited_nodes,
    ))
}
