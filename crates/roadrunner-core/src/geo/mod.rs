//! Validated geographic values, physical units, and distance calculations.

mod bounding_box;
mod canonical_coordinate;
mod coordinate;
mod haversine;
mod units;

pub use bounding_box::{BoundingBox, BoundingBoxError};
pub use canonical_coordinate::{CanonicalCoordinate, CanonicalCoordinateError, E7_SCALE};
pub use coordinate::{Coordinate, CoordinateError};
pub use haversine::haversine_distance;
pub use units::{Distance, KilometersPerHour, MeasurementUnit, Meters, Seconds, UnitError};
