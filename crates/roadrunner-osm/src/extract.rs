use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use osmpbf::{Element, ElementReader, RelMemberType};
use roadrunner_core::geo::CanonicalCoordinate;
use sha2::{Digest, Sha256};
use tracing::info;

use crate::NORMALIZATION_VERSION;
use crate::error::{OsmError, invalid};
use crate::model::{
    DatasetProvenance, NormalizedNode, NormalizedOsmDataset, NormalizedRelation,
    NormalizedRelationMember, NormalizedRestriction, NormalizedSplitPoint, NormalizedWay,
    OsmElementKind, RestrictionKind, SourceStatistics, SplitPoint,
};

/// Extracts a deterministic, profile-independent routing dataset from a PBF.
///
/// The first pass selects candidate ways and restriction relations. The second
/// pass retains only coordinates referenced by those objects, avoiding a full
/// in-memory copy of the source node table.
///
/// # Errors
///
/// Returns an error for unreadable or malformed PBF input, duplicate source
/// identities, unsupported coordinate precision, or missing referenced nodes.
#[allow(clippy::too_many_lines)]
pub fn extract_pbf(
    path: impl AsRef<Path>,
    source_id: impl Into<String>,
) -> Result<NormalizedOsmDataset, OsmError> {
    let path = path.as_ref();
    let source_id = source_id.into();
    if source_id.trim().is_empty() {
        return Err(invalid("source_id must not be empty"));
    }
    let (source_sha256, source_size_bytes) = hash_file(path)?;

    let mut ways = BTreeMap::new();
    let mut relations = BTreeMap::new();
    let mut required_nodes = BTreeSet::new();
    let mut reference_counts = BTreeMap::<i64, u32>::new();
    let mut extraction_error = None;
    let mut nodes_seen = 0_u64;
    let mut ways_seen = 0_u64;
    let mut relations_seen = 0_u64;
    let mut restriction_relations_seen = 0_u64;

    ElementReader::from_path(path)?.for_each(|element| {
        if extraction_error.is_some() {
            return;
        }
        let result = match element {
            Element::Node(_) | Element::DenseNode(_) => {
                nodes_seen = nodes_seen.saturating_add(1);
                Ok(Ok(()))
            }
            Element::Way(way) => extract_way(&way).map(|candidate| {
                ways_seen = ways_seen.saturating_add(1);
                if let Some(normalized) = candidate {
                    for node_id in &normalized.node_refs {
                        required_nodes.insert(*node_id);
                        let count = reference_counts.entry(*node_id).or_default();
                        *count = count.saturating_add(1);
                    }
                    if ways.insert(normalized.osm_id, normalized).is_some() {
                        return Err(invalid(format!("duplicate OSM way {}", way.id())));
                    }
                }
                Ok(())
            }),
            Element::Relation(relation) => extract_relation(&relation).map(|candidate| {
                relations_seen = relations_seen.saturating_add(1);
                if let Some(normalized) = candidate {
                    restriction_relations_seen = restriction_relations_seen.saturating_add(1);
                    for member in &normalized.members {
                        if member.kind == OsmElementKind::Node && member.role == "via" {
                            required_nodes.insert(member.osm_id);
                        }
                    }
                    if relations.insert(normalized.osm_id, normalized).is_some() {
                        return Err(invalid(format!("duplicate OSM relation {}", relation.id())));
                    }
                }
                Ok(())
            }),
        };
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) | Err(error) => extraction_error = Some(error),
        }
    })?;
    if let Some(error) = extraction_error {
        return Err(error);
    }

    let mut nodes = BTreeMap::new();
    let mut coordinate_error = None;
    ElementReader::from_path(path)?.for_each(|element| {
        if coordinate_error.is_some() {
            return;
        }
        let candidate = match element {
            Element::Node(node) if required_nodes.contains(&node.id()) => {
                let tags = decode_tags(
                    node.raw_tags(),
                    node.raw_tags().len(),
                    node.raw_stringtable(),
                    "node",
                    node.id(),
                );
                Some((
                    node.id(),
                    exact_e7(node.id(), node.nano_lat(), node.nano_lon()),
                    tags,
                ))
            }
            Element::DenseNode(node) if required_nodes.contains(&node.id()) => Some((
                node.id(),
                exact_e7(node.id(), node.nano_lat(), node.nano_lon()),
                Ok(node
                    .tags()
                    .map(|(key, value)| (key.to_owned(), value.to_owned()))
                    .collect()),
            )),
            _ => None,
        };
        let Some((osm_id, coordinate, tags)) = candidate else {
            return;
        };
        match coordinate.and_then(|coordinate| tags.map(|tags| (coordinate, tags))) {
            Ok((coordinate, tags)) => {
                let (tags, unsupported_tags) = partition_node_tags(tags);
                if nodes
                    .insert(
                        osm_id,
                        NormalizedNode {
                            osm_id,
                            coordinate,
                            tags,
                            unsupported_tags,
                        },
                    )
                    .is_some()
                {
                    coordinate_error = Some(invalid(format!("duplicate OSM node {osm_id}")));
                }
            }
            Err(error) => coordinate_error = Some(error),
        }
    })?;
    if let Some(error) = coordinate_error {
        return Err(error);
    }

    let missing: Vec<_> = required_nodes
        .iter()
        .filter(|node_id| !nodes.contains_key(node_id))
        .copied()
        .take(10)
        .collect();
    if !missing.is_empty() {
        return Err(invalid(format!(
            "PBF is incomplete; missing required node coordinates (first IDs: {missing:?})"
        )));
    }

    let mut split_reasons = BTreeMap::<i64, BTreeSet<SplitPoint>>::new();
    for way in ways.values() {
        if let Some(first) = way.node_refs.first() {
            split_reasons
                .entry(*first)
                .or_default()
                .insert(SplitPoint::WayEndpoint);
        }
        if let Some(last) = way.node_refs.last() {
            split_reasons
                .entry(*last)
                .or_default()
                .insert(SplitPoint::WayEndpoint);
        }
    }
    for (node_id, count) in reference_counts {
        if count > 1 {
            split_reasons
                .entry(node_id)
                .or_default()
                .insert(SplitPoint::Junction);
        }
    }
    for relation in relations.values() {
        for member in &relation.members {
            if member.kind == OsmElementKind::Node && member.role == "via" {
                split_reasons
                    .entry(member.osm_id)
                    .or_default()
                    .insert(SplitPoint::RestrictionVia);
            }
        }
    }
    for node in nodes.values() {
        for (key, reason) in [
            ("barrier", SplitPoint::Barrier),
            ("ford", SplitPoint::Ford),
            ("highway", SplitPoint::HighwayNode),
        ] {
            if node.tags.contains_key(key) || node.unsupported_tags.contains_key(key) {
                split_reasons.entry(node.osm_id).or_default().insert(reason);
            }
        }
        if ["access", "vehicle", "motor_vehicle", "motorcycle"]
            .into_iter()
            .any(|key| node.tags.contains_key(key))
        {
            split_reasons
                .entry(node.osm_id)
                .or_default()
                .insert(SplitPoint::AccessBoundary);
        }
    }
    let split_points = split_reasons
        .into_iter()
        .map(|(osm_node_id, reasons)| NormalizedSplitPoint {
            osm_node_id,
            reasons: reasons.into_iter().collect(),
        })
        .collect();

    let dataset = NormalizedOsmDataset {
        normalization_version: NORMALIZATION_VERSION.to_owned(),
        provenance: DatasetProvenance {
            source_id,
            source_sha256,
            source_size_bytes,
        },
        source_statistics: SourceStatistics {
            nodes_seen,
            ways_seen,
            candidate_ways: u64::try_from(ways.len()).unwrap_or(u64::MAX),
            relations_seen,
            restriction_relations_seen,
            referenced_nodes_requested: u64::try_from(required_nodes.len()).unwrap_or(u64::MAX),
            referenced_nodes_resolved: u64::try_from(nodes.len()).unwrap_or(u64::MAX),
            referenced_nodes_missing: 0,
        },
        nodes: nodes.into_values().collect(),
        ways: ways.into_values().collect(),
        relations: relations.into_values().collect(),
        split_points,
    };
    info!(
        nodes = dataset.nodes.len(),
        ways = dataset.ways.len(),
        restrictions = dataset.relations.len(),
        "normalized OSM PBF"
    );
    Ok(dataset)
}

