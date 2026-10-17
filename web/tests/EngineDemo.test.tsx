// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { URL as NodeURL } from "node:url";
import { readFileSync } from "node:fs";
import { EngineDemo } from "../src/EngineDemo";
vi.mock("../src/DeliveryMap", () => ({
  DeliveryMap: ({ at }: { at: number }) => (
    <div data-testid="delivery-time">{at}</div>
  ),
}));
vi.mock("../src/DispatchMap", () => ({
  DispatchMap: ({ sequence }: { sequence: number }) => (
    <div data-testid="dispatch-progress">{sequence}</div>
  ),
}));
vi.mock("../src/MapView", () => ({ style: {} }));
const motion = vi.hoisted(() => ({ reduced: true }));
vi.mock("../src/useReveal", () => ({ useReducedMotion: () => motion.reduced }));
const fetcher = vi.fn();
beforeEach(() => {
  motion.reduced = true;
  vi.stubGlobal("fetch", fetcher);
  fetcher.mockImplementation((url: string) =>
    Promise.resolve(
      new Response(
        readFileSync(new NodeURL("../public" + url, import.meta.url), "utf8"),
      ),
    ),
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});
it("decodes the public engine runs, scrubs accepted time and switches without API calls", async () => {
  render(<EngineDemo />);
  await screen.findByTestId("delivery-time");
  expect(screen.getByRole("button", { name: "Play demo" })).toBeDisabled();
  fireEvent.change(screen.getByLabelText("Demo playback"), {
    target: { value: "0.5" },
  });
  expect(Number(screen.getByTestId("delivery-time").textContent)).toBeCloseTo(
    145.965089,
  );
  fireEvent.click(screen.getByRole("tab", { name: "Dispatch visualization" }));
  expect(screen.getByTestId("dispatch-progress")).toHaveTextContent("0");
  fireEvent.change(screen.getByLabelText("Demo playback"), {
    target: { value: "1" },
  });
  expect(screen.getByTestId("dispatch-progress")).toHaveTextContent("1");
  expect(
    screen.getByRole("link", { name: "Open workspace ↗" }),
  ).toHaveAttribute("href", "/workspace?mode=dispatch");
  expect(fetcher).toHaveBeenCalledTimes(5);
  expect(
    fetcher.mock.calls.every(([url]) => String(url).startsWith("/demos/")),
  ).toBe(true);
});
it("reports missing evidence and retries without showing invented positions", async () => {
  fetcher.mockResolvedValue(new Response("missing", { status: 404 }));
  render(<EngineDemo />);
  await screen.findByRole("alert");
  expect(screen.queryByTestId("delivery-time")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Retry evidence" }));
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(10));
});

it("autoplays a normal-motion preview and restarts playback when changing tabs", async () => {
  motion.reduced = false;
  render(<EngineDemo />);
  await screen.findByTestId("delivery-time");
  expect(screen.getByRole("button", { name: "Pause" })).toBeEnabled();
  await waitFor(() =>
    expect(
      Number(
        (screen.getByLabelText("Demo playback") as HTMLInputElement).value,
      ),
    ).toBeGreaterThan(0),
  );
  fireEvent.click(screen.getByRole("button", { name: "Pause" }));
  fireEvent.click(screen.getByRole("tab", { name: "Dispatch visualization" }));
  expect(screen.getByRole("button", { name: "Pause" })).toBeEnabled();
});
