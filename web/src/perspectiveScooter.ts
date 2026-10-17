import { riderVisual } from "./riderVisual";
export function relativeDirection(heading: number, cameraBearing: number) {
  return Math.round(((((heading - cameraBearing) % 360) + 360) % 360) / 45) % 8;
}
const cache = new Map<string, string>();
// Eight camera-relative projected illustrations. Upright billboards, not ground-flat overhead sprites.
export function perspectiveScooter(visualId: string, direction: number) {
  const v = riderVisual(visualId),
    key = `${v.name}:${direction}`;
  const existing = cache.get(key);
  if (existing) return existing;
  if (direction === 0) {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 96"><defs><linearGradient id="h" x2=".8" y2="1"><stop stop-color="${v.helmet}"/><stop offset="1" stop-color="${v.dark}"/></linearGradient><linearGradient id="b"><stop stop-color="${v.dark}"/><stop offset=".5" stop-color="${v.body}"/><stop offset="1" stop-color="${v.dark}"/></linearGradient></defs><ellipse cx="40" cy="90" rx="22" ry="5" fill="#000" opacity=".3"/><rect x="33" y="75" width="14" height="18" rx="6" fill="#232427"/><path d="M23 62Q19 71 24 80L29 83H51L56 80Q61 71 57 62Z" fill="url(#b)" stroke="${v.dark}"/><path d="M25 47L22 73M55 47L58 73" stroke="#303133" stroke-width="8" stroke-linecap="round"/><path d="M20 36L12 29M60 36L68 29" stroke="#444" stroke-width="2"/><ellipse cx="11" cy="27" rx="5" ry="6" fill="#b5b7b3" stroke="#333" stroke-width="2"/><ellipse cx="69" cy="27" rx="5" ry="6" fill="#b5b7b3" stroke="#333" stroke-width="2"/><path d="M17 47Q21 28 33 28H47Q59 28 63 47" fill="${v.jacket}" stroke="${v.dark}" stroke-width="2"/><path d="M19 44L16 51M61 44L64 51" stroke="${v.jacket}" stroke-width="7" stroke-linecap="round"/><rect x="26" y="3" width="28" height="30" rx="13" fill="url(#h)" stroke="#454544"/><path d="M29 8Q40 1 50 10" stroke="${v.helmet}" stroke-width="2" fill="none"/><rect x="23" y="41" width="34" height="29" rx="4" fill="#373937" stroke="#222"/><path d="M24 46H56" stroke="#8d9089" stroke-width="3"/><rect x="37" y="43" width="6" height="9" rx="1" fill="#292b29"/><rect x="32" y="73" width="16" height="7" rx="3" fill="#d64c45" stroke="#7c322d"/><path d="M28 72v6M52 72v6" stroke="#e4ddc9" stroke-width="3" stroke-linecap="round"/><rect x="37" y="84" width="6" height="4" rx="1" fill="#bc5148"/></svg>`;
    const asset = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
    cache.set(key, asset);
    return asset;
  }
  if (direction === 4) {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 96"><defs><linearGradient id="body"><stop stop-color="${v.dark}"/><stop offset=".5" stop-color="${v.body}"/><stop offset="1" stop-color="${v.dark}"/></linearGradient></defs><ellipse cx="40" cy="90" rx="22" ry="5" fill="#000" opacity=".3"/><rect x="23" y="20" width="34" height="31" rx="4" fill="#373a38"/><path d="M21 46Q23 29 34 28H46Q57 29 59 46" fill="${v.jacket}"/><rect x="27" y="3" width="26" height="30" rx="13" fill="${v.helmet}" stroke="${v.dark}"/><path d="M30 17Q40 13 50 17V24Q40 35 30 24Z" fill="#d6ad91" stroke="#303332" stroke-width="2"/><path d="M21 46L14 50M59 46L66 50" stroke="${v.jacket}" stroke-width="8" stroke-linecap="round"/><path d="M25 34L12 27M55 34L68 27" stroke="#656b67" stroke-width="2"/><ellipse cx="11" cy="25" rx="5" ry="6" fill="#363a38"/><ellipse cx="69" cy="25" rx="5" ry="6" fill="#363a38"/><path d="M18 48H62" stroke="#252729" stroke-width="4"/><path d="M23 57Q17 69 24 80M57 57Q63 69 56 80" stroke="#303336" stroke-width="8" stroke-linecap="round"/><path d="M24 57Q27 51 40 51Q53 51 56 57L58 79Q56 86 49 85H31Q24 86 22 79Z" fill="url(#body)" stroke="${v.dark}"/><path d="M27 48Q40 40 53 48" stroke="${v.body}" stroke-width="9" stroke-linecap="round"/><circle cx="40" cy="47" r="7" fill="#dfe4db" stroke="#555c58" stroke-width="2"/><rect x="33" y="77" width="14" height="16" rx="6" fill="#252729"/><path d="M30 80Q30 68 40 68Q50 68 50 80" fill="${v.body}" stroke="${v.dark}"/><path d="M27 61v8M53 61v8" stroke="#e4dfcb" stroke-width="3" stroke-linecap="round"/><path d="M37 61h6M37 64h6" stroke="${v.dark}" stroke-width="1.5"/></svg>`;
    const asset = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
    cache.set(key, asset);
    return asset;
  }
  const a = (direction * Math.PI) / 4,
    s = Math.sin(a),
    c = Math.cos(a);
  const p = (x: number, y: number, z: number) => [
    40 + x * c + y * s,
    80 + (x * s - y * c) * 0.42 - z,
  ];
  const pts = (items: number[][]) =>
    items
      .map(([x, y, z]) =>
        p(x, y, z)
          .map((n) => n.toFixed(2))
          .join(","),
      )
      .join(" ");
  const box = (
    x: number,
    y: number,
    z: number,
    w: number,
    l: number,
    h: number,
    color: string,
  ) => {
    const corners = [
      [-w / 2, -l / 2],
      [w / 2, -l / 2],
      [w / 2, l / 2],
      [-w / 2, l / 2],
    ].map(([dx, dy]) => [x + dx, y + dy]);
    const faces = corners
      .map(([cx, cy], i) => {
        const [nx, ny] = corners[(i + 1) % 4];
        return `<polygon points="${pts([
          [cx, cy, z],
          [nx, ny, z],
          [nx, ny, z + h],
          [cx, cy, z + h],
        ])}" fill="${color}" stroke="${v.dark}" stroke-width=".8"/>`;
      })
      .join("");
    return (
      faces +
      `<polygon points="${pts(corners.map(([cx, cy]) => [cx, cy, z + h]))}" fill="${v.light}" stroke="${v.dark}" stroke-width=".8"/>`
    );
  };
  const wheel = (y: number) => {
    const [x, py] = p(0, y, 7);
    return `<ellipse cx="${x}" cy="${py}" rx="${3 + Math.abs(s) * 3}" ry="8" fill="#222" stroke="#b7b7af" stroke-width="1.2"/>`;
  };
  const body = box(0, 0, 9, 13, 31, 9, v.body);
  const [hx, hy] = p(0, 1, 48),
    [bx, by] = p(0, -1, 28),
    [handx, handy] = p(0, 12, 25);
  const rider = `<path d="M${bx - 7},${by + 2}L${handx - 10},${handy}M${bx + 7},${by + 2}L${handx + 10},${handy}" stroke="${v.jacket}" stroke-width="6" stroke-linecap="round"/><ellipse cx="${bx}" cy="${by}" rx="9" ry="12" fill="${v.jacket}" stroke="${v.dark}"/><ellipse cx="${hx}" cy="${hy}" rx="10" ry="11" fill="${v.helmet}" stroke="${v.dark}"/><path d="M${hx - 6},${hy - 4}Q${hx},${hy - 8} ${hx + 6},${hy - 4}" fill="none" stroke="${v.light}" stroke-width="2"/>${c < -0.3 ? `<path d="M${hx - 8},${hy}Q${hx},${hy + 5} ${hx + 8},${hy}" stroke="#252829" stroke-width="5" fill="none"/>` : ""}`;
  const cargo = box(0, -12, 18, 19, 13, 15, v.dark);
  const front = box(0, 13, 16, 12, 9, 12, v.body);
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 96"><ellipse cx="40" cy="82" rx="24" ry="8" fill="#000" opacity=".28"/>${wheel(-16)}${wheel(16)}${body}${c > 0 ? front + rider + cargo : cargo + rider + front}</svg>`;
  const asset = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  cache.set(key, asset);
  return asset;
}
