# roadrunner-dataset

Offline Phase 23 started-leg dataset construction, evidence qualification, dependency
splits and immutable publication. Depends on core, dispatch and simulation only.

See the [dataset guide](../../docs/eta-dataset.md) and
[ADR 0020](../../docs/adr/0020-eta-dataset-evidence-and-synthetic-traces.md).

```sh
cargo +1.99.0 test -p roadrunner-dataset --locked
cargo +1.99.0 run -p roadrunner-cli --locked -- dataset build data/fixtures/phase-23/request.json /tmp/eta-v1
cargo +1.99.0 run -p roadrunner-cli --locked -- dataset verify /tmp/eta-v1
```

Publication is synthetic only. Original B0 and source contracts remain immutable;
training, physical measurement claims and operational extraction are not implemented.

Phase 24 adds `baseline::evaluate_b0` and immutable self-contained baseline artifacts.
It evaluates original B0 only; no fitting or ML inference occurs. See the
[baseline guide](../../docs/eta-baseline.md) and [ADR 0021](../../docs/adr/0021-original-b0-evaluation.md).
