import { useEffect, useRef, useState } from "react";
import type { Catalog } from "./contracts";
import { apiText } from "./contracts";
import {
  decodeDispatch,
  type DispatchEvidence,
  type DispatchRequest,
} from "./dispatchEvidence";
import { DispatchMap } from "./DispatchMap";
import { RiderPortrait } from "./RiderPortrait";
import { useReducedMotion, useReveal } from "./useReveal";
import { durationLabel } from "./geometry";
export function DispatchWorkspace({
  catalog,
  token,
  onExit,
}: {
  catalog?: Catalog;
  token: string;
  onExit: () => void;
}) {
  const [evidence, setEvidence] = useState<DispatchEvidence>(),
    [inspected, setInspected] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [mode, setMode] = useState("spatial"),
    [radius, setRadius] = useState(1000),
    [limit, setLimit] = useState(6),
    [ready, setReady] = useState(60),
    [aware, setAware] = useState(true),
    [publish, setPublish] = useState(false),
    [pickup, setPickup] = useState<number>(),
    [customer, setCustomer] = useState<number>(),
    [riders, setRiders] = useState<DispatchRequest["riders"]>([]);
  const [setupOpen, setSetupOpen] = useState(true),
    [resultsOpen, setResultsOpen] = useState(() => window.innerWidth > 780);
  const abort = useRef<AbortController | null>(null),
    generation = useRef(0),
    reduced = useReducedMotion();
  const sequence = useReveal(evidence?.evaluation_id, false, reduced),
    inspection = useReveal(
      evidence ? `${evidence.evaluation_id}:${inspected}` : undefined,
      false,
      reduced,
    );
  useEffect(() => {
    abort.current?.abort();
    generation.current++;
    setEvidence(undefined);
    setInspected("");
    setBusy(false);
    setPickup(catalog?.anchors[4]?.node ?? catalog?.anchors[0]?.node);
    setCustomer(catalog?.anchors[7]?.node ?? catalog?.anchors.at(-1)?.node);
    setRiders(
      catalog?.anchors
        .filter((a, i) =>
          catalog.graph.snapshot_digest ===
          "a3f19ae1814288d33c2db7bd3d60f402ee047d2cb0b8e60ad24e1d82a4273e95"
            ? [0, 6, 11, 20, 24, 34].includes(a.node)
            : i < 6,
        )
        .map((a, i) => ({ node: a.node, capacity: 2, available: i !== 5 })) ??
        [],
    );
    return () => {
      abort.current?.abort();
    };
  }, [catalog]);
  async function run() {
    if (!catalog || pickup === undefined || customer === undefined) return;
    abort.current?.abort();
    const controller = new AbortController();
    abort.current = controller;
    const current = ++generation.current;
    setBusy(true);
    setError("");
    setEvidence(undefined);
    const request: DispatchRequest = {
      graph_digest: catalog.graph.snapshot_digest,
      pickup_node: pickup,
      customer_node: customer,
      riders,
      radius_meters: mode === "exhaustive" ? null : radius,
      limit: mode === "empty" ? 0 : limit,
      ready_in_seconds: ready,
      preparation_aware: aware,
      publish_demo: publish,
    };
    try {
      const raw = await apiText(
          "/v1/visualization/dispatch",
          token,
          controller.signal,
          request,
        ),
        result = decodeDispatch(raw, request);
      if (current !== generation.current) return;
      setEvidence(result);
      setSetupOpen(window.innerWidth > 780);
      setResultsOpen(true);
      setInspected(result.selected ?? result.candidates[0]?.rider ?? "");
    } catch (e) {
      if (!controller.signal.aborted)
        setError(
          e instanceof Error ? e.message : "Dispatch evidence unavailable",
        );
    } finally {
      if (current === generation.current) setBusy(false);
    }
  }
  const inspectedRider = evidence?.riders.find((r) => r.id === inspected);
  const candidate = evidence?.candidates.find((c) => c.rider === inspected),
    stage =
      sequence.progress < 0.55
        ? "Seeking a rider"
        : sequence.progress < 1
          ? "Evaluating candidates"
          : evidence?.selected
            ? "Candidate picked"
            : "No candidate selected";
  function download() {
    if (!evidence) return;
    const url = URL.createObjectURL(
      new Blob([evidence.raw], { type: "application/json" }),
    );
    const a = document.createElement("a");
    a.href = url;
    a.download = "roadrunner-dispatch-evaluation.json";
    a.click();
    URL.revokeObjectURL(url);
  }
  return (
    <main className="delivery-workspace dispatch-workspace">
      <DispatchMap
        evidence={evidence}
        inspected={inspected}
        sequence={sequence.progress}
        playing={sequence.playing}
        routeProgress={
          sequence.progress < 0.65
            ? 0
            : sequence.progress < 1
              ? (sequence.progress - 0.65) / 0.35
              : inspection.progress
        }
        reduced={reduced}
        onInspect={setInspected}
      />
      <button className="delivery-exit" onClick={onExit}>
        ← Routing workspace
      </button>
      <details
        className="dispatch-setup floating-dispatch-card"
        open={setupOpen}
        onToggle={(e) => {
          setSetupOpen(e.currentTarget.open);
          if (e.currentTarget.open && window.innerWidth <= 780)
            setResultsOpen(false);
        }}
      >
        <summary>Dispatch evaluation · isolated demo</summary>
        <div className="dispatch-card-body">
          <p>
            Actual Roadrunner decisions on explicit graph nodes. No live-world
            mutations or GPS feed.
          </p>
          <label>
            Pickup / restaurant
            <select
              aria-label="Dispatch pickup node"
              value={pickup ?? ""}
              onChange={(e) => setPickup(Number(e.target.value))}
            >
              {catalog?.anchors.map((a) => (
                <option key={a.node} value={a.node}>
                  Node {a.node}
                </option>
              ))}
            </select>
          </label>
          <label>
            Customer
            <select
              aria-label="Dispatch customer node"
              value={customer ?? ""}
              onChange={(e) => setCustomer(Number(e.target.value))}
            >
              {catalog?.anchors.map((a) => (
                <option key={a.node} value={a.node}>
                  Node {a.node}
                </option>
              ))}
            </select>
          </label>
          <label>
            Candidate policy
            <select value={mode} onChange={(e) => setMode(e.target.value)}>
              <option value="spatial">Spatial radius + limit</option>
              <option value="exhaustive">Exhaustive eligible fleet</option>
              <option value="empty">Empty spatial shortlist</option>
            </select>
          </label>
          {mode !== "exhaustive" && (
            <>
              <label>
                Radius (metres)
                <input
                  type="number"
                  min={0}
                  max={5000}
                  value={radius}
                  onChange={(e) => setRadius(Number(e.target.value))}
                />
              </label>
              <label>
                Candidate limit
                <input
                  type="number"
                  min={0}
                  max={16}
                  value={limit}
                  onChange={(e) => setLimit(Number(e.target.value))}
                />
              </label>
            </>
          )}
          <label>
            Ready in seconds
            <input
              type="number"
              min={0}
              max={3600}
              value={ready}
              onChange={(e) => setReady(Number(e.target.value))}
            />
          </label>
          <label>
            <input
              type="checkbox"
              checked={aware}
              onChange={(e) => setAware(e.target.checked)}
            />{" "}
            Preparation-aware score
          </label>
          <label>
            <input
              type="checkbox"
              checked={publish}
              onChange={(e) => setPublish(e.target.checked)}
            />{" "}
            Publish in isolated demo world
          </label>
          <details>
            <summary>Rider starts, capacity and eligibility</summary>
            {riders.map((r, i) => (
              <fieldset key={i}>
                <legend>Rider {i + 1}</legend>
                <label>
                  Start
                  <select
                    aria-label={`Rider ${i + 1} start node`}
                    value={r.node}
                    onChange={(e) =>
                      setRiders((v) =>
                        v.map((r, j) =>
                          j === i ? { ...r, node: Number(e.target.value) } : r,
                        ),
                      )
                    }
                  >
                    {catalog?.anchors.map((a) => (
                      <option key={a.node} value={a.node}>
                        {a.node}
                      </option>
                    ))}
                  </select>
                </label>
                <label>
                  Capacity
                  <input
                    type="number"
                    min={0}
                    max={8}
                    value={r.capacity}
                    onChange={(e) =>
                      setRiders((v) =>
                        v.map((r, j) =>
                          j === i
                            ? { ...r, capacity: Number(e.target.value) }
                            : r,
                        ),
                      )
                    }
                  />
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={r.available}
                    onChange={(e) =>
                      setRiders((v) =>
                        v.map((r, j) =>
                          j === i ? { ...r, available: e.target.checked } : r,
                        ),
                      )
                    }
                  />{" "}
                  Available
                </label>
              </fieldset>
            ))}
          </details>
          <button disabled={!catalog || busy} onClick={run}>
            {busy ? "Evaluating…" : "Evaluate dispatch"}
          </button>
          {!catalog && <p>Loading graph anchors / connect engine first.</p>}
          {error && <p role="alert">{error}</p>}
        </div>
      </details>
      <details
        className="dispatch-results floating-dispatch-card"
        open={resultsOpen}
        onToggle={(e) => {
          setResultsOpen(e.currentTarget.open);
          if (e.currentTarget.open && window.innerWidth <= 780)
            setSetupOpen(false);
        }}
      >
        <summary>
          Candidate evidence · {evidence?.candidates.length ?? 0} evaluated
        </summary>
        <div className="dispatch-card-body">
          {!evidence ? (
            <p>
              {busy
                ? "Computation pending. No search progress is inferred."
                : "Run an evaluation to inspect candidates and supported routes."}
            </p>
          ) : (
            <>
              <p>
                <strong>
                  {evidence.selected
                    ? `Recommended rider ${evidence.selected}`
                    : "Unassigned · no feasible selected rider"}
                </strong>
                <br />
                {evidence.committed
                  ? "Committed in isolated demo world"
                  : "Recommendation only · no publication"}
                <br />
                No active execution
              </p>
              <p>
                {evidence.decision.evidence.coverage} coverage ·{" "}
                {evidence.candidates.length}/{evidence.riders.length} riders
                evaluated. Canonical rider order, not discovery chronology.
              </p>
              <table>
                <thead>
                  <tr>
                    <th>Rider</th>
                    <th>Pickup</th>
                    <th>Delivery</th>
                    <th>Wait</th>
                    <th>Score s</th>
                  </tr>
                </thead>
                <tbody>
                  {evidence.candidates.map((c) => (
                    <tr key={c.rider} data-selected={c.rider === inspected}>
                      <td>
                        <button
                          aria-label={`Inspect candidate ${c.rider}`}
                          aria-pressed={c.rider === inspected}
                          onClick={() => setInspected(c.rider)}
                        >
                          <RiderPortrait id={c.rider} />
                          {c.rider}
                          {evidence.selected === c.rider ? " ★" : ""}
                        </button>
                      </td>
                      {c.feasible ? (
                        <>
                          <td>{durationLabel(c.pickup_seconds!)}</td>
                          <td>{durationLabel(c.delivery_seconds!)}</td>
                          <td>{durationLabel(c.waiting_seconds!)}</td>
                          <td>{c.score_seconds?.toFixed(2)}</td>
                        </>
                      ) : (
                        <td colSpan={4}>{c.rejection}</td>
                      )}
                    </tr>
                  ))}
                </tbody>
              </table>
              {!evidence.candidates.length && (
                <p>Empty shortlist. No rider was discovered or selected.</p>
              )}
              <p>
                {candidate
                  ? `Inspecting rider ${candidate.rider} · ${candidate.rider === evidence.selected ? "recommended" : "hypothetical alternative"}`
                  : "Select an evaluated candidate"}
              </p>
              {!candidate && inspectedRider && (
                <p>
                  No candidate evaluation retained for rider {inspectedRider.id}
                  . No score, rank or hypothetical route is inferred.
                </p>
              )}
              {candidate?.feasible && (
                <p>
                  {candidate.legs.every((l) => l.geometry)
                    ? "Directed routes captured during this evaluation. Estimated travel, not GPS or executed movement."
                    : "Hypothetical geometry unavailable; no replacement line is invented."}
                </p>
              )}
              <p>{evidence.explanation}</p>
              <details>
                <summary>Original decision, source and policy</summary>
                <pre>{JSON.stringify(evidence.decision, null, 2)}</pre>
                <p>
                  Evaluation {evidence.evaluation_id}
                  <br />
                  Graph {evidence.graph_digest}
                  <br />
                  Source {evidence.geometry_source}
                </p>
              </details>
              <button onClick={download}>Download retained evidence</button>
            </>
          )}
        </div>
      </details>
      {evidence && (
        <section
          className="dispatch-presentation floating-dispatch-card"
          aria-label="Dispatch presentation"
        >
          <strong>{stage}</strong>
          <p>
            {evidence.request.radius_meters === null
              ? "Decorative search metaphor · exhaustive eligible fleet, no geographic radius"
              : `Declared Haversine filter · pickup center · ${evidence.request.radius_meters} m inclusive · limit ${evidence.request.limit}`}
          </p>
          <small>
            Presentation of retained results · not a historical radar scan or
            computation progress. Display remains pinned to submitted inputs.
          </small>
          <div>
            <button
              onClick={() => {
                sequence.replay();
                inspection.replay();
              }}
            >
              Replay dispatch presentation
            </button>
            <button
              onClick={() => {
                sequence.skip();
                inspection.skip();
              }}
            >
              Skip animation
            </button>
            <button
              onClick={() => {
                if (sequence.playing) sequence.pause();
                else sequence.resume();
                if (inspection.playing) inspection.pause();
                else inspection.resume();
              }}
            >
              {sequence.playing || inspection.playing
                ? "Pause presentation"
                : "Resume presentation"}
            </button>
          </div>
        </section>
      )}
      <div className="dispatch-legend">
        Gold restaurant / pickup · Lavender customer · ★ recommendation · Demo
        publication is separate
      </div>
    </main>
  );
}
