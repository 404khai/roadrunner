use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::geo::{CanonicalCoordinate, Meters, Seconds, haversine_distance};

use super::{
    BuilderNodeId, BuilderSegmentId, DirectedEdge, EdgeId, EdgeProperties, GeometryRange,
    GraphError, GraphSnapshotId, Node, NodeId, Orientation, RoadSegment, RoadSegmentId,
};

/// Immutable metadata that participates in graph semantics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphMetadata {
    routing_profile: String,
    jurisdiction_policy: String,
    build_identity: GraphBuildIdentity,
    turn_restrictions_enforced: bool,
}

/// Versioned inputs that explain how a graph snapshot was compiled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphBuildIdentity {
    source_dataset: String,
    source_integrity: String,
    compiler_version: String,
    normalization_version: String,
    build_configuration: String,
}

impl GraphBuildIdentity {
    /// Creates declared graph-compilation identity.
    #[must_use]
    pub fn new(
        source_dataset: impl Into<String>,
        source_integrity: impl Into<String>,
        normalization_version: impl Into<String>,
        build_configuration: impl Into<String>,
    ) -> Self {
        Self {
            source_dataset: source_dataset.into(),
            source_integrity: source_integrity.into(),
            compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
            normalization_version: normalization_version.into(),
            build_configuration: build_configuration.into(),
        }
    }
    pub(super) fn from_artifact(
        source_dataset: String,
        source_integrity: String,
        compiler_version: String,
        normalization_version: String,
        build_configuration: String,
    ) -> Self {
        Self {
            source_dataset,
            source_integrity,
            compiler_version,
            normalization_version,
            build_configuration,
        }
    }
    /// Returns the declared source dataset identity.
    #[must_use]
    pub fn source_dataset(&self) -> &str {
        &self.source_dataset
    }
    /// Returns the declared source integrity value.
    #[must_use]
    pub fn source_integrity(&self) -> &str {
        &self.source_integrity
    }
    /// Returns the compiler version.
    #[must_use]
    pub fn compiler_version(&self) -> &str {
        &self.compiler_version
    }
    /// Returns the normalization-rules version.
    #[must_use]
    pub fn normalization_version(&self) -> &str {
        &self.normalization_version
    }
    /// Returns the canonical build configuration identity.
    #[must_use]
    pub fn build_configuration(&self) -> &str {
        &self.build_configuration
    }
}

impl GraphMetadata {
    /// Creates graph semantic metadata.
    #[must_use]
    pub fn new(
        routing_profile: impl Into<String>,
        jurisdiction_policy: impl Into<String>,
        build_identity: GraphBuildIdentity,
    ) -> Self {
        Self {
            routing_profile: routing_profile.into(),
            jurisdiction_policy: jurisdiction_policy.into(),
            build_identity,
            turn_restrictions_enforced: false,
        }
    }
    /// Returns the routing-profile identifier.
    #[must_use]
    pub fn routing_profile(&self) -> &str {
        &self.routing_profile
    }
    /// Returns the jurisdiction-policy identifier.
    #[must_use]
    pub fn jurisdiction_policy(&self) -> &str {
        &self.jurisdiction_policy
    }
    /// Returns declared compilation identity.
    #[must_use]
    pub const fn build_identity(&self) -> &GraphBuildIdentity {
        &self.build_identity
    }
    /// Returns whether turn restrictions are enforced.
    #[must_use]
    pub const fn turn_restrictions_enforced(&self) -> bool {
        self.turn_restrictions_enforced
    }
}

#[derive(Debug, Clone)]
struct SegmentDraft {
    from: BuilderNodeId,
    to: BuilderNodeId,
    geometry: Vec<CanonicalCoordinate>,
    forward: Option<EdgeProperties>,
    reverse: Option<EdgeProperties>,
}

/// Mutable deterministic construction boundary for a routing graph.
#[derive(Debug, Clone)]
pub struct GraphBuilder {
    snapshot_id: GraphSnapshotId,
    metadata: GraphMetadata,
    nodes: BTreeMap<BuilderNodeId, CanonicalCoordinate>,
    segments: BTreeMap<BuilderSegmentId, SegmentDraft>,
}

impl GraphBuilder {
    /// Creates an empty graph builder.
    #[must_use]
    pub fn new(snapshot_id: GraphSnapshotId, metadata: GraphMetadata) -> Self {
        Self {
            snapshot_id,
            metadata,
            nodes: BTreeMap::new(),
            segments: BTreeMap::new(),
        }
    }

    /// Adds a construction-time routing node.
    ///
    /// # Errors
    ///
    /// Returns an error when the key already exists.
    pub fn add_node(
        &mut self,
        id: BuilderNodeId,
        coordinate: CanonicalCoordinate,
    ) -> Result<(), GraphError> {
        if self.nodes.insert(id, coordinate).is_some() {
            return Err(GraphError::DuplicateBuilderNode { id });
        }
        Ok(())
    }

