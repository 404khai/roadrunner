use roadrunner_core::geo::Seconds;
use serde::Serialize;

use crate::evaluation::rank_scores;
use crate::{
    BaselineRanking, DispatchEvaluationError, DispatchInstant, FeasibleCandidate, OrderReadiness,
    ScoreContributions,
};

/// Readiness fact used to predict pickup waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReadinessSource {
    /// An actual ready event overrides any forecast.
    Observed,
    /// No actual ready event exists, so use the current forecast.
    Expected,
}

/// Decision-scoped readiness inputs and explicit waiting-penalty configuration.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PreparationPolicyEvidence {
    /// Both original forecast and observation, preserved independently.
    pub readiness: OrderReadiness,
    /// Effective ready instant used by all candidates in this decision.
    pub effective_ready_at: DispatchInstant,
    /// Whether the effective instant is observed or forecast.
    pub source: ReadinessSource,
    /// Non-negative finite penalty seconds per second waiting at pickup.
    pub idle_penalty_weight: f64,
}

impl PreparationPolicyEvidence {
    pub(crate) fn new(
        readiness: OrderReadiness,
        strategy: PreparationAwareStrategy,
    ) -> Result<Self, DispatchEvaluationError> {
        let (effective_ready_at, source) = if let Some(at) = readiness.observed_at {
            (at, ReadinessSource::Observed)
        } else if let Some(at) = readiness.expected_at {
            (at, ReadinessSource::Expected)
        } else {
            return Err(DispatchEvaluationError::MissingReadiness);
        };
        Ok(Self {
            readiness,
            effective_ready_at,
            source,
            idle_penalty_weight: strategy.idle_penalty_weight(),
        })
    }
}

/// Preparation-aware objective breakdown, all durations expressed in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PreparationScoreContributions {
    /// Elapsed evaluation-to-delivery duration, including waiting exactly once.
    pub completion_time: Seconds,
    /// Pickup idle time, distinct from road travel and stop service.
    pub waiting: Seconds,
    /// Additional preference penalty: waiting multiplied by configured weight.
    pub idle_penalty: Seconds,
}

/// Pure Phase 14 ranking: elapsed completion time plus weighted pickup waiting.
///
/// Timing, feasibility, and routing have already been evaluated. A positive penalty
/// may favor a later-arriving rider when it saves idle time; it is not an SLA constraint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreparationAwareStrategy {
    idle_penalty_weight: f64,
}

impl Default for PreparationAwareStrategy {
    fn default() -> Self {
        Self {
            idle_penalty_weight: 1.0,
        }
    }
}

impl PreparationAwareStrategy {
    /// Configures penalty seconds per second waiting at pickup; zero is allowed.
    ///
    /// # Errors
    /// Rejects negative or non-finite weights with `InvalidMetric`.
    pub fn new(idle_penalty_weight: f64) -> Result<Self, DispatchEvaluationError> {
        if !idle_penalty_weight.is_finite() || idle_penalty_weight < 0.0 {
            return Err(DispatchEvaluationError::InvalidMetric);
        }
        Ok(Self {
            idle_penalty_weight,
        })
    }

    /// Returns the explicit dimensionless waiting-penalty weight.
    #[must_use]
    pub const fn idle_penalty_weight(self) -> f64 {
        self.idle_penalty_weight
    }

    /// Ranks feasible plans by checked completion duration plus idle penalty.
    ///
    /// # Errors
    /// Rejects non-finite multiplication/sums and duplicate candidate identities.
    pub fn rank(
        self,
        candidates: &[FeasibleCandidate],
    ) -> Result<BaselineRanking, DispatchEvaluationError> {
        let mut scores = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            let evaluation = &candidate.evaluation;
            let idle_penalty = Seconds::new(evaluation.waiting.value() * self.idle_penalty_weight)
                .map_err(|_| DispatchEvaluationError::InvalidMetric)?;
            let total = evaluation
                .completion_time
                .checked_add(idle_penalty)
                .map_err(|_| DispatchEvaluationError::InvalidMetric)?;
            scores.push((
                candidate.rider,
                ScoreContributions {
                    pickup_travel: evaluation.pickup_travel,
                    delivery_travel: evaluation.delivery_travel,
                    preparation: Some(PreparationScoreContributions {
                        completion_time: evaluation.completion_time,
                        waiting: evaluation.waiting,
                        idle_penalty,
                    }),
                    total,
                },
            ));
        }
        rank_scores(scores)
    }
}
