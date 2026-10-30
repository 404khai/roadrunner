use std::collections::BTreeMap;
use std::fmt;

use h3o::{CellIndex, LatLng, Resolution};
use roadrunner_core::geo::Coordinate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    DispatchEvaluationError, DispatchInstant, DispatchSnapshot, FulfillmentState, OrderId, RiderId,
    WorldVersion, basic_dispatch_eligible_snapshot,
};

/// Invalid H3 configuration or hierarchy operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum H3Error {
    /// H3 supports integer resolutions from zero through fifteen.
    #[error("H3 resolution must be in 0..=15")]
    InvalidResolution,
    /// A cell ID must identify a valid H3 cell, rather than an edge or vertex.
    #[error("invalid H3 cell ID")]
    InvalidCell,
    /// Aggregation cannot recover a finer resolution from coarser cells.
    #[error("H3 parent resolution must not exceed the source resolution")]
    FinerResolution,
}

/// Validated H3 resolution; serialized as an integer in `0..=15`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct H3Resolution(Resolution);

impl TryFrom<u8> for H3Resolution {
    type Error = H3Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Resolution::try_from(value)
            .map(Self)
            .map_err(|_| H3Error::InvalidResolution)
    }
}

impl From<H3Resolution> for u8 {
    fn from(value: H3Resolution) -> Self {
        value.0.into()
    }
}

/// Validated H3 cell identity, serialized as a hexadecimal string for JavaScript safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct H3Cell(CellIndex);

impl H3Cell {
    /// Indexes an authoritative coordinate at exactly the requested resolution.
    ///
    /// # Panics
    /// Panics if H3 rejects a validated finite WGS 84 coordinate, violating the
    /// shared coordinate invariant.
    #[must_use]
    pub fn at(coordinate: Coordinate, resolution: H3Resolution) -> Self {
        let point = LatLng::new(coordinate.latitude(), coordinate.longitude())
            .unwrap_or_else(|error| panic!("validated coordinate rejected by H3: {error}"));
        Self(point.to_cell(resolution.0))
    }

    /// Returns the resolution encoded by this cell.
    #[must_use]
    pub const fn resolution(self) -> H3Resolution {
        H3Resolution(self.0.resolution())
    }

    /// Returns this cell's logical ancestor, including itself at equal resolution.
    /// Geographic containment of descendants is approximate.
    ///
    /// # Errors
    /// Rejects a finer resolution than this cell.
    pub fn parent(self, resolution: H3Resolution) -> Result<Self, H3Error> {
        self.0
            .parent(resolution.0)
            .map(Self)
            .ok_or(H3Error::FinerResolution)
    }
}

impl fmt::Display for H3Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl TryFrom<String> for H3Cell {
    type Error = H3Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse().map(Self).map_err(|_| H3Error::InvalidCell)
    }
}

impl From<H3Cell> for String {
    fn from(value: H3Cell) -> Self {
        value.to_string()
    }
}

/// Exact membership of one occupied cell in a read-only dispatch projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct H3CellContents {
    /// Cell at the projection's resolution.
    pub cell: H3Cell,
    /// Basic Dispatch eligible idle riders, sorted by stable identity.
    /// Count is rider supply, not parcel capacity or per-order feasibility.
    pub idle_riders: Vec<RiderId>,
    /// Unassigned awaiting-pickup orders, sorted by stable identity.
    /// Count is pickup demand, not parcel units or ready-food observations.
    pub unassigned_orders: Vec<OrderId>,
}

/// Immutable geographic projection of a coherent dispatch snapshot.
///
/// This is analysis, not a candidate policy, road index, ETA, or operational truth.
/// World identity is caller-scoped; it is not a runtime namespace or commit token.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct H3DispatchProjection {
    world_identity: u64,
    world_version: WorldVersion,
    evaluated_at: DispatchInstant,
    resolution: H3Resolution,
    cells: Vec<H3CellContents>,
}

impl H3DispatchProjection {
    /// Buckets eligible idle riders and unassigned pickups using original coordinates.
    /// The fixed source identity, version, and evaluation instant travel with the output.
    ///
    /// # Errors
    /// Propagates incoherent rider eligibility as an evaluation error.
    pub fn from_snapshot(
        snapshot: &DispatchSnapshot<'_>,
        resolution: H3Resolution,
    ) -> Result<Self, DispatchEvaluationError> {
        let data = snapshot.world().data();
        let mut cells = BTreeMap::new();
        for (rider, state) in &data.riders {
            if basic_dispatch_eligible_snapshot(snapshot, *rider)? {
                contents(&mut cells, H3Cell::at(state.coordinate, resolution))
                    .idle_riders
                    .push(*rider);
            }
        }
        for (id, order) in &data.orders {
            if data.fulfillment[id] == FulfillmentState::AwaitingPickup
                && !data.assignments.contains_key(id)
            {
                contents(&mut cells, H3Cell::at(order.pickup, resolution))
                    .unassigned_orders
                    .push(*id);
            }
        }
        Ok(Self {
            world_identity: snapshot.world().identity(),
            world_version: snapshot.world().version(),
            evaluated_at: snapshot.evaluated_at(),
            resolution,
            cells: cells.into_values().collect(),
        })
    }

    /// Occupied cells sorted by H3 identity; empty cells are omitted.
    #[must_use]
    pub fn cells(&self) -> &[H3CellContents] {
        &self.cells
    }

    /// Returns the caller-scoped identity of the source world.
    #[must_use]
    pub const fn world_identity(&self) -> u64 {
        self.world_identity
    }

    /// Returns the exact source world version, including for empty projections.
    #[must_use]
    pub const fn world_version(&self) -> WorldVersion {
        self.world_version
    }

    /// Returns the fixed source snapshot instant.
    #[must_use]
    pub const fn evaluated_at(&self) -> DispatchInstant {
        self.evaluated_at
    }

    /// Returns the common resolution of all projected cells.
    #[must_use]
    pub const fn resolution(&self) -> H3Resolution {
        self.resolution
    }

    /// Logically aggregates membership into coarser cells, preserving source provenance.
    /// Re-index original coordinates instead when geographic coarse-cell membership is needed.
    ///
    /// # Errors
    /// Rejects resolutions finer than this projection, even for empty projections.
    pub fn coarsen(&self, resolution: H3Resolution) -> Result<Self, H3Error> {
        if resolution.0 > self.resolution.0 {
            return Err(H3Error::FinerResolution);
        }
        let mut cells = BTreeMap::new();
        for source in &self.cells {
            let target = contents(&mut cells, source.cell.parent(resolution)?);
            target.idle_riders.extend_from_slice(&source.idle_riders);
            target
                .unassigned_orders
                .extend_from_slice(&source.unassigned_orders);
        }
        for cell in cells.values_mut() {
            cell.idle_riders.sort_unstable();
            cell.unassigned_orders.sort_unstable();
        }
        Ok(Self {
            world_identity: self.world_identity,
            world_version: self.world_version,
            evaluated_at: self.evaluated_at,
            resolution,
            cells: cells.into_values().collect(),
        })
    }
}

fn contents(cells: &mut BTreeMap<H3Cell, H3CellContents>, cell: H3Cell) -> &mut H3CellContents {
    cells.entry(cell).or_insert_with(|| H3CellContents {
        cell,
        idle_riders: Vec::new(),
        unassigned_orders: Vec::new(),
    })
}
