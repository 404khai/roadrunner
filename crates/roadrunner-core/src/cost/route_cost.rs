use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::geo::{Meters, Seconds};

use super::CostError;

/// The semantic unit of a route cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostKind {
    /// Distance measured in meters.
    Distance,
    /// Travel time measured in seconds.
    TravelTime,
}

impl fmt::Display for CostKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Distance => formatter.write_str("distance"),
            Self::TravelTime => formatter.write_str("travel-time"),
        }
    }
}

/// A validated, non-negative scalar tagged with its semantic cost kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RouteCostRepr", into = "RouteCostRepr")]
pub struct RouteCost {
    kind: CostKind,
    value: f64,
}

impl RouteCost {
    /// Creates a validated route cost.
    ///
    /// # Errors
    ///
    /// Returns [`CostError`] when `value` is negative or not finite.
    pub fn new(kind: CostKind, value: f64) -> Result<Self, CostError> {
        if !value.is_finite() {
            return Err(CostError::NotFinite { kind, value });
        }
        if value < 0.0 {
            return Err(CostError::Negative { kind, value });
        }
        Ok(Self {
            kind,
            value: if value == 0.0 { 0.0 } else { value },
        })
    }

    /// Creates a zero cost of the selected kind.
    #[must_use]
    pub const fn zero(kind: CostKind) -> Self {
        Self { kind, value: 0.0 }
    }

    /// Creates a distance cost from a validated meter value.
    #[must_use]
    pub const fn from_distance(distance: Meters) -> Self {
        Self {
            kind: CostKind::Distance,
            value: distance.value(),
        }
    }

    /// Creates a travel-time cost from a validated second value.
    #[must_use]
    pub const fn from_travel_time(travel_time: Seconds) -> Self {
        Self {
            kind: CostKind::TravelTime,
            value: travel_time.value(),
        }
    }

    /// Returns the cost kind.
    #[must_use]
    pub const fn kind(self) -> CostKind {
        self.kind
    }

    /// Returns the scalar value in the unit identified by [`Self::kind`].
    #[must_use]
    pub const fn value(self) -> f64 {
        self.value
    }

    /// Adds another cost with the same semantic kind.
    ///
    /// # Errors
    ///
    /// Returns [`CostError::MismatchedKinds`] when the units differ, or
    /// [`CostError::NotFinite`] when finite inputs overflow during addition.
    pub fn checked_add(self, other: Self) -> Result<Self, CostError> {
        if self.kind != other.kind {
            return Err(CostError::MismatchedKinds {
                left: self.kind,
                right: other.kind,
            });
        }
        Self::new(self.kind, self.value + other.value)
    }
}

impl PartialOrd for RouteCost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.kind == other.kind).then(|| self.value.total_cmp(&other.value))
    }
}

#[derive(Serialize, Deserialize)]
struct RouteCostRepr {
    kind: CostKind,
    value: f64,
}

impl TryFrom<RouteCostRepr> for RouteCost {
    type Error = CostError;

    fn try_from(value: RouteCostRepr) -> Result<Self, Self::Error> {
        Self::new(value.kind, value.value)
    }
}

impl From<RouteCost> for RouteCostRepr {
    fn from(value: RouteCost) -> Self {
        Self {
            kind: value.kind(),
            value: value.value(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_negative_and_non_finite_costs() {
        assert_eq!(
            RouteCost::new(CostKind::Distance, -1.0),
            Err(CostError::Negative {
                kind: CostKind::Distance,
                value: -1.0,
            })
        );
        assert!(matches!(
            RouteCost::new(CostKind::TravelTime, f64::NAN),
            Err(CostError::NotFinite {
                kind: CostKind::TravelTime,
                ..
            })
        ));
        assert!(RouteCost::new(CostKind::TravelTime, f64::INFINITY).is_err());
        let negative_zero = RouteCost::new(CostKind::Distance, -0.0);
        assert_eq!(negative_zero.map(RouteCost::value), Ok(0.0));
        assert!(negative_zero.is_ok_and(|value| value.value().is_sign_positive()));
    }

    #[test]
    fn adds_costs_of_the_same_kind() {
        let first = RouteCost::new(CostKind::Distance, 10.0);
        let second = RouteCost::new(CostKind::Distance, 2.5);
        let (Ok(first), Ok(second)) = (first, second) else {
            panic!("expected valid test costs");
        };

        assert_eq!(
            first.checked_add(second),
            RouteCost::new(CostKind::Distance, 12.5)
        );
    }

    #[test]
    fn rejects_mixed_kind_arithmetic_and_comparison() {
        let distance = RouteCost::zero(CostKind::Distance);
        let travel_time = RouteCost::zero(CostKind::TravelTime);

        assert_eq!(
            distance.checked_add(travel_time),
            Err(CostError::MismatchedKinds {
                left: CostKind::Distance,
                right: CostKind::TravelTime,
            })
        );
        assert_eq!(distance.partial_cmp(&travel_time), None);
    }

    #[test]
    fn deserialization_preserves_validation() {
        let serialized = r#"{"kind":"distance","value":-1}"#;

        assert!(serde_json::from_str::<RouteCost>(serialized).is_err());
    }
}
