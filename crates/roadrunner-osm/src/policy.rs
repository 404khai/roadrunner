use std::collections::BTreeMap;

use roadrunner_core::geo::KilometersPerHour;
use roadrunner_core::graph::{AccessClass, EdgeProperties};

use crate::error::{OsmError, invalid};
use crate::model::NormalizedWay;

/// Initial delivery-motorcycle routing profile identifier.
pub const DELIVERY_MOTORCYCLE_PROFILE: &str = "delivery_motorcycle_v1";
/// Initial Nigeria default-access and speed-policy identifier.
pub const NG_JURISDICTION_POLICY: &str = "ng_v1";

#[derive(Debug, Clone, Copy)]
pub(crate) enum Direction {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy)]
enum Directionality {
    Both,
    Forward,
    Reverse,
}

pub(crate) fn compile_directions(
    way: &NormalizedWay,
) -> Result<(Option<EdgeProperties>, Option<EdgeProperties>), OsmError> {
    let highway = tag(&way.tags, "highway")
        .ok_or_else(|| invalid(format!("normalized way {} lacks highway", way.osm_id)))?;
    let default_speed = match default_speed_kph(highway) {
        Some(speed) => speed,
        None if explicit_motorcycle_permission(&way.tags) => 15.0,
        None => return Ok((None, None)),
    };
    if matches!(
        highway.trim().to_ascii_lowercase().as_str(),
        "construction" | "proposed" | "raceway"
    ) {
        return Ok((None, None));
    }
    let directionality = directionality(&way.tags);
    let forward = if matches!(
        directionality,
        Directionality::Both | Directionality::Forward
    ) {
        properties(&way.tags, Direction::Forward, default_speed)?
    } else {
        None
    };
    let reverse = if matches!(
        directionality,
        Directionality::Both | Directionality::Reverse
    ) {
        properties(&way.tags, Direction::Reverse, default_speed)?
    } else {
        None
    };
    Ok((forward, reverse))
}

fn properties(
    tags: &BTreeMap<String, String>,
    direction: Direction,
    default_speed: f64,
) -> Result<Option<EdgeProperties>, OsmError> {
    let Some(access) = access_class(tags, direction) else {
        return Ok(None);
    };
    let legal_speed = speed_value(tags, direction);
    let effective = legal_speed.map_or(default_speed, |limit| default_speed.min(limit));
    if effective <= 0.0 {
        return Ok(None);
    }
    let effective = KilometersPerHour::new(effective)
        .map_err(|error| invalid(format!("invalid effective speed: {error}")))?;
    let legal_speed_limit = legal_speed
        .map(KilometersPerHour::new)
        .transpose()
        .map_err(|error| invalid(format!("invalid legal speed: {error}")))?;
    Ok(Some(EdgeProperties::new(
        effective,
        legal_speed_limit,
        access,
    )))
}

fn directionality(tags: &BTreeMap<String, String>) -> Directionality {
    if let Some(value) = tag(tags, "oneway:motorcycle") {
        match normalize(value).to_ascii_lowercase().as_str() {
            "no" | "0" | "false" => return Directionality::Both,
            "yes" | "1" | "true" => return Directionality::Forward,
            "-1" | "reverse" => return Directionality::Reverse,
            _ => {}
        }
    }
    if let Some(value) = tag(tags, "oneway") {
        match normalize(value).to_ascii_lowercase().as_str() {
            "no" | "0" | "false" => return Directionality::Both,
            "-1" | "reverse" => return Directionality::Reverse,
            "yes" | "1" | "true" => return Directionality::Forward,
            _ => {}
        }
    }
    if tag(tags, "junction").is_some_and(|value| value.trim().eq_ignore_ascii_case("roundabout"))
        || tag(tags, "highway").is_some_and(|value| value.trim().eq_ignore_ascii_case("motorway"))
    {
        Directionality::Forward
    } else {
        Directionality::Both
    }
}

fn access_class(tags: &BTreeMap<String, String>, direction: Direction) -> Option<AccessClass> {
    let directional_key = match direction {
        Direction::Forward => "motorcycle:forward",
        Direction::Reverse => "motorcycle:backward",
    };
    let value = tag(tags, directional_key)
        .or_else(|| tag(tags, "motorcycle"))
        .or_else(|| tag(tags, "motor_vehicle"))
        .or_else(|| tag(tags, "vehicle"))
        .or_else(|| tag(tags, "access"));
    match value
        .map(|item| normalize(item).to_ascii_lowercase())
        .as_deref()
    {
        Some("no" | "agricultural" | "forestry") => None,
        Some("yes" | "permissive" | "designated" | "official") | None => Some(AccessClass::General),
        Some(_) => Some(AccessClass::Contextual),
    }
}