    /// Adds a physical segment and its permitted directional traversals.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicate keys, missing endpoints, malformed geometry,
    /// or a segment with no traversal.
    pub fn add_segment(
        &mut self,
        id: BuilderSegmentId,
        from: BuilderNodeId,
        to: BuilderNodeId,
        geometry: Vec<CanonicalCoordinate>,
        forward: Option<EdgeProperties>,
        reverse: Option<EdgeProperties>,
    ) -> Result<(), GraphError> {
        if self.segments.contains_key(&id) {
            return Err(GraphError::DuplicateBuilderSegment { id });
        }
        let from_coordinate =
            self.nodes
                .get(&from)
                .copied()
                .ok_or(GraphError::MissingBuilderNode {
                    segment_id: id,
                    node_id: from,
                })?;
        let to_coordinate = self
            .nodes
            .get(&to)
            .copied()
            .ok_or(GraphError::MissingBuilderNode {
                segment_id: id,
                node_id: to,
            })?;
        if geometry.len() < 2 {
            return Err(GraphError::GeometryTooShort { segment_id: id });
        }
        if geometry.first() != Some(&from_coordinate) || geometry.last() != Some(&to_coordinate) {
            return Err(GraphError::GeometryEndpointMismatch { segment_id: id });
        }
        if forward.is_none() && reverse.is_none() {
            return Err(GraphError::SegmentWithoutEdges { segment_id: id });
        }
        if forward
            .into_iter()
            .chain(reverse)
            .any(|properties| properties.effective_free_flow_speed().value() == 0.0)
        {
            return Err(GraphError::ZeroEffectiveSpeed { segment_id: id });
        }
        self.segments.insert(
            id,
            SegmentDraft {
                from,
                to,
                geometry,
                forward,
                reverse,
            },
        );
        Ok(())
    }

    /// Validates, deterministically orders, and freezes the graph.
    ///
    /// # Errors
    ///
    /// Returns an error when dense IDs or derived measurements cannot be represented.
    #[allow(clippy::too_many_lines)]
    pub fn finalize(self) -> Result<FrozenGraph, GraphError> {
        let mut node_ids = BTreeMap::new();
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for (index, (builder_id, coordinate)) in self.nodes.into_iter().enumerate() {
            let value = u32::try_from(index).map_err(|_| GraphError::DenseIdOverflow {
                collection: "nodes",
            })?;
            let id = NodeId::new(value);
            node_ids.insert(builder_id, id);
            nodes.push(Node::new(id, coordinate));
        }

        let mut segments = Vec::with_capacity(self.segments.len());
        let mut geometry = Vec::new();
        let mut edge_drafts = Vec::new();
        for (index, (_builder_id, draft)) in self.segments.into_iter().enumerate() {
            let segment_value = u32::try_from(index).map_err(|_| GraphError::DenseIdOverflow {
                collection: "segments",
            })?;
            let segment_id = RoadSegmentId::new(segment_value);
            let from = node_ids[&draft.from];
            let to = node_ids[&draft.to];
            let start = u32::try_from(geometry.len()).map_err(|_| GraphError::DenseIdOverflow {
                collection: "geometry points",
            })?;
            let len =
                u32::try_from(draft.geometry.len()).map_err(|_| GraphError::DenseIdOverflow {
                    collection: "geometry points",
                })?;
            let mut distance = Meters::ZERO;
            for pair in draft.geometry.windows(2) {
                distance = distance
                    .checked_add(haversine_distance(
                        pair[0].to_coordinate(),
                        pair[1].to_coordinate(),
                    ))
                    .map_err(|source| GraphError::DerivedMeasurement { source })?;
            }
            geometry.extend(draft.geometry);
            segments.push(RoadSegment::new(
                segment_id,
                from,
                to,
                GeometryRange::new(start, len),
                distance,
            ));
            if let Some(properties) = draft.forward {
                edge_drafts.push((
                    from,
                    to,
                    segment_id,
                    Orientation::Forward,
                    properties,
                    distance,
                ));
            }
            if let Some(properties) = draft.reverse {
                edge_drafts.push((
                    to,
                    from,
                    segment_id,
                    Orientation::Reverse,
                    properties,
                    distance,
                ));
            }
        }

        edge_drafts.sort_by_key(|(from, to, segment, orientation, _, _)| {
            (*from, *to, *segment, *orientation)
        });
        let mut edges = Vec::with_capacity(edge_drafts.len());
        for (index, (from, to, segment, orientation, properties, distance)) in
            edge_drafts.into_iter().enumerate()
        {
            let value = u32::try_from(index).map_err(|_| GraphError::DenseIdOverflow {
                collection: "edges",
            })?;
            let meters_per_second = properties.effective_free_flow_speed().value() / 3.6;
            let travel_time = Seconds::new(distance.value() / meters_per_second)
                .map_err(|source| GraphError::DerivedMeasurement { source })?;
            edges.push(DirectedEdge::new(
                EdgeId::new(value),
                segment,
                orientation,
                from,
                to,
                properties,
                travel_time,
            ));
        }

        let mut adjacency_offsets = vec![0_usize; nodes.len() + 1];
        for edge in &edges {
            let source =
                usize::try_from(edge.from().value()).map_err(|_| GraphError::DenseIdOverflow {
                    collection: "nodes",
                })?;
            adjacency_offsets[source + 1] += 1;
        }
        for index in 1..adjacency_offsets.len() {
            adjacency_offsets[index] += adjacency_offsets[index - 1];
        }

        Ok(FrozenGraph {
            snapshot_id: self.snapshot_id,
            metadata: self.metadata,
            nodes,
            segments,
            edges,
            adjacency_offsets,
            geometry,
        })
    }
}

