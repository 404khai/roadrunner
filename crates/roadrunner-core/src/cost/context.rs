use serde::{Deserialize, Serialize};

/// Immutable information available while evaluating an edge cost.
///
/// Phase 4 cost models are static, so the context is intentionally empty. Later
/// phases can add departure time and traffic state without coupling them to graph
/// topology.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RoutingContext {}

impl RoutingContext {
    /// Creates the deterministic Phase 4 routing context.
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }
}
