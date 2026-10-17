import { describe, it, expect } from "vitest";
import { readFileSync } from "node:fs";
import {
  actionProgress,
  decodeScene,
  frameAt,
  sampleRider,
  actionKey,
} from "../src/deliveryScene";
const raw = (kind: string) =>
  readFileSync(new URL(`./delivery-${kind}.json`, import.meta.url), "utf8");
const load = (kind: string) => {
  const text = raw(kind);
  return decodeScene(text, JSON.parse(text).graph_digest);
};
describe("actual Roadrunner scene evidence and causal movement", () => {
  it("keeps two actors in one frame and follows curved directed paths", () => {
    const scene = load("fleet"),
      frame = frameAt(scene, 20);
    expect(frame.riders).toHaveLength(2);
    expect(frame.riders.map((r) => r.execution?.stage)).toEqual([
      "Travelling",
      "Travelling",
    ]);
    expect(
      scene.routes.some((r) =>
        r.geometry?.occurrences.some((o) => o.orientation === "reverse"),
      ),
    ).toBe(true);
    for (const r of frame.riders) {
      const p = sampleRider(scene, r, 20);
      expect(p.position).toHaveLength(2);
      expect(p.progress).toBeGreaterThan(0);
      expect(p.progress).toBeLessThan(1);
      expect(p.bearing).toBeGreaterThanOrEqual(0);
      expect(p.bearing).toBeLessThan(360);
    }
  });
  it("never retroactively rebases progress; delay and plan suffix retain the active identity", () => {
    const scene = load("delayed"),
      a = scene.routes[0].original;
    expect(actionProgress(scene, "1", "0", 1)).toBeCloseTo(1 / a.arrival);
    expect(actionProgress(scene, "1", "0", 2)).toBeCloseTo(2 / a.arrival);
    const progressAt2 = 2 / a.arrival,
      newArrival = scene.schedules[0].execution.arrival;
    expect(actionProgress(scene, "1", "0", 5)).toBeCloseTo(
      progressAt2 + ((1 - progressAt2) * 3) / (newArrival - 2),
    );
    const before = frameAt(scene, 1).riders[0],
      delayed = frameAt(scene, 2).riders[0],
      suffix = frameAt(scene, 5).riders[0];
    expect(before.execution?.action_id).toBe(delayed.execution?.action_id);
    expect(delayed.execution?.generation).not.toBe(
      before.execution?.generation,
    );
    expect(suffix.plan_revision).not.toBe(before.plan_revision);
    expect(suffix.execution?.originating_plan).toBe(
      before.execution?.originating_plan,
    );
    expect(suffix.execution?.route).toEqual(before.execution?.route);
    expect(actionKey(suffix.id, suffix.execution!.action_id)).toBe(
      actionKey(before.id, before.execution!.action_id),
    );
    expect(sampleRider(scene, delayed, 2 - 1e-7).position).toEqual(
      sampleRider(scene, before, 2 - 1e-7).position,
    );
  });
  it("renders accepted stages and custody only after actual transitions", () => {
    const scene = load("solo"),
      order = (t: number) => frameAt(scene, t).orders[0];
    const waiting = scene.frames.find(
      (f) => f.riders[0].execution?.stage === "Waiting",
    )!;
    const service = scene.frames.find(
      (f) => f.riders[0].execution?.stage === "Servicing",
    )!;
    expect(sampleRider(scene, waiting.riders[0], waiting.at).quality).toBe(
      "Accepted simulation endpoint",
    );
    expect(order(service.at).status).toBe("awaiting_pickup");
    expect(order(service.at + 14.9).status).toBe("awaiting_pickup");
    expect(order(service.at + 15).status).toBe("onboard");
    const completion = scene.effects
      .filter((e) => e.effect === "Completion")
      .at(-1)!;
    expect(order(completion.at - 0.01).status).toBe("onboard");
    expect(order(completion.at).status).toBe("delivered");
  });
  it("holds an expired predicted endpoint without completing an unfinished action", () => {
    const scene = load("unfinished"),
      r = frameAt(scene, scene.end).riders[0];
    const future = sampleRider(scene, r, 10000);
    expect(future.progress).toBe(1);
    expect(future.quality).toContain("awaiting accepted arrival");
    expect(r.execution?.stage).toBe("Travelling");
    expect(scene.effects).toHaveLength(0);
    expect(frameAt(scene, 10000).orders[0].status).toBe("awaiting_pickup");
  });
  it("supports a no-travel service with neutral bearing instead of indexing a missing segment", () => {
    const scene = load("fleet"),
      r = frameAt(scene, 0).riders.find((r) => r.id === "2")!;
    expect(r.execution?.route.edges).toHaveLength(0);
    expect(sampleRider(scene, r, 0).bearing).toBeUndefined();
    expect(sampleRider(scene, r, 0).position).toHaveLength(2);
  });
  it("keeps unsupported geometry and traffic unavailable without upgrading observations", () => {
    const original = JSON.parse(raw("solo"));
    original.routes[0].geometry = null;
    original.routes[0].geometry_error = "original graph unavailable";
    original.traffic_semantics = "unsupported/v9";
    original.frames.forEach(
      (f: { traffic: unknown; traffic_digest: unknown }) => {
        f.traffic = null;
        f.traffic_digest = null;
      },
    );
    const scene = decodeScene(JSON.stringify(original), original.graph_digest),
      r = frameAt(scene, 1).riders[0];
    expect(scene.trafficSupported).toBe(false);
    expect(sampleRider(scene, r, 1).position).toBeUndefined();
    expect(sampleRider(scene, r, 1).quality).toContain("unavailable");
  });
  it("pins current traffic separately from original frozen route prediction", () => {
    const scene = load("traffic");
    expect(scene.trafficSupported).toBe(true);
    expect(frameAt(scene, 0).traffic_digest).not.toBe(
      frameAt(scene, 10).traffic_digest,
    );
    expect(frameAt(scene, 0).riders[0].execution?.route.routing).toEqual(
      frameAt(scene, 10).riders[0].execution?.route.routing,
    );
    expect(
      frameAt(scene, 10).traffic?.find((t) => t.edge_id === 3)?.multiplier,
    ).toBe(3);
  });
  it.each(["graph", "direction", "duplicate", "clock", "semantics"])(
    "rejects ambiguous or unsupported %s evidence",
    (kind) => {
      const s = JSON.parse(raw("delayed"));
      if (kind === "graph") s.routes[0].geometry.graph_digest = "0".repeat(64);
      if (kind === "direction") s.routes[0].geometry.occurrences[0].from = 999;
      if (kind === "duplicate") s.routes.push(s.routes[0]);
      if (kind === "clock") s.frames[3].at = 9999;
      if (kind === "semantics") s.motion_model = "gps/v1";
      expect(() => decodeScene(JSON.stringify(s), s.graph_digest)).toThrow();
    },
  );
});

