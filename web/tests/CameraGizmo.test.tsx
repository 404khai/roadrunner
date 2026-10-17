// @vitest-environment jsdom
import { it, expect, vi } from "vitest";
import { render, fireEvent, screen, cleanup } from "@testing-library/react";
import { CameraGizmo, projectAxis } from "../src/CameraGizmo";
it("projects world axes and provides keyboard presets and rear return", () => {
  expect(projectAxis(1, 0, 0, 0, 0)).toEqual([70, 42]);
  expect(projectAxis(0, 1, 0, 0, 0)).toEqual([42, 14]);
  expect(projectAxis(0, 0, 1, 0, 90)[1]).toBeCloseTo(14);
  const onPreset = vi.fn(),
    onRear = vi.fn();
  render(
    <CameraGizmo
      bearing={0}
      pitch={55}
      onOrbit={vi.fn()}
      onPreset={onPreset}
      onRear={onRear}
      onFront={vi.fn()}
    />,
  );
  fireEvent.click(
    screen.getByRole("button", { name: "View rider from X axis" }),
  );
  fireEvent.keyDown(
    screen.getByRole("button", { name: "View rider from Z axis" }),
    { key: "Enter" },
  );
  expect(onPreset.mock.calls).toEqual([["X"], ["Z"]]);
  fireEvent.click(screen.getByRole("button", { name: "Rear" }));
  expect(onRear).toHaveBeenCalledOnce();
  cleanup();
});
