import { describe, it, expect } from "vitest";
import { Vector3, Matrix4 } from "three";
import raw from "./delivery-distributed.json?raw";
import { decodeScene } from "../src/deliveryScene";
import {
  ambientActors,
  sampleAmbient,
  ambientSprite,
} from "../src/ambientScene";
import {
  vehicleModel,
  disposeVehicle,
  headingRotation,
} from "../src/vehicleModels";
const scene = decodeScene(raw, JSON.parse(raw).graph_digest);
describe("synthetic scene boundary", () => {
  it("points model forward along compass heading after Mercator Y inversion", () => {
    for (const [bearing, expected] of [
      [0, [0, -1, 0]],
      [90, [1, 0, 0]],
      [180, [0, 1, 0]],
      [270, [-1, 0, 0]],
    ] as const) {
      const transform = new Matrix4()
        .makeRotationZ(headingRotation(bearing))
        .multiply(new Matrix4().makeScale(1, -1, 1));
      const forward = new Vector3(0, 1, 0).applyMatrix4(transform);
      expect(forward.x).toBeCloseTo(expected[0]);
      expect(forward.y).toBeCloseTo(expected[1]);
    }
  });
  it("samples reproducible bounded actors without changing engine evidence", () => {
    const before = scene.raw,
      actors = ambientActors(scene, 1000);
    expect(actors).toHaveLength(48);
    expect(new Set(actors.map((a) => a.id)).size).toBe(48);
    expect(new Set(actors.map((a) => a.kind)).size).toBe(4);
    expect(ambientActors(scene, 0)).toEqual([]);
    expect(actors.map((a) => sampleAmbient(a, 23))).toEqual(
      ambientActors(scene, 48).map((a) => sampleAmbient(a, 23)),
    );
    for (const a of actors) {
      const p = sampleAmbient(a, 23);
      expect(p.position.every(Number.isFinite)).toBe(true);
      expect(Number.isFinite(p.bearing)).toBe(true);
      expect(p.id.startsWith("ambient:")).toBe(true);
      expect(sampleAmbient(a, 24).distance).toBeGreaterThan(p.distance);
    }
    expect(scene.raw).toBe(before);
    expect(ambientActors(scene, 12).map((a) => a.id)).toEqual(
      actors.slice(0, 12).map((a) => a.id),
    );
  });
  it("has distinct overhead and projected assets and reusable bounded models", () => {
    for (const kind of ["car", "truck", "van", "motorcycle"] as const) {
      expect(ambientSprite(kind, "#aaa")).not.toBe(
        ambientSprite(kind, "#aaa", 0),
      );
      expect(ambientSprite(kind, "#aaa", 0)).not.toBe(
        ambientSprite(kind, "#aaa", 90),
      );
      const model = vehicleModel(kind, "#aaa");
      expect(model.children.length).toBeLessThan(64);
      expect(model.userData.wheels.length).toBeGreaterThan(0);
      disposeVehicle(model);
    }
    const rider = vehicleModel("rider", "#aaa", "1");
    expect(rider.children.length).toBeGreaterThan(10);
    disposeVehicle(rider);
  });
});
