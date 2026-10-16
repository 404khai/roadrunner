//! Shortest-path algorithms and canonical route results.

mod alternatives;
mod astar;
mod dijkstra;
mod error;
mod evidence;
mod geometry;
mod heuristic;
mod result;
mod search;

pub use alternatives::{
    AlternativeRoute, AlternativeRouteOptions, AlternativeRoutes, AlternativeTermination,
    alternatives,
};
pub use astar::astar;
pub use dijkstra::dijkstra;
pub use error::{RouteEndpoint, RoutingError};
pub use evidence::{
    EvaluationProvenance, GeometryEncoding, RouteAuthority, RouteEvidence, RouteEvidenceSemantics,
    RouteSearchSummary, route_evidence,
};
pub use geometry::{
    CheckedRouteGeometry, GeometryError, GeometryLimits, TraversalOccurrence,
    checked_route_geometry,
};
pub use heuristic::{
    DistanceHaversine, Heuristic, HeuristicConfigError, TravelTimeHaversine, ZeroHeuristic,
};
pub use result::{RouteResult, RoutingAlgorithm};
