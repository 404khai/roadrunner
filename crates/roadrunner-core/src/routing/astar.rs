use std::cmp::Ordering;
use std::collections::BinaryHeap;

use crate::cost::{RouteCost, RoutingContext, TraversalEvaluator};
use crate::geo::{Meters, Seconds};
use crate::graph::{EdgeId, FrozenGraph, NodeId};

use super::result::RouteMetrics;
use super::search::{
    Label, evaluate, expanded_index, index, reconstruct_expanded_route, reconstruct_route,
    validate_request,
};
use super::{Heuristic, RouteResult, RoutingAlgorithm, RoutingError};

#[derive(Debug, Clone, Copy, PartialEq)]
struct QueueEntry {
    node: NodeId,
    objective: f64,
    estimated_total: f64,
}
impl Eq for QueueEntry {}
impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimated_total
            .total_cmp(&self.estimated_total)
            .then_with(|| other.objective.total_cmp(&self.objective))
            .then_with(|| other.node.cmp(&self.node))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExpandedQueueEntry {
    node: NodeId,
    incoming: Option<EdgeId>,
    objective: f64,
    estimated_total: f64,
}
impl Eq for ExpandedQueueEntry {}
impl PartialOrd for ExpandedQueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ExpandedQueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .estimated_total
            .total_cmp(&self.estimated_total)
            .then_with(|| other.objective.total_cmp(&self.objective))
            .then_with(|| other.incoming.cmp(&self.incoming))
    }
}

/// Calculates a lowest-cost route using A* and an explicit compatible heuristic.
///
/// # Errors
///
/// Returns a typed error for invalid endpoints, incompatible policies,
/// evaluation failure, arithmetic failure, or no permitted route.
#[allow(clippy::too_many_lines)]
pub fn astar(
    graph: &FrozenGraph,
    source: NodeId,
    destination: NodeId,
    evaluator: &dyn TraversalEvaluator,
    heuristic: &dyn Heuristic,
    context: &RoutingContext,
) -> Result<RouteResult, RoutingError> {
    validate_request(graph, source, destination, evaluator)?;
    if heuristic.kind() != evaluator.kind() {
        return Err(RoutingError::HeuristicKindMismatch {
            heuristic: heuristic.kind(),
            evaluator: evaluator.kind(),
        });
    }
    if !evaluator.supports_heuristic(heuristic.policy()) {
        return Err(RoutingError::HeuristicPolicyMismatch {
            policy: heuristic.policy(),
        });
    }
    if heuristic
        .graph_snapshot_id()
        .is_some_and(|snapshot_id| snapshot_id != graph.snapshot_id())
    {
        return Err(RoutingError::HeuristicGraphMismatch);
    }
    if !heuristic.supports(evaluator.capability()) {
        return Err(RoutingError::HeuristicCapabilityMismatch {
            capability: evaluator.capability(),
        });
    }
    let zero = Label {
        objective: RouteCost::zero(evaluator.kind()),
        elapsed: Seconds::ZERO,
    };
    if source == destination {
        return Ok(RouteResult::new(
            graph.snapshot_id(),
            graph.metadata().snapshot_digest().to_owned(),
            RoutingAlgorithm::AStar,
            vec![source],
            Vec::new(),
            RouteMetrics {
                total_distance: Meters::ZERO,
                total_cost: zero.objective,
                elapsed_travel_time: zero.elapsed,
                expanded_states: 1,
            },
        ));
    }
    let initial = heuristic
        .estimate(graph, source, destination)
        .map_err(|source_error| RoutingError::HeuristicEvaluation {
            node_id: source,
            source: source_error,
        })?;
    if graph.metadata().turn_restrictions_enforced() {
        return astar_expanded(
            graph,
            source,
            destination,
            evaluator,
            heuristic,
            context,
            zero,
            initial.value(),
        );
    }
    let mut best = vec![None; graph.node_count()];
    let mut predecessors = vec![None; graph.node_count()];
    let mut queue = BinaryHeap::new();
    let mut expanded_states = 0;
    best[index(source)] = Some(zero);
    queue.push(QueueEntry {
        node: source,
        objective: 0.0,
        estimated_total: initial.value(),
    });
    while let Some(entry) = queue.pop() {
        let Some(current) = best[index(entry.node)] else {
            continue;
        };
        if entry
            .objective
            .total_cmp(&current.objective.value())
            .is_gt()
        {
            continue;
        }
        expanded_states += 1;
        if entry.node == destination {
            return reconstruct_route(
                graph,
                RoutingAlgorithm::AStar,
                source,
                destination,
                &predecessors,
                current,
                expanded_states,
            );
        }
        for edge in
            graph
                .outgoing_edges(entry.node)
                .map_err(|source_error| RoutingError::Graph {
                    source: source_error,
                })?
        {
            let Some(candidate) = evaluate(graph, evaluator, edge, current, *context)? else {
                continue;
            };
            let target = index(edge.to());
            if best[target].is_some_and(|known: Label| candidate.objective >= known.objective) {
                continue;
            }
            let estimate =
                heuristic
                    .estimate(graph, edge.to(), destination)
                    .map_err(|source_error| RoutingError::HeuristicEvaluation {
                        node_id: edge.to(),
                        source: source_error,
                    })?;
            let total = candidate
                .objective
                .checked_add(estimate)
                .map_err(|source_error| RoutingError::EstimatedTotal {
                    node_id: edge.to(),
                    source: source_error,
                })?;
            best[target] = Some(candidate);
            predecessors[target] = Some(edge.id());
            queue.push(QueueEntry {
                node: edge.to(),
                objective: candidate.objective.value(),
                estimated_total: total.value(),
            });
        }
    }
    Err(RoutingError::NoRoute {
        source_node: source,
        destination,
    })
}

