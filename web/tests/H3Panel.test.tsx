// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { H3Panel } from "../src/H3Panel";
import { decodeRoute, type RouteRequest } from "../src/contracts";
import forward from "./forward.json";
import input from "./forward-request.json";

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});

it("compares real route envelopes despite different heuristic provenance and switches the map without reindexing or rerouting", async () => {
  const result = decodeRoute(JSON.stringify(forward), input as RouteRequest);
  const dijkstra = structuredClone(forward);
  dijkstra.provenance.heuristic = null as unknown as string;
  dijkstra.routes[0].provenance.heuristic = null as unknown as string;
  dijkstra.routes[0].search.algorithm = "dijkstra";
  const fetcher = vi
    .fn()
    .mockResolvedValueOnce(new Response(JSON.stringify(dijkstra)))
    .mockResolvedValueOnce(new Response(JSON.stringify(forward)));
  vi.stubGlobal("fetch", fetcher);
  const onRoute = vi.fn();
  render(
    <H3Panel
      cells={[]}
      resolution={9}
      onResolution={vi.fn()}
      result={result}
      token=""
      onRoute={onRoute}
      truncated={false}
      unsupported={0}
    />,
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Compare Dijkstra and A*" }),
  );
  await screen.findByText(/equal within numeric tolerance/);
  expect(fetcher).toHaveBeenCalledTimes(2);
  expect(JSON.parse(fetcher.mock.calls[0][1].body).algorithm).toBe("dijkstra");
  expect(JSON.parse(fetcher.mock.calls[1][1].body).algorithm).toBe("astar");
  fireEvent.click(
    screen.getByRole("button", { name: /Dijkstra.*expanded states/ }),
  );
  expect(onRoute.mock.calls[0][0].request.algorithm).toBe("dijkstra");
  expect(fetcher).toHaveBeenCalledTimes(2);
});

it("rejects comparisons when routing context changes between requests", async () => {
  const result = decodeRoute(JSON.stringify(forward), input as RouteRequest);
  const changed = structuredClone(forward);
  changed.provenance.traffic_digest = "different" as unknown as null;
  changed.routes[0].provenance.traffic_digest = "different" as unknown as null;
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValueOnce(new Response(JSON.stringify(forward)))
      .mockResolvedValueOnce(new Response(JSON.stringify(changed))),
  );
  render(
    <H3Panel
      cells={[]}
      resolution={9}
      onResolution={vi.fn()}
      result={result}
      token=""
      onRoute={vi.fn()}
      truncated={false}
      unsupported={0}
    />,
  );
  fireEvent.click(
    screen.getByRole("button", { name: "Compare Dijkstra and A*" }),
  );
  await waitFor(() => expect(screen.getByRole("alert")).toBeInTheDocument());
  expect(
    screen.queryByText(/equal within numeric tolerance/),
  ).not.toBeInTheDocument();
});
