import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import * as ml from "maplibre-gl";
import Radar from "./vendor/reactbits/Radar";
import { radiusRing } from "./dispatchEvidence";

/** Decorative seeking scope; geographic eligibility remains the engine's boundary. */
export function DispatchRadar({
  map,
  longitude,
  latitude,
  radius,
  paused,
  progress,
  onSweep,
}: {
  map: ml.Map;
  longitude: number;
  latitude: number;
  radius: number | null;
  paused: boolean;
  progress: number;
  onSweep: (from: number, to: number, bounds: DOMRect) => void;
}) {
  const host = useRef<HTMLDivElement | null>(null);
  if (!host.current) {
    host.current = document.createElement("div");
    host.current.className = "dispatch-radar";
    host.current.setAttribute("aria-hidden", "true");
  }
  useEffect(() => {
    const center: [number, number] = [longitude, latitude];
    const element = host.current;
    if (!element) return;
    const marker = new ml.Marker({ element, anchor: "center" })
      .setLngLat(center)
      .addTo(map);
    const resize = () => {
      // Bounded presentation size, not a claim of radar discovery or exact coverage.
      const p = map.project(center);
      const ring = radiusRing(center, radius ?? 500);
      const extent = Math.max(
        ...ring.map((point) => {
          const q = map.project(point);
          return Math.hypot(q.x - p.x, q.y - p.y);
        }),
      );
      const diameter = Math.min(1200, Math.max(160, extent * 2));
      element.style.width = `${diameter}px`;
      element.style.height = `${diameter}px`;
    };
    resize();
    map.on("move", resize);
    map.on("resize", resize);
    return () => {
      map.off("move", resize);
      map.off("resize", resize);
      marker.remove();
    };
  }, [map, longitude, latitude, radius]);
  return createPortal(
    <Radar
      color="#3ef09a"
      backgroundColor="transparent"
      targets={0}
      clutter={0}
      mouseInteraction={false}
      intro={false}
      speed={2.5}
      trail={0.5}
      ringCount={4}
      spokeCount={12}
      grain={0}
      glow={0.7}
      gridOpacity={0.85}
      scale={0.94}
      paused={paused}
      sweepProgress={Math.min(1, progress / 0.75)}
      onSweep={(from, to) => {
        const bounds = host.current?.getBoundingClientRect();
        if (bounds) onSweep(from, to, bounds);
      }}
      dpr={1}
    />,
    host.current,
  );
}
