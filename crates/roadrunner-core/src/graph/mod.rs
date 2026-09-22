//! Directed road-network graph types and adjacency-list storage.

mod artifact;
mod edge;
mod error;
mod ids;
mod network;
mod node;
mod segment;

pub use artifact::{
    GraphArtifactError, decode_graph_artifact, encode_graph_artifact, verify_graph_deep,
    write_graph_artifact_atomic,
};
pub use edge::{AccessClass, DirectedEdge, EdgeProperties, Orientation};
pub use error::GraphError;
pub use ids::{BuilderNodeId, BuilderSegmentId, EdgeId, GraphSnapshotId, NodeId, RoadSegmentId};
pub use network::{
    DISTANCE_INVARIANT_TOLERANCE_METERS, FrozenGraph, GraphBuildIdentity, GraphBuilder,
    GraphMetadata,
};
pub use node::Node;
pub use segment::{GeometryRange, RoadSegment};
