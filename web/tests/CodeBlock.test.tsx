// @vitest-environment jsdom
import "@testing-library/jest-dom/vitest";
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { CodeBlock } from "../src/CodeBlock";
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
});
it("copies exact command bytes, including quotes, environment variables and newlines", async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);
  vi.stubGlobal("navigator", { clipboard: { writeText } });
  const command =
    '# Start the engine\nexport TOKEN="keep $VALUE literal"\ncargo run -- --trusted-local graph';
  const { container } = render(<CodeBlock>{command}</CodeBlock>);
  expect(container.querySelector("code")?.textContent).toBe(command);
  expect(container.querySelector(".syntax-command")).toHaveTextContent(
    "export",
  );
  fireEvent.click(screen.getByRole("button", { name: "Copy code" }));
  await screen.findByRole("button", { name: "Code copied" });
  expect(writeText).toHaveBeenCalledWith(command);
});
it("keeps JSON intact and reports clipboard failures without a false success", async () => {
  vi.stubGlobal("navigator", {
    clipboard: { writeText: vi.fn().mockRejectedValue(Error("denied")) },
  });
  const json = JSON.stringify(
    { request: "<script>literal</script>", enabled: true },
    null,
    2,
  );
  const { container } = render(<CodeBlock language="json">{json}</CodeBlock>);
  expect(container.querySelector("code")?.textContent).toBe(json);
  expect(container.querySelector("script")).toBeNull();
  expect(container.querySelector(".syntax-key")).toHaveTextContent('"request"');
  fireEvent.click(screen.getByRole("button", { name: "Copy code" }));
  await screen.findByText(
    "Clipboard unavailable. Select the code to copy it manually.",
  );
  expect(
    screen.queryByRole("button", { name: "Code copied" }),
  ).not.toBeInTheDocument();
});