/// An immutable, validated, densely indexed routing graph snapshot.
#[derive(Debug, Clone)]
pub struct FrozenGraph {
    snapshot_id: GraphSnapshotId,
    metadata: GraphMetadata,
    nodes: Vec<Node>,
    segments: Vec<RoadSegment>,
    edges: Vec<DirectedEdge>,
    adjacency_offsets: Vec<usize>,
    geometry: Vec<CanonicalCoordinate>,
}

impl FrozenGraph {
    pub(super) fn from_validated_parts(
        snapshot_id: GraphSnapshotId,
        metadata: GraphMetadata,
        nodes: Vec<Node>,
        segments: Vec<RoadSegment>,
        edges: Vec<DirectedEdge>,
        adjacency_offsets: Vec<usize>,
        geometry: Vec<CanonicalCoordinate>,
    ) -> Self {
        Self {
            snapshot_id,
            metadata,
            nodes,
            segments,
            edges,
            adjacency_offsets,
            geometry,
        }
    }

    /// Returns snapshot identity.
    #[must_use]
    pub const fn snapshot_id(&self) -> GraphSnapshotId {
        self.snapshot_id
    }
    /// Returns semantic graph metadata.
    #[must_use]
    pub const fn metadata(&self) -> &GraphMetadata {
        &self.metadata
    }
    /// Returns a node by dense identity.
    #[must_use]
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.value() as usize)
    }
    /// Returns a road segment by dense identity.
    #[must_use]
    pub fn segment(&self, id: RoadSegmentId) -> Option<&RoadSegment> {
        self.segments.get(id.value() as usize)
    }
    /// Returns a directed edge by dense identity.
    #[must_use]
    pub fn edge(&self, id: EdgeId) -> Option<&DirectedEdge> {
        self.edges.get(id.value() as usize)
    }
    /// Returns pre-canonicalized outgoing adjacency without allocation.
    ///
    /// # Errors
    ///
    /// Returns an error when the node identity is outside this snapshot.
    pub fn outgoing_edges(&self, id: NodeId) -> Result<&[DirectedEdge], GraphError> {
        let index = id.value() as usize;
        if index >= self.nodes.len() {
            return Err(GraphError::NodeNotFound { id });
        }
        Ok(&self.edges[self.adjacency_offsets[index]..self.adjacency_offsets[index + 1]])
    }
    /// Returns segment geometry in canonical orientation.
    ///
    /// # Errors
    ///
    /// Returns an error when the segment identity is outside this snapshot.
    pub fn segment_geometry(
        &self,
        id: RoadSegmentId,
    ) -> Result<&[CanonicalCoordinate], GraphError> {
        let segment = self.segment(id).ok_or(GraphError::SegmentNotFound { id })?;
        let range = segment.geometry();
        let start = range.start() as usize;
        let end = start + range.len() as usize;
        Ok(&self.geometry[start..end])
    }
    /// Returns every node in dense-ID order.
    #[must_use]
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    /// Returns every segment in dense-ID order.
    #[must_use]
    pub fn segments(&self) -> &[RoadSegment] {
        &self.segments
    }
    /// Returns every edge in dense-ID and adjacency order.
    #[must_use]
    pub fn edges(&self) -> &[DirectedEdge] {
        &self.edges
    }
    /// Returns number of nodes.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    /// Returns number of physical segments.
    #[must_use]
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
    /// Returns number of directed edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
    /// Returns number of canonical geometry points.
    #[must_use]
    pub fn geometry_point_count(&self) -> usize {
        self.geometry.len()
    }

    pub(super) fn adjacency_offsets(&self) -> &[usize] {
        &self.adjacency_offsets
    }

    pub(super) fn geometry_points(&self) -> &[CanonicalCoordinate] {
        &self.geometry
    }
}