#[allow(clippy::too_many_arguments)]
fn astar_expanded(
    graph: &FrozenGraph,
    source: NodeId,
    destination: NodeId,
    evaluator: &dyn TraversalEvaluator,
    heuristic: &dyn Heuristic,
    context: &RoutingContext,
    zero: Label,
    initial_estimate: f64,
) -> Result<RouteResult, RoutingError> {
    let mut best = vec![None; graph.edge_count() + 1];
    let mut predecessors = vec![None; graph.edge_count() + 1];
    let mut queue = BinaryHeap::new();
    let mut expanded_states = 0;
    best[0] = Some(zero);
    queue.push(ExpandedQueueEntry {
        node: source,
        incoming: None,
        objective: 0.0,
        estimated_total: initial_estimate,
    });
    while let Some(entry) = queue.pop() {
        let state = entry.incoming.map_or(0, expanded_index);
        let Some(current) = best[state] else {
            continue;
        };
        if entry
            .objective
            .total_cmp(&current.objective.value())
            .is_gt()
        {
            continue;
        }
        expanded_states += 1;
        if entry.node == destination {
            return reconstruct_expanded_route(
                graph,
                RoutingAlgorithm::AStar,
                source,
                state,
                &predecessors,
                current,
                expanded_states,
            );
        }
        for edge in
            graph
                .outgoing_edges(entry.node)
                .map_err(|source_error| RoutingError::Graph {
                    source: source_error,
                })?
        {
            if entry
                .incoming
                .is_some_and(|incoming| !graph.is_maneuver_allowed(incoming, edge.id()))
            {
                continue;
            }
            let Some(candidate) = evaluate(graph, evaluator, edge, current, *context)? else {
                continue;
            };
            let target = expanded_index(edge.id());
            if best[target].is_some_and(|known: Label| candidate.objective >= known.objective) {
                continue;
            }
            let estimate =
                heuristic
                    .estimate(graph, edge.to(), destination)
                    .map_err(|source_error| RoutingError::HeuristicEvaluation {
                        node_id: edge.to(),
                        source: source_error,
                    })?;
            let total = candidate
                .objective
                .checked_add(estimate)
                .map_err(|source_error| RoutingError::EstimatedTotal {
                    node_id: edge.to(),
                    source: source_error,
                })?;
            best[target] = Some(candidate);
            predecessors[target] = Some(state);
            queue.push(ExpandedQueueEntry {
                node: edge.to(),
                incoming: Some(edge.id()),
                objective: candidate.objective.value(),
                estimated_total: total.value(),
            });
        }
    }
    Err(RoutingError::NoRoute {
        source_node: source,
        destination,
    })
}
