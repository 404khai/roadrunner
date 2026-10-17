import { expect, it } from "vitest";
import {
  perspectiveScooter,
  relativeDirection,
} from "../src/perspectiveScooter";
it("selects camera-relative views across north wrap without inventing physical heading", () => {
  expect(relativeDirection(359, 1)).toBe(0);
  expect(relativeDirection(90, 0)).toBe(2);
  expect(relativeDirection(90, 90)).toBe(0);
  expect(relativeDirection(180, 0)).toBe(4);
  expect(relativeDirection(-90, 0)).toBe(6);
});
it("provides eight stable upright illustrations in each rider's original cosmetic palette", () => {
  const views = Array.from({ length: 8 }, (_, i) => perspectiveScooter("1", i));
  expect(new Set(views).size).toBe(8);
  expect(perspectiveScooter("1", 2)).toBe(views[2]);
  expect(perspectiveScooter("2", 2)).not.toBe(views[2]);
  for (const uri of views) {
    const svg = decodeURIComponent(uri.split(",")[1]);
    expect(svg).toContain('viewBox="0 0 80 96"');
    expect(svg).not.toMatch(/NaN|undefined|Infinity/);
  }
});
