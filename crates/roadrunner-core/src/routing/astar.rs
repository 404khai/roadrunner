use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::cost::{CostKind, CostModel, RouteCost, RoutingContext};
use crate::geo::{Coordinate, Meters, haversine_distance};
use crate::graph::{Graph, NodeId};

use super::search::{evaluate_edge_cost, reconstruct_route, sorted_outgoing, validate_endpoints};
use super::{RouteResult, RoutingAlgorithm, RoutingError};

#[derive(Debug, Clone, Copy)]
struct QueueEntry {
    node_id: NodeId,
    route_cost: f64,
    estimated_total: f64,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id
            && self.route_cost.total_cmp(&other.route_cost).is_eq()
            && self
                .estimated_total
                .total_cmp(&other.estimated_total)
                .is_eq()
    }
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
            .then_with(|| other.route_cost.total_cmp(&self.route_cost))
            .then_with(|| other.node_id.cmp(&self.node_id))
    }
}

#[derive(Debug, Clone, Copy)]
struct ScaledHaversine {
    destination: Coordinate,
    cost_kind: CostKind,
    cost_per_meter: f64,
}

impl ScaledHaversine {
    fn prepare(
        graph: &Graph,
        destination: NodeId,
        cost_model: &dyn CostModel,
        context: RoutingContext,
    ) -> Result<Self, RoutingError> {
        let destination = graph
            .node(destination)
            .ok_or(RoutingError::MissingHeuristicNode {
                node_id: destination,
            })?
            .coordinate();
        let cost_kind = cost_model.kind();
        let mut cost_per_meter: Option<f64> = None;
        for edge in graph.edges() {
            let from = graph
                .node(edge.from())
                .ok_or(RoutingError::MissingEdgeNode {
                    edge_id: edge.id(),
                    node_id: edge.from(),
                })?
                .coordinate();
            let to = graph
                .node(edge.to())
                .ok_or(RoutingError::MissingEdgeNode {
                    edge_id: edge.id(),
                    node_id: edge.to(),
                })?
                .coordinate();
            let geometric_distance = haversine_distance(from, to).value();
            if geometric_distance == 0.0 {
                continue;
            }

            let edge_cost = evaluate_edge_cost(cost_model, edge, context, cost_kind)?;
            let ratio = edge_cost.value() / geometric_distance;
            if !ratio.is_finite() {
                continue;
            }
            cost_per_meter = Some(cost_per_meter.map_or(ratio, |current| current.min(ratio)));
        }

        Ok(Self {
            destination,
            cost_kind,
            cost_per_meter: cost_per_meter.unwrap_or(0.0),
        })
    }

    fn estimate(self, graph: &Graph, node_id: NodeId) -> Result<RouteCost, RoutingError> {
        let coordinate = graph
            .node(node_id)
            .ok_or(RoutingError::MissingHeuristicNode { node_id })?
            .coordinate();
        let estimate =
            haversine_distance(coordinate, self.destination).value() * self.cost_per_meter;
        RouteCost::new(self.cost_kind, estimate)
            .map_err(|source| RoutingError::HeuristicEvaluation { node_id, source })
    }
}

/// Calculates the lowest-cost directed route using A* search.
///
/// Haversine distance is scaled by the graph's minimum actual edge cost per
/// geodesic meter under the selected model and context. That lower bound makes the
/// heuristic admissible and consistent even when edge cost is travel time or a
/// custom non-negative additive value. If no positive geographic edge exists, the
/// scale is zero and A* safely degenerates to Dijkstra's search order.
///
/// # Errors
///
/// Returns [`RoutingError`] for absent endpoints, unreachable destinations, cost
/// failures, or graph/predecessor invariant violations.
pub fn astar(
    graph: &Graph,
    source: NodeId,
    destination: NodeId,
    cost_model: &dyn CostModel,
    context: &RoutingContext,
) -> Result<RouteResult, RoutingError> {
    validate_endpoints(graph, source, destination)?;

    let cost_kind = cost_model.kind();
    let zero_cost = RouteCost::zero(cost_kind);
    if source == destination {
        return Ok(RouteResult::new(
            RoutingAlgorithm::AStar,
            vec![source],
            Vec::new(),
            Meters::ZERO,
            zero_cost,
            1,
        ));
    }

    let heuristic = ScaledHaversine::prepare(graph, destination, cost_model, *context)?;
    let initial_estimate = heuristic.estimate(graph, source)?;
    let mut best_costs = HashMap::new();
    let mut predecessors = HashMap::new();
    let mut queue = BinaryHeap::new();
    let mut visited_nodes = 0;
    best_costs.insert(source, zero_cost);
    queue.push(QueueEntry {
        node_id: source,
        route_cost: zero_cost.value(),
        estimated_total: initial_estimate.value(),
    });

    while let Some(entry) = queue.pop() {
        let Some(current_cost) = best_costs.get(&entry.node_id).copied() else {
            continue;
        };
        if entry.route_cost.total_cmp(&current_cost.value()).is_gt() {
            continue;
        }

        visited_nodes += 1;
        if entry.node_id == destination {
            return reconstruct_route(
                graph,
                RoutingAlgorithm::AStar,
                source,
                destination,
                &predecessors,
                current_cost,
                visited_nodes,
            );
        }

        for edge in sorted_outgoing(graph, entry.node_id)? {
            let edge_cost = evaluate_edge_cost(cost_model, edge, *context, cost_kind)?;
            let candidate_cost = current_cost.checked_add(edge_cost).map_err(|source| {
                RoutingError::CostAccumulation {
                    edge_id: edge.id(),
                    source,
                }
            })?;
            let improves = best_costs
                .get(&edge.to())
                .is_none_or(|known_cost| candidate_cost < *known_cost);
            if !improves {
                continue;
            }

            let estimate = heuristic.estimate(graph, edge.to())?;
            let estimated_total = candidate_cost.checked_add(estimate).map_err(|source| {
                RoutingError::EstimatedTotal {
                    node_id: edge.to(),
                    source,
                }
            })?;
            best_costs.insert(edge.to(), candidate_cost);
            predecessors.insert(edge.to(), edge.id());
            queue.push(QueueEntry {
                node_id: edge.to(),
                route_cost: candidate_cost.value(),
                estimated_total: estimated_total.value(),
            });
        }
    }

    Err(RoutingError::NoRoute {
        source_node: source,
        destination,
    })
}

