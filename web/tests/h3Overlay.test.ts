import { expect, it } from "vitest";
import { h3Overlay } from "../src/h3Overlay";
import type { Anchor, RouteEvidence } from "../src/contracts";
import forward from "./forward.json";

it("matches the same official H3 reference cell as the Rust adapter", () => {
  const anchors: Anchor[] = [
    { node: 7, coordinate: { latitude: 37.7955, longitude: -122.3937 } },
  ];
  const result = h3Overlay(anchors, undefined, 10);
  expect(result.cells).toEqual([
    { cell: "8a283082a677fff", nodes: [7], routeVertices: 0 },
  ]);
  const ring = result.geojson.features[0].geometry.coordinates[0];
  expect(ring[0]).toEqual(ring.at(-1));
  expect(ring[0][0]).toBeLessThan(-120); // GeoJSON longitude first.
});

it("conserves node IDs and route vertex samples at every resolution without changing route evidence", () => {
  const route = forward.routes[0] as RouteEvidence;
  const before = JSON.stringify(route);
  const anchors: Anchor[] = [
    { node: 9, coordinate: { latitude: 6.5244, longitude: 3.3792 } },
    { node: 1, coordinate: { latitude: 6.5244, longitude: 3.3792 } },
  ];
  for (let resolution = 0; resolution <= 15; resolution++) {
    const result = h3Overlay(anchors, route, resolution);
    expect(result.cells.flatMap((c) => c.nodes).sort((a, b) => a - b)).toEqual([
      1, 9,
    ]);
    expect(result.cells.reduce((n, c) => n + c.routeVertices, 0)).toBe(
      route.geometry.points_e7.length,
    );
    expect(result.cells.map((c) => c.cell)).toEqual(
      [...result.cells.map((c) => c.cell)].sort(),
    );
  }
  expect(JSON.stringify(route)).toBe(before);
});

it("rejects invalid resolution and supports empty and polar inputs explicitly", () => {
  expect(() => h3Overlay([], undefined, 16)).toThrow();
  expect(() => h3Overlay([], undefined, 1.2)).toThrow();
  expect(h3Overlay([], undefined, 9).cells).toEqual([]);
  const polar = h3Overlay(
    [{ node: 0, coordinate: { latitude: 90, longitude: 0 } }],
    undefined,
    9,
  );
  expect(polar.cells).toHaveLength(1);
  expect(polar.geojson.features).toHaveLength(0);
  expect(polar.unsupported).toBe(1);
});
