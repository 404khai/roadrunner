use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use crate::cost::{CostModel, RouteCost, RoutingContext};
use crate::geo::Meters;
use crate::graph::{Edge, EdgeId, Graph, NodeId};

use super::{RouteEndpoint, RouteResult, RoutingError};

#[derive(Debug, Clone, Copy)]
struct QueueEntry {
    node_id: NodeId,
    cost: f64,
}

impl PartialEq for QueueEntry {
    fn eq(&self, other: &Self) -> bool {
        self.node_id == other.node_id && self.cost.total_cmp(&other.cost).is_eq()
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
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.node_id.cmp(&self.node_id))
    }
}

/// Calculates the lowest-cost directed route using Dijkstra's algorithm.
///
/// This implementation uses a binary heap, best-cost map, and predecessor edge
/// map. Equal-cost work is ordered by stable node and edge identities, making the
/// selected path independent of hash iteration and edge insertion order.
///
/// # Errors
///
/// Returns [`RoutingError`] for absent endpoints, unreachable destinations, cost
/// failures, or graph/predecessor invariant violations.
pub fn dijkstra(
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
            vec![source],
            Vec::new(),
            Meters::ZERO,
            zero_cost,
            1,
        ));
    }

    let mut best_costs = HashMap::new();
    let mut predecessors = HashMap::new();
    let mut queue = BinaryHeap::new();
    let mut visited_nodes = 0;
    best_costs.insert(source, zero_cost);
    queue.push(QueueEntry {
        node_id: source,
        cost: zero_cost.value(),
    });

    while let Some(entry) = queue.pop() {
        let Some(current_cost) = best_costs.get(&entry.node_id).copied() else {
            continue;
        };
        if entry.cost.total_cmp(&current_cost.value()).is_gt() {
            continue;
        }

        visited_nodes += 1;
        if entry.node_id == destination {
            return reconstruct_route(
                graph,
                source,
                destination,
                &predecessors,
                current_cost,
                visited_nodes,
            );
        }

        let mut outgoing: Vec<&Edge> = graph
            .neighbors(entry.node_id)
            .map_err(|source| RoutingError::Graph { source })?
            .iter()
            .collect();
        outgoing.sort_unstable_by(|left, right| {
            left.to()
                .cmp(&right.to())
                .then_with(|| left.id().cmp(&right.id()))
        });

        for edge in outgoing {
            let edge_cost = cost_model.edge_cost(edge, context).map_err(|source| {
                RoutingError::CostEvaluation {
                    edge_id: edge.id(),
                    source,
                }
            })?;
            if edge_cost.kind() != cost_kind {
                return Err(RoutingError::CostKindMismatch {
                    edge_id: edge.id(),
                    expected: cost_kind,
                    actual: edge_cost.kind(),
                });
            }
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

            best_costs.insert(edge.to(), candidate_cost);
            predecessors.insert(edge.to(), edge.id());
            queue.push(QueueEntry {
                node_id: edge.to(),
                cost: candidate_cost.value(),
            });
        }
    }

    Err(RoutingError::NoRoute {
        source_node: source,
        destination,
    })
}

fn validate_endpoints(
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

fn reconstruct_route(
    graph: &Graph,
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
        path,
        edges,
        total_distance,
        total_cost,
        visited_nodes,
    ))
}

#[cfg(test)]
mod tests {
    use crate::cost::{CostError, CostKind, DistanceCost, TravelTimeCost};
    use crate::geo::{Coordinate, Seconds};
    use crate::graph::Node;
    use crate::routing::RoutingAlgorithm;

    use super::*;

    const A: NodeId = NodeId::new(1);
    const B: NodeId = NodeId::new(2);
    const C: NodeId = NodeId::new(3);
    const D: NodeId = NodeId::new(4);

    fn meters(value: f64) -> Meters {
        let result = Meters::new(value);
        let Ok(value) = result else {
            panic!("expected valid test distance: {result:?}");
        };
        value
    }