#[cfg(test)]
mod tests {
    use crate::cost::{DistanceCost, TravelTimeCost};
    use crate::geo::Seconds;
    use crate::graph::{Edge, EdgeId, Node};
    use crate::routing::{RouteEndpoint, dijkstra};

    use super::*;

    const A: NodeId = NodeId::new(1);
    const B: NodeId = NodeId::new(2);
    const C: NodeId = NodeId::new(3);
    const D: NodeId = NodeId::new(4);

    fn coordinate(latitude: f64, longitude: f64) -> Coordinate {
        match Coordinate::new(latitude, longitude) {
            Ok(coordinate) => coordinate,
            Err(error) => panic!("test coordinate is invalid: {error}"),
        }
    }

    fn seconds(value: f64) -> Seconds {
        match Seconds::new(value) {
            Ok(seconds) => seconds,
            Err(error) => panic!("test duration is invalid: {error}"),
        }
    }

    fn graph_with_nodes(nodes: &[(NodeId, Coordinate)]) -> Graph {
        let mut graph = Graph::new();
        for (node_id, coordinate) in nodes {
            let result = graph.add_node(Node::new(*node_id, *coordinate));
            assert!(result.is_ok(), "test node insertion failed: {result:?}");
        }
        graph
    }

    fn coordinate_for(graph: &Graph, node_id: NodeId) -> Coordinate {
        match graph.node(node_id) {
            Some(node) => node.coordinate(),
            None => panic!("test node {node_id} is missing"),
        }
    }

    fn add_geographic_edge(graph: &mut Graph, id: u64, from: NodeId, to: NodeId) {
        let distance = haversine_distance(coordinate_for(graph, from), coordinate_for(graph, to));
        add_geographic_edge_with_time(graph, id, from, to, distance.value());
    }

    fn add_geographic_edge_with_time(
        graph: &mut Graph,
        id: u64,
        from: NodeId,
        to: NodeId,
        travel_time_seconds: f64,
    ) {
        let distance = haversine_distance(coordinate_for(graph, from), coordinate_for(graph, to));
        let edge = Edge::new(
            EdgeId::new(id),
            from,
            to,
            distance,
            seconds(travel_time_seconds),
        );
        let result = graph.add_edge(edge);
        assert!(result.is_ok(), "test edge insertion failed: {result:?}");
    }

    fn guided_graph() -> Graph {
        let mut graph = graph_with_nodes(&[
            (A, coordinate(0.0, 0.0)),
            (B, coordinate(0.0, 0.01)),
            (C, coordinate(0.01, 0.0)),
            (D, coordinate(0.0, 0.02)),
        ]);
        add_geographic_edge(&mut graph, 10, A, B);
        add_geographic_edge(&mut graph, 11, B, D);
        add_geographic_edge(&mut graph, 12, A, C);
        graph
    }

