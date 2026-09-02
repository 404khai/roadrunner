//! Shortest-path algorithms and canonical route results.

mod dijkstra;
mod error;
mod result;

pub use dijkstra::dijkstra;
pub use error::{RouteEndpoint, RoutingError};
pub use result::{RouteResult, RoutingAlgorithm};
