use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::Coordinate;

/// An inclusive, axis-aligned geographic bounding box.
///
/// Antimeridian-crossing boxes are deferred. Consequently, the western longitude
/// must be less than or equal to the eastern longitude.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "BoundingBoxRepr", into = "BoundingBoxRepr")]
pub struct BoundingBox {
    south_west: Coordinate,
    north_east: Coordinate,
}

impl BoundingBox {
    /// Creates a box from its south-west and north-east corners.
    ///
    /// # Errors
    ///
    /// Returns [`BoundingBoxError`] when latitude or longitude bounds are inverted.
    pub fn new(south_west: Coordinate, north_east: Coordinate) -> Result<Self, BoundingBoxError> {
        if south_west.latitude() > north_east.latitude() {
            return Err(BoundingBoxError::InvertedLatitude {
                south: south_west.latitude(),
                north: north_east.latitude(),
            });
        }
        if south_west.longitude() > north_east.longitude() {
            return Err(BoundingBoxError::InvertedLongitude {
                west: south_west.longitude(),
                east: north_east.longitude(),
            });
        }

        Ok(Self {
            south_west,
            north_east,
        })
    }

    /// Returns the south-west corner.
    #[must_use]
    pub const fn south_west(self) -> Coordinate {
        self.south_west
    }

    /// Returns the north-east corner.
    #[must_use]
    pub const fn north_east(self) -> Coordinate {
        self.north_east
    }

    /// Returns whether a coordinate lies inside or on the box boundary.
    #[must_use]
    pub fn contains(self, coordinate: Coordinate) -> bool {
        (self.south_west.latitude()..=self.north_east.latitude()).contains(&coordinate.latitude())
            && (self.south_west.longitude()..=self.north_east.longitude())
                .contains(&coordinate.longitude())
    }
}

#[derive(Serialize, Deserialize)]
struct BoundingBoxRepr {
    south_west: Coordinate,
    north_east: Coordinate,
}

impl TryFrom<BoundingBoxRepr> for BoundingBox {
    type Error = BoundingBoxError;

    fn try_from(value: BoundingBoxRepr) -> Result<Self, Self::Error> {
        Self::new(value.south_west, value.north_east)
    }
}

impl From<BoundingBox> for BoundingBoxRepr {
    fn from(value: BoundingBox) -> Self {
        Self {
            south_west: value.south_west(),
            north_east: value.north_east(),
        }
    }
}

/// Errors produced when constructing a bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum BoundingBoxError {
    /// The southern latitude is north of the northern latitude.
    #[error("south latitude {south} exceeds north latitude {north}")]
    InvertedLatitude {
        /// The rejected southern latitude.
        south: f64,
        /// The rejected northern latitude.
        north: f64,
    },

    /// The western longitude is east of the eastern longitude.
    #[error("west longitude {west} exceeds east longitude {east}")]
    InvertedLongitude {
        /// The rejected western longitude.
        west: f64,
        /// The rejected eastern longitude.
        east: f64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coordinate(latitude: f64, longitude: f64) -> Coordinate {
        let result = Coordinate::new(latitude, longitude);
        let Ok(coordinate) = result else {
            panic!("expected test coordinate to be valid: {result:?}");
        };
        coordinate
    }

    #[test]
    fn contains_interior_and_boundary_coordinates() {
        let bounds = BoundingBox::new(coordinate(6.0, 3.0), coordinate(7.0, 4.0));
        let Ok(bounds) = bounds else {
            panic!("expected test bounds to be valid");
        };

        assert!(bounds.contains(coordinate(6.5, 3.5)));
        assert!(bounds.contains(coordinate(6.0, 3.0)));
        assert!(bounds.contains(coordinate(7.0, 4.0)));
        assert!(!bounds.contains(coordinate(7.1, 3.5)));
    }

    #[test]
    fn rejects_inverted_latitude_bounds() {
        assert_eq!(
            BoundingBox::new(coordinate(7.0, 3.0), coordinate(6.0, 4.0)),
            Err(BoundingBoxError::InvertedLatitude {
                south: 7.0,
                north: 6.0,
            })
        );
    }

    #[test]
    fn rejects_inverted_longitudes_and_antimeridian_crossing_boxes() {
        assert_eq!(
            BoundingBox::new(coordinate(-10.0, 170.0), coordinate(10.0, -170.0)),
            Err(BoundingBoxError::InvertedLongitude {
                west: 170.0,
                east: -170.0,
            })
        );
    }

    #[test]
    fn deserialization_preserves_box_invariants() {
        let serialized = r#"{
            "south_west":{"latitude":7,"longitude":3},
            "north_east":{"latitude":6,"longitude":4}
        }"#;

        assert!(serde_json::from_str::<BoundingBox>(serialized).is_err());
    }
}