    #[test]
    fn matches_dijkstra_optimal_cost_and_path() {
        let graph = guided_graph();
        let context = RoutingContext::new();
        let dijkstra = dijkstra(&graph, A, D, &DistanceCost, &context);
        let astar = astar(&graph, A, D, &DistanceCost, &context);
        let (Ok(dijkstra), Ok(astar)) = (dijkstra, astar) else {
            panic!("expected both algorithms to find a route");
        };

        assert_eq!(astar.algorithm(), RoutingAlgorithm::AStar);
        assert_eq!(astar.total_cost(), dijkstra.total_cost());
        assert_eq!(astar.path(), dijkstra.path());
        assert_eq!(astar.path(), &[A, B, D]);
    }

    #[test]
    fn explores_fewer_nodes_when_geography_guides_the_search() {
        let graph = guided_graph();
        let context = RoutingContext::new();
        let dijkstra = dijkstra(&graph, A, D, &DistanceCost, &context);
        let astar = astar(&graph, A, D, &DistanceCost, &context);
        let (Ok(dijkstra), Ok(astar)) = (dijkstra, astar) else {
            panic!("expected both algorithms to find a route");
        };

        assert_eq!(dijkstra.visited_nodes(), 4);
        assert_eq!(astar.visited_nodes(), 3);
    }

    #[test]
    fn remains_admissible_for_travel_time_cost() {
        let mut graph = graph_with_nodes(&[
            (A, coordinate(0.0, 0.0)),
            (B, coordinate(0.0, 0.01)),
            (C, coordinate(0.01, 0.0)),
            (D, coordinate(0.0, 0.02)),
        ]);
        add_geographic_edge_with_time(&mut graph, 10, A, B, 100.0);
        add_geographic_edge_with_time(&mut graph, 11, B, D, 100.0);
        add_geographic_edge_with_time(&mut graph, 12, A, C, 1.0);
        add_geographic_edge_with_time(&mut graph, 13, C, D, 1.0);
        let context = RoutingContext::new();
        let dijkstra = dijkstra(&graph, A, D, &TravelTimeCost, &context);
        let astar = astar(&graph, A, D, &TravelTimeCost, &context);
        let (Ok(dijkstra), Ok(astar)) = (dijkstra, astar) else {
            panic!("expected both algorithms to find a route");
        };

        assert_eq!(astar.total_cost(), dijkstra.total_cost());
        assert_eq!(astar.path(), dijkstra.path());
        assert_eq!(astar.path(), &[A, C, D]);
    }

    #[test]
    fn zero_geographic_span_safely_degenerates_to_dijkstra() {
        let mut graph = graph_with_nodes(&[
            (A, Coordinate::ORIGIN),
            (B, Coordinate::ORIGIN),
            (D, Coordinate::ORIGIN),
        ]);
        add_geographic_edge(&mut graph, 10, A, B);
        add_geographic_edge(&mut graph, 11, B, D);
        let context = RoutingContext::new();
        let dijkstra = dijkstra(&graph, A, D, &DistanceCost, &context);
        let astar = astar(&graph, A, D, &DistanceCost, &context);
        let (Ok(dijkstra), Ok(astar)) = (dijkstra, astar) else {
            panic!("expected both algorithms to find a route");
        };

        assert_eq!(astar.total_cost(), dijkstra.total_cost());
        assert_eq!(astar.visited_nodes(), dijkstra.visited_nodes());
    }

    #[test]
    fn source_equal_to_destination_returns_a_zero_cost_route() {
        let graph = graph_with_nodes(&[(A, Coordinate::ORIGIN)]);

        let result = astar(&graph, A, A, &DistanceCost, &RoutingContext::new());
        let Ok(result) = result else {
            panic!("expected a trivial route: {result:?}");
        };

        assert_eq!(result.algorithm(), RoutingAlgorithm::AStar);
        assert_eq!(result.path(), &[A]);
        assert_eq!(result.total_cost(), RouteCost::zero(CostKind::Distance));
        assert_eq!(result.visited_nodes(), 1);
    }

    #[test]
    fn reports_unreachable_destinations_in_disconnected_graphs() {
        let graph = graph_with_nodes(&[(A, coordinate(0.0, 0.0)), (B, coordinate(0.0, 0.01))]);

        assert_eq!(
            astar(&graph, A, B, &DistanceCost, &RoutingContext::new()),
            Err(RoutingError::NoRoute {
                source_node: A,
                destination: B,
            })
        );
    }

    #[test]
    fn rejects_unknown_endpoints() {
        let graph = graph_with_nodes(&[(A, Coordinate::ORIGIN)]);

        assert_eq!(
            astar(&graph, B, A, &DistanceCost, &RoutingContext::new()),
            Err(RoutingError::NodeNotFound {
                endpoint: RouteEndpoint::Source,
                node_id: B,
            })
        );
        assert_eq!(
            astar(&graph, A, B, &DistanceCost, &RoutingContext::new()),
            Err(RoutingError::NodeNotFound {
                endpoint: RouteEndpoint::Destination,
                node_id: B,
            })
        );
    }
}
