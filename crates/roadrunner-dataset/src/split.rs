//! Exact, claim-specific assignments with transitive dependency closure; no row randomness.
use crate::{DatasetError, SourceCapture};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Declared role of a whole protected dependency component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Partition {
    /// Descriptive source validation, no independence claim.
    Descriptive,
    /// Authorized future fitting population.
    Train,
    /// Development holdout, never a final test after repeated tuning.
    Validation,
    /// Protected final holdout.
    Test,
    /// Explicitly excluded with a retained reason.
    Excluded,
}
/// Caller-declared assignments, frozen before Phase 24 evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitRequest {
    /// Generalization target, not a universal independence assertion.
    pub claim: String,
    /// Every retained source run must have exactly one assignment.
    pub assignments: BTreeMap<String, Partition>,
    /// Excluded source reasons; no silent inconvenient-source removal.
    pub excluded_reasons: BTreeMap<String, String>,
}
/// Whether the declared package supports a protected predictive holdout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationAvailability {
    /// Independent protected components exist for the declared synthetic claim only.
    Available,
    /// No manufactured holdout or physical-generalization pass.
    EvaluationUnavailable(String),
}
/// Frozen exact assignments and connected components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SplitPlan {
    /// Exact caller request retained for reproduction.
    pub request: SplitRequest,
    /// Canonical component ID (lexicographically first run) per source.
    pub groups: BTreeMap<String, String>,
    /// Predictive evaluation availability, separate from engineering publication.
    pub evaluation: EvaluationAvailability,
}
fn dependency_keys(s: &SourceCapture) -> BTreeSet<String> {
    let mut keys = s.lineage.dependencies.clone();
    keys.extend([
        format!("graph:{}", s.graph.sha256),
        format!("family:{}", s.lineage.family),
        format!("world:{}", s.lineage.base_world),
        format!("realization:{}", s.lineage.realization),
        format!("replay:{}:{}", s.graph.sha256, s.scenario.sha256),
    ]);
    keys
}

/// Conservatively protects graph, family, base world, realization and explicit episode ancestry.
/// This protocol tests unseen graph/family/base-world components, not random-row independence.
/// # Errors
/// Rejects missing/extra assignments, conflicting shared dependencies and unsupported claims.
pub fn plan_splits(
    sources: &[SourceCapture],
    request: SplitRequest,
) -> Result<SplitPlan, DatasetError> {
    if request.claim != "descriptive-synthetic/v1"
        && request.claim != "unseen-synthetic-components/v1"
    {
        return Err(DatasetError::Split(
            "unsupported generalization claim".into(),
        ));
    }
    let mut groups: BTreeMap<_, _> = sources
        .iter()
        .map(|s| (s.run.clone(), s.run.clone()))
        .collect();
    if groups.len() != sources.len() || groups.keys().ne(request.assignments.keys()) {
        return Err(DatasetError::Split(
            "duplicate/missing/extra source assignment".into(),
        ));
    }
    let keys: Vec<_> = sources.iter().map(dependency_keys).collect();
    loop {
        let before = groups.clone();
        for (i, a) in sources.iter().enumerate() {
            for (j, b) in sources.iter().enumerate().skip(i + 1) {
                if !keys[i].is_disjoint(&keys[j]) {
                    let old_a = groups[&a.run].clone();
                    let old_b = groups[&b.run].clone();
                    let canonical = old_a.clone().min(old_b.clone());
                    for group in groups.values_mut() {
                        if *group == old_a || *group == old_b {
                            group.clone_from(&canonical);
                        }
                    }
                }
            }
        }
        if groups == before {
            break;
        }
    }
    let mut roles = BTreeMap::new();
    for (run, group) in &groups {
        let role = request.assignments[run];
        if let Some(old) = roles.insert(group.clone(), role) {
            if old != role {
                return Err(DatasetError::Split(
                    "dependency crosses protected partitions".into(),
                ));
            }
        }
        if role == Partition::Excluded
            && request
                .excluded_reasons
                .get(run)
                .is_none_or(String::is_empty)
        {
            return Err(DatasetError::Split("excluded source has no reason".into()));
        }
    }
    if request
        .excluded_reasons
        .keys()
        .any(|r| request.assignments.get(r) != Some(&Partition::Excluded))
    {
        return Err(DatasetError::Split(
            "invalid exclusion reason assignment".into(),
        ));
    }
    let evaluation = if request.claim == "unseen-synthetic-components/v1"
        && roles.values().any(|p| *p == Partition::Train)
        && roles.values().any(|p| *p == Partition::Validation)
        && roles.values().any(|p| *p == Partition::Test)
    {
        EvaluationAvailability::Available
    } else {
        EvaluationAvailability::EvaluationUnavailable(
            "InsufficientProtectedIndependentComponents; descriptive only".into(),
        )
    };
    if request.claim == "descriptive-synthetic/v1"
        && roles
            .values()
            .any(|p| !matches!(p, Partition::Descriptive | Partition::Excluded))
    {
        return Err(DatasetError::Split(
            "descriptive claim cannot assign fitting/holdout roles".into(),
        ));
    }
    Ok(SplitPlan {
        request,
        groups,
        evaluation,
    })
}
