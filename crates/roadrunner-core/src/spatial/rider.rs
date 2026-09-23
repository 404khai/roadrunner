use std::collections::{BinaryHeap, HashSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::geo::{
    Coordinate, KilometersPerHour, MEAN_EARTH_RADIUS_METERS, Meters, Seconds, haversine_distance,
};

/// Stable identity of a rider within one lookup snapshot.
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

/// One available rider's position at snapshot construction time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RiderLocation {
    /// Rider identity.
    pub id: RiderId,
    /// Current WGS 84 location.
    pub coordinate: Coordinate,
}

/// A rider found within the requested straight-line radius.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RiderCandidate {
    /// Rider identity.
    pub rider_id: RiderId,
    /// Great-circle distance to the query location.
    pub distance: Meters,
    /// Straight-line distance divided by the configured speed.
    pub estimated_arrival: Seconds,
}

/// Invalid rider lookup snapshot configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum RiderLookupError {
    /// The same rider identity appears twice.
    #[error("duplicate rider ID {0}")]
    DuplicateRider(u64),
    /// A positive speed is required to estimate arrival time.
    #[error("rider lookup speed must be positive")]
    ZeroSpeed,
    /// A positive speed is too small for a finite global ETA.
    #[error("rider lookup speed is too small for finite arrival estimates")]
    SpeedTooLow,
}

/// Queries available riders by straight-line distance.
pub trait RiderLookup {
    /// Returns riders within `radius`, ordered by distance then rider ID.
    ///
    /// `limit == 0` returns an empty result. Boundary riders are included.
    fn nearest_riders(
        &self,
        location: Coordinate,
        radius: Meters,
        limit: usize,
    ) -> Vec<RiderCandidate>;

    /// Number of riders in the snapshot.
    fn rider_count(&self) -> usize;
}

/// Baseline lookup that evaluates every rider for each query.
#[derive(Debug, Clone)]
pub struct LinearRiderLocator {
    riders: Vec<RiderLocation>,
    speed: KilometersPerHour,
}

impl LinearRiderLocator {
    /// Builds a baseline snapshot from available riders.
    ///
    /// # Errors
    ///
    /// Rejects duplicate IDs and zero speed.
    pub fn new(
        riders: impl IntoIterator<Item = RiderLocation>,
        speed: KilometersPerHour,
    ) -> Result<Self, RiderLookupError> {
        let riders: Vec<_> = riders.into_iter().collect();
        validate(&riders, speed)?;
        Ok(Self { riders, speed })
    }
}

impl RiderLookup for LinearRiderLocator {
    fn nearest_riders(
        &self,
        location: Coordinate,
        radius: Meters,
        limit: usize,
    ) -> Vec<RiderCandidate> {
        if limit == 0 {
            return Vec::new();
        }
        let mut candidates = Vec::new();
        for rider in &self.riders {
            let distance = haversine_distance(location, rider.coordinate);
            if distance <= radius {
                candidates.push(candidate(*rider, distance, self.speed));
            }
        }
        order_and_limit(candidates, limit)
    }

    fn rider_count(&self) -> usize {
        self.riders.len()
    }
}

/// Immutable 3D median-split tree over unit-sphere rider positions.
///
/// Rebuild this snapshot when rider locations or availability change. The tree
/// only prunes possible matches; returned distances use the same Haversine
/// calculation as [`LinearRiderLocator`].
#[derive(Debug, Clone)]
pub struct IndexedRiderLocator {
    root: Option<Box<TreeNode>>,
    count: usize,
    speed: KilometersPerHour,
}

impl IndexedRiderLocator {
    /// Builds an immutable spatial snapshot from available riders.
    ///
    /// # Errors
    ///
    /// Rejects duplicate IDs and zero speed.
    pub fn new(
        riders: impl IntoIterator<Item = RiderLocation>,
        speed: KilometersPerHour,
    ) -> Result<Self, RiderLookupError> {
        let riders: Vec<_> = riders.into_iter().collect();
        validate(&riders, speed)?;
        let count = riders.len();
        let mut points: Vec<_> = riders
            .into_iter()
            .map(|rider| PositionedRider {
                position: unit_position(rider.coordinate),
                rider,
            })
            .collect();
        let root = TreeNode::build(&mut points, 0);
        Ok(Self { root, count, speed })
    }
}

impl RiderLookup for IndexedRiderLocator {
    fn nearest_riders(
        &self,
        location: Coordinate,
        radius: Meters,
        limit: usize,
    ) -> Vec<RiderCandidate> {
        if limit == 0 {
            return Vec::new();
        }
        let Some(root) = &self.root else {
            return Vec::new();
        };
        let position = unit_position(location);
        let mut nearest = BinaryHeap::with_capacity(limit.min(self.count));
        root.collect(position, location, radius, self.speed, limit, &mut nearest);
        nearest
            .into_sorted_vec()
            .into_iter()
            .map(|ranked: RankedCandidate| ranked.0)
            .collect()
    }

    fn rider_count(&self) -> usize {
        self.count
    }
}

