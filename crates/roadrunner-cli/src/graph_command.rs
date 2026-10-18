//! Verified graph loading and atomic import, shared by Phase 36 adapters.
use roadrunner_core::graph::{FrozenGraph, decode_graph_artifact, verify_graph_deep};
use roadrunner_osm::{GraphProvenance, load_snapshot_bundle};
use serde_json::json;
use std::path::Path;

pub(super) struct Input {
    pub graph: FrozenGraph,
    pub provenance: Option<GraphProvenance>,
}

pub(super) fn load(path: &str) -> Result<Input, String> {
    if Path::new(path).is_dir() {
        let loaded = load_snapshot_bundle(path, true).map_err(|e| e.to_string())?;
        Ok(Input {
            graph: loaded.graph,
            provenance: Some(loaded.provenance),
        })
    } else {
        let bytes = std::fs::read(path).map_err(|e| format!("could not read graph {path}: {e}"))?;
        let graph = decode_graph_artifact(&bytes).map_err(|e| e.to_string())?;
        verify_graph_deep(&graph).map_err(|e| e.to_string())?;
        Ok(Input {
            graph,
            provenance: None,
        })
    }
}

pub(super) fn stats(path: &str, args: &[String]) -> Result<(), String> {
    let (_, json) = crate::command_options::flags(args, &[])?;
    let input = load(path)?;
    let g = input.graph;
    let value = json!({"schema_version":1,"graph_snapshot_digest":g.metadata().snapshot_digest(),"graph_snapshot_id":g.snapshot_id(),"nodes":g.node_count(),"segments":g.segment_count(),"directed_edges":g.edge_count(),"metadata":g.metadata(),"deep_verified":true,"provenance_available":input.provenance.is_some()});
    crate::terminal::output(
        &value,
        &format!(
            "Graph {}\n  Nodes          {}\n  Segments       {}\n  Directed edges {}\n  Deep verified  yes\n  OSM provenance {}",
            g.metadata().snapshot_digest(),
            g.node_count(),
            g.segment_count(),
            g.edge_count(),
            if input.provenance.is_some() {
                "available"
            } else {
                "not in standalone artifact"
            }
        ),
        json,
    )
}

pub(super) fn import(input: &str, destination: &str, args: &[String]) -> Result<(), String> {
    let (options, json) = crate::command_options::flags(args, &["--source-id"])?;
    let identity = crate::command_options::required(&options, "--source-id")?;
    if Path::new(destination).exists() {
        return Err(format!(
            "snapshot publication target already exists: {destination}"
        ));
    }
    let dataset = roadrunner_osm::extract_pbf(input, identity).map_err(|e| e.to_string())?;
    let encoded = roadrunner_osm::encode_dataset_artifact(&dataset).map_err(|e| e.to_string())?;
    let decoded = roadrunner_osm::decode_dataset_artifact(&encoded).map_err(|e| e.to_string())?;
    let compiled = roadrunner_osm::compile_motorcycle_graph(&decoded).map_err(|e| e.to_string())?;
    roadrunner_osm::write_snapshot_bundle_atomic(destination, &compiled)
        .map_err(|e| e.to_string())?;
    let value =
        json!({"schema_version":1,"snapshot_directory":destination,"manifest":compiled.manifest});
    crate::terminal::output(
        &value,
        &format!(
            "Imported {}\n  Snapshot {}\n  Nodes {} · segments {} · directed edges {}\n  Graph {}",
            input,
            destination,
            compiled.graph.node_count(),
            compiled.graph.segment_count(),
            compiled.graph.edge_count(),
            compiled.manifest.graph_snapshot_digest
        ),
        json,
    )
}
