//! Exact-radius rider lookup with a linear baseline and an immutable spatial tree.

mod rider;

pub use rider::{
    IndexedRiderLocator, LinearRiderLocator, RiderCandidate, RiderId, RiderLocation, RiderLookup,
    RiderLookupError,
};
