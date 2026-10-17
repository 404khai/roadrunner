// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Site } from "../src/Site";
vi.mock("../src/EngineDemo", () => ({ EngineDemo: () => null }));
vi.mock("../src/Documentation", () => ({ Documentation: () => null }));
afterEach(cleanup);
it("opens navigation and closes it on Escape, outside click and link selection", () => {
  render(<Site />);
  const toggle = screen.getByRole("button", { name: "Open navigation" });
  fireEvent.click(toggle);
  expect(toggle).toHaveAttribute("aria-expanded", "true");
  fireEvent.keyDown(document, { key: "Escape" });
  expect(toggle).toHaveAttribute("aria-expanded", "false");
  expect(toggle).toHaveFocus();
  fireEvent.click(toggle);
  fireEvent.pointerDown(document.body);
  expect(toggle).toHaveAttribute("aria-expanded", "false");
  fireEvent.click(toggle);
  fireEvent.click(screen.getByRole("link", { name: "The engine" }));
  expect(toggle).toHaveAttribute("aria-expanded", "false");
  const cta = screen.getByRole("link", { name: "View on GitHub" });
  expect(cta.querySelector("svg.github-mark")).not.toBeNull();
  expect(cta.textContent).not.toMatch(/[↗→]/);
});
