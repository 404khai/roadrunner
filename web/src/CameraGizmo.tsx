import { useRef } from "react";
export function projectAxis(
  x: number,
  y: number,
  z: number,
  bearing: number,
  pitch: number,
) {
  const b = (bearing * Math.PI) / 180,
    p = (pitch * Math.PI) / 180;
  return [
    42 + 28 * (x * Math.cos(b) - y * Math.sin(b)),
    42 -
      28 *
        ((x * Math.sin(b) + y * Math.cos(b)) * Math.cos(p) + z * Math.sin(p)),
  ];
}
export function CameraGizmo({
  bearing,
  pitch,
  onOrbit,
  onPreset,
  onRear,
  onFront,
}: {
  bearing: number;
  pitch: number;
  onOrbit: (bearing: number, pitch: number) => void;
  onPreset: (axis: "X" | "Y" | "Z") => void;
  onRear: () => void;
  onFront: () => void;
}) {
  const drag = useRef<{ x: number; y: number; b: number; p: number } | null>(
    null,
  );
  return (
    <div className="camera-gizmo" aria-label="Rider camera orientation">
      <svg
        viewBox="0 0 84 84"
        role="group"
        aria-label="Drag to orbit rider"
        onPointerDown={(e) => {
          if ((e.target as Element).closest('[role="button"]')) return;
          drag.current = { x: e.clientX, y: e.clientY, b: bearing, p: pitch };
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (d)
            onOrbit(
              d.b + (e.clientX - d.x) * 0.8,
              Math.max(0, Math.min(70, d.p - (e.clientY - d.y) * 0.5)),
            );
        }}
        onPointerUp={() => {
          drag.current = null;
        }}
        onPointerCancel={() => {
          drag.current = null;
        }}
      >
        <circle cx="42" cy="42" r="31" fill="none" stroke="#ffffff15" />
        {(
          [
            ["X", 1, 0, 0, "#dc8177"],
            ["Y", 0, 1, 0, "#8cafe0"],
            ["Z", 0, 0, 1, "#a6c67f"],
          ] as const
        ).map(([axis, x, y, z, color]) => {
          const [cx, cy] = projectAxis(x, y, z, bearing, pitch);
          return (
            <g
              key={axis}
              role="button"
              tabIndex={0}
              aria-label={`View rider from ${axis} axis`}
              onClick={() => onPreset(axis)}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  onPreset(axis);
                }
              }}
            >
              <line
                x1="42"
                y1="42"
                x2={cx}
                y2={cy}
                stroke={color}
                strokeWidth="2"
              />
              <circle cx={cx} cy={cy} r="9" fill={color} />
              <text
                x={cx}
                y={cy + 3}
                textAnchor="middle"
                fill="#191919"
                fontSize="9"
                fontWeight="700"
              >
                {axis}
              </text>
            </g>
          );
        })}
        <circle cx="42" cy="42" r="3" fill="#d8d8ce" />
      </svg>
      <button onClick={onRear} title="Return behind selected rider and follow">
        Rear
      </button>
      <button onClick={onFront} title="Look at selected rider from the front">
        Front
      </button>
    </div>
  );
}
