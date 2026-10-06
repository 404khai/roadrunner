# Phase 16 paired dispatch fixture

`paired-strategies.json` is a synthetic five-node graph with six directed edges,
two riders, fourteen scheduled orders, and a 900-second window. The nearest rider
has a slow pickup road; the farther rider has a faster one. Mixed pickup locations,
seeded readiness delays, soft deadlines, one disconnected pickup, and an order after
the horizon exercise fleet interactions and explicit comparison populations.

The fixed seed is 16. No live routing or delivery dataset is involved. Readiness is
sampled before dispatch, independently of the policy, and only observed at its event.
The document uses `basic` so it can also be run with `simulate`; the benchmark command
replaces this field with the four comparison policies.

```bash
cargo run -p roadrunner-cli -- benchmark dispatch data/fixtures/phase-16/paired-strategies.json --json
```

See [benchmark methodology](../../../docs/dispatch-strategy-benchmarks.md).
