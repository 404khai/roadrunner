import { useEffect, useState } from "react";
import { CodeBlock } from "./CodeBlock";
const repository = "https://github.com/404khai/roadrunner";
type OpenApi = {
  paths: Record<
    string,
    Record<
      string,
      {
        summary?: string;
        description?: string;
        operationId?: string;
        requestBody?: unknown;
        parameters?: unknown;
        responses?: unknown;
        security?: unknown;
      }
    >
  >;
  components?: { schemas?: Record<string, unknown> };
};
export function Documentation() {
  const [spec, setSpec] = useState<OpenApi>(),
    [error, setError] = useState(""),
    [query, setQuery] = useState("");
  useEffect(() => {
    const controller = new AbortController();
    fetch("/docs/openapi.json", { signal: controller.signal })
      .then((r) => {
        if (!r.ok) throw Error("API reference unavailable");
        return r.json();
      })
      .then(setSpec)
      .catch((e) => {
        if (!controller.signal.aborted) setError(String(e));
      });
    return () => controller.abort();
  }, []);
  return (
    <main id="content" className="documentation">
      <div className="docs-heading">
        <p className="eyebrow">ROADRUNNER / FIELD GUIDE</p>
        <h1>
          Understand the journey.
          <br />
          Then build your own.
        </h1>
        <p>
          Run the engine locally, inspect its decisions, and work with the
          actual HTTP contract.
        </p>
      </div>
      <div className="docs-layout">
        <nav aria-label="Documentation sections">
          {Object.entries({
            start: "Getting started",
            routing: "Route planning",
            dispatch: "Dispatch",
            simulation: "Delivery simulation",
            evidence: "Evidence & limits",
            api: "API reference",
          }).map(([id, label]) => (
            <a key={id} href={"#" + id}>
              {label}
            </a>
          ))}
        </nav>
        <article>
          <section id="start">
            <p className="eyebrow">01 / GETTING STARTED</p>
            <h2>Run a real engine.</h2>
            <p>
              Prerequisites: Rust stable with Cargo, Node 22.12+ (Node 24
              recommended), and npm. PostgreSQL is optional. The checked Lagos
              graph lets you start without downloading OSM data.
            </p>
            <CodeBlock language="shell">
              {
                'git clone https://github.com/404khai/roadrunner.git\ncd roadrunner\nexport ROADRUNNER_API_TOKEN="$(openssl rand -hex 32)"\ncargo run -p roadrunner-api -- --trusted-local \\\n  data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph'
              }
            </CodeBlock>
            <p>
              In another terminal, use the same token value. Keep it in the
              server environment; do not use a VITE-prefixed variable or commit
              credentials.
            </p>
            <CodeBlock language="shell">
              {
                'cd web\nnpm ci\nROADRUNNER_API_URL=http://127.0.0.1:3000 \\\nROADRUNNER_API_TOKEN="$ROADRUNNER_API_TOKEN" npm run dev'
              }
            </CodeBlock>
            <p>
              Open the Vite URL shown in the terminal. Landing demos replay
              retained engine output without an API. Open a workspace to compute
              fresh results through the same-origin development proxy. If
              connection fails, check the listener, proxy URL and credential.
              The workspace connection dialog supports an in-memory bearer
              credential.
            </p>
            <p>
              <a href="/workspace">Open route workspace ↗</a> ·{" "}
              <a
                href={repository + "/blob/main/crates/roadrunner-api/README.md"}
              >
                Server setup and persistence ↗
              </a>
            </p>
          </section>
          <section id="routing">
            <p className="eyebrow">02 / ROUTING</p>
            <h2>Topology first. Cost explicit.</h2>
            <p>
              Connect to load graph-bound anchors. Choose origin and
              destination, select Dijkstra or A*, then choose distance,
              free-flow travel time, or traffic-aware cost. Traffic-aware
              routing requires an adopted traffic context. Alternatives compare
              bounded Yen results with diversity and termination evidence.
            </p>
            <img
              src="/docs/routing.png"
              alt="Routing workspace showing selected route and route statistics"
              loading="lazy"
            />
            <p>
              Inspect distance, predicted duration, search counters, and
              original directed edge occurrences. Geographic heuristics must be
              admissible for the objective. Equal optimal costs need not imply
              an identical path when ties exist.
            </p>
            <p>
              <a href={repository + "/blob/main/docs/routing.md"}>
                Routing algorithms and assumptions ↗
              </a>
            </p>
          </section>
          <section id="dispatch">
            <p className="eyebrow">03 / DISPATCH</p>
            <h2>Inspect the reason, not just the rider.</h2>
            <p>
              Open Dispatch visualization, choose pickup and customer anchors,
              configure riders, radius or exhaustive selection, candidate limit,
              and preparation awareness. Run the evaluation to inspect
              eligibility, pickup and delivery estimates, waiting and score. A
              recommendation does not establish active delivery execution.
            </p>
            <img
              src="/docs/dispatch.png"
              alt="Engine dispatch evidence with candidates and selected rider"
              loading="lazy"
            />
            <p>
              The staged reveal is a reading aid, not measured solver
              chronology. Publication is opt-in in the isolated demo;
              operational dispatch uses separate versioned command contracts.
            </p>
            <a href="/workspace?mode=dispatch">Try dispatch visualization ↗</a>
          </section>
          <section id="simulation">
            <p className="eyebrow">04 / DELIVERY SIMULATION</p>
            <h2>One rider. One action. One timeline.</h2>
            <p>
              Select a preset, run the simulation, then play or scrub logical
              time. Distributed Fleet requires the pinned Phase 10 graph.
              Inspect assigned orders, scalar capacity and custody, next stop,
              ETA, action identity, and accepted execution stage. Gold
              restaurant glyphs identify pickups; lavender person glyphs
              identify customers.
            </p>
            <img
              src="/docs/delivery.png"
              alt="Delivery simulation showing rider location tags and route geometry"
              loading="lazy"
            />
            <p>
              Overhead 2D and perspective views show helmet location tags below
              zoom 18. Zoom in to inspect scooters. Camera and representation
              changes preserve the same rider, action, and sampled position.
              Reduced motion disables automatic landing demo playback; the
              scrubber remains available.
            </p>
            <p>
              Movement follows original directed route geometry. Estimated
              arrival cannot complete an action: retained accepted arrival,
              waiting, service and completion events remain authoritative.
              Missing original geometry leaves position unavailable.
            </p>
            <a href="/workspace?mode=delivery">Try delivery simulation ↗</a>
          </section>
          <section id="evidence">
            <p className="eyebrow">05 / EVIDENCE & LIMITS</p>
            <h2>Know what the picture proves.</h2>
            <p>
              Landing demos are checked engine responses from the deterministic
              Lagos fixture, not live GPS or production traffic. Dispatch riders
              stay at evaluated coordinates. Route reveals show geometry, not a
              moving delivery. Delivery positions between events are estimates.
              Synthetic ambient vehicles, when explicitly enabled in the full
              workspace, are a separate renderer layer.
            </p>
            <p>
              Basemap tiles and buildings depend on OpenFreeMap and
              network/WebGL availability. Missing tiles do not invalidate
              retained route evidence. No throughput, ETA accuracy, or
              production readiness claim follows from these demos. Keep graph
              identity, time domain and evidence version together when comparing
              results.
            </p>
            <p>
              <a
                href={
                  repository + "/blob/main/docs/live-delivery-visualization.md"
                }
              >
                Delivery contract ↗
              </a>{" "}
              ·{" "}
              <a
                href={repository + "/blob/main/docs/dispatch-visualization.md"}
              >
                Dispatch contract ↗
              </a>{" "}
              ·{" "}
              <a href={repository + "/blob/main/docs/benchmarks.md"}>
                Reproducible benchmarks ↗
              </a>
            </p>
          </section>
          <section id="api">
            <p className="eyebrow">06 / HTTP API</p>
            <h2>The actual server contract.</h2>
            <p>
              This reference is generated by the API’s OpenAPI 3.1 producer.
              Expand an operation for request bodies, parameters and responses,
              then inspect referenced schemas below.{" "}
              <a href="/docs/openapi.json" download>
                Download OpenAPI JSON ↓
              </a>
              . For interactive requests, open{" "}
              <code>http://127.0.0.1:3000/swagger-ui/</code> on your running
              server.
            </p>
            <h3>Authentication and errors</h3>
            <p>
              Send <code>Authorization: Bearer &lt;token&gt;</code>.
              Capabilities distinguish read, write and simulate, plus trusted
              observation source and optional resource allowlists. Trusted-local
              mode binds loopback. Keep server credentials out of public
              bundles. Missing or invalid credentials return 401; insufficient
              capability or scope returns 403.
            </p>
            <p>
              Domain errors use{" "}
              <code>{'{"code":"invalid_input","message":"…"}'}</code>. Common
              statuses include 404 for missing resources, 409 for state
              conflicts and 422 for invalid input. Malformed transport bodies
              and framework limits can have different bodies; inspect status and
              content type before decoding. Versioned writes require the exact
              command envelope and expected revision described by their schema;
              do not infer a write request from a read response.
            </p>
            <h3>First request</h3>
            <CodeBlock language="shell">
              {
                'curl http://127.0.0.1:3000/v1/state \\\n  -H "Authorization: Bearer $ROADRUNNER_API_TOKEN"\n\n# Inspect graph-bound anchors before building a route request\ncurl http://127.0.0.1:3000/v1/visualization/anchors \\\n  -H "Authorization: Bearer $ROADRUNNER_API_TOKEN"'
              }
            </CodeBlock>
            <p>
              <a href="/demos/route-request.json" download>
                Download a concrete fixture route request
              </a>{" "}
              ·{" "}
              <a href="/demos/route.json" download>
                Corresponding engine response
              </a>
              . POST this request to <code>/v1/visualization/routes</code> with
              JSON content type and bearer authorization. Its anchors and digest
              apply only to the checked Phase 10 graph. The operational{" "}
              <code>/v1/routes</code> response is a separate contract.
            </p>
            <label className="api-search">
              Filter operations
              <input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="routes, orders, actions…"
              />
            </label>
            {error && (
              <p role="alert">
                {error}. Download the contract from your running server at
                /openapi.json.
              </p>
            )}
            {!spec && !error && <p role="status">Loading API contract…</p>}
            {spec && (
              <>
                <div className="api-operations">
                  {Object.entries(spec.paths)
                    .filter(([path]) =>
                      path.toLowerCase().includes(query.toLowerCase()),
                    )
                    .map(([path, methods]) =>
                      Object.entries(methods)
                        .filter(([method]) =>
                          ["get", "post", "patch", "delete", "put"].includes(
                            method,
                          ),
                        )
                        .map(([method, operation]) => (
                          <details key={method + path}>
                            <summary>
                              <span className="http-method">
                                {method.toUpperCase()}
                              </span>
                              <code>{path}</code>
                            </summary>
                            <h4>
                              {operation.summary || operation.operationId}
                            </h4>
                            {operation.description && (
                              <p>{operation.description}</p>
                            )}
                            <CodeBlock language="json">
                              {JSON.stringify(operation, null, 2)}
                            </CodeBlock>
                          </details>
                        )),
                    )}
                </div>
                <h3>Schema dictionary</h3>
                <p>
                  Names in <code>$ref</code> resolve to the definitions below.
                  Required fields, enums, nullability and numeric constraints
                  are retained exactly.
                </p>
                {Object.entries(spec.components?.schemas ?? {}).map(
                  ([name, schema]) => (
                    <details key={name} id={"schema-" + name}>
                      <summary>{name}</summary>
                      <CodeBlock language="json">
                        {JSON.stringify(schema, null, 2)}
                      </CodeBlock>
                    </details>
                  ),
                )}
              </>
            )}
          </section>
        </article>
      </div>
    </main>
  );
}
