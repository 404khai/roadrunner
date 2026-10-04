//! Deterministic Phase 14 comparisons with fixed exogenous readiness events.

#[path = "support/preparation_scenario.rs"]
mod scenario;

use scenario::{Scenario, ScenarioResult, run};

fn main() -> ScenarioResult<()> {
    let mut comparisons = Vec::new();
    for (name, ready_after_seconds, actual_after_seconds) in [
        ("long-preparation", 1080.0, 1080.0),
        ("already-ready", 0.0, 0.0),
        ("underestimated-preparation", 1080.0, 1500.0),
    ] {
        let scenario = Scenario::new(ready_after_seconds)?;
        let ready_at = scenario
            .at
            .checked_add(roadrunner_core::geo::Seconds::new(actual_after_seconds)?)?;
        let baseline = run(&scenario, false, ready_at)?;
        let preparation = run(&scenario, true, ready_at)?;
        assert_eq!(baseline.observed.rider, scenario.near);
        if ready_after_seconds > 0.0 {
            assert_eq!(preparation.observed.rider, scenario.far);
            assert!(preparation.observed.waiting < baseline.observed.waiting);
            assert_eq!(
                preparation.observed.delivery_completed_at,
                baseline.observed.delivery_completed_at
            );
        } else {
            assert_eq!(preparation.observed.rider, scenario.near);
        }
        comparisons.push(serde_json::json!({
            "scenario": name,
            "actual_ready_at": ready_at,
            "baseline": baseline,
            "preparation_aware": preparation,
        }));
    }
    println!("{}", serde_json::to_string_pretty(&comparisons)?);
    Ok(())
}
