import { useEffect, useRef, useState } from "react";
import { Check, Copy } from "lucide-react";

export function CodeBlock({
  children,
  language = "shell",
}: {
  children: string;
  language?: "shell" | "json";
}) {
  const [state, setState] = useState<"idle" | "copied" | "error">("idle");
  const timeout = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => () => clearTimeout(timeout.current), []);
  async function copy() {
    clearTimeout(timeout.current);
    try {
      await navigator.clipboard.writeText(children);
      setState("copied");
      timeout.current = setTimeout(() => setState("idle"), 2000);
    } catch {
      setState("error");
    }
  }
  const tokens = children.split(
    /("(?:\\.|[^"\\])*"(?=\s*:)|"(?:\\.|[^"\\])*"|'[^']*'|#[^\n]*|\$[A-Za-z_][A-Za-z0-9_]*|--[\w-]+|\b(?:true|false|null|\d+(?:\.\d+)?)\b|\b(?:git|cd|cargo|npm|curl|export|run|ci|dev|clone)\b)/g,
  );
  return (
    <div className="code-block">
      <div className="code-toolbar">
        <span>
          {language === "shell" ? "TERMINAL / SHELL" : "JSON / CONTRACT"}
        </span>
        <button
          type="button"
          onClick={() => void copy()}
          aria-label={state === "copied" ? "Code copied" : "Copy code"}
        >
          {state === "copied" ? <Check size={14} /> : <Copy size={14} />}{" "}
          {state === "copied" ? "Copied!" : "Copy"}
        </button>
      </div>
      <pre>
        <code>
          {tokens.map((token, i) => {
            const kind =
              language === "json"
                ? token.startsWith('"')
                  ? /^\s*:/.test(tokens[i + 1] ?? "")
                    ? "key"
                    : "string"
                  : /^(true|false|null|\d)/.test(token)
                    ? "value"
                    : "plain"
                : token.startsWith("#")
                  ? "comment"
                  : /^["']/.test(token)
                    ? "string"
                    : token.startsWith("$")
                      ? "variable"
                      : token.startsWith("--")
                        ? "flag"
                        : /^(git|cd|cargo|npm|curl|export|run|ci|dev|clone)$/.test(
                              token,
                            )
                          ? "command"
                          : "plain";
            return (
              <span key={i} className={"syntax-" + kind}>
                {token}
              </span>
            );
          })}
        </code>
      </pre>
      <span className="copy-status" role="status">
        {state === "error"
          ? "Clipboard unavailable. Select the code to copy it manually."
          : state === "copied"
            ? "Copied to clipboard."
            : ""}
      </span>
    </div>
  );
}
