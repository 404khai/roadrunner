import { useEffect, useRef, useState, useMemo } from "react";
import * as maplibregl from "maplibre-gl";
import workerUrl from "maplibre-gl/dist/maplibre-gl-worker.mjs?url";
maplibregl.setWorkerUrl(workerUrl);
import type {
  GeoJSONSource,
  Map as LibreMap,
  StyleSpecification,
} from "maplibre-gl";
import {
  Plus,
  Minus,
  LocateFixed,
  Compass,
  Map as MapIcon,
  Plug,
} from "lucide-react";
import type { FeatureCollection } from "geojson";
import type {
  Anchor,
  Catalog,
  RouteResult,
  AlternativeMember,
} from "./contracts";
import { presentationPath, revealPrefix, type Position } from "./geometry";
const empty: FeatureCollection = { type: "FeatureCollection", features: [] };
const style: StyleSpecification = {
  version: 8,
  glyphs: "https://tiles.openfreemap.org/fonts/{fontstack}/{range}.pbf",
  sources: {
    basemap: {
      type: "vector",
      url: "https://tiles.openfreemap.org/planet",
      attribution:
        '<a href="https://openfreemap.org/">OpenFreeMap</a> © <a href="https://www.openmaptiles.org/">OpenMapTiles</a> © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
    },
  },
  layers: [
    {
      id: "background",
      type: "background",
      paint: { "background-color": "#191919" },
    },
    {
      id: "landuse",
      type: "fill",
      source: "basemap",
      "source-layer": "landuse",
      paint: { "fill-color": "#202020" },
    },
    {
      id: "park",
      type: "fill",
      source: "basemap",
      "source-layer": "park",
      paint: { "fill-color": "#222222" },
    },
    {
      id: "water",
      type: "fill",
      source: "basemap",
      "source-layer": "water",
      paint: { "fill-color": "#0d0d0d" },
    },
    {
      id: "buildings",
      type: "fill",
      source: "basemap",
      "source-layer": "building",
      minzoom: 14,
      paint: { "fill-color": "#242424", "fill-outline-color": "#2e2e2e" },
    },
    {
      id: "roads-casing",
      type: "line",
      source: "basemap",
      "source-layer": "transportation",
      filter: ["!=", ["get", "class"], "rail"],
      layout: { "line-join": "round", "line-cap": "round" },
      paint: {
        "line-color": "#111111",
        "line-width": [
          "interpolate",
          ["linear"],
          ["zoom"],
          10,
          1,
          14,
          5,
          18,
          19,
        ],
      },
    },
    {
      id: "roads",
      type: "line",
      source: "basemap",
      "source-layer": "transportation",
      filter: ["!=", ["get", "class"], "rail"],
      layout: { "line-join": "round", "line-cap": "round" },
      paint: {
        "line-color": [
          "match",
          ["get", "class"],
          ["motorway", "trunk", "primary"],
          "#555555",
          ["secondary", "tertiary"],
          "#484848",
          "#363636",
        ],
        "line-width": [
          "interpolate",
          ["linear"],
          ["zoom"],
          10,
          0.5,
          14,
          2.5,
          18,
          14,
        ],
      },
    },
    {
      id: "road-labels",
      type: "symbol",
      source: "basemap",
      "source-layer": "transportation_name",
      minzoom: 14,
      layout: {
        "symbol-placement": "line",
        "text-field": ["coalesce", ["get", "name:en"], ["get", "name"]],
        "text-font": ["Noto Sans Regular"],
        "text-size": 11,
        "text-max-angle": 30,
        "symbol-spacing": 320,
      },
      paint: {
        "text-color": "#8e8e8e",
        "text-halo-color": "#191919",
        "text-halo-width": 2,
      },
    },
    {
      id: "place-labels",
      type: "symbol",
      source: "basemap",
      "source-layer": "place",
      maxzoom: 15,
      layout: {
        "text-field": ["coalesce", ["get", "name:en"], ["get", "name"]],
        "text-font": ["Noto Sans Regular"],
        "text-size": ["interpolate", ["linear"], ["zoom"], 6, 10, 14, 15],
        "text-transform": "uppercase",
        "text-letter-spacing": 0.12,
      },
      paint: {
        "text-color": "#9c9c9c",
        "text-halo-color": "#191919",
        "text-halo-width": 2,
      },
    },
  ],
};
function line(points: Position[]): FeatureCollection {
  return points.length < 2
    ? empty
    : {
        type: "FeatureCollection",
        features: [
          {
            type: "Feature",
            properties: {},
            geometry: { type: "LineString", coordinates: points },
          },
        ],
      };
}
function source(map: LibreMap, id: string, data: FeatureCollection) {
  (map.getSource(id) as GeoJSONSource | undefined)?.setData(data);
}
export function MapView({
  catalog,
  panelOpen,
  connected,
  onConnect,
  origin,
  destination,
  result,
  progress,
  reduced,
  onSelect,
  selection,
  alternatives,
  visible,
  selectedMember,
  onSelectMember,
  segment,
  onSelectSegment,
}: {
  catalog?: Catalog;
  panelOpen: boolean;
  connected: boolean;
  onConnect: () => void;
  origin?: Anchor;
  destination?: Anchor;
  result?: RouteResult;
  alternatives?: AlternativeMember[];
  visible: Set<string>;
  selectedMember: string;
  onSelectMember: (id: string) => void;
  segment?: number;
  onSelectSegment: (ordinal: number) => void;
  progress: number;
  reduced: boolean;
  onSelect: (node: number) => void;
  selection: "origin" | "destination";
}) {
  const container = useRef<HTMLDivElement>(null),
    mapRef = useRef<LibreMap | null>(null),
    selectRef = useRef(onSelect);
  selectRef.current = onSelect;
  const memberRef = useRef(onSelectMember),
    segmentRef = useRef(onSelectSegment);
  memberRef.current = onSelectMember;
  segmentRef.current = onSelectSegment;
  const comparisonPaths = useMemo(
    () =>
      alternatives
        ?.filter((m) => visible.has(m.member_id))
        .map((m) => ({ member: m, path: presentationPath(m.route) })),
    [alternatives, visible],
  );
  const [ready, setReady] = useState(false),
    [failed, setFailed] = useState(false),
    [tilesFailed, setTilesFailed] = useState(false);
  const path = useMemo(
    () => (result ? presentationPath(result.evidence) : undefined),
    [result],
  );
  const fit = () => {
    const map = mapRef.current;
    if (!map) return;
    const points =
      (comparisonPaths?.length
        ? comparisonPaths.flatMap((p) => p.path.points)
        : path?.points) ??
      catalog?.anchors.map(
        (a) => [a.coordinate.longitude, a.coordinate.latitude] as Position,
      );
    if (!points?.length) return;
    const bounds = new maplibregl.LngLatBounds(points[0], points[0]);
    points.forEach((p) => bounds.extend(p));
    const { clientWidth: width, clientHeight: height } = map.getContainer();
    if (!width || !height) return;
    const padding = {
      top:
        path && width <= 760
          ? 280
          : path && !panelOpen && width <= 1050
            ? 340
            : 190,
      bottom: path ? 180 : 90,
      left: panelOpen && width > 760 ? 415 : 55,
      right: 65,
    };
    // A resize or virtual keyboard may temporarily leave less room than overlays.
    // Keep a real fitting area instead of asking MapLibre to fit negative space.
    const horizontal = Math.min(
      1,
      Math.max(1, width - 80) / (padding.left + padding.right),
    );
    const vertical = Math.min(
      1,
      Math.max(1, height - 80) / (padding.top + padding.bottom),
    );
    padding.left *= horizontal;
    padding.right *= horizontal;
    padding.top *= vertical;
    padding.bottom *= vertical;
    map.fitBounds(bounds, {
      padding,
      maxZoom: 17,
      duration: reduced ? 0 : 900,
    });
  };
  const fitRef = useRef(fit);
  fitRef.current = fit;
  useEffect(() => {
    if (!container.current) return;
    setReady(false);
    setFailed(false);
    setTilesFailed(false);
    let map: LibreMap;
    try {
      map = new maplibregl.Map({
        container: container.current,
        style,
        center: [3.38, 6.524],
        zoom: 14,
        attributionControl: { compact: true },
        maxPitch: 0,
        renderWorldCopies: false,
        canvasContextAttributes: { preserveDrawingBuffer: true },
      });
    } catch {
      setFailed(true);
      return;
    }
    mapRef.current = map;
    map.on("style.load", () => {
      for (const id of [
        "anchors",
        "alternatives",
        "alternative-labels",
        "selected-segments",
        "inspected",
        "full-route",
        "reveal",
        "head",
        "endpoints",
      ])
        map.addSource(id, { type: "geojson", data: empty });
      map.addLayer({
        id: "anchors",
        type: "circle",
        source: "anchors",
        paint: {
          "circle-radius": 4,
          "circle-color": "#858585",
          "circle-stroke-width": 1.5,
          "circle-stroke-color": "#1b1b1b",
        },
      });
      for (let rank = 1; rank <= 5; rank++) {
        map.addLayer({
          id: `alternative-${rank}`,
          type: "line",
          source: "alternatives",
          filter: ["==", ["get", "rank"], rank],
          layout: { "line-join": "round", "line-cap": "round" },
          paint: {
            "line-color": rank % 2 === 0 ? "#c0c0c0" : "#8f8f8f",
            "line-width": 5,
            "line-dasharray": rank % 2 === 0 ? [2, 1.5] : [0.6, 1.4],
          },
        });
        map.on("mouseenter", `alternative-${rank}`, () => {
          map.getCanvas().style.cursor = "pointer";
        });
        map.on("mouseleave", `alternative-${rank}`, () => {
          map.getCanvas().style.cursor = "";
        });
      }
      map.addLayer({
        id: "route-halo",
        type: "line",
        source: "full-route",
        layout: { "line-join": "round", "line-cap": "round" },
        paint: { "line-color": "#080808", "line-width": 12 },
      });
      map.addLayer({
        id: "route-underlay",
        type: "line",
        source: "full-route",
        layout: { "line-join": "round", "line-cap": "round" },
        paint: { "line-color": "#5c5c5c", "line-width": 6 },
      });
      map.addLayer({
        id: "selected-route",
        type: "line",
        source: "reveal",
        layout: { "line-join": "round", "line-cap": "round" },
        paint: { "line-color": "#ffffff", "line-width": 6 },
      });
      map.addLayer({
        id: "segment-hit",
        type: "line",
        source: "selected-segments",
        paint: { "line-width": 14, "line-opacity": 0 },
      });
      map.on("mouseenter", "segment-hit", () => {
        map.getCanvas().style.cursor = "pointer";
      });
      map.on("mouseleave", "segment-hit", () => {
        map.getCanvas().style.cursor = "";
      });
      map.addLayer({
        id: "inspected-halo",
        type: "line",
        source: "inspected",
        layout: { "line-cap": "round" },
        paint: { "line-width": 12, "line-color": "#fff" },
      });
      map.addLayer({
        id: "inspected-segment",
        type: "line",
        source: "inspected",
        paint: {
          "line-width": 5,
          "line-color": "#111",
          "line-dasharray": [1, 1],
        },
      });
      map.addLayer({
        id: "head-halo",
        type: "circle",
        source: "head",
        paint: {
          "circle-color": "#ffffff",
          "circle-radius": 17,
          "circle-opacity": 0.12,
        },
      });
      map.addLayer({
        id: "head",
        type: "circle",
        source: "head",
        paint: {
          "circle-color": "#fff",
          "circle-radius": 7,
          "circle-stroke-color": "#111",
          "circle-stroke-width": 2,
        },
      });
      map.addLayer({
        id: "alternative-labels",
        type: "symbol",
        source: "alternative-labels",
        layout: {
          "symbol-placement": "line",
          "symbol-spacing": 800,
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Regular"],
          "text-size": 11,
          "text-offset": [0, 1.2],
        },
        paint: {
          "text-color": "#ddd",
          "text-halo-color": "#111",
          "text-halo-width": 2,
        },
      });
      map.addLayer({
        id: "endpoint-outline",
        type: "circle",
        source: "endpoints",
        paint: {
          "circle-radius": 12,
          "circle-color": "#111",
          "circle-stroke-color": "#fff",
          "circle-stroke-width": 2,
        },
      });
      map.addLayer({
        id: "endpoint-label",
        type: "symbol",
        source: "endpoints",
        layout: {
          "text-field": ["get", "label"],
          "text-font": ["Noto Sans Regular"],
          "text-size": 11,
          "text-allow-overlap": true,
        },
        paint: { "text-color": "#fff" },
      });
      // Resolve a single topmost hit before React can update any layer sources.
      // Separate delegated click handlers can otherwise select a second member on overlap.
      map.on("click", (e) => {
        const hit = map.queryRenderedFeatures(e.point, {
          layers: [
            "segment-hit",
            ...Array.from({ length: 5 }, (_, i) => `alternative-${i + 1}`),
            "anchors",
          ],
        })[0];
        if (!hit) return;
        if (
          hit.layer.id === "segment-hit" &&
          typeof hit.properties.ordinal === "number"
        )
          segmentRef.current(hit.properties.ordinal);
        else if (
          hit.layer.id.startsWith("alternative-") &&
          typeof hit.properties.member === "string"
        )
          memberRef.current(hit.properties.member);
        else if (
          hit.layer.id === "anchors" &&
          typeof hit.properties.node === "number"
        )
          selectRef.current(hit.properties.node);
      });
      map.on("mouseenter", "anchors", () => {
        map.getCanvas().style.cursor = "pointer";
      });
      map.on("mouseleave", "anchors", () => {
        map.getCanvas().style.cursor = "";
      });
      setReady(true);
    });
    map.on("error", (e) => {
      if ("sourceId" in e && e.sourceId === "basemap") setTilesFailed(true);
      else if (String(e.error).includes("Worker")) setFailed(true);
    });
    const canvas = map.getCanvas();
    const lost = (e: Event) => {
      e.preventDefault();
      setFailed(true);
    };
    canvas.addEventListener("webglcontextlost", lost);
    const resize = new ResizeObserver(() => {
      map.setPadding({ top: 0, bottom: 0, left: 0, right: 0 });
      map.resize();
      fitRef.current();
    });
    resize.observe(container.current);
    return () => {
      resize.disconnect();
      canvas.removeEventListener("webglcontextlost", lost);
      map.remove();
      mapRef.current = null;
    };
  }, []);
  useEffect(() => {
    if (!ready || !catalog || !mapRef.current) return;
    source(mapRef.current, "anchors", {
      type: "FeatureCollection",
      features: catalog.anchors.map((a) => ({
        type: "Feature",
        properties: { node: a.node },
        geometry: {
          type: "Point",
          coordinates: [a.coordinate.longitude, a.coordinate.latitude],
        },
      })),
    });
    fitRef.current();
  }, [ready, catalog]);
  useEffect(() => {
    if (!ready || !mapRef.current) return;
    source(mapRef.current, "endpoints", {
      type: "FeatureCollection",
      features: (
        [
          ["origin", origin],
          ["destination", destination],
        ] as const
      ).flatMap(([kind, a]) =>
        a && !(kind === "destination" && origin?.node === destination?.node)
          ? [
              {
                type: "Feature" as const,
                properties: {
                  kind,
                  label:
                    origin?.node === destination?.node
                      ? "A=B"
                      : kind === "origin"
                        ? "A"
                        : "B",
                },
                geometry: {
                  type: "Point" as const,
                  coordinates: [a.coordinate.longitude, a.coordinate.latitude],
                },
              },
            ]
          : [],
      ),
    });
  }, [ready, origin, destination]);
  useEffect(() => {
    if (!ready || !mapRef.current) return;
    source(
      mapRef.current,
      "full-route",
      path && !result?.evidence.geometry.no_travel ? line(path.points) : empty,
    );
    if (path) fitRef.current();
  }, [ready, path, result, panelOpen, comparisonPaths]);
  useEffect(() => {
    if (!ready || !mapRef.current) return;
    const map = mapRef.current;
    source(map, "alternatives", {
      type: "FeatureCollection",
      features: (comparisonPaths ?? [])
        .filter(
          (p) =>
            p.member.member_id !== selectedMember && p.path.points.length >= 2,
        )
        .map((p) => ({
          type: "Feature",
          properties: {
            member: p.member.member_id,
            rank: p.member.rank,
            label: `Route ${String.fromCharCode(64 + p.member.rank)}`,
          },
          geometry: { type: "LineString", coordinates: p.path.points },
        })),
    });
    // Label only a physical segment exclusive to this visible route. A shared
    // prefix must not put an alternative's label on the selected white line.
    const usage = new Map<number, number>();
    for (const p of comparisonPaths ?? [])
      for (const id of new Set(
        p.member.route.geometry.occurrences.map((o) => o.segment),
      ))
        usage.set(id, (usage.get(id) ?? 0) + 1);
    const labels: FeatureCollection = {
      type: "FeatureCollection",
      features: [],
    };
    for (const p of comparisonPaths ?? []) {
      if (p.member.member_id === selectedMember) continue;
      const geometry = p.member.route.geometry;
      const exclusive = geometry.occurrences
        .filter((o) => usage.get(o.segment) === 1)
        .reduce<(typeof geometry.occurrences)[number] | undefined>(
          (best, o) =>
            !best || o.distance_meters > best.distance_meters ? o : best,
          undefined,
        );
      if (exclusive)
        labels.features.push({
          type: "Feature",
          properties: {
            label: `Route ${String.fromCharCode(64 + p.member.rank)}`,
          },
          geometry: {
            type: "LineString",
            coordinates: geometry.points_e7
              .slice(exclusive.point_start, exclusive.point_end)
              .map((p) => [p.longitude_e7 / 1e7, p.latitude_e7 / 1e7]),
          },
        });
    }
    source(map, "alternative-labels", labels);
    const g = result?.evidence.geometry;
    source(map, "selected-segments", {
      type: "FeatureCollection",
      features:
        !alternatives || !g
          ? []
          : g.occurrences.map((o) => ({
              type: "Feature",
              properties: { ordinal: o.ordinal },
              geometry: {
                type: "LineString",
                coordinates: g.points_e7
                  .slice(o.point_start, o.point_end)
                  .map((p) => [p.longitude_e7 / 1e7, p.latitude_e7 / 1e7]),
              },
            })),
    });
    const o = segment === undefined ? undefined : g?.occurrences[segment];
    source(
      map,
      "inspected",
      o && g
        ? line(
            g.points_e7
              .slice(o.point_start, o.point_end)
              .map((p) => [p.longitude_e7 / 1e7, p.latitude_e7 / 1e7]),
          )
        : empty,
    );
  }, [ready, comparisonPaths, selectedMember, result, segment, alternatives]);
  useEffect(() => {
    if (!ready || !mapRef.current) return;
    const map = mapRef.current;
    if (!path || result?.evidence.geometry.no_travel) {
      source(map, "reveal", empty);
      source(map, "head", empty);
      return;
    }
    const frame = revealPrefix(path, progress);
    source(map, "reveal", line(frame.line));
    source(
      map,
      "head",
      progress > 0 && progress < 1
        ? {
            type: "FeatureCollection",
            features: [
              {
                type: "Feature",
                properties: {},
                geometry: { type: "Point", coordinates: frame.head },
              },
            ],
          }
        : empty,
    );
  }, [ready, path, progress, result]);
  return (
    <section
      className="map-area"
      aria-label="Interactive route map"
      data-map-ready={ready}
      data-reveal-progress={progress.toFixed(3)}
      data-selected-evaluation={result?.evidence.evaluation_id}
      data-visible-route-count={comparisonPaths?.length}
      data-inspected-ordinal={segment}
    >
      <div ref={container} className="map-canvas" />
      <div className="map-heading">
        <span className="map-tag">
          <span className="status-dot" /> CANONICAL OVERLAY
        </span>
        <span className="map-tag muted-tag">2D / WGS84</span>
      </div>
      {!result && (
        <div className="map-instruction">
          <MapIcon size={16} />
          <span>
            {catalog
              ? `Choose ${selection === "origin" ? "an origin" : "a destination"} from the graph nodes`
              : "Connect to Roadrunner to explore a road network"}
            <small>
              {catalog
                ? "Click a visible point or use the node selectors."
                : "Your graph supplies the route. The basemap supplies context."}
            </small>
          </span>
        </div>
      )}
      <div className="map-tools">
        <button
          className="engine-control"
          aria-label={
            connected
              ? "Engine connected: connection settings"
              : "Connect your engine"
          }
          title={
            connected
              ? "Engine connected — connection settings"
              : "Connect your engine"
          }
          onClick={onConnect}
        >
          <Plug size={19} />
          <span className={`engine-status ${connected ? "connected" : ""}`} />
        </button>
        <button
          aria-label="Fit route or loaded nodes"
          onClick={() => fitRef.current()}
        >
          <LocateFixed size={19} />
        </button>
        <div className="zoom-group">
          <button
            aria-label="Zoom in"
            onClick={() =>
              mapRef.current?.zoomIn({ duration: reduced ? 0 : 250 })
            }
          >
            <Plus size={19} />
          </button>
          <button
            aria-label="Zoom out"
            onClick={() =>
              mapRef.current?.zoomOut({ duration: reduced ? 0 : 250 })
            }
          >
            <Minus size={19} />
          </button>
        </div>
        <button
          aria-label="Reset north"
          onClick={() =>
            mapRef.current?.resetNorth({ duration: reduced ? 0 : 250 })
          }
        >
          <Compass size={20} />
          <small>N</small>
        </button>
      </div>
      <div className="map-caption">
        {catalog ? (
          <>
            <span>{catalog.graph.build_identity.source_dataset}</span>
            <span>
              {alternatives
                ? `White: Route ${String.fromCharCode(64 + (alternatives.find((m) => m.member_id === selectedMember)?.rank ?? 1))} · dashed: alternatives`
                : "Graph-qualified road geometry"}
            </span>
          </>
        ) : (
          <>
            <span>ROADRUNNER</span>
            <span>Routing engine workspace</span>
          </>
        )}
      </div>
      {failed && (
        <div className="map-error" role="alert">
          Map rendering unavailable. Enable WebGL or reload this page. Route
          metrics and source evidence remain available.
        </div>
      )}
      {!failed && tilesFailed && (
        <div className="tile-error" role="status">
          Basemap content unavailable · canonical route overlay remains
          available
        </div>
      )}
    </section>
  );
}
