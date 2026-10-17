// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { URL as NodeURL } from "node:url";
import { it, expect, vi, afterEach } from "vitest";
import {
  render,
  screen,
  fireEvent,
  cleanup,
  waitFor,
} from "@testing-library/react";
const raw = readFileSync(
  new NodeURL("../../docs/evidence/phase-31/spatial.json", import.meta.url),
  "utf8",
);
const empty = readFileSync(
  new NodeURL("../../docs/evidence/phase-31/empty.json", import.meta.url),
  "utf8",
);
import catalog from "./catalog.json";
import { catalogSchema } from "../src/contracts";
vi.mock("../src/DispatchMap", () => ({
  DispatchMap: ({ inspected }: { inspected: string }) => (
    <div data-testid="inspection">{inspected}</div>
  ),
}));
vi.mock("../src/useReveal", () => ({
  useReducedMotion: () => true,
  useReveal: () => ({
    progress: 1,
    playing: false,
    replay: vi.fn(),
    skip: vi.fn(),
    pause: vi.fn(),
    resume: vi.fn(),
  }),
}));
import { DispatchWorkspace } from "../src/DispatchWorkspace";
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
it("inspects real candidates, replays without re-evaluation and distinguishes recommendation", async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(raw));
  vi.stubGlobal("fetch", fetch);
  render(
    <DispatchWorkspace
      catalog={catalogSchema.parse(catalog)}
      token=""
      onExit={vi.fn()}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: "Evaluate dispatch" }));
  await screen.findByText("Recommended rider 2");
  expect(document.body.textContent).toContain(
    "Recommendation only · no publication",
  );
  expect(document.body.textContent).toContain("No active execution");
  fireEvent.click(screen.getByRole("button", { name: "Inspect candidate 3" }));
  expect(screen.getByTestId("inspection").textContent).toBe("3");
  fireEvent.click(
    screen.getByRole("button", { name: "Replay dispatch presentation" }),
  );
  expect(fetch).toHaveBeenCalledOnce();
  const request = JSON.parse(fetch.mock.calls[0][1].body);
  expect(request.riders.map((r: { node: number }) => r.node)).toEqual([
    0, 6, 11, 20, 24, 34,
  ]);
});
it("shows an empty shortlist without selecting a rider and preserves unavailable states", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(empty)));
  const view = render(
    <DispatchWorkspace
      catalog={catalogSchema.parse(catalog)}
      token=""
      onExit={vi.fn()}
    />,
  );
  fireEvent.change(screen.getByLabelText("Candidate policy"), {
    target: { value: "empty" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Evaluate dispatch" }));
  await screen.findByText("Unassigned · no feasible selected rider");
  expect(
    screen.queryByRole("button", { name: /Inspect candidate/ }),
  ).toBeNull();
  view.rerender(<DispatchWorkspace token="" onExit={vi.fn()} />);
  await waitFor(() =>
    expect(
      (
        screen.getByRole("button", {
          name: "Evaluate dispatch",
        }) as HTMLButtonElement
      ).disabled,
    ).toBe(true),
  );
});
