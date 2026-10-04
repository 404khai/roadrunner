use std::collections::BTreeMap;

use roadrunner_core::cost::{
    RoutingContext, TimeDependentCost, TimeDependentTrafficSnapshot, TrafficAwareCost,
    TrafficSnapshot, TravelTimeCost, TraversalEvaluator,
};
use roadrunner_core::geo::{Coordinate, Seconds};
use roadrunner_core::graph::{FrozenGraph, NodeId};
use roadrunner_core::routing::{RouteResult, RoutingError, dijkstra};
use serde::Serialize;

use crate::{DispatchEvaluationError, OrderId, RiderId};

/// Semantic traffic context identity, including explicit free-flow routing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TrafficIdentity {
    /// No traffic overlay.
    FreeFlow,
    /// Immutable static traffic digest.
    Static(String),
    /// Immutable FIFO time-dependent traffic digest.
    TimeDependent(String),
}

/// Inputs fixed across every route leg within a dispatch decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoutingProvenance {
    /// Full authoritative graph digest (core represents this as a string).
    pub graph_digest: String,
    /// Traffic kind and exact snapshot digest.
    pub traffic: TrafficIdentity,
    /// Exact compiled routing profile.
    pub profile: String,
}

/// Caller-supplied node-backed projection of an authoritative location fact.
#[derive(Debug, Clone, PartialEq)]
pub struct RoutingAnchor {
    /// Stable coordinate to which this projection belongs.
    pub coordinate: Coordinate,
    /// Graph digest defining the node identity domain.
    pub graph_digest: String,
    /// Graph-local node, not a durable domain location identity.
    pub node: NodeId,
}

impl RoutingAnchor {
    /// Validates location binding, full graph identity, and node existence.
    ///
    /// # Errors
    /// Returns an anchor error instead of candidate `NoRoute`.
    pub fn validate(
        &self,
        graph: &FrozenGraph,
        coordinate: Coordinate,
    ) -> Result<(), DispatchEvaluationError> {
        if self.graph_digest != graph.metadata().snapshot_digest()
            || graph.node(self.node).is_none()
            || self.coordinate != coordinate
        {
            return Err(DispatchEvaluationError::InvalidAnchor);
        }
        Ok(())
    }
}

/// Decision-scoped projections supplied by callers until coordinate snapping exists.
#[derive(Debug, Clone, Default)]
pub struct RoutingAnchors {
    /// Rider current locations.
    pub riders: BTreeMap<RiderId, RoutingAnchor>,
    /// Order pickup locations.
    pub pickups: BTreeMap<OrderId, RoutingAnchor>,
    /// Order dropoff locations.
    pub dropoffs: BTreeMap<OrderId, RoutingAnchor>,
}

impl RoutingAnchors {
    pub(crate) fn validate(
        &self,
        graph: &FrozenGraph,
        data: &crate::WorldData,
    ) -> Result<(), DispatchEvaluationError> {
        for (rider, anchor) in &self.riders {
            anchor.validate(
                graph,
                data.riders
                    .get(rider)
                    .ok_or(DispatchEvaluationError::InvalidAnchor)?
                    .coordinate,
            )?;
        }
        for (order, anchor) in &self.pickups {
            anchor.validate(
                graph,
                data.orders
                    .get(order)
                    .ok_or(DispatchEvaluationError::InvalidAnchor)?
                    .pickup,
            )?;
        }
        for (order, anchor) in &self.dropoffs {
            anchor.validate(
                graph,
                data.orders
                    .get(order)
                    .ok_or(DispatchEvaluationError::InvalidAnchor)?
                    .dropoff,
            )?;
        }
        Ok(())
    }
}

/// Route response with explicit pinned traffic/profile provenance.
#[derive(Debug, Clone)]
pub struct RoutedLeg {
    /// Core road movement only, without stop waiting or service.
    pub route: RouteResult,
    /// Context used by the provider.
    pub provenance: RoutingProvenance,
    /// Explicit routing departure second for this leg.
    pub departure: Seconds,
}

/// Valid route absence is distinct from provider evaluation failure.
#[derive(Debug, Clone)]
pub enum RouteOutcome {
    /// A computed legal road route.
    RouteFound(Box<RoutedLeg>),
    /// Valid endpoints, no permitted path.
    NoRoute,
}

