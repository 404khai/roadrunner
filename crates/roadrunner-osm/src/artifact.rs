use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::NORMALIZATION_VERSION;
use crate::error::{OsmError, invalid};
use crate::model::NormalizedOsmDataset;

const MAGIC: &str = "ROADRUNNER_NORMALIZED_OSM";
const SCHEMA_VERSION: u32 = 1;
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Serialize, Deserialize)]
struct Envelope {
    magic: String,
    schema_version: u32,
    payload_sha256: String,
    payload_json: String,
}

/// A validated normalized dataset plus the integrity of its exact payload.
#[derive(Debug, Clone)]
pub struct DecodedDataset {
    /// Validated source dataset.
    pub(crate) dataset: NormalizedOsmDataset,
    /// SHA-256 of the canonical payload bytes.
    pub(crate) payload_sha256: String,
}

impl DecodedDataset {
    /// Returns the validated normalized dataset.
    #[must_use]
    pub const fn dataset(&self) -> &NormalizedOsmDataset {
        &self.dataset
    }

    /// Returns the integrity hash of the canonical payload.
    #[must_use]
    pub fn payload_sha256(&self) -> &str {
        &self.payload_sha256
    }
}

/// Encodes a normalized dataset in its independently framed canonical artifact.
///
/// # Errors
///
/// Returns an error when invariants fail or deterministic JSON encoding fails.
pub fn encode_dataset_artifact(dataset: &NormalizedOsmDataset) -> Result<Vec<u8>, OsmError> {
    validate_dataset(dataset)?;
    let payload = serde_json::to_vec(dataset)?;
    let payload_sha256 = format!("{:x}", Sha256::digest(&payload));
    let payload_json = String::from_utf8(payload)
        .map_err(|error| invalid(format!("canonical payload is not UTF-8: {error}")))?;
    Ok(serde_json::to_vec(&Envelope {
        magic: MAGIC.to_owned(),
        schema_version: SCHEMA_VERSION,
        payload_sha256,
        payload_json,
    })?)
}

/// Decodes and fully validates a normalized dataset artifact.
///
/// # Errors
///
/// Returns an error for malformed framing, unsupported schema, integrity
/// mismatch, non-canonical encoding, or invalid normalized topology.
pub fn decode_dataset_artifact(bytes: &[u8]) -> Result<DecodedDataset, OsmError> {
    let envelope: Envelope = serde_json::from_slice(bytes)?;
    if envelope.magic != MAGIC {
        return Err(OsmError::InvalidMagic);
    }
    if envelope.schema_version != SCHEMA_VERSION {
        return Err(OsmError::UnsupportedSchema {
            version: envelope.schema_version,
        });
    }
    let actual = format!("{:x}", Sha256::digest(envelope.payload_json.as_bytes()));
    if actual != envelope.payload_sha256 {
        return Err(OsmError::IntegrityMismatch);
    }
    let dataset: NormalizedOsmDataset = serde_json::from_str(&envelope.payload_json)?;
    validate_dataset(&dataset)?;
    let canonical = serde_json::to_string(&dataset)?;
    if canonical != envelope.payload_json {
        return Err(invalid("normalized OSM payload is not canonical"));
    }
    let canonical_artifact = encode_dataset_artifact(&dataset)?;
    if canonical_artifact != bytes {
        return Err(invalid("normalized OSM artifact framing is not canonical"));
    }
    Ok(DecodedDataset {
        dataset,
        payload_sha256: actual,
    })
}

/// Atomically publishes a normalized dataset artifact.
///
/// # Errors
///
/// Returns an error when encoding, durable writing, or publication fails.
pub fn write_dataset_artifact_atomic(
    path: impl AsRef<Path>,
    dataset: &NormalizedOsmDataset,
) -> Result<(), OsmError> {
    let path = path.as_ref();
    let bytes = encode_dataset_artifact(dataset)?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .ok_or_else(|| invalid("artifact path requires a file name"))?;
    let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{}.tmp-{}-{sequence}",
        file_name.to_string_lossy(),
        std::process::id()
    ));
    let publication = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|source| io_error("temporary-file creation", &temporary, source))?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|source| io_error("temporary-file write", &temporary, source))?;
        std::fs::rename(&temporary, path)
            .map_err(|source| io_error("atomic rename", path, source))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|source| io_error("directory synchronization", parent, source))
    })();
    if publication.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    publication
}