fn extract_way(way: &osmpbf::Way<'_>) -> Result<Option<NormalizedWay>, OsmError> {
    let all_tags = decode_tags(
        way.raw_tags(),
        way.raw_tags().len(),
        way.raw_stringtable(),
        "way",
        way.id(),
    )?;
    let is_road = all_tags.contains_key("highway");
    let is_ferry = all_tags
        .get("route")
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("ferry"));
    if !is_road && !is_ferry {
        return Ok(None);
    }
    let node_refs: Vec<_> = way.refs().collect();
    if node_refs.len() < 2 {
        return Ok(None);
    }
    validate_positive_id("way", way.id())?;
    for node_id in &node_refs {
        validate_positive_id("node reference", *node_id)?;
    }
    let (tags, unsupported_tags) = partition_way_tags(all_tags);
    Ok(Some(NormalizedWay {
        osm_id: way.id(),
        node_refs,
        tags,
        unsupported_tags,
    }))
}

fn extract_relation(
    relation: &osmpbf::Relation<'_>,
) -> Result<Option<NormalizedRelation>, OsmError> {
    let all_tags = decode_tags(
        relation.raw_tags(),
        relation.raw_tags().len(),
        relation.raw_stringtable(),
        "relation",
        relation.id(),
    )?;
    if all_tags.get("type").map(String::as_str) != Some("restriction") {
        return Ok(None);
    }
    validate_positive_id("relation", relation.id())?;
    let restrictions = all_tags
        .iter()
        .filter(|(key, _)| key.as_str() == "restriction" || key.starts_with("restriction:"))
        .map(|(tag, value)| NormalizedRestriction {
            tag: tag.clone(),
            value: value.clone(),
            kind: if value.starts_with("no_") {
                RestrictionKind::No
            } else if value.starts_with("only_") {
                RestrictionKind::Only
            } else {
                RestrictionKind::Unsupported
            },
            conditional: tag.ends_with(":conditional"),
        })
        .collect();
    let mut members = Vec::new();
    for member in relation.members() {
        validate_positive_id("relation member", member.member_id)?;
        let role = member.role().map_err(OsmError::from)?.to_owned();
        let kind = match member.member_type {
            RelMemberType::Node => OsmElementKind::Node,
            RelMemberType::Way => OsmElementKind::Way,
            RelMemberType::Relation => OsmElementKind::Relation,
        };
        members.push(NormalizedRelationMember {
            kind,
            osm_id: member.member_id,
            role,
        });
    }
    let except = all_tags.get("except").cloned();
    let unsupported_tags = all_tags
        .into_iter()
        .filter(|(key, _)| {
            key != "type"
                && key != "except"
                && key != "restriction"
                && !key.starts_with("restriction:")
        })
        .collect();
    Ok(Some(NormalizedRelation {
        osm_id: relation.id(),
        restrictions,
        except,
        members,
        unsupported_tags,
    }))
}

