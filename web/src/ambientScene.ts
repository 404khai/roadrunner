import type { Scene } from "./deliveryScene";
import {
  haversine,
  revealPrefix,
  type Position,
  type PresentationPath,
} from "./geometry";
export type VehicleKind = "car" | "truck" | "van" | "motorcycle";
export type AmbientActor = {
  id: string;
  kind: VehicleKind;
  path: PresentationPath;
  speed: number;
  phase: number;
  color: string;
};
export type VehiclePose = {
  id: string;
  kind: VehicleKind | "rider";
  position: Position;
  bearing: number;
  distance: number;
  color: string;
  visualId?: string;
};
export const AMBIENT_LIMIT = 48;
// Explicit renderer-only scene configuration. Graph geometry is a cosmetic trajectory,
// never evidence of car/truck access, legality, collision avoidance or congestion.
export function ambientActors(scene: Scene, count: number): AmbientActor[] {
  const roads = scene.roads.edges.filter((e) => e.points_e7.length > 1);
  return Array.from(
    {
      length: Math.min(
        AMBIENT_LIMIT,
        Math.max(0, Math.floor(count)),
        roads.length ? AMBIENT_LIMIT : 0,
      ),
    },
    (_, i) => {
      const edge = roads[(i * 17 + 7) % roads.length];
      const points: Position[] = edge.points_e7.map((p) => [
        p.longitude_e7 / 1e7,
        p.latitude_e7 / 1e7,
      ]);
      const cumulative = [0];
      for (let j = 1; j < points.length; j++)
        cumulative.push(
          cumulative[j - 1] + haversine(points[j - 1], points[j]),
        );
      return {
        id: `ambient:${scene.run_identity}:${i}`,
        kind: (["car", "truck", "van", "motorcycle"] as const)[i % 4],
        path: { points, cumulative, total: cumulative.at(-1)! },
        speed: 3 + (i % 5),
        phase: ((i * 37) % 101) / 101,
        color: ["#b5b7b0", "#998575", "#8c9b93", "#a694ad"][i % 4],
      };
    },
  );
}
export function sampleAmbient(
  actor: AmbientActor,
  elapsed: number,
): VehiclePose {
  const distance =
    Math.max(0, elapsed) * actor.speed + actor.phase * actor.path.total;
  // Declared periodic trajectory: disappear at end, reappear at start. No U-turn claim.
  const p = actor.path.total
    ? (distance % actor.path.total) / actor.path.total
    : 1;
  const head = revealPrefix(actor.path, p).head;
  const index = Math.max(
    1,
    actor.path.cumulative.findIndex((d) => d > p * actor.path.total),
  );
  const a = actor.path.points[index - 1],
    b = actor.path.points[index];
  const rad = Math.PI / 180;
  const bearing =
    Math.atan2(
      Math.sin((b[0] - a[0]) * rad) * Math.cos(b[1] * rad),
      Math.cos(a[1] * rad) * Math.sin(b[1] * rad) -
        Math.sin(a[1] * rad) *
          Math.cos(b[1] * rad) *
          Math.cos((b[0] - a[0]) * rad),
    ) / rad;
  return {
    id: actor.id,
    kind: actor.kind,
    position: head,
    bearing,
    distance,
    color: actor.color,
  };
}
function buildAmbientSprite(
  kind: VehicleKind,
  color: string,
  heading?: number,
) {
  if (heading !== undefined) {
    const a = (Math.round(heading / 45) * Math.PI) / 4,
      l =
        kind === "truck" ? 38 : kind === "van" ? 31 : kind === "car" ? 26 : 21,
      w = kind === "motorcycle" ? 7 : 15,
      h = kind === "truck" ? 19 : kind === "van" ? 17 : 10;
    const project = (x: number, y: number, z: number) =>
      `${32 + x * Math.cos(a) + y * Math.sin(a)},${45 + (x * Math.sin(a) - y * Math.cos(a)) * 0.42 - z}`;
    const face = (ps: number[][], fill: string) =>
      `<polygon points="${ps.map((p) => project(p[0], p[1], p[2])).join(" ")}" fill="${fill}" stroke="#383838" stroke-width="1"/>`;
    const x = w / 2,
      y = l / 2;
    return `data:image/svg+xml,${encodeURIComponent(
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><ellipse cx="32" cy="47" rx="22" ry="7" fill="#151515" opacity=".3"/>${face(
        [
          [-x, -y, 0],
          [x, -y, 0],
          [x, -y, h],
          [-x, -y, h],
        ],
        color,
      )}${face(
        [
          [-x, y, 0],
          [x, y, 0],
          [x, y, h],
          [-x, y, h],
        ],
        color,
      )}${face(
        [
          [-x, -y, 0],
          [-x, y, 0],
          [-x, y, h],
          [-x, -y, h],
        ],
        color,
      )}${face(
        [
          [x, -y, 0],
          [x, y, 0],
          [x, y, h],
          [x, -y, h],
        ],
        color,
      )}${face(
        [
          [-x, -y, h],
          [x, -y, h],
          [x, y, h],
          [-x, y, h],
        ],
        color,
      )}${face(
        [
          [-x * 0.8, y * 0.3, h + 0.1],
          [x * 0.8, y * 0.3, h + 0.1],
          [x * 0.8, y * 0.8, h + 0.1],
          [-x * 0.8, y * 0.8, h + 0.1],
        ],
        "#405358",
      )}</svg>`,
    )}`;
  }
  const length =
    kind === "truck" ? 48 : kind === "van" ? 40 : kind === "car" ? 32 : 26;
  const width = kind === "motorcycle" ? 10 : 20;
  return `data:image/svg+xml,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 54"><rect x="${16 - width / 2}" y="${27 - length / 2}" width="${width}" height="${length}" rx="4" fill="${color}" stroke="#282828" stroke-width="2"/><path d="M${18 - width / 2} ${29 - length / 2}h${width - 4}v8H${18 - width / 2}z" fill="#3d4749"/><path d="M${18 - width / 2} ${23 + length / 2}h${width - 4}" stroke="#d38373" stroke-width="3"/></svg>`)}`;
}

const spriteCache = new Map<string, string>();
export function ambientSprite(
  kind: VehicleKind,
  color: string,
  heading?: number,
) {
  const direction =
    heading === undefined
      ? undefined
      : (((Math.round(heading / 45) * 45) % 360) + 360) % 360;
  const key = `${kind}:${color}:${direction}`;
  const cached = spriteCache.get(key);
  if (cached) return cached;
  const asset = buildAmbientSprite(kind, color, direction);
  if (spriteCache.size < 256) spriteCache.set(key, asset);
  return asset;
}