#[allow(clippy::too_many_lines)]
fn validate_dataset(dataset: &NormalizedOsmDataset) -> Result<(), OsmError> {
    if dataset.normalization_version != NORMALIZATION_VERSION {
        return Err(invalid(format!(
            "unsupported normalization version {}",
            dataset.normalization_version
        )));
    }
    if dataset.provenance.source_id.trim().is_empty()
        || !is_sha256(&dataset.provenance.source_sha256)
    {
        return Err(invalid("invalid dataset provenance"));
    }
    strictly_sorted_unique(&dataset.nodes, |node| node.osm_id, "nodes")?;
    strictly_sorted_unique(&dataset.ways, |way| way.osm_id, "ways")?;
    strictly_sorted_unique(&dataset.relations, |relation| relation.osm_id, "relations")?;
    strictly_sorted_unique(
        &dataset.split_points,
        |point| point.osm_node_id,
        "split points",
    )?;
    let node_ids: std::collections::BTreeSet<_> =
        dataset.nodes.iter().map(|node| node.osm_id).collect();
    let mut required_node_ids = std::collections::BTreeSet::new();
    let mut reference_counts = std::collections::BTreeMap::<i64, u32>::new();
    let mut expected_split_reasons = std::collections::BTreeMap::<
        i64,
        std::collections::BTreeSet<crate::model::SplitPoint>,
    >::new();
    for node in &dataset.nodes {
        if node.osm_id <= 0
            || roadrunner_core::geo::CanonicalCoordinate::new(
                node.coordinate.latitude_e7(),
                node.coordinate.longitude_e7(),
            )
            .is_err()
        {
            return Err(invalid(format!("invalid normalized node {}", node.osm_id)));
        }
    }
    for way in &dataset.ways {
        if way.osm_id <= 0 || way.node_refs.len() < 2 {
            return Err(invalid(format!(
                "way {} has fewer than two nodes",
                way.osm_id
            )));
        }
        if let Some(missing) = way.node_refs.iter().find(|id| !node_ids.contains(id)) {
            return Err(invalid(format!(
                "way {} references missing node {missing}",
                way.osm_id
            )));
        }
        for node_id in &way.node_refs {
            required_node_ids.insert(*node_id);
            let count = reference_counts.entry(*node_id).or_default();
            *count = count.saturating_add(1);
        }
        if let Some(first) = way.node_refs.first() {
            expected_split_reasons
                .entry(*first)
                .or_default()
                .insert(crate::model::SplitPoint::WayEndpoint);
        }
        if let Some(last) = way.node_refs.last() {
            expected_split_reasons
                .entry(*last)
                .or_default()
                .insert(crate::model::SplitPoint::WayEndpoint);
        }
        if !way.tags.contains_key("highway") {
            return Err(invalid(format!("way {} has no highway tag", way.osm_id)));
        }
    }
    for relation in &dataset.relations {
        if relation.osm_id <= 0 || relation.restriction.is_empty() {
            return Err(invalid(format!(
                "invalid restriction relation {}",
                relation.osm_id
            )));
        }
        for member in &relation.members {
            if member.osm_id <= 0 {
                return Err(invalid(format!(
                    "relation {} has invalid member identity {}",
                    relation.osm_id, member.osm_id
                )));
            }
            if member.kind == crate::model::OsmElementKind::Node && member.role == "via" {
                required_node_ids.insert(member.osm_id);
                expected_split_reasons
                    .entry(member.osm_id)
                    .or_default()
                    .insert(crate::model::SplitPoint::RestrictionVia);
                if !node_ids.contains(&member.osm_id) {
                    return Err(invalid(format!(
                        "relation {} references missing via node {}",
                        relation.osm_id, member.osm_id
                    )));
                }
            }
        }
    }
    for (node_id, count) in reference_counts {
        if count > 1 {
            expected_split_reasons
                .entry(node_id)
                .or_default()
                .insert(crate::model::SplitPoint::Junction);
        }
    }
    if node_ids != required_node_ids {
        return Err(invalid(
            "normalized node set is not exactly the set required by retained topology",
        ));
    }
    for split in &dataset.split_points {
        if !node_ids.contains(&split.osm_node_id)
            || split.reasons.is_empty()
            || split.reasons.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(invalid(format!(
                "invalid split point {}",
                split.osm_node_id
            )));
        }
    }
    let expected_split_points: Vec<_> = expected_split_reasons
        .into_iter()
        .map(
            |(osm_node_id, reasons)| crate::model::NormalizedSplitPoint {
                osm_node_id,
                reasons: reasons.into_iter().collect(),
            },
        )
        .collect();
    if dataset.split_points != expected_split_points {
        return Err(invalid(
            "split points do not match retained way and restriction topology",
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn strictly_sorted_unique<T, K: Ord + Copy>(
    values: &[T],
    key: impl Fn(&T) -> K,
    collection: &str,
) -> Result<(), OsmError> {
    if values.windows(2).any(|pair| key(&pair[0]) >= key(&pair[1])) {
        return Err(invalid(format!(
            "{collection} are not strictly ordered by source identity"
        )));
    }
    Ok(())
}

fn io_error(operation: &'static str, path: &Path, source: std::io::Error) -> OsmError {
    OsmError::Io {
        operation,
        path: path.to_path_buf(),
        source,
    }
}
