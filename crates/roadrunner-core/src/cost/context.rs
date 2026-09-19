use serde::{Deserialize, Serialize};

use crate::geo::Seconds;

/// Immutable request information available during traversal evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RoutingContext {
    departure_time: Seconds,
}

impl RoutingContext {
    /// Creates a context departing at logical time zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            departure_time: Seconds::ZERO,
        }
    }

    /// Creates a context with an explicit logical departure time.
    #[must_use]
    pub const fn with_departure_time(departure_time: Seconds) -> Self {
        Self { departure_time }
    }

    /// Returns the request's logical departure time.
    #[must_use]
    pub const fn departure_time(self) -> Seconds {
        self.departure_time
    }
}

impl Default for RoutingContext {
    fn default() -> Self {
        Self::new()
    }
}
