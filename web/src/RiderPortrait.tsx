import { riderVisual } from "./riderVisual";
import { portraitAsset } from "./riderPortraitAsset";
export function RiderPortrait({
  id,
  visualId,
}: {
  id: string;
  visualId?: string;
}) {
  const key = visualId ?? id;
  return (
    <img
      className="rider-portrait"
      src={portraitAsset(key)}
      alt={`Rider ${id} helmet portrait · ${riderVisual(key).name}`}
    />
  );
}
