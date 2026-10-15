//! Bounded immutable synthetic capture, independently replayable from retained inputs.
use crate::{DatasetError, canonical, digest};
use roadrunner_core::graph::{FrozenGraph, decode_graph_artifact, encode_graph_artifact};
use roadrunner_dispatch::{ActionId, ExecutionEffect};
use roadrunner_simulation::{SimulationScenario, SyntheticTrace, simulate_with_run_identity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Hierarchical metadata; all fields are lineage/grouping, never predictors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyntheticLineage {
    /// Generator mechanism, not an independence assertion.
    pub generator: String,
    /// Scenario ancestry/family, shared descendants must retain it.
    pub family: String,
    /// Base-world ancestry established by the source owner.
    pub base_world: String,
    /// Concrete exogenous realization; equal seeds alone do not prove coupling.
    pub realization: String,
    /// Policy variant, not an observed counterfactual or predictor.
    pub policy: String,
    /// Shared replay/episode/coupling references; closure protects every shared key.
    pub dependencies: BTreeSet<String>,
}
/// Exact immutable bytes with separate integrity identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedBytes {
    /// Recoverable original UTF-8 encoding, not just a digest.
    pub utf8: String,
    /// SHA-256 of the precise original bytes.
    pub sha256: String,
}
impl RetainedBytes {
    /// Captures exact bytes without reinterpreting their meaning.
    #[must_use]
    pub fn new(utf8: String) -> Self {
        Self {
            sha256: digest(utf8.as_bytes()),
            utf8,
        }
    }
    /// Checks retained bytes, not semantic compatibility.
    /// # Errors
    /// Fails on any byte/digest mismatch.
    pub fn verify(&self) -> Result<(), DatasetError> {
        if digest(self.utf8.as_bytes()) != self.sha256 {
            return Err(DatasetError::Integrity("retained byte digest".into()));
        }
        Ok(())
    }
}
/// One fresh simulation's inclusive boundary. No global cross-run instant is asserted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCapture {
    /// Capture/source schema and interpretation.
    pub schema: String,
    /// Archive-assigned run identity; repeated namespace/ActionId is run-qualified.
    pub run: String,
    /// Newly derived instrumented synthetic evidence, never retrofitted old movements.
    pub derivation: String,
    /// Recoverable graph schema 4 bytes, semantic verification performed on load.
    pub graph: RetainedBytes,
    /// Complete original generator/configuration/seed/policy and horizon bytes.
    pub scenario: RetainedBytes,
    /// Complete result/trace bytes at the inclusive horizon.
    pub result: RetainedBytes,
    /// Hierarchical source relationships.
    pub lineage: SyntheticLineage,
}
impl SourceCapture {
    /// Runs the unchanged deterministic engine and captures a fresh derived source.
    /// # Errors
    /// Rejects invalid graph/scenario, empty source identities or execution failure.
    pub fn simulate(
        graph: &FrozenGraph,
        scenario: &SimulationScenario,
        run: &str,
        lineage: SyntheticLineage,
    ) -> Result<Self, DatasetError> {
        let result = simulate_with_run_identity(graph, scenario, run)
            .map_err(|e| DatasetError::Source(e.to_string()))?;
        Ok(Self {
            schema: "eta-source/v1".into(),
            run: run.into(),
            derivation: "new-instrumented-resimulation/v1".into(),
            graph: RetainedBytes::new(
                String::from_utf8(
                    encode_graph_artifact(graph)
                        .map_err(|e| DatasetError::Source(e.to_string()))?,
                )
                .map_err(|e| DatasetError::Source(e.to_string()))?,
            ),
            scenario: RetainedBytes::new(
                String::from_utf8(canonical(scenario)?)
                    .map_err(|e| DatasetError::Source(e.to_string()))?,
            ),
            result: RetainedBytes::new(
                String::from_utf8(canonical(&result)?)
                    .map_err(|e| DatasetError::Source(e.to_string()))?,
            ),
            lineage,
        })
    }
    /// Verifies bytes, original graph semantics, deterministic source replay and allocator/effect census.
    /// # Errors
    /// Corrupt/unsupported evidence blocks this source's completeness/publication claim.
    pub fn verify(&self) -> Result<(FrozenGraph, SyntheticTrace), DatasetError> {
        if self.schema != "eta-source/v1"
            || self.derivation != "new-instrumented-resimulation/v1"
            || self.run.is_empty()
            || [
                &self.lineage.generator,
                &self.lineage.family,
                &self.lineage.base_world,
                &self.lineage.realization,
                &self.lineage.policy,
            ]
            .iter()
            .any(|s| s.is_empty())
        {
            return Err(DatasetError::Source(
                "unsupported source/lineage identity".into(),
            ));
        }
        self.graph.verify()?;
        self.scenario.verify()?;
        self.result.verify()?;
        let graph = decode_graph_artifact(self.graph.utf8.as_bytes()).map_err(|e| {
            DatasetError::Source(format!("original graph semantics unavailable: {e}"))
        })?;
        let scenario: SimulationScenario = serde_json::from_str(&self.scenario.utf8)?;
        let replay = simulate_with_run_identity(&graph, &scenario, &self.run)
            .map_err(|e| DatasetError::Source(e.to_string()))?;
        let recorded: serde_json::Value = serde_json::from_str(&self.result.utf8)?;
        if canonical(&recorded)? != canonical(&replay)? {
            return Err(DatasetError::Integrity(
                "instrumented source replay differs".into(),
            ));
        }
        let trace = replay.trace;
        reconcile_trace(&trace)?;
        Ok((graph, trace))
    }
    /// Recoverable content identity includes boundary, lineage and all source bytes.
    /// # Errors
    /// Rejects invalid JSON representation.
    pub fn content_id(&self) -> Result<String, DatasetError> {
        Ok(digest(&canonical(self)?))
    }
}

