import { useEffect, useMemo, useState } from "react";
import type { Map, GeoJSONSource } from "maplibre-gl";
import { h3LocationOverlay } from "./h3Overlay";

/** Renderer-only scene cells; controls never change dispatch or simulation state. */
export function H3MapOverlay({
  map,
  locations,
  beforeLayer,
  scope,
}: {
  map: Map | null;
  locations: readonly (readonly [number, number])[];
  beforeLayer: string;
  scope: "dispatch" | "delivery";
}) {
  const [enabled, setEnabled] = useState(false);
  const [resolution, setResolution] = useState(11);
  const overlay = useMemo(
    () => h3LocationOverlay(enabled ? locations : [], resolution),
    [enabled, locations, resolution],
  );
  const sourceId = `${scope}-h3-cells`;
  useEffect(() => {
    if (!map) return;
    if (!map.getSource(sourceId)) {
      map.addSource(sourceId, {
        type: "geojson",
        data: { type: "FeatureCollection", features: [] },
      });
      map.addLayer(
        {
          id: `${sourceId}-fill`,
          type: "fill",
          source: sourceId,
          paint: { "fill-color": "#c5c5c5", "fill-opacity": 0.045 },
        },
        beforeLayer,
      );
      map.addLayer(
        {
          id: `${sourceId}-outline`,
          type: "line",
          source: sourceId,
          paint: {
            "line-color": "#c3c3bb",
            "line-width": 1.2,
            "line-opacity": 0.7,
          },
        },
        beforeLayer,
      );
    }
    (map.getSource(sourceId) as GeoJSONSource).setData(overlay.geojson);
    // The owning map disposes these sources and layers when the view is unmounted.
  }, [map, sourceId, beforeLayer, overlay]);
  return (
    <div className="h3-map-controls">
      <label>
        <input
          type="checkbox"
          checked={enabled}
          onChange={(e) => setEnabled(e.target.checked)}
        />{" "}
        H3 hexagons
      </label>
      {enabled && (
        <>
          <label>
            Cell resolution{" "}
            <select
              aria-label="H3 cell resolution"
              value={resolution}
              onChange={(e) => setResolution(Number(e.target.value))}
            >
              {[7, 8, 9, 10, 11, 12].map((value) => (
                <option key={value} value={value}>
                  {value}
                </option>
              ))}
            </select>
          </label>
          <small>
            {overlay.cells.length} occupied location cells ·{" "}
            {scope === "delivery"
              ? "Captured riders, active stops and route vertices"
              : "Riders, stops and inspected-route vertices"}
            . Geographic samples; not traffic or dispatch eligibility.
          </small>
          {(overlay.truncated || overlay.unsupported > 0) && (
            <small role="status">
              At most 1,000 polygons; {overlay.unsupported} polar/seam cells
              withheld.
            </small>
          )}
          {!overlay.cells.length && (
            <small>No scene locations available.</small>
          )}
        </>
      )}
    </div>
  );
}
