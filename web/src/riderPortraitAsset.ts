import { riderVisual } from "./riderVisual";
const cache = new Map<string, string>();
function portraitMarkup(id: string) {
  const v = riderVisual(id);
  return `      <ellipse cx="40" cy="76" rx="32" ry="7" fill="#000" opacity=".18" />
      <path d="M9 81Q9 59 31 57H49Q71 59 71 81Z" fill="${v.jacket}" />
      <path d="M27 64L34 81H46L53 64" fill="${v.body}" />
      <path d="M31 48H49V64Q40 71 31 64Z" fill="#c99777" />
      <path
        d="M16 35Q15 8 40 6Q65 8 64 35L60 54Q53 65 40 66Q27 65 20 54Z"
        fill="${v.body}"
        stroke="${v.light}"
        stroke-width="1.2"
      />
      <path d="M22 37Q40 29 58 37L56 51Q40 62 24 51Z" fill="#dbb197" />
      <path d="M20 29Q40 19 60 29L58 42Q40 36 22 42Z" fill="#262729" />
      <path
        d="M25 30Q40 25 55 30"
        fill="none"
        stroke="#ffffff"
        stroke-opacity=".2"
        stroke-width="2"
      />
      <path
        d="M20 45L27 60Q40 68 53 60L60 45L55 61Q40 76 25 61Z"
        fill="${v.dark}"
      />
      <path d="M37 11H43V24H37Z" fill="${v.helmet}" opacity=".85" />
      <path
        d="M25 17Q29 13 33 13"
        stroke="${v.light}"
        stroke-width="3"
        stroke-linecap="round"
      />
      <path
        d="M34 53Q40 56 46 53"
        stroke="#9a6652"
        stroke-width="1.5"
        fill="none"
      />`;
}
export function portraitAsset(id: string) {
  const key = `portrait:${id}`;
  let asset = cache.get(key);
  if (!asset) {
    asset = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 82">${portraitMarkup(id)}</svg>`)}`;
    cache.set(key, asset);
  }
  return asset;
}
export function riderTagAsset(id: string) {
  const key = `tag:${id}`;
  let asset = cache.get(key);
  if (!asset) {
    const v = riderVisual(id);
    asset = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 36 44"><path d="M18 42L6 27A15 15 0 1 1 30 27Z" fill="#252727" stroke="${v.body}" stroke-width="2"/><svg x="4" y="2" width="28" height="30" viewBox="0 0 80 82">${portraitMarkup(id)}</svg></svg>`)}`;
    cache.set(key, asset);
  }
  return asset;
}
