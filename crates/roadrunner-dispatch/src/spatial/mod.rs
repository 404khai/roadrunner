//! Exact-radius rider lookup and read-only H3 dispatch aggregation.

mod h3;
mod rider;

pub use h3::{H3Cell, H3CellContents, H3DispatchProjection, H3Error, H3Resolution};
pub use rider::{
    IndexedRiderLocator, LinearRiderLocator, RiderCandidate, RiderLocation, RiderLookup,
    RiderLookupError,
};

// Compatibility re-export; the canonical identity is defined in dispatch domain.
pub use crate::RiderId;
