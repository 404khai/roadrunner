use std::collections::HashMap;

use super::{Edge, EdgeId, GraphError, Node, NodeId};

/// A directed graph backed by outgoing adjacency lists.
///
/// Edges are stored once, in their source node's adjacency list. A separate edge
/// index provides stable identity lookup and removal without duplicating edge data.
#[derive(Debug, Default)]
pub struct Graph {
    nodes: HashMap<NodeId, Node>,
    outgoing: HashMap<NodeId, Vec<Edge>>,
    edge_sources: HashMap<EdgeId, NodeId>,
}

impl Graph {
    /// Creates an empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a node and initializes its empty outgoing adjacency list.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::DuplicateNode`] when the node identity already exists.
    pub fn add_node(&mut self, node: Node) -> Result<(), GraphError> {
        let id = node.id();
        if self.nodes.contains_key(&id) {
            return Err(GraphError::DuplicateNode { id });
        }

        self.nodes.insert(id, node);
        self.outgoing.insert(id, Vec::new());
        Ok(())
    }

    /// Adds a directed edge to its source node's adjacency list.
    ///
    /// Parallel edges are accepted when they have distinct identities.
    ///
    /// # Errors
    ///
    /// Returns an error when the edge identity already exists, either endpoint is
    /// absent, or the graph's internal adjacency invariant is broken.
    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
        if self.edge_sources.contains_key(&edge.id()) {
            return Err(GraphError::DuplicateEdge { id: edge.id() });
        }
        if !self.nodes.contains_key(&edge.from()) {
            return Err(GraphError::MissingSourceNode {
                edge_id: edge.id(),
                node_id: edge.from(),
            });
        }
        if !self.nodes.contains_key(&edge.to()) {
            return Err(GraphError::MissingDestinationNode {
                edge_id: edge.id(),
                node_id: edge.to(),
            });
        }

