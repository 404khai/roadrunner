use thiserror::Error;

use super::CostKind;

/// Errors produced while constructing or combining route costs.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum CostError {
    /// A cost is NaN or infinite.
    #[error("{kind} cost must be finite, got {value}")]
    NotFinite {
        /// The semantic kind of cost.
        kind: CostKind,
        /// The rejected numeric value.
        value: f64,
    },

    /// A cost is less than zero.
    #[error("{kind} cost must be non-negative, got {value}")]
    Negative {
        /// The semantic kind of cost.
        kind: CostKind,
        /// The rejected numeric value.
        value: f64,
    },

    /// An operation attempted to mix costs with different units.
    #[error("cannot combine {left} cost with {right} cost")]
    MismatchedKinds {
        /// The left-hand cost kind.
        left: CostKind,
        /// The right-hand cost kind.
        right: CostKind,
    },
}
