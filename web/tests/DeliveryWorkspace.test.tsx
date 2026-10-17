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

it("offers the pinned distributed demo explicitly and hides irrelevant node selectors", () => {
  render(
    <DeliveryWorkspace
      catalog={catalogSchema.parse(catalog)}
      token=""
      onExit={() => {}}
    />,
  );
  const option = screen.getByRole("option", {
    name: "Distributed Fleet Demo · 6 riders",
  }) as HTMLOptionElement;
  expect(option.selected).toBe(true);
  expect(option.disabled).toBe(false);
  expect(screen.queryByLabelText("rider node")).toBeNull();
  expect(screen.getByText(/Six fixed graph-node starts/)).toBeTruthy();
});
