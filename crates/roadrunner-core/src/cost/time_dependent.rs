//! FIFO time-dependent routing over deterministic edge profiles.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::geo::Seconds;
use crate::graph::{DirectedEdge, EdgeId, FrozenGraph, GraphSnapshotId, RoadSegment};

use super::{
    CostError, CostKind, HeuristicPolicy, RouteCost, RoutingContext, SearchCapability,
    TrafficMultiplier, TravelTimeCost, TraversalEvaluation, TraversalEvaluator, TraversalState,
};

/// A multiplier at an absolute logical departure time in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficPoint {
    /// Absolute logical time since the scenario epoch.
    pub departure_seconds: Seconds,
    /// Factor applied to free-flow traversal time.
    pub multiplier: TrafficMultiplier,
}

/// One directed edge's ordered traffic profile.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrafficProfile {
    /// Snapshot-local directed edge identity.
    pub edge_id: EdgeId,
    /// Time points joined by linear interpolation.
    pub points: Vec<TrafficPoint>,
}

/// Invalid graph-bound time-dependent traffic input.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum TimeDependentTrafficError {
    /// The profile references an absent directed edge.
    #[error("time-dependent profile references absent edge {edge_id}")]
    UnknownEdge {
        /// Absent edge.
        edge_id: EdgeId,
    },
    /// A directed edge has multiple profiles.
    #[error("time-dependent profile repeats edge {edge_id}")]
    DuplicateEdge {
        /// Repeated edge.
        edge_id: EdgeId,
    },
    /// At least one time point is required.
    #[error("time-dependent profile for edge {edge_id} has no points")]
    EmptyProfile {
        /// Edge with an empty profile.
        edge_id: EdgeId,
    },
    /// Time points must strictly increase.
    #[error("time-dependent profile for edge {edge_id} has non-increasing times")]
    NonIncreasingTime {
        /// Edge with unsorted or repeated times.
        edge_id: EdgeId,
    },
    /// The edge's arrival function would decrease over an interval.
    #[error(
        "time-dependent profile for edge {edge_id} violates FIFO between {from_seconds}s and {to_seconds}s"
    )]
    NonFifo {
        /// Edge with an invalid interval.
        edge_id: EdgeId,
        /// Interval start.
        from_seconds: f64,
        /// Interval end.
        to_seconds: f64,
    },
}

/// Immutable time profiles bound to one graph snapshot.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimeDependentTrafficSnapshot {
    graph_snapshot_id: GraphSnapshotId,
    graph_snapshot_digest: String,
    digest: String,
    profiles: Vec<Option<Vec<TrafficPoint>>>,
}

impl TimeDependentTrafficSnapshot {
    /// Validates deterministic time profiles and proves FIFO for each edge.
    ///
    /// Unspecified edges use factor 1.0. A profile is constant before its first
    /// point and after its last point, and linearly interpolated between points.
    ///
    /// # Errors
    ///
    /// Rejects absent/duplicate edges, empty or unordered profiles, and
    /// intervals with a decreasing arrival-time function.
    pub fn new(
        graph: &FrozenGraph,
        profiles: impl IntoIterator<Item = TrafficProfile>,
    ) -> Result<Self, TimeDependentTrafficError> {
        let mut ordered: Vec<_> = profiles.into_iter().collect();
        ordered.sort_by_key(|profile| profile.edge_id);
        let mut indexed = vec![None; graph.edge_count()];
        for profile in ordered {
            let edge =
                graph
                    .edge(profile.edge_id)
                    .ok_or(TimeDependentTrafficError::UnknownEdge {
                        edge_id: profile.edge_id,
                    })?;
            let slot = &mut indexed[profile.edge_id.value() as usize];
            if slot.is_some() {
                return Err(TimeDependentTrafficError::DuplicateEdge {
                    edge_id: profile.edge_id,
                });
            }
            if profile.points.is_empty() {
                return Err(TimeDependentTrafficError::EmptyProfile {
                    edge_id: profile.edge_id,
                });
            }
            for pair in profile.points.windows(2) {
                let start = pair[0];
                let end = pair[1];
                let span = end.departure_seconds.value() - start.departure_seconds.value();
                if span <= 0.0 {
                    return Err(TimeDependentTrafficError::NonIncreasingTime {
                        edge_id: profile.edge_id,
                    });
                }
                let decrease = start.multiplier.value() - end.multiplier.value();
                if decrease > 0.0 && edge.free_flow_travel_time().value() * decrease > span {
                    return Err(TimeDependentTrafficError::NonFifo {
                        edge_id: profile.edge_id,
                        from_seconds: start.departure_seconds.value(),
                        to_seconds: end.departure_seconds.value(),
                    });
                }
            }
            *slot = Some(profile.points);
        }
        let mut hasher = Sha256::new();
        hasher.update(b"roadrunner_time_dependent_traffic_v1");
        hasher.update(graph.snapshot_id().value().to_le_bytes());
        hasher.update(graph.metadata().snapshot_digest().as_bytes());
        let edge_count = graph
            .edges()
            .last()
            .map_or(0_u64, |edge| u64::from(edge.id().value()) + 1);
        hasher.update(edge_count.to_le_bytes());
        for (edge, points) in graph.edges().iter().zip(&indexed) {
            let Some(points) = points else {
                continue;
            };
            hasher.update(edge.id().value().to_le_bytes());
            hasher.update((points.len() as u64).to_le_bytes());
            for point in points {
                hasher.update(point.departure_seconds.value().to_bits().to_le_bytes());
                hasher.update(point.multiplier.value().to_bits().to_le_bytes());
            }
        }
        Ok(Self {
            graph_snapshot_id: graph.snapshot_id(),
            graph_snapshot_digest: graph.metadata().snapshot_digest().to_owned(),
            digest: format!("{:x}", hasher.finalize()),
            profiles: indexed,
        })
    }