fn partition_way_tags(
    tags: BTreeMap<String, String>,
) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    const SUPPORTED: &[&str] = &[
        "access",
        "ferry",
        "highway",
        "junction",
        "maxspeed",
        "maxspeed:backward",
        "maxspeed:forward",
        "maxspeed:motorcycle",
        "maxspeed:motorcycle:backward",
        "maxspeed:motorcycle:forward",
        "motor_vehicle",
        "motorcycle",
        "motorcycle:backward",
        "motorcycle:forward",
        "oneway",
        "oneway:motorcycle",
        "route",
        "service",
        "vehicle",
    ];
    let mut structured = BTreeMap::new();
    let mut unsupported = BTreeMap::new();
    for (key, value) in tags {
        if SUPPORTED.binary_search(&key.as_str()).is_ok() {
            structured.insert(key, value);
        } else if is_relevant_unsupported_key(&key) {
            unsupported.insert(key, value);
        }
    }
    (structured, unsupported)
}

fn is_relevant_unsupported_key(key: &str) -> bool {
    matches!(
        key,
        "bridge"
            | "construction"
            | "ford"
            | "lanes"
            | "layer"
            | "smoothness"
            | "surface"
            | "toll"
            | "tracktype"
            | "tunnel"
            | "width"
    ) || key.ends_with(":conditional")
        || key.starts_with("access:")
        || key.starts_with("maxspeed:")
        || key.starts_with("motor_vehicle:")
        || key.starts_with("motorcycle:")
        || key.starts_with("oneway:")
        || key.starts_with("vehicle:")
        || key.starts_with("motorcar:")
        || key.starts_with("bicycle:")
        || key.starts_with("hgv:")
}

