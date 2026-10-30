import { useCallback, useEffect, useRef, useState, useMemo } from "react";
import {
  ArrowDownUp,
  Bike,
  ScanLine,
  ArrowRight,
  Check,
  ChevronDown,
  Circle,
  CornerDownRight,
  Download,
  Fingerprint,
  KeyRound,
  LoaderCircle,
  LocateFixed,
  MapPin,
  Pause,
  Play,
  RotateCcw,
  Route,
  SkipForward,
  Unplug,
  X,
} from "lucide-react";
import {
  apiText,
  ApiError,
  catalogSchema,
  decodeRoute,
  decodeAlternatives,
  type ComparisonResult,
  parseExact,
  type Anchor,
  type Catalog,
  type RouteRequest,
  type RouteResult,
} from "./contracts";
import { distanceLabel, durationLabel } from "./geometry";
import { ComparisonPanel, SegmentPanel, routeName } from "./ComparisonPanel";
import { GraphPanel } from "./GraphPanel";
import { useGraphDebugger } from "./useGraphDebugger";
import { DispatchWorkspace } from "./DispatchWorkspace";
import { DeliveryWorkspace } from "./DeliveryWorkspace";
import { MapView } from "./MapView";
import { H3Panel } from "./H3Panel";
import { h3Overlay } from "./h3Overlay";
import { useReducedMotion, useReveal } from "./useReveal";
function message(error: unknown) {
  if (error instanceof ApiError)
    return { code: error.code, text: error.message };
  return {
    code: "unsupported_evidence",
    text:
      error instanceof Error ? error.message : "The result could not be read.",
  };
}
export function App() {
  const [h3Enabled, setH3Enabled] = useState(
    () => new URLSearchParams(window.location.search).get("h3") === "1",
  );
  const [h3Resolution, setH3Resolution] = useState(9);
  const [deliveryMode, setDeliveryMode] = useState(
    () =>
      new URLSearchParams(window.location.search).get("mode") === "delivery",
  );
  const [dispatchMode, setDispatchMode] = useState(
    () =>
      new URLSearchParams(window.location.search).get("mode") === "dispatch",
  );
  const [catalog, setCatalog] = useState<Catalog>(),
    [origin, setOrigin] = useState<Anchor>(),
    [destination, setDestination] = useState<Anchor>();
  const [token, setToken] = useState(""),
    [draftToken, setDraftToken] = useState(""),
    [settings, setSettings] = useState(false);
  const [algorithm, setAlgorithm] =
      useState<RouteRequest["algorithm"]>("astar"),
    [objective, setObjective] =
      useState<RouteRequest["objective"]>("travel_time");
  const [selection, setSelection] = useState<"origin" | "destination">(
      "origin",
    ),
    [busy, setBusy] = useState(false),
    [connecting, setConnecting] = useState(false),
    [error, setError] = useState<{ code: string; text: string }>(),
    [singleResult, setSingleResult] = useState<RouteResult>();
  const [comparison, setComparison] = useState<ComparisonResult>();
  const [count, setCount] = useState(1);
  const [selected, setSelected] = useState("");
  const [visible, setVisible] = useState<Set<string>>(new Set());
  const [segment, setSegment] = useState<number>();
  const member = comparison?.members.find((m) => m.member_id === selected);
  const result = useMemo(
    () =>
      member && comparison
        ? {
            evidence: member.route,
            raw: comparison.raw,
            request: comparison.request,
          }
        : singleResult,
    [member, comparison, singleResult],
  );
  function setResult(next: RouteResult | undefined) {
    setSingleResult(next);
    setComparison(undefined);
    setSegment(undefined);
  }
  function selectMember(id: string) {
    setSelected(id);
    setVisible((v) => new Set([...v, id]));
    setSegment(undefined);
  }
  function toggleMember(id: string) {
    const next = new Set(visible);
    if (next.has(id)) {
      if (next.size === 1) return;
      next.delete(id);
      if (selected === id) {
        setSelected([...next][0]);
        setSegment(undefined);
      }
    } else next.add(id);
    setVisible(next);
  }
  const [revealId, setRevealId] = useState(0);
  const [collapsed, setCollapsed] = useState(false);
  const resultPanel = useRef<HTMLDivElement>(null);
  const panelBody = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (comparison)
      resultPanel.current?.scrollIntoView?.({
        block: "start",
        behavior: "instant",
      });
  }, [comparison]);
  const request = useRef<AbortController | null>(null),
    connection = useRef<AbortController | null>(null);
  const catalogDigest = useRef<string | undefined>(undefined);
  const [userReduced, setUserReduced] = useState(false);
  const systemReduced = useReducedMotion();
  const largeRoute = (result?.evidence.geometry.points_e7.length ?? 0) > 16000;
  const reduced = systemReduced || userReduced,
    reveal = useReveal(
      result ? `${result.evidence.evaluation_id}:${revealId}` : undefined,
      (result?.evidence.geometry.no_travel ?? false) || largeRoute,
      reduced,
    );
  useEffect(
    () => () => {
      request.current?.abort();
      connection.current?.abort();
    },
    [],
  );
  useEffect(() => {
    if (!settings) return;
    const previous = document.activeElement as HTMLElement | null;
    return () => previous?.focus();
  }, [settings]);
  const connect = useCallback(async (credential: string, offset = 0) => {
    connection.current?.abort();
    request.current?.abort();
    setBusy(false);
    const controller = new AbortController();
    connection.current = controller;
    setConnecting(true);
    setError(undefined);
    setSingleResult(undefined);
    setComparison(undefined);
    setSegment(undefined);
    try {
      const next = catalogSchema.parse(
        parseExact(
          await apiText(
            `/v1/visualization/anchors?offset=${offset}`,
            credential,
            controller.signal,
          ),
        ),
      );
      if (controller.signal.aborted) return;
      setCatalog(next);
      if (
        offset === 0 ||
        catalogDigest.current !== next.graph.snapshot_digest
      ) {
        setOrigin(undefined);
        setDestination(undefined);
      }
      catalogDigest.current = next.graph.snapshot_digest;
      setToken(credential);
      setSettings(false);
    } catch (e) {
      if (!controller.signal.aborted) {
        setError(message(e));
        if (offset === 0) {
          setCatalog(undefined);
          setOrigin(undefined);
          setDestination(undefined);
        }
      }
    } finally {
      if (!controller.signal.aborted) setConnecting(false);
    }
  }, []);
  useEffect(() => {
    void connect("");
    return () => connection.current?.abort();
  }, [connect]); // Initial same-origin proxy handshake; credentials are held only in memory.
  function changeAnchor(which: "origin" | "destination", node: number) {
    const anchor = catalog?.anchors.find((a) => a.node === node);
    if (!anchor) return;
    request.current?.abort();
    setBusy(false);
    setResult(undefined);
    setError(undefined);
    if (which === "origin") {
      setOrigin(anchor);
      setSelection("destination");
    } else {
      setDestination(anchor);
      setSelection("origin");
    }
  }
  function invalidate() {
    request.current?.abort();
    setBusy(false);
    setResult(undefined);
    setError(undefined);
  }
  async function calculate() {
    if (!catalog || !origin || !destination) return;
    request.current?.abort();
    const controller = new AbortController();
    request.current = controller;
    setBusy(true);
    setError(undefined);
    setResult(undefined);
    const query: RouteRequest = {
      origin: { ...origin, graph_digest: catalog.graph.snapshot_digest },
      destination: {
        ...destination,
        graph_digest: catalog.graph.snapshot_digest,
      },
      algorithm: count > 1 ? "dijkstra" : algorithm,
      objective,
      ...(count > 1 ? { alternatives: count } : {}),
    };
    try {
      const raw = await apiText(
        count > 1
          ? "/v1/visualization/routes/alternatives"
          : "/v1/visualization/routes",
        token,
        controller.signal,
        query,
      );
      const next =
        count > 1 ? decodeAlternatives(raw, query) : decodeRoute(raw, query);
      if (controller.signal.aborted) return;
      if ("members" in next) {
        setComparison(next);
        setSelected(next.members[0].member_id);
        setVisible(new Set(next.members.map((m) => m.member_id)));
        setSegment(undefined);
      } else setResult(next);
      if (window.matchMedia("(max-width: 760px)").matches) setCollapsed(true);
      setRevealId((x) => x + 1);
    } catch (e) {
      if (!controller.signal.aborted) setError(message(e));
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  function download() {
    if (!result) return;
    const url = URL.createObjectURL(
      new Blob([result.raw], { type: "application/json" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = `roadrunner-${result.evidence.evaluation_id}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }
  const route = result?.evidence;
  const h3 = useMemo(
    () =>
      h3Enabled
        ? h3Overlay(catalog?.anchors ?? [], route, h3Resolution)
        : undefined,
    [h3Enabled, catalog, route, h3Resolution],
  );
  const debug = useGraphDebugger(catalog?.graph.snapshot_digest, token);
  const noRoute = error?.code === "no_route";
  if (dispatchMode)
    return (
      <DispatchWorkspace
        catalog={catalog}
        token={token}
        onExit={() => setDispatchMode(false)}
      />
    );
  if (deliveryMode)
    return (
      <DeliveryWorkspace
        catalog={catalog}
        token={token}
        onExit={() => setDeliveryMode(false)}
      />
    );
  return (
    <div
      className={`workspace ${collapsed ? "panel-collapsed" : "panel-open"}`}
    >
      <main className="main-layout">
        <aside
          className={`sidebar ${collapsed ? "collapsed" : ""}`}
          aria-label="Spatial workspace"
        >
          <button
            className="panel-toggle"
            aria-expanded={!collapsed}
            aria-controls="spatial-workspace-content"
            onClick={() => setCollapsed(!collapsed)}
          >
            <span className="status-dot" />
            <span>SPATIAL WORKSPACE</span>
            <span className="panel-toggle-meta">
              {debug.enabled
                ? "GRAPH INSPECTION"
                : `${origin ? `A${origin.node}` : "A"} → ${destination ? `B${destination.node}` : "B"}`}
            </span>
            <ChevronDown size={16} />
          </button>
          <div
            id="spatial-workspace-content"
            ref={panelBody}
            className="panel-body"
            tabIndex={collapsed ? -1 : 0}
            hidden={collapsed}
          >
            <div
              className="workspace-modes"
              role="group"
              aria-label="Workspace mode"
            >
              <button
                aria-pressed={!debug.enabled}
                onClick={() => {
                  debug.setEnabled(false);
                  panelBody.current?.scrollTo?.({ top: 0 });
                }}
              >
                Routing
              </button>
              <button
                aria-pressed={debug.enabled}
                onClick={() => {
                  debug.setEnabled(true);
                  panelBody.current?.scrollTo?.({ top: 0 });
                }}
              >
                Developer graph
              </button>
            </div>
            <div
              className="workspace-launchers"
              role="group"
              aria-label="Engine workspaces"
            >
              <button
                className="delivery-entry"
                onClick={() => setDeliveryMode(true)}
              >
                <Bike size={19} aria-hidden="true" /> Delivery simulation
              </button>
              <button
                className="delivery-entry"
                onClick={() => setDispatchMode(true)}
              >
                <ScanLine size={19} aria-hidden="true" /> Dispatch visualization
              </button>
              <button
                className="delivery-entry"
                aria-pressed={h3Enabled}
                onClick={() => setH3Enabled(!h3Enabled)}
              >
                H3 inspection
              </button>
            </div>
            {debug.enabled && <GraphPanel debug={debug} route={route} />}
            <div hidden={debug.enabled} className="panel-intro">
              <h1>Route planner</h1>
              <p>Inspect the path your engine chooses.</p>
            </div>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void calculate();
              }}
              className="route-form"
              hidden={debug.enabled}
            >
              <div className="section-label">
                ROUTE ENDPOINTS <span>GRAPH NODES</span>
              </div>
              <div className="anchor-fields">
                {(["origin", "destination"] as const).map((which, i) => {
                  const a = which === "origin" ? origin : destination;
                  return (
                    <div
                      className={`anchor-field ${selection === which ? "active" : ""}`}
                      key={which}
                    >
                      <span className="anchor-icon">
                        {i === 0 ? <Circle size={16} /> : <MapPin size={18} />}
                      </span>
                      <div>
                        <label htmlFor={which}>
                          {i === 0 ? "Origin" : "Destination"}
                        </label>
                        <select
                          id={which}
                          value={a?.node ?? ""}
                          disabled={!catalog || connecting}
                          onFocus={() => setSelection(which)}
                          onChange={(e) =>
                            changeAnchor(which, Number(e.target.value))
                          }
                        >
                          <option value="" disabled>
                            Select a graph node
                          </option>
                          {a &&
                            !catalog?.anchors.some(
                              (n) => n.node === a.node,
                            ) && <option value={a.node}>Node {a.node}</option>}
                          {catalog?.anchors.map((n) => (
                            <option key={n.node} value={n.node}>
                              Node {n.node} · {n.coordinate.latitude.toFixed(5)}
                              , {n.coordinate.longitude.toFixed(5)}
                            </option>
                          ))}
                        </select>
                        <small>
                          {a
                            ? `${a.coordinate.latitude.toFixed(7)}, ${a.coordinate.longitude.toFixed(7)}`
                            : "Choose on the map or from the list"}
                        </small>
                      </div>
                      <button
                        type="button"
                        className="icon-button"
                        aria-label={`Choose ${which} on map`}
                        onClick={() => setSelection(which)}
                      >
                        <LocateFixed size={16} />
                      </button>
                    </div>
                  );
                })}
                <button
                  type="button"
                  className="swap-button"
                  disabled={!origin || !destination}
                  aria-label="Swap origin and destination"
                  onClick={() => {
                    invalidate();
                    setOrigin(destination);
                    setDestination(origin);
                  }}
                >
                  <ArrowDownUp size={16} />
                </button>
              </div>
              {catalog && catalog.node_count > 500 && (
                <div className="catalog-pages">
                  <span>
                    Nodes {catalog.offset + 1}–
                    {catalog.offset + catalog.anchors.length} of{" "}
                    {catalog.node_count}
                  </span>
                  <button
                    type="button"
                    disabled={catalog.offset === 0 || connecting}
                    onClick={() =>
                      void connect(token, Math.max(0, catalog.offset - 500))
                    }
                  >
                    Previous
                  </button>
                  <button
                    type="button"
                    disabled={catalog.next_offset === null || connecting}
                    onClick={() => void connect(token, catalog.next_offset!)}
                  >
                    Next
                  </button>
                </div>
              )}
              <label className="comparison-mode">
                Route results
                <select
                  aria-label="Route results"
                  value={count}
                  onChange={(e) => {
                    invalidate();
                    setCount(Number(e.target.value));
                  }}
                >
                  <option value={1}>Single route</option>
                  <option value={2}>Compare up to 2 routes</option>
                  <option value={3}>Compare up to 3 routes</option>
                  <option value={5}>Compare up to 5 routes</option>
                </select>
              </label>
              <div className="options-row">
                <label>
                  Algorithm
                  <select
                    aria-label="Algorithm"
                    disabled={count > 1}
                    value={count > 1 ? "dijkstra" : algorithm}
                    onChange={(e) => {
                      invalidate();
                      setAlgorithm(e.target.value as RouteRequest["algorithm"]);
                    }}
                  >
                    <option value="astar">A* search</option>
                    <option value="dijkstra">
                      {count > 1 ? "Yen · Dijkstra" : "Dijkstra"}
                    </option>
                  </select>
                </label>
                <label>
                  Optimize for
                  <select
                    aria-label="Optimize for"
                    value={objective}
                    onChange={(e) => {
                      invalidate();
                      setObjective(e.target.value as RouteRequest["objective"]);
                    }}
                  >
                    <option value="travel_time">Travel time</option>
                    <option value="distance">Distance</option>
                    <option value="traffic_aware">Traffic-aware time</option>
                  </select>
                </label>
              </div>
              <button
                className="primary-button"
                type="submit"
                disabled={!origin || !destination || busy || connecting}
              >
                {busy ? (
                  <LoaderCircle className="spinner" size={18} />
                ) : (
                  <Route size={18} />
                )}
                <span>{busy ? "Computing route…" : "Find route"}</span>
                {!busy && <ArrowRight size={18} />}
              </button>
              <p className="form-note">
                Explicit graph anchors. No automatic snapping.
              </p>
            </form>
            {h3 && !debug.enabled && (
              <H3Panel
                cells={h3.cells}
                resolution={h3Resolution}
                onResolution={setH3Resolution}
                result={count === 1 ? result : undefined}
                token={token}
                onRoute={setResult}
                truncated={h3.truncated}
                unsupported={h3.unsupported}
              />
            )}
            <div
              ref={resultPanel}
              className="result-panel"
              hidden={debug.enabled}
              aria-live="polite"
              aria-atomic="true"
            >
              {connecting ? (
                <div className="empty-result">
                  <LoaderCircle size={23} className="spinner" />
                  <h2>Connecting to the engine</h2>
                  <p>Loading available graph anchors.</p>
                </div>
              ) : busy ? (
                <div className="empty-result">
                  <Route size={25} />
                  <h2>Finding your route</h2>
                  <p>
                    Roadrunner is evaluating the selected graph. Search progress
                    is unavailable.
                  </p>
                </div>
              ) : error ? (
                <div className="error-result" role="alert">
                  <Unplug size={24} />
                  <h2>
                    {noRoute
                      ? "No route found"
                      : catalog
                        ? "Route unavailable"
                        : "Engine unavailable"}
                  </h2>
                  <p>{error.text}</p>
                  <code>{error.code}</code>
                  {!catalog && (
                    <button
                      onClick={() => setSettings(true)}
                      className="secondary-button"
                    >
                      Connection settings <ArrowRight size={15} />
                    </button>
                  )}
                </div>
              ) : route ? (
                <>
                  {comparison && (
                    <ComparisonPanel
                      comparison={comparison}
                      selected={selected}
                      visible={visible}
                      onSelect={selectMember}
                      onToggle={toggleMember}
                    />
                  )}
                  <div className="result-heading">
                    <span className="section-label">
                      SELECTED ROUTE
                      {member
                        ? ` · ${routeName(member.rank).toUpperCase()}`
                        : ""}
                    </span>
                    <span className="result-badge">
                      <Check size={12} />
                      {route.geometry.no_travel ? "NO TRAVEL" : "COMPUTED"}
                    </span>
                  </div>
                  <div className="main-metrics">
                    <div>
                      <span className="metric-value">
                        {durationLabel(route.predicted_elapsed_seconds)}
                      </span>
                      <span className="metric-label">Predicted road time</span>
                    </div>
                    <div>
                      <span className="metric-value">
                        {distanceLabel(route.distance_meters)}
                      </span>
                      <span className="metric-label">Road distance</span>
                    </div>
                  </div>
                  <div className="route-facts">
                    <span>
                      <CornerDownRight size={16} />
                      {route.geometry.no_travel
                        ? "Origin equals destination"
                        : `${route.geometry.occurrences.length} directed traversals`}
                    </span>
                    <span>
                      {route.search.algorithm === "yen"
                        ? "Yen · Dijkstra"
                        : route.search.algorithm === "a_star"
                          ? "A*"
                          : "Dijkstra"}
                    </span>
                  </div>
                  <div className="prediction-note">
                    Deterministic prediction
                    {route.provenance.traffic_digest
                      ? " · pinned traffic context"
                      : " · free-flow context"}
                    . Road time excludes pickup, waiting and service.
                  </div>
                  {member && (
                    <SegmentPanel
                      key={member.member_id}
                      member={member}
                      ordinal={segment}
                      onChange={setSegment}
                    />
                  )}
                  <details className="evidence-details">
                    <summary>
                      <Fingerprint size={16} />
                      Evaluation evidence
                      <ChevronDown size={15} />
                    </summary>
                    <dl>
                      <dt>Objective</dt>
                      <dd>
                        {route.objective.value.toLocaleString(undefined, {
                          maximumFractionDigits: 4,
                        })}{" "}
                        {route.objective_unit} · {route.objective.kind}
                      </dd>
                      <dt>Profile / jurisdiction</dt>
                      <dd>
                        {route.routing_profile} / {route.jurisdiction_policy}
                      </dd>
                      <dt>Graph digest</dt>
                      <dd>{route.geometry.graph_digest}</dd>
                      <dt>Spatial path</dt>
                      <dd>{route.geometry.path_id}</dd>
                      <dt>Evaluation</dt>
                      <dd>{route.evaluation_id}</dd>
                      <dt>Evaluator</dt>
                      <dd>{route.provenance.evaluator}</dd>
                      <dt>Departure</dt>
                      <dd>
                        {route.provenance.departure_seconds} ·{" "}
                        {route.provenance.time_domain}
                      </dd>
                      <dt>Traffic digest</dt>
                      <dd>
                        {route.provenance.traffic_digest ?? "Not applicable"}
                      </dd>
                      <dt>Heuristic</dt>
                      <dd>{route.provenance.heuristic ?? "Not applicable"}</dd>
                      <dt>Access context</dt>
                      <dd>
                        Private:{" "}
                        {String(
                          route.provenance.routing_context.private_access,
                        )}{" "}
                        · Permit:{" "}
                        {String(route.provenance.routing_context.permit_access)}
                      </dd>
                      <dt>Cost breakdown</dt>
                      <dd>Unavailable — components not retained</dd>
                      <dt>Guidance / physical telemetry</dt>
                      <dd>Unavailable</dd>
                      <dt>Evidence semantics</dt>
                      <dd>v1 · WGS84 E7 · deterministic_prediction</dd>
                    </dl>
                    <button className="secondary-button" onClick={download}>
                      <Download size={14} />
                      Download source evidence
                    </button>
                  </details>
                </>
              ) : (
                <div className="empty-result">
                  <span className="empty-icon">
                    <Route size={25} />
                  </span>
                  <h2>A path worth inspecting.</h2>
                  <p>
                    Select two graph nodes to see Roadrunner’s directed road
                    geometry and routing evidence.
                  </p>
                </div>
              )}
            </div>
            <footer className="sidebar-footer">
              <span className="status-dot" />
              <span>
                {catalog
                  ? `${catalog.node_count.toLocaleString()} nodes · ${catalog.edge_count.toLocaleString()} directed edges`
                  : "Waiting for a graph"}
              </span>
              <span className="footer-version">v0.1</span>
            </footer>
          </div>
        </aside>
        <div className="map-workspace">
          <MapView
            h3Cells={h3?.geojson}
            onConnect={() => setSettings(true)}
            connected={!!catalog}
            panelOpen={!collapsed}
            catalog={catalog}
            debug={debug}
            onInspectGraph={(selection) => {
              debug.setSelection(selection);
              setCollapsed(false);
            }}
            origin={result?.request.origin ?? origin}
            destination={result?.request.destination ?? destination}
            result={result}
            alternatives={comparison?.members}
            visible={visible}
            selectedMember={selected}
            onSelectMember={selectMember}
            segment={segment}
            onSelectSegment={setSegment}
            progress={reveal.progress}
            reduced={reduced}
            selection={selection}
            onSelect={(node) => changeAnchor(selection, node)}
          />
          {route && collapsed && !debug.enabled && (
            <div className="floating-metrics">
              <span>
                {durationLabel(route.predicted_elapsed_seconds)}
                <small>Predicted road time</small>
              </span>
              <span>
                {distanceLabel(route.distance_meters)}
                <small>Road distance</small>
              </span>
              {comparison && (
                <div
                  className="floating-route-switch"
                  aria-label="Select comparison route"
                >
                  {comparison.members.map((m) => (
                    <button
                      key={m.member_id}
                      aria-label={`View ${routeName(m.rank)}`}
                      aria-pressed={selected === m.member_id}
                      onClick={() => selectMember(m.member_id)}
                    >
                      {String.fromCharCode(64 + m.rank)}
                    </button>
                  ))}
                </div>
              )}
              <span className="floating-algorithm">
                {route.search.algorithm === "yen"
                  ? "Yen · Dijkstra"
                  : route.search.algorithm === "a_star"
                    ? "A*"
                    : "Dijkstra"}
              </span>
            </div>
          )}
          {route && !debug.enabled && (
            <div className="reveal-bar" data-testid="reveal-controls">
              <div className="reveal-status">
                <span className="reveal-symbol">
                  <Route size={18} />
                </span>
                <div>
                  <strong>
                    {route.geometry.no_travel
                      ? "Already at destination"
                      : reveal.progress < 1
                        ? "Revealing computed route"
                        : "Route in view"}
                  </strong>
                  <small>
                    {largeRoute
                      ? "Large route · static presentation"
                      : reduced
                        ? "Reduced motion · static result"
                        : "Presentation only · no journey or search progress"}
                  </small>
                </div>
              </div>
              <div className="reveal-actions">
                <button
                  aria-label="Reduce motion"
                  aria-pressed={reduced}
                  title="Reduce motion"
                  disabled={systemReduced}
                  onClick={() => setUserReduced(!userReduced)}
                >
                  <Circle size={14} />
                </button>
                {!reduced && !largeRoute && !route.geometry.no_travel && (
                  <>
                    <label className="speed-label">
                      <span className="sr-only">Reveal speed</span>
                      <select
                        aria-label="Reveal speed"
                        value={reveal.speed}
                        onChange={(e) =>
                          reveal.setSpeed(Number(e.target.value))
                        }
                      >
                        <option value={0.5}>0.5×</option>
                        <option value={1}>1×</option>
                        <option value={2}>2×</option>
                      </select>
                    </label>
                    {reveal.progress < 1 && (
                      <>
                        <button
                          aria-label={
                            reveal.playing ? "Pause reveal" : "Resume reveal"
                          }
                          onClick={
                            reveal.playing ? reveal.pause : reveal.resume
                          }
                        >
                          {reveal.playing ? (
                            <Pause size={17} />
                          ) : (
                            <Play size={17} />
                          )}
                        </button>
                        <button aria-label="Skip reveal" onClick={reveal.skip}>
                          <SkipForward size={17} />
                        </button>
                      </>
                    )}
                    <button className="replay-button" onClick={reveal.replay}>
                      <RotateCcw size={16} />
                      <span>Replay reveal</span>
                    </button>
                  </>
                )}
              </div>
              <div className="reveal-track">
                <span style={{ width: `${reveal.progress * 100}%` }} />
              </div>
            </div>
          )}
        </div>
      </main>
      {settings && (
        <div className="dialog-backdrop" onClick={() => setSettings(false)}>
          <section
            className="connection-dialog"
            role="dialog"
            aria-modal="true"
            aria-labelledby="connection-title"
            onClick={(e) => e.stopPropagation()}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setSettings(false);
                return;
              }
              if (e.key !== "Tab") return;
              const items = [
                ...e.currentTarget.querySelectorAll<HTMLElement>(
                  "button:not(:disabled),input",
                ),
              ];
              const first = items[0],
                last = items.at(-1);
              if (e.shiftKey && document.activeElement === first) {
                e.preventDefault();
                last?.focus();
              } else if (!e.shiftKey && document.activeElement === last) {
                e.preventDefault();
                first?.focus();
              }
            }}
          >
            <button
              className="dialog-close icon-button"
              aria-label="Close connection settings"
              onClick={() => setSettings(false)}
            >
              <X size={20} />
            </button>
            <KeyRound size={25} />
            <h2 id="connection-title">Connect your engine</h2>
            <p>
              The local development proxy connects to Roadrunner. Enter a bearer
              token if the proxy has no configured credential.
            </p>
            <form
              onSubmit={(e) => {
                e.preventDefault();
                void connect(draftToken);
              }}
            >
              <label htmlFor="token">Bearer token</label>
              <input
                id="token"
                type="password"
                autoComplete="off"
                value={draftToken}
                onChange={(e) => setDraftToken(e.target.value)}
                placeholder="Held in memory for this session"
                autoFocus
              />
              <button className="primary-button" disabled={connecting}>
                {connecting ? "Connecting…" : "Connect"}
                <ArrowRight size={17} />
              </button>
            </form>
            <small>
              API endpoint: same-origin /v1 · Credential is never stored in
              browser storage.
            </small>
          </section>
        </div>
      )}
    </div>
  );
}
