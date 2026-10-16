// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { App } from "../src/App";
import catalog from "./catalog.json";
import forward from "./forward.json";
import noTravel from "./no-travel.json";
vi.mock("../src/MapView", () => ({
  MapView: ({ progress }: { progress: number }) => (
    <div data-testid="map-progress">{progress}</div>
  ),
}));
let fetcher: ReturnType<typeof vi.fn>;
beforeEach(() => {
  vi.stubGlobal("matchMedia", () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
  vi.stubGlobal("requestAnimationFrame", () => 1);
  vi.stubGlobal("cancelAnimationFrame", vi.fn());
  fetcher = vi.fn(async () => new Response(JSON.stringify(catalog)));
  vi.stubGlobal("fetch", fetcher);
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
async function ready() {
  render(<App />);
  await waitFor(() => expect(screen.getByLabelText("Origin")).toBeEnabled());
  fireEvent.change(screen.getByLabelText("Origin"), { target: { value: "0" } });
  fireEvent.change(screen.getByLabelText("Destination"), {
    target: { value: "35" },
  });
}
it("consumes actual evidence, replays, pauses, skips and reduces motion without another routing request", async () => {
  await ready();
  fetcher.mockResolvedValueOnce(new Response(JSON.stringify(forward)));
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  await screen.findByText("212 m");
  expect(screen.getByText("38 sec")).toBeInTheDocument();
  const count = fetcher.mock.calls.length;
  fireEvent.click(await screen.findByRole("button", { name: "Skip reveal" }));
  expect(screen.getByTestId("map-progress")).toHaveTextContent("1");
  fireEvent.click(screen.getByRole("button", { name: "Replay reveal" }));
  fireEvent.click(screen.getByRole("button", { name: "Pause reveal" }));
  fireEvent.click(screen.getByRole("button", { name: "Reduce motion" }));
  expect(
    screen.getByText("Reduced motion · static result"),
  ).toBeInTheDocument();
  expect(screen.getByTestId("map-progress")).toHaveTextContent("1");
  expect(fetcher).toHaveBeenCalledTimes(count);
  const query = JSON.parse(fetcher.mock.calls[1][1].body);
  expect(query.origin.node).toBe(0);
  expect(query.destination.node).toBe(35);
  expect(query.origin.graph_digest).toBe(catalog.graph.snapshot_digest);
});
it("distinguishes pending computation, no route and unavailable traffic without a successful line", async () => {
  await ready();
  let respond!: (value: Response) => void;
  fetcher.mockImplementationOnce(
    () =>
      new Promise<Response>((resolve) => {
        respond = resolve;
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  expect(screen.getByText("Finding your route")).toBeInTheDocument();
  expect(screen.queryByTestId("reveal-controls")).not.toBeInTheDocument();
  await act(async () =>
    respond(
      new Response(
        JSON.stringify({
          code: "no_route",
          message: "No route exists between the supplied nodes",
        }),
        { status: 404 },
      ),
    ),
  );
  expect(await screen.findByText("No route found")).toBeInTheDocument();
  expect(screen.queryByText("SELECTED ROUTE")).not.toBeInTheDocument();
  fetcher.mockResolvedValueOnce(
    new Response(
      JSON.stringify({
        code: "invalid_input",
        message: "traffic context unavailable",
      }),
      { status: 422 },
    ),
  );
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  expect(
    await screen.findByText("traffic context unavailable"),
  ).toBeInTheDocument();
});
it("suppresses stale route responses when anchors change", async () => {
  await ready();
  let respond!: (value: Response) => void;
  fetcher.mockImplementationOnce(
    () =>
      new Promise<Response>((resolve) => {
        respond = resolve;
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  const signal = fetcher.mock.calls.at(-1)![1].signal;
  fireEvent.change(screen.getByLabelText("Destination"), {
    target: { value: "22" },
  });
  expect(signal.aborted).toBe(true);
  await act(async () => respond(new Response(JSON.stringify(forward))));
  expect(screen.queryByText("212 m")).not.toBeInTheDocument();
  expect(screen.queryByTestId("reveal-controls")).not.toBeInTheDocument();
});
it("presents no-travel without a moving line or replay control", async () => {
  await ready();
  fireEvent.change(screen.getByLabelText("Destination"), {
    target: { value: "0" },
  });
  fetcher.mockResolvedValueOnce(new Response(JSON.stringify(noTravel)));
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  expect(await screen.findByText("Already at destination")).toBeInTheDocument();
  expect(screen.getByText("0 m")).toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: "Replay reveal" }),
  ).not.toBeInTheDocument();
});
it("handles failed authentication and lets the user retry with an in-memory token", async () => {
  fetcher.mockResolvedValueOnce(
    new Response(
      JSON.stringify({
        code: "unauthorized",
        message: "valid bearer credential required",
      }),
      { status: 401 },
    ),
  );
  render(<App />);
  await screen.findByText("Engine unavailable");
  fireEvent.click(screen.getByRole("button", { name: "Connection settings" }));
  fireEvent.change(screen.getByLabelText("Bearer token"), {
    target: { value: "local-test-token" },
  });
  fireEvent.click(screen.getByRole("button", { name: /^Connect$/ }));
  await waitFor(() => expect(screen.getByLabelText("Origin")).toBeEnabled());
  expect(fetcher.mock.calls.at(-1)![1].headers.Authorization).toBe(
    "Bearer local-test-token",
  );
  expect(localStorage.length).toBe(0);
  expect(sessionStorage.length).toBe(0);
});

it("collapses the floating workspace without discarding route evidence or rerouting", async () => {
  await ready();
  fetcher.mockResolvedValueOnce(new Response(JSON.stringify(forward)));
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  await screen.findByText("212 m");
  const calls = fetcher.mock.calls.length;
  const toggle = screen.getByRole("button", { name: /SPATIAL WORKSPACE/ });
  const progress = screen.getByTestId("map-progress").textContent;
  fireEvent.click(toggle);
  expect(toggle).toHaveAttribute("aria-expanded", "false");
  expect(
    document.getElementById("spatial-workspace-content"),
  ).not.toBeVisible();
  expect(document.querySelector(".floating-metrics")).toHaveTextContent(
    "212 m",
  );
  expect(screen.getByTestId("map-progress").textContent).toBe(progress);
  fireEvent.click(toggle);
  expect(screen.getByLabelText("Origin")).toHaveValue("0");
  expect(screen.getByLabelText("Destination")).toHaveValue("35");
  expect(fetcher).toHaveBeenCalledTimes(calls);
});