    /// Returns the graph identity to which the profiles are bound.
    #[must_use]
    pub const fn graph_snapshot_id(&self) -> GraphSnapshotId {
        self.graph_snapshot_id
    }

    /// Returns the graph's semantic digest.
    #[must_use]
    pub fn graph_snapshot_digest(&self) -> &str {
        &self.graph_snapshot_digest
    }

    /// Returns a deterministic digest of the graph-bound profiles.
    #[must_use]
    pub fn traffic_snapshot_digest(&self) -> &str {
        &self.digest
    }

    /// Returns whether these profiles belong to the supplied graph snapshot.
    #[must_use]
    pub fn supports_graph(&self, graph: &FrozenGraph) -> bool {
        self.graph_snapshot_id == graph.snapshot_id()
            && self.graph_snapshot_digest == graph.metadata().snapshot_digest()
            && self.profiles.len() == graph.edge_count()
    }

    /// Returns the interpolated factor for an edge at its absolute entry time.
    ///
    /// An absent edge ID returns `None`; an edge without a profile returns 1.0.
    #[must_use]
    pub fn multiplier_at(&self, edge_id: EdgeId, entry_time: Seconds) -> Option<f64> {
        let profile = self.profiles.get(edge_id.value() as usize)?;
        let Some(points) = profile else {
            return Some(1.0);
        };
        let time = entry_time.value();
        let first = points.first()?;
        if time <= first.departure_seconds.value() {
            return Some(first.multiplier.value());
        }
        let last = points.last()?;
        if time >= last.departure_seconds.value() {
            return Some(last.multiplier.value());
        }
        let upper = points.partition_point(|point| point.departure_seconds.value() < time);
        let start = points[upper - 1];
        let end = points[upper];
        let fraction = (time - start.departure_seconds.value())
            / (end.departure_seconds.value() - start.departure_seconds.value());
        Some(
            (start.multiplier.value()
                + (end.multiplier.value() - start.multiplier.value()) * fraction)
                .max(1.0),
        )
    }
}

/// Earliest-arrival evaluator using validated FIFO edge profiles.
#[derive(Debug, Clone, Copy)]
pub struct TimeDependentCost<'a> {
    traffic: &'a TimeDependentTrafficSnapshot,
}

impl<'a> TimeDependentCost<'a> {
    /// Binds the evaluator to one immutable time-dependent traffic scenario.
    #[must_use]
    pub const fn new(traffic: &'a TimeDependentTrafficSnapshot) -> Self {
        Self { traffic }
    }
}

impl TraversalEvaluator for TimeDependentCost<'_> {
    fn kind(&self) -> CostKind {
        CostKind::TravelTime
    }

    fn capability(&self) -> SearchCapability {
        SearchCapability::FifoEarliestArrival
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
        let entry_time = context
            .departure_time()
            .checked_add(state.elapsed_travel_time())
            .map_err(|_| CostError::EdgeEntryTimeOverflow)?;
        let multiplier = self
            .traffic
            .multiplier_at(edge.id(), entry_time)
            .ok_or(CostError::MissingTrafficEdge { edge_id: edge.id() })?;
        let adjusted = travel_time.value() * multiplier;
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
