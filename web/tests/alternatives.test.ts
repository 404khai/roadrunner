import { expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { decodeAlternatives, type RouteRequest } from "../src/contracts";
import example from "./alternatives.json";
import { presentationPath } from "../src/geometry";
const file = (name: string) =>
  readFileSync(new URL(`./${name}.json`, import.meta.url), "utf8");
const query = JSON.parse(file("alternatives-request")) as RouteRequest;
const raw = file("alternatives");
it("consumes real ranked Yen members without changing their geometry or evidence", () => {
  const r = decodeAlternatives(raw, query);
  expect(r.raw).toBe(raw);
  expect(r.members).toHaveLength(3);
  const source = JSON.parse(raw);
  r.members.forEach((m, i) => {
    expect(m.route).toEqual(source.routes[i].route);
    expect(presentationPath(m.route).points).toEqual(
      m.route.geometry.points_e7.map((p) => [
        p.longitude_e7 / 1e7,
        p.latitude_e7 / 1e7,
      ]),
    );
    expect(
      m.inspection?.segments.reduce((n, s) => n + s.objective_cost, 0),
    ).toBeCloseTo(m.route.objective.value, 9);
    expect(m.inspection?.traffic_delay_seconds).toBeNull();
  });
  expect(r.members[1].route.objective.value).toBeGreaterThan(
    r.members[0].route.objective.value,
  );
});
it("preserves directed reverse curves and no-travel alternatives", () => {
  const reverse = decodeAlternatives(
    file("alternatives-reverse"),
    JSON.parse(file("alternatives-reverse-request")),
  );
  expect(
    reverse.members[0].route.geometry.occurrences.some(
      (o) => o.orientation === "reverse",
    ),
  ).toBe(true);
  const no = decodeAlternatives(
    file("alternatives-no-travel"),
    JSON.parse(file("alternatives-no-travel-request")),
  );
  expect(no.members).toHaveLength(1);
  expect(no.summary.termination).toBe("exhausted");
  expect(no.members[0].route.geometry.points_e7).toHaveLength(1);
  expect(no.members[0].inspection?.segments).toEqual([]);
});
it("rejects inconsistent identities, ranks, provenance, objective weights and required semantics", () => {
  for (const mutate of [
    (r: typeof example) => (r.schema_version = 2),
    (r: typeof example) => (r.routes[1].rank = 1),
    (r: typeof example) => (r.routes[1].member_id = r.routes[0].member_id),
    (r: typeof example) => r.routes[1].route.provenance.departure_seconds++,
    (r: typeof example) =>
      r.routes[1].route.geometry.occurrences[0].point_end++,
    (r: typeof example) =>
      r.routes[1].segment_inspection.segments[0].objective_cost++,
    (r: typeof example) =>
      (r.routes[1].segment_inspection.evaluation_id =
        r.routes[0].route.evaluation_id),
    (r: typeof example) => (r.summary.truncated = true),
    (r: typeof example) => (r.summary.scope = "all paths"),
    (r: typeof example) => (r.routes[0].route.search.state_mode = "node"),
    (r: typeof example) => (r.routes[1].max_shared_distance_ratio = 0.99),
  ]) {
    const r = JSON.parse(raw);
    mutate(r);
    expect(() => decodeAlternatives(JSON.stringify(r), query)).toThrow();
  }
  expect(() =>
    decodeAlternatives(raw, { ...query, algorithm: "astar" }),
  ).toThrow();
});
it("degrades unknown optional inspection, preserves bounded partial results and exact counters", () => {
  const r = JSON.parse(raw);
  r.routes.forEach(
    (m: (typeof example.routes)[number]) =>
      (m.segment_inspection.schema_version = 2),
  );
  r.summary.termination = "budget";
  r.summary.truncated = true;
  r.summary.search_states = "REPLACE_BIGINT";
  const text = JSON.stringify(r).replace(
    '"REPLACE_BIGINT"',
    "18446744073709551615",
  );
  const decoded = decodeAlternatives(text, query);
  expect(decoded.members[0].inspection).toBeUndefined();
  expect(decoded.summary.truncated).toBe(true);
  expect(decoded.summary.search_states).toBe(18446744073709551615n);
});

it("consumes actual traffic-factor and same-path delay evidence without treating base time as objective", () => {
  const r = decodeAlternatives(
    file("alternatives-traffic"),
    JSON.parse(file("alternatives-traffic-request")),
  );
  const m = r.members.find(
    (m) => (m.inspection?.traffic_delay_seconds ?? 0) > 0,
  )!;
  expect(m).toBeDefined();
  expect(m.route.provenance.traffic_digest).not.toBeNull();
  expect(m.inspection!.segments.some((s) => s.traffic_factor === 2)).toBe(true);
  expect(
    m.inspection!.base_seconds + m.inspection!.traffic_delay_seconds!,
  ).toBeCloseTo(m.route.predicted_elapsed_seconds, 9);
  expect(m.route.objective.value).toBe(m.route.predicted_elapsed_seconds);
});
