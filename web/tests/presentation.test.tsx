// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, renderHook } from "@testing-library/react";
import { useReveal } from "../src/useReveal";
let frames: Map<number, FrameRequestCallback>,
  seq = 0;
beforeEach(() => {
  frames = new Map();
  vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
    frames.set(++seq, cb);
    return seq;
  });
  vi.stubGlobal("cancelAnimationFrame", (id: number) => frames.delete(id));
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
function tick(now: number) {
  act(() => {
    const callbacks = [...frames.values()];
    frames.clear();
    callbacks.forEach((cb) => cb(now));
  });
}
describe("presentation-only reveal lifecycle", () => {
  it("pauses, resumes, changes speed, skips and replays without any network operation", () => {
    const fetch = vi.fn();
    vi.stubGlobal("fetch", fetch);
    const { result } = renderHook(() =>
      useReveal("evaluation:original", false, false),
    );
    tick(0);
    tick(650);
    expect(result.current.progress).toBe(0.25);
    act(() => result.current.pause());
    tick(1200);
    expect(result.current.progress).toBe(0.25);
    act(() => result.current.setSpeed(2));
    act(() => result.current.resume());
    tick(1500);
    tick(1825);
    expect(result.current.progress).toBe(0.5);
    act(() => result.current.skip());
    expect(result.current.progress).toBe(1);
    act(() => result.current.replay());
    tick(2000);
    tick(2325);
    expect(result.current.progress).toBe(0.25);
    expect(fetch).not.toHaveBeenCalled();
  });
  it("resets on replacement evidence and cancels the previous animation", () => {
    const { result, rerender } = renderHook(
      ({ id }) => useReveal(id, false, false),
      { initialProps: { id: "first" } },
    );
    tick(0);
    tick(1300);
    expect(result.current.progress).toBe(0.5);
    rerender({ id: "second" });
    expect(result.current.progress).toBe(0);
    tick(2000);
    tick(2650);
    expect(result.current.progress).toBe(0.25);
  });
  it("renders equivalent static results for reduced motion and no travel", () => {
    const { result, rerender } = renderHook(
      ({ reduced, noTravel }) => useReveal("original", noTravel, reduced),
      { initialProps: { reduced: true, noTravel: false } },
    );
    expect(result.current.progress).toBe(1);
    expect(result.current.playing).toBe(false);
    act(() => result.current.replay());
    expect(frames.size).toBe(0);
    rerender({ reduced: false, noTravel: true });
    expect(result.current.progress).toBe(1);
    expect(result.current.playing).toBe(false);
  });
});