        let outgoing = self
            .outgoing
            .get_mut(&edge.from())
            .ok_or(GraphError::MissingAdjacency {
                node_id: edge.from(),
            })?;
        outgoing.push(edge);
        self.edge_sources.insert(edge.id(), edge.from());
        Ok(())
    }

    /// Removes and returns an edge by identity.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::EdgeNotFound`] when the identity is unknown, or an
    /// invariant error when the edge index and adjacency storage disagree.
    pub fn remove_edge(&mut self, id: EdgeId) -> Result<Edge, GraphError> {
        let source = self
            .edge_sources
            .get(&id)
            .copied()
            .ok_or(GraphError::EdgeNotFound { id })?;
        let outgoing = self
            .outgoing
            .get_mut(&source)
            .ok_or(GraphError::MissingAdjacency { node_id: source })?;
        let position = outgoing.iter().position(|edge| edge.id() == id).ok_or(
            GraphError::MissingStoredEdge {
                edge_id: id,
                source_node: source,
            },
        )?;
        let edge = outgoing.remove(position);
        self.edge_sources.remove(&id);
        Ok(edge)
    }

    /// Returns the node with the supplied identity.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Returns the edge with the supplied identity.
    #[must_use]
    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        let source = self.edge_sources.get(&id)?;
        self.outgoing
            .get(source)?
            .iter()
            .find(|edge| edge.id() == id)
    }

    /// Returns the outgoing edges for a node in insertion order.
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::NodeNotFound`] when the node is absent, or
    /// [`GraphError::MissingAdjacency`] when an internal invariant is broken.
    pub fn neighbors(&self, id: NodeId) -> Result<&[Edge], GraphError> {
        if !self.nodes.contains_key(&id) {
            return Err(GraphError::NodeNotFound { id });
        }

        self.outgoing
            .get(&id)
            .map(Vec::as_slice)
            .ok_or(GraphError::MissingAdjacency { node_id: id })
    }

    /// Returns whether a node identity exists in the graph.
    #[must_use]
    pub fn contains_node(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Returns the number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of directed edges in the graph.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edge_sources.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::{Coordinate, Meters, Seconds};

    const A: NodeId = NodeId::new(1);
    const B: NodeId = NodeId::new(2);
    const C: NodeId = NodeId::new(3);
    const D: NodeId = NodeId::new(4);

    fn node(id: NodeId) -> Node {
        Node::new(id, Coordinate::ORIGIN)
    }

    fn edge(id: EdgeId, from: NodeId, to: NodeId) -> Edge {
        Edge::new(id, from, to, Meters::ZERO, Seconds::ZERO)
    }

    fn add_nodes(graph: &mut Graph, ids: &[NodeId]) {
        for id in ids {
            assert_eq!(graph.add_node(node(*id)), Ok(()));
        }
    }

    fn edge_ids(graph: &Graph, node_id: NodeId) -> Vec<EdgeId> {
        let Ok(edges) = graph.neighbors(node_id) else {
            panic!("expected node {node_id} to have an adjacency list");
        };
        edges.iter().map(|edge| edge.id()).collect()
    }

    #[test]
    fn empty_graph_has_no_nodes_or_edges() {
        let graph = Graph::new();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.contains_node(A));
        assert_eq!(graph.node(A), None);
        assert_eq!(graph.neighbors(A), Err(GraphError::NodeNotFound { id: A }));
    }

    #[test]
    fn single_node_has_an_empty_adjacency_list() {
        let mut graph = Graph::new();

        assert_eq!(graph.add_node(node(A)), Ok(()));

        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.contains_node(A));
        assert_eq!(graph.node(A), Some(&node(A)));
        assert_eq!(graph.neighbors(A), Ok([].as_slice()));
    }

    #[test]
    fn directed_edge_is_visible_only_from_its_source() {
        let mut graph = Graph::new();
        add_nodes(&mut graph, &[A, B]);
        let edge = edge(EdgeId::new(10), A, B);

        assert_eq!(graph.add_edge(edge), Ok(()));

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.edge(edge.id()), Some(&edge));
        assert_eq!(edge_ids(&graph, A), vec![edge.id()]);
        assert!(edge_ids(&graph, B).is_empty());
    }

    #[test]
    fn reciprocal_edges_model_a_bidirectional_road() {
        let mut graph = Graph::new();
        add_nodes(&mut graph, &[A, B]);
        let forward = edge(EdgeId::new(10), A, B);
        let reverse = edge(EdgeId::new(11), B, A);

        assert_eq!(graph.add_edge(forward), Ok(()));
        assert_eq!(graph.add_edge(reverse), Ok(()));

        assert_eq!(edge_ids(&graph, A), vec![forward.id()]);
        assert_eq!(edge_ids(&graph, B), vec![reverse.id()]);
    }

    #[test]
    fn cycles_preserve_each_outgoing_adjacency() {
        let mut graph = Graph::new();
        add_nodes(&mut graph, &[A, B, C]);
        let ab = edge(EdgeId::new(10), A, B);
        let bc = edge(EdgeId::new(11), B, C);
        let ca = edge(EdgeId::new(12), C, A);

        assert_eq!(graph.add_edge(ab), Ok(()));
        assert_eq!(graph.add_edge(bc), Ok(()));
        assert_eq!(graph.add_edge(ca), Ok(()));

        assert_eq!(edge_ids(&graph, A), vec![ab.id()]);
        assert_eq!(edge_ids(&graph, B), vec![bc.id()]);
        assert_eq!(edge_ids(&graph, C), vec![ca.id()]);
    }

    #[test]
    fn disconnected_components_remain_independent() {
        let mut graph = Graph::new();
        add_nodes(&mut graph, &[A, B, C, D]);
        let ab = edge(EdgeId::new(10), A, B);
        let cd = edge(EdgeId::new(11), C, D);

        assert_eq!(graph.add_edge(ab), Ok(()));
        assert_eq!(graph.add_edge(cd), Ok(()));

        assert_eq!(edge_ids(&graph, A), vec![ab.id()]);
        assert!(edge_ids(&graph, B).is_empty());
        assert_eq!(edge_ids(&graph, C), vec![cd.id()]);
        assert!(edge_ids(&graph, D).is_empty());
    }

    #[test]
    fn edge_insertion_rejects_missing_endpoints_without_mutation() {
        let mut graph = Graph::new();
        assert_eq!(graph.add_node(node(A)), Ok(()));
        let missing_source = edge(EdgeId::new(10), B, A);
        let missing_destination = edge(EdgeId::new(11), A, B);

        assert_eq!(
            graph.add_edge(missing_source),
            Err(GraphError::MissingSourceNode {
                edge_id: missing_source.id(),
                node_id: B,
            })
        );
        assert_eq!(
            graph.add_edge(missing_destination),
            Err(GraphError::MissingDestinationNode {
                edge_id: missing_destination.id(),
                node_id: B,
            })
        );
        assert_eq!(graph.edge_count(), 0);
        assert!(edge_ids(&graph, A).is_empty());
    }

    #[test]
    fn duplicate_identities_are_rejected_but_parallel_edges_are_allowed() {
        let mut graph = Graph::new();
        add_nodes(&mut graph, &[A, B]);
        let first = edge(EdgeId::new(10), A, B);
        let parallel = edge(EdgeId::new(11), A, B);

        assert_eq!(
            graph.add_node(node(A)),
            Err(GraphError::DuplicateNode { id: A })
        );
        assert_eq!(graph.add_edge(first), Ok(()));
        assert_eq!(
            graph.add_edge(edge(first.id(), B, A)),
            Err(GraphError::DuplicateEdge { id: first.id() })
        );
        assert_eq!(graph.add_edge(parallel), Ok(()));
        assert_eq!(edge_ids(&graph, A), vec![first.id(), parallel.id()]);
        assert_eq!(graph.edge_count(), 2);
    }

    #[test]
    fn removing_an_edge_updates_lookup_adjacency_and_count() {
        let mut graph = Graph::new();
        add_nodes(&mut graph, &[A, B]);
        let edge = edge(EdgeId::new(10), A, B);
        assert_eq!(graph.add_edge(edge), Ok(()));

        assert_eq!(graph.remove_edge(edge.id()), Ok(edge));

        assert_eq!(graph.edge(edge.id()), None);
        assert!(edge_ids(&graph, A).is_empty());
        assert_eq!(graph.edge_count(), 0);
        assert_eq!(
            graph.remove_edge(edge.id()),
            Err(GraphError::EdgeNotFound { id: edge.id() })
        );
    }
}
