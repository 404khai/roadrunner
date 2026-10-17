import { parse } from "lossless-json";
import { z } from "zod";

const nonnegative = z.number().finite().nonnegative();
const index = z.number().int().min(0).max(0xffffffff);
const point = z
  .object({
    latitude_e7: z.number().int().min(-900000000).max(900000000),
    longitude_e7: z.number().int().min(-1800000000).max(1800000000),
  })
  .passthrough();
const provenance = z
  .object({
    evaluator: z.string(),
    traffic_digest: z.string().nullable(),
    departure_seconds: nonnegative,
    time_domain: z.string(),
    heuristic: z.string().nullable(),
    routing_context: z
      .object({
        departure_time: nonnegative,
        private_access: z.boolean(),
        permit_access: z.boolean(),
      })
      .passthrough(),
  })
  .passthrough();
export const evidenceSchema = z
  .object({
    semantics: z
      .object({
        schema_version: z.literal(1),
        geometry: z.literal("wgs84_e7"),
        authority: z.literal("deterministic_prediction"),
      })
      .passthrough(),
    geometry: z
      .object({
        schema_version: z.literal(1),
        encoding: z.literal("wgs84_e7"),
        graph_digest: z.string().regex(/^[a-f0-9]{64}$/),
        path_id: z.string().regex(/^path-v1:[a-f0-9]{64}$/),
        nodes: z.array(index).min(1),
        points_e7: z.array(point).min(1).max(200000),
        occurrences: z
          .array(
            z
              .object({
                ordinal: index,
                edge: index,
                segment: index,
                from: index,
                to: index,
                point_start: index,
                point_end: index,
                orientation: z.enum(["forward", "reverse"]),
                distance_meters: nonnegative,
              })
              .passthrough(),
          )
          .max(20000),
        no_travel: z.boolean(),
      })
      .passthrough(),
    evaluation_id: z.string().regex(/^evaluation-v1:[a-f0-9]{64}$/),
    provenance,
    routing_profile: z.string(),
    jurisdiction_policy: z.string(),
    distance_meters: nonnegative,
    predicted_elapsed_seconds: nonnegative,
    objective: z
      .object({ kind: z.enum(["distance", "travel_time"]), value: nonnegative })
      .passthrough(),
    objective_unit: z.enum(["meters", "seconds"]),
    cost_breakdown: z.string(),
    search: z
      .object({
        algorithm: z.enum(["dijkstra", "a_star", "yen"]),
        state_mode: z.enum([
          "node",
          "node_incoming_edge_with_source_sentinel",
          "loopless_path",
        ]),
        expanded_states: z.union([
          z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
          z.bigint().nonnegative(),
        ]),
        counter_semantics: z.string(),
        relaxation: z.string(),
        termination: z.literal("route_found"),
        detailed_trace: z.string(),
      })
      .passthrough(),
    optional_capabilities: z.array(z.string()).optional(),
  })
  .passthrough();
export type RouteEvidence = z.infer<typeof evidenceSchema>;
export const anchorSchema = z.object({
  node: index,
  coordinate: z.object({
    latitude: z.number().min(-90).max(90),
    longitude: z.number().min(-180).max(180),
  }),
});
export type Anchor = z.infer<typeof anchorSchema>;
export const catalogSchema = z.object({
  schema_version: z.literal(1),
  selection_semantics: z.literal("explicit_graph_node_no_snapping/v1"),
  graph: z
    .object({
      snapshot_digest: z.string().regex(/^[a-f0-9]{64}$/),
      routing_profile: z.string(),
      jurisdiction_policy: z.string(),
      turn_restrictions_enforced: z.boolean(),
      build_identity: z.object({ source_dataset: z.string() }).passthrough(),
    })
    .passthrough(),
  node_count: index,
  edge_count: index,
  offset: index,
  next_offset: index.nullable(),
  anchors: z.array(anchorSchema).max(500),
});
export type Catalog = z.infer<typeof catalogSchema>;
export type RouteRequest = {
  origin: Anchor & { graph_digest: string };
  destination: Anchor & { graph_digest: string };
  algorithm: "astar" | "dijkstra";
  objective: "distance" | "travel_time" | "traffic_aware";
  departure_time?: string;
  alternatives?: number;
};
export type RouteResult = {
  evidence: RouteEvidence;
  raw: string;
  request: RouteRequest;
};

