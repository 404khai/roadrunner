//! Deterministic, scaled binary64 action-weighted errors; no fitting or MAPE.
use crate::DatasetError;
use serde::{Deserialize, Serialize};

/// Nearest-rank quantiles at fixed probabilities, independent of midpoint median.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorQuantiles {
    /// Smallest ordered value with empirical cumulative probability at least 0.50.
    pub p50: f64,
    /// Nearest-rank 0.95 quantile.
    pub p95: f64,
    /// Nearest-rank 0.99 quantile.
    pub p99: f64,
}
/// Exact point-error metrics over equally weighted, eligible, identity-paired actions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Number of paired logical actions, not independent groups.
    pub samples: usize,
    /// Primary mean absolute error in seconds.
    pub mae_seconds: f64,
    /// Root mean squared error in seconds.
    pub rmse_seconds: f64,
    /// Mean signed label-minus-B0 residual in seconds.
    pub mean_signed_residual_seconds: f64,
    /// Midpoint median signed residual in seconds.
    pub median_signed_residual_seconds: f64,
    /// Fixed nearest-rank absolute-error tails in seconds.
    pub absolute_error_quantiles_seconds: ErrorQuantiles,
    /// Strictly positive residual tails; absent with no underpredictions.
    pub positive_residual_quantiles_seconds: Option<ErrorQuantiles>,
    /// Actions with label greater than prediction (no epsilon threshold).
    pub underprediction_count: usize,
    /// Strictly positive residual count divided by paired actions.
    pub underprediction_frequency: f64,
    /// Mean strictly positive residual; unavailable when there are none.
    pub underprediction_severity_seconds: Option<f64>,
}
pub(super) fn count(n: usize) -> Result<f64, DatasetError> {
    u32::try_from(n)
        .map(f64::from)
        .map_err(|_| DatasetError::Source("metric population exceeds supported u32 domain".into()))
}
fn mean(values: &[f64]) -> Result<f64, DatasetError> {
    let scale = values.iter().fold(0.0_f64, |m, x| m.max(x.abs()));
    if scale == 0.0 {
        return Ok(0.0);
    }
    let (mut sum, mut correction) = (0.0, 0.0);
    for value in values {
        let adjusted = value / scale - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    Ok(scale * (sum / count(values.len())?))
}
fn quantiles(ordered: &[f64]) -> Result<ErrorQuantiles, DatasetError> {
    let percentile = |p: usize| {
        let rank = ordered
            .len()
            .checked_mul(p)
            .ok_or_else(|| DatasetError::Source("quantile rank overflow".into()))?
            .div_ceil(100);
        ordered
            .get(rank.saturating_sub(1))
            .copied()
            .ok_or_else(|| DatasetError::Source("empty quantile population".into()))
    };
    Ok(ErrorQuantiles {
        p50: percentile(50)?,
        p95: percentile(95)?,
        p99: percentile(99)?,
    })
}
/// Computes fixed metrics on signed residuals after identity/eligibility pairing.
/// Empty populations return None; nonfinite input is a contract failure.
pub(super) fn metrics(mut residuals: Vec<f64>) -> Result<Option<ErrorMetrics>, DatasetError> {
    if residuals.is_empty() {
        return Ok(None);
    }
    if residuals.iter().any(|r| !r.is_finite()) {
        return Err(DatasetError::Source("nonfinite residual".into()));
    }
    residuals.sort_by(f64::total_cmp);
    let mut absolute: Vec<_> = residuals.iter().map(|r| r.abs()).collect();
    absolute.sort_by(f64::total_cmp);
    let positive: Vec<_> = residuals.iter().copied().filter(|r| *r > 0.0).collect();
    let n = residuals.len();
    let upper = residuals[n / 2];
    let median = if n % 2 == 1 {
        upper
    } else {
        let lower = residuals[n / 2 - 1];
        if lower.signum() == upper.signum() {
            lower + (upper - lower) / 2.0
        } else {
            lower / 2.0 + upper / 2.0
        }
    };
    let scale = absolute[n - 1];
    let normalized_squares: Vec<_> = if scale == 0.0 {
        vec![0.0; n]
    } else {
        residuals.iter().map(|r| (r / scale).powi(2)).collect()
    };
    let result = ErrorMetrics {
        samples: n,
        mae_seconds: mean(&absolute)?,
        rmse_seconds: scale * mean(&normalized_squares)?.sqrt(),
        mean_signed_residual_seconds: mean(&residuals)?,
        median_signed_residual_seconds: median,
        absolute_error_quantiles_seconds: quantiles(&absolute)?,
        positive_residual_quantiles_seconds: if positive.is_empty() {
            None
        } else {
            Some(quantiles(&positive)?)
        },
        underprediction_count: positive.len(),
        underprediction_frequency: count(positive.len())? / count(n)?,
        underprediction_severity_seconds: if positive.is_empty() {
            None
        } else {
            Some(mean(&positive)?)
        },
    };
    if [
        result.mae_seconds,
        result.rmse_seconds,
        result.mean_signed_residual_seconds,
        result.median_signed_residual_seconds,
    ]
    .iter()
    .any(|v| !v.is_finite())
    {
        return Err(DatasetError::Source("metric arithmetic overflow".into()));
    }
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ok<T>(v: Result<T, DatasetError>) -> T {
        v.unwrap_or_else(|e| panic!("{e}"))
    }
    fn close(actual: f64, expected: f64) {
        assert!((actual - expected).abs() < 1e-12, "{actual} != {expected}");
    }
    fn exact(actual: f64, expected: f64) {
        // These fixtures deliberately require exact binary64 zero/extreme values.
        assert_eq!(actual.to_bits(), expected.to_bits());
    }
    #[test]
    fn independent_signed_error_fixture_and_quantile_definitions() {
        // Predictions [10, 10, 10, 0], observed [6, 10, 12, 8].
        let actual = ok(metrics(vec![-4.0, 0.0, 2.0, 8.0])).unwrap_or_else(|| panic!("metrics"));
        close(actual.mae_seconds, 3.5);
        close(actual.rmse_seconds, 21.0_f64.sqrt());
        close(actual.mean_signed_residual_seconds, 1.5);
        close(actual.median_signed_residual_seconds, 1.0);
        assert_eq!(
            actual.absolute_error_quantiles_seconds,
            ErrorQuantiles {
                p50: 2.0,
                p95: 8.0,
                p99: 8.0
            }
        );
        assert_eq!(
            actual.positive_residual_quantiles_seconds,
            Some(ErrorQuantiles {
                p50: 2.0,
                p95: 8.0,
                p99: 8.0
            })
        );
        close(actual.underprediction_frequency, 0.5);
        assert_eq!(actual.underprediction_severity_seconds, Some(5.0));
    }
    #[test]
    fn empty_zero_and_no_underprediction_are_distinct() {
        assert_eq!(ok(metrics(vec![])), None);
        let zero = ok(metrics(vec![0.0])).unwrap_or_else(|| panic!("metrics"));
        exact(zero.mae_seconds, 0.0);
        exact(zero.rmse_seconds, 0.0);
        exact(zero.underprediction_frequency, 0.0);
        assert_eq!(zero.positive_residual_quantiles_seconds, None);
        assert_eq!(zero.underprediction_severity_seconds, None);
        let over = ok(metrics(vec![-2.0, -4.0])).unwrap_or_else(|| panic!("metrics"));
        exact(over.mean_signed_residual_seconds, -3.0);
    }
    #[test]
    fn nonfinite_rejected_and_scaled_extremes_do_not_overflow() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(metrics(vec![value]).is_err());
        }
        let result = ok(metrics(vec![-f64::MAX, f64::MAX])).unwrap_or_else(|| panic!("metrics"));
        exact(result.mae_seconds, f64::MAX);
        exact(result.rmse_seconds, f64::MAX);
        exact(result.mean_signed_residual_seconds, 0.0);
        exact(result.median_signed_residual_seconds, 0.0);
        let tiny = ok(metrics(vec![f64::from_bits(1), f64::from_bits(1)]))
            .unwrap_or_else(|| panic!("metrics"));
        exact(tiny.mae_seconds, f64::from_bits(1));
    }
    #[test]
    fn order_invariance_and_nearest_rank_at_one_hundred_actions() {
        let values: Vec<_> = (1..=100).map(f64::from).collect();
        let mut reversed = values.clone();
        reversed.reverse();
        let result = ok(metrics(values));
        assert_eq!(result, ok(metrics(reversed)));
        assert_eq!(
            result
                .unwrap_or_else(|| panic!("metrics"))
                .absolute_error_quantiles_seconds,
            ErrorQuantiles {
                p50: 50.0,
                p95: 95.0,
                p99: 99.0
            }
        );
    }
    #[test]
    fn retained_hand_calculated_fixture_matches_metric_kernel() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/fixtures/phase-24/metric-oracles.json"
        )))
        .unwrap_or_else(|e| panic!("{e}"));
        for case in fixture["cases"]
            .as_array()
            .unwrap_or_else(|| panic!("cases"))
        {
            let predictions: Vec<f64> = serde_json::from_value(case["predictions_seconds"].clone())
                .unwrap_or_else(|e| panic!("{e}"));
            let labels: Vec<f64> = serde_json::from_value(case["observed_seconds"].clone())
                .unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(predictions.len(), labels.len());
            let actual = ok(metrics(
                labels
                    .into_iter()
                    .zip(predictions)
                    .map(|(label, prediction)| label - prediction)
                    .collect(),
            ))
            .unwrap_or_else(|| panic!("metrics"));
            let mut expected = case["expected"].clone();
            let mse = expected
                .as_object_mut()
                .unwrap_or_else(|| panic!("expected"))
                .remove("mean_squared_error_seconds_squared")
                .and_then(|v| v.as_f64())
                .unwrap_or_else(|| panic!("mse"));
            expected["rmse_seconds"] = serde_json::json!(mse.sqrt());
            expected["samples"] = serde_json::json!(actual.samples);
            let expected: ErrorMetrics =
                serde_json::from_value(expected).unwrap_or_else(|e| panic!("{e}"));
            close(actual.mae_seconds, expected.mae_seconds);
            close(actual.rmse_seconds, expected.rmse_seconds);
            close(
                actual.mean_signed_residual_seconds,
                expected.mean_signed_residual_seconds,
            );
            close(
                actual.median_signed_residual_seconds,
                expected.median_signed_residual_seconds,
            );
            assert_eq!(
                actual.absolute_error_quantiles_seconds,
                expected.absolute_error_quantiles_seconds
            );
            assert_eq!(
                actual.positive_residual_quantiles_seconds,
                expected.positive_residual_quantiles_seconds
            );
            assert_eq!(actual.underprediction_count, expected.underprediction_count);
            close(
                actual.underprediction_frequency,
                expected.underprediction_frequency,
            );
            assert_eq!(
                actual.underprediction_severity_seconds,
                expected.underprediction_severity_seconds
            );
        }
    }
}
