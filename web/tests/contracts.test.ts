import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import { decodeRoute, parseExact, type RouteRequest } from "../src/contracts";
import {
  presentationPath,
  revealPrefix,
  distanceLabel,
  durationLabel,
} from "../src/geometry";
const file = (name: string) =>
  readFileSync(new URL(`./${name}.json`, import.meta.url), "utf8");
const query = JSON.parse(file("forward-request")) as RouteRequest;
const raw = file("forward");
function mutate(f: (r: ReturnType<typeof JSON.parse>) => void) {
  const r = JSON.parse(raw);
  f(r);
  return JSON.stringify(r);
}
describe("actual Roadrunner Phase 26 client consumption", () => {
  it("preserves complete directed curves, boundaries, provenance and source bytes", () => {
    const result = decodeRoute(raw, query);
    const r = result.evidence;
    expect(result.raw).toBe(raw);
    expect(r.distance_meters).toBe(212.33597101394872);
    expect(r.predicted_elapsed_seconds).toBe(38.22047478251078);
    expect(r.geometry.points_e7).toHaveLength(12);
    expect(r.geometry.occurrences).toHaveLength(4);
    expect(r.provenance.departure_seconds).toBe(1792152000);
    expect(r.geometry.nodes).toEqual([0, 9, 7, 21, 35]);
    expect(presentationPath(r).points).toEqual(
      r.geometry.points_e7.map((p) => [
        p.longitude_e7 / 1e7,
        p.latitude_e7 / 1e7,
      ]),
    );
  });
  it("honors reverse traversal without reversing already directed geometry again", () => {
    const forward = decodeRoute(raw, query).evidence;
    const reverse = decodeRoute(
      file("reverse"),
      JSON.parse(file("reverse-request")),
    ).evidence;
    expect(reverse.geometry.points_e7).toEqual(
      forward.geometry.points_e7.slice().reverse(),
    );
    expect(presentationPath(reverse).points).toEqual(
      presentationPath(forward).points.slice().reverse(),
    );
  });
  it("accepts no-travel as a single point with zero traversals", () => {
    const r = decodeRoute(
      file("no-travel"),
      JSON.parse(file("no-travel-request")),
    ).evidence;
    expect(r.geometry.no_travel).toBe(true);
    expect(r.geometry.occurrences).toEqual([]);
    expect(presentationPath(r).total).toBe(0);
    expect(revealPrefix(presentationPath(r), 0.5).line).toHaveLength(1);
  });
  it("rejects required version, geometry, authority, objective and graph mismatches", () => {
    for (const change of [
      (r) => (r.schema_version = 2),
      (r) => (r.routes[0].semantics.authority = "physical"),
      (r) => (r.routes[0].geometry.encoding = "xyz"),
      (r) =>
        (r.routes[0].objective_unit =
          "seconds" === r.routes[0].objective_unit ? "meters" : "seconds"),
      (r) => (r.routes[0].geometry.graph_digest = "0".repeat(64)),
      (r) => (r.routes[0].semantics.schema_version = 2),
    ] as ((r: ReturnType<typeof JSON.parse>) => void)[])
      expect(() => decodeRoute(mutate(change), query)).toThrow();
  });
  it("rejects geometry gaps, boundary corruption, wrong anchors and truncated results", () => {
    for (const change of [
      (r) => (r.routes[0].geometry.occurrences[1].point_start = 0),
      (r) => (r.routes[0].geometry.points_e7[3].latitude_e7 += 100),
      (r) => (r.routes[0].geometry.nodes[0] = 999),
      (r) => r.routes[0].geometry.points_e7.pop(),
      (r) => (r.routes[0].geometry.occurrences[0].from = 999),
    ] as ((r: ReturnType<typeof JSON.parse>) => void)[])
      expect(() => decodeRoute(mutate(change), query)).toThrow();
  });
  it("preserves exact counters above Number.MAX_SAFE_INTEGER and ignores optional additions", () => {
    const text = raw.replace(
      /"expanded_states":\s*\d+/,
      '"expanded_states":9007199254740993',
    );
    const r = decodeRoute(text, query);
    expect(r.evidence.search.expanded_states).toBe(9007199254740993n);
    expect(parseExact('{"n":18446744073709551615}')).toEqual({
      n: 18446744073709551615n,
    });
    const extra = mutate((r) => {
      delete r.routes[0].optional_capabilities;
      r.routes[0].future_optional = { enabled: true };
    });
    expect(decodeRoute(extra, query).evidence).toHaveProperty(
      "future_optional",
    );
  });
  it("reveals a derived prefix, preserves source and completes at the exact final vertex", () => {
    const result = decodeRoute(raw, query),
      before = JSON.stringify(result.evidence),
      path = presentationPath(result.evidence);
    for (const p of [0, 0.1, 0.3, 0.6, 0.9, 1]) {
      const prefix = revealPrefix(path, p);
      expect(prefix.line[0]).toEqual(path.points[0]);
      expect(prefix.line.length).toBeLessThanOrEqual(path.points.length + 1);
    }
    expect(revealPrefix(path, 1).line).toEqual(path.points);
    expect(JSON.stringify(result.evidence)).toBe(before);
    expect(path.total).toBeCloseTo(result.evidence.distance_meters, 7);
  });
  it("retains duplicate vertices and repeated traversal occurrences in presentation", () => {
    const r = decodeRoute(raw, query).evidence;
    const path = presentationPath(r);
    expect(path.points[2]).toEqual(path.points[3]);
    const points = r.geometry.points_e7;
    const repeated = presentationPath({
      ...r,
      geometry: {
        ...r.geometry,
        points_e7: [...points, ...points.slice().reverse(), ...points],
      },
    });
    expect(repeated.points).toEqual([
      ...path.points,
      ...path.points.slice().reverse(),
      ...path.points,
    ]);
    expect(repeated.total).toBeCloseTo(path.total * 3, 7);
    // After two full traversals, the leading edge is back at the start of the third.
    const prefix = revealPrefix(repeated, 2 / 3);
    expect(prefix.head[0]).toBeCloseTo(path.points[0][0], 10);
    expect(prefix.head[1]).toBeCloseTo(path.points[0][1], 10);
    expect(revealPrefix(repeated, 1).line).toEqual(repeated.points);
  });
  it("formats source metrics without changing their meaning", () => {
    expect(distanceLabel(212.335)).toBe("212 m");
    expect(durationLabel(38.22)).toBe("38 sec");
    expect(distanceLabel(1234)).toBe("1.23 km");
    expect(durationLabel(90)).toBe("1.5 min");
  });
});
