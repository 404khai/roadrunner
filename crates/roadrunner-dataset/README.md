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

Publication is synthetic only. Original B0 is stored and checked; no Phase 24 metrics,
training, physical measurement claims or operational extraction are implemented.