/// Checks census using authority allocators/effects and pending attempts, independently of labels.
/// # Errors
/// Rejects holes, duplicate starts, owner mismatch, malformed schedules or missing horizon evidence.
pub fn reconcile_trace(t: &SyntheticTrace) -> Result<(), DatasetError> {
    let fail = || DatasetError::Integrity("started trace/authority census mismatch".into());
    if t.semantics != "synthetic-started-actions/v1" || t.run_identity.is_empty() {
        return Err(fail());
    }
    let mut starts = BTreeMap::new();
    for (i, s) in t.starts.iter().enumerate() {
        let a = &s.original;
        if a.action_id.value() != u64::try_from(i).map_err(|_| fail())?
            || a.generation.value() != 0
            || a.route.departed_at > t.horizon
            || s.committed_revision.namespace != t.final_revision.namespace
            || s.committed_revision.number > t.final_revision.number
            || !t
                .history
                .iter()
                .any(|h| h.after == s.committed_revision && h.execution_changed.contains(&s.rider))
            || starts.insert(a.action_id, s).is_some()
        {
            return Err(fail());
        }
    }
    if u64::try_from(starts.len()).map_err(|_| fail())? != t.committed_start_count {
        return Err(fail());
    }
    let mut generations: BTreeMap<ActionId, u64> = starts.keys().map(|a| (*a, 0)).collect();
    for change in &t.schedules {
        let a = &change.execution;
        let s = starts.get(&a.action_id).ok_or_else(fail)?;
        let generation = generations.get_mut(&a.action_id).ok_or_else(fail)?;
        *generation += 1;
        if change.rider != s.rider
            || a.route != s.original.route
            || a.generation.value() != *generation
            || change.at > t.horizon
            || change.revision.number <= s.committed_revision.number
        {
            return Err(fail());
        }
    }
    let mut arrivals = BTreeMap::new();
    for e in &t.effects {
        let s = starts.get(&e.action).ok_or_else(fail)?;
        if e.rider != s.rider || e.at > t.horizon || e.generation.value() > generations[&e.action] {
            return Err(fail());
        }
        if e.effect == ExecutionEffect::Arrival {
            if let Some(old) = arrivals.insert(e.action, (e.at, e.generation)) {
                if old != (e.at, e.generation) {
                    return Err(fail());
                }
            }
        }
    }
    let mut arrival_actions = BTreeSet::new();
    for record in &t.arrivals {
        let start = starts.get(&record.action).ok_or_else(fail)?;
        if !arrival_actions.insert(record.action)
            || record.rider != start.rider
            || arrivals.get(&record.action) != Some(&(record.reported_at, record.generation))
            || record.processing_sample_at != record.reported_at
            || record.processing_sample_at > t.horizon
        {
            return Err(fail());
        }
    }
    if arrival_actions.len() != arrivals.len() {
        return Err(fail());
    }
    for s in &t.starts {
        if !arrivals.contains_key(&s.original.action_id) {
            let active = t.final_active.get(&s.rider).ok_or_else(fail)?;
            if active.action_id != s.original.action_id
                || active.route != s.original.route
                || !t.pending.iter().any(|p| {
                    p.at > t.horizon
                        && p.action["kind"] == "Move"
                        && p.action["data"]["rider"] == serde_json::json!(s.rider)
                        && p.action["data"]["execution_id"] == serde_json::json!(active.id)
                })
            {
                return Err(fail());
            }
        }
    }
    Ok(())
}
