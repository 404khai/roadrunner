// @vitest-environment jsdom
import { beforeEach, afterEach, it, expect, vi } from "vitest";
import {
  render,
  screen,
  waitFor,
  cleanup,
  fireEvent,
} from "@testing-library/react";
import raw from "./delivery-solo.json?raw";
import { catalogSchema } from "../src/contracts";
import catalog from "./catalog.json";
vi.mock("../src/DeliveryMap", () => ({
  DeliveryMap: () => <div>Delivery map renderer</div>,
}));
vi.mock("../src/useReveal", () => ({ useReducedMotion: () => true }));
import { DeliveryWorkspace } from "../src/DeliveryWorkspace";
beforeEach(() =>
  vi.stubGlobal(
    "matchMedia",
    vi.fn(() => ({ matches: false })),
  ),
);
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
it("uses actual engine evidence, starts paused for reduced motion, and keeps timer/authority separate", async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(raw));
  vi.stubGlobal("fetch", fetch);
  render(
    <DeliveryWorkspace
      catalog={catalogSchema.parse(catalog)}
      token=""
      onExit={() => {}}
    />,
  );
  fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
    target: { value: "solo" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Run delivery simulation" }),
  );
  await waitFor(() =>
    expect(
      screen.getByRole("button", { name: "Play simulation presentation" }),
    ).toBeTruthy(),
  );
  expect(fetch.mock.calls[0][0]).toBe("/v1/visualization/deliveries");
  expect(
    screen.getAllByRole("img", { name: /Rider 1 helmet portrait/ }).length,
  ).toBeGreaterThan(0);
  expect(document.querySelector(".delivery-panel")).toBeNull();
  expect(
    screen.getByRole("button", { name: "Expand Delivery simulation" }),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Collapse Rider 1" }));
  expect(screen.getByRole("button", { name: "Expand Rider 1" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Expand Rider 1" }));
  expect(
    screen.getByText("Simulated position · uniform route progress"),
  ).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Presentation seconds"), {
    target: { value: "125" },
  });
  expect(screen.getByText("Accepted simulation endpoint")).toBeTruthy();
  expect(screen.getByText("Servicing", { selector: "h3 span" })).toBeTruthy();
  expect(screen.getByText("0 delivered")).toBeTruthy();
});
it("shows missing connection and explicit server errors", async () => {
  const view = render(<DeliveryWorkspace token="" onExit={() => {}} />);
  expect(
    (
      screen.getByRole("button", {
        name: "Run delivery simulation",
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  view.rerender(
    <DeliveryWorkspace
      catalog={catalogSchema.parse(catalog)}
      token=""
      onExit={() => {}}
    />,
  );
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ message: "scene graph unavailable" }), {
        status: 409,
      }),
    ),
  );
  fireEvent.change(screen.getByRole("combobox", { name: "Scenario" }), {
    target: { value: "solo" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Run delivery simulation" }),
  );
  expect(await screen.findByRole("alert")).toHaveProperty(
    "textContent",
    "scene graph unavailable",
  );
});

it("defaults to the fleet showcase and keeps the legacy distributed preset", () => {
  render(
    <DeliveryWorkspace
      catalog={catalogSchema.parse(catalog)}
      token=""
      onExit={() => {}}
    />,
  );
  const option = screen.getByRole("option", {
    name: "Fleet showcase · 12 riders / 16 orders",
  }) as HTMLOptionElement;
  expect(option.selected).toBe(true);
  expect(option.disabled).toBe(false);
  expect(screen.queryByLabelText("rider node")).toBeNull();
  expect(screen.getByText(/Graph-qualified fleet starts/)).toBeTruthy();
});

it("opens saved evidence without a catalog or API, resets speed, and seeks both ways", async () => {
  const fetch = vi.fn();
  vi.stubGlobal("fetch", fetch);
  render(<DeliveryWorkspace token="" onExit={() => {}} />);
  const upload = () =>
    fireEvent.change(screen.getByLabelText("Open saved replay"), {
      target: { files: [{ size: raw.length, text: async () => raw }] },
    });
  upload();
  await screen.findByLabelText("Presentation speed");
  expect(
    (screen.getByLabelText("Presentation speed") as HTMLSelectElement).value,
  ).toBe("1");
  expect(
    screen.getByRole("button", { name: "Play simulation presentation" }),
  ).toBeTruthy();
  expect(
    Array.from(
      (screen.getByLabelText("Presentation speed") as HTMLSelectElement)
        .options,
    ).map((o) => o.value),
  ).toEqual(["1", "2", "5", "10"]);
  fireEvent.change(screen.getByLabelText("Presentation speed"), {
    target: { value: "10" },
  });
  fireEvent.change(screen.getByLabelText("Presentation seconds"), {
    target: { value: "125" },
  });
  expect(screen.getByText("Accepted simulation endpoint")).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Simulation presentation time"), {
    target: { value: "1" },
  });
  expect(
    screen.getByText("Simulated position · uniform route progress"),
  ).toBeTruthy();
  fireEvent.click(
    screen.getByRole("button", { name: "Expand Delivery simulation" }),
  );
  upload();
  await waitFor(() =>
    expect(
      (screen.getByLabelText("Presentation speed") as HTMLSelectElement).value,
    ).toBe("1"),
  );
  expect(fetch).not.toHaveBeenCalled();
});

