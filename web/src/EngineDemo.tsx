import { useEffect, useRef, useState } from "react";
import * as ml from "maplibre-gl";
import {
  catalogSchema,
  decodeRoute,
  parseExact,
  type RouteRequest,
  type RouteResult,
} from "./contracts";
import { decodeScene, frameAt, type Scene } from "./deliveryScene";
import {
  decodeDispatch,
  type DispatchEvidence,
  type DispatchRequest,
} from "./dispatchEvidence";
import { DeliveryMap } from "./DeliveryMap";
import { DispatchMap } from "./DispatchMap";
import {
  presentationPath,
  revealPrefix,
  distanceLabel,
  durationLabel,
} from "./geometry";
import { style } from "./MapView";
import { useReducedMotion } from "./useReveal";
export type DemoMode = "delivery" | "dispatch" | "routing";
const labels: Record<DemoMode, string> = {
  delivery: "Delivery simulation",
  dispatch: "Dispatch visualization",
  routing: "Route planning",
};
const files = ["catalog", "delivery", "dispatch", "route", "route-request"];
export function EngineDemo() {
  const [mode, setMode] = useState<DemoMode>("delivery"),
    [playing, setPlaying] = useState(true),
    [progress, setProgress] = useState(0),
    [selected, setSelected] = useState(""),
    [error, setError] = useState(""),
    [retry, setRetry] = useState(0);
  const [data, setData] = useState<{
    scene: Scene;
    dispatch: DispatchEvidence;
    route: RouteResult;
  }>();
  const reduced = useReducedMotion();
  useEffect(() => {
    const controller = new AbortController();
    setError("");
    Promise.all(
      files.map(async (name) => {
        const response = await fetch(`/demos/${name}.json`, {
          signal: controller.signal,
        });
        if (!response.ok)
          throw Error(`Demo evidence unavailable (${response.status})`);
        return response.text();
      }),
    )
      .then(([catalogRaw, deliveryRaw, dispatchRaw, routeRaw, requestRaw]) => {
        const catalog = catalogSchema.parse(parseExact(catalogRaw));
        const request = JSON.parse(dispatchRaw).request as DispatchRequest;
        const scene = decodeScene(deliveryRaw, catalog.graph.snapshot_digest);
        const dispatch = decodeDispatch(dispatchRaw, request);
        const route = decodeRoute(
          routeRaw,
          JSON.parse(requestRaw) as RouteRequest,
        );
        if (
          route.evidence.geometry.graph_digest !==
            catalog.graph.snapshot_digest ||
          dispatch.graph_digest !== catalog.graph.snapshot_digest
        )
          throw Error("Demo graph identity mismatch");
        if (controller.signal.aborted) return;
        setSelected(scene.frames[0].riders[0]?.id ?? "");
        setData({ scene, dispatch, route });
      })
      .catch((e) => {
        if (!controller.signal.aborted)
          setError(
            e instanceof Error ? e.message : "Unsupported demo evidence",
          );
      });
    return () => controller.abort();
  }, [retry]);
  const complete = progress === 1;
  useEffect(() => {
    if (!data || !playing || reduced || complete) return;
    let previous = performance.now();
    const timer = window.setInterval(() => {
      const now = performance.now();
      // Delivery playback uses logical seconds at 1×; other tabs reveal over 30s.
      const duration =
        mode === "delivery"
          ? Math.max(
              0.001,
              Math.min(data.scene.end, data.scene.frames.at(-1)!.at) -
                data.scene.start,
            ) * 1000
          : 30000;
      const delta = Math.min(now - previous, 100) / duration;
      previous = now;
      if (!document.hidden) setProgress((p) => Math.min(1, p + delta));
    }, 50);
    return () => clearInterval(timer);
  }, [data, playing, reduced, complete, mode]);
  const at = data
    ? data.scene.start +
      progress *
        (Math.min(data.scene.end, data.scene.frames.at(-1)!.at) -
          data.scene.start)
    : 0;
  return (
    <section
      className="engine-demo"
      id="demo"
      aria-label="Interactive engine demos"
    >
      <div className="demo-tabs" role="tablist" aria-label="Engine demos">
        {(Object.keys(labels) as DemoMode[]).map((key) => (
          <button
            key={key}
            role="tab"
            aria-selected={mode === key}
            id={`tab-${key}`}
            aria-controls="demo-panel"
            onClick={() => {
              setMode(key);
              setSelected(
                key === "delivery"
                  ? (data?.scene.frames[0].riders[0]?.id ?? "")
                  : "",
              );
              setProgress(0);
              setPlaying(true);
            }}
          >
            {labels[key]}
          </button>
        ))}
      </div>
      <div
        id="demo-panel"
        role="tabpanel"
        aria-labelledby={`tab-${mode}`}
        className="demo-stage"
      >
        {!data && !error && (
          <p className="demo-status" role="status">
            Loading checked engine evidence…
          </p>
        )}
        {error && (
          <div className="demo-status" role="alert">
            <p>{error}</p>
            <button onClick={() => setRetry((r) => r + 1)}>
              Retry evidence
            </button>
          </div>
        )}
        {data &&
          (mode === "delivery" ? (
            <DeliveryMap
              mini
              cooperativeGestures
              showNetwork
              scene={data.scene}
              at={at}
              selected={selected}
              onSelect={setSelected}
              traffic={false}
              onEdge={() => {}}
              reduced={reduced}
            />
          ) : mode === "dispatch" ? (
            <DispatchMap
              cooperativeGestures
              evidence={data.dispatch}
              inspected={selected || data.dispatch.selected || ""}
              sequence={progress}
              routeProgress={progress}
              reduced={reduced}
              onInspect={setSelected}
            />
          ) : (
            <RoutePreview
              result={data.route}
              progress={progress}
              reduced={reduced}
            />
          ))}
        <div className="demo-caption">
          <span className="status-dot" />
          Retained engine run · simulated, not live GPS
        </div>
      </div>
      {data && (
        <div className="demo-evidence" aria-label="Retained engine summary">
          {mode === "delivery" ? (
            (() => {
              const frame = frameAt(data.scene, at);
              const rider =
                frame.riders.find((r) => r.id === selected) ?? frame.riders[0];
              return (
                <>
                  <span>
                    <strong>{frame.riders.length}</strong> riders ·{" "}
                    {frame.orders.length} orders
                  </span>
                  <span>
                    Rider {rider?.id ?? "unavailable"} ·{" "}
                    {rider?.execution?.stage ?? "Idle"}
                  </span>
                  <span>
                    Custody {rider?.load ?? "unavailable"} / capacity{" "}
                    {rider?.capacity ?? "unavailable"}
                  </span>
                  <span>
                    Next:{" "}
                    {rider?.stops[0]
                      ? rider.stops[0].kind + " #" + rider.stops[0].order
                      : "none"}{" "}
                    · arrival prediction{" "}
                    {rider?.execution
                      ? rider.execution.arrival.toFixed(1) + "s logical"
                      : "unavailable"}
                  </span>
                  <span>
                    Action {rider?.execution?.action_id ?? "none"} ·{" "}
                    {rider?.assigned.length ?? 0} assigned
                  </span>
                </>
              );
            })()
          ) : mode === "dispatch" ? (
            <>
              <span>
                <strong>{data.dispatch.candidates.length}</strong> evaluated
                candidates
              </span>
              <span>Recommended rider {data.dispatch.selected ?? "none"}</span>
              <span>
                Publication:{" "}
                {data.dispatch.committed ? "committed" : "not committed"}
              </span>
              <span>Execution: inactive</span>
            </>
          ) : (
            <>
              <span>
                <strong>A*</strong> · travel time
              </span>
              <span>{distanceLabel(data.route.evidence.distance_meters)}</span>
              <span>
                {durationLabel(data.route.evidence.predicted_elapsed_seconds)}{" "}
                predicted
              </span>
              <span>
                {String(data.route.evidence.search.expanded_states)} expanded
                states
              </span>
            </>
          )}
        </div>
      )}
      <div className="demo-controls">
        <button
          disabled={!data || !!error || reduced}
          onClick={() => {
            if (progress === 1) setProgress(0);
            setPlaying((p) => !p);
          }}
        >
          {playing && !reduced && progress < 1
            ? "Pause"
            : progress === 1
              ? "Replay"
              : "Play demo"}
        </button>
        <input
          aria-label="Demo playback"
          type="range"
          min="0"
          max="1"
          step="0.001"
          value={progress}
          onChange={(e) => setProgress(Number(e.target.value))}
          disabled={!data}
        />
        <span>
          {mode === "delivery"
            ? `${Math.round(at)}s`
            : `${Math.round(progress * 100)}%`}
        </span>
        <a href={`/workspace?mode=${mode}`}>Open workspace ↗</a>
      </div>
      <p className="demo-note">
        {mode === "delivery"
          ? "Original directed routes, accepted execution stages, and estimated movement between retained events."
          : mode === "dispatch"
            ? "A retained recommendation. The reveal is presentation order, not candidate chronology or rider movement."
            : "A* on the checked Lagos road graph. The reveal follows original directed geometry; it is not a vehicle position."}{" "}
        {reduced && "Reduced motion: use the scrubber to inspect the run."}
      </p>
    </section>
  );
}
function RoutePreview({
  result,
  progress,
  reduced,
}: {
  result: RouteResult;
  progress: number;
  reduced: boolean;
}) {
  const container = useRef<HTMLDivElement>(null),
    map = useRef<ml.Map | null>(null),
    [ready, setReady] = useState(false),
    [error, setError] = useState("");
  useEffect(() => {
    if (!container.current) return;
    let m: ml.Map;
    try {
      m = new ml.Map({
        container: container.current,
        cooperativeGestures: true,
        style,
        center: [3.38, 6.524],
        zoom: 15,
      });
      map.current = m;
    } catch {
      setError("WebGL unavailable. Route metrics remain available.");
      return;
    }
    m.addControl(new ml.NavigationControl(), "bottom-right");
    m.on("error", () =>
      setError("Basemap unavailable; engine geometry remains independent."),
    );
    m.on("style.load", () => {
      m.addSource("demo-route", {
        type: "geojson",
        data: { type: "FeatureCollection", features: [] },
      });
      m.addLayer({
        id: "demo-route",
        source: "demo-route",
        type: "line",
        paint: { "line-color": "#b6cba5", "line-width": 6 },
        layout: { "line-join": "round", "line-cap": "round" },
      });
      const points = presentationPath(result.evidence).points;
      if (points.length) {
        const bounds = new ml.LngLatBounds(points[0], points[0]);
        points.forEach((p) => bounds.extend(p));
        m.fitBounds(bounds, { padding: 70, duration: 0, maxZoom: 17 });
      }
      setReady(true);
    });
    return () => {
      m.remove();
      map.current = null;
    };
  }, [result]);
  useEffect(() => {
    if (!ready) return;
    const path = presentationPath(result.evidence);
    const points = revealPrefix(path, reduced ? 1 : progress).line;
    (map.current?.getSource("demo-route") as ml.GeoJSONSource)?.setData({
      type: "Feature",
      properties: {},
      geometry: {
        type: "LineString",
        coordinates: points.length === 1 ? [points[0], points[0]] : points,
      },
    });
  }, [ready, progress, reduced, result]);
  return (
    <>
      <div
        ref={container}
        className="route-demo-map"
        aria-label="A star route on the Lagos graph"
      />
      <div className="route-demo-metrics">
        <strong>A* · free-flow travel time</strong>
        <span>
          {distanceLabel(result.evidence.distance_meters)} ·{" "}
          {durationLabel(result.evidence.predicted_elapsed_seconds)}
        </span>
        {error && <span role="status">{error}</span>}
      </div>
    </>
  );
}
