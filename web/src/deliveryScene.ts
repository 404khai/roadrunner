import { z } from "zod";
import { evidenceSchema, parseExact } from "./contracts";
import { decodeGraph, type GraphView } from "./graphContracts";
import {
  haversine,
  revealPrefix,
  type PresentationPath,
  type Position,
} from "./geometry";
const time = z.number().finite().nonnegative();
const id = z
  .union([
    z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER),
    z.bigint().nonnegative(),
    z.string().regex(/^(0|[1-9]\d*)$/),
  ])
  .transform(String);
const coordinate = z.object({
  latitude: z.number().finite().min(-85).max(85),
  longitude: z.number().finite().min(-180).max(180),
});
const stop = z.union([z.object({ Pickup: id }), z.object({ Dropoff: id })]);
const leg = z.object({
  departed_at: time,
  travel: time,
  distance: time,
  nodes: z.array(z.number().int().nonnegative()),
  edges: z.array(z.number().int().nonnegative()),
  routing: z.object({
    graph_digest: z.string(),
    traffic: z.unknown(),
    profile: z.string(),
  }),
});
const execution = z.object({
  action_id: id,
  generation: id,
  originating_plan: id,
  stop,
  arrival: time,
  service_end: time.nullable(),
  stage: z.enum(["Travelling", "Arrived", "Waiting", "Servicing"]),
  route: leg,
});
const traffic = z.array(
  z.object({
    edge_id: z.number().int().nonnegative(),
    multiplier: z.number().finite().min(1),
  }),
);
const rider = z.object({
  id,
  coordinate,
  capacity: id,
  load: id,
  availability: z.string(),
  assigned: z.array(id),
  stops: z.array(z.object({ kind: z.enum(["pickup", "delivery"]), order: id })),
  plan_revision: id.nullable(),
  execution: execution.nullable(),
});
const order = z.object({
  id,
  pickup: coordinate,
  dropoff: coordinate,
  demand: id,
  status: z.enum(["awaiting_pickup", "onboard", "delivered", "cancelled"]),
  rider: id.nullable(),
  readiness: z.object({
    expected_at: time.nullable(),
    observed_at: time.nullable(),
  }),
});
const schema = z.object({
  schema_version: z.literal(1),
  semantics: z.literal("synthetic-delivery-scene/v1"),
  motion_model: z.literal("causal-uniform-distance/v1"),
  traffic_semantics: z.string().optional(),
  run_identity: z.string(),
  graph_digest: z.string(),
  time_domain: z.literal("simulation_logical_seconds_no_civil_epoch"),
  start: time,
  end: time,
  frames: z
    .array(
      z.object({
        at: time,
        revision: id,
        riders: z.array(rider).max(32),
        orders: z.array(order).max(64),
        traffic: traffic.nullable(),
        traffic_digest: z.string().nullable(),
      }),
    )
    .min(1)
    .max(2001),
  routes: z.array(
    z.object({
      rider: id,
      action: id,
      original: execution,
      traffic,
      geometry: evidenceSchema.shape.geometry.nullable(),
      geometry_error: z.string().nullable(),
    }),
  ),
  schedules: z.array(
    z.object({
      rider: id,
      at: time,
      execution,
      revision: z.object({ number: id }),
    }),
  ),
  effects: z.array(
    z.object({
      action: id,
      rider: id,
      generation: id,
      effect: z.enum(["Arrival", "Completion"]),
      at: time,
    }),
  ),
  dispatch_audit: z
    .object({
      basic: z.array(z.unknown()),
      insertion: z.array(z.unknown()),
      fleet: z.array(z.unknown()),
      recovery: z.array(z.unknown()),
      failures: z.array(z.unknown()).optional(),
    })
    .optional(),
  roads: z.unknown(),
});
export type Scene = z.infer<typeof schema> & {
  roads: GraphView;
  trafficSupported: boolean;
  visualIds: Map<string, string>;
  raw: string;
  paths: Map<string, PresentationPath>;
};
export type SceneRider = z.infer<typeof rider>;
export function actionKey(rider: string, action: string) {
  return `${rider}:${action}`;
}
export function decodeScene(raw: string, digest: string): Scene {
  const s = schema.parse(parseExact(raw));
  if (s.graph_digest !== digest || s.end < s.start)
    throw Error("Scene graph/time mismatch");
  const paths = new Map<string, PresentationPath>();
  const keys = new Set<string>();
  for (const r of s.routes) {
    const k = actionKey(r.rider, r.action),
      g = r.geometry;
    if (r.action !== r.original.action_id || keys.has(k))
      throw Error("Ambiguous action identity");
    keys.add(k);
    if (!g) continue;
    if (
      g.graph_digest !== digest ||
      r.original.route.routing.graph_digest !== digest ||
      g.nodes.join() !== r.original.route.nodes.join() ||
      g.occurrences.length !== r.original.route.edges.length ||
      g.nodes.length !== g.occurrences.length + 1
    )
      throw Error("Frozen geometry identity mismatch");
    let end = 0;
    g.occurrences.forEach((o, i) => {
      if (
        o.ordinal !== i ||
        o.edge !== r.original.route.edges[i] ||
        o.from !== g.nodes[i] ||
        o.to !== g.nodes[i + 1] ||
        o.point_start !== end ||
        o.point_end <= o.point_start + 1 ||
        o.point_end > g.points_e7.length
      )
        throw Error("Invalid directed geometry");
      if (
        i &&
        JSON.stringify(g.points_e7[o.point_start]) !==
          JSON.stringify(g.points_e7[end - 1])
      )
        throw Error("Discontinuous geometry");
      end = o.point_end;
    });
    if (
      (g.no_travel && (g.occurrences.length || g.points_e7.length !== 1)) ||
      (!g.no_travel && end !== g.points_e7.length)
    )
      throw Error("Invalid no-travel geometry");
    const points: Position[] = g.points_e7.map((p) => [
      p.longitude_e7 / 1e7,
      p.latitude_e7 / 1e7,
    ]);
    if (
      points.some(
        (p, i) =>
          Math.abs(p[1]) > 85 ||
          (i > 0 && Math.abs(p[0] - points[i - 1][0]) > 180),
      )
    )
      throw Error("Unsupported polar/seam geometry");
    const cumulative = [0];
    points
      .slice(1)
      .forEach((p, i) =>
        cumulative.push(cumulative[i] + haversine(points[i], p)),
      );
    paths.set(k, { points, cumulative, total: cumulative.at(-1)! });
  }
  let scheduleAt = s.start;
  const generations = new Map<string, bigint>();
  for (const change of s.schedules) {
    const k = actionKey(change.rider, change.execution.action_id);
    const original = s.routes.find(
      (r) =>
        r.rider === change.rider && r.action === change.execution.action_id,
    )?.original;
    if (
      !original ||
      change.at < scheduleAt ||
      BigInt(change.execution.generation) <=
        (generations.get(k) ?? BigInt(original.generation)) ||
      JSON.stringify(original.route) !== JSON.stringify(change.execution.route)
    )
      throw Error("Invalid causal schedule lineage");
    generations.set(k, BigInt(change.execution.generation));
    scheduleAt = change.at;
  }
  let revision = 0n;
  let last = s.start;
  for (const f of s.frames) {
    if (
      f.at < last ||
      f.at > s.end ||
      new Set(f.riders.map((r) => r.id)).size !== f.riders.length
    )
      throw Error("Invalid scene frame ordering/identity");
    if (BigInt(f.revision) < revision)
      throw Error("Reversed operational revision");
    revision = BigInt(f.revision);
    last = f.at;
    for (const r of f.riders)
      if (r.execution) {
        const original = s.routes.find(
          (a) => a.rider === r.id && a.action === r.execution!.action_id,
        )?.original;
        if (
          !original ||
          JSON.stringify(original.route) !== JSON.stringify(r.execution.route)
        )
          throw Error("Missing or changed original frozen action");
      }
  }
  return {
    ...s,
    visualIds: new Map(s.frames[0].riders.map((r, i) => [r.id, String(i + 1)])),
    trafficSupported:
      s.traffic_semantics === "synthetic-static-edge-factors/v1" &&
      s.frames.every((f) => f.traffic !== null && f.traffic_digest !== null),
    roads: decodeGraph(JSON.stringify(s.roads), {
      digest,
      nodeOffset: 0,
      edgeOffset: 0,
      objective: "travel_time",
    }),
    raw,
    paths,
  };
}
// Immutable source actions + ordered interventions. Later delays never rewrite earlier samples.
export function actionProgress(
  scene: Scene,
  rider: string,
  action: string,
  at: number,
) {
  const original = scene.routes.find(
    (r) => r.rider === rider && r.action === action,
  )?.original;
  if (!original) return 0;
  let anchor = original.route.departed_at,
    arrival = original.arrival,
    progress = 0;
  const advance = (t: number) =>
    t < anchor
      ? progress
      : arrival <= anchor
        ? 1
        : Math.min(
            1,
            progress +
              (1 - progress) * Math.max(0, (t - anchor) / (arrival - anchor)),
          );
  for (const s of scene.schedules.filter(
    (s) => s.rider === rider && s.execution.action_id === action,
  )) {
    if (s.at > at) break;
    progress = advance(s.at);
    anchor = s.at;
    arrival = s.execution.arrival;
  }
  return advance(at);
}
export function frameAt(scene: Scene, at: number) {
  let lo = 0,
    hi = scene.frames.length - 1;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (scene.frames[mid].at <= at) lo = mid;
    else hi = mid - 1;
  }
  return scene.frames[lo];
}
export function sampleRider(scene: Scene, r: SceneRider, at: number) {
  const a = r.execution,
    path = a && scene.paths.get(actionKey(r.id, a.action_id));
  if (!a)
    return {
      position: [r.coordinate.longitude, r.coordinate.latitude] as Position,
      bearing: undefined,
      quality: "Simulation endpoint",
      progress: 0,
    };
  if (!path)
    return {
      position: undefined,
      bearing: undefined,
      quality: "Position unavailable · original geometry unsupported",
      progress: 0,
    };
  const progress =
    a.stage === "Travelling" ? actionProgress(scene, r.id, a.action_id, at) : 1;
  const position = revealPrefix(path, progress).head;
  const target = progress * path.total;
  let i = Math.min(1, path.points.length - 1);
  while (i < path.points.length - 1 && path.cumulative[i] <= target) i++;
  while (i > 0 && haversine(path.points[i - 1], path.points[i]) < 0.001) i--;
  const from = path.points[Math.max(0, i - 1)],
    to = path.points[i];
  const rad = Math.PI / 180,
    dl = (to[0] - from[0]) * rad;
  const bearing =
    i > 0
      ? (Math.atan2(
          Math.sin(dl) * Math.cos(to[1] * rad),
          Math.cos(from[1] * rad) * Math.sin(to[1] * rad) -
            Math.sin(from[1] * rad) * Math.cos(to[1] * rad) * Math.cos(dl),
        ) /
          rad +
          360) %
        360
      : undefined;
  return {
    position,
    bearing,
    progress,
    quality:
      a.stage !== "Travelling"
        ? "Accepted simulation endpoint"
        : progress >= 1
          ? "Estimated endpoint · awaiting accepted arrival"
          : "Simulated position · uniform route progress",
  };
}