    fn seconds(value: f64) -> Seconds {
        let result = Seconds::new(value);
        let Ok(value) = result else {
            panic!("expected valid test duration: {result:?}");
        };
        value
    }

    fn graph_with_nodes(node_ids: &[NodeId]) -> Graph {
        let mut graph = Graph::new();
        for node_id in node_ids {
            let result = graph.add_node(Node::new(*node_id, Coordinate::ORIGIN));
            assert!(result.is_ok(), "test node insertion failed: {result:?}");
        }
        graph
    }

    fn add_edge(
        graph: &mut Graph,
        id: u64,
        from: NodeId,
        to: NodeId,
        distance: f64,
        travel_time: f64,
    ) {
        let edge = Edge::new(
            EdgeId::new(id),
            from,
            to,
            meters(distance),
            seconds(travel_time),
        );
        let result = graph.add_edge(edge);
        assert!(result.is_ok(), "test edge insertion failed: {result:?}");
    }

    #[test]
    fn finds_the_known_lowest_cost_path() {
        let mut graph = graph_with_nodes(&[A, B, C, D]);
        add_edge(&mut graph, 10, A, B, 2.0, 2.0);
        add_edge(&mut graph, 11, B, D, 2.0, 2.0);
        add_edge(&mut graph, 12, A, C, 10.0, 10.0);
        add_edge(&mut graph, 13, C, D, 1.0, 1.0);

        let result = dijkstra(&graph, A, D, &DistanceCost, &RoutingContext::new());
        let Ok(result) = result else {
            panic!("expected a route: {result:?}");
        };

        assert_eq!(result.path(), &[A, B, D]);
        assert_eq!(result.edges(), &[EdgeId::new(10), EdgeId::new(11)]);
        assert_eq!(result.algorithm(), RoutingAlgorithm::Dijkstra);
        assert_eq!(result.total_distance(), meters(4.0));
        assert_eq!(result.total_cost(), RouteCost::from_distance(meters(4.0)));
        assert_eq!(result.visited_nodes(), 3);
    }

    #[test]
    fn source_equal_to_destination_returns_a_zero_cost_route() {
        let graph = graph_with_nodes(&[A]);

        let result = dijkstra(&graph, A, A, &TravelTimeCost, &RoutingContext::new());
        let Ok(result) = result else {
            panic!("expected a trivial route: {result:?}");
        };

        assert_eq!(result.path(), &[A]);
        assert!(result.edges().is_empty());
        assert_eq!(result.total_distance(), Meters::ZERO);
        assert_eq!(result.total_cost(), RouteCost::zero(CostKind::TravelTime));
        assert_eq!(result.visited_nodes(), 1);
    }

    #[test]
    fn reports_unreachable_destinations_in_disconnected_graphs() {
        let mut graph = graph_with_nodes(&[A, B, C]);
        add_edge(&mut graph, 10, A, B, 1.0, 1.0);

        assert_eq!(
            dijkstra(&graph, A, C, &DistanceCost, &RoutingContext::new()),
            Err(RoutingError::NoRoute {
                source_node: A,
                destination: C,
            })
        );
    }

    #[test]
    fn rejects_unknown_endpoints() {
        let graph = graph_with_nodes(&[A]);

        assert_eq!(
            dijkstra(&graph, B, A, &DistanceCost, &RoutingContext::new()),
            Err(RoutingError::NodeNotFound {
                endpoint: RouteEndpoint::Source,
                node_id: B,
            })
        );
        assert_eq!(
            dijkstra(&graph, A, B, &DistanceCost, &RoutingContext::new()),
            Err(RoutingError::NodeNotFound {
                endpoint: RouteEndpoint::Destination,
                node_id: B,
            })
        );
    }

