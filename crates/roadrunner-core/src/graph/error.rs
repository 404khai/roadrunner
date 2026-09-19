use thiserror::Error;

use crate::geo::UnitError;

use super::{BuilderNodeId, BuilderSegmentId, EdgeId, NodeId, RoadSegmentId};

/// Errors produced while building or querying a frozen graph.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum GraphError {
    /// A construction node key is duplicated.
    #[error("builder node {id} already exists")]
    DuplicateBuilderNode {
        /// Duplicated construction key.
        id: BuilderNodeId,
    },
    /// A construction segment key is duplicated.
    #[error("builder segment {id} already exists")]
    DuplicateBuilderSegment {
        /// Duplicated construction key.
        id: BuilderSegmentId,
    },
    /// A segment references an absent construction node.
    #[error("segment {segment_id} references missing builder node {node_id}")]
    MissingBuilderNode {
        /// Segment containing the invalid reference.
        segment_id: BuilderSegmentId,
        /// Missing construction node.
        node_id: BuilderNodeId,
    },
    /// A segment has fewer than two geometry points.
    #[error("segment {segment_id} geometry requires at least two points")]
    GeometryTooShort {
        /// Segment with insufficient geometry.
        segment_id: BuilderSegmentId,
    },
    /// Segment geometry does not begin and end at its routing nodes.
    #[error("segment {segment_id} geometry endpoints do not match its nodes")]
    GeometryEndpointMismatch {
        /// Segment with mismatched endpoints.
        segment_id: BuilderSegmentId,
    },
    /// No directed traversal was supplied for a segment.
    #[error("segment {segment_id} has no directed traversal")]
    SegmentWithoutEdges {
        /// Segment with no traversal.
        segment_id: BuilderSegmentId,
    },
    /// Effective traversal speed must be positive.
    #[error("segment {segment_id} has a zero effective speed")]
    ZeroEffectiveSpeed {
        /// Segment containing the invalid speed.
        segment_id: BuilderSegmentId,
    },
    /// A graph collection cannot be represented by dense identifiers.
    #[error("{collection} contains too many elements for dense identifiers")]
    DenseIdOverflow {
        /// Collection that exceeded its dense ID width.
        collection: &'static str,
    },
    /// A derived distance or duration is invalid.
    #[error("derived measurement is invalid: {source}")]
    DerivedMeasurement {
        /// Invalid derived unit value.
        #[source]
        source: UnitError,
    },
    /// A requested node is absent.
    #[error("node {id} does not exist")]
    NodeNotFound {
        /// Missing node identity.
        id: NodeId,
    },
    /// A requested edge is absent.
    #[error("edge {id} does not exist")]
    EdgeNotFound {
        /// Missing edge identity.
        id: EdgeId,
    },
    /// A requested segment is absent.
    #[error("road segment {id} does not exist")]
    SegmentNotFound {
        /// Missing segment identity.
        id: RoadSegmentId,
    },
}
