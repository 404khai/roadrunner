use serde::{Deserialize, Serialize};

/// Stable identity of a rider across dispatch records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RiderId(u64);

impl RiderId {
    /// Creates a rider identity.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying identity.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}
