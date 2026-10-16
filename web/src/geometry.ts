import type { RouteEvidence } from "./contracts";
export type Position = [number, number];
export type PresentationPath = {
  points: Position[];
  cumulative: number[];
  total: number;
};
export function haversine(a: Position, b: Position) {
  const rad = Math.PI / 180,
    dlat = (b[1] - a[1]) * rad,
    dlon = (b[0] - a[0]) * rad;
  const h =
    Math.sin(dlat / 2) ** 2 +
    Math.cos(a[1] * rad) * Math.cos(b[1] * rad) * Math.sin(dlon / 2) ** 2;
  return (
    6371008.8 * 2 * Math.atan2(Math.sqrt(h), Math.sqrt(Math.max(0, 1 - h)))
  );
}
export function presentationPath(route: RouteEvidence): PresentationPath {
  const points: Position[] = route.geometry.points_e7.map((p) => [
    p.longitude_e7 / 1e7,
    p.latitude_e7 / 1e7,
  ]);
  const cumulative = [0];
  for (let i = 1; i < points.length; i++)
    cumulative.push(cumulative[i - 1] + haversine(points[i - 1], points[i]));
  return { points, cumulative, total: cumulative.at(-1)! };
}
// A display-only prefix. Never modifies, normalizes or reorders the source evidence.
export function revealPrefix(
  path: PresentationPath,
  progress: number,
): { line: Position[]; head: Position } {
  const { points, cumulative, total } = path;
  if (progress >= 1 || total === 0)
    return { line: points.slice(), head: points.at(-1)! };
  const target = Math.max(0, progress) * total;
  let lo = 1,
    hi = cumulative.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (cumulative[mid] <= target) lo = mid + 1;
    else hi = mid;
  }
  const i = lo,
    span = cumulative[i] - cumulative[i - 1],
    t = span ? (target - cumulative[i - 1]) / span : 0;
  const head: Position = [
    points[i - 1][0] + (points[i][0] - points[i - 1][0]) * t,
    points[i - 1][1] + (points[i][1] - points[i - 1][1]) * t,
  ];
  return { line: [...points.slice(0, i), head], head };
}
export function distanceLabel(meters: number) {
  return meters >= 1000
    ? `${(meters / 1000).toFixed(2)} km`
    : `${Math.round(meters)} m`;
}
export function durationLabel(seconds: number) {
  if (seconds === 0) return "0 sec";
  if (seconds < 60) return `${Math.round(seconds)} sec`;
  return `${(seconds / 60).toFixed(1)} min`;
}
