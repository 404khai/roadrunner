import { expect, it } from "vitest";
import { readFileSync } from "node:fs";
import definitions from "../src/fleet-demo-scenarios.json";
import {
  deliveryScenario,
  DISTRIBUTED_DEMO_DIGEST,
} from "../src/DeliveryWorkspace";
import {
  decodeReplay,
  MAX_REPLAY_BYTES,
  replayEvents,
} from "../src/historicalReplay";

for (const size of ["12", "24"] as const) {
  it(`qualifies the complete ${size}-rider capture and shared live preset`, () => {
    const raw = readFileSync(
      new URL(`../public/demos/fleet-${size}.json`, import.meta.url),
      "utf8",
    );
    expect(Buffer.byteLength(raw)).toBeLessThan(MAX_REPLAY_BYTES);
    const scene = decodeReplay(raw);
    expect(
      deliveryScenario(DISTRIBUTED_DEMO_DIGEST, `showcase-${size}`, {
        rider: 0,
        pickup: 1,
        customer: 2,
      }),
    ).toEqual(definitions[size]);
    expect(scene.frames.every((f) => f.riders.length === Number(size))).toBe(
      true,
    );
    expect(new Set(scene.frames[0].riders.map((r) => r.id)).size).toBe(
      Number(size),
    );
    expect(
      scene.frames.at(-1)?.orders.every((o) => o.status === "delivered"),
    ).toBe(true);
    expect(
      scene.frames.some((f) => f.riders.some((r) => Number(r.load) === 2)),
    ).toBe(true);
    expect(
      scene.frames.some(
        (f) =>
          f.riders.filter((r) => r.execution?.stage === "Travelling").length >
          6,
      ),
    ).toBe(true);
    expect(scene.routes.every((r) => r.geometry && !r.geometry_error)).toBe(
      true,
    );
    expect(
      replayEvents(scene).filter(
        (e) => e.label === "Synthetic traffic context changed",
      ),
    ).toHaveLength(2);
    expect(() =>
      deliveryScenario("different-graph", `showcase-${size}`, {
        rider: 0,
        pickup: 1,
        customer: 2,
      }),
    ).toThrow(/bundled Phase 10 graph/);
  });
}
