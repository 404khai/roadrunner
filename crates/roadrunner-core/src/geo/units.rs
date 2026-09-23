use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Identifies a physical unit when reporting validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementUnit {
    /// Distance measured in meters.
    Meters,
    /// Duration measured in seconds.
    Seconds,
    /// Speed measured in kilometers per hour.
    KilometersPerHour,
}

impl fmt::Display for MeasurementUnit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Meters => formatter.write_str("meters"),
            Self::Seconds => formatter.write_str("seconds"),
            Self::KilometersPerHour => formatter.write_str("kilometers per hour"),
        }
    }
}

/// Errors produced when constructing a physical unit value.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum UnitError {
    /// A unit value is NaN or infinite.
    #[error("{unit} must be finite, got {value}")]
    NotFinite {
        /// The unit being constructed.
        unit: MeasurementUnit,
        /// The rejected numeric value.
        value: f64,
    },

    /// A unit value is less than zero.
    #[error("{unit} must be non-negative, got {value}")]
    Negative {
        /// The unit being constructed.
        unit: MeasurementUnit,
        /// The rejected numeric value.
        value: f64,
    },
}

fn validate(value: f64, unit: MeasurementUnit) -> Result<f64, UnitError> {
    if !value.is_finite() {
        return Err(UnitError::NotFinite { unit, value });
    }
    if value < 0.0 {
        return Err(UnitError::Negative { unit, value });
    }
    Ok(if value == 0.0 { 0.0 } else { value })
}

macro_rules! non_negative_unit {
    ($(#[$meta:meta])* $name:ident, $unit:expr, $suffix:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(try_from = "f64", into = "f64")]
        #[repr(transparent)]
        pub struct $name(f64);

        impl $name {
            /// A zero-valued measurement.
            pub const ZERO: Self = Self(0.0);

            /// Creates a validated measurement.
            ///
            /// # Errors
            ///
            /// Returns [`UnitError`] when `value` is negative or not finite.
            pub fn new(value: f64) -> Result<Self, UnitError> {
                validate(value, $unit).map(Self)
            }

            /// Returns the numeric value in this type's named unit.
            #[must_use]
            pub const fn value(self) -> f64 {
                self.0
            }

            /// Adds another measurement of the same unit and validates the result.
            ///
            /// # Errors
            ///
            /// Returns [`UnitError`] when finite inputs overflow during addition.
            pub fn checked_add(self, other: Self) -> Result<Self, UnitError> {
                Self::new(self.0 + other.0)
            }
        }

        impl TryFrom<f64> for $name {
            type Error = UnitError;

            fn try_from(value: f64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for f64 {
            fn from(value: $name) -> Self {
                value.value()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{} {}", self.0, $suffix)
            }
        }
    };
}

non_negative_unit!(
    /// A non-negative, finite distance measured in meters.
    Meters,
    MeasurementUnit::Meters,
    "m"
);

non_negative_unit!(
    /// A non-negative, finite duration measured in seconds.
    Seconds,
    MeasurementUnit::Seconds,
    "s"
);

non_negative_unit!(
    /// A non-negative, finite speed measured in kilometers per hour.
    KilometersPerHour,
    MeasurementUnit::KilometersPerHour,
    "km/h"
);

/// Roadrunner's canonical distance type.
pub type Distance = Meters;

impl Meters {
    pub(crate) fn from_calculation(value: f64) -> Self {
        debug_assert!(value.is_finite());
        debug_assert!(value >= 0.0);
        Self(value)
    }
}

impl Seconds {
    pub(crate) fn from_calculation(value: f64) -> Self {
        debug_assert!(value.is_finite());
        debug_assert!(value >= 0.0);
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn accepts_zero_and_positive_measurements() {
        assert_eq!(Meters::new(0.0), Ok(Meters::ZERO));
        assert_eq!(Seconds::new(12.5).map(Seconds::value), Ok(12.5));
        assert_eq!(
            KilometersPerHour::new(50.0).map(KilometersPerHour::value),
            Ok(50.0)
        );
    }

    #[test]
    fn rejects_negative_measurements() {
        assert_eq!(
            Meters::new(-1.0),
            Err(UnitError::Negative {
                unit: MeasurementUnit::Meters,
                value: -1.0,
            })
        );
    }

    #[test]
    fn rejects_non_finite_measurements() {
        assert!(matches!(
            Seconds::new(f64::INFINITY),
            Err(UnitError::NotFinite {
                unit: MeasurementUnit::Seconds,
                ..
            })
        ));
        assert!(matches!(
            KilometersPerHour::new(f64::NAN),
            Err(UnitError::NotFinite {
                unit: MeasurementUnit::KilometersPerHour,
                ..
            })
        ));
    }

    #[test]
    fn serializes_as_a_number_and_validates_deserialization() {
        let meters = Meters::new(42.5);
        let Ok(meters) = meters else {
            panic!("expected a valid meter value");
        };

        assert!(matches!(
            serde_json::to_value(meters),
            Ok(value) if value == json!(42.5)
        ));
        assert!(serde_json::from_str::<Meters>("-1").is_err());
    }

    #[test]
    fn checked_add_revalidates_the_result() {
        let largest = Meters::new(f64::MAX);
        let Ok(largest) = largest else {
            panic!("expected the largest finite meter value to be valid");
        };

        assert_eq!(Meters::new(-0.0), Ok(Meters::ZERO));
        assert!(matches!(
            largest.checked_add(largest),
            Err(UnitError::NotFinite {
                unit: MeasurementUnit::Meters,
                ..
            })
        ));
    }
}
