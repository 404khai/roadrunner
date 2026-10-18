import { expect, it } from "vitest";
import { readFileSync } from "node:fs";
import {
  decodeReplay,
  MAX_REPLAY_BYTES,
  replayEvents,
} from "../src/historicalReplay";
import { frameAt, sampleRider } from "../src/deliveryScene";
const raw = readFileSync(
  new URL("./delivery-delayed.json", import.meta.url),
  "utf8",
);
it("reopens the original bytes and deterministically samples out of order across delays and suffix edits", () => {
  const a = decodeReplay(raw),
    b = decodeReplay(raw);
  expect(a.raw).toBe(raw);
  expect(a.run_identity).toBe(b.run_identity);
  const sample = (scene: typeof a, time: number) => {
    const frame = frameAt(scene, time);
    return {
      frame,
      samples: frame.riders.map((r) => sampleRider(scene, r, time)),
    };
  };
  const expected = [0, 1, 2, 5, 125, a.end].map((t) => sample(a, t));
  [a.end, 125, 5, 2, 1, 0].forEach((t) => sample(b, t));
  expect([0, 1, 2, 5, 125, a.end].map((t) => sample(b, t))).toEqual(expected);
  const events = replayEvents(a);
  expect(events).toEqual(replayEvents(b));
  expect(events.some((e) => e.label.includes("created"))).toBe(true);
  expect(events.some((e) => e.label.includes("assigned to rider"))).toBe(true);
  expect(events.some((e) => e.label.includes("schedule changed"))).toBe(true);
  expect(events.some((e) => e.label.includes("plan revised"))).toBe(true);
  expect(events.some((e) => e.label.includes("delivered"))).toBe(true);
});
it("rejects oversized, graph-inconsistent and incomplete replay evidence", () => {
  expect(() => decodeReplay(" ".repeat(MAX_REPLAY_BYTES + 1))).toThrow("8 MiB");
  const invalid = JSON.parse(raw);
  invalid.graph_digest = "wrong-graph";
  expect(() => decodeReplay(JSON.stringify(invalid))).toThrow();
  const incomplete = JSON.parse(raw);
  incomplete.frames = incomplete.frames.filter(
    (f: { at: number }) => f.at > incomplete.start,
  );
  expect(() => decodeReplay(JSON.stringify(incomplete))).toThrow(
    "initial boundary",
  );
});

it("indexes supported retained traffic changes without changing frozen route context", () => {
  const scene = decodeReplay(
    readFileSync(new URL("./delivery-traffic.json", import.meta.url), "utf8"),
  );
  expect(replayEvents(scene)).toContainEqual({
    at: 10,
    label: "Synthetic traffic context changed",
  });
  expect(frameAt(scene, 10).riders[0].execution?.route.routing).toEqual(
    frameAt(scene, 0).riders[0].execution?.route.routing,
  );
});
