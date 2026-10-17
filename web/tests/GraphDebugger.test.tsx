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
import type { GraphDebugger } from "../src/useGraphDebugger";
import type { GraphSelection } from "../src/graphContracts";
import type { RouteResult } from "../src/contracts";
import catalog from "./catalog.json";
import graph from "./graph-page.json";
import noTraffic from "./graph-no-traffic.json";
import forward from "./forward.json";
vi.mock("../src/MapView", () => ({
  MapView: ({
    debug,
    result,
    onInspectGraph,
  }: {
    debug: GraphDebugger;
    result?: RouteResult;
    onInspectGraph: (s: GraphSelection) => void;
  }) => (
    <div
      data-testid="map"
      data-developer={debug.enabled}
      data-selection={
        debug.selection ? `${debug.selection.kind}:${debug.selection.id}` : ""
      }
      data-view={debug.view?.graph_view_id}
      data-evaluation={result?.evidence.evaluation_id}
    >
      <button onClick={() => onInspectGraph({ kind: "node", id: 9 })}>
        Pick test node
      </button>
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
  fetcher = vi.fn(
    async (path: string) =>
      new Response(
        JSON.stringify(
          path.includes("/anchors")
            ? catalog
            : path.includes("objective=traffic_aware")
              ? noTraffic
              : path.includes("/graph?")
                ? graph
                : forward,
        ),
      ),
  );
  vi.stubGlobal("fetch", fetcher);
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
async function ready() {
  render(<App />);
  await waitFor(() => expect(screen.getByLabelText("Origin")).toBeEnabled());
}
it("inspects nodes, edges, layers and unavailable diagnostics without rewriting routing evidence", async () => {
  await ready();
  fireEvent.change(screen.getByLabelText("Origin"), { target: { value: "0" } });
  fireEvent.change(screen.getByLabelText("Destination"), {
    target: { value: "35" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Find route" }));
  await screen.findByText("212 m");
  const evaluation = screen.getByTestId("map").getAttribute("data-evaluation");
  fireEvent.click(screen.getByRole("button", { name: "Developer graph" }));
  await screen.findByLabelText("Inspect graph edge");
  fireEvent.change(screen.getByLabelText("Inspect graph edge"), {
    target: { value: "3" },
  });
  expect(
    screen.getByRole("region", { name: "Graph entity inspector" }),
  ).toHaveTextContent("0→9");
  expect(
    screen.getByText("Unavailable · not retained by this graph"),
  ).toBeInTheDocument();
  const calls = fetcher.mock.calls.length;
  fireEvent.click(screen.getByLabelText("Direction arrows"));
  fireEvent.click(screen.getByLabelText("Objective weights"));
  fireEvent.click(screen.getByRole("button", { name: "Pick test node" }));
  expect(screen.getByTestId("map")).toHaveAttribute("data-selection", "node:9");
  expect(screen.getByText("Node 9", { exact: true })).toBeInTheDocument();
  fireEvent.click(screen.getByText("Road class & search diagnostics"));
  expect(
    screen.getByLabelText("Explored-node overlay unavailable"),
  ).toBeDisabled();
  expect(
    screen.getByText(forward.routes[0].search.counter_semantics),
  ).toBeInTheDocument();
  expect(fetcher).toHaveBeenCalledTimes(calls);
  fireEvent.click(screen.getByRole("button", { name: "Routing" }));
  expect(screen.getByLabelText("Origin")).toHaveValue("0");
  expect(screen.getByLabelText("Destination")).toHaveValue("35");
  expect(screen.getByTestId("map")).toHaveAttribute(
    "data-evaluation",
    evaluation,
  );
  expect(
    fetcher.mock.calls.filter((c) => c[0] === "/v1/visualization/routes"),
  ).toHaveLength(1);
});
it("shows missing traffic cost explicitly and rejects stale captures after objective replacement", async () => {
  await ready();
  let respond!: (r: Response) => void;
  fetcher.mockImplementationOnce(
    () =>
      new Promise<Response>((resolve) => {
        respond = resolve;
      }),
  );
  fireEvent.click(screen.getByRole("button", { name: "Developer graph" }));
  expect(screen.getByText("Capturing graph page…")).toBeInTheDocument();
  const signal = fetcher.mock.calls.at(-1)![1].signal;
  fireEvent.change(screen.getByLabelText("Graph cost objective"), {
    target: { value: "traffic_aware" },
  });
  await screen.findByLabelText("Inspect graph edge");
  expect(signal.aborted).toBe(true);
  await act(async () => respond(new Response(JSON.stringify(graph))));
  fireEvent.change(screen.getByLabelText("Inspect graph edge"), {
    target: { value: "0" },
  });
  expect(
    screen.getByText("Unavailable · adopt traffic on the engine"),
  ).toBeInTheDocument();
  expect(
    screen.getByText("Unavailable · no adopted snapshot"),
  ).toBeInTheDocument();
  expect(screen.getByTestId("map")).toHaveAttribute(
    "data-view",
    noTraffic.graph_view_id,
  );
});
it("reports invalid graph evidence and retry availability without showing stale nodes", async () => {
  await ready();
  fetcher.mockResolvedValueOnce(
    new Response(JSON.stringify({ ...graph, schema_version: 99 })),
  );
  fireEvent.click(screen.getByRole("button", { name: "Developer graph" }));
  await screen.findByText("Graph unavailable");
  expect(screen.queryByLabelText("Inspect graph edge")).not.toBeInTheDocument();
  expect(screen.getByTestId("map")).not.toHaveAttribute("data-view");
  fireEvent.click(screen.getByRole("button", { name: "Retry graph capture" }));
  await screen.findByLabelText("Inspect graph edge");
});

it("replaces bounded pages, preserves cursor history, and never merges old edge selections", async () => {
  await ready();
  // Synthetic transport pagination using real directed shapes; producer page caps are tested in Rust.
  const first = structuredClone({
    ...graph,
    edge_page: { ...graph.edge_page, next_offset: 114 as number | null },
  });
  first.edge_count = 228;
  first.edge_page.next_offset = 114;
  first.graph_view_id = `graph-view-v1:${"0".repeat(64)}`;
  const second = structuredClone(first);
  second.edges.forEach((e) => {
    e.edge += 114;
  });
  second.edge_page = { offset: 114, end: 228, next_offset: null };
  second.graph_view_id = `graph-view-v1:${"1".repeat(64)}`;
  fetcher.mockImplementation(
    async (path: string) =>
      new Response(
        JSON.stringify(path.includes("edge_offset=114") ? second : first),
      ),
  );
  fireEvent.click(screen.getByRole("button", { name: "Developer graph" }));
  await screen.findByLabelText("Inspect graph edge");
  fireEvent.change(screen.getByLabelText("Inspect graph edge"), {
    target: { value: "3" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Next edge page" }));
  await waitFor(() =>
    expect(
      screen
        .getByLabelText("Inspect graph edge")
        .querySelector('option[value="114"]'),
    ).not.toBeNull(),
  );
  expect(
    screen
      .getByLabelText("Inspect graph edge")
      .querySelector('option[value="3"]'),
  ).toBeNull();
  expect(screen.getByTestId("map")).toHaveAttribute("data-selection", "");
  fireEvent.change(screen.getByLabelText("Inspect graph edge"), {
    target: { value: "114" },
  });
  expect(screen.getByTestId("map")).toHaveAttribute(
    "data-selection",
    "edge:114",
  );
  fireEvent.click(screen.getByRole("button", { name: "Previous edge page" }));
  await waitFor(() =>
    expect(
      screen
        .getByLabelText("Inspect graph edge")
        .querySelector('option[value="3"]'),
    ).not.toBeNull(),
  );
  expect(
    screen
      .getByLabelText("Inspect graph edge")
      .querySelector('option[value="114"]'),
  ).toBeNull();
  expect(
    screen.getByRole("button", { name: "Previous edge page" }),
  ).toBeDisabled();
});
