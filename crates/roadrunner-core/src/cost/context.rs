use serde::{Deserialize, Serialize};

use crate::geo::Seconds;

/// Immutable request information available during traversal evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RoutingContext {
    departure_time: Seconds,
    private_access: bool,
    permit_access: bool,
}

impl RoutingContext {
    /// Creates a context departing at logical time zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            departure_time: Seconds::ZERO,
            private_access: false,
            permit_access: false,
        }
    }

    /// Creates a context with an explicit logical departure time.
    #[must_use]
    pub const fn with_departure_time(departure_time: Seconds) -> Self {
        Self {
            departure_time,
            private_access: false,
            permit_access: false,
        }
    }

    /// Returns the request's logical departure time.
    #[must_use]
    pub const fn departure_time(self) -> Seconds {
        self.departure_time
    }

    /// Adds explicit private-road authorization to this request.
    #[must_use]
    pub const fn with_private_access(mut self) -> Self {
        self.private_access = true;
        self
    }

    /// Adds explicit permit-controlled-road authorization to this request.
    #[must_use]
    pub const fn with_permit_access(mut self) -> Self {
        self.permit_access = true;
        self
    }

    /// Returns whether the request has private-road authorization.
    #[must_use]
    pub const fn has_private_access(self) -> bool {
        self.private_access
    }

    /// Returns whether the request has permit-controlled-road authorization.
    #[must_use]
    pub const fn has_permit_access(self) -> bool {
        self.permit_access
    }
}

impl Default for RoutingContext {
    fn default() -> Self {
        Self::new()
    }
}
