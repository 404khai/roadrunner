import { beforeEach, it, expect, vi } from "vitest";
import type { Map as LibreMap, CustomRenderMethodInput } from "maplibre-gl";
const gpu = vi.hoisted(() => ({
  render: vi.fn(),
  dispose: vi.fn(),
  reset: vi.fn(),
}));
vi.mock("three", async (importOriginal) => {
  const original = await importOriginal<typeof import("three")>();
  return {
    ...original,
    WebGLRenderer: class {
      autoClear = false;
      info = { render: { calls: 1, triangles: 1 }, memory: { geometries: 1 } };
      resetState = gpu.reset;
      render = gpu.render;
      dispose = gpu.dispose;
    },
  };
});
import { immersiveLayer } from "../src/immersiveLayer";
import { Scene, Matrix4 } from "three";
import type { VehiclePose } from "../src/ambientScene";
beforeEach(() => vi.clearAllMocks());
it("reuses models for sampled progress and heading and disposes on disable", () => {
  const canvas = { dataset: {} },
    map = { getCanvas: () => canvas, triggerRepaint: vi.fn() };
  const layer = immersiveLayer(vi.fn());
  layer.onAdd!(map as unknown as LibreMap, {} as WebGL2RenderingContext);
  const pose: VehiclePose = {
    id: "rider:1",
    kind: "rider",
    position: [3.38, 6.52],
    bearing: 90,
    distance: 3,
    color: "#aaa",
    visualId: "1",
  };
  layer.update([pose], false);
  const args = {
    defaultProjectionData: { mainMatrix: new Matrix4().identity().toArray() },
  } as unknown as CustomRenderMethodInput;
  layer.render({} as WebGL2RenderingContext, args);
  const scene = gpu.render.mock.calls[0][0] as Scene,
    model = scene.getObjectByName("rider:1")!;
  expect(model.rotation.z).toBeCloseTo(Math.PI / 2);
  layer.update([{ ...pose, position: [3.381, 6.52], distance: 4 }], false);
  expect(scene.getObjectByName("rider:1")).toBe(model);
  layer.update([], false);
  expect(scene.getObjectByName("rider:1")).toBeUndefined();
  layer.onRemove!(map as unknown as LibreMap, {} as WebGL2RenderingContext);
  expect(gpu.dispose).toHaveBeenCalledOnce();
  expect(canvas.dataset).toEqual({});
});
it("reports renderer failure once and stops attempting the failed renderer", () => {
  const error = vi.fn(),
    map = { getCanvas: () => ({ dataset: {} }), triggerRepaint: vi.fn() },
    layer = immersiveLayer(error);
  layer.onAdd!(map as unknown as LibreMap, {} as WebGL2RenderingContext);
  gpu.render.mockImplementationOnce(() => {
    throw Error("GPU unavailable");
  });
  const args = {
    defaultProjectionData: { mainMatrix: new Matrix4().identity().toArray() },
  } as unknown as CustomRenderMethodInput;
  layer.render({} as WebGL2RenderingContext, args);
  layer.render({} as WebGL2RenderingContext, args);
  expect(error).toHaveBeenCalledOnce();
  expect(gpu.render).toHaveBeenCalledOnce();
  layer.onRemove!(map as unknown as LibreMap, {} as WebGL2RenderingContext);
});