// Preserve large JSON integers before native Number can round them. E7 and indices fit exactly.
export function parseExact(raw: string): unknown {
  return parse(raw, null, (v: string) =>
    /^-?\d+$/.test(v) && !Number.isSafeInteger(Number(v))
      ? BigInt(v)
      : Number(v),
  );
}
export function decodeRoute(raw: string, request: RouteRequest): RouteResult {
  const envelope = z
    .object({
      schema_version: z.literal(1),
      routes: z.array(evidenceSchema).length(1),
      provenance,
    })
    .parse(parseExact(raw));
  validateRoute(envelope.routes[0], envelope.provenance, request);
  return { evidence: envelope.routes[0], raw, request };
}
function validateRoute(
  r: RouteEvidence,
  shared: z.infer<typeof provenance>,
  request: RouteRequest,
  alternative = false,
) {
  const g = r.geometry,
    p = g.points_e7,
    o = g.occurrences;
  const fail = () => {
    throw new Error(
      "Route evidence is inconsistent with its directed path or request.",
    );
  };
  if (
    g.graph_digest !== request.origin.graph_digest ||
    g.graph_digest !== request.destination.graph_digest ||
    g.nodes[0] !== request.origin.node ||
    g.nodes.at(-1) !== request.destination.node
  )
    fail();
  if (
    r.objective_unit !==
      (r.objective.kind === "distance" ? "meters" : "seconds") ||
    r.objective.kind !==
      (request.objective === "distance" ? "distance" : "travel_time")
  )
    fail();
  if (
    r.search.algorithm !==
    (alternative
      ? "yen"
      : request.algorithm === "astar"
        ? "a_star"
        : "dijkstra")
  )
    fail();
  const expectedEvaluator =
    request.objective === "distance"
      ? "distance/v1"
      : request.objective === "travel_time"
        ? "free-flow-travel-time/v1"
        : "traffic-aware-travel-time/v1";
  if (r.provenance.evaluator !== expectedEvaluator) fail();
  if (
    JSON.stringify(r.provenance) !== JSON.stringify(shared) ||
    r.provenance.departure_seconds !==
      r.provenance.routing_context.departure_time
  )
    fail();
  if (
    request.departure_time &&
    r.provenance.departure_seconds !== Date.parse(request.departure_time) / 1000
  )
    fail();
  const equals = (a: (typeof p)[number], b: (typeof p)[number]) =>
    a.latitude_e7 === b.latitude_e7 && a.longitude_e7 === b.longitude_e7;
  if (g.no_travel) {
    if (
      o.length ||
      p.length !== 1 ||
      g.nodes.length !== 1 ||
      request.origin.node !== request.destination.node ||
      r.distance_meters !== 0 ||
      r.predicted_elapsed_seconds !== 0 ||
      r.objective.value !== 0
    )
      fail();
  } else {
    if (!o.length || g.nodes.length !== o.length + 1) fail();
    let end = 0;
    o.forEach((occ, i) => {
      if (
        occ.ordinal !== i ||
        occ.from !== g.nodes[i] ||
        occ.to !== g.nodes[i + 1] ||
        occ.point_start !== end ||
        occ.point_end <= end + 1 ||
        occ.point_end > p.length ||
        occ.distance_meters <= 0
      )
        fail();
      if (i && !equals(p[end - 1], p[end])) fail();
      end = occ.point_end;
    });
    if (end !== p.length) fail();
    if (
      Math.abs(
        o.reduce((sum, occ) => sum + occ.distance_meters, 0) -
          r.distance_meters,
      ) > 1e-8
    )
      fail();
  }
  const start = p[0],
    finish = p.at(-1)!;
  for (const [a, b] of [
    [start, request.origin],
    [finish, request.destination],
  ] as const) {
    if (
      a.latitude_e7 !== Math.round(b.coordinate.latitude * 1e7) ||
      a.longitude_e7 !== Math.round(b.coordinate.longitude * 1e7)
    )
      fail();
  }
  if (
    p.some(
      (x, i) =>
        Math.abs(x.latitude_e7) > 850000000 ||
        (i > 0 &&
          Math.abs(x.longitude_e7 - p[i - 1].longitude_e7) > 1800000000),
    )
  )
    throw new Error(
      "This map does not support polar or antimeridian-crossing route presentation. The supplied geometry cannot be rendered here.",
    );
  if (
    alternative &&
    (r.search.state_mode !== "loopless_path" || r.provenance.heuristic !== null)
  )
    fail();
  if (
    request.objective === "traffic_aware"
      ? !r.provenance.traffic_digest
      : r.provenance.traffic_digest !== null
  )
    fail();
}
const exactCount = z.union([
  z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
  z.bigint().nonnegative(),
]);
const inspectionSchema = z.object({
  schema_version: z.literal(1),
  semantics: z.literal("pinned_static_path_evaluation/v1"),
  evaluation_id: z.string(),
  graph_digest: z.string(),
  traffic_digest: z.string().nullable(),
  base_seconds: nonnegative,
  traffic_delay_seconds: nonnegative.nullable(),
  segments: z
    .array(
      z.object({
        ordinal: index,
        edge: index,
        objective_cost: nonnegative,
        predicted_elapsed_seconds: nonnegative,
        base_seconds: nonnegative,
        traffic_factor: z.number().finite().min(1).nullable(),
      }),
    )
    .max(20000),
});
export type SegmentInspection = z.infer<typeof inspectionSchema>;
const alternativesSchema = z.object({
  schema_version: z.literal(1),
  result_set_id: z.string().regex(/^alternatives-v1:[a-f0-9]{64}$/),
  provenance,
  routes: z
    .array(
      z.object({
        member_id: z.string(),
        rank: z.number().int().min(1).max(5),
        max_shared_distance_ratio: z.number().min(0).max(1),
        route: evidenceSchema,
        segment_inspection: z.unknown().optional(),
      }),
    )
    .min(1)
    .max(5),
  summary: z.object({
    termination: z.enum([
      "requested_count",
      "exhausted",
      "cost_limit",
      "budget",
    ]),
    truncated: z.boolean(),
    options: z.object({
      max_routes: z.number().int().min(1).max(5),
      max_shared_distance_ratio: z.number().min(0).max(1),
      max_cost_factor: z.number().finite().min(1),
      max_search_states: exactCount,
      max_candidate_paths: exactCount,
    }),
    ranked_paths: exactCount,
    search_states: exactCount,
    work_unit: z.literal("loopless_path_pops_across_root_and_spur_searches"),
    scope: z.literal(
      "selected_diverse_legal_loopless_paths_static_nonnegative;not_all_candidates_or_legal_walks",
    ),
    overlap: z.literal("shared_physical_segment_distance_over_shorter_route"),
    candidate_history: z.literal("not_retained"),
  }),
});
export type AlternativeMember = {
  member_id: string;
  rank: number;
  max_shared_distance_ratio: number;
  route: RouteEvidence;
  inspection?: SegmentInspection;
};
export type ComparisonResult = {
  raw: string;
  request: RouteRequest;
  result_set_id: string;
  members: AlternativeMember[];
  summary: z.infer<typeof alternativesSchema>["summary"];
};
export function decodeAlternatives(
  raw: string,
  request: RouteRequest,
): ComparisonResult {
  const e = alternativesSchema.parse(parseExact(raw));
  const fail = () => {
    throw new Error(
      "Alternative evidence is inconsistent with its result set or evaluation context.",
    );
  };
  if (
    request.algorithm !== "dijkstra" ||
    e.routes.length > (request.alternatives ?? 3) ||
    e.summary.options.max_routes !== (request.alternatives ?? 3) ||
    e.summary.truncated !== (e.summary.termination === "budget") ||
    (e.summary.termination === "requested_count" &&
      e.routes.length !== e.summary.options.max_routes)
  )
    fail();
  const identities = new Set<string>();
  const close = (a: number, b: number) =>
    Math.abs(a - b) <= 1e-8 * Math.max(1, Math.abs(a), Math.abs(b));
  const members = e.routes.map((m, i): AlternativeMember => {
    validateRoute(m.route, e.provenance, request, true);
    if (
      m.rank !== i + 1 ||
      m.member_id !== `${e.result_set_id}/${m.route.evaluation_id}` ||
      identities.has(m.route.geometry.path_id) ||
      (i > 0 &&
        (m.route.objective.value < e.routes[i - 1].route.objective.value ||
          m.route.objective.value >
            e.routes[0].route.objective.value *
              e.summary.options.max_cost_factor ||
          m.max_shared_distance_ratio >
            e.summary.options.max_shared_distance_ratio))
    )
      fail();
    if (
      m.route.routing_profile !== e.routes[0].route.routing_profile ||
      m.route.jurisdiction_policy !== e.routes[0].route.jurisdiction_policy
    )
      fail();
    identities.add(m.route.geometry.path_id);
    // Optional, versioned diagnostics degrade to unavailable on older/unknown extensions.
    const parsed = inspectionSchema.safeParse(m.segment_inspection);
    const inspection = parsed.success ? parsed.data : undefined;
    if (inspection) {
      const d = inspection,
        r = m.route;
      if (
        d.evaluation_id !== r.evaluation_id ||
        d.graph_digest !== r.geometry.graph_digest ||
        d.traffic_digest !== r.provenance.traffic_digest ||
        d.segments.length !== r.geometry.occurrences.length
      )
        fail();
      d.segments.forEach((s, j) => {
        if (
          s.ordinal !== j ||
          s.edge !== r.geometry.occurrences[j].edge ||
          (d.traffic_digest
            ? s.traffic_factor === null
            : s.traffic_factor !== null) ||
          !close(
            s.objective_cost,
            r.objective.kind === "distance"
              ? r.geometry.occurrences[j].distance_meters
              : s.predicted_elapsed_seconds,
          ) ||
          !close(
            s.predicted_elapsed_seconds,
            s.base_seconds * (s.traffic_factor ?? 1),
          )
        )
          fail();
      });
      const sum = (
        key: "objective_cost" | "predicted_elapsed_seconds" | "base_seconds",
      ) => d.segments.reduce((n, s) => n + s[key], 0);
      if (
        !close(sum("objective_cost"), r.objective.value) ||
        !close(sum("predicted_elapsed_seconds"), r.predicted_elapsed_seconds) ||
        !close(sum("base_seconds"), d.base_seconds) ||
        (d.traffic_digest
          ? d.traffic_delay_seconds === null ||
            !close(
              d.traffic_delay_seconds,
              r.predicted_elapsed_seconds - d.base_seconds,
            )
          : d.traffic_delay_seconds !== null)
      )
        fail();
    }
    return {
      member_id: m.member_id,
      rank: m.rank,
      max_shared_distance_ratio: m.max_shared_distance_ratio,
      route: m.route,
      inspection,
    };
  });
  return {
    raw,
    request,
    result_set_id: e.result_set_id,
    members,
    summary: e.summary,
  };
}
export class ApiError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
  }
}
export async function apiText(
  path: string,
  token: string,
  signal: AbortSignal,
  request?: object,
) {
  let response: Response;
  try {
    response = await fetch(path, {
      signal,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(request ? { "Content-Type": "application/json" } : {}),
      },
      ...(request ? { method: "POST", body: JSON.stringify(request) } : {}),
    });
  } catch (e) {
    if (signal.aborted) throw e;
    throw new ApiError(
      "connection_unavailable",
      "Roadrunner is unavailable. Check that the API is running and the development proxy points to it.",
    );
  }
  const text = await response.text();
  if (!response.ok) {
    let body: { code?: string; message?: string } = {};
    try {
      body = JSON.parse(text);
    } catch {
      /* Proxy may return a non-JSON failure. */
    }
    throw new ApiError(
      body.code || `http_${response.status}`,
      body.message || `Roadrunner returned HTTP ${response.status}.`,
    );
  }
  return text;
}
