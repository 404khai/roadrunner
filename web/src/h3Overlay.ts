import { cellToBoundary, latLngToCell } from "h3-js";
import type { FeatureCollection, Polygon } from "geojson";
import type { Anchor, RouteEvidence } from "./contracts";

export type H3Bucket = { cell: string; nodes: number[]; routeVertices: number };
export function h3Overlay(
  anchors: Anchor[],
  route: RouteEvidence | undefined,
  resolution: number,
) {
  if (!Number.isInteger(resolution) || resolution < 0 || resolution > 15)
    throw new Error("H3 resolution must be in 0..=15");
  const buckets = new Map<string, H3Bucket>();
  function bucket(latitude: number, longitude: number) {
    const cell = latLngToCell(latitude, longitude, resolution);
    let value = buckets.get(cell);
    if (!value) {
      value = { cell, nodes: [], routeVertices: 0 };
      buckets.set(cell, value);
    }
    return value;
  }
  for (const a of anchors)
    bucket(a.coordinate.latitude, a.coordinate.longitude).nodes.push(a.node);
  for (const point of route?.geometry.points_e7 ?? [])
    bucket(point.latitude_e7 / 1e7, point.longitude_e7 / 1e7).routeVertices++;
  const cells = [...buckets.values()].sort((a, b) =>
    a.cell.localeCompare(b.cell),
  );
  for (const cell of cells) cell.nodes.sort((a, b) => a - b);
  return {
    cells,
    ...cellPolygons(
      cells.map((value) => ({
        cell: value.cell,
        properties: {
          nodes: value.nodes.length,
          route_vertices: value.routeVertices,
        },
      })),
    ),
  };
}

/** Display-only buckets of retained scene locations; no entity or graph-node IDs are invented. */
export function h3LocationOverlay(
  positions: readonly (readonly [number, number])[],
  resolution: number,
) {
  if (!Number.isInteger(resolution) || resolution < 0 || resolution > 15)
    throw new Error("H3 resolution must be in 0..=15");
  const cells = [
    ...new Set(
      positions.map(([longitude, latitude]) =>
        latLngToCell(latitude, longitude, resolution),
      ),
    ),
  ].sort();
  return {
    cells,
    ...cellPolygons(cells.map((cell) => ({ cell, properties: {} }))),
  };
}

function cellPolygons(
  cells: { cell: string; properties: Record<string, number> }[],
) {
  const geojson: FeatureCollection<Polygon> = {
    type: "FeatureCollection",
    features: [],
  };
  let unsupported = 0;
  for (const value of cells.slice(0, 1000)) {
    const ring = cellToBoundary(value.cell, true);
    if (
      ring.some(
        (p, i) =>
          Math.abs(p[1]) > 85 ||
          (i > 0 && Math.abs(p[0] - ring[i - 1][0]) > 180),
      )
    ) {
      unsupported++;
      continue;
    }
    geojson.features.push({
      type: "Feature",
      properties: {
        cell: value.cell,
        ...value.properties,
      },
      geometry: { type: "Polygon", coordinates: [ring] },
    });
  }
  return { geojson, truncated: cells.length > 1000, unsupported };
}
