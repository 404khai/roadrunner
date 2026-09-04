//! Shortest-path algorithms and canonical route results.

mod astar;
mod dijkstra;
mod error;
mod result;
mod search;

pub use astar::astar;
pub use dijkstra::dijkstra;
pub use error::{RouteEndpoint, RoutingError};
pub use result::{RouteResult, RoutingAlgorithm};
