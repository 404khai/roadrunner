#![allow(dead_code)]

use roadrunner_core::geo::{CanonicalCoordinate, KilometersPerHour};
use roadrunner_core::graph::{
    AccessClass, BuilderNodeId, BuilderSegmentId, EdgeProperties, FrozenGraph, GraphBuildIdentity,
    GraphBuilder, GraphMetadata, GraphSnapshotId, NodeId,
};

pub fn generated_builder(node_count: u32, fan_out: u32) -> GraphBuilder {
    let mut builder = GraphBuilder::new(
        GraphSnapshotId::new(1),
        GraphMetadata::new(
            "synthetic_v1",
            "none",
            GraphBuildIdentity::new(
                "forward_banded_v1",
                "generated-seed-0",
                "synthetic-v1",
                "fan-out-3",
            ),
        ),
    );
    for id in 0..node_count {
        let coordinate = canonical(0, i32::try_from(id).unwrap_or(i32::MAX) * 10);
        assert!(
            builder
                .add_node(BuilderNodeId::new(u64::from(id)), coordinate)
                .is_ok()
        );
    }
    let speed = KilometersPerHour::new(36.0).unwrap_or(KilometersPerHour::ZERO);
    let properties = EdgeProperties::new(speed, None, AccessClass::General);
    let mut segment_id = 0_u64;
    for from in 0..node_count {
        for offset in 1..=fan_out {
            let Some(to) = from.checked_add(offset).filter(|to| *to < node_count) else {
                continue;
            };
            let geometry = vec![
                canonical(0, i32::try_from(from).unwrap_or(i32::MAX) * 10),
                canonical(0, i32::try_from(to).unwrap_or(i32::MAX) * 10),
            ];
            assert!(
                builder
                    .add_segment(
                        BuilderSegmentId::new(segment_id),
                        BuilderNodeId::new(u64::from(from)),
                        BuilderNodeId::new(u64::from(to)),
                        geometry,
                        Some(properties),
                        None
                    )
                    .is_ok()
            );
            segment_id += 1;
        }
    }
    builder
}

pub fn generated_graph(node_count: u32, fan_out: u32) -> FrozenGraph {
    match generated_builder(node_count, fan_out).finalize() {
        Ok(graph) => graph,
        Err(error) => panic!("generated graph failed: {error}"),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QueryCase {
    pub source: NodeId,
    pub destination: NodeId,
}

pub fn query_corpus(node_count: u32, seed: u64) -> Vec<QueryCase> {
    let local_start =
        u32::try_from(seed % u64::from(node_count.saturating_sub(20).max(1))).unwrap_or(0);
    vec![
        QueryCase {
            source: NodeId::new(local_start),
            destination: NodeId::new((local_start + 10).min(node_count - 1)),
        },
        QueryCase {
            source: NodeId::new(0),
            destination: NodeId::new(node_count / 2),
        },
        QueryCase {
            source: NodeId::new(0),
            destination: NodeId::new(node_count - 1),
        },
        QueryCase {
            source: NodeId::new(node_count - 1),
            destination: NodeId::new(0),
        },
        QueryCase {
            source: NodeId::new(local_start),
            destination: NodeId::new(local_start),
        },
    ]
}

pub fn canonical(latitude_e7: i32, longitude_e7: i32) -> CanonicalCoordinate {
    match CanonicalCoordinate::new(latitude_e7, longitude_e7) {
        Ok(value) => value,
        Err(error) => panic!("invalid benchmark coordinate: {error}"),
    }
}

pub fn destination(node_count: u32) -> NodeId {
    NodeId::new(node_count - 1)
}
