import { z } from "zod";
import { catalogSchema, parseExact, type RouteRequest } from "./contracts";
const count = z.number().int().min(0).max(0xffffffff);
const scalar = z.number().finite().nonnegative();
const point = z.object({
  latitude_e7: z.number().int().min(-900000000).max(900000000),
  longitude_e7: z.number().int().min(-1800000000).max(1800000000),
});
const page = z.object({
  offset: count,
  end: count,
  next_offset: count.nullable(),
});
const edge = z.object({
  edge: count,
  segment: count,
  from: count,
  to: count,
  orientation: z.enum(["forward", "reverse"]),
  points_e7: z.array(point).min(2).max(20000),
  distance_meters: scalar,
  base_seconds: scalar,
  traffic_multiplier: z.number().finite().min(1).nullable(),
  access: z.enum([
    "general",
    "destination",
    "delivery",
    "customers",
    "private",
    "permit_required",
    "unknown_explicit",
  ]),
  road_class: z.null(),
  cost_status: z.enum([
    "traversable",
    "forbidden",
    "unavailable_no_traffic_snapshot",
  ]),
  objective_cost: scalar.nullable(),
  predicted_elapsed_seconds: scalar.nullable(),
});
const schema = z.object({
  schema_version: z.literal(1),
  semantics: z.literal("bounded_directed_graph_inspection/v1"),
  graph_view_id: z.string().regex(/^graph-view-v1:[a-f0-9]{64}$/),
  graph: catalogSchema.shape.graph,
  node_count: count,
  edge_count: count,
  node_page: page,
  edge_page: page,
  point_count: count.max(20000),
  nodes_scope: z.literal("node_page_plus_exported_edge_endpoints"),
  edge_scope: z.literal("contiguous_page_not_spatial_query"),
  nodes: z.array(z.object({ node: count, coordinate_e7: point })).max(900),
  edges: z.array(edge).max(200),
  traffic_snapshot_digest: z.string().nullable(),
  evaluation: z.object({
    objective: z.enum(["distance", "travel_time", "traffic_aware"]),
    evaluator: z.string(),
    objective_unit: z.enum(["meters", "seconds"]),
    traffic_digest: z.string().nullable(),
    departure_seconds: scalar,
    time_domain: z.string(),
    routing_context: z.object({
      departure_time: scalar,
      private_access: z.literal(false),
      permit_access: z.literal(false),
    }),
    scope: z.literal(
      "standalone_directed_traversal_default_access;not_maneuver_or_route_feasibility",
    ),
  }),
  capabilities: z.object({
    road_class: z.literal("unavailable_not_retained"),
    explored_node_membership: z.literal("unavailable_not_retained"),
    ordered_search_trace: z.literal("unavailable_not_retained"),
  }),
});
export type GraphView = z.infer<typeof schema> & { raw: string };
export type GraphEdge = GraphView["edges"][number];
export type GraphSelection = { kind: "edge" | "node"; id: number };
export type GraphQuery = {
  digest: string;
  nodeOffset: number;
  edgeOffset: number;
  objective: RouteRequest["objective"];
};
export function decodeGraph(raw: string, q: GraphQuery): GraphView {
  const parsed = schema.safeParse(parseExact(raw));
  if (!parsed.success)
    throw new Error(
      "Unsupported graph inspection schema, capabilities or required fields.",
    );
  const g = parsed.data,
    e = g.evaluation;
  const fail = () => {
    throw new Error(
      "Graph inspection evidence is inconsistent with its page, identity or static evaluation.",
    );
  };
  if (
    g.graph.snapshot_digest !== q.digest ||
    g.node_page.offset !== q.nodeOffset ||
    g.edge_page.offset !== q.edgeOffset ||
    e.objective !== q.objective ||
    e.routing_context.departure_time !== e.departure_seconds ||
    e.objective_unit !== (q.objective === "distance" ? "meters" : "seconds") ||
    e.evaluator !==
      (q.objective === "distance"
        ? "distance/v1"
        : q.objective === "travel_time"
          ? "free-flow-travel-time/v1"
          : "traffic-aware-travel-time/v1") ||
    e.traffic_digest !==
      (q.objective === "traffic_aware" ? g.traffic_snapshot_digest : null)
  )
    fail();
  for (const [p, total] of [
    [g.node_page, g.node_count],
    [g.edge_page, g.edge_count],
  ] as const) {
    if (
      p.offset > p.end ||
      p.end > total ||
      p.next_offset !== (p.end < total ? p.end : null)
    )
      fail();
  }
  if (
    g.node_page.end !== Math.min(g.node_count, q.nodeOffset + 500) ||
    g.edge_page.end !== q.edgeOffset + g.edges.length ||
    (g.edges.length === 0 && g.edge_page.next_offset !== null)
  )
    fail();
  if (g.nodes.some((n) => Math.abs(n.coordinate_e7.latitude_e7) > 850000000))
    throw new Error("This map does not support polar graph node presentation.");
  const nodes = new Map(g.nodes.map((n) => [n.node, n.coordinate_e7]));
  if (
    nodes.size !== g.nodes.length ||
    g.nodes.some((n) => n.node >= g.node_count)
  )
    fail();
  for (let i = g.node_page.offset; i < g.node_page.end; i++)
    if (!nodes.has(i)) fail();
  const equal = (
    a: z.infer<typeof point> | undefined,
    b: z.infer<typeof point>,
  ) => a?.latitude_e7 === b.latitude_e7 && a.longitude_e7 === b.longitude_e7;
  const close = (a: number, b: number) =>
    Math.abs(a - b) <= 1e-8 * Math.max(1, a, b);
  const allowed = new Set<number>();
  for (let i = g.node_page.offset; i < g.node_page.end; i++) allowed.add(i);
  let points = 0;
  for (const [i, edge] of g.edges.entries()) {
    points += edge.points_e7.length;
    allowed.add(edge.from);
    allowed.add(edge.to);
    if (
      edge.edge !== q.edgeOffset + i ||
      edge.edge >= g.edge_count ||
      !equal(nodes.get(edge.from), edge.points_e7[0]) ||
      !equal(nodes.get(edge.to), edge.points_e7.at(-1)!) ||
      (g.traffic_snapshot_digest === null) !==
        (edge.traffic_multiplier === null)
    )
      fail();
    if (edge.cost_status === "traversable") {
      if (
        edge.objective_cost === null ||
        edge.predicted_elapsed_seconds === null ||
        !close(
          edge.predicted_elapsed_seconds,
          edge.base_seconds *
            (q.objective === "traffic_aware"
              ? (edge.traffic_multiplier ?? 0)
              : 1),
        ) ||
        !close(
          edge.objective_cost,
          q.objective === "distance"
            ? edge.distance_meters
            : edge.predicted_elapsed_seconds,
        )
      )
        fail();
    } else if (
      edge.objective_cost !== null ||
      edge.predicted_elapsed_seconds !== null
    )
      fail();
    if (
      (q.objective === "traffic_aware" &&
        g.traffic_snapshot_digest === null) !==
      (edge.cost_status === "unavailable_no_traffic_snapshot")
    )
      fail();
    if (
      edge.points_e7.some(
        (p, j) =>
          Math.abs(p.latitude_e7) > 850000000 ||
          (j > 0 &&
            Math.abs(p.longitude_e7 - edge.points_e7[j - 1].longitude_e7) >
              1800000000),
      )
    )
      throw new Error(
        "This map does not support polar or antimeridian-crossing graph geometry.",
      );
  }
  if (
    points !== g.point_count ||
    allowed.size !== nodes.size ||
    g.nodes.some((n) => !allowed.has(n.node))
  )
    fail();
  return { ...g, raw };
}