/// Narrow immutable routing dependency; implementations must not read live state.
///
/// Graph, traffic, and profile are fixed for the provider lifetime. Each leg
/// supplies its own checked departure. No matrix or cache interface is required.
pub trait RouteProvider {
    /// Pinned graph used to validate every anchor.
    fn graph(&self) -> &FrozenGraph;
    /// Pinned graph/traffic/profile identity.
    fn provenance(&self) -> RoutingProvenance;
    /// Computes road movement for one leg.
    ///
    /// # Errors
    /// Provider failures must not be converted to `NoRoute`.
    fn route(
        &self,
        from: NodeId,
        to: NodeId,
        departure: Seconds,
    ) -> Result<RouteOutcome, DispatchEvaluationError>;
}

/// Supported core travel-time inputs; no arbitrary unproven profile substitution.
#[derive(Debug, Clone, Copy)]
pub enum TrafficContext<'a> {
    /// Static free-flow movement.
    FreeFlow,
    /// Graph-bound static traffic.
    Static(&'a TrafficSnapshot),
    /// Graph-bound FIFO time-dependent traffic.
    TimeDependent(&'a TimeDependentTrafficSnapshot),
}

/// Production adapter to Roadrunner's own Dijkstra implementation.
pub struct CoreRouteProvider<'a> {
    graph: &'a FrozenGraph,
    traffic: TrafficContext<'a>,
}

impl<'a> CoreRouteProvider<'a> {
    /// Pins graph and traffic, rejecting mismatched overlays immediately.
    ///
    /// # Errors
    /// Rejects unsupported or mismatched pinned routing inputs.
    pub fn new(
        graph: &'a FrozenGraph,
        traffic: TrafficContext<'a>,
    ) -> Result<Self, DispatchEvaluationError> {
        let supports = match traffic {
            TrafficContext::FreeFlow => true,
            TrafficContext::Static(t) => t.supports_graph(graph),
            TrafficContext::TimeDependent(t) => t.supports_graph(graph),
        };
        if !supports {
            return Err(DispatchEvaluationError::ProvenanceMismatch);
        }
        Ok(Self { graph, traffic })
    }
}

impl RouteProvider for CoreRouteProvider<'_> {
    fn graph(&self) -> &FrozenGraph {
        self.graph
    }
    fn provenance(&self) -> RoutingProvenance {
        RoutingProvenance {
            graph_digest: self.graph.metadata().snapshot_digest().to_owned(),
            profile: self.graph.metadata().routing_profile().to_owned(),
            traffic: match self.traffic {
                TrafficContext::FreeFlow => TrafficIdentity::FreeFlow,
                TrafficContext::Static(t) => {
                    TrafficIdentity::Static(t.traffic_snapshot_digest().to_owned())
                }
                TrafficContext::TimeDependent(t) => {
                    TrafficIdentity::TimeDependent(t.traffic_snapshot_digest().to_owned())
                }
            },
        }
    }
    fn route(
        &self,
        from: NodeId,
        to: NodeId,
        departure: Seconds,
    ) -> Result<RouteOutcome, DispatchEvaluationError> {
        let static_cost;
        let time_cost;
        let evaluator: &dyn TraversalEvaluator = match self.traffic {
            TrafficContext::FreeFlow => &TravelTimeCost,
            TrafficContext::Static(t) => {
                static_cost = TrafficAwareCost::new(t);
                &static_cost
            }
            TrafficContext::TimeDependent(t) => {
                time_cost = TimeDependentCost::new(t);
                &time_cost
            }
        };
        match dijkstra(
            self.graph,
            from,
            to,
            evaluator,
            &RoutingContext::with_departure_time(departure),
        ) {
            Ok(route) => Ok(RouteOutcome::RouteFound(Box::new(RoutedLeg {
                route,
                provenance: self.provenance(),
                departure,
            }))),
            Err(RoutingError::NoRoute { .. }) => Ok(RouteOutcome::NoRoute),
            Err(error) => Err(DispatchEvaluationError::Routing(error)),
        }
    }
}
