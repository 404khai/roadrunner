// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { URL as NodeURL } from "node:url";
import { it, expect, vi, afterEach } from "vitest";
import { render, screen, fireEvent, cleanup } from "@testing-library/react";
import { decodeScene } from "../src/deliveryScene";
import { DispatchAuditCard } from "../src/DispatchAuditCard";
function load(name: string) {
  const raw = readFileSync(
    new NodeURL(`../../docs/evidence/phase-31/${name}.json`, import.meta.url),
    "utf8",
  );
  return decodeScene(raw, JSON.parse(raw).graph_digest);
}
afterEach(cleanup);
it("labels nonradial fleet evidence and binds the decorative anchor without candidate rankings", () => {
  const onFocus = vi.fn(),
    scene = load("fleet-audit");
  render(<DispatchAuditCard scene={scene} onFocus={onFocus} />);
  expect(document.body.textContent).toContain("Nonradial policy");
  expect(document.body.textContent).toContain("committed in simulation");
  expect(document.body.textContent).toContain("not candidate rankings");
  fireEvent.click(
    screen.getByRole("button", { name: "Present nonradial search metaphor" }),
  );
  expect(onFocus).toHaveBeenCalledOnce();
  expect(onFocus.mock.calls[0][0].label).toContain("no search radius");
});
it("retains incomplete search without implying publication and handles absent records", () => {
  const view = render(
    <DispatchAuditCard
      scene={load("insertion-incomplete")}
      onFocus={vi.fn()}
    />,
  );
  fireEvent.change(screen.getByLabelText("Decision family"), {
    target: { value: "insertion" },
  });
  expect(document.body.textContent).toContain("SearchIncomplete");
  expect(document.body.textContent).toContain("not committed");
  view.rerender(<DispatchAuditCard onFocus={vi.fn()} />);
  expect(document.body.textContent).toContain("No retained insertion decision");
});
it("shows recovery receipt without claiming a radial search or per-rider optimum", () => {
  render(
    <DispatchAuditCard scene={load("recovery-audit")} onFocus={vi.fn()} />,
  );
  fireEvent.change(screen.getByLabelText("Decision family"), {
    target: { value: "recovery" },
  });
  expect(document.body.textContent).toContain("LocalMinimum");
  expect(document.body.textContent).toContain("No fastest-courier claim");
});

it("retains unavailable forecast failures without inferring feasibility", () => {
  render(
    <DispatchAuditCard scene={load("unavailable-audit")} onFocus={vi.fn()} />,
  );
  fireEvent.change(screen.getByLabelText("Decision family"), {
    target: { value: "failures" },
  });
  expect(document.body.textContent).toContain("Unavailable prediction input");
  expect(document.body.textContent).toContain("PredictionUnavailable/v1");
  expect(document.body.textContent).toContain("not committed");
  expect(document.body.textContent).toContain("Input complete: false");
});
