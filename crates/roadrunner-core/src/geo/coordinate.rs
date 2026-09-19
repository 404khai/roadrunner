use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A validated WGS 84 latitude and longitude pair in decimal degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CoordinateRepr", into = "CoordinateRepr")]
pub struct Coordinate {
    latitude: f64,
    longitude: f64,
}

impl Coordinate {
    /// The intersection of the equator and prime meridian.
    pub const ORIGIN: Self = Self {
        latitude: 0.0,
        longitude: 0.0,
    };

    /// Creates a validated coordinate in decimal degrees.
    ///
    /// # Errors
    ///
    /// Returns [`CoordinateError`] when either component is not finite, latitude
    /// falls outside `[-90, 90]`, or longitude falls outside `[-180, 180]`.
    pub fn new(latitude: f64, longitude: f64) -> Result<Self, CoordinateError> {
        if !latitude.is_finite() {
            return Err(CoordinateError::LatitudeNotFinite { latitude });
        }
        if !longitude.is_finite() {
            return Err(CoordinateError::LongitudeNotFinite { longitude });
        }
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(CoordinateError::LatitudeOutOfRange { latitude });
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(CoordinateError::LongitudeOutOfRange { longitude });
        }

        Ok(Self {
            latitude,
            longitude,
        })
    }

    pub(crate) const fn from_validated(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    /// Returns latitude in decimal degrees.
    #[must_use]
    pub const fn latitude(self) -> f64 {
        self.latitude
    }

    /// Returns longitude in decimal degrees.
    #[must_use]
    pub const fn longitude(self) -> f64 {
        self.longitude
    }
}

#[derive(Serialize, Deserialize)]
struct CoordinateRepr {
    latitude: f64,
    longitude: f64,
}

impl TryFrom<CoordinateRepr> for Coordinate {
    type Error = CoordinateError;

    fn try_from(value: CoordinateRepr) -> Result<Self, Self::Error> {
        Self::new(value.latitude, value.longitude)
    }
}

impl From<Coordinate> for CoordinateRepr {
    fn from(value: Coordinate) -> Self {
        Self {
            latitude: value.latitude(),
            longitude: value.longitude(),
        }
    }
}

/// Errors produced when constructing a coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum CoordinateError {
    /// Latitude is NaN or infinite.
    #[error("latitude must be finite, got {latitude}")]
    LatitudeNotFinite {
        /// The rejected latitude.
        latitude: f64,
    },

    /// Longitude is NaN or infinite.
    #[error("longitude must be finite, got {longitude}")]
    LongitudeNotFinite {
        /// The rejected longitude.
        longitude: f64,
    },

    /// Latitude is outside the inclusive WGS 84 range.
    #[error("latitude must be between -90 and 90 degrees, got {latitude}")]
    LatitudeOutOfRange {
        /// The rejected latitude.
        latitude: f64,
    },

    /// Longitude is outside the inclusive WGS 84 range.
    #[error("longitude must be between -180 and 180 degrees, got {longitude}")]
    LongitudeOutOfRange {
        /// The rejected longitude.
        longitude: f64,
    },
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn accepts_boundary_coordinates() {
        assert!(Coordinate::new(-90.0, -180.0).is_ok());
        assert!(Coordinate::new(90.0, 180.0).is_ok());
    }

    #[test]
    fn rejects_out_of_range_components() {
        assert_eq!(
            Coordinate::new(90.1, 0.0),
            Err(CoordinateError::LatitudeOutOfRange { latitude: 90.1 })
        );
        assert_eq!(
            Coordinate::new(0.0, -180.1),
            Err(CoordinateError::LongitudeOutOfRange { longitude: -180.1 })
        );
    }

    #[test]
    fn rejects_non_finite_components() {
        assert!(matches!(
            Coordinate::new(f64::NAN, 0.0),
            Err(CoordinateError::LatitudeNotFinite { .. })
        ));
        assert!(matches!(
            Coordinate::new(0.0, f64::INFINITY),
            Err(CoordinateError::LongitudeNotFinite { .. })
        ));
    }

    #[test]
    fn serde_round_trip_preserves_fields_and_validation() {
        let coordinate = Coordinate::new(6.5244, 3.3792);
        let Ok(coordinate) = coordinate else {
            panic!("expected the Lagos coordinate to be valid");
        };

        assert!(matches!(
            serde_json::to_value(coordinate),
            Ok(value) if value == json!({ "latitude": 6.5244, "longitude": 3.3792 })
        ));
        assert!(serde_json::from_str::<Coordinate>(r#"{"latitude":91,"longitude":0}"#).is_err());
    }
}
