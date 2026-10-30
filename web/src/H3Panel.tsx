import { useEffect, useRef, useState } from "react";
import {
  apiText,
  decodeRoute,
  type RouteRequest,
  type RouteResult,
} from "./contracts";
import type { H3Bucket } from "./h3Overlay";
import { distanceLabel, durationLabel } from "./geometry";

export function H3Panel({
  cells,
  resolution,
  onResolution,
  result,
  token,
  onRoute,
  truncated,
  unsupported,
}: {
  cells: H3Bucket[];
  resolution: number;
  onResolution: (value: number) => void;
  result?: RouteResult;
  token: string;
  onRoute: (value: RouteResult) => void;
  truncated: boolean;
  unsupported: number;
}) {
  const [pair, setPair] = useState<RouteResult[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const abort = useRef<AbortController | null>(null);
  const departure =
    result?.request.departure_time ??
    (result
      ? new Date(
          result.evidence.provenance.departure_seconds * 1000,
        ).toISOString()
      : undefined);
  const context = result
    ? JSON.stringify({
        ...result.request,
        departure_time: departure,
        algorithm: undefined,
      })
    : "";
  useEffect(() => {
    abort.current?.abort();
    setPair([]);
    setError("");
    setBusy(false);
    return () => abort.current?.abort();
  }, [context, token]);
  async function compare() {
    if (!result) return;
    abort.current?.abort();
    const controller = new AbortController();
    abort.current = controller;
    setBusy(true);
    setError("");
    setPair([]);
    try {
      const results = [];
      for (const algorithm of ["dijkstra", "astar"] as const) {
        const request: RouteRequest = {
          ...result.request,
          departure_time: departure,
          algorithm,
          alternatives: undefined,
        };
        const raw = await apiText(
          "/v1/visualization/routes",
          token,
          controller.signal,
          request,
        );
        results.push(decodeRoute(raw, request));
      }
      if (controller.signal.aborted) return;
      const contextOf = (value: RouteResult) =>
        JSON.stringify({
          graph: value.evidence.geometry.graph_digest,
          profile: value.evidence.routing_profile,
          jurisdiction: value.evidence.jurisdiction_policy,
          provenance: { ...value.evidence.provenance, heuristic: null },
        });
      if (contextOf(results[0]) !== contextOf(results[1]))
        throw new Error(
          "Routing context changed; this pair cannot be compared. Run again.",
        );
      setPair(results);
    } catch (e) {
      if (!controller.signal.aborted)
        setError(e instanceof Error ? e.message : "Comparison unavailable");
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  return (
    <section className="h3-panel" aria-label="H3 inspection">
      <h2>H3 geographic index</h2>
      <p>
        Cells group loaded graph nodes and selected-route geometry vertices. H3
        does not compute a road route.
      </p>
      <label>
        Cell resolution
        <select
          aria-label="H3 resolution"
          value={resolution}
          onChange={(e) => onResolution(Number(e.target.value))}
        >
          {[6, 7, 8, 9, 10, 11, 12].map((value) => (
            <option key={value} value={value}>
              {value}
            </option>
          ))}
        </select>
      </label>
      <p>
        <strong>{cells.length} occupied cells</strong> ·{" "}
        {cells.reduce((n, c) => n + c.nodes.length, 0)} loaded nodes
      </p>
      <p className="form-note">
        Pale outline: loaded-node cells. Gold outline: route-vertex cells.
        Vertex samples do not describe all cells crossed by the route.
      </p>
      {(truncated || unsupported > 0) && (
        <p role="status">
          Showing at most 1,000 cells. {unsupported} polar or seam-crossing
          polygons withheld.
        </p>
      )}
      <button
        className="primary-button"
        type="button"
        disabled={!result || busy}
        onClick={() => void compare()}
      >
        {busy ? "Comparing…" : "Compare Dijkstra and A*"}
      </button>
      {!result && (
        <p className="form-note">
          Find a single route first, then compare both searches on the same
          request.
        </p>
      )}
      {error && <p role="alert">{error}</p>}
      {pair.length === 2 && (
        <div aria-live="polite">
          <p>
            Optimal cost:{" "}
            {Math.abs(
              pair[0].evidence.objective.value -
                pair[1].evidence.objective.value,
            ) <=
            1e-8 * Math.max(1, pair[0].evidence.objective.value)
              ? "equal within numeric tolerance"
              : "different — inspect evidence"}
            . Tied paths may differ.
          </p>
          {pair.map((value) => (
            <button
              type="button"
              className="h3-search-result"
              key={value.request.algorithm}
              onClick={() => onRoute(value)}
              aria-pressed={
                result?.request.algorithm === value.request.algorithm
              }
            >
              <strong>
                {value.request.algorithm === "astar" ? "A*" : "Dijkstra"}
              </strong>
              <span>
                {String(value.evidence.search.expanded_states)} expanded states
              </span>
              <span>
                {distanceLabel(value.evidence.distance_meters)} ·{" "}
                {durationLabel(value.evidence.predicted_elapsed_seconds)}
              </span>
              <span>
                Cost {value.evidence.objective.value.toFixed(3)}{" "}
                {value.evidence.objective_unit}
              </span>
            </button>
          ))}
          <p className="form-note">
            Engine counters from two requests, not a latency benchmark. H3 cell
            counts are a separate geographic metric.
          </p>
        </div>
      )}
      <details>
        <summary>Inspect cell IDs and membership</summary>
        <div className="h3-cell-list">
          {cells.slice(0, 1000).map((cell) => (
            <div key={cell.cell}>
              <code>{cell.cell}</code>
              <span>
                Nodes: {cell.nodes.join(", ") || "none"} · route vertices:{" "}
                {cell.routeVertices}
              </span>
            </div>
          ))}
          {!cells.length && <p>No loaded coordinates to index.</p>}
        </div>
      </details>
    </section>
  );
}