fn explicit_motorcycle_permission(tags: &BTreeMap<String, String>) -> bool {
    tag(tags, "motorcycle").is_some_and(|value| {
        matches!(
            normalize(value).to_ascii_lowercase().as_str(),
            "yes" | "designated" | "permissive"
        )
    })
}

fn speed_value(tags: &BTreeMap<String, String>, direction: Direction) -> Option<f64> {
    let directional_motorcycle = match direction {
        Direction::Forward => "maxspeed:motorcycle:forward",
        Direction::Reverse => "maxspeed:motorcycle:backward",
    };
    let directional = match direction {
        Direction::Forward => "maxspeed:forward",
        Direction::Reverse => "maxspeed:backward",
    };
    [
        directional_motorcycle,
        "maxspeed:motorcycle",
        directional,
        "maxspeed",
    ]
    .into_iter()
    .find_map(|key| tag(tags, key).and_then(parse_speed_kph))
}

fn parse_speed_kph(value: &str) -> Option<f64> {
    let normalized = normalize(value).to_ascii_lowercase();
    if normalized.contains(';') || normalized == "none" || normalized == "signals" {
        return None;
    }
    if let Some(number) = normalized.strip_suffix("mph") {
        return number
            .trim()
            .parse::<f64>()
            .ok()
            .map(|mph| mph * 1.609_344)
            .filter(|speed| speed.is_finite() && *speed > 0.0);
    }
    let number = normalized
        .strip_suffix("km/h")
        .or_else(|| normalized.strip_suffix("kph"))
        .unwrap_or(&normalized)
        .trim();
    number
        .parse::<f64>()
        .ok()
        .filter(|speed| speed.is_finite() && *speed > 0.0)
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

fn tag<'a>(tags: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    tags.get(key).map(String::as_str)
}

fn normalize(value: &str) -> &str {
    value.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn way(tags: &[(&str, &str)]) -> NormalizedWay {
        NormalizedWay {
            osm_id: 1,
            node_refs: vec![1, 2],
            tags: tags
                .iter()
                .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
                .collect(),
            unsupported_tags: BTreeMap::new(),
        }
    }

    #[test]
    fn compiles_static_direction_and_contextual_access() {
        let road = way(&[
            ("highway", "residential"),
            ("oneway", "-1"),
            ("access", "destination"),
            ("maxspeed", "30 mph"),
        ]);
        let Ok((forward, reverse)) = compile_directions(&road) else {
            panic!("valid policy");
        };
        assert!(forward.is_none());
        let Some(reverse) = reverse else {
            panic!("reverse traversal");
        };
        assert_eq!(reverse.access(), AccessClass::Contextual);
        assert!((reverse.effective_free_flow_speed().value() - 30.0).abs() < f64::EPSILON);
        assert!(reverse.legal_speed_limit().is_some());
    }

    #[test]
    fn motorcycle_override_allows_other_highway_class() {
        let road = way(&[("highway", "path"), ("motorcycle", "yes")]);
        let Ok((forward, reverse)) = compile_directions(&road) else {
            panic!("valid policy");
        };
        assert!(forward.is_some());
        assert!(reverse.is_some());
    }

    #[test]
    fn motorcycle_oneway_override_restores_both_directions() {
        let road = way(&[
            ("highway", "primary"),
            ("oneway", "yes"),
            ("oneway:motorcycle", "no"),
        ]);
        let Ok((forward, reverse)) = compile_directions(&road) else {
            panic!("valid policy");
        };
        assert!(forward.is_some());
        assert!(reverse.is_some());
    }

    #[test]
    fn directional_access_and_speed_are_independent() {
        let road = way(&[
            ("highway", "primary"),
            ("motorcycle:forward", "no"),
            ("maxspeed:backward", "20"),
        ]);
        let Ok((forward, reverse)) = compile_directions(&road) else {
            panic!("valid policy");
        };
        assert!(forward.is_none());
        let Some(reverse) = reverse else {
            panic!("reverse traversal");
        };
        assert!((reverse.effective_free_flow_speed().value() - 20.0).abs() < f64::EPSILON);
    }
}
