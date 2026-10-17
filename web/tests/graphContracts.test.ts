import { expect, it } from "vitest";
import { readFileSync } from "node:fs";
import example from "./graph-page.json";
import { decodeGraph } from "../src/graphContracts";
const file = (name: string) =>
  readFileSync(new URL(`./${name}.json`, import.meta.url), "utf8");
const raw = file("graph-page");
const query = {
  digest: example.graph.snapshot_digest,
  nodeOffset: 0,
  edgeOffset: 0,
  objective: "travel_time" as const,
};
it("preserves actual directed graph geometry and distinguishes nodes, physical segments and edges", () => {
  const g = decodeGraph(raw, query);
  expect(g.raw).toBe(raw);
  expect(g.nodes).toHaveLength(58);
  expect(g.edges).toHaveLength(114);
  expect(g.point_count).toBe(527);
  const f = g.edges.find((e) => e.edge === 3)!,
    r = g.edges.find((e) => e.edge === 20)!;
  expect(f.segment).toBe(r.segment);
  expect(f.orientation).toBe("reverse");
  expect(f.points_e7).toEqual(r.points_e7.slice().reverse());
  expect(f.objective_cost).toBe(f.base_seconds);
  expect(g.edges).toEqual(example.edges);
  expect(f.road_class).toBeNull();
});
it("does not convert unavailable traffic into factor 1 or zero objective cost", () => {
  const g = decodeGraph(file("graph-no-traffic"), {
    ...query,
    objective: "traffic_aware",
  });
  expect(g.traffic_snapshot_digest).toBeNull();
  expect(
    g.edges.every(
      (e) =>
        e.traffic_multiplier === null &&
        e.objective_cost === null &&
        e.cost_status === "unavailable_no_traffic_snapshot",
    ),
  ).toBe(true);
});
it("rejects mismatched identity, page coverage, endpoints, costs, unavailable semantics and required version", () => {
  const mutations: ((r: typeof example) => void)[] = [
    (r) => {
      r.schema_version = 2;
    },
    (r) => {
      r.graph.snapshot_digest = "0".repeat(64);
    },
    (r) => {
      r.edge_page.end--;
    },
    (r) => {
      r.edges[0].edge++;
    },
    (r) => {
      r.edges[0].points_e7[0].latitude_e7++;
    },
    (r) => {
      r.edges[0].objective_cost++;
    },
    (r) => {
      Object.assign(r.evaluation, { traffic_digest: "invented" });
    },
    (r) => {
      r.capabilities.road_class = "inferred_from_basemap";
    },
    (r) => {
      r.point_count++;
    },
    (r) => {
      r.nodes.pop();
    },
  ];
  for (const mutate of mutations) {
    const r = structuredClone(example);
    mutate(r);
    expect(() => decodeGraph(JSON.stringify(r), query)).toThrow();
  }
});

it("consumes actual directed traffic factors without applying them to unrelated static objectives", () => {
  const g = decodeGraph(file("graph-traffic"), {
    ...query,
    objective: "traffic_aware",
  });
  expect(g.edges[3].traffic_multiplier).toBe(2);
  expect(g.edges[3].objective_cost).toBeCloseTo(g.edges[3].base_seconds * 2, 9);
  expect(g.edges[20].traffic_multiplier).toBe(1);
  expect(g.evaluation.traffic_digest).toBe(g.traffic_snapshot_digest);
});
