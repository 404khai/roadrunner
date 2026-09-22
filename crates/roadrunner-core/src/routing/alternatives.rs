//! Bounded Yen-style enumeration of loopless static routes.

use std::cmp::Ordering;
use std::collections::{BTreeSet, BinaryHeap};

use serde::Serialize;

use crate::cost::{RouteCost, RoutingContext, SearchCapability, TraversalEvaluator};
use crate::geo::{Meters, Seconds};
use crate::graph::{EdgeId, FrozenGraph, NodeId, RoadSegmentId};

use super::result::RouteMetrics;
use super::search::{Label, evaluate, validate_request};
use super::{RouteResult, RoutingAlgorithm, RoutingError};

/// Limits and diversity rules for a bounded alternative-route request.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct AlternativeRouteOptions {
    /// Maximum number of returned routes, including the primary.
    pub max_routes: usize,
    /// Maximum fraction of the shorter route's physical length shared with any selected route.
    pub max_shared_distance_ratio: f64,
    /// Maximum objective cost divided by the primary route cost.
    pub max_cost_factor: f64,
    /// Maximum path states expanded across all searches and queued in one search.
    pub max_search_states: usize,
    /// Maximum distinct candidate paths retained by Yen enumeration.
    pub max_candidate_paths: usize,
}

impl Default for AlternativeRouteOptions {
    fn default() -> Self {
        Self {
            max_routes: 3,
            max_shared_distance_ratio: 0.8,
            max_cost_factor: 1.5,
            max_search_states: 50_000,
            max_candidate_paths: 1_000,
        }
    }
}

impl AlternativeRouteOptions {
    fn validate(self) -> Result<(), RoutingError> {
        let reason = if self.max_routes == 0 {
            Some("max_routes must be positive")
        } else if !self.max_shared_distance_ratio.is_finite()
            || !(0.0..1.0).contains(&self.max_shared_distance_ratio)
        {
            Some("max_shared_distance_ratio must be finite and in [0, 1)")
        } else if !self.max_cost_factor.is_finite() || self.max_cost_factor < 1.0 {
            Some("max_cost_factor must be finite and at least 1")
        } else if self.max_search_states == 0 || self.max_candidate_paths == 0 {
            Some("search and candidate budgets must be positive")
        } else {
            None
        };
        match reason {
            Some(reason) => Err(RoutingError::InvalidAlternativeOptions { reason }),
            None => Ok(()),
        }
    }
}

/// A ranked route and its largest physical overlap with a previously selected route.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlternativeRoute {
    /// One-based route rank.
    pub rank: usize,
    /// Validated route through the requested graph snapshot.
    pub route: RouteResult,
    /// Maximum shared-length ratio against all lower-ranked returned routes.
    pub max_shared_distance_ratio: f64,
}

/// Bounded alternative-route result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AlternativeRoutes {
    /// Selected, ranked routes; the primary is first.
    pub routes: Vec<AlternativeRoute>,
    /// Rules and work limits used for this request.
    pub options: AlternativeRouteOptions,
    /// Number of distinct loopless complete paths ranked before selection ended.
    pub ranked_paths: usize,
    /// Search states expanded across all spur searches.
    pub search_states: usize,
    /// True if a search or candidate budget prevented complete enumeration.
    pub truncated: bool,
    /// Why enumeration stopped.
    pub termination: AlternativeTermination,
}

/// Reason alternative-route enumeration stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternativeTermination {
    /// The requested number of diverse routes was selected.
    RequestedCount,
    /// No further loopless path exists.
    Exhausted,
    /// All remaining paths exceed the permitted cost factor.
    CostLimit,
    /// The work budget was exhausted before enumeration completed.
    Budget,
}

#[derive(Clone)]
struct Path {
    nodes: Vec<NodeId>,
    edges: Vec<EdgeId>,
    label: Label,
    distance: Meters,
    search_states: usize,
}

impl Path {
    fn new(source: NodeId, evaluator: &dyn TraversalEvaluator) -> Self {
        Self {
            nodes: vec![source],
            edges: Vec::new(),
            label: Label {
                objective: RouteCost::zero(evaluator.kind()),
                elapsed: Seconds::ZERO,
            },
            distance: Meters::ZERO,
            search_states: 0,
        }
    }

    fn route(&self, graph: &FrozenGraph) -> RouteResult {
        RouteResult::new(
            graph.snapshot_id(),
            graph.metadata().snapshot_digest().to_owned(),
            RoutingAlgorithm::Yen,
            self.nodes.clone(),
            self.edges.clone(),
            RouteMetrics {
                total_distance: self.distance,
                total_cost: self.label.objective,
                elapsed_travel_time: self.label.elapsed,
                expanded_states: self.search_states,
            },
        )
    }
}

