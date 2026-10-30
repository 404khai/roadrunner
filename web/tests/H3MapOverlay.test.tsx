// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import type { Map, GeoJSONSource } from "maplibre-gl";
import { H3MapOverlay } from "../src/H3MapOverlay";

afterEach(cleanup);

it.each(["dispatch", "delivery"] as const)(
  "toggles %s geometry and updates retained locations without inventing entity IDs",
  (scope) => {
    const sources = new globalThis.Map<
      string,
      { setData: ReturnType<typeof vi.fn> }
    >();
    const map = {
      getSource: (id: string) => sources.get(id),
      addSource: vi.fn((id: string) => sources.set(id, { setData: vi.fn() })),
      addLayer: vi.fn(),
    } as unknown as Map;
    const beforeLayer = `${scope}-routes`;
    const locations = [[3.3792, 6.5244]] as const;
    const { rerender } = render(
      <H3MapOverlay
        map={map}
        locations={locations}
        beforeLayer={beforeLayer}
        scope={scope}
      />,
    );
    const source = map.getSource(`${scope}-h3-cells`) as GeoJSONSource;
    const data = () =>
      vi.mocked(source.setData).mock.calls.at(-1)?.[0] as {
        features: { properties: object }[];
      };
    expect(data().features).toEqual([]);
    fireEvent.click(screen.getByRole("checkbox", { name: "H3 hexagons" }));
    expect(data().features).toHaveLength(1);
    expect(Object.keys(data().features[0].properties)).toEqual(["cell"]);
    fireEvent.change(screen.getByLabelText("H3 cell resolution"), {
      target: { value: "12" },
    });
    expect(map.addSource).toHaveBeenCalledTimes(1);
    const first = JSON.stringify(data());
    rerender(
      <H3MapOverlay
        map={map}
        locations={[[3.4, 6.54]]}
        beforeLayer={beforeLayer}
        scope={scope}
      />,
    );
    expect(JSON.stringify(data())).not.toBe(first);
    expect(screen.getByRole("checkbox", { name: "H3 hexagons" })).toBeChecked();
    fireEvent.click(screen.getByRole("checkbox", { name: "H3 hexagons" }));
    expect(data().features).toEqual([]);
    expect(
      screen.queryByLabelText("H3 cell resolution"),
    ).not.toBeInTheDocument();
  },
);

it("keeps controls usable before map readiness and handles empty scenes", () => {
  render(
    <H3MapOverlay
      map={null}
      locations={[]}
      beforeLayer="delivery-routes"
      scope="delivery"
    />,
  );
  fireEvent.click(screen.getByRole("checkbox", { name: "H3 hexagons" }));
  expect(screen.getByText("No scene locations available.")).toBeInTheDocument();
});
