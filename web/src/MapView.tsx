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
import type { Anchor, Catalog, RouteResult } from "./contracts";
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
}: {
  catalog?: Catalog;
  panelOpen: boolean;
  connected: boolean;
  onConnect: () => void;
  origin?: Anchor;
  destination?: Anchor;
  result?: RouteResult;
  progress: number;
  reduced: boolean;
  onSelect: (node: number) => void;
  selection: "origin" | "destination";
}) {
  const container = useRef<HTMLDivElement>(null),
    mapRef = useRef<LibreMap | null>(null),
    selectRef = useRef(onSelect);
  selectRef.current = onSelect;
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
      path?.points ??
      catalog?.anchors.map(
        (a) => [a.coordinate.longitude, a.coordinate.latitude] as Position,
      );
    if (!points?.length) return;
    const bounds = new maplibregl.LngLatBounds(points[0], points[0]);
    points.forEach((p) => bounds.extend(p));
    map.fitBounds(bounds, {
      padding: {
        top:
          path && map.getContainer().clientWidth <= 760
            ? 280
            : path && !panelOpen && map.getContainer().clientWidth <= 1050
              ? 340
              : 190,
        bottom: path ? 180 : 90,
        left: panelOpen && map.getContainer().clientWidth > 760 ? 415 : 55,
        right: 65,
      },
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
      for (const id of ["anchors", "full-route", "reveal", "head", "endpoints"])
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
      map.on("click", "anchors", (e) => {
        const id = e.features?.[0]?.properties?.node;
        if (typeof id === "number") selectRef.current(id);
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
  }, [ready, path, result, panelOpen]);
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
            <span>Graph-qualified road geometry</span>
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
