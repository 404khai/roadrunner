import { useEffect, useRef, useState, useMemo } from "react";
import * as ml from "maplibre-gl";
import { DispatchRadar } from "./DispatchRadar";
import { style } from "./MapView";
import { riderTagAsset } from "./riderPortraitAsset";
import { riderVisual } from "./riderVisual";
import { radiusRing, legPath, type DispatchEvidence } from "./dispatchEvidence";
import { revealPrefix } from "./geometry";
import type { FeatureCollection } from "geojson";
import { H3MapOverlay } from "./H3MapOverlay";
export function DispatchMap({
  evidence,
  inspected,
  sequence,
  playing,
  routeProgress,
  reduced,
  onInspect,
  cooperativeGestures = false,
}: {
  cooperativeGestures?: boolean;
  evidence?: DispatchEvidence;
  inspected: string;
  sequence: number;
  playing: boolean;
  routeProgress: number;
  reduced: boolean;
  onInspect: (id: string) => void;
}) {
  const container = useRef<HTMLDivElement>(null),
    map = useRef<ml.Map | null>(null),
    markers = useRef(
      new Map<string, { marker: ml.Marker; element: HTMLButtonElement }>(),
    ),
    handler = useRef(onInspect),
    pulsed = useRef(new Set<string>());
  handler.current = onInspect;
  const [ready, setReady] = useState(false),
    [error, setError] = useState(""),
    [tilted, setTilted] = useState(false);
  const h3Locations = useMemo(
    () =>
      evidence
        ? [
            ...[
              evidence.pickup,
              evidence.customer,
              ...evidence.riders.map((r) => r.coordinate),
            ].map((c) => [c.longitude, c.latitude] as [number, number]),
            ...(evidence.candidates
              .find((c) => c.rider === inspected)
              ?.legs.flatMap((leg) =>
                leg.geometry ? legPath(leg.geometry).points : [],
              ) ?? []),
          ]
        : [],
    [evidence, inspected],
  );
  useEffect(() => {
    if (!container.current) return;
    let m: ml.Map;
    try {
      m = new ml.Map({
        container: container.current,
        cooperativeGestures,
        style,
        center: [3.38, 6.524],
        zoom: 15,
        maxPitch: 60,
      });
      map.current = m;
    } catch {
      setError(
        "Map/WebGL unavailable. Candidate evidence remains inspectable.",
      );
      return;
    }
    m.addControl(new ml.NavigationControl(), "bottom-right");
    m.on("error", () =>
      setError(
        "Basemap may be unavailable. Engine routes and evidence remain independent.",
      ),
    );
    m.on("load", () => {
      for (const id of ["dispatch-boundary", "dispatch-path", "dispatch-stops"])
        m.addSource(id, {
          type: "geojson",
          data: { type: "FeatureCollection", features: [] },
        });
      m.addLayer({
        id: "dispatch-buildings",
        type: "fill-extrusion",
        source: "basemap",
        "source-layer": "building",
        minzoom: 14,
        paint: {
          "fill-extrusion-color": "#777773",
          "fill-extrusion-height": ["coalesce", ["get", "render_height"], 0],
          "fill-extrusion-base": ["coalesce", ["get", "render_min_height"], 0],
          "fill-extrusion-opacity": 0.8,
        },
      });
      m.addLayer({
        id: "dispatch-boundary",
        type: "line",
        source: "dispatch-boundary",
        paint: {
          "line-color": "#a9c7b4",
          "line-width": 2,
          "line-dasharray": [3, 3],
          "line-opacity": 0.65,
        },
      });
      m.addLayer({
        id: "dispatch-path-casing",
        type: "line",
        source: "dispatch-path",
        paint: { "line-color": "#181818", "line-width": 10 },
      });
      m.addLayer({
        id: "dispatch-path",
        type: "line",
        source: "dispatch-path",
        layout: { "line-cap": "round", "line-join": "round" },
        paint: { "line-color": "#e5d1a4", "line-width": 5 },
      });
      m.addLayer({
        id: "dispatch-stops",
        type: "circle",
        source: "dispatch-stops",
        paint: {
          "circle-radius": 10,
          "circle-color": [
            "match",
            ["get", "kind"],
            "pickup",
            "#b68a44",
            "#8973b9",
          ],
          "circle-stroke-width": 3,
          "circle-stroke-color": "#eee",
        },
      });
      m.addLayer({
        id: "dispatch-stop-labels",
        type: "symbol",
        source: "dispatch-stops",
        layout: {
          "text-field": [
            "match",
            ["get", "kind"],
            "pickup",
            "Restaurant / pickup",
            "Customer",
          ],
          "text-font": ["Noto Sans Regular"],
          "text-size": 12,
          "text-offset": [0, 2],
        },
        paint: {
          "text-color": "#efeee2",
          "text-halo-color": "#191919",
          "text-halo-width": 2,
        },
      });
      setReady(true);
    });
    const entries = markers.current;
    return () => {
      entries.forEach((e) => e.marker.remove());
      entries.clear();
      m.remove();
      map.current = null;
    };
  }, [cooperativeGestures]);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready || !evidence) return;
    const coords = [
      evidence.pickup,
      evidence.customer,
      ...evidence.riders.map((r) => r.coordinate),
    ];
    const p = coords.map((c) => [c.longitude, c.latitude] as [number, number]);
    if (evidence.request.radius_meters !== null)
      p.push(...radiusRing(p[0], evidence.request.radius_meters));
    const bounds = new ml.LngLatBounds(p[0], p[0]);
    p.forEach((v) => bounds.extend(v));
    m.fitBounds(bounds, { padding: 70, maxZoom: 17, duration: 0 });
  }, [ready, evidence]);
  useEffect(() => {
    map.current?.easeTo({
      pitch: tilted ? 55 : 0,
      duration: reduced ? 0 : 700,
    });
  }, [tilted, reduced]);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready) return;
    const set = (id: string, features: FeatureCollection["features"]) =>
      (m.getSource(id) as ml.GeoJSONSource).setData({
        type: "FeatureCollection",
        features,
      });
    const center =
        evidence &&
        ([evidence.pickup.longitude, evidence.pickup.latitude] as [
          number,
          number,
        ]),
      radius = evidence?.request.radius_meters;
    const line = (points: number[][]) => ({
      type: "Feature" as const,
      properties: {},
      geometry: { type: "LineString" as const, coordinates: points },
    });
    set(
      "dispatch-boundary",
      center && radius !== null && radius !== undefined
        ? [line(radiusRing(center, radius))]
        : [],
    );
    set(
      "dispatch-stops",
      evidence
        ? [evidence.pickup, evidence.customer].map((c, i) => ({
            type: "Feature",
            properties: { kind: i === 0 ? "pickup" : "customer" },
            geometry: { type: "Point", coordinates: [c.longitude, c.latitude] },
          }))
        : [],
    );
    const candidate = evidence?.candidates.find((c) => c.rider === inspected);
    const paths =
      candidate?.legs.flatMap((l) =>
        l.geometry ? [legPath(l.geometry)] : [],
      ) ?? [];
    const target = paths.reduce((sum, p) => sum + p.total, 0) * routeProgress;
    let elapsed = 0;
    set(
      "dispatch-path",
      paths.flatMap((p) => {
        const local = p.total
          ? Math.min(1, Math.max(0, (target - elapsed) / p.total))
          : 1;
        elapsed += p.total;
        return p.points.length > 1 && local > 0
          ? [line(revealPrefix(p, local).line)]
          : [];
      }),
    );
  }, [ready, evidence, sequence, routeProgress, inspected]);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready) return;
    const keep = new Set<string>();
    for (const r of evidence?.riders ?? []) {
      const key = `${evidence!.evaluation_id}:${r.id}`;
      keep.add(key);
      let e = markers.current.get(key);
      if (!e) {
        const element = document.createElement("button"),
          img = document.createElement("img");
        element.type = "button";
        element.className = "dispatch-rider-tag";
        img.src = riderTagAsset(r.id);
        img.alt = "";
        const label = document.createElement("span");
        label.className = "dispatch-rider-id";
        label.textContent = `R${r.id}`;
        const star = document.createElement("span");
        star.className = "dispatch-recommendation-star";
        star.setAttribute("aria-hidden", "true");
        star.innerHTML =
          '<svg viewBox="0 0 24 24" fill="currentColor"><path d="m12 2 3.09 6.26L22 9.27l-5 4.87L18.18 21 12 17.76 5.82 21 7 14.14 2 9.27l6.91-1.01Z"/></svg>';
        const pulse = document.createElement("span");
        pulse.className = "dispatch-scan-pulse";
        pulse.setAttribute("aria-hidden", "true");
        element.append(pulse, star, img, label);
        element.addEventListener("click", () => handler.current(r.id));
        e = {
          element,
          marker: new ml.Marker({ element, anchor: "bottom" })
            .setLngLat([r.coordinate.longitude, r.coordinate.latitude])
            .addTo(m),
        };
        markers.current.set(key, e);
      }
      const recommended = sequence >= 1 && evidence!.selected === r.id;
      e.element.setAttribute(
        "aria-label",
        `Inspect dispatch rider ${r.id}${recommended ? ", recommended candidate" : ""}`,
      );
      e.element.dataset.candidate = String(
        sequence >= 0.3 &&
          sequence < 1 &&
          evidence!.candidates.some((c) => c.rider === r.id),
      );
      e.element.dataset.recommended = String(recommended);
      e.element.dataset.inspected = String(sequence >= 1 && inspected === r.id);
      e.element.style.setProperty("--rider-color", riderVisual(r.id).body);
    }
    for (const [key, e] of markers.current)
      if (!keep.has(key)) {
        e.marker.remove();
        markers.current.delete(key);
      }
  }, [ready, evidence, inspected, sequence]);
  useEffect(() => {
    if (sequence === 0) pulsed.current.clear();
    markers.current.forEach(({ element }) => {
      element
        .querySelector(".dispatch-scan-pulse")
        ?.getAnimations()
        .forEach((animation) => {
          if (sequence === 0 || sequence >= 1) animation.cancel();
          else if (playing) animation.play();
          else animation.pause();
        });
    });
  }, [sequence, playing]);
  return (
    <>
      <div
        className="delivery-canvas"
        ref={container}
        aria-label="Dispatch candidate map"
      />
      <div className="dispatch-view-controls">
        <H3MapOverlay
          map={ready ? map.current : null}
          locations={h3Locations}
          beforeLayer="dispatch-boundary"
          scope="dispatch"
        />
        <button onClick={() => setTilted((v) => !v)}>
          {tilted ? "Overhead view" : "Perspective view"}
        </button>
        <button
          disabled={
            !evidence?.candidates
              .find((c) => c.rider === inspected)
              ?.legs.some((l) => l.geometry)
          }
          onClick={() => {
            const m = map.current,
              points = evidence?.candidates
                .find((c) => c.rider === inspected)
                ?.legs.flatMap((l) =>
                  l.geometry ? legPath(l.geometry).points : [],
                );
            if (!m || !points?.length) return;
            const b = new ml.LngLatBounds(points[0], points[0]);
            points.forEach((p) => b.extend(p));
            m.fitBounds(b, {
              padding: 80,
              maxZoom: 18,
              duration: reduced ? 0 : 600,
            });
          }}
        >
          Fit candidate route
        </button>
      </div>
      {error && (
        <p className="delivery-map-error" role="status">
          {error}
        </p>
      )}
      {evidence && ready && map.current && sequence < 1 && !reduced && (
        <DispatchRadar
          key={evidence.evaluation_id}
          map={map.current}
          longitude={evidence.pickup.longitude}
          latitude={evidence.pickup.latitude}
          radius={evidence.request.radius_meters}
          paused={!playing}
          progress={sequence}
          onSweep={(from, to, bounds) => {
            if (to < from) {
              pulsed.current.clear();
              markers.current.forEach(({ element }) =>
                element
                  .querySelector(".dispatch-scan-pulse")
                  ?.getAnimations()
                  .forEach((animation) => animation.cancel()),
              );
              return;
            }
            const tau = Math.PI * 2;
            markers.current.forEach(({ element }, key) => {
              if (pulsed.current.has(key)) return;
              const tag = element.getBoundingClientRect();
              const dx = tag.x + tag.width / 2 - (bounds.x + bounds.width / 2);
              const dy = tag.y + 17 - (bounds.y + bounds.height / 2);
              const bearing = (Math.atan2(dx, -dy) + tau) % tau;
              if (
                Math.hypot(dx, dy) > bounds.width * 0.47 ||
                bearing < from ||
                bearing >= to
              )
                return;
              pulsed.current.add(key);
              element.querySelector(".dispatch-scan-pulse")?.animate(
                [
                  {
                    transform: "translate(-50%, -50%) scale(0.5)",
                    opacity: 0.95,
                  },
                  { transform: "translate(-50%, -50%) scale(2.8)", opacity: 0 },
                ],
                { duration: 550, easing: "ease-out" },
              );
            });
          }}
        />
      )}
    </>
  );
}
