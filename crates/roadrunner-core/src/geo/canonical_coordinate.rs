use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::Coordinate;

/// Number of canonical coordinate units in one degree.
pub const E7_SCALE: i32 = 10_000_000;

/// A canonical fixed-point WGS 84 coordinate at 10^-7 degree precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CanonicalCoordinate {
    latitude_e7: i32,
    longitude_e7: i32,
}

impl CanonicalCoordinate {
    /// Creates a canonical coordinate from already-scaled components.
    ///
    /// # Errors
    ///
    /// Returns an error when either component is outside the WGS 84 range.
    pub fn new(latitude_e7: i32, longitude_e7: i32) -> Result<Self, CanonicalCoordinateError> {
        let latitude_limit = 90 * E7_SCALE;
        let longitude_limit = 180 * E7_SCALE;
        if !(-latitude_limit..=latitude_limit).contains(&latitude_e7) {
            return Err(CanonicalCoordinateError::LatitudeOutOfRange { latitude_e7 });
        }
        if !(-longitude_limit..=longitude_limit).contains(&longitude_e7) {
            return Err(CanonicalCoordinateError::LongitudeOutOfRange { longitude_e7 });
        }
        Ok(Self {
            latitude_e7,
            longitude_e7,
        })
    }

    /// Canonicalizes a validated floating-point coordinate using `f64::round`.
    ///
    /// # Errors
    ///
    /// Returns an error if checked conversion to the E7 representation fails.
    pub fn from_coordinate(coordinate: Coordinate) -> Result<Self, CanonicalCoordinateError> {
        #[allow(clippy::cast_possible_truncation)]
        fn scaled(value: f64) -> Result<i32, CanonicalCoordinateError> {
            let scaled = (value * f64::from(E7_SCALE)).round();
            if scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
                return Err(CanonicalCoordinateError::ConversionOverflow { value });
            }
            // The explicit bounds check above proves this conversion fits.
            Ok(scaled as i32)
        }

        Self::new(
            scaled(coordinate.latitude())?,
            scaled(coordinate.longitude())?,
        )
    }

    /// Returns latitude in canonical E7 units.
    #[must_use]
    pub const fn latitude_e7(self) -> i32 {
        self.latitude_e7
    }

    /// Returns longitude in canonical E7 units.
    #[must_use]
    pub const fn longitude_e7(self) -> i32 {
        self.longitude_e7
    }

    /// Converts the canonical value to decimal degrees.
    #[must_use]
    pub fn to_coordinate(self) -> Coordinate {
        Coordinate::from_validated(
            f64::from(self.latitude_e7) / f64::from(E7_SCALE),
            f64::from(self.longitude_e7) / f64::from(E7_SCALE),
        )
    }
}

/// Errors produced while constructing canonical coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum CanonicalCoordinateError {
    /// Latitude is outside the E7 WGS 84 range.
    #[error("E7 latitude {latitude_e7} is outside the WGS 84 range")]
    LatitudeOutOfRange {
        /// Rejected value.
        latitude_e7: i32,
    },
    /// Longitude is outside the E7 WGS 84 range.
    #[error("E7 longitude {longitude_e7} is outside the WGS 84 range")]
    LongitudeOutOfRange {
        /// Rejected value.
        longitude_e7: i32,
    },
    /// Floating-point conversion cannot fit the canonical integer representation.
    #[error("coordinate component {value} cannot be represented as E7")]
    ConversionOverflow {
        /// Rejected decimal-degree component.
        value: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_and_round_trips_coordinates() {
        let Ok(coordinate) = Coordinate::new(6.5244, 3.3792) else {
            panic!("valid coordinate");
        };
        let Ok(canonical) = CanonicalCoordinate::from_coordinate(coordinate) else {
            panic!("coordinate should fit E7");
        };

        assert_eq!(canonical.latitude_e7(), 65_244_000);
        assert_eq!(canonical.longitude_e7(), 33_792_000);
        assert_eq!(canonical.to_coordinate(), coordinate);
    }

    #[test]
    fn rejects_out_of_range_components() {
        assert!(CanonicalCoordinate::new(900_000_001, 0).is_err());
        assert!(CanonicalCoordinate::new(0, -1_800_000_001).is_err());
    }
}
