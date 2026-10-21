# Phase 38–39 — fleet showcase and v0.1.0 documentation

## Delivered behavior

Phase 38 adds shared, graph-qualified 12-rider/16-order and 24-rider/28-order scenarios.
The CLI `roadrunner demo` runs actual simulation plus a pinned alternative-route
comparison. The delivery workspace can load retained producer output without an API
or recompute the shared preset when connected to the correct graph. Replays start
paused at 1×. A scrollable fleet grid keeps all riders inspectable and reports
travelling/at-stop/idle populations. A mobile navigation positioning correction avoids
stretching a collapsed card between conflicting top/bottom constraints.

Scenarios include staggered arrivals, explicit readiness/service, capacity-two custody,
ETA/action inspection and two synthetic traffic changes. Existing delay/recovery and
routing-comparison presets complete the walkthrough; the fleet scene does not claim
a reroute solely from a traffic update. No algorithm, admission rule, evidence format,
capture limit or publication authority changes.

| Scenario | Configured riders | Riders receiving work | Orders delivered | Outstanding | Frames | Encoded bytes |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| fleet-showcase-12/v1 | 12 | 8 | 16 | 0 | 131 | 2,043,597 |
| fleet-showcase-24/v1 | 24 | 14 | 28 | 0 | 227 | 5,728,351 |

These are exact capture/accounting observations, not throughput benchmarks. Both use
the verified 58-node/114-edge Phase 10 artifact, seed 38, 900 logical seconds,
MultiOrder dispatch and work budget 1,024. The rest of the configured riders stay
honestly idle. Both scenes include more than six simultaneous travelling riders and
at least one load-of-two state. The captures fit the existing 8 MiB replay boundary.

[Canonical inputs](../web/src/fleet-demo-scenarios.json) are embedded by the CLI and
API reproduction example and imported by the workspace. [Reproduction instructions](demo.md)
use the real authenticated in-process Axum endpoint, twice, with operational-state
comparison before/after. The retained [12-rider check](evidence/phase-38/fleet-12-check.json)
and [24-rider check](evidence/phase-38/fleet-24-check.json) record identical replay and
unchanged authority; CI repeats the byte comparison. [Artifact hashes/environment](evidence/phase-38/manifest.json)
identify the local collection. Compact captures are excluded from Prettier by the
existing fixture-byte exclusion; they must not be pretty-printed past export limits.

## Documentation consolidation

Phase 39 already had substantial domain documentation. The missing algorithms,
routing, optimization and machine-learning chapters now explain mathematical
assumptions and link actual implementations/contracts. The architecture overview and
README no longer stop at Phase 19/22. The [index](README.md), [demo tour](demo.md) and
[v0.1.0 scope](v0.1.0.md) connect the existing guides without replacing historical
completion evidence or claiming roadmap phase numbers alone prove support.

## Verification

- CLI suite: 24 tests, including both fleet completions, byte-identical headless replay,
  actual alternative routes and invalid population/graph/argument refusal.
- API reproduction example: both retained captures regenerated identically twice;
  operational state unchanged; full bytes match retained files. CI repeats this.
- Frontend: real-fixture decoding, all configured riders, completion/custody/traffic,
  graph mismatch, unavailable replay, no-API loading, paused 1×, rider-24 inspection
  and seeking. All 92 tests pass across 23 files with one worker. ESLint, Prettier
  and the production build pass.
- Rust 1.99 Clippy for CLI/API all targets; workspace format check. No new dependencies.
- Running browser: both recorded scenes loaded without credentials/API; 24 riders
  present, rider 24 selected with original route/action/ETA, 11 travelling at 100s,
  paused 1× retained. Narrow viewport checks preserve selectable roster and map.
  [Desktop evidence](evidence/phase-38/fleet-24-desktop.png) and
  [narrow viewport](evidence/phase-38/fleet-24-mobile.png).

A heavily parallel local frontend run hit five-second test timeouts. A bounded-worker
run is used for verification; the full-capture UI integration test has an explicit
15-second test timeout, which is not a performance threshold or a runtime deadline.
The production build retains its existing large-chunk warning. PostgreSQL/crash
recovery and the full Rust workspace remain covered by CI; they are not asserted as
locally rerun by this change. No performance experiment or supported-scale claim was
created from these small demos.

## Completion boundary

Phases 38–39 supply a reproducible evaluator experience and the required documentation
set for the v0.1.0 engineering preview. They do not close Phase 40/v1.0, publish a
release tag or authorize new city-scale visual capture. Basemap tiles still require
the external provider; a saved scene is finite evidence rather than live operations.
