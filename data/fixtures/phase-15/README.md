# Phase 15 synthetic delivery scenario

`seeded-deliveries.json` is a self-contained CLI document with a synthetic directed
road graph and version-1 simulation scenario. It contains five nodes (one isolated),
six directed edges, two initially available riders, and six scheduled orders.
Store/restaurant and customer locations are graph-node anchors. No real users,
production traffic, preparation feed, or production-scale claim is represented.

The 600-second horizon includes five order creations; four orders can be delivered,
one exceeds rider scalar capacity, and one is scheduled at second 700. Two orders
use seeded readiness delays, others have fixed actual ready events. Predictions and
observations are separate. Static traffic changes at seconds 60 and 120 replace the
complete overlay. Directed edge 2 is the store-to-customer road under canonical graph
compilation; traffic is bound to the graph represented by this document.

```bash
cargo run -p roadrunner-cli -- simulate data/fixtures/phase-15/seeded-deliveries.json
cargo run -p roadrunner-cli -- simulate data/fixtures/phase-15/seeded-deliveries.json --json
```

Same fixture and seed replay byte-for-byte, including generated readiness, decisions,
event sequence, and summary. Input order identities define canonical generator order.
See [simulation documentation](../../../docs/simulation.md) for schema, event ordering,
metric populations, partial execution, graph artifacts, and limitations.
