#[cfg(test)]
use std::collections::BTreeMap;

use roadrunner_core::geo::KilometersPerHour;
use roadrunner_core::graph::{AccessClass, EdgeProperties};
use serde::{Deserialize, Serialize};

use crate::error::{OsmError, invalid};
use crate::model::{NormalizedNode, NormalizedWay};

/// Delivery-motorcycle routing profile with remediated semantics.
pub const DELIVERY_MOTORCYCLE_PROFILE: &str = "delivery_motorcycle_v2";
/// Nigeria policy with explicit access, surface, and speed semantics.
pub const NG_JURISDICTION_POLICY: &str = "ng_v2";
/// Highest speed this profile can compile.
pub const PROFILE_MAX_SPEED_KPH: f64 = 80.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Direction {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DirectionalityDecision {
    Bidirectional,
    ForwardOnly,
    ReverseOnly,
    UnsupportedDynamic,
    UnknownExplicit,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct SpeedDecision {
    pub(crate) class_default_kph: f64,
    pub(crate) legal_limit_kph: Option<f64>,
    pub(crate) profile_maximum_kph: f64,
    pub(crate) physical_cap_kph: Option<f64>,
    pub(crate) effective_kph: f64,
    pub(crate) source_status: String,
    pub(crate) physical_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TraversalPolicyDecision {
    pub(crate) direction: Direction,
    pub(crate) access: Option<AccessClass>,
    pub(crate) speed: Option<SpeedDecision>,
    #[serde(skip)]
    pub(crate) properties: Option<EdgeProperties>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WayPolicyDecision {
    pub(crate) directionality: DirectionalityDecision,
    pub(crate) directionality_reason: String,
    pub(crate) forward: TraversalPolicyDecision,
    pub(crate) reverse: TraversalPolicyDecision,
    pub(crate) diagnostics: Vec<String>,
}

pub(crate) fn compile_directions(way: &NormalizedWay) -> Result<WayPolicyDecision, OsmError> {
    let mut diagnostics = Vec::new();
    let Some(class_default) = way_class_default(way, &mut diagnostics) else {
        return Ok(excluded_way(diagnostics));
    };
    let (directionality, directionality_reason) = directionality(way);
    if matches!(
        directionality,
        DirectionalityDecision::UnsupportedDynamic | DirectionalityDecision::UnknownExplicit
    ) {
        diagnostics.push(format!("directionality excluded: {directionality_reason}"));
    }
    let forward_enabled = matches!(
        directionality,
        DirectionalityDecision::Bidirectional | DirectionalityDecision::ForwardOnly
    );
    let reverse_enabled = matches!(
        directionality,
        DirectionalityDecision::Bidirectional | DirectionalityDecision::ReverseOnly
    );
    Ok(WayPolicyDecision {
        directionality,
        directionality_reason,
        forward: traversal_decision(way, Direction::Forward, class_default, forward_enabled)?,
        reverse: traversal_decision(way, Direction::Reverse, class_default, reverse_enabled)?,
        diagnostics,
    })
}

pub(crate) fn apply_node_semantics(
    properties: Option<EdgeProperties>,
    nodes: [&NormalizedNode; 2],
) -> Option<EdgeProperties> {
    let mut properties = properties?;
    let mut access = properties.access();
    let mut speed = properties.effective_free_flow_speed().value();
    for node in nodes {
        if node.tags.contains_key("barrier") && node_access(node).is_none() {
            return None;
        }
        if let Some(node_access) = node_access(node) {
            if node_access != AccessClass::General {
                access = if access == AccessClass::General || access == node_access {
                    node_access
                } else {
                    AccessClass::UnknownExplicit
                };
            }
        }
        if node.tags.contains_key("ford") {
            speed = speed.min(10.0);
        }
    }
    let effective = KilometersPerHour::new(speed).ok()?;
    properties = EdgeProperties::new(effective, properties.legal_speed_limit(), access);
    Some(properties)
}

fn node_access(node: &NormalizedNode) -> Option<AccessClass> {
    let value = ["motorcycle", "motor_vehicle", "vehicle", "access"]
        .into_iter()
        .find_map(|key| node.tags.get(key))
        .map(String::as_str);
    match value
        .map(|item| normalize(item).to_ascii_lowercase())
        .as_deref()
    {
        Some("no" | "agricultural" | "forestry") => None,
        Some("yes" | "permissive" | "designated" | "official") => Some(AccessClass::General),
        Some("destination") => Some(AccessClass::Destination),
        Some("delivery") => Some(AccessClass::Delivery),
        Some("customers") => Some(AccessClass::Customers),
        Some("private") => Some(AccessClass::Private),
        Some("permit") => Some(AccessClass::PermitRequired),
        Some(_) => Some(AccessClass::UnknownExplicit),
        None if node.tags.contains_key("barrier") => None,
        None => Some(AccessClass::General),
    }
}

fn excluded_way(diagnostics: Vec<String>) -> WayPolicyDecision {
    WayPolicyDecision {
        directionality: DirectionalityDecision::UnknownExplicit,
        directionality_reason: "way class is not compiled by this profile".to_owned(),
        forward: empty_traversal(Direction::Forward),
        reverse: empty_traversal(Direction::Reverse),
        diagnostics,
    }
}

fn empty_traversal(direction: Direction) -> TraversalPolicyDecision {
    TraversalPolicyDecision {
        direction,
        access: None,
        speed: None,
        properties: None,
    }
}

fn traversal_decision(
    way: &NormalizedWay,
    direction: Direction,
    class_default: f64,
    enabled: bool,
) -> Result<TraversalPolicyDecision, OsmError> {
    if !enabled {
        return Ok(empty_traversal(direction));
    }
    let Some(access) = access_class(way, direction) else {
        return Ok(empty_traversal(direction));
    };
    let Some(speed) = speed_decision(way, direction, class_default)? else {
        return Ok(TraversalPolicyDecision {
            direction,
            access: Some(access),
            speed: None,
            properties: None,
        });
    };
    let effective = KilometersPerHour::new(speed.effective_kph)
        .map_err(|error| invalid(format!("invalid effective speed: {error}")))?;
    let legal_speed_limit = speed
        .legal_limit_kph
        .map(KilometersPerHour::new)
        .transpose()
        .map_err(|error| invalid(format!("invalid legal speed: {error}")))?;
    Ok(TraversalPolicyDecision {
        direction,
        access: Some(access),
        speed: Some(speed),
        properties: Some(EdgeProperties::new(effective, legal_speed_limit, access)),
    })
}

fn way_class_default(way: &NormalizedWay, diagnostics: &mut Vec<String>) -> Option<f64> {
    if tag(way, "route").is_some_and(|value| value.trim().eq_ignore_ascii_case("ferry")) {
        diagnostics.push("ferry connector preserved but unsupported by profile v2".to_owned());
        return None;
    }
    let highway = tag(way, "highway")?;
    if matches!(
        highway.trim().to_ascii_lowercase().as_str(),
        "construction" | "proposed" | "raceway"
    ) {
        diagnostics.push(format!("unsupported highway class {highway}"));
        return None;
    }
    default_speed_kph(highway).or_else(|| {
        if explicit_motorcycle_permission(way) {
            Some(15.0)
        } else {
            diagnostics.push(format!("unsupported highway class {highway}"));
            None
        }
    })
}

fn directionality(way: &NormalizedWay) -> (DirectionalityDecision, String) {
    if let Some(value) = tag(way, "oneway:motorcycle") {
        return parse_directionality(value, "oneway:motorcycle");
    }
    if let Some(value) = tag(way, "oneway") {
        return parse_directionality(value, "oneway");
    }
    if tag(way, "junction").is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "roundabout" | "circular"
        )
    }) {
        return (
            DirectionalityDecision::ForwardOnly,
            "supported circular junction default".to_owned(),
        );
    }
    if tag(way, "highway").is_some_and(|value| value.trim().eq_ignore_ascii_case("motorway")) {
        return (
            DirectionalityDecision::ForwardOnly,
            "motorway default".to_owned(),
        );
    }
    (
        DirectionalityDecision::Bidirectional,
        "documented missing-oneway default".to_owned(),
    )
}

fn parse_directionality(value: &str, key: &str) -> (DirectionalityDecision, String) {
    let normalized = normalize(value).to_ascii_lowercase();
    let decision = match normalized.as_str() {
        "no" | "0" | "false" => DirectionalityDecision::Bidirectional,
        "yes" | "1" | "true" => DirectionalityDecision::ForwardOnly,
        "-1" | "reverse" => DirectionalityDecision::ReverseOnly,
        "reversible" | "alternating" => DirectionalityDecision::UnsupportedDynamic,
        _ => DirectionalityDecision::UnknownExplicit,
    };
    (decision, format!("{key}={value}"))
}

fn access_class(way: &NormalizedWay, direction: Direction) -> Option<AccessClass> {
    let suffix = match direction {
        Direction::Forward => "forward",
        Direction::Reverse => "backward",
    };
    let directional_keys = [
        format!("motorcycle:{suffix}"),
        format!("motor_vehicle:{suffix}"),
        format!("vehicle:{suffix}"),
        format!("access:{suffix}"),
    ];
    let value = directional_keys
        .iter()
        .find_map(|key| tag(way, key))
        .or_else(|| tag(way, "motorcycle"))
        .or_else(|| tag(way, "motor_vehicle"))
        .or_else(|| tag(way, "vehicle"))
        .or_else(|| tag(way, "access"));
    match value
        .map(|item| normalize(item).to_ascii_lowercase())
        .as_deref()
    {
        Some("no" | "agricultural" | "forestry") => None,
        Some("yes" | "permissive" | "designated" | "official") | None => Some(AccessClass::General),
        Some("destination") => Some(AccessClass::Destination),
        Some("delivery") => Some(AccessClass::Delivery),
        Some("customers") => Some(AccessClass::Customers),
        Some("private") => Some(AccessClass::Private),
        Some("permit") => Some(AccessClass::PermitRequired),
        Some(_) => Some(AccessClass::UnknownExplicit),
    }
}

fn explicit_motorcycle_permission(way: &NormalizedWay) -> bool {
    tag(way, "motorcycle").is_some_and(|value| {
        matches!(
            normalize(value).to_ascii_lowercase().as_str(),
            "yes" | "designated" | "permissive"
        )
    })
}

fn speed_decision(
    way: &NormalizedWay,
    direction: Direction,
    class_default: f64,
) -> Result<Option<SpeedDecision>, OsmError> {
    let (legal_limit, source_status) = speed_value(way, direction);
    let (physical_cap, physical_status, allowed) = physical_policy(way);
    if !allowed {
        return Ok(None);
    }
    let mut effective = class_default.min(PROFILE_MAX_SPEED_KPH);
    if let Some(limit) = legal_limit {
        effective = effective.min(limit);
    }
    if let Some(cap) = physical_cap {
        effective = effective.min(cap);
    }
    if !effective.is_finite() || effective <= 0.0 {
        return Err(invalid(
            "speed policy produced a non-positive effective speed",
        ));
    }
    Ok(Some(SpeedDecision {
        class_default_kph: class_default,
        legal_limit_kph: legal_limit,
        profile_maximum_kph: PROFILE_MAX_SPEED_KPH,
        physical_cap_kph: physical_cap,
        effective_kph: effective,
        source_status,
        physical_status,
    }))
}

fn speed_value(way: &NormalizedWay, direction: Direction) -> (Option<f64>, String) {
    let directional_motorcycle = match direction {
        Direction::Forward => "maxspeed:motorcycle:forward",
        Direction::Reverse => "maxspeed:motorcycle:backward",
    };
    let directional = match direction {
        Direction::Forward => "maxspeed:forward",
        Direction::Reverse => "maxspeed:backward",
    };
    for key in [
        directional_motorcycle,
        "maxspeed:motorcycle",
        directional,
        "maxspeed",
    ] {
        if let Some(value) = tag(way, key) {
            return match parse_speed_kph(value) {
                SpeedParse::Parsed(speed) => (Some(speed), format!("parsed {key}={value}")),
                SpeedParse::Unsupported => (
                    None,
                    format!("unsupported explicit {key}={value}; class fallback"),
                ),
                SpeedParse::Invalid => (
                    None,
                    format!("invalid explicit {key}={value}; class fallback"),
                ),
            };
        }
    }
    (None, "missing explicit speed; class default".to_owned())
}

enum SpeedParse {
    Parsed(f64),
    Unsupported,
    Invalid,
}

fn parse_speed_kph(value: &str) -> SpeedParse {
    let normalized = normalize(value).to_ascii_lowercase();
    if normalized.contains(';') || matches!(normalized.as_str(), "none" | "signals" | "walk") {
        return SpeedParse::Unsupported;
    }
    let parsed = if let Some(number) = normalized.strip_suffix("mph") {
        number.trim().parse::<f64>().ok().map(|mph| mph * 1.609_344)
    } else {
        let number = normalized
            .strip_suffix("km/h")
            .or_else(|| normalized.strip_suffix("kph"))
            .unwrap_or(&normalized)
            .trim();
        number.parse::<f64>().ok()
    };
    match parsed {
        Some(speed) if speed.is_finite() && speed > 0.0 => SpeedParse::Parsed(speed),
        _ => SpeedParse::Invalid,
    }
}

fn physical_policy(way: &NormalizedWay) -> (Option<f64>, String, bool) {
    let mut caps = Vec::new();
    let mut reasons = Vec::new();
    for (key, policy) in [
        ("surface", surface_cap(tag(way, "surface"))),
        ("tracktype", tracktype_cap(tag(way, "tracktype"))),
        ("smoothness", smoothness_cap(tag(way, "smoothness"))),
    ] {
        match policy {
            PhysicalValue::Missing => {}
            PhysicalValue::Normal(value) => reasons.push(format!("{key}={value}:normal")),
            PhysicalValue::Capped(value, cap) => {
                reasons.push(format!("{key}={value}:cap={cap}"));
                caps.push(cap);
            }
            PhysicalValue::Forbidden(value) => {
                return (None, format!("{key}={value}:forbidden"), false);
            }
            PhysicalValue::Unknown(value) => {
                return (None, format!("{key}={value}:unknown-explicit"), false);
            }
        }
    }
    let cap = caps.into_iter().reduce(f64::min);
    let status = if reasons.is_empty() {
        "physical semantics missing; no cap".to_owned()
    } else {
        reasons.join(";")
    };
    (cap, status, true)
}

enum PhysicalValue<'a> {
    Missing,
    Normal(&'a str),
    Capped(&'a str, f64),
    Forbidden(&'a str),
    Unknown(&'a str),
}

fn surface_cap(value: Option<&str>) -> PhysicalValue<'_> {
    let Some(value) = value else {
        return PhysicalValue::Missing;
    };
    match normalize(value).to_ascii_lowercase().as_str() {
        "paved" | "asphalt" | "concrete" | "concrete:plates" | "paving_stones" => {
            PhysicalValue::Normal(value)
        }
        "unpaved" | "compacted" | "fine_gravel" | "gravel" => PhysicalValue::Capped(value, 20.0),
        "ground" | "dirt" | "earth" => PhysicalValue::Capped(value, 15.0),
        "sand" | "mud" => PhysicalValue::Forbidden(value),
        _ => PhysicalValue::Unknown(value),
    }
}

fn tracktype_cap(value: Option<&str>) -> PhysicalValue<'_> {
    let Some(value) = value else {
        return PhysicalValue::Missing;
    };
    match normalize(value).to_ascii_lowercase().as_str() {
        "grade1" => PhysicalValue::Capped(value, 20.0),
        "grade2" => PhysicalValue::Capped(value, 15.0),
        "grade3" => PhysicalValue::Capped(value, 12.0),
        "grade4" | "grade5" => PhysicalValue::Capped(value, 10.0),
        _ => PhysicalValue::Unknown(value),
    }
}

fn smoothness_cap(value: Option<&str>) -> PhysicalValue<'_> {
    let Some(value) = value else {
        return PhysicalValue::Missing;
    };
    match normalize(value).to_ascii_lowercase().as_str() {
        "excellent" | "good" | "intermediate" => PhysicalValue::Normal(value),
        "bad" => PhysicalValue::Capped(value, 20.0),
        "very_bad" => PhysicalValue::Capped(value, 15.0),
        "horrible" | "very_horrible" => PhysicalValue::Capped(value, 10.0),
        "impassable" => PhysicalValue::Forbidden(value),
        _ => PhysicalValue::Unknown(value),
    }
}

fn default_speed_kph(highway: &str) -> Option<f64> {
    match normalize(highway).to_ascii_lowercase().as_str() {
        "motorway" => Some(80.0),
        "motorway_link" | "trunk" => Some(60.0),
        "trunk_link" | "primary" => Some(50.0),
        "primary_link" | "secondary" => Some(45.0),
        "secondary_link" | "tertiary" => Some(40.0),
        "tertiary_link" | "unclassified" | "road" => Some(35.0),
        "residential" => Some(30.0),
        "living_street" => Some(15.0),
        "service" | "track" => Some(20.0),
        _ => None,
    }
}

fn tag<'a>(way: &'a NormalizedWay, key: &str) -> Option<&'a str> {
    way.tags
        .get(key)
        .or_else(|| way.unsupported_tags.get(key))
        .map(String::as_str)
}

