//! Immutable static traffic overlay and free-flow-time multiplier evaluator.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::geo::Seconds;
use crate::graph::{DirectedEdge, EdgeId, FrozenGraph, GraphSnapshotId, RoadSegment};

use super::{
    CostError, CostKind, HeuristicPolicy, RouteCost, RoutingContext, SearchCapability,
    TravelTimeCost, TraversalEvaluation, TraversalEvaluator, TraversalState,
};

/// A finite congestion factor no lower than the free-flow baseline of 1.0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct TrafficMultiplier(f64);

impl TrafficMultiplier {
    /// No traffic delay.
    pub const NORMAL: Self = Self(1.0);

    /// Validates a numeric traffic multiplier.
    ///
    /// # Errors
    ///
    /// Rejects non-finite values and values below 1.0. Keeping the multiplier
    /// at or above 1.0 preserves the free-flow A* travel-time lower bound.
    pub fn new(value: f64) -> Result<Self, TrafficError> {
        if !value.is_finite() || value < 1.0 {
            return Err(TrafficError::InvalidMultiplier { value });
        }
        Ok(Self(value))
    }

    /// Returns the dimensionless factor.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for TrafficMultiplier {
    type Error = TrafficError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<TrafficMultiplier> for f64 {
    fn from(value: TrafficMultiplier) -> Self {
        value.0
    }
}

/// Named deterministic scenario levels, mapped to numeric factors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrafficLevel {
    /// Free-flow time × 1.0.
    Normal,
    /// Free-flow time × 1.25.
    Moderate,
    /// Free-flow time × 1.6.
    Heavy,
    /// Free-flow time × 2.5.
    Severe,
}

impl TrafficLevel {
    /// Returns this level's numeric multiplier.
    #[must_use]
    pub const fn multiplier(self) -> TrafficMultiplier {
        match self {
            Self::Normal => TrafficMultiplier::NORMAL,
            Self::Moderate => TrafficMultiplier(1.25),
            Self::Heavy => TrafficMultiplier(1.6),
            Self::Severe => TrafficMultiplier(2.5),
        }
    }
}

/// Invalid traffic overlay input.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum TrafficError {
    /// A factor would speed up the free-flow baseline or is non-finite.
    #[error("traffic multiplier must be finite and at least 1.0, got {value}")]
    InvalidMultiplier {
        /// Rejected factor.
        value: f64,
    },
    /// Override references an absent directed edge.
    #[error("traffic override references absent edge {edge_id}")]
    UnknownEdge {
        /// Absent directed edge.
        edge_id: EdgeId,
    },
    /// Override repeats an edge ID.
    #[error("traffic override repeats edge {edge_id}")]
    DuplicateEdge {
        /// Repeated directed edge.
        edge_id: EdgeId,
    },
}

/// Immutable directional traffic factors bound to one graph snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrafficSnapshot {
    graph_snapshot_id: GraphSnapshotId,
    graph_snapshot_digest: String,
    digest: String,
    multipliers: Vec<TrafficMultiplier>,
}

impl TrafficSnapshot {
    /// Builds a deterministic overlay; unspecified edges use factor 1.0.
    ///
    /// # Errors
    ///
    /// Rejects unknown or duplicate directed edge IDs.
    pub fn new(
        graph: &FrozenGraph,
        overrides: impl IntoIterator<Item = (EdgeId, TrafficMultiplier)>,
    ) -> Result<Self, TrafficError> {
        let mut multipliers = vec![TrafficMultiplier::NORMAL; graph.edge_count()];
        let mut ordered: Vec<_> = overrides.into_iter().collect();
        ordered.sort_by_key(|(edge_id, _)| *edge_id);
        for (index, &(edge_id, multiplier)) in ordered.iter().enumerate() {
            if index > 0 && ordered[index - 1].0 == edge_id {
                return Err(TrafficError::DuplicateEdge { edge_id });
            }
            let Some(slot) = multipliers.get_mut(edge_id.value() as usize) else {
                return Err(TrafficError::UnknownEdge { edge_id });
            };
            *slot = multiplier;
        }
        let mut hasher = Sha256::new();
        hasher.update(b"roadrunner_traffic_snapshot_v1");
        hasher.update(graph.snapshot_id().value().to_le_bytes());
        hasher.update(graph.metadata().snapshot_digest().as_bytes());
        let edge_count = graph
            .edges()
            .last()
            .map_or(0_u64, |edge| u64::from(edge.id().value()) + 1);
        hasher.update(edge_count.to_le_bytes());
        for (index, multiplier) in multipliers.iter().enumerate() {
            if *multiplier != TrafficMultiplier::NORMAL {
                hasher.update(graph.edges()[index].id().value().to_le_bytes());
                hasher.update(multiplier.value().to_bits().to_le_bytes());
            }
        }
        Ok(Self {
            graph_snapshot_id: graph.snapshot_id(),
            graph_snapshot_digest: graph.metadata().snapshot_digest().to_owned(),
            digest: format!("{:x}", hasher.finalize()),
            multipliers,
        })
    }

