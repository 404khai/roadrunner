// @vitest-environment jsdom
import { beforeEach, afterEach, describe, it, expect, vi } from "vitest";
import { cleanup, render } from "@testing-library/react";
import raw from "./delivery-delayed.json?raw";
import fleetRaw from "./delivery-fleet.json?raw";
import { decodeScene } from "../src/deliveryScene";
const engine = vi.hoisted(() => ({
  markers: [] as {
    element: HTMLElement;
    removed: boolean;
    position: unknown;
  }[],
  setData: vi.fn(),
}));
vi.mock("../src/MapView", () => ({ style: {} }));
vi.mock("maplibre-gl", () => ({
  Map: class {
    sources = new Map<string, object>();
    addControl() {}
    addLayer() {}
    fitBounds() {}
    remove() {}
    on(event: string, cb: unknown) {
      if (event === "style.load") (cb as () => void)();
    }
    addSource(id: string) {
      this.sources.set(id, { setData: engine.setData });
    }
    getSource(id: string) {
      return this.sources.get(id);
    }
    getContainer() {
      return { clientWidth: 1200, clientHeight: 800 };
    }
    getCanvas() {
      return document.createElement("canvas");
    }
  },
  Marker: class {
    record: { element: HTMLElement; removed: boolean; position: unknown };
    constructor({ element }: { element: HTMLElement }) {
      this.record = { element, removed: false, position: null };
      engine.markers.push(this.record);
    }
    setLngLat(position: unknown) {
      this.record.position = position;
      return this;
    }
    addTo() {
      document.body.append(this.record.element);
      return this;
    }
    remove() {
      this.record.removed = true;
      this.record.element.remove();
    }
  },
  NavigationControl: class {},
  LngLatBounds: class {
    extend() {
      return this;
    }
  },
}));
import { DeliveryMap } from "../src/DeliveryMap";
const scene = decodeScene(raw, JSON.parse(raw).graph_digest);
beforeEach(() =>
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null),
);
afterEach(() => {
  cleanup();
  engine.markers = [];
  engine.setData.mockClear();
  vi.restoreAllMocks();
});
describe("delivery marker lifecycle", () => {
  it("reuses one rider element across schedule and suffix changes, moving it causally", () => {
    const props = {
      scene,
      selected: "1",
      onSelect: vi.fn(),
      traffic: false,
      onEdge: vi.fn(),
    };
    const view = render(<DeliveryMap {...props} at={1} />),
      first = engine.markers[0];
    expect(engine.markers).toHaveLength(1);
    expect(first.element.dataset.action).toBe("0");
    expect(first.element.dataset.generation).toBe("0");
    const position = first.position;
    view.rerender(<DeliveryMap {...props} at={2} />);
    expect(engine.markers).toHaveLength(1);
    expect(first.element.dataset.generation).toBe("1");
    expect(first.position).not.toEqual(position);
    view.rerender(<DeliveryMap {...props} at={5} />);
    expect(engine.markers).toHaveLength(1);
    expect(first.element.dataset.action).toBe("0");
    expect(first.removed).toBe(false);
    const uploads = engine.setData.mock.calls.length;
    view.rerender(<DeliveryMap {...props} at={5.1} />);
    expect(engine.setData.mock.calls.length).toBe(uploads); // no per-frame geometry uploads
    view.unmount();
    expect(first.removed).toBe(true);
  });
  it("renders each simultaneous rider once and suppresses unsupported active positions", () => {
    const fleet = decodeScene(fleetRaw, scene.graph_digest);
    const props = {
      selected: "1",
      onSelect: vi.fn(),
      traffic: true,
      onEdge: vi.fn(),
    };
    const view = render(<DeliveryMap {...props} scene={fleet} at={20} />);
    expect(engine.markers).toHaveLength(2);
    expect(
      new Set(engine.markers.map((m) => m.element.dataset.rider)).size,
    ).toBe(2);
    const missing = { ...fleet, paths: new Map() };
    view.rerender(<DeliveryMap {...props} scene={missing} at={20} />);
    expect(document.querySelectorAll(".scooter-marker")).toHaveLength(0);
  });
});
