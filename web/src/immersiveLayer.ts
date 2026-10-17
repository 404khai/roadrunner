import * as T from "three";
import {
  MercatorCoordinate,
  type CustomLayerInterface,
  type Map as MapLibreMap,
  type CustomRenderMethodInput,
} from "maplibre-gl";
import { vehicleModel, disposeVehicle, headingRotation } from "./vehicleModels";
import type { VehiclePose } from "./ambientScene";
export type ImmersiveLayer = CustomLayerInterface & {
  update: (poses: VehiclePose[], reduced: boolean) => void;
};
export function immersiveLayer(onError: () => void): ImmersiveLayer {
  const scene = new T.Scene(),
    camera = new T.Camera();
  const models = new Map<string, { group: T.Group; signature: string }>();
  let renderer: T.WebGLRenderer, map: MapLibreMap;
  let anchor: MercatorCoordinate | undefined;
  let failed = false;
  scene.add(new T.HemisphereLight(0xffffff, 0x424249, 2.4));
  const sun = new T.DirectionalLight(0xffffff, 2);
  sun.position.set(-30, -50, 100);
  scene.add(sun);
  return {
    id: "delivery-models",
    type: "custom",
    renderingMode: "3d",
    onAdd(m, gl) {
      map = m;
      renderer = new T.WebGLRenderer({
        canvas: m.getCanvas(),
        context: gl,
        antialias: true,
      });
      renderer.autoClear = false;
    },
    update(poses, reduced) {
      if (!anchor && poses.length)
        anchor = MercatorCoordinate.fromLngLat(poses[0].position, 0);
      const keep = new Set<string>();
      for (const p of poses) {
        keep.add(p.id);
        const signature = `${p.kind}:${p.color}:${p.visualId}`;
        let entry = models.get(p.id);
        if (entry?.signature !== signature) {
          if (entry) {
            scene.remove(entry.group);
            disposeVehicle(entry.group);
          }
          entry = {
            group: vehicleModel(p.kind, p.color, p.visualId),
            signature,
          };
          entry.group.name = p.id;
          scene.add(entry.group);
          models.set(p.id, entry);
        }
        const origin = MercatorCoordinate.fromLngLat(p.position, 0),
          // Delivery models use a declared 1.6x cosmetic emphasis for readability.
          s =
            origin.meterInMercatorCoordinateUnits() *
            (p.kind === "rider" ? 1.6 : 1);
        entry.group.position.set(
          origin.x - (anchor?.x ?? 0),
          origin.y - (anchor?.y ?? 0),
          origin.z,
        );
        // Negated Y maps local north to Mercator north. Heading is clockwise from north.
        entry.group.scale.set(s, -s, s);
        entry.group.rotation.z = headingRotation(p.bearing);
        for (const wheel of entry.group.userData.wheels as T.Mesh[]) {
          wheel.rotation.x = reduced ? 0 : p.distance / 0.28;
        }
      }
      for (const [id, e] of models)
        if (!keep.has(id)) {
          scene.remove(e.group);
          disposeVehicle(e.group);
          models.delete(id);
        }
      map?.triggerRepaint();
    },
    render(_gl: WebGL2RenderingContext, args: CustomRenderMethodInput) {
      if (failed) return;
      try {
        camera.projectionMatrix.fromArray(
          args.defaultProjectionData.mainMatrix,
        );
        if (anchor)
          camera.projectionMatrix.multiply(
            new T.Matrix4().makeTranslation(anchor.x, anchor.y, 0),
          );
        renderer.resetState();
        const start = performance.now();
        renderer.render(scene, camera);
        const canvas = map.getCanvas();
        canvas.dataset.modelCount = String(models.size);
        canvas.dataset.modelCalls = String(renderer.info.render.calls);
        canvas.dataset.modelTriangles = String(renderer.info.render.triangles);
        canvas.dataset.modelGeometries = String(
          renderer.info.memory.geometries,
        );
        canvas.dataset.modelFrameMs = String(performance.now() - start);
      } catch {
        failed = true;
        onError();
      }
    },
    onRemove() {
      for (const e of models.values()) disposeVehicle(e.group);
      models.clear();
      renderer?.dispose();
      for (const key of [
        "modelCount",
        "modelCalls",
        "modelTriangles",
        "modelGeometries",
        "modelFrameMs",
      ])
        if (map) delete map.getCanvas().dataset[key];
    },
  };
}
