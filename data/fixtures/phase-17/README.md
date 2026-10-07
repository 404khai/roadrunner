# Phase 17 scenarios

Schema 2 fixtures use one rider (capacity 2), two graph nodes, a bidirectional road,
explicit per-order policies (2s pickup service, 3s dropoff service) and valid forecasts.
They contain deterministic synthetic facts, not production delivery data.

```bash
cargo +1.99.0 run -p roadrunner-cli -- simulate data/fixtures/phase-17/successful-pooling.json --json
```

| Fixture | Required behavior |
| --- | --- |
| successful-pooling | A assigned, B inserted during frozen A travel, both picked up then delivered; independent colocated stops |
| frozen-wait | B arrives during A readiness wait; active A still completes at 27s |
| frozen-service | B arrives during A pickup service; same frozen A completion |
| capacity-prefix | Some placements overload onboard demand; drop A before picking B remains feasible |
| complete-infeasibility | B demand exceeds capacity even alone; complete scoped rejection |
| hard-deadline | B's 20s hard deadline cannot be satisfied; no B terms or work leak |
| cumulative-repeat | Original A acceptance reference with zero delay allowance survives repeated B/C insertion; delayed placements rejected |
| unavailable-forecast | Typed PredictionUnavailable; typed nonpublication evidence; existing execution continues |
| budget-exhaustion | Budget 1 permits a feasible partial search but no commit; later fresh complete attempts may admit |
| baseline-predicted-breach | Traffic worsens after acceptance; hard-breached A rider excluded during new admission and executes unchanged |
| realized-readiness-violation | Valid hard admission under forecast, actual readiness later misses deadline; explicit actual violation |

Custody/fulfillment corruption and picked-up/frozen-dropoff states are tested directly
in dispatch fixtures; fresh runtime scenario construction rejects corrupt worlds.

Run all fixtures with byte-replay, coverage checks and external process measurements:

```bash
cargo +1.99.0 build -p roadrunner-cli --release --locked
python3 scripts/collect_multi_order_benchmark.py --output /tmp/phase17.json
```