it("shows a distributed fleet using exact graph locations and actual engine ownership", () => {
  const scene = load("distributed"),
    initial = scene.frames[0],
    frame = frameAt(scene, 20);
  expect(initial.riders).toHaveLength(6);
  expect(
    new Set(
      initial.riders.map(
        (r) => `${r.coordinate.latitude},${r.coordinate.longitude}`,
      ),
    ).size,
  ).toBe(6);
  expect(frame.orders).toHaveLength(5);
  expect(
    new Set(
      frame.orders.map((o) => `${o.pickup.latitude},${o.pickup.longitude}`),
    ).size,
  ).toBe(5);
  expect(
    frame.riders.filter((r) => r.execution?.stage === "Travelling"),
  ).toHaveLength(3);
  expect(frame.riders.filter((r) => r.execution === null)).toHaveLength(2);
  expect(frame.riders.find((r) => r.id === "1")?.assigned).toEqual(["1", "2"]);
  for (const r of frame.riders)
    if (r.execution) {
      const source = scene.routes.find(
        (a) => a.action === r.execution!.action_id && a.rider === r.id,
      )!;
      expect(source.geometry).not.toBeNull();
      expect(sampleRider(scene, r, 20).position).toBeDefined();
      for (const order of r.assigned)
        expect(frame.orders.find((o) => o.id === order)?.rider).toBe(r.id);
    }
  expect(
    frameAt(scene, scene.end).orders.every((o) => o.status === "delivered"),
  ).toBe(true);
});
