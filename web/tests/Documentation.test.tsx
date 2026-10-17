// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import spec from "../public/docs/openapi.json";
import { Documentation } from "../src/Documentation";
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
it("publishes the generated operations and schemas and filters paths", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(new Response(JSON.stringify(spec))),
  );
  render(<Documentation />);
  await screen.findByText("/v1/riders/{id}/actions/arrival", {
    selector: "code",
  });
  expect(
    screen.getAllByText("/v1/routes", { selector: "code" })[0],
  ).toBeInTheDocument();
  fireEvent.change(screen.getByPlaceholderText("routes, orders, actions…"), {
    target: { value: "visualization/dispatch" },
  });
  expect(
    screen.queryByText("/v1/riders/{id}/actions/arrival", { selector: "code" }),
  ).not.toBeInTheDocument();
  expect(
    screen.getAllByText("/v1/visualization/dispatch", { selector: "code" }),
  ).toHaveLength(1);
  expect(
    screen.getByRole("link", { name: "Download OpenAPI JSON ↓" }),
  ).toHaveAttribute("href", "/docs/openapi.json");
});
