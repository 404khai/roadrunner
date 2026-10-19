# Fleet demonstration

Run commands from the repository root unless stated otherwise. This walkthrough
uses retained, engine-produced synthetic evidence; it does not require a live API,
a database, GPS observations or credentials.

## Five-minute workspace tour

```bash
cd web
npm ci
npm run dev
```

Open `http://127.0.0.1:5173/workspace?mode=delivery`. Choose **Recorded fleet**:
12 or 24 riders, then **Open recorded fleet demo**. The scene starts paused at
**1×**. Press Play or scrub the timeline; the simulation does not recompute per frame.
The external basemap needs network access. The replay itself is served locally.

1. Watch staggered orders appear and inspect the dispatch decisions and selected
   riders. In **Dispatch audit**, choose **Insertion** to inspect the retained
   MultiOrder decisions. All configured riders remain selectable, including idle riders.
2. Select a rider to inspect its accepted action, route, next stop, load and ETA.
   The first two riders have capacity two; the capture includes simultaneous custody
   of two orders. Restaurant readiness and pickup/dropoff service produce real waits.
3. Scrub around 30 seconds for a synthetic traffic update; at 180 seconds the factors
   return to normal. These are scenario inputs, never live traffic observations.
4. Scrub to 900 seconds: all 16 or 28 orders are delivered. Twelve configured riders
   does not mean twelve working riders: 8 receive work in that scene; 14 of 24 receive
   work in the larger scene. Idle riders are not given fabricated movement.
5. For accepted delays and replanning, use the existing delay preset with the API
   connected. For alternative route comparison, switch to the routing workspace.
   Those are separate supported demonstrations; this fleet replay does not claim a
   recovery occurred merely because traffic changed.

With a live API connected to the bundled Phase 10 graph, choose the 12- or 24-rider
live preset and run it to recompute the same scenario. A different graph cannot use
these snapshot-local anchors. Other graph-compatible presets remain available.
See [API setup](../crates/roadrunner-api/README.md) for configuration.

## Terminal demonstration

```bash
CARGO_INCREMENTAL=0 cargo build -p roadrunner-cli --locked
./target/debug/roadrunner demo
./target/debug/roadrunner demo --riders 24
./target/debug/roadrunner demo --riders 24 --json > /tmp/roadrunner-demo.json
```

The CLI executes the headless scenario and calculates alternatives on a pinned
comparison pair. It prints the workspace entry point; it does not start a server
or write a browser capture. The optional `--graph` must identify the same semantic
graph as the scenario. `--riders` accepts only 12 or 24. JSON includes the full
scenario, actual simulation result and alternative-route evidence.

## Reproduce the browser captures

The canonical inputs live in [fleet-demo-scenarios.json](../web/src/fleet-demo-scenarios.json).
CLI, live workspace presets and capture reproduction consume those same definitions.
The captures are compact JSON to remain inside the unchanged 8 MiB replay limit.

```bash
CARGO_INCREMENTAL=0 cargo build -p roadrunner-api --example fleet_demo --locked
./target/debug/examples/fleet_demo 12 web/public/demos/fleet-12.json --check
./target/debug/examples/fleet_demo 24 web/public/demos/fleet-24.json --check
```

This invokes the actual authenticated Axum scene endpoint in process, checks two
executions for byte-identical results, compares operational state before and after,
and compares the result to the retained file. It uses an isolated fixed namespace,
a logical clock and a read/simulate-only fixture credential. No PostgreSQL or
network server is involved. To generate a new file, omit `--check` and use a new
output path; existing files are never overwritten.

## Deliberate limits

These are bounded visual examples on the verified 58-node, 114-edge Phase 10 graph,
not a city-scale capacity claim. Existing scene restrictions remain 32 riders,
64 orders, 2,000 processed entries and an 8 MiB encoded response/replay. The larger
population example in the roadmap is illustrative, not implemented by raising caps.
Headless scale experiments are separate: [Phase 34](phase-34-completion.md).
Positions are route-derived synthetic estimates, with accepted events retaining
authority. Unknown geometry remains unavailable. See [motion semantics](live-delivery-visualization.md)
and [historical replay](historical-replay.md).
