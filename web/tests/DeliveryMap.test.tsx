// @vitest-environment jsdom
import { beforeEach, afterEach, describe, it, expect, vi } from "vitest";
import {
  cleanup,
  render,
  fireEvent,
  screen,
  act,
} from "@testing-library/react";
import raw from "./delivery-delayed.json?raw";
import fleetRaw from "./delivery-fleet.json?raw";
import { decodeScene } from "../src/deliveryScene";
const engine = vi.hoisted(() => ({
  markers: [] as {
    element: HTMLElement;
    removed: boolean;
    position: unknown;
  }[],
  zoom: 19,
  setData: vi.fn(),
  easeTo: vi.fn(),
  events: {} as Record<string, (event: { originalEvent?: object }) => void>,
}));
vi.mock("../src/useReveal", () => ({
  useReveal: () => ({ progress: 1, replay: vi.fn() }),
}));
vi.mock("../src/immersiveLayer", () => ({
  immersiveLayer: () => ({
    id: "delivery-models",
    type: "custom",
    update: vi.fn(),
  }),
}));
vi.mock("../src/MapView", () => ({ style: {} }));
vi.mock("maplibre-gl", () => ({
  Map: class {
    sources = new Map<string, object>();
    removed = false;
    addControl() {}
    getLayer() {
      return this.removed ? undefined : {};
    }
    removeLayer() {
      if (this.removed) throw Error("Map already removed");
    }
    addLayer() {}
    fitBounds() {}
    getBearing() {
      return 0;
    }
    getZoom() {
      return engine.zoom;
    }
    getPitch() {
      return 55;
    }
    easeTo = engine.easeTo;
    setLayoutProperty() {}
    remove() {
      this.removed = true;
    }
    on(event: string, cb: unknown) {
      engine.events[event] = cb as (event: { originalEvent?: object }) => void;
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
    setPitchAlignment() {
      return this;
    }
    setOffset() {
      return this;
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
beforeEach(() => {
  engine.zoom = 19;
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(null);
});
afterEach(() => {
  cleanup();
  engine.markers = [];
  engine.setData.mockClear();
  engine.easeTo.mockClear();
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

it("preserves the same rider/action/position through perspective changes and exits follow on gestures", () => {
  render(
    <DeliveryMap
      scene={scene}
      selected="1"
      at={5}
      onSelect={vi.fn()}
      onEdge={vi.fn()}
      traffic={false}
      reduced
    />,
  );
  const first = engine.markers[0],
    position = first.position;
  fireEvent.click(screen.getByRole("button", { name: "Perspective 2.5D" }));
  act(() => engine.events.pitch({}));
  expect(engine.markers).toHaveLength(1);
  expect(first.position).toEqual(position);
  expect(first.element.dataset.action).toBe("0");
  expect(first.element.dataset.view).toBe("perspective");
  expect(engine.easeTo).toHaveBeenCalledWith(
    expect.objectContaining({ pitch: 55, duration: 0 }),
  );
  expect(screen.getByRole("button", { name: "Stop following" })).toBeTruthy();
  expect(engine.easeTo).toHaveBeenCalledWith(
    expect.objectContaining({ center: position, zoom: 18.5, duration: 0 }),
  );
  act(() => engine.events.movestart({ originalEvent: {} }));
  expect(
    screen.getByRole("button", { name: "Follow selected rider" }),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Overhead 2D" }));
  expect(first.element.dataset.view).toBe("overhead");
  expect(first.position).toEqual(position);
});

it("does not follow a missing original position and keeps unknown heading neutral", () => {
  const missing = { ...scene, paths: new Map() };
  render(
    <DeliveryMap
      scene={missing}
      selected="1"
      at={5}
      onSelect={vi.fn()}
      onEdge={vi.fn()}
      traffic={false}
    />,
  );
  expect(
    (
      screen.getByRole("button", {
        name: "Follow selected rider",
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  fireEvent.click(screen.getByRole("button", { name: "Perspective 2.5D" }));
  act(() => engine.events.pitch({}));
  expect(document.querySelectorAll(".scooter-marker")).toHaveLength(0);
});

it("auto-follows new rider selections, can disable auto-follow, and retains marker identity", () => {
  const props = {
    scene,
    selected: "1",
    at: 5,
    onSelect: vi.fn(),
    onEdge: vi.fn(),
    traffic: false,
    reduced: true,
  };
  const view = render(<DeliveryMap {...props} selectionRevision={0} />);
  const first = engine.markers[0];
  fireEvent.click(screen.getByRole("button", { name: "Perspective 2.5D" }));
  act(() => engine.events.pitch({}));
  expect(screen.getByRole("button", { name: "Stop following" })).toBeTruthy();
  act(() => engine.events.movestart({ originalEvent: {} }));
  view.rerender(<DeliveryMap {...props} selectionRevision={1} />);
  expect(screen.getByRole("button", { name: "Stop following" })).toBeTruthy();
  fireEvent.click(
    screen.getByRole("checkbox", { name: "Auto-follow selected rider" }),
  );
  view.rerender(<DeliveryMap {...props} selectionRevision={2} />);
  expect(
    screen.getByRole("button", { name: "Follow selected rider" }),
  ).toBeTruthy();
  expect(engine.markers).toHaveLength(1);
  expect(engine.markers[0]).toBe(first);
  expect(first.element.dataset.action).toBe("0");
});

it("uses a compact portrait tag when zoomed out without replacing rider markers", () => {
  render(
    <DeliveryMap
      scene={scene}
      selected="1"
      at={5}
      onSelect={vi.fn()}
      onEdge={vi.fn()}
      traffic={false}
      reduced
    />,
  );
  const first = engine.markers[0],
    position = first.position;
  fireEvent.click(screen.getByRole("button", { name: "Perspective 2.5D" }));
  act(() => engine.events.pitch({}));
  engine.zoom = 16;
  act(() => engine.events.zoom({}));
  expect(first.element.dataset.view).toBe("tag");
  expect(engine.markers).toHaveLength(1);
  expect(first.position).toEqual(position);
  expect(first.element.dataset.action).toBe("0");
  expect(decodeURIComponent(first.element.querySelector("img")!.src)).toContain(
    'viewBox="0 0 36 44"',
  );
  engine.zoom = 19;
  act(() => engine.events.zoom({}));
  expect(first.element.dataset.view).toBe("perspective");
  expect(engine.markers[0]).toBe(first);
});

it("unmounts an active 3D workspace after map destruction without removing a dead layer", async () => {
  const view = render(
    <DeliveryMap
      scene={scene}
      selected="1"
      at={5}
      onSelect={vi.fn()}
      onEdge={vi.fn()}
      traffic={false}
      reduced
    />,
  );
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: "Immersive 3D" }));
  });
  expect(screen.queryByRole("alert")).toBeNull();
  expect(() => view.unmount()).not.toThrow();
});