    /// Returns the graph snapshot used to construct this overlay.
    #[must_use]
    pub const fn graph_snapshot_id(&self) -> GraphSnapshotId {
        self.graph_snapshot_id
    }

    /// Returns the graph's semantic digest.
    #[must_use]
    pub fn graph_snapshot_digest(&self) -> &str {
        &self.graph_snapshot_digest
    }

    /// Returns the canonical graph-bound traffic scenario digest.
    #[must_use]
    pub fn traffic_snapshot_digest(&self) -> &str {
        &self.digest
    }

    /// Returns the factor for a directed edge, if the edge exists.
    #[must_use]
    pub fn multiplier(&self, edge_id: EdgeId) -> Option<TrafficMultiplier> {
        self.multipliers.get(edge_id.value() as usize).copied()
    }

    /// Returns whether this overlay belongs to the supplied graph snapshot.
    #[must_use]
    pub fn supports_graph(&self, graph: &FrozenGraph) -> bool {
        self.graph_snapshot_id == graph.snapshot_id()
            && self.graph_snapshot_digest == graph.metadata().snapshot_digest()
            && self.multipliers.len() == graph.edge_count()
    }
}

/// Static fastest-route evaluator using a graph-bound traffic snapshot.
#[derive(Debug, Clone, Copy)]
pub struct TrafficAwareCost<'a> {
    traffic: &'a TrafficSnapshot,
}

impl<'a> TrafficAwareCost<'a> {
    /// Binds the evaluator to an immutable traffic scenario.
    #[must_use]
    pub const fn new(traffic: &'a TrafficSnapshot) -> Self {
        Self { traffic }
    }
}

impl TraversalEvaluator for TrafficAwareCost<'_> {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }

    fn capability(&self) -> SearchCapability {
        SearchCapability::StaticNonNegative
    }

    fn supports_graph(&self, graph: &FrozenGraph) -> bool {
        self.traffic.supports_graph(graph)
    }

    fn supports_heuristic(&self, policy: HeuristicPolicy) -> bool {
        matches!(
            policy,
            HeuristicPolicy::Zero | HeuristicPolicy::TravelTimeHaversine
        )
    }

    fn evaluate(
        &self,
        edge: &DirectedEdge,
        segment: &RoadSegment,
        state: TraversalState,
        context: &RoutingContext,
    ) -> Result<TraversalEvaluation, CostError> {
        let base = TravelTimeCost.evaluate(edge, segment, state, context)?;
        let TraversalEvaluation::Traversable { travel_time, .. } = base else {
            return Ok(base);
        };
        let multiplier = self
            .traffic
            .multiplier(edge.id())
            .ok_or(CostError::MissingTrafficEdge { edge_id: edge.id() })?;
        let adjusted = travel_time.value() * multiplier.value();
        let objective_cost = RouteCost::new(CostKind::TravelTime, adjusted)?;
        let travel_time = Seconds::new(adjusted).map_err(|_| CostError::NotFinite {
            kind: CostKind::TravelTime,
            value: adjusted,
        })?;
        Ok(TraversalEvaluation::Traversable {
            objective_cost,
            travel_time,
        })
    }
}