    #[test]
    fn handles_cycles_without_revisiting_equal_cost_paths() {
        let mut graph = graph_with_nodes(&[A, B, C]);
        add_edge(&mut graph, 10, A, B, 0.0, 0.0);
        add_edge(&mut graph, 11, B, A, 0.0, 0.0);
        add_edge(&mut graph, 12, B, C, 1.0, 1.0);

        let result = dijkstra(&graph, A, C, &DistanceCost, &RoutingContext::new());
        let Ok(result) = result else {
            panic!("expected a route through the cycle: {result:?}");
        };

        assert_eq!(result.path(), &[A, B, C]);
        assert_eq!(result.visited_nodes(), 3);
    }

    #[test]
    fn cost_model_changes_the_selected_route() {
        let mut graph = graph_with_nodes(&[A, B, C, D]);
        add_edge(&mut graph, 10, A, B, 1.0, 10.0);
        add_edge(&mut graph, 11, B, D, 1.0, 10.0);
        add_edge(&mut graph, 12, A, C, 5.0, 1.0);
        add_edge(&mut graph, 13, C, D, 5.0, 1.0);

        let distance = dijkstra(&graph, A, D, &DistanceCost, &RoutingContext::new());
        let travel_time = dijkstra(&graph, A, D, &TravelTimeCost, &RoutingContext::new());
        let (Ok(distance), Ok(travel_time)) = (distance, travel_time) else {
            panic!("expected both cost models to find a route");
        };

        assert_eq!(distance.path(), &[A, B, D]);
        assert_eq!(travel_time.path(), &[A, C, D]);
        assert_eq!(travel_time.total_distance(), meters(10.0));
        assert_eq!(
            travel_time.total_cost(),
            RouteCost::from_travel_time(seconds(2.0))
        );
    }

    #[test]
    fn chooses_the_cheaper_parallel_edge() {
        let mut graph = graph_with_nodes(&[A, B]);
        add_edge(&mut graph, 10, A, B, 10.0, 10.0);
        add_edge(&mut graph, 11, A, B, 2.0, 2.0);

        let result = dijkstra(&graph, A, B, &DistanceCost, &RoutingContext::new());
        let Ok(result) = result else {
            panic!("expected a route: {result:?}");
        };

        assert_eq!(result.edges(), &[EdgeId::new(11)]);
        assert_eq!(result.total_cost(), RouteCost::from_distance(meters(2.0)));
    }

    #[test]
    fn equal_cost_ties_are_independent_of_edge_insertion_order() {
        fn tied_graph(reverse: bool) -> Graph {
            let mut graph = graph_with_nodes(&[A, B, C, D]);
            let edges = if reverse {
                [(13, C, D), (12, A, C), (11, B, D), (10, A, B)]
            } else {
                [(10, A, B), (11, B, D), (12, A, C), (13, C, D)]
            };
            for (id, from, to) in edges {
                add_edge(&mut graph, id, from, to, 1.0, 1.0);
            }
            graph
        }

        for graph in [tied_graph(false), tied_graph(true)] {
            let result = dijkstra(&graph, A, D, &DistanceCost, &RoutingContext::new());
            let Ok(result) = result else {
                panic!("expected a route: {result:?}");
            };
            assert_eq!(result.path(), &[A, B, D]);
        }
    }

    #[derive(Debug)]
    struct WrongKindCost;

    impl CostModel for WrongKindCost {
        fn kind(&self) -> CostKind {
            CostKind::Distance
        }

        fn edge_cost(
            &self,
            _edge: &Edge,
            _context: &RoutingContext,
        ) -> Result<RouteCost, CostError> {
            Ok(RouteCost::zero(CostKind::TravelTime))
        }
    }

    #[test]
    fn rejects_costs_that_do_not_match_the_model_kind() {
        let mut graph = graph_with_nodes(&[A, B]);
        add_edge(&mut graph, 10, A, B, 1.0, 1.0);

        assert_eq!(
            dijkstra(&graph, A, B, &WrongKindCost, &RoutingContext::new()),
            Err(RoutingError::CostKindMismatch {
                edge_id: EdgeId::new(10),
                expected: CostKind::Distance,
                actual: CostKind::TravelTime,
            })
        );
    }
}
