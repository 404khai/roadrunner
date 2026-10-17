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
import alternatives from "./alternatives.json";
import { type RouteResult } from "../src/contracts";
import noTravel from "./no-travel.json";
vi.mock("../src/MapView", () => ({
  MapView: ({
    progress,
    result,
    visible,
    segment,
  }: {
    progress: number;
    result?: RouteResult;
    visible: Set<string>;
    segment?: number;
  }) => (
    <div
      data-testid="map-progress"
      data-evaluation={result?.evidence.evaluation_id}
      data-visible={visible.size}
      data-segment={segment}
    >
      {progress}
    </div>
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

it("compares, selects, hides and inspects real alternatives without rerouting or rewriting rank", async () => {
  await ready();
  fireEvent.change(screen.getByLabelText("Destination"), {
    target: { value: "22" },
  });
  fireEvent.change(screen.getByLabelText("Route results"), {
    target: { value: "3" },
  });
  expect(screen.getByLabelText("Algorithm")).toBeDisabled();
  fetcher.mockResolvedValueOnce(new Response(JSON.stringify(alternatives)));
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  await screen.findByRole("button", { name: "Select Route B" });
  const count = fetcher.mock.calls.length,
    bytes = JSON.stringify(alternatives);
  expect(fetcher.mock.calls.at(-1)![0]).toBe(
    "/v1/visualization/routes/alternatives",
  );
  expect(JSON.parse(fetcher.mock.calls.at(-1)![1].body)).toMatchObject({
    algorithm: "dijkstra",
    alternatives: 3,
  });
  fireEvent.click(screen.getByRole("button", { name: "Select Route B" }));
  expect(screen.getByTestId("map-progress")).toHaveAttribute(
    "data-evaluation",
    alternatives.routes[1].route.evaluation_id,
  );
  expect(screen.getByText("ENGINE PRIMARY")).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Hide Route B" }));
  expect(
    screen.getByRole("button", { name: "Select Route A" }),
  ).toHaveAttribute("aria-pressed", "true");
  expect(screen.getByTestId("map-progress")).toHaveAttribute(
    "data-visible",
    "2",
  );
  fireEvent.click(screen.getByRole("button", { name: "Select Route B" }));
  expect(screen.getByTestId("map-progress")).toHaveAttribute(
    "data-visible",
    "3",
  );
  fireEvent.click(screen.getByText("Inspect selected route segments"));
  fireEvent.click(screen.getByRole("button", { name: "Next segment" }));
  expect(screen.getByTestId("map-progress")).toHaveAttribute(
    "data-segment",
    "1",
  );
  fireEvent.change(screen.getByLabelText("Traversal number"), {
    target: { value: "" },
  });
  expect(screen.getByLabelText("Traversal number")).toHaveDisplayValue("");
  fireEvent.change(screen.getByLabelText("Traversal number"), {
    target: { value: "4" },
  });
  expect(screen.getByTestId("map-progress")).toHaveAttribute(
    "data-segment",
    "3",
  );
  fireEvent.click(screen.getByRole("button", { name: "Replay reveal" }));
  fireEvent.click(screen.getByRole("button", { name: /SPATIAL WORKSPACE/ }));
  fireEvent.click(screen.getByRole("button", { name: "View Route C" }));
  expect(screen.getByTestId("map-progress")).toHaveAttribute(
    "data-evaluation",
    alternatives.routes[2].route.evaluation_id,
  );
  expect(fetcher).toHaveBeenCalledTimes(count);
  expect(JSON.stringify(alternatives)).toBe(bytes);
});
it("labels partial comparisons, unavailable optional inspection, and invalidates on objective change", async () => {
  await ready();
  fireEvent.change(screen.getByLabelText("Destination"), {
    target: { value: "22" },
  });
  fireEvent.change(screen.getByLabelText("Route results"), {
    target: { value: "3" },
  });
  const partial = structuredClone(alternatives);
  partial.summary.termination = "budget";
  partial.summary.truncated = true;
  partial.routes.forEach((m) => {
    m.segment_inspection.schema_version = 2;
  });
  fetcher.mockResolvedValueOnce(new Response(JSON.stringify(partial)));
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  await screen.findByText("Partial comparison · search budget reached");
  fireEvent.click(screen.getByText("Inspect selected route segments"));
  expect(screen.getAllByText("Unavailable").length).toBeGreaterThan(0);
  fireEvent.change(screen.getByLabelText("Optimize for"), {
    target: { value: "distance" },
  });
  expect(
    screen.queryByRole("button", { name: "Select Route A" }),
  ).not.toBeInTheDocument();
  expect(screen.getByTestId("map-progress")).not.toHaveAttribute(
    "data-evaluation",
  );
});