it("rejects unsupported saved evidence while preserving the existing scene", async () => {
  render(<DeliveryWorkspace token="" onExit={() => {}} />);
  const upload = (text: string) =>
    fireEvent.change(screen.getByLabelText("Open saved replay"), {
      target: { files: [{ size: text.length, text: async () => text }] },
    });
  upload(raw);
  await screen.findByLabelText("Presentation speed");
  fireEvent.click(
    screen.getByRole("button", { name: "Expand Delivery simulation" }),
  );
  upload('{"schema_version":99}');
  expect(await screen.findByRole("alert")).toHaveProperty(
    "textContent",
    "Unsupported replay: expected downloaded delivery scene evidence",
  );
  expect(screen.getByLabelText("Presentation speed")).toBeTruthy();
});

it("opens a recorded 24-rider fleet without an API and keeps every rider inspectable", async () => {
  const fleet = await import("../public/demos/fleet-24.json?raw");
  const fetch = vi.fn().mockResolvedValue(new Response(fleet.default));
  vi.stubGlobal("fetch", fetch);
  render(<DeliveryWorkspace token="" onExit={() => {}} />);
  fireEvent.change(screen.getByLabelText("Recorded fleet"), {
    target: { value: "24" },
  });
  fireEvent.click(
    screen.getByRole("button", { name: "Open recorded fleet demo" }),
  );
  await screen.findByLabelText("Presentation speed");
  expect(fetch.mock.calls[0][0]).toBe("/demos/fleet-24.json");
  expect(fetch.mock.calls[0][1]).not.toHaveProperty("headers");
  expect(
    (screen.getByLabelText("Presentation speed") as HTMLSelectElement).value,
  ).toBe("1");
  expect(
    screen.getAllByRole("button", { name: /^Inspect rider \d+$/ }),
  ).toHaveLength(24);
  expect(
    screen.getByRole("button", { name: "Play simulation presentation" }),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Inspect rider 24" }));
  expect(
    screen.getByRole("button", { name: "Collapse Rider 24" }),
  ).toBeTruthy();
  fireEvent.change(screen.getByLabelText("Presentation seconds"), {
    target: { value: "900" },
  });
  expect(screen.getByText("28 delivered")).toBeTruthy();
}, 15000);

it("reports recorded fleet load failures without opening a scene", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response("unavailable", { status: 503 })),
  );
  render(<DeliveryWorkspace token="" onExit={() => {}} />);
  fireEvent.click(
    screen.getByRole("button", { name: "Open recorded fleet demo" }),
  );
  expect((await screen.findByRole("alert")).textContent).toContain(
    "Recorded fleet demo unavailable (503)",
  );
  expect(screen.queryByLabelText("Presentation speed")).toBeNull();
});
