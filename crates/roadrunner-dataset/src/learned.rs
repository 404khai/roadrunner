//! Verified, restricted interchange for offline whole-route fitting and inference.
use crate::{ActionKey, DatasetBundle, DatasetError, baseline, canonical, digest};
use roadrunner_dispatch::FrozenExecutionLeg;
use serde::{Deserialize, Serialize};

/// Bound whole-route request. Metadata is never part of the numeric feature vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteInput {
    /// Canonical committed start; not an edge or counterfactual observation.
    pub key: ActionKey,
    /// Original pinned directed route and original routing context.
    pub route: FrozenExecutionLeg,
    /// Digest checked before inference, separate from action identity.
    pub route_sha256: String,
    /// Historical prediction reference in synthetic logical seconds.
    pub reference_seconds: f64,
    /// Original start-order witness, serialized without narrowing integers.
    pub witness: crate::AvailabilityWitness,
    /// Only B0 seconds and route distance meters, in that exact order.
    pub values: [f64; 2],
}

/// A verified interchange retains labels separately from restricted inference inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResearchExport {
    /// Target, units and preprocessing boundary version.
    pub schema: String,
    /// Rust-verified baseline, census, partitions, cohorts and diagnostic segments.
    pub baseline: baseline::BaselineReport,
    /// No scenario, seed, future traffic, arrival or intervention predictors.
    pub inputs: Vec<RouteInput>,
}

/// Verifies original sources, reruns the simulator, and exports restricted inputs.
/// # Errors
/// Rejects invalid source/census/features/splits; never exports unverified rows.
pub fn export_research(dataset: &DatasetBundle) -> Result<ResearchExport, DatasetError> {
    let baseline = baseline::evaluate_b0(dataset)?;
    let inputs = dataset
        .rows
        .iter()
        .zip(&baseline.actions)
        .map(|(row, action)| {
            Ok(RouteInput {
                key: row.key.clone(),
                route: row.features.original_route.clone(),
                route_sha256: digest(&canonical(&row.features.original_route)?),
                reference_seconds: action.start_effective_at.value(),
                witness: row.features.witness.clone(),
                values: [
                    row.features.b0_seconds.value(),
                    row.features.distance_meters.value(),
                ],
            })
        })
        .collect::<Result<_, DatasetError>>()?;
    Ok(ResearchExport {
        schema: "eta-route-research/v1; whole-started-leg; seconds,meters; binary64".into(),
        baseline,
        inputs,
    })
}
