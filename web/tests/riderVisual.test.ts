import { expect, it } from "vitest";
import { riderVisual, scooterAsset } from "../src/riderVisual";
it("provides 32 stable distinct cosmetic combinations without changing domain identities", () => {
  const visuals = Array.from({ length: 32 }, (_, i) =>
    riderVisual(String(i + 1)),
  );
  expect(
    new Set(visuals.map((v) => [v.body, v.helmet, v.jacket].join())).size,
  ).toBe(32);
  expect(riderVisual("1").name).toContain("Ochre");
  expect(riderVisual("2").name).toContain("Sage");
  expect(scooterAsset("1")).toBe(scooterAsset("1"));
  expect(scooterAsset("1")).not.toBe(scooterAsset("2"));
});
