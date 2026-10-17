import { useEffect, useRef } from "react";
import {
  Bug,
  ChevronLeft,
  ChevronRight,
  Download,
  LoaderCircle,
  RefreshCw,
} from "lucide-react";
import type { GraphDebugger } from "./useGraphDebugger";
import type { RouteEvidence } from "./contracts";
export function GraphPanel({
  debug,
  route,
}: {
  debug: GraphDebugger;
  route?: RouteEvidence;
}) {
  const { view, layers, selection } = debug;
  const inspector = useRef<HTMLElement>(null);
  useEffect(() => {
    if (selection)
      inspector.current?.scrollIntoView?.({
        block: "nearest",
        behavior: "instant",
      });
  }, [selection]);
  const edge =
    selection?.kind === "edge"
      ? view?.edges.find((e) => e.edge === selection.id)
      : undefined;
  const node =
    selection?.kind === "node"
      ? view?.nodes.find((n) => n.node === selection.id)
      : undefined;
  function download() {
    if (!view) return;
    const url = URL.createObjectURL(
      new Blob([view.raw], { type: "application/json" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = `${view.graph_view_id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }
  return (
    <section className="graph-panel" aria-label="Developer graph debugger">
      <div className="panel-intro">
        <span className="developer-kicker">
          <Bug size={13} /> DEVELOPER MODE
        </span>
        <h1>Graph debugger</h1>
        <p>Inspect the network behind the route.</p>
      </div>
      <div className="graph-panel-content">
        <div className="graph-control-heading">
          <span className="section-label">GRAPH LAYERS</span>
          <button
            className="icon-button"
            aria-label="Refresh graph snapshot"
            disabled={debug.busy}
            onClick={debug.refresh}
          >
            <RefreshCw size={15} />
          </button>
        </div>
        <div className="graph-layer-grid">
          {(
            [
              ["nodes", "Nodes"],
              ["edges", "Directed edges"],
              ["directions", "Direction arrows"],
              ["weights", "Objective weights"],
            ] as const
          ).map(([key, label]) => (
            <label key={key}>
              <input
                type="checkbox"
                checked={layers[key]}
                onChange={(e) =>
                  debug.setLayers((l) => ({ ...l, [key]: e.target.checked }))
                }
              />
              {label}
            </label>
          ))}
        </div>
        <label className="graph-objective">
          Inspect edge cost
          <select
            aria-label="Graph cost objective"
            value={debug.objective}
            onChange={(e) =>
              debug.setObjective(e.target.value as GraphDebugger["objective"])
            }
          >
            <option value="travel_time">Free-flow travel time</option>
            <option value="distance">Distance</option>
            <option value="traffic_aware">Traffic-aware time</option>
          </select>
        </label>
        <p className="graph-note">
          A captured graph view, not a live feed. Costs evaluate each edge
          independently under default access. Incoming-turn legality and
          whole-route feasibility are separate.
        </p>
        {debug.busy ? (
          <div className="graph-status" role="status">
            <LoaderCircle size={19} className="spinner" /> Capturing graph page…
          </div>
        ) : debug.error ? (
          <div className="error-result" role="alert">
            <h2>Graph unavailable</h2>
            <p>{debug.error}</p>
            <button className="secondary-button" onClick={debug.refresh}>
              Retry graph capture
            </button>
          </div>
        ) : !view ? (
          <p className="graph-note">
            Connect an engine to inspect its immutable graph.
          </p>
        ) : (
          <>
            <div className="graph-page-stats">
              <div>
                <strong>{view.nodes.length}</strong>
                <small>nodes loaded</small>
              </div>
              <div>
                <strong>{view.edges.length}</strong>
                <small>directed edges</small>
              </div>
              <div>
                <strong>{view.point_count.toLocaleString()}</strong>
                <small>E7 vertices</small>
              </div>
            </div>
            <div className="graph-pages">
              {(["node", "edge"] as const).map((kind) => {
                const page = kind === "node" ? view.node_page : view.edge_page;
                return (
                  <div key={kind}>
                    <span>
                      {kind === "node" ? "Node" : "Edge"} page{" "}
                      <b>
                        {page.offset === page.end
                          ? "empty"
                          : `${page.offset}–${page.end - 1}`}
                      </b>{" "}
                      / {kind === "node" ? view.node_count : view.edge_count}
                    </span>
                    <button
                      aria-label={`Previous ${kind} page`}
                      disabled={
                        kind === "node"
                          ? !debug.canBackNodes
                          : !debug.canBackEdges
                      }
                      onClick={() => debug.page(kind, false)}
                    >
                      <ChevronLeft size={15} />
                    </button>
                    <button
                      aria-label={`Next ${kind} page`}
                      disabled={page.next_offset === null}
                      onClick={() => debug.page(kind, true)}
                    >
                      <ChevronRight size={15} />
                    </button>
                  </div>
                );
              })}
            </div>
            <p className="graph-note">
              Contiguous graph pages, not a viewport query. Only returned
              entities are shown. Nodes include this node page and exported edge
              endpoints. Refresh or paging replaces the complete capture.
            </p>
            <label className="graph-edge-select">
              Inspect a directed edge
              <select
                aria-label="Inspect graph edge"
                value={edge?.edge ?? ""}
                onChange={(e) =>
                  debug.setSelection({
                    kind: "edge",
                    id: Number(e.target.value),
                  })
                }
              >
                <option value="" disabled>
                  Select an edge or click the map
                </option>
                {view.edges.map((e) => (
                  <option key={e.edge} value={e.edge}>
                    Edge {e.edge} · {e.from} → {e.to} · {e.orientation}
                  </option>
                ))}
              </select>
            </label>
            <section
              ref={inspector}
              className="graph-inspector"
              aria-label="Graph entity inspector"
              aria-live="polite"
            >
              <div className="section-label">
                {edge
                  ? "DIRECTED EDGE"
                  : node
                    ? "GRAPH NODE"
                    : "ENTITY INSPECTOR"}
                <span>
                  {edge
                    ? `#${edge.edge}`
                    : node
                      ? `#${node.node}`
                      : "PICK ON MAP"}
                </span>
              </div>
              {edge ? (
                <>
                  <div className="graph-edge-heading">
                    <strong>
                      {edge.from}
                      <span>→</span>
                      {edge.to}
                    </strong>
                    <span className="graph-status-badge">
                      {edge.cost_status === "traversable"
                        ? "EDGE TRAVERSABLE"
                        : edge.cost_status === "forbidden"
                          ? "DEFAULT ACCESS FORBIDDEN"
                          : "COST UNAVAILABLE"}
                    </span>
                  </div>
                  <dl>
                    <dt>Edge / physical segment</dt>
                    <dd>
                      {edge.edge} / {edge.segment}
                    </dd>
                    <dt>Canonical orientation</dt>
                    <dd>{edge.orientation}</dd>
                    <dt>Distance</dt>
                    <dd>
                      {edge.distance_meters.toLocaleString(undefined, {
                        maximumFractionDigits: 4,
                      })}{" "}
                      meters
                    </dd>
                    <dt>Base travel time</dt>
                    <dd>
                      {edge.base_seconds.toLocaleString(undefined, {
                        maximumFractionDigits: 4,
                      })}{" "}
                      seconds
                    </dd>
                    <dt>Adopted traffic multiplier</dt>
                    <dd>
                      {edge.traffic_multiplier === null
                        ? "Unavailable · no adopted snapshot"
                        : `${edge.traffic_multiplier}×`}
                    </dd>
                    <dt>Current captured objective cost</dt>
                    <dd>
                      {edge.objective_cost === null
                        ? edge.cost_status === "forbidden"
                          ? "Forbidden under default access"
                          : "Unavailable · adopt traffic on the engine"
                        : `${edge.objective_cost.toLocaleString(undefined, { maximumFractionDigits: 4 })} ${view.evaluation.objective_unit}`}
                    </dd>
                    <dt>Evaluated elapsed road time</dt>
                    <dd>
                      {edge.predicted_elapsed_seconds === null
                        ? "Unavailable"
                        : `${edge.predicted_elapsed_seconds.toFixed(4)} seconds`}
                    </dd>
                    <dt>Compiled access class</dt>
                    <dd>{edge.access.replaceAll("_", " ")}</dd>
                    <dt>Road class</dt>
                    <dd>Unavailable · not retained by this graph</dd>
                    <dt>Directed canonical vertices</dt>
                    <dd>{edge.points_e7.length} · exact WGS84 E7</dd>
                  </dl>
                  <p className="graph-note">
                    This status is for a standalone traversal. It does not prove
                    a permitted incoming maneuver or a legal route.
                  </p>
                </>
              ) : node ? (
                <>
                  <strong className="graph-node-id">Node {node.node}</strong>
                  <dl>
                    <dt>Latitude E7 / longitude E7</dt>
                    <dd>
                      {node.coordinate_e7.latitude_e7} /{" "}
                      {node.coordinate_e7.longitude_e7}
                    </dd>
                    <dt>Coordinate</dt>
                    <dd>
                      {(node.coordinate_e7.latitude_e7 / 1e7).toFixed(7)},{" "}
                      {(node.coordinate_e7.longitude_e7 / 1e7).toFixed(7)}
                    </dd>
                    <dt>Incoming / outgoing on this edge page</dt>
                    <dd>
                      {view.edges.filter((e) => e.to === node.node).length} /{" "}
                      {view.edges.filter((e) => e.from === node.node).length}
                    </dd>
                  </dl>
                  <p className="graph-note">
                    Page-local counts are not the node’s complete degree.
                  </p>
                </>
              ) : (
                <p className="graph-note">
                  Click a graph node or directed edge to inspect its source
                  evidence. Parallel opposite directions are offset for
                  legibility; canonical geometry is unchanged.
                </p>
              )}
            </section>
            <details className="graph-evidence">
              <summary>Captured graph & evaluation evidence</summary>
              <dl>
                <dt>Graph view identity</dt>
                <dd>{view.graph_view_id}</dd>
                <dt>Graph digest</dt>
                <dd>{view.graph.snapshot_digest}</dd>
                <dt>Profile / jurisdiction</dt>
                <dd>
                  {view.graph.routing_profile} /{" "}
                  {view.graph.jurisdiction_policy}
                </dd>
                <dt>Turn restrictions</dt>
                <dd>
                  {String(view.graph.turn_restrictions_enforced)} · supported
                  subset
                </dd>
                <dt>Evaluator</dt>
                <dd>{view.evaluation.evaluator}</dd>
                <dt>Captured departure / clock</dt>
                <dd>
                  {view.evaluation.departure_seconds} ·{" "}
                  {view.evaluation.time_domain}
                </dd>
                <dt>Adopted overlay digest</dt>
                <dd>
                  {view.traffic_snapshot_digest ?? "Unavailable · none adopted"}
                </dd>
                <dt>Cost evaluation traffic digest</dt>
                <dd>{view.evaluation.traffic_digest ?? "Not applied"}</dd>
                <dt>Routing access context</dt>
                <dd>Private: false · Permit: false</dd>
              </dl>
              <button className="secondary-button" onClick={download}>
                <Download size={14} /> Download graph evidence
              </button>
            </details>
          </>
        )}
        <details className="graph-capabilities">
          <summary>Road class & search diagnostics</summary>
          <p>
            Road class is not retained by the loaded graph. Basemap classes and
            profile default speeds cannot establish it.
          </p>
          <label>
            <input type="checkbox" disabled /> Explored-node overlay unavailable
          </label>
          <p>
            Search node membership and ordered events are not retained. A
            selected route is not a search trace.
          </p>
          {route ? (
            <>
              <span className="section-label">
                LAST ROUTED RESULT · ORIGINAL CONTEXT
              </span>
              <dl>
                <dt>Algorithm</dt>
                <dd>{route.search.algorithm}</dd>
                <dt>Expanded search states</dt>
                <dd>{String(route.search.expanded_states)}</dd>
                <dt>State mode</dt>
                <dd>{route.search.state_mode}</dd>
                <dt>Counter semantics</dt>
                <dd>{route.search.counter_semantics}</dd>
                <dt>Evaluation</dt>
                <dd>{route.evaluation_id}</dd>
                <dt>Traffic context</dt>
                <dd>{route.provenance.traffic_digest ?? "Not applied"}</dd>
              </dl>
              <p>
                These counters do not locate explored nodes. They belong to the
                routed result, not this graph capture.
              </p>
            </>
          ) : (
            <p>
              Run a route in Routing mode to inspect its original search
              summary.
            </p>
          )}
        </details>
      </div>
    </section>
  );
}
