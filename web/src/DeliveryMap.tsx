import { useEffect, useRef, useState } from "react";
import * as ml from "maplibre-gl";
import type { FeatureCollection } from "geojson";
import { style } from "./MapView";
import { frameAt, sampleRider, actionKey, type Scene } from "./deliveryScene";
import { scooterAsset, riderVisual } from "./riderVisual";
import type { Position } from "./geometry";
// Renderer-owned stop glyphs; semantic type is also carried by labels and color.
function stopGlyph(kind: "pickup" | "delivery") {
  const canvas = document.createElement("canvas");
  canvas.width = 32;
  canvas.height = 32;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 2.5;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.beginPath();
  if (kind === "pickup") {
    ctx.moveTo(10, 14);
    ctx.lineTo(10, 26);
    ctx.moveTo(6, 6);
    ctx.lineTo(6, 12);
    ctx.quadraticCurveTo(10, 17, 14, 12);
    ctx.lineTo(14, 6);
    ctx.moveTo(10, 6);
    ctx.lineTo(10, 12);
    ctx.moveTo(24, 26);
    ctx.lineTo(24, 6);
    ctx.quadraticCurveTo(17, 11, 20, 17);
    ctx.lineTo(24, 17);
  } else {
    ctx.arc(16, 10, 4, 0, Math.PI * 2);
    ctx.moveTo(8, 26);
    ctx.quadraticCurveTo(8, 18, 16, 18);
    ctx.quadraticCurveTo(24, 18, 24, 26);
    ctx.closePath();
  }
  ctx.stroke();
  return { width: 32, height: 32, data: ctx.getImageData(0, 0, 32, 32).data };
}
export function DeliveryMap({
  scene,
  at,
  selected,
  onSelect,
  traffic,
  onEdge,
}: {
  scene?: Scene;
  at: number;
  selected: string;
  onSelect: (id: string) => void;
  traffic: boolean;
  onEdge: (id: number) => void;
}) {
  const container = useRef<HTMLDivElement>(null),
    map = useRef<ml.Map | null>(null),
    markers = useRef(
      new Map<
        string,
        {
          marker: ml.Marker;
          element: HTMLButtonElement;
          img: HTMLImageElement;
          label: HTMLSpanElement;
        }
      >(),
    );
  const [fitId, setFitId] = useState(0);
  const [ready, setReady] = useState(false),
    [error, setError] = useState(""),
    [basemapError, setBasemapError] = useState(false);
  const handlers = useRef({ onSelect, onEdge });
  handlers.current = { onSelect, onEdge };
  useEffect(() => {
    if (!container.current) return;
    let m: ml.Map;
    try {
      m = new ml.Map({
        container: container.current,
        style,
        center: [3.4, 6.5],
        zoom: 12,
        pitch: 0,
        maxPitch: 0,
        bearing: 0,
        attributionControl: { compact: true },
      });
      map.current = m;
    } catch {
      setError("WebGL unavailable. Execution and evidence remain inspectable.");
      return;
    }
    m.addControl(
      new ml.NavigationControl({ showCompass: false }),
      "bottom-right",
    );
    m.on("error", (e) => {
      if (e.error) setBasemapError(true);
    });
    m.on("style.load", () => {
      for (const id of ["delivery-roads", "delivery-routes", "delivery-stops"])
        m.addSource(id, {
          type: "geojson",
          data: { type: "FeatureCollection", features: [] },
        });
      m.addLayer({
        id: "delivery-roads",
        type: "line",
        source: "delivery-roads",
        layout: { "line-cap": "round", "line-join": "round" },
        paint: {
          "line-color": [
            "step",
            ["get", "factor"],
            "#57b7a0",
            1.4,
            "#ddbc5d",
            2,
            "#ed8860",
            3,
            "#e35b69",
          ],
          "line-width": 3,
          "line-offset": 5,
          "line-opacity": 0.75,
        },
      });
      m.addLayer({
        id: "delivery-routes-casing",
        type: "line",
        source: "delivery-routes",
        layout: { "line-cap": "round", "line-join": "round" },
        paint: { "line-color": "#151515", "line-width": 11 },
      });
      m.addLayer({
        id: "delivery-routes",
        type: "line",
        source: "delivery-routes",
        layout: { "line-cap": "round", "line-join": "round" },
        paint: {
          "line-color": ["get", "color"],
          "line-width": ["case", ["get", "selected"], 6, 3],
          "line-opacity": 0.95,
        },
      });
      m.addLayer({
        id: "delivery-stops",
        type: "circle",
        source: "delivery-stops",
        paint: {
          "circle-radius": 13,
          "circle-color": [
            "match",
            ["get", "kind"],
            "pickup",
            "#b68a44",
            "#8973b9",
          ],
          "circle-stroke-width": 3,
          "circle-stroke-color": "#202020",
        },
      });
      for (const kind of ["pickup", "delivery"] as const) {
        const glyph = stopGlyph(kind);
        if (glyph) m.addImage(`stop-${kind}`, glyph, { pixelRatio: 2 });
      }
      m.addLayer({
        id: "delivery-stop-icons",
        type: "symbol",
        source: "delivery-stops",
        layout: {
          "icon-image": [
            "match",
            ["get", "kind"],
            "pickup",
            "stop-pickup",
            "stop-delivery",
          ],
          "icon-allow-overlap": true,
          "icon-ignore-placement": true,
        },
      });
      m.addLayer({
        id: "delivery-stop-labels",
        type: "symbol",
        source: "delivery-stops",
        layout: {
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Regular"],
          "text-size": 11,
          "text-offset": [0, 2.1],
          "text-allow-overlap": false,
        },
        paint: {
          "text-color": [
            "match",
            ["get", "kind"],
            "pickup",
            "#f1d5a3",
            "#d8c5fa",
          ],
          "text-halo-color": "#191919",
          "text-halo-width": 2,
        },
      });
      m.on("click", "delivery-routes", (e) => {
        const id = e.features?.[0]?.properties?.rider;
        if (id) handlers.current.onSelect(String(id));
      });
      m.on("click", "delivery-roads", (e) => {
        const id = e.features?.[0]?.properties?.edge;
        if (id !== undefined) handlers.current.onEdge(Number(id));
      });
      setReady(true);
    });
    const lost = (e: Event) => {
      e.preventDefault();
      setError(
        "Map context lost. Reload to restore the map; execution remains inspectable.",
      );
    };
    m.getCanvas().addEventListener("webglcontextlost", lost);
    const resized = () => setFitId((v) => v + 1);
    window.addEventListener("resize", resized);
    const entries = markers.current;
    return () => {
      window.removeEventListener("resize", resized);
      entries.forEach((v) => v.marker.remove());
      entries.clear();
      m.getCanvas().removeEventListener("webglcontextlost", lost);
      m.remove();
      map.current = null;
    };
  }, []);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready || !scene) return;
    const points = scene.frames[0].riders
      .map((r) => [r.coordinate.longitude, r.coordinate.latitude] as Position)
      .concat([...scene.paths.values()].flatMap((p) => p.points));
    if (!points.length) return;
    const bounds = new ml.LngLatBounds(points[0], points[0]);
    points.forEach((p) => bounds.extend(p));
    const { clientWidth: width, clientHeight: height } = m.getContainer();
    const padding =
      width <= 500
        ? { top: 310, bottom: 210, left: 50, right: 70 }
        : { top: 230, bottom: 170, left: 90, right: 340 };
    const scale = Math.min(
      1,
      Math.max(80, height - 140) / (padding.top + padding.bottom),
    );
    padding.top *= scale;
    padding.bottom *= scale;
    const horizontal = Math.min(
      1,
      Math.max(80, width - 100) / (padding.left + padding.right),
    );
    padding.left *= horizontal;
    padding.right *= horizontal;
    m.fitBounds(bounds, { padding, maxZoom: 17, duration: 0 });
  }, [ready, scene, fitId]);
  const frame = scene && frameAt(scene, at);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready) return;
    const f = frame;
    const keep = new Set<string>();
    for (const r of f?.riders ?? []) {
      const sample = sampleRider(scene!, r, at),
        key = `${scene!.run_identity}:${r.id}`;
      if (!sample.position) continue;
      keep.add(key);
      let entry = markers.current.get(key);
      if (!entry) {
        const element = document.createElement("button");
        element.className = "scooter-marker";
        element.type = "button";
        element.dataset.rider = r.id;
        const img = document.createElement("img");
        img.src = scooterAsset(scene!.visualIds.get(r.id) ?? r.id);
        element.style.setProperty(
          "--rider-color",
          riderVisual(scene!.visualIds.get(r.id) ?? r.id).body,
        );
        img.alt = "";
        const label = document.createElement("span");
        element.append(img, label);
        element.addEventListener("click", () =>
          handlers.current.onSelect(r.id),
        );
        const marker = new ml.Marker({
          element,
          anchor: "center",
          rotationAlignment: "map",
          pitchAlignment: "map",
        })
          .setLngLat(sample.position)
          .addTo(m);
        entry = { marker, element, img, label };
        markers.current.set(key, entry);
      }
      entry.marker.setLngLat(sample.position);
      entry.img.style.transform = `rotate(${sample.bearing ?? 0}deg)`;
      entry.element.dataset.action = r.execution?.action_id ?? "";
      entry.element.dataset.generation = r.execution?.generation ?? "";
      entry.element.dataset.selected = String(selected === r.id);
      entry.element.setAttribute(
        "aria-label",
        `Inspect rider ${r.id}: ${r.execution?.stage ?? "Idle"}. ${sample.quality}`,
      );
      entry.element.title = `Rider ${r.id} · ${r.execution?.stage ?? "Idle"}\n${sample.quality}\nRoute-derived bearing; not measured GPS`;
      entry.label.textContent = `R${r.id}`;
    }
    for (const [key, v] of markers.current)
      if (!keep.has(key)) {
        v.marker.remove();
        markers.current.delete(key);
      }
  }, [ready, scene, at, selected, frame]);
  // Geometry is uploaded only at captured transitions or selection/context changes.
  // Animation frames move the existing DOM markers without retransmitting paths.
  useEffect(() => {
    const m = map.current;
    if (!m || !ready) return;
    const f = frame;
    const source = (id: string, features: FeatureCollection["features"]) =>
      (m.getSource(id) as ml.GeoJSONSource).setData({
        type: "FeatureCollection",
        features,
      });
    source(
      "delivery-routes",
      (f?.riders ?? []).flatMap((r) => {
        const p =
          r.execution &&
          scene!.paths.get(actionKey(r.id, r.execution.action_id));
        return p && p.points.length > 1
          ? [
              {
                type: "Feature" as const,
                properties: {
                  rider: r.id,
                  selected: r.id === selected,
                  color: riderVisual(scene!.visualIds.get(r.id) ?? r.id).body,
                },
                geometry: {
                  type: "LineString" as const,
                  coordinates: p.points,
                },
              },
            ]
          : [];
      }),
    );
    source(
      "delivery-stops",
      (f?.orders ?? [])
        .filter((o) => o.status !== "cancelled" && o.status !== "delivered")
        .flatMap((o) =>
          (["pickup", "delivery"] as const)
            .filter(
              (kind) => kind !== "pickup" || o.status === "awaiting_pickup",
            )
            .map((kind) => {
              const c = kind === "pickup" ? o.pickup : o.dropoff;
              return {
                type: "Feature" as const,
                properties: {
                  kind,
                  label: `${kind === "pickup" ? "Pickup" : "Customer"} #${o.id}`,
                },
                geometry: {
                  type: "Point" as const,
                  coordinates: [c.longitude, c.latitude],
                },
              };
            }),
        ),
    );
    source(
      "delivery-roads",
      traffic && scene?.trafficSupported && f
        ? scene.roads.edges.map((e) => ({
            type: "Feature" as const,
            properties: {
              edge: e.edge,
              factor:
                f.traffic?.find((t) => t.edge_id === e.edge)?.multiplier ?? 1,
            },
            geometry: {
              type: "LineString" as const,
              coordinates: e.points_e7.map((p) => [
                p.longitude_e7 / 1e7,
                p.latitude_e7 / 1e7,
              ]),
            },
          }))
        : [],
    );
  }, [ready, scene, frame, selected, traffic]);
  return (
    <div className="delivery-map" aria-label="Synthetic delivery map">
      <div className="delivery-canvas" ref={container} />
      <button
        className="delivery-fit"
        disabled={!scene}
        onClick={() => setFitId((v) => v + 1)}
      >
        Fit scene
      </button>
      <div className="delivery-map-label">
        <span className="status-dot" /> SYNTHETIC EXECUTION{" "}
        <small>2D · estimated motion · no GPS feed</small>
      </div>
      {error && (
        <p className="delivery-map-error" role="alert">
          {error}
        </p>
      )}
      {basemapError && (
        <p className="delivery-basemap-error">
          Basemap context may be unavailable. Engine geometry is independent.
        </p>
      )}
      {!scene && (
        <div className="delivery-map-empty">
          <h2>From plans to deliveries.</h2>
          <p>
            Run a Roadrunner scenario to follow its riders, stops and accepted
            execution.
          </p>
        </div>
      )}
      <div className="delivery-legend">
        <span className="pickup-legend">● Restaurant / pickup</span>
        <span className="customer-legend">● Customer / delivery</span>
        <span>▰ Delivery scooter</span>
        {traffic && scene?.trafficSupported && (
          <small>
            Road factors: green 1× · gold ≥1.4× · orange ≥2× · red ≥3×
            <br />
            Synthetic context · first 200 directed edges, not city coverage
          </small>
        )}
      </div>
    </div>
  );
}