#[derive(Clone)]
struct QueuedPath(Path);

impl PartialEq for QueuedPath {
    fn eq(&self, other: &Self) -> bool {
        self.0.label.objective == other.0.label.objective && self.0.edges == other.0.edges
    }
}
impl Eq for QueuedPath {}
impl PartialOrd for QueuedPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for QueuedPath {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .label
            .objective
            .value()
            .total_cmp(&self.0.label.objective.value())
            .then_with(|| other.0.edges.cmp(&self.0.edges))
    }
}

enum SearchOutcome {
    Found(Path),
    NoRoute,
    Truncated,
}

/// Returns bounded loopless alternatives for a static, non-negative objective.
///
/// Paths are enumerated by objective cost and filtered by maximum cost and
/// shared physical road length. The primary route is always returned when one
/// exists. A truncated result may contain fewer routes than requested.
///
/// # Errors
///
/// Returns a typed error for invalid endpoints/options, unsupported evaluator
/// capability, traversal errors, or failure to establish the primary within the
/// search budget. An unreachable destination returns [`RoutingError::NoRoute`].
#[allow(clippy::too_many_lines)]
pub fn alternatives(
    graph: &FrozenGraph,
    source: NodeId,
    destination: NodeId,
    evaluator: &dyn TraversalEvaluator,
    context: &RoutingContext,
    options: AlternativeRouteOptions,
) -> Result<AlternativeRoutes, RoutingError> {
    options.validate()?;
    validate_request(graph, source, destination, evaluator)?;
    if evaluator.capability() != SearchCapability::StaticNonNegative {
        return Err(RoutingError::UnsupportedCapability {
            capability: evaluator.capability(),
        });
    }
    let mut search_states = 0;
    let primary = match shortest_simple_path(
        graph,
        Path::new(source, evaluator),
        destination,
        evaluator,
        *context,
        &BTreeSet::new(),
        &BTreeSet::new(),
        &mut search_states,
        options.max_search_states,
    )? {
        SearchOutcome::Found(path) => path,
        SearchOutcome::NoRoute => {
            return Err(RoutingError::NoRoute {
                source_node: source,
                destination,
            });
        }
        SearchOutcome::Truncated => return Err(RoutingError::AlternativeSearchLimit),
    };
    let primary_cost = primary.label.objective.value();
    let mut ranked = vec![primary.clone()];
    let mut selected = vec![AlternativeRoute {
        rank: 1,
        route: primary.route(graph),
        max_shared_distance_ratio: 0.0,
    }];
    let mut candidates = BinaryHeap::new();
    let mut seen = BTreeSet::from([primary.edges.clone()]);
    let mut truncated = false;
    let mut termination = AlternativeTermination::RequestedCount;
    while selected.len() < options.max_routes {
        let Some(previous) = ranked.last() else {
            termination = AlternativeTermination::Exhausted;
            break;
        };
        for spur_index in 0..previous.edges.len() {
            let root = prefix_path(graph, previous, spur_index, evaluator, *context)?;
            let banned_nodes = root.nodes[..spur_index].iter().copied().collect();
            let banned_edges = ranked
                .iter()
                .filter(|path| {
                    path.edges.len() > spur_index && path.edges[..spur_index] == root.edges
                })
                .map(|path| path.edges[spur_index])
                .collect();
            let result = shortest_simple_path(
                graph,
                root,
                destination,
                evaluator,
                *context,
                &banned_nodes,
                &banned_edges,
                &mut search_states,
                options.max_search_states,
            )?;
            match result {
                SearchOutcome::Found(path) => {
                    if seen.insert(path.edges.clone()) {
                        if candidates.len() >= options.max_candidate_paths {
                            truncated = true;
                            break;
                        }
                        candidates.push(QueuedPath(path));
                    }
                }
                SearchOutcome::NoRoute => {}
                SearchOutcome::Truncated => {
                    truncated = true;
                    break;
                }
            }
        }
        if truncated {
            termination = AlternativeTermination::Budget;
            break;
        }
        let Some(QueuedPath(next)) = candidates.pop() else {
            termination = AlternativeTermination::Exhausted;
            break;
        };
        let next_cost = next.label.objective.value();
        let over_cost_limit = if primary_cost == 0.0 {
            next_cost > 0.0
        } else {
            next_cost / primary_cost > options.max_cost_factor
        };
        if over_cost_limit {
            termination = AlternativeTermination::CostLimit;
            break;
        }
        let overlap = selected
            .iter()
            .map(|route| shared_distance_ratio(graph, &next, &route.route))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .fold(0.0_f64, f64::max);
        if overlap <= options.max_shared_distance_ratio {
            selected.push(AlternativeRoute {
                rank: selected.len() + 1,
                route: next.route(graph),
                max_shared_distance_ratio: overlap,
            });
        }
        ranked.push(next);
    }
    Ok(AlternativeRoutes {
        routes: selected,
        options,
        ranked_paths: ranked.len(),
        search_states,
        truncated,
        termination,
    })
}

