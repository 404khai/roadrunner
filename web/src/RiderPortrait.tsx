import { riderVisual } from "./riderVisual";
export function RiderPortrait({
  id,
  visualId,
}: {
  id: string;
  visualId?: string;
}) {
  const v = riderVisual(visualId ?? id);
  return (
    <svg
      className="rider-portrait"
      viewBox="0 0 80 82"
      role="img"
      aria-label={`Rider ${id} helmet portrait · ${v.name}`}
    >
      <ellipse cx="40" cy="76" rx="32" ry="7" fill="#000" opacity=".18" />
      <path d="M9 81Q9 59 31 57H49Q71 59 71 81Z" fill={v.jacket} />
      <path d="M27 64L34 81H46L53 64" fill={v.body} />
      <path d="M31 48H49V64Q40 71 31 64Z" fill="#c99777" />
      <path
        d="M16 35Q15 8 40 6Q65 8 64 35L60 54Q53 65 40 66Q27 65 20 54Z"
        fill={v.body}
        stroke={v.light}
        strokeWidth="1.2"
      />
      <path d="M22 37Q40 29 58 37L56 51Q40 62 24 51Z" fill="#dbb197" />
      <path d="M20 29Q40 19 60 29L58 42Q40 36 22 42Z" fill="#262729" />
      <path
        d="M25 30Q40 25 55 30"
        fill="none"
        stroke="#ffffff"
        strokeOpacity=".2"
        strokeWidth="2"
      />
      <path
        d="M20 45L27 60Q40 68 53 60L60 45L55 61Q40 76 25 61Z"
        fill={v.dark}
      />
      <path d="M37 11H43V24H37Z" fill={v.helmet} opacity=".85" />
      <path
        d="M25 17Q29 13 33 13"
        stroke={v.light}
        strokeWidth="3"
        strokeLinecap="round"
      />
      <path
        d="M34 53Q40 56 46 53"
        stroke="#9a6652"
        strokeWidth="1.5"
        fill="none"
      />
    </svg>
  );
}