fn validate(riders: &[RiderLocation], speed: KilometersPerHour) -> Result<(), RiderLookupError> {
    if speed.value() == 0.0 {
        return Err(RiderLookupError::ZeroSpeed);
    }
    if !((std::f64::consts::PI * MEAN_EARTH_RADIUS_METERS * 3.6) / speed.value()).is_finite() {
        return Err(RiderLookupError::SpeedTooLow);
    }
    let mut seen = HashSet::with_capacity(riders.len());
    for rider in riders {
        if !seen.insert(rider.id) {
            return Err(RiderLookupError::DuplicateRider(rider.id.value()));
        }
    }
    Ok(())
}

fn candidate(rider: RiderLocation, distance: Meters, speed: KilometersPerHour) -> RiderCandidate {
    RiderCandidate {
        rider_id: rider.id,
        distance,
        estimated_arrival: Seconds::from_calculation(distance.value() * 3.6 / speed.value()),
    }
}

fn order_and_limit(mut candidates: Vec<RiderCandidate>, limit: usize) -> Vec<RiderCandidate> {
    candidates.sort_unstable_by(|a, b| {
        a.distance
            .value()
            .total_cmp(&b.distance.value())
            .then_with(|| a.rider_id.cmp(&b.rider_id))
    });
    candidates.truncate(limit);
    candidates
}

#[derive(Debug, Clone, Copy)]
struct PositionedRider {
    position: [f64; 3],
    rider: RiderLocation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct RankedCandidate(RiderCandidate);

impl Eq for RankedCandidate {}

impl PartialOrd for RankedCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RankedCandidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .distance
            .value()
            .total_cmp(&other.0.distance.value())
            .then_with(|| self.0.rider_id.cmp(&other.0.rider_id))
    }
}

#[derive(Debug, Clone)]
struct TreeNode {
    point: PositionedRider,
    min: [f64; 3],
    max: [f64; 3],
    left: Option<Box<Self>>,
    right: Option<Box<Self>>,
}

impl TreeNode {
    fn build(points: &mut [PositionedRider], depth: usize) -> Option<Box<Self>> {
        if points.is_empty() {
            return None;
        }
        let axis = depth % 3;
        let middle = points.len() / 2;
        points.select_nth_unstable_by(middle, |a, b| {
            a.position[axis]
                .total_cmp(&b.position[axis])
                .then_with(|| a.rider.id.cmp(&b.rider.id))
        });
        let (left_points, pivot_and_right) = points.split_at_mut(middle);
        let (pivot, right_points) = pivot_and_right.split_first_mut()?;
        let left = Self::build(left_points, depth + 1);
        let right = Self::build(right_points, depth + 1);
        let mut node = Self {
            point: *pivot,
            min: pivot.position,
            max: pivot.position,
            left,
            right,
        };
        for child in [node.left.as_deref(), node.right.as_deref()]
            .into_iter()
            .flatten()
        {
            for axis in 0..3 {
                node.min[axis] = node.min[axis].min(child.min[axis]);
                node.max[axis] = node.max[axis].max(child.max[axis]);
            }
        }
        Some(Box::new(node))
    }

    fn collect(
        &self,
        position: [f64; 3],
        location: Coordinate,
        radius: Meters,
        speed: KilometersPerHour,
        limit: usize,
        nearest: &mut BinaryHeap<RankedCandidate>,
    ) {
        // A small margin protects boundary results from trigonometric rounding.
        let threshold = if nearest.len() == limit {
            nearest.peek().map_or(radius.value(), |worst| {
                worst.0.distance.value().min(radius.value())
            })
        } else {
            radius.value()
        };
        if self.box_distance_squared(position)
            > (chord_for_radius(Meters::from_calculation(threshold)) + 1.0e-12).powi(2)
        {
            return;
        }
        let distance = haversine_distance(location, self.point.rider.coordinate);
        if distance <= radius {
            let found = RankedCandidate(candidate(self.point.rider, distance, speed));
            if nearest.len() < limit {
                nearest.push(found);
            } else if nearest.peek().is_some_and(|worst| found < *worst) {
                nearest.pop();
                nearest.push(found);
            }
        }
        let mut children = [self.left.as_deref(), self.right.as_deref()];
        children.sort_by(|a, b| {
            a.map_or(f64::INFINITY, |child| child.box_distance_squared(position))
                .total_cmp(&b.map_or(f64::INFINITY, |child| child.box_distance_squared(position)))
        });
        for child in children.into_iter().flatten() {
            child.collect(position, location, radius, speed, limit, nearest);
        }
    }

    fn box_distance_squared(&self, position: [f64; 3]) -> f64 {
        (0..3)
            .map(|axis| {
                let delta = if position[axis] < self.min[axis] {
                    self.min[axis] - position[axis]
                } else if position[axis] > self.max[axis] {
                    position[axis] - self.max[axis]
                } else {
                    0.0
                };
                delta * delta
            })
            .sum()
    }
}

fn unit_position(coordinate: Coordinate) -> [f64; 3] {
    let latitude = coordinate.latitude().to_radians();
    let longitude = coordinate.longitude().to_radians();
    let cos_latitude = latitude.cos();
    [
        cos_latitude * longitude.cos(),
        cos_latitude * longitude.sin(),
        latitude.sin(),
    ]
}

fn chord_for_radius(radius: Meters) -> f64 {
    // Beyond half the Earth's circumference every position is eligible.
    let angle = (radius.value() / MEAN_EARTH_RADIUS_METERS).min(std::f64::consts::PI);
    2.0 * (angle / 2.0).sin()
}
