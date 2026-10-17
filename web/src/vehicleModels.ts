import * as T from "three";
import { RoundedBoxGeometry } from "three/addons/geometries/RoundedBoxGeometry.js";
import { riderVisual } from "./riderVisual";
import type { VehiclePose } from "./ambientScene";
// Original procedural assets, authored in this repository. Metres, local +Y forward,
// +Z up. No downloaded asset/rig dependency; low-poly parts share per-model materials.
export function headingRotation(bearing: number) {
  return (bearing * Math.PI) / 180;
}
export function vehicleModel(
  kind: VehiclePose["kind"],
  color: string,
  visualId = "1",
) {
  const group = new T.Group();
  const wheels: T.Mesh[] = [];
  const materials = new Map<string, T.MeshStandardMaterial>();
  const material = (c: string) => {
    if (!materials.has(c))
      materials.set(
        c,
        new T.MeshStandardMaterial({
          color: c,
          roughness: 0.6,
          metalness: 0.06,
        }),
      );
    return materials.get(c)!;
  };
  const box = (
    w: number,
    l: number,
    h: number,
    x: number,
    y: number,
    z: number,
    c: string,
  ) => {
    const m = new T.Mesh(new T.BoxGeometry(w, l, h), material(c));
    m.position.set(x, y, z);
    group.add(m);
    return m;
  };
  const wheel = (x: number, y: number, r: number) => {
    const m = new T.Mesh(
      new T.TorusGeometry(r * 0.73, r * 0.27, 8, 20),
      material("#252629"),
    );
    m.rotation.y = Math.PI / 2;
    m.position.set(x, y, r);
    group.add(m);
    wheels.push(m);
    const hub = new T.Mesh(
      new T.CylinderGeometry(r * 0.45, r * 0.45, 0.15, 12),
      material("#737b76"),
    );
    hub.rotation.z = Math.PI / 2;
    hub.position.set(x, y, r);
    group.add(hub);
  };
  if (kind === "rider" || kind === "motorcycle") {
    const v = riderVisual(visualId),
      body = kind === "rider" ? v.body : color,
      jacket = kind === "rider" ? v.jacket : "#45474b";
    const round = (
      w: number,
      l: number,
      h: number,
      x: number,
      y: number,
      z: number,
      c: string,
    ) => {
      const mesh = new T.Mesh(
        new RoundedBoxGeometry(w, l, h, 2, Math.min(w, l, h) * 0.22),
        material(c),
      );
      mesh.position.set(x, y, z);
      group.add(mesh);
      return mesh;
    };
    const oval = (
      w: number,
      l: number,
      h: number,
      x: number,
      y: number,
      z: number,
      c: string,
    ) => {
      const mesh = new T.Mesh(new T.SphereGeometry(1, 16, 12), material(c));
      mesh.scale.set(w, l, h);
      mesh.position.set(x, y, z);
      group.add(mesh);
      return mesh;
    };
    const limb = (a: number[], b: number[], r: number, c: string) => {
      const from = new T.Vector3(...a),
        to = new T.Vector3(...b),
        delta = to.clone().sub(from);
      const mesh = new T.Mesh(
        new T.CapsuleGeometry(r, Math.max(0.01, delta.length() - 2 * r), 4, 8),
        material(c),
      );
      mesh.position.copy(from.add(to).multiplyScalar(0.5));
      mesh.quaternion.setFromUnitVectors(
        new T.Vector3(0, 1, 0),
        delta.normalize(),
      );
      group.add(mesh);
    };
    wheel(0, -0.72, 0.28);
    wheel(0, 0.75, 0.28);
    round(0.48, 1.32, 0.22, 0, -0.03, 0.42, body);
    oval(0.3, 0.51, 0.24, 0, -0.33, 0.5, body);
    round(0.46, 0.69, 0.14, 0, -0.22, 0.77, "#282b2d");
    // Rounded leg shield and front fender, distinct from the handlebar/headlamp pod.
    const shield = oval(0.31, 0.14, 0.42, 0, 0.56, 0.73, body);
    shield.rotation.x = -0.12;
    oval(0.19, 0.35, 0.16, 0, 0.75, 0.43, body);
    oval(0.36, 0.13, 0.13, 0, 0.6, 1.14, body);
    round(0.83, 0.09, 0.075, 0, 0.6, 1.09, "#26282a");
    for (const x of [-0.37, 0.37])
      round(0.15, 0.1, 0.09, x, 0.6, 1.09, "#252729");
    const lamp = new T.Mesh(
      new T.CylinderGeometry(0.125, 0.125, 0.08, 20),
      material("#dfe4db"),
    );
    lamp.position.set(0, 0.74, 1.14);
    group.add(lamp);
    const rim = new T.Mesh(
      new T.TorusGeometry(0.13, 0.018, 6, 20),
      material("#656b67"),
    );
    rim.rotation.x = Math.PI / 2;
    rim.position.set(0, 0.79, 1.14);
    group.add(rim);
    for (const x of [-0.22, 0.22])
      oval(0.035, 0.025, 0.1, x, 0.68, 0.68, "#e4dfcb");
    for (const z of [0.6, 0.64, 0.68])
      round(0.12, 0.02, 0.014, 0, 0.71, z, "#303332");
    round(0.64, 0.55, 0.56, 0, -0.7, 1.09, "#373a38");
    round(0.66, 0.57, 0.09, 0, -0.7, 1.4, "#444843");
    round(0.64, 0.025, 0.045, 0, -0.986, 1.31, "#9a9c94");
    round(0.08, 0.04, 0.14, 0, -1, 1.31, "#292d29");
    oval(0.12, 0.03, 0.065, 0, -0.82, 0.57, "#db4d45");
    for (const x of [-0.22, 0.22])
      oval(0.04, 0.025, 0.05, x, -0.8, 0.56, "#ded6bf");
    round(0.22, 0.035, 0.16, 0, -0.91, 0.36, "#303533");
    round(0.1, 0.04, 0.03, 0, -0.94, 0.28, "#c24c43");
    oval(0.28, 0.22, 0.33, 0, -0.03, 1.17, jacket);
    // Open-face helmet: skin inset, dark rim and side pads, without a flat visor bar.
    oval(
      0.25,
      0.24,
      0.27,
      0,
      0.02,
      1.7,
      kind === "rider" ? v.helmet : "#c5c1b7",
    );
    oval(0.17, 0.095, 0.17, 0, 0.225, 1.65, "#d6ad91");
    oval(0.205, 0.05, 0.04, 0, 0.252, 1.82, "#292d2e");
    for (const x of [-0.223, 0.223])
      oval(0.04, 0.14, 0.16, x, 0.035, 1.63, "#303332");
    limb([0, 0.02, 1.42], [0, 0.02, 1.5], 0.085, "#d6ad91");
    for (const x of [-1, 1]) {
      limb([x * 0.245, 0, 1.33], [x * 0.32, 0.27, 1.14], 0.09, jacket);
      limb([x * 0.32, 0.27, 1.14], [x * 0.35, 0.57, 1.09], 0.075, jacket);
      oval(0.075, 0.07, 0.06, x * 0.35, 0.59, 1.08, "#d6ad91");
      limb([x * 0.19, -0.16, 0.83], [x * 0.27, 0.22, 0.68], 0.11, "#303336");
      limb([x * 0.27, 0.22, 0.68], [x * 0.28, 0.21, 0.38], 0.09, "#303336");
      round(0.16, 0.28, 0.1, x * 0.28, 0.26, 0.29, "#242729");
      limb([x * 0.38, 0.63, 1.14], [x * 0.45, 0.67, 1.43], 0.018, "#5e6461");
      oval(0.09, 0.025, 0.075, x * 0.45, 0.68, 1.45, "#b1b9b4");
    }
  } else {
    const l = kind === "truck" ? 6 : kind === "van" ? 4.5 : 3.8,
      w = kind === "truck" ? 2.2 : 1.8;
    box(w, l, 0.65, 0, 0, 0.7, color);
    if (kind === "truck") {
      box(w, l * 0.6, 1.9, 0, -0.8, 1.9, color);
      box(w, 0.95, 1.2, 0, 2, 1.4, color);
      box(w * 0.88, 0.08, 0.55, 0, 2.49, 1.7, "#3d4d55");
    } else {
      box(w * 0.9, l * 0.6, kind === "van" ? 1.5 : 0.75, 0, -0.1, 1.4, color);
      box(w * 0.8, 0.08, 0.45, 0, l * 0.3, 1.56, "#3d4d55");
    }
    for (const x of [-w / 2, w / 2])
      for (const y of [-l * 0.32, l * 0.32]) wheel(x, y, 0.35);
    for (const x of [-w * 0.32, w * 0.32]) {
      box(0.3, 0.06, 0.15, x, l / 2 + 0.02, 0.85, "#efe6b5");
      box(0.3, 0.06, 0.15, x, -l / 2 - 0.02, 0.85, "#b46c60");
    }
  }
  group.userData.wheels = wheels;
  return group;
}
export function disposeVehicle(group: T.Group) {
  group.traverse((o) => {
    if (o instanceof T.Mesh) {
      o.geometry.dispose();
      const m = o.material;
      (Array.isArray(m) ? m : [m]).forEach((v) => v.dispose());
    }
  });
}
