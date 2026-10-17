import scooterTemplate from "./assets/delivery-scooter.svg?raw";
// Cosmetic renderer identity. It does not imply vehicle capabilities or dispatch rank.
const presets = [
  {
    name: "Ochre / graphite",
    body: "#d8a64f",
    light: "#f4d594",
    dark: "#785224",
    helmet: "#e2d5bc",
    jacket: "#32302c",
  },
  {
    name: "Sage / ivory",
    body: "#79b7a6",
    light: "#c8e6da",
    dark: "#315f54",
    helmet: "#f2e9d7",
    jacket: "#364844",
  },
  {
    name: "Coral / charcoal",
    body: "#d9867b",
    light: "#f5c1ad",
    dark: "#81433f",
    helmet: "#dcd9d5",
    jacket: "#3e3030",
  },
  {
    name: "Lilac / slate",
    body: "#afa0d2",
    light: "#e1d7f6",
    dark: "#615078",
    helmet: "#d7cee9",
    jacket: "#37343f",
  },
  {
    name: "Olive / sand",
    body: "#a6b26e",
    light: "#dae2b7",
    dark: "#565f35",
    helmet: "#e6ddc4",
    jacket: "#3a3e2e",
  },
  {
    name: "Rose / pearl",
    body: "#cf9fb7",
    light: "#f1d4e1",
    dark: "#785066",
    helmet: "#efdfeb",
    jacket: "#44333d",
  },
  {
    name: "Copper / cream",
    body: "#bd8c63",
    light: "#e8c6a2",
    dark: "#69462c",
    helmet: "#f1e3d0",
    jacket: "#3f3329",
  },
  {
    name: "Stone / ink",
    body: "#b9b7af",
    light: "#ece9e1",
    dark: "#64635c",
    helmet: "#ddd8cc",
    jacket: "#292b2c",
  },
];
export function riderVisual(id: string) {
  const n = BigInt(id),
    base = presets[Number((n > 0n ? n - 1n : 0n) % BigInt(presets.length))];
  const variant = Number(
    ((n > 0n ? n - 1n : 0n) / BigInt(presets.length)) % 4n,
  );
  return {
    ...base,
    variant,
    name:
      variant === 0
        ? base.name
        : `${base.name} · ${["", "ash", "sand", "mist"][variant]}`,
    helmet:
      variant === 0
        ? base.helmet
        : ["", "#888980", "#d9c1ab", "#b9c5bf"][variant],
    jacket:
      variant === 0
        ? base.jacket
        : ["", "#242524", "#625448", "#424f48"][variant],
  };
}
const assets = new Map<string, string>();
export function scooterAsset(id: string) {
  const v = riderVisual(id),
    key = `${v.name}:${v.variant}`;
  let asset = assets.get(key);
  if (asset) return asset;
  let svg = scooterTemplate;
  for (const [from, to] of [
    ["#1141a8", v.dark],
    ["#4388ff", v.body],
    ["#164ab8", v.dark],
    ["#8db8ff", v.light],
    ["#478dff", v.body],
    ["#7bacff", v.light],
    ["#e6edf9", v.helmet],
    ["#a0afc8", v.helmet],
    ["#596780", v.dark],
    ["#242e40", v.jacket],
    ["#536074", v.dark],
  ])
    svg = svg.replaceAll(from, to);
  asset = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
  assets.set(key, asset);
  return asset;
}