fn partition_node_tags(
    tags: BTreeMap<String, String>,
) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    const SUPPORTED: &[&str] = &[
        "access",
        "barrier",
        "ford",
        "highway",
        "motor_vehicle",
        "motorcycle",
        "vehicle",
    ];
    let mut structured = BTreeMap::new();
    let mut unsupported = BTreeMap::new();
    for (key, value) in tags {
        if SUPPORTED.binary_search(&key.as_str()).is_ok() {
            structured.insert(key, value);
        } else if is_relevant_unsupported_key(&key) {
            unsupported.insert(key, value);
        }
    }
    (structured, unsupported)
}

fn decode_tags(
    raw_tags: impl Iterator<Item = (u32, u32)>,
    expected_count: usize,
    string_table: &[Vec<u8>],
    kind: &str,
    osm_id: i64,
) -> Result<BTreeMap<String, String>, OsmError> {
    let mut tags = BTreeMap::new();
    let mut actual_count = 0_usize;
    for (key_index, value_index) in raw_tags {
        actual_count += 1;
        let key_bytes = string_table.get(key_index as usize).ok_or_else(|| {
            invalid(format!(
                "OSM {kind} {osm_id} tag key index {key_index} is out of range"
            ))
        })?;
        let value_bytes = string_table.get(value_index as usize).ok_or_else(|| {
            invalid(format!(
                "OSM {kind} {osm_id} tag value index {value_index} is out of range"
            ))
        })?;
        let key = std::str::from_utf8(key_bytes).map_err(|error| {
            invalid(format!(
                "OSM {kind} {osm_id} has a non-UTF-8 tag key: {error}"
            ))
        })?;
        let value = std::str::from_utf8(value_bytes).map_err(|error| {
            invalid(format!(
                "OSM {kind} {osm_id} has a non-UTF-8 tag value: {error}"
            ))
        })?;
        if tags.insert(key.to_owned(), value.to_owned()).is_some() {
            return Err(invalid(format!(
                "OSM {kind} {osm_id} contains duplicate tag key {key}"
            )));
        }
    }
    if actual_count != expected_count {
        return Err(invalid(format!(
            "OSM {kind} {osm_id} has mismatched tag key/value counts"
        )));
    }
    Ok(tags)
}

fn exact_e7(
    node_id: i64,
    latitude_nano: i64,
    longitude_nano: i64,
) -> Result<CanonicalCoordinate, OsmError> {
    if latitude_nano % 100 != 0 || longitude_nano % 100 != 0 {
        return Err(invalid(format!(
            "OSM node {node_id} uses precision finer than canonical E7"
        )));
    }
    let latitude_e7 = i32::try_from(latitude_nano / 100)
        .map_err(|_| invalid(format!("OSM node {node_id} latitude does not fit E7")))?;
    let longitude_e7 = i32::try_from(longitude_nano / 100)
        .map_err(|_| invalid(format!("OSM node {node_id} longitude does not fit E7")))?;
    CanonicalCoordinate::new(latitude_e7, longitude_e7).map_err(|error| {
        invalid(format!(
            "OSM node {node_id} has invalid coordinate: {error}"
        ))
    })
}

fn validate_positive_id(kind: &str, id: i64) -> Result<(), OsmError> {
    if id <= 0 {
        return Err(invalid(format!("{kind} ID {id} is not a published OSM ID")));
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<(String, u64), OsmError> {
    let file = File::open(path).map_err(|source| OsmError::Io {
        operation: "source open",
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut size = 0_u64;
    let mut buffer = vec![0_u8; 64 * 1024].into_boxed_slice();
    loop {
        let read = reader.read(&mut buffer).map_err(|source| OsmError::Io {
            operation: "source read",
            path: path.to_path_buf(),
            source,
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        size = size
            .checked_add(read as u64)
            .ok_or_else(|| invalid("source size overflow"))?;
    }
    Ok((format!("{:x}", hasher.finalize()), size))
}