fn prefix_path(
    graph: &FrozenGraph,
    path: &Path,
    edge_count: usize,
    evaluator: &dyn TraversalEvaluator,
    context: RoutingContext,
) -> Result<Path, RoutingError> {
    let mut prefix = Path::new(path.nodes[0], evaluator);
    for &edge_id in &path.edges[..edge_count] {
        let edge = graph
            .edge(edge_id)
            .ok_or(RoutingError::MissingRouteEdge { edge_id })?;
        prefix = extend(graph, prefix, edge, evaluator, context)?
            .ok_or(RoutingError::MissingRouteEdge { edge_id })?;
    }
    Ok(prefix)
}

#[allow(clippy::too_many_arguments)]
fn shortest_simple_path(
    graph: &FrozenGraph,
    root: Path,
    destination: NodeId,
    evaluator: &dyn TraversalEvaluator,
    context: RoutingContext,
    banned_nodes: &BTreeSet<NodeId>,
    banned_edges: &BTreeSet<EdgeId>,
    search_states: &mut usize,
    max_search_states: usize,
) -> Result<SearchOutcome, RoutingError> {
    let start = *search_states;
    let mut queue = BinaryHeap::from([QueuedPath(root)]);
    while let Some(QueuedPath(mut path)) = queue.pop() {
        if *search_states >= max_search_states {
            return Ok(SearchOutcome::Truncated);
        }
        *search_states += 1;
        let node = path.nodes[path.nodes.len() - 1];
        if node == destination {
            path.search_states = *search_states - start;
            return Ok(SearchOutcome::Found(path));
        }
        for edge in graph
            .outgoing_edges(node)
            .map_err(|source| RoutingError::Graph { source })?
        {
            if banned_nodes.contains(&edge.to())
                || banned_edges.contains(&edge.id())
                || path.nodes.contains(&edge.to())
                || path
                    .edges
                    .last()
                    .is_some_and(|incoming| !graph.is_maneuver_allowed(*incoming, edge.id()))
            {
                continue;
            }
            if let Some(next) = extend(graph, path.clone(), edge, evaluator, context)? {
                if queue.len() >= max_search_states {
                    return Ok(SearchOutcome::Truncated);
                }
                queue.push(QueuedPath(next));
            }
        }
    }
    Ok(SearchOutcome::NoRoute)
}

fn extend(
    graph: &FrozenGraph,
    mut path: Path,
    edge: &crate::graph::DirectedEdge,
    evaluator: &dyn TraversalEvaluator,
    context: RoutingContext,
) -> Result<Option<Path>, RoutingError> {
    let label = Label {
        objective: path.label.objective,
        elapsed: path.label.elapsed,
    };
    let Some(next) = evaluate(graph, evaluator, edge, label, context)? else {
        return Ok(None);
    };
    let segment = graph
        .segment(edge.segment())
        .ok_or(RoutingError::MissingRouteEdge { edge_id: edge.id() })?;
    path.distance = path
        .distance
        .checked_add(segment.distance())
        .map_err(|source| RoutingError::DistanceAccumulation {
            edge_id: edge.id(),
            source,
        })?;
    path.label = next;
    path.nodes.push(edge.to());
    path.edges.push(edge.id());
    Ok(Some(path))
}

fn shared_distance_ratio(
    graph: &FrozenGraph,
    candidate: &Path,
    other: &RouteResult,
) -> Result<f64, RoutingError> {
    let segments: BTreeSet<RoadSegmentId> = other
        .edges()
        .iter()
        .map(|edge_id| {
            graph
                .edge(*edge_id)
                .map(|edge| edge.segment())
                .ok_or(RoutingError::MissingRouteEdge { edge_id: *edge_id })
        })
        .collect::<Result<_, _>>()?;
    let mut shared = 0.0;
    for edge_id in &candidate.edges {
        let edge = graph
            .edge(*edge_id)
            .ok_or(RoutingError::MissingRouteEdge { edge_id: *edge_id })?;
        if segments.contains(&edge.segment()) {
            let segment = graph
                .segment(edge.segment())
                .ok_or(RoutingError::MissingRouteEdge { edge_id: *edge_id })?;
            shared += segment.distance().value();
        }
    }
    let shorter = candidate
        .distance
        .value()
        .min(other.total_distance().value());
    Ok(if shorter == 0.0 {
        0.0
    } else {
        (shared / shorter).min(1.0)
    })
}
