import { z } from "zod";
import { parseExact, evidenceSchema } from "./contracts";
import { haversine, type Position, type PresentationPath } from "./geometry";
const number = z.number().finite().nonnegative();
const coordinate = z.object({
  latitude: z.number().finite().min(-85).max(85),
  longitude: z.number().finite().min(-180).max(180),
});
const request = z.object({
  graph_digest: z.string(),
  pickup_node: number,
  customer_node: number,
  riders: z
    .array(z.object({ node: number, capacity: number, available: z.boolean() }))
    .max(16),
  radius_meters: number.nullable(),
  limit: number,
  ready_in_seconds: number,
  preparation_aware: z.boolean(),
  publish_demo: z.boolean(),
});
const leg = z.object({
  geometry: evidenceSchema.shape.geometry.nullable(),
  unavailable: z.string().nullable(),
  departure_seconds: number.optional(),
  routing: z.object({ graph_digest: z.string() }).passthrough().optional(),
});
const candidate = z.object({
  rider: z.string(),
  feasible: z.boolean(),
  rejection: z.string().optional(),
  pickup_seconds: number.optional(),
  delivery_seconds: number.optional(),
  waiting_seconds: number.optional(),
  score_seconds: number.optional(),
  completion_seconds: number.optional(),
  legs: z.array(leg).max(2),
});
const schema = z.object({
  schema_version: z.literal(1),
  semantics: z.literal("isolated-dispatch-evaluation/v1"),
  evaluation_id: z.string(),
  graph_digest: z.string(),
  evaluated_at: number,
  time_domain: z.literal("simulation_logical_seconds_no_civil_epoch"),
  pickup: coordinate,
  customer: coordinate,
  riders: z
    .array(
      z.object({
        id: z.string(),
        coordinate,
        capacity: number,
        available: z.boolean(),
      }),
    )
    .max(16),
  candidates: z.array(candidate).max(16),
  selected: z.string().nullable(),
  committed: z.boolean(),
  active_execution: z.literal(false),
  decision: z
    .object({
      evidence: z
        .object({
          strategy: z.string(),
          eligibility_policy: z.string(),
          coverage: z.enum(["Complete", "PotentiallyIncomplete"]),
          candidate_policy: z.union([
            z.literal("Exhaustive"),
            z.object({ Spatial: z.object({ radius: number, limit: number }) }),
          ]),
        })
        .passthrough(),
    })
    .passthrough(),
  explanation: z.string(),
  request,
  publication_scope: z.literal("isolated_demo_world_only"),
  geometry_source: z.literal("captured_during_original_evaluation"),
  eligible_population: z.string().min(1),
  distance_model: z.literal("haversine_metres_inclusive_boundary"),
  discovery_order_retained: z.literal(false),
});
export type DispatchRequest = z.infer<typeof request>;
export type DispatchEvidence = z.infer<typeof schema> & { raw: string };
export function decodeDispatch(
  raw: string,
  expected: DispatchRequest,
): DispatchEvidence {
  const result = schema.parse(parseExact(raw));
  if (
    JSON.stringify(result.request) !==
      JSON.stringify(request.parse(expected)) ||
    result.graph_digest !== expected.graph_digest
  )
    throw Error("Dispatch request/graph association mismatch");
  if (
    result.eligible_population !== result.decision.evidence.eligibility_policy
  )
    throw Error("Eligibility association mismatch");
  const policy = result.decision.evidence.candidate_policy;
  if (
    expected.radius_meters === null
      ? policy !== "Exhaustive"
      : typeof policy !== "object" ||
        policy.Spatial.radius !== expected.radius_meters ||
        policy.Spatial.limit !== expected.limit
  )
    throw Error("Unsupported spatial association");
  if (
    result.decision.evidence.coverage !==
    (expected.radius_meters === null ? "Complete" : "PotentiallyIncomplete")
  )
    throw Error("Unsupported coverage claim");
  const ids = new Set(result.riders.map((r) => r.id)),
    candidates = new Set(result.candidates.map((c) => c.rider));
  if (
    ids.size !== result.riders.length ||
    candidates.size !== result.candidates.length ||
    result.candidates.some((c) => !ids.has(c.rider)) ||
    (result.selected !== null &&
      !result.candidates.some(
        (c) => c.rider === result.selected && c.feasible,
      )) ||
    (result.committed && !result.selected)
  )
    throw Error("Invalid candidate identity/publication");
  for (const c of result.candidates) {
    if (
      c.feasible &&
      [
        c.pickup_seconds,
        c.delivery_seconds,
        c.waiting_seconds,
        c.score_seconds,
      ].some((v) => v === undefined)
    )
      throw Error("Missing candidate metrics");
    for (const l of c.legs)
      if (
        l.geometry &&
        (l.routing?.graph_digest !== result.graph_digest ||
          l.geometry.graph_digest !== result.graph_digest)
      )
        throw Error("Hypothetical route graph mismatch");
  }
  return { ...result, raw };
}
export function legPath(
  geometry: z.infer<typeof evidenceSchema.shape.geometry>,
): PresentationPath {
  const points: Position[] = geometry.points_e7.map((p) => [
      p.longitude_e7 / 1e7,
      p.latitude_e7 / 1e7,
    ]),
    cumulative = [0];
  for (let i = 1; i < points.length; i++)
    cumulative.push(cumulative[i - 1] + haversine(points[i - 1], points[i]));
  return { points, cumulative, total: cumulative.at(-1)! };
}
// Spherical destination formula matches the producer's Haversine distance model.
export function radiusRing(center: Position, meters: number): Position[] {
  const lat = (center[1] * Math.PI) / 180,
    lon = (center[0] * Math.PI) / 180,
    d = meters / 6371008.8;
  return Array.from({ length: 97 }, (_, i) => {
    const b = (i / 96) * Math.PI * 2,
      y = Math.asin(
        Math.sin(lat) * Math.cos(d) + Math.cos(lat) * Math.sin(d) * Math.cos(b),
      ),
      x =
        lon +
        Math.atan2(
          Math.sin(b) * Math.sin(d) * Math.cos(lat),
          Math.cos(d) - Math.sin(lat) * Math.sin(y),
        );
    return [(x * 180) / Math.PI, (y * 180) / Math.PI];
  });
}
