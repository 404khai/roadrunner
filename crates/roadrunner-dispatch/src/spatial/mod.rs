//! Exact-radius rider lookup with a linear baseline and an immutable spatial tree.

mod rider;

pub use rider::{
    IndexedRiderLocator, LinearRiderLocator, RiderCandidate, RiderLocation, RiderLookup,
    RiderLookupError,
};

// Compatibility re-export; the canonical identity is defined in dispatch domain.
pub use crate::RiderId;