fn normalize(value: &str) -> &str {
    value.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn way(tags: &[(&str, &str)]) -> NormalizedWay {
        let mut supported = BTreeMap::new();
        let mut unsupported = BTreeMap::new();
        for (key, value) in tags {
            if matches!(
                *key,
                "surface" | "tracktype" | "smoothness" | "access:forward"
            ) {
                unsupported.insert((*key).to_owned(), (*value).to_owned());
            } else {
                supported.insert((*key).to_owned(), (*value).to_owned());
            }
        }
        NormalizedWay {
            osm_id: 1,
            node_refs: vec![1, 2],
            tags: supported,
            unsupported_tags: unsupported,
        }
    }

    #[test]
    fn compiles_reverse_direction_and_reason_specific_access() {
        let decision = compile_directions(&way(&[
            ("highway", "residential"),
            ("oneway", "-1"),
            ("access", "destination"),
            ("maxspeed", "30 mph"),
        ]))
        .unwrap_or_else(|error| panic!("valid policy: {error}"));
        assert!(decision.forward.properties.is_none());
        let reverse = decision
            .reverse
            .properties
            .unwrap_or_else(|| panic!("reverse traversal"));
        assert_eq!(reverse.access(), AccessClass::Destination);
        assert!((reverse.effective_free_flow_speed().value() - 30.0).abs() < f64::EPSILON);
    }

    #[test]
    fn unknown_directionality_emits_no_edges() {
        let decision = compile_directions(&way(&[("highway", "primary"), ("oneway", "sideways")]))
            .unwrap_or_else(|error| panic!("policy decision: {error}"));
        assert_eq!(
            decision.directionality,
            DirectionalityDecision::UnknownExplicit
        );
        assert!(decision.forward.properties.is_none());
        assert!(decision.reverse.properties.is_none());
    }

    #[test]
    fn unpaved_surface_caps_free_flow_speed() {
        let decision = compile_directions(&way(&[("highway", "primary"), ("surface", "unpaved")]))
            .unwrap_or_else(|error| panic!("policy decision: {error}"));
        let forward = decision
            .forward
            .properties
            .unwrap_or_else(|| panic!("forward traversal"));
        assert!((forward.effective_free_flow_speed().value() - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn motorcycle_oneway_override_restores_both_directions() {
        let decision = compile_directions(&way(&[
            ("highway", "primary"),
            ("oneway", "yes"),
            ("oneway:motorcycle", "no"),
        ]))
        .unwrap_or_else(|error| panic!("policy decision: {error}"));
        assert!(decision.forward.properties.is_some());
        assert!(decision.reverse.properties.is_some());
    }
}
