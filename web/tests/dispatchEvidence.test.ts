import { readFileSync } from "node:fs";
import { URL as NodeURL } from "node:url";
import { it, expect } from "vitest";
const spatial = readFileSync(
  new NodeURL("../../docs/evidence/phase-31/spatial.json", import.meta.url),
  "utf8",
);
const exhaustive = readFileSync(
  new NodeURL("../../docs/evidence/phase-31/exhaustive.json", import.meta.url),
  "utf8",
);
const empty = readFileSync(
  new NodeURL("../../docs/evidence/phase-31/empty.json", import.meta.url),
  "utf8",
);
const committed = readFileSync(
  new NodeURL("../../docs/evidence/phase-31/committed.json", import.meta.url),
  "utf8",
);
const rejected = readFileSync(
  new NodeURL("../../docs/evidence/phase-31/rejected.json", import.meta.url),
  "utf8",
);
import { decodeDispatch, radiusRing, legPath } from "../src/dispatchEvidence";
import { haversine } from "../src/geometry";
it("decodes real spatial, exhaustive, unassigned and demo publication evidence", () => {
  for (const raw of [spatial, exhaustive, empty, committed, rejected]) {
    const s = JSON.parse(raw),
      result = decodeDispatch(raw, s.request);
    expect(result.active_execution).toBe(false);
    expect(result.selected).toEqual(s.selected);
  }
  const result = decodeDispatch(spatial, JSON.parse(spatial).request);
  expect(result.selected).toBe("2");
  expect(result.candidates).toHaveLength(5);
  expect(result.committed).toBe(false);
  expect(
    result.candidates[0].legs[1].geometry?.occurrences.some(
      (o) => o.orientation === "reverse",
    ),
  ).toBe(true);
  expect(
    legPath(result.candidates[0].legs[0].geometry!).points.length,
  ).toBeGreaterThan(2);
});
it("rejects rebound policy, context, identity and hypothetical geometry", () => {
  const original = JSON.parse(spatial);
  for (const mutate of [
    (v: typeof original) => (v.graph_digest = "wrong"),
    (v: typeof original) => (v.decision.evidence.coverage = "Complete"),
    (v: typeof original) => (v.request.radius_meters = 3),
    (v: typeof original) => (v.selected = "99"),
    (v: typeof original) =>
      (v.candidates[0].legs[0].geometry.graph_digest = "wrong"),
  ]) {
    const v = structuredClone(original);
    mutate(v);
    expect(() => decodeDispatch(JSON.stringify(v), original.request)).toThrow();
  }
  const missing = structuredClone(original);
  missing.candidates[0].legs[0].geometry = null;
  missing.candidates[0].legs[0].unavailable = "unsupported";
  expect(
    decodeDispatch(JSON.stringify(missing), original.request).candidates[0]
      .legs[0].geometry,
  ).toBeNull();
});
it("renders the declared Haversine boundary without converting pixels to traffic coverage", () => {
  const center: [number, number] = [3.38, 6.52];
  const ring = radiusRing(center, 1000);
  expect(ring).toHaveLength(97);
  for (const p of ring) expect(haversine(center, p)).toBeCloseTo(1000, 5);
  for (const p of radiusRing(center, 0))
    expect(haversine(center, p)).toBeCloseTo(0, 5);
});
