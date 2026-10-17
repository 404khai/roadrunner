import { useEffect, useRef, useState, useMemo } from "react";
import * as ml from "maplibre-gl";
import type { FeatureCollection } from "geojson";
import { style } from "./MapView";
import { frameAt, sampleRider, actionKey, type Scene } from "./deliveryScene";
import { perspectiveScooter, relativeDirection } from "./perspectiveScooter";
import { riderTagAsset } from "./riderPortraitAsset";
import type { AuditFocus } from "./DispatchAuditCard";
import { CameraGizmo } from "./CameraGizmo";
import { useReveal } from "./useReveal";
import { revealPrefix } from "./geometry";
import { scooterAsset, riderVisual } from "./riderVisual";
import type { Position } from "./geometry";
import {
  ambientActors,
  sampleAmbient,
  ambientSprite,
  type VehiclePose,
} from "./ambientScene";
import type { ImmersiveLayer } from "./immersiveLayer";
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
  reduced = false,
  selectionRevision = 0,
  dispatchFocus,
  showNetwork = false,
  mini = false,
  cooperativeGestures = false,
}: {
  mini?: boolean;
  showNetwork?: boolean;
  cooperativeGestures?: boolean;
  scene?: Scene;
  at: number;
  selected: string;
  onSelect: (id: string) => void;
  traffic: boolean;
  onEdge: (id: number) => void;
  reduced?: boolean;
  selectionRevision?: number;
  dispatchFocus?: AuditFocus;
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
  const [perspective, setPerspective] = useState(false),
    [follow, setFollow] = useState(false),
    [buildings, setBuildings] = useState(true),
    [cameraBearing, setCameraBearing] = useState(0),
    [cameraPitch, setCameraPitch] = useState(0),
    [cameraZoom, setCameraZoom] = useState(19);
  const [immersive, setImmersive] = useState(false),
    [ambientCount, setAmbientCount] = useState(0),
    [modelError, setModelError] = useState(false),
    [modelsReady, setModelsReady] = useState(false);
  const modelLayer = useRef<ImmersiveLayer | null>(null);
  const ambientMarkers = useRef(
    new Map<string, { marker: ml.Marker; img: HTMLImageElement }>(),
  );
  const actors = useMemo(
    () => (scene ? ambientActors(scene, ambientCount) : []),
    [scene, ambientCount],
  );
  const lastFollow = useRef(0),
    cinematicUntil = useRef(0),
    followTarget = useRef("");
  const [autoFollow, setAutoFollow] = useState(true);
  const [navigationOpen, setNavigationOpen] = useState(
    () => !mini && window.innerWidth > 780,
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
        cooperativeGestures,
        style,
        center: [3.4, 6.5],
        zoom: 12,
        pitch: 0,
        maxPitch: 70,
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
    m.on("rotate", () => setCameraBearing(m.getBearing()));
    m.on("pitch", () => setCameraPitch(m.getPitch()));
    m.on("zoom", () => setCameraZoom(m.getZoom()));
    m.on("idle", () => {
      const c = m.getCanvas();
      c.dataset.viewZoom = String(m.getZoom());
      c.dataset.viewPitch = String(m.getPitch());
      if (m.getLayer("delivery-buildings"))
        c.dataset.buildingFeatures = String(
          m.queryRenderedFeatures({ layers: ["delivery-buildings"] }).length,
        );
    });
    m.on("movestart", (event) => {
      if (event.originalEvent) setFollow(false);
    });
    m.on("error", (e) => {
      if (e.error) setBasemapError(true);
    });
    m.on("style.load", () => {
      for (const id of [
        "delivery-network",
        "delivery-roads",
        "delivery-routes",
        "delivery-stops",
        "delivery-reveal",
      ])
        m.addSource(id, {
          type: "geojson",
          data: { type: "FeatureCollection", features: [] },
        });
      m.addLayer({
        id: "delivery-buildings",
        type: "fill-extrusion",
        source: "basemap",
        "source-layer": "building",
        minzoom: 14,
        filter: [
          "all",
          ["!=", ["get", "hide_3d"], true],
          [">", ["coalesce", ["get", "render_height"], 0], 0],
        ],
        layout: { visibility: "none" },
        paint: {
          "fill-extrusion-color": "#777773",
          "fill-extrusion-height": ["coalesce", ["get", "render_height"], 0],
          "fill-extrusion-base": ["coalesce", ["get", "render_min_height"], 0],
          "fill-extrusion-opacity": 1,
        },
      });
      m.addLayer({
        id: "delivery-network",
        type: "line",
        source: "delivery-network",
        paint: {
          "line-color": "#777b72",
          "line-width": 2,
          "line-opacity": 0.65,
        },
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
        id: "delivery-reveal",
        type: "line",
        source: "delivery-reveal",
        layout: { "line-cap": "round", "line-join": "round" },
        paint: {
          "line-color": "#f4f4ef",
          "line-width": 4,
          "line-opacity": 0.9,
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
      setCameraZoom(m.getZoom());
      setReady(true);
    });
    const lost = (e: Event) => {
      e.preventDefault();
      setError(
        "Map context lost. Reload to restore the map; execution remains inspectable.",
      );
    };
    m.getCanvas().addEventListener("webglcontextlost", lost);
    const resized = () => {
      setFollow(false);
      setFitId((v) => v + 1);
    };
    window.addEventListener("resize", resized);
    const entries = markers.current;
    const ambientEntries = ambientMarkers.current;
    return () => {
      window.removeEventListener("resize", resized);
      entries.forEach((v) => v.marker.remove());
      entries.clear();
      ambientEntries.forEach((e) => e.marker.remove());
      ambientEntries.clear();
      m.getCanvas().removeEventListener("webglcontextlost", lost);
      m.remove();
      map.current = null;
    };
  }, [cooperativeGestures]);
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
    m.fitBounds(bounds, {
      padding,
      maxZoom: 17,
      duration: 0,
      pitch: m.getPitch(),
      bearing: m.getBearing(),
    });
  }, [ready, scene, fitId]);
  const frame = scene && frameAt(scene, at);
  const activeRider = frame?.riders.find((r) => r.id === selected);
  const activeSample = useMemo(
    () =>
      scene && activeRider ? sampleRider(scene, activeRider, at) : undefined,
    [scene, activeRider, at],
  );
  const activePath =
    scene && activeRider?.execution
      ? scene.paths.get(actionKey(selected, activeRider.execution.action_id))
      : undefined;
  const reveal = useReveal(
    scene && activeRider?.execution
      ? `${scene.run_identity}:${selected}:${activeRider.execution.action_id}`
      : undefined,
    !activePath || activePath.total === 0,
    reduced,
  );
  useEffect(() => {
    const m = map.current;
    if (!m || !ready) return;
    m.easeTo({
      pitch: immersive ? 65 : perspective ? 55 : 0,
      bearing: perspective ? m.getBearing() : 0,
      duration: reduced ? 0 : 800,
    });
  }, [ready, perspective, immersive, reduced]);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready) return;
    m.setLayoutProperty(
      "delivery-buildings",
      "visibility",
      (perspective || immersive) && buildings ? "visible" : "none",
    );
  }, [ready, perspective, immersive, buildings]);
  useEffect(() => {
    if (autoFollow && (perspective || immersive)) {
      lastFollow.current = 0;
      cinematicUntil.current = 0;
      followTarget.current = "";
      setFollow(true);
    }
  }, [selected, selectionRevision, perspective, immersive, autoFollow]);
  useEffect(() => {
    if (follow && !activeSample?.position) setFollow(false);
  }, [follow, activeSample]);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready || !follow || !activeSample?.position) return;
    const now = performance.now();
    if (now - lastFollow.current < 100 || now < cinematicUntil.current) return;
    lastFollow.current = now;
    const target = `${selected}:${perspective}:${immersive}:${selectionRevision}`;
    const cinematic = followTarget.current !== target;
    followTarget.current = target;
    if (cinematic && !reduced) cinematicUntil.current = now + 1100;
    m.easeTo({
      center: activeSample.position,
      bearing: activeSample.bearing ?? 0,
      pitch: immersive ? 65 : perspective ? 55 : 0,
      zoom: immersive ? 20.5 : perspective ? 18.5 : 17.5,
      offset: [0, m.getContainer().clientHeight * 0.12],
      duration: reduced ? 0 : cinematic ? 1100 : 160,
    });
  }, [
    ready,
    follow,
    perspective,
    immersive,
    activeSample,
    reduced,
    selected,
    selectionRevision,
  ]);
  useEffect(() => {
    const m = map.current;
    if (!m || !ready) return;
    (m.getSource("delivery-reveal") as ml.GeoJSONSource).setData({
      type: "FeatureCollection",
      features:
        activePath && reveal.progress < 1 && activePath.points.length > 1
          ? [
              {
                type: "Feature",
                properties: {},
                geometry: {
                  type: "LineString",
                  coordinates: revealPrefix(activePath, reveal.progress).line,
                },
              },
            ]
          : [],
    });
  }, [ready, activePath, reveal.progress]);

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
          rotationAlignment: "viewport",
          pitchAlignment: "viewport",
        })
          .setLngLat(sample.position)
          .addTo(m);
        entry = { marker, element, img, label };
        markers.current.set(key, entry);
      }
      entry.marker.setLngLat(sample.position);
      const tag = cameraZoom < 18;
      entry.img.style.visibility =
        immersive && !tag && modelsReady && !modelError ? "hidden" : "visible";
      const visualId = scene!.visualIds.get(r.id) ?? r.id;
      const tilted = perspective && cameraPitch >= 20;
      const image = tag
        ? riderTagAsset(visualId)
        : tilted
          ? perspectiveScooter(
              visualId,
              relativeDirection(sample.bearing ?? 0, cameraBearing),
            )
          : scooterAsset(visualId);
      if (entry.img.getAttribute("src") !== image) entry.img.src = image;
      entry.img.style.transform =
        tag || tilted
          ? "none"
          : `rotate(${(sample.bearing ?? 0) - cameraBearing}deg)`;
      entry.element.dataset.view = tag
        ? "tag"
        : tilted
          ? "perspective"
          : "overhead";
      entry.marker.setOffset(tag ? [0, -18] : tilted ? [0, -28] : [0, 0]);
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
  }, [
    ready,
    scene,
    at,
    selected,
    frame,
    perspective,
    immersive,
    modelError,
    modelsReady,
    cameraBearing,
    cameraZoom,
    cameraPitch,
  ]);
  // Lazy renderer: overhead and independently usable 2.5D never require Three.js.
  useEffect(() => {
    const m = map.current;
    if (!ready || !m || !immersive) return;
    let cancelled = false;
    import("./immersiveLayer")
      .then(({ immersiveLayer }) => {
        if (cancelled) return;
        try {
          const layer = immersiveLayer(() => setModelError(true));
          m.addLayer(layer);
          modelLayer.current = layer;
          setModelsReady(true);
          setModelError(false);
        } catch {
          if (m.getLayer("delivery-models")) m.removeLayer("delivery-models");
          setModelError(true);
        }
      })
      .catch(() => {
        if (!cancelled) setModelError(true);
      });
    return () => {
      cancelled = true;
      if (modelLayer.current) {
        if (map.current === m && m.getLayer("delivery-models"))
          m.removeLayer("delivery-models");
        modelLayer.current = null;
        setModelsReady(false);
      }
    };
  }, [ready, immersive]);
  useEffect(() => {
    const m = map.current;
    if (!ready || !m) return;
    const poses: VehiclePose[] = actors.map((a) =>
      sampleAmbient(a, at - (scene?.start ?? 0)),
    );
    const keep = new Set<string>();
    for (const p of poses) {
      if (immersive && modelsReady && !modelError) continue;
      keep.add(p.id);
      let entry = ambientMarkers.current.get(p.id);
      if (!entry) {
        const element = document.createElement("div"),
          img = document.createElement("img");
        element.className = "ambient-marker";
        element.title = `Synthetic ${p.kind} · cosmetic trajectory · not tracked traffic`;
        img.alt = "";
        element.append(img);
        entry = {
          marker: new ml.Marker({
            element,
            rotationAlignment: "viewport",
            pitchAlignment: "viewport",
          })
            .setLngLat(p.position)
            .addTo(m),
          img,
        };
        ambientMarkers.current.set(p.id, entry);
      }
      const ambientTilt = perspective && cameraPitch >= 20;
      const src = ambientSprite(
        p.kind === "rider" ? "motorcycle" : p.kind,
        p.color,
        ambientTilt ? p.bearing - cameraBearing : undefined,
      );
      if (entry.img.getAttribute("src") !== src) entry.img.src = src;
      entry.img.style.transform = perspective
        ? "none"
        : `rotate(${p.bearing - cameraBearing}deg)`;
      entry.marker.setPitchAlignment("viewport");
      entry.marker.setLngLat(p.position);
    }
    for (const [id, e] of ambientMarkers.current)
      if (!keep.has(id)) {
        e.marker.remove();
        ambientMarkers.current.delete(id);
      }
    if (immersive && scene) {
      for (const r of cameraZoom >= 18 ? (frame?.riders ?? []) : []) {
        const sample = sampleRider(scene, r, at);
        if (!sample.position) continue;
        poses.push({
          id: `rider:${r.id}`,
          kind: "rider",
          position: sample.position,
          bearing: sample.bearing ?? 0,
          distance:
            sample.progress *
            (r.execution
              ? (scene.paths.get(actionKey(r.id, r.execution.action_id))
                  ?.total ?? 0)
              : 0),
          color: riderVisual(scene.visualIds.get(r.id) ?? r.id).body,
          visualId: scene.visualIds.get(r.id) ?? r.id,
        });
      }
      modelLayer.current?.update(poses, reduced);
    }
  }, [
    ready,
    actors,
    scene,
    frame,
    at,
    immersive,
    modelError,
    modelsReady,
    perspective,
    cameraBearing,
    cameraZoom,
    cameraPitch,
    reduced,
  ]);
  useEffect(() => {
    const m = map.current;
    if (!ready || !m || !dispatchFocus) return;
    const element = document.createElement("div");
    element.className = reduced ? "audit-pulse reduced" : "audit-pulse";
    element.setAttribute("role", "status");
    element.textContent = dispatchFocus.label;
    const marker = new ml.Marker({ element, anchor: "center" })
      .setLngLat(dispatchFocus.center)
      .addTo(m);
    const timer = window.setTimeout(
      () => marker.remove(),
      reduced ? 5000 : 3000,
    );
    return () => {
      window.clearTimeout(timer);
      marker.remove();
    };
  }, [ready, dispatchFocus, reduced]);
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
      "delivery-network",
      showNetwork && scene
        ? scene.roads.edges.map((e) => ({
            type: "Feature" as const,
            properties: {},
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
  }, [ready, scene, frame, selected, traffic, showNetwork]);
  const viewButtons = (
    <>
      <div role="group" aria-label="Map perspective">
        <button
          aria-pressed={!perspective && !immersive}
          onClick={() => {
            setPerspective(false);
            setImmersive(false);
          }}
        >
          Overhead 2D
        </button>
        <button
          aria-pressed={perspective && !immersive}
          onClick={() => {
            setPerspective(true);
            setImmersive(false);
          }}
        >
          Perspective 2.5D
        </button>
      </div>
      <button
        aria-pressed={immersive}
        onClick={() => {
          setPerspective(true);
          setImmersive(true);
          setBuildings(true);
        }}
      >
        Immersive 3D
      </button>
    </>
  );
  return (
    <div className="delivery-map" aria-label="Synthetic delivery map">
      <div className="delivery-canvas" ref={container} />
      <button
        className="delivery-fit"
        disabled={!scene}
        onClick={() => {
          setFollow(false);
          setFitId((v) => v + 1);
        }}
      >
        Fit scene
      </button>
      {(perspective || immersive) && (
        <CameraGizmo
          bearing={cameraBearing}
          pitch={cameraPitch}
          onOrbit={(bearing, pitch) => {
            setFollow(false);
            map.current?.jumpTo({
              center: activeSample?.position,
              bearing,
              pitch,
            });
          }}
          onPreset={(axis) => {
            setFollow(false);
            const m = map.current;
            if (m)
              m.easeTo({
                center: activeSample?.position,
                bearing: axis === "X" ? 90 : 0,
                pitch: axis === "Z" ? 0 : 60,
                duration: reduced ? 0 : 700,
              });
          }}
          onFront={() => {
            setFollow(false);
            map.current?.easeTo({
              center: activeSample?.position,
              bearing: (activeSample?.bearing ?? 0) + 180,
              zoom: immersive ? 20.5 : 18.5,
              pitch: immersive ? 65 : 55,
              duration: reduced ? 0 : 700,
            });
          }}
          onRear={() => {
            lastFollow.current = 0;
            cinematicUntil.current = 0;
            followTarget.current = "";
            setFollow(true);
          }}
        />
      )}
      {mini && <div className="mini-view-modes">{viewButtons}</div>}
      <details
        open={navigationOpen}
        onToggle={(e) => setNavigationOpen(e.currentTarget.open)}
        className="navigation-card"
        aria-label="Navigation presentation"
      >
        <summary>{mini ? "Scene controls" : "Navigation view"}</summary>
        <div className="navigation-card-body">
          {!mini && viewButtons}
          <label>
            <input
              type="checkbox"
              checked={autoFollow}
              onChange={(e) => {
                setAutoFollow(e.target.checked);
                if (!e.target.checked) setFollow(false);
              }}
            />{" "}
            Auto-follow selected rider
          </label>
          <label>
            Synthetic ambient vehicles{" "}
            <select
              aria-label="Synthetic ambient vehicles"
              value={ambientCount}
              onChange={(e) => setAmbientCount(Number(e.target.value))}
            >
              <option value={0}>Off</option>
              <option value={12}>12 · light</option>
              <option value={24}>24 · standard</option>
              <option value={48}>48 · maximum</option>
            </select>
          </label>
          {ambientCount > 0 && (
            <small>
              Decorative car/truck/van/motorcycle scene. Fixed trajectories
              repeat at road endpoints; no traffic, legality or collision
              simulation.
            </small>
          )}
          {modelError && (
            <small role="alert">
              3D renderer unavailable. Illustrated vehicles remain usable.
            </small>
          )}
          <button
            aria-pressed={follow}
            disabled={!activeSample?.position}
            onClick={() => {
              lastFollow.current = 0;
              cinematicUntil.current = 0;
              followTarget.current = "";
              setFollow((v) => !v);
            }}
          >
            {follow ? "Stop following" : "Follow selected rider"}
          </button>
          {(perspective || immersive) && (
            <button
              onClick={() => {
                setFollow(false);
                const m = map.current;
                if (m)
                  m.easeTo({
                    bearing: m.getBearing() + 45,
                    pitch: immersive ? 65 : 55,
                    duration: reduced ? 0 : 600,
                  });
              }}
            >
              Orbit selected view
            </button>
          )}
          <button onClick={reveal.replay} disabled={!activePath}>
            Reveal active route
          </button>
          {perspective && (
            <label>
              <input
                type="checkbox"
                checked={buildings}
                onChange={(e) => setBuildings(e.target.checked)}
              />{" "}
              Building context
            </label>
          )}
          <small>
            {follow
              ? `Following rider ${selected}. Drag/zoom to exit.`
              : "Free camera"}{" "}
            ·{" "}
            {immersive
              ? "Procedural 3D models · flat surface"
              : perspective
                ? "Camera-relative illustration"
                : "Overhead scooter"}
            <br />
            {perspective && buildings
              ? "Provider-derived heights where available; no terrain or verified surface placement."
              : "Flat road context; buildings optional."}
          </small>
        </div>
      </details>
      <div className="delivery-map-label">
        <span className="status-dot" /> SYNTHETIC EXECUTION{" "}
        <small>
          {immersive ? "3D" : perspective ? "2.5D" : "2D"} · estimated motion ·
          no GPS feed
        </small>
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
