use roadrunner_core::graph::FrozenGraph;
use roadrunner_dispatch::PreparationAwareStrategy;
use serde::Serialize;

use crate::{DispatchPolicy, SimulationError, SimulationResult, SimulationScenario, simulate};

/// Paired strategy results over one graph and identical exogenous scenario facts.
/// Each result includes summaries, unfinished populations, decisions, and execution evidence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyComparison {
    /// Comparison schema version, independent from simulation output schema.
    pub schema_version: u32,
    /// Independent runs in canonical A/B/C/D order.
    pub runs: Vec<SimulationResult>,
}

/// Runs all four Phase 16 strategies from fresh worlds with identical seed and inputs.
/// The scenario's dispatch field is replaced; all other facts remain fixed. No actual
/// readiness is supplied to ranking before its observation event.
///
/// # Errors
/// Rejects invalid weights or any scenario/evaluation/execution failure. No partial
/// comparison is returned, and no invalid run becomes an Unassigned result.
pub fn compare_strategies(
    graph: &FrozenGraph,
    scenario: &SimulationScenario,
    idle_penalty_weight: f64,
) -> Result<StrategyComparison, SimulationError> {
    PreparationAwareStrategy::new(idle_penalty_weight)?;
    let mut runs = Vec::with_capacity(4);
    for dispatch in [
        DispatchPolicy::NearestRider,
        DispatchPolicy::LowestPickupEta,
        DispatchPolicy::LowestCompletionTime,
        DispatchPolicy::PreparationAware {
            idle_penalty_weight,
        },
    ] {
        let mut paired = scenario.clone();
        paired.dispatch = dispatch;
        runs.push(simulate(graph, &paired)?);
    }
    Ok(StrategyComparison {
        schema_version: 1,
        runs,
    })
}
