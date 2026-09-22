use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use roadrunner_core::graph::{
    FrozenGraph, decode_graph_artifact, encode_graph_artifact, verify_graph_deep,
};
use sha2::{Digest, Sha256};

use crate::compile::{BuildManifest, CompiledGraph};
use crate::error::{OsmError, invalid};
use crate::provenance::{
    GraphProvenance, decode_provenance_artifact, encode_provenance_artifact,
    provenance_artifact_sha256,
};

const GRAPH_FILE: &str = "graph.rr-graph";
const PROVENANCE_FILE: &str = "graph.rr-provenance";
const MANIFEST_FILE: &str = "manifest.json";
static PUBLICATION_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Fully validated graph snapshot bundle.
#[derive(Debug, Clone)]
pub struct LoadedGraphSnapshot {
    /// Trusted immutable routing graph.
    pub graph: FrozenGraph,
    /// Trusted source-to-graph correspondence.
    pub provenance: GraphProvenance,
    /// Validated build manifest.
    pub manifest: BuildManifest,
}

/// Publishes graph, provenance, and manifest as one atomic directory.
///
/// # Errors
///
/// Returns an error if validation, durable writing, or atomic publication fails.
pub fn write_snapshot_bundle_atomic(
    path: impl AsRef<Path>,
    compiled: &CompiledGraph,
) -> Result<(), OsmError> {
    let path = path.as_ref();
    if path.exists() {
        return Err(invalid(format!(
            "snapshot publication target already exists: {}",
            path.display()
        )));
    }
    let graph_bytes = encode_graph_artifact(&compiled.graph)
        .map_err(|error| invalid(format!("graph encoding failed: {error}")))?;
    let provenance_bytes = encode_provenance_artifact(&compiled.provenance, &compiled.graph)?;
    verify_graph_deep(&compiled.graph)
        .map_err(|error| invalid(format!("deep graph verification failed: {error}")))?;
    validate_manifest(
        &compiled.manifest,
        &compiled.graph,
        &compiled.provenance,
        &graph_bytes,
        &provenance_bytes,
    )?;
    let manifest_bytes = serde_json::to_vec(&compiled.manifest)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| invalid("snapshot path requires a directory name"))?;
    let sequence = PUBLICATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{sequence}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    std::fs::create_dir(&temporary)
        .map_err(|source| io_error("temporary snapshot directory creation", &temporary, source))?;
    let publication = (|| {
        write_durable(&temporary.join(GRAPH_FILE), &graph_bytes)?;
        write_durable(&temporary.join(PROVENANCE_FILE), &provenance_bytes)?;
        write_durable(&temporary.join(MANIFEST_FILE), &manifest_bytes)?;
        File::open(&temporary)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| io_error("snapshot directory synchronization", &temporary, source))?;
        std::fs::rename(&temporary, path)
            .map_err(|source| io_error("atomic snapshot publication", path, source))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| io_error("parent directory synchronization", parent, source))
    })();
    if publication.is_err() {
        let _ = std::fs::remove_dir_all(&temporary);
    }
    publication
}

/// Loads and validates one coherent graph snapshot directory.
///
/// # Errors
///
/// Returns an error for missing, corrupt, noncanonical, mismatched, or deeply
/// invalid snapshot contents.
pub fn load_snapshot_bundle(
    path: impl AsRef<Path>,
    deep: bool,
) -> Result<LoadedGraphSnapshot, OsmError> {
    let path = path.as_ref();
    let graph_bytes = read_file(&path.join(GRAPH_FILE))?;
    let provenance_bytes = read_file(&path.join(PROVENANCE_FILE))?;
    let manifest_bytes = read_file(&path.join(MANIFEST_FILE))?;
    let graph = decode_graph_artifact(&graph_bytes)
        .map_err(|error| invalid(format!("graph validation failed: {error}")))?;
    let provenance = decode_provenance_artifact(&provenance_bytes, &graph)?;
    let manifest: BuildManifest = serde_json::from_slice(&manifest_bytes)?;
    if serde_json::to_vec(&manifest)? != manifest_bytes {
        return Err(invalid("snapshot manifest is not canonical"));
    }
    validate_manifest(
        &manifest,
        &graph,
        &provenance,
        &graph_bytes,
        &provenance_bytes,
    )?;
    if deep {
        verify_graph_deep(&graph)
            .map_err(|error| invalid(format!("deep graph verification failed: {error}")))?;
    }
    Ok(LoadedGraphSnapshot {
        graph,
        provenance,
        manifest,
    })
}

fn validate_manifest(
    manifest: &BuildManifest,
    graph: &FrozenGraph,
    provenance: &GraphProvenance,
    graph_bytes: &[u8],
    provenance_bytes: &[u8],
) -> Result<(), OsmError> {
    let graph_hash = format!("{:x}", Sha256::digest(graph_bytes));
    let provenance_hash = provenance_artifact_sha256(provenance_bytes);
    let compact_id = compact_snapshot_id(&manifest.graph_snapshot_digest)?;
    if manifest.graph_artifact_sha256 != graph_hash
        || manifest.provenance_artifact_sha256 != provenance_hash
        || manifest.graph_snapshot_id != graph.snapshot_id().value()
        || manifest.graph_snapshot_id != compact_id
        || manifest.graph_snapshot_digest != graph.metadata().snapshot_digest()
        || manifest.graph_snapshot_digest != provenance.graph_snapshot_digest
        || manifest.normalized_dataset_sha256 != provenance.normalized_dataset_sha256
        || manifest.graph_node_count != graph.node_count()
        || manifest.graph_segment_count != graph.segment_count()
        || manifest.graph_edge_count != graph.edge_count()
        || manifest.routing_profile != graph.metadata().routing_profile()
        || manifest.jurisdiction_policy != graph.metadata().jurisdiction_policy()
        || manifest.compiler_version != graph.metadata().build_identity().compiler_version()
        || manifest.compiler_semantic_version != "osm_graph_compiler_v3"
        || manifest.normalization_version
            != graph.metadata().build_identity().normalization_version()
        || manifest.build_configuration != graph.metadata().build_identity().build_configuration()
        || manifest.source_id != graph.metadata().build_identity().source_dataset()
        || manifest.normalized_dataset_sha256
            != graph.metadata().build_identity().source_integrity()
        || manifest.turn_restrictions_enforced != graph.metadata().turn_restrictions_enforced()
    {
        return Err(invalid(
            "snapshot manifest does not agree with graph and provenance",
        ));
    }
    Ok(())
}

fn compact_snapshot_id(digest: &str) -> Result<u64, OsmError> {
    if digest.len() != 64 {
        return Err(invalid("snapshot digest is not SHA-256"));
    }
    u64::from_str_radix(&digest[..16], 16)
        .map_err(|_| invalid("snapshot digest is not canonical lowercase hexadecimal"))
}

fn read_file(path: &Path) -> Result<Vec<u8>, OsmError> {
    std::fs::read(path).map_err(|source| io_error("snapshot read", path, source))
}

fn write_durable(path: &Path, bytes: &[u8]) -> Result<(), OsmError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|source| io_error("snapshot file creation", path, source))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|source| io_error("snapshot file write", path, source))
}

fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> OsmError {
    OsmError::Io {
        operation,
        path: PathBuf::from(path),
        source,
    }
}
