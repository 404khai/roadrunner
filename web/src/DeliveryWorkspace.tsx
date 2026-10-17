import { useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  Play,
  Pause,
  RotateCcw,
  Download,
  TrafficCone,
  ChevronDown,
} from "lucide-react";
import { RiderPortrait } from "./RiderPortrait";
import { riderVisual } from "./riderVisual";
import type { ReactNode } from "react";
import type { Catalog } from "./contracts";
import { decodeScene, frameAt, sampleRider, type Scene } from "./deliveryScene";
import { DispatchAuditCard, type AuditFocus } from "./DispatchAuditCard";
import { DeliveryMap } from "./DeliveryMap";
import { useReducedMotion } from "./useReveal";
import { durationLabel } from "./geometry";
export const DISTRIBUTED_DEMO_DIGEST =
  "a3f19ae1814288d33c2db7bd3d60f402ee047d2cb0b8e60ad24e1d82a4273e95";
export function deliveryScenario(
  digest: string,
  kind: string,
  nodes: { rider: number; pickup: number; customer: number },
) {
  const policy = {
    id: "scene-service/v1",
    version: 1,
    deadline: "SoftObserved",
    max_completion_delay: null,
    pickup_service: 15,
    dropoff_service: 10,
    readiness: "CreatedAtFallbackV1",
  };
  const order = (
    id: number,
    pickup: number,
    dropoff: number,
    created: number,
    ready: number,
  ) => ({
    id,
    pickup_node: pickup,
    dropoff_node: dropoff,
    created_at_seconds: created,
    expected_ready_at_seconds: ready,
    actual_readiness: { kind: "fixed", at_seconds: ready },
    demand: 1,
    admission: policy,
  });
  const scenario = {
    schema_version: 4,
    scenario_id: `delivery-${kind}/v1`,
    graph_snapshot_digest: digest,
    seed: 30,
    start_seconds: 0,
    end_seconds: 900,
    routing_epoch_seconds: 0,
    dispatch: {
      kind: "dynamic",
      work_budget: 256,
      forecast_validity_seconds: 2000,
      algorithm: "Greedy",
      recovery: {
        version: 1,
        reroute_penalty: 10,
        assignment_stability_penalty: 10,
        minimum_improvement: 1,
        cooldown: 5,
      },
    },
    riders: [
      {
        id: 1,
        node: nodes.rider,
        capacity: kind === "fleet" ? 1 : 2,
        available: true,
      },
      ...(kind === "fleet"
        ? [{ id: 2, node: nodes.customer, capacity: 1, available: true }]
        : []),
    ],
    orders: [
      order(1, nodes.pickup, nodes.customer, 0, 120),
      ...(kind === "solo"
        ? []
        : [
            order(
              2,
              kind === "fleet" ? nodes.customer : nodes.rider,
              kind === "fleet" ? nodes.rider : nodes.pickup,
              kind === "delayed" ? 5 : 0,
              kind === "fleet" ? 0 : 90,
            ),
          ]),
    ],
    initial_traffic: [],
    traffic_changes: [],
    dynamic_events:
      kind === "delayed"
        ? [
            {
              at_seconds: 2,
              change: { kind: "road_delay", rider: 1, additional_seconds: 80 },
            },
          ]
        : [],
  };
  if (kind === "distributed") {
    if (digest !== DISTRIBUTED_DEMO_DIGEST)
      throw Error(
        "Distributed Fleet Demo requires the pinned Phase 10 graph. Choose another preset or explicit scenario JSON for this graph.",
      );
    scenario.scenario_id = "distributed-fleet/v1";
    scenario.riders = [1, 22, 23, 49, 24, 5].map((node, i) => ({
      id: i + 1,
      node,
      capacity: 1,
      available: true,
    }));
    scenario.orders = [
      [0, 35, 0, 90],
      [16, 18, 0, 0],
      [26, 23, 0, 150],
      [55, 50, 5, 20],
      [25, 24, 0, 45],
    ].map(([p, d, t, r], i) => order(i + 1, p, d, t, r));
    scenario.dynamic_events = [];
  }
  return scenario;
}
function FloatingCard({
  title,
  subtitle,
  preview,
  className,
  open,
  onToggle,
  children,
}: {
  title: ReactNode;
  subtitle?: ReactNode;
  preview?: ReactNode;
  className: string;
  open: boolean;
  onToggle: () => void;
  children: ReactNode;
}) {
  return (
    <section className={`delivery-card ${className} ${open ? "is-open" : ""}`}>
      <button
        className="delivery-card-heading"
        aria-expanded={open}
        aria-label={`${open ? "Collapse" : "Expand"} ${typeof title === "string" ? title : "card"}`}
        onClick={onToggle}
      >
        <span>{title}</span>
        {!open && preview}
        <small>{subtitle}</small>
        <ChevronDown size={15} />
      </button>
      <div className="delivery-card-body" hidden={!open}>
        {children}
      </div>
    </section>
  );
}
export function DeliveryWorkspace({
  catalog,
  token,
  onExit,
}: {
  catalog?: Catalog;
  token: string;
  onExit: () => void;
}) {
  const [dispatchFocus, setDispatchFocus] = useState<AuditFocus>();
  const [scene, setScene] = useState<Scene>(),
    [at, setAt] = useState(0),
    [playing, setPlaying] = useState(false),
    [speed, setSpeed] = useState(4),
    [selected, setSelected] = useState("1"),
    [selectionRevision, setSelectionRevision] = useState(0),
    [edge, setEdge] = useState<number>(),
    [traffic, setTraffic] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [kind, setKind] = useState(
      catalog?.graph.snapshot_digest === DISTRIBUTED_DEMO_DIGEST
        ? "distributed"
        : "fleet",
    ),
    [advanced, setAdvanced] = useState(false),
    [draft, setDraft] = useState(""),
    [configOpen, setConfigOpen] = useState(true),
    [ridersOpen, setRidersOpen] = useState(true),
    [inspectOpen, setInspectOpen] = useState(
      () => !window.matchMedia("(max-width:600px)").matches,
    ),
    [ordersOpen, setOrdersOpen] = useState(false),
    [trafficOpen, setTrafficOpen] = useState(false);
  const [nodes, setNodes] = useState({
    rider: 0,
    pickup: catalog?.anchors.some((a) => a.node === 35)
      ? 35
      : (catalog?.anchors[1]?.node ?? 0),
    customer: catalog?.anchors.some((a) => a.node === 22)
      ? 22
      : (catalog?.anchors[2]?.node ?? 0),
  });
  const reduced = useReducedMotion(),
    request = useRef<AbortController | null>(null);
  useEffect(() => () => request.current?.abort(), []);
  useEffect(() => {
    if (!playing || !scene) return;
    let handle = 0,
      last = performance.now();
    const tick = (now: number) => {
      const elapsed = Math.min(0.25, (now - last) / 1000);
      last = now;
      setAt((value) => Math.min(scene.end, value + elapsed * speed));
      handle = requestAnimationFrame(tick);
    };
    handle = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(handle);
  }, [playing, scene, speed]);
  const running = playing && !!scene && at < scene.end;
  useEffect(() => {
    if (scene && at >= scene.end) setPlaying(false);
  }, [scene, at]);
  async function run() {
    if (!catalog) return;
    request.current?.abort();
    const controller = new AbortController();
    request.current = controller;
    setBusy(true);
    setError("");
    setPlaying(false);
    try {
      const scenario =
        advanced && draft.trim()
          ? JSON.parse(draft)
          : deliveryScenario(catalog.graph.snapshot_digest, kind, nodes);
      const response = await fetch("/v1/visualization/deliveries", {
        method: "POST",
        signal: controller.signal,
        headers: {
          "Content-Type": "application/json",
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: JSON.stringify(scenario),
      });
      const raw = await response.text();
      if (!response.ok) {
        let msg = `Scene request failed (${response.status})`;
        try {
          msg = JSON.parse(raw).message ?? msg;
        } catch {
          /* non-JSON proxy failure */
        }
        throw Error(msg);
      }
      const next = decodeScene(raw, catalog.graph.snapshot_digest);
      if (controller.signal.aborted) return;
      setDispatchFocus(undefined);
      setScene(next);
      setAt(next.start);
      setSelected(next.frames[0].riders[0]?.id ?? "");
      setEdge(undefined);
      setPlaying(!reduced);
      setConfigOpen(false);
      if (window.matchMedia("(max-width:600px)").matches) setInspectOpen(false);
    } catch (e) {
      if (!controller.signal.aborted)
        setError(e instanceof Error ? e.message : "Scene unavailable");
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  const frame = scene && frameAt(scene, at),
    rider = frame?.riders.find((r) => r.id === selected),
    sample = scene && rider && sampleRider(scene, rider, at),
    a = rider?.execution;
  const road = scene?.roads.edges.find((e) => e.edge === edge),
    factor =
      road && frame && scene?.trafficSupported
        ? (frame.traffic?.find((t) => t.edge_id === road.edge)?.multiplier ?? 1)
        : undefined;
  const step = () => {
    if (!scene) return;
    setPlaying(false);
    setAt(scene.frames.find((f) => f.at > at)?.at ?? scene.end);
  };
  const download = () => {
    if (!scene) return;
    const url = URL.createObjectURL(
      new Blob([scene.raw], { type: "application/json" }),
    );
    const el = document.createElement("a");
    el.href = url;
    el.download = "roadrunner-delivery-scene.json";
    el.click();
    URL.revokeObjectURL(url);
  };
  return (
    <main className="delivery-workspace">
      <DispatchAuditCard scene={scene} onFocus={setDispatchFocus} />
      <DeliveryMap
        dispatchFocus={dispatchFocus}
        reduced={reduced}
        scene={scene}
        at={at}
        selected={selected}
        selectionRevision={selectionRevision}
        onSelect={(id) => {
          setSelected(id);
          setSelectionRevision((v) => v + 1);
          setInspectOpen(true);
        }}
        traffic={traffic}
        onEdge={(id) => {
          setEdge(id);
          setTrafficOpen(true);
        }}
      />
      <button onClick={onExit} className="delivery-back">
        <ArrowLeft size={14} /> Routing workspace
      </button>
      <FloatingCard
        title="Delivery simulation"
        subtitle={scene ? "ENGINE SCENE" : "SETUP"}
        className="scene-setup"
        open={configOpen}
        onToggle={() => setConfigOpen((v) => !v)}
      >
        <div className="delivery-eyebrow">ROADRUNNER / EXECUTION LAB</div>
        <h1>Deliveries in motion.</h1>
        <p className="delivery-subtitle">
          Real engine decisions. A simulated journey.
        </p>
        <div className="delivery-config">
          <label>
            Scenario
            <select value={kind} onChange={(e) => setKind(e.target.value)}>
              <option
                value="distributed"
                disabled={
                  catalog?.graph.snapshot_digest !== DISTRIBUTED_DEMO_DIGEST
                }
              >
                Distributed Fleet Demo · 6 riders
              </option>
              <option value="solo">One rider · pickup & service</option>
              <option value="fleet">
                Two riders · simultaneous deliveries
              </option>
              <option value="delayed">Delay & active plan suffix change</option>
            </select>
          </label>
          {kind === "distributed" ? (
            <p className="distributed-demo-note">
              Six fixed graph-node starts · five pickup/customer pairs across
              the bundled street network. Assignments remain engine-selected.
              Node controls apply to the other presets.
            </p>
          ) : (
            <div className="delivery-node-grid">
              {(["rider", "pickup", "customer"] as const).map((key) => (
                <label key={key}>
                  {key}
                  <select
                    aria-label={`${key} node`}
                    value={nodes[key]}
                    onChange={(e) =>
                      setNodes((n) => ({ ...n, [key]: Number(e.target.value) }))
                    }
                  >
                    {catalog?.anchors.map((n) => (
                      <option key={n.node} value={n.node}>
                        Node {n.node}
                      </option>
                    ))}
                  </select>
                </label>
              ))}
            </div>
          )}
          <button
            className="delivery-run"
            disabled={!catalog || busy}
            onClick={() => void run()}
          >
            <Play size={15} />
            {busy ? "Executing engine scenario…" : "Run delivery simulation"}
          </button>
          {!catalog && (
            <p role="status">
              Connect to Roadrunner from the routing workspace first.
            </p>
          )}
          <button
            className="delivery-text-button"
            onClick={() => {
              setAdvanced((v) => !v);
              if (!draft && catalog)
                setDraft(
                  JSON.stringify(
                    deliveryScenario(
                      catalog.graph.snapshot_digest,
                      kind,
                      nodes,
                    ),
                    null,
                    2,
                  ),
                );
            }}
          >
            {" "}
            {advanced ? "Hide" : "Edit"} explicit scenario JSON
          </button>
          {advanced && (
            <label className="delivery-json">
              Engine scenario
              <textarea
                aria-label="Engine scenario JSON"
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                spellCheck={false}
              />
              <small>
                JSON overrides the presets and nodes above. Max 32 riders / 64
                orders / 2,000 queue entries.
              </small>
            </label>
          )}
        </div>
      </FloatingCard>
      {error && (
        <p role="alert" className="delivery-error scene-error">
          {error}
        </p>
      )}
      {scene && frame && (
        <>
          <FloatingCard
            title="Riders"
            preview={
              <span
                className="collapsed-rider-stack"
                aria-label={`${frame.riders.length} riders in simulation`}
                style={{
                  width:
                    26 +
                    Math.max(0, frame.riders.length - 1) *
                      Math.min(12, 68 / Math.max(1, frame.riders.length - 1)),
                }}
              >
                {frame.riders.map((r, i) => (
                  <span
                    key={r.id}
                    style={{
                      left:
                        i *
                        Math.min(12, 68 / Math.max(1, frame.riders.length - 1)),
                    }}
                  >
                    <RiderPortrait
                      id={r.id}
                      visualId={scene.visualIds.get(r.id)}
                    />
                  </span>
                ))}
              </span>
            }
            subtitle={`${frame.riders.length} IN SCENE`}
            className="scene-riders"
            open={ridersOpen}
            onToggle={() => setRidersOpen((v) => !v)}
          >
            <div className="delivery-rider-roster">
              {frame.riders.map((r) => (
                <button
                  key={r.id}
                  aria-label={`Inspect rider ${r.id}`}
                  aria-pressed={r.id === selected}
                  style={
                    {
                      "--rider-color": riderVisual(
                        scene.visualIds.get(r.id) ?? r.id,
                      ).body,
                    } as React.CSSProperties
                  }
                  onClick={() => {
                    setSelected(r.id);
                    setSelectionRevision((v) => v + 1);
                    setInspectOpen(true);
                  }}
                >
                  <RiderPortrait
                    id={r.id}
                    visualId={scene.visualIds.get(r.id)}
                  />
                  <strong>Rider {r.id}</strong>
                  <small>{r.execution?.stage ?? "Idle"}</small>
                  <span>
                    {r.load}/{r.capacity} onboard
                  </span>
                </button>
              ))}
            </div>
          </FloatingCard>
          {rider && (
            <FloatingCard
              title={`Rider ${rider.id}`}
              preview={
                <span className="collapsed-rider-stack selected">
                  <span>
                    <RiderPortrait
                      id={rider.id}
                      visualId={scene.visualIds.get(rider.id)}
                    />
                  </span>
                </span>
              }
              subtitle={a?.stage ?? "Idle"}
              className="scene-inspector"
              open={inspectOpen}
              onToggle={() => setInspectOpen((v) => !v)}
            >
              <div className="selected-rider-portrait">
                <RiderPortrait
                  id={rider.id}
                  visualId={scene.visualIds.get(rider.id)}
                />
                <div>
                  <strong>Rider {rider.id}</strong>
                  <small>
                    {
                      riderVisual(scene.visualIds.get(rider.id) ?? rider.id)
                        .name
                    }
                  </small>
                  <span>{rider.assigned.length} assigned orders</span>
                </div>
              </div>
              <section
                className="delivery-inspector"
                aria-label="Selected rider execution"
              >
                <h3>
                  Rider {rider.id} <span>{a?.stage ?? "Idle"}</span>
                </h3>
                <p className="position-quality">{sample?.quality}</p>
                <div className="delivery-facts">
                  <span>
                    Onboard / capacity
                    <strong>
                      {rider.load} / {rider.capacity} slots
                    </strong>
                  </span>
                  <span>
                    Assigned orders
                    <strong>
                      {rider.assigned.map((id) => `#${id}`).join(", ") ||
                        "None"}
                    </strong>
                  </span>
                  <span>
                    Next stop
                    <strong>
                      {rider.stops[0]
                        ? `${rider.stops[0].kind} #${rider.stops[0].order}`
                        : "No remaining stops"}
                    </strong>
                  </span>
                  <span>
                    Arrival prediction
                    <strong>
                      {a ? `${a.arrival.toFixed(1)}s logical` : "Unavailable"}
                    </strong>
                  </span>
                  <span>
                    Road ETA remaining
                    <strong>
                      {a?.stage === "Travelling"
                        ? a.arrival <= at
                          ? "Prediction expired"
                          : durationLabel(a.arrival - at)
                        : "Not travelling"}
                    </strong>
                  </span>
                  <span>
                    Service end prediction
                    <strong>
                      {a?.service_end !== null && a?.service_end !== undefined
                        ? `${a.service_end.toFixed(1)}s logical`
                        : "Not servicing"}
                    </strong>
                  </span>
                </div>
                <ol className="delivery-stops-list">
                  {rider.stops.map((s, i) => (
                    <li key={`${i}:${s.kind}:${s.order}`}>
                      <span>{i + 1}</span>
                      {s.kind} order #{s.order}
                      {i === 0 && a && <small>Active action</small>}
                    </li>
                  ))}
                </ol>
                <details>
                  <summary>Action & evidence</summary>
                  <dl>
                    <dt>ActionId</dt>
                    <dd>{a?.action_id ?? "None"}</dd>
                    <dt>Schedule generation</dt>
                    <dd>{a?.generation ?? "None"}</dd>
                    <dt>Originating plan</dt>
                    <dd>{a?.originating_plan ?? "None"}</dd>
                    <dt>Current plan revision</dt>
                    <dd>{rider.plan_revision}</dd>
                    <dt>Scene identity</dt>
                    <dd>{scene.run_identity}</dd>
                    <dt>Operational revision</dt>
                    <dd>{frame.revision}</dd>
                    <dt>Original frozen road prediction</dt>
                    <dd>
                      {a
                        ? `${a.route.distance.toFixed(1)} meters / ${a.route.travel.toFixed(1)} seconds`
                        : "None"}
                    </dd>
                    <dt>Original route traffic</dt>
                    <dd>
                      {a ? JSON.stringify(a.route.routing.traffic) : "None"}
                    </dd>
                    <dt>Accepted effects for action</dt>
                    <dd>
                      {scene.effects
                        .filter(
                          (e) =>
                            e.rider === rider.id &&
                            e.action === a?.action_id &&
                            e.at <= at,
                        )
                        .map((e) => `${e.effect} at ${e.at}s`)
                        .join("; ") || "None"}
                    </dd>
                    <dt>Bearing</dt>
                    <dd>
                      {sample?.bearing === undefined
                        ? "Unavailable; neutral north pose"
                        : `${Math.round(sample.bearing)}° route-derived`}
                    </dd>
                  </dl>
                </details>
              </section>
            </FloatingCard>
          )}
          <FloatingCard
            title="Orders"
            subtitle={`${frame.orders.filter((o) => o.status === "delivered").length} delivered`}
            className="scene-orders"
            open={ordersOpen}
            onToggle={() => setOrdersOpen((v) => !v)}
          >
            <div className="delivery-orders">
              {frame.orders.length === 0 && (
                <p>No orders created at this time.</p>
              )}
              {frame.orders.map((o) => (
                <details key={o.id}>
                  <summary>
                    #{o.id}
                    <span>{o.status.replaceAll("_", " ")}</span>
                  </summary>
                  <p>
                    Rider {o.rider ?? "unassigned"} · demand {o.demand} slots
                  </p>
                  <p>
                    Pickup {o.pickup.latitude}, {o.pickup.longitude}
                    <br />
                    Customer {o.dropoff.latitude}, {o.dropoff.longitude}
                  </p>
                  <p>
                    Readiness{" "}
                    {o.readiness.observed_at !== null
                      ? `accepted at ${o.readiness.observed_at}s`
                      : `predicted at ${o.readiness.expected_at ?? "unknown"}s`}
                  </p>
                </details>
              ))}
            </div>
          </FloatingCard>
          <FloatingCard
            title="Traffic context"
            subtitle={scene.trafficSupported ? "SYNTHETIC" : "UNAVAILABLE"}
            className="scene-traffic"
            open={trafficOpen}
            onToggle={() => setTrafficOpen((v) => !v)}
          >
            <section className="delivery-traffic">
              <div className="delivery-section-title">
                <TrafficCone size={16} />
                <h2>Traffic context</h2>
              </div>
              <label>
                <input
                  type="checkbox"
                  disabled={!scene.trafficSupported}
                  checked={traffic && scene.trafficSupported}
                  onChange={(e) => setTraffic(e.target.checked)}
                />{" "}
                Show supported road factors
              </label>
              <p>
                {!scene.trafficSupported &&
                  "Traffic evidence unavailable or unsupported. "}
                Synthetic engine overlay. Not live observations or vehicle
                density. {scene.roads.edges.length}/{scene.roads.edge_count}{" "}
                directed edges loaded.
              </p>
              <label>
                Inspect directed edge
                <select
                  aria-label="Inspect traffic edge"
                  value={edge ?? ""}
                  onChange={(e) =>
                    setEdge(
                      e.target.value === ""
                        ? undefined
                        : Number(e.target.value),
                    )
                  }
                >
                  <option value="">Select an edge</option>
                  {scene.roads.edges.map((e) => (
                    <option key={e.edge} value={e.edge}>
                      Edge {e.edge}: {e.from} → {e.to}
                    </option>
                  ))}
                </select>
              </label>
              {road && (
                <p>
                  Edge {road.edge} · {road.orientation}
                  <br />
                  Current factor{" "}
                  {factor === undefined ? "unavailable" : `${factor}×`} · base{" "}
                  {road.base_seconds.toFixed(1)}s<br />
                  Current standalone road time{" "}
                  {factor === undefined
                    ? "unavailable"
                    : road.cost_status !== "traversable"
                      ? "forbidden"
                      : (road.base_seconds * factor).toFixed(1)}
                  s<br />
                  Default access: {road.cost_status}. Maneuver/route feasibility
                  is separate.
                </p>
              )}
              <details>
                <summary>Context & limitations</summary>
                <p>Current synthetic traffic: {frame.traffic_digest}</p>
                <p>
                  Original action traffic stays pinned across context updates.
                  The displayed current road factor is not a recomputation of
                  the frozen action. Turn penalties, physical counts, road
                  density and observed speed are unavailable.
                </p>
              </details>
            </section>
          </FloatingCard>
          <button
            className="delivery-evidence-download"
            onClick={download}
            title="Download original scene evidence"
          >
            <Download size={15} />
            <span>Scene evidence</span>
          </button>
        </>
      )}
      {scene && (
        <section
          className="delivery-playback"
          aria-label="Simulation presentation clock"
        >
          <div>
            <button
              aria-label={
                running
                  ? "Pause simulation presentation"
                  : "Play simulation presentation"
              }
              onClick={() => setPlaying((v) => !v)}
              disabled={at >= scene.end}
            >
              {running ? <Pause size={18} /> : <Play size={18} />}
            </button>
            <button
              aria-label="Restart simulation presentation"
              onClick={() => {
                setAt(scene.start);
                setPlaying(false);
              }}
            >
              <RotateCcw size={16} />
            </button>
            <strong>
              {at.toFixed(1)}
              <small> / {scene.end}s logical</small>
            </strong>
            <label className="delivery-jump">
              Time
              <input
                aria-label="Presentation seconds"
                type="number"
                min={scene.start}
                max={scene.end}
                step="0.1"
                value={Number(at.toFixed(1))}
                onChange={(e) => {
                  const value = e.target.valueAsNumber;
                  if (Number.isFinite(value)) {
                    setAt(Math.max(scene.start, Math.min(scene.end, value)));
                    setPlaying(false);
                  }
                }}
              />
            </label>
            <button onClick={step}>Next event</button>
            <select
              aria-label="Presentation speed"
              value={speed}
              onChange={(e) => setSpeed(Number(e.target.value))}
            >
              {[1, 4, 16, 64].map((s) => (
                <option key={s} value={s}>
                  {s}×
                </option>
              ))}
            </select>
          </div>
          <input
            aria-label="Simulation presentation time"
            type="range"
            min={scene.start}
            max={scene.end}
            step="0.1"
            value={at}
            onChange={(e) => {
              setAt(Number(e.target.value));
              setPlaying(false);
            }}
          />
          <small>
            {reduced
              ? "Reduced motion: starts paused. Step or scrub supported captures. "
              : ""}
            Presentation of this finite engine run · no live-world mutations
          </small>
        </section>
      )}
    </main>
  );
}
