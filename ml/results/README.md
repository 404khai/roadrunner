# ETA baseline evaluation evidence

`phase24-b0-v1/` is the actual original frozen-route B0 evaluation of the immutable
`ml/data/phase23-representative-v1` dataset. It retains the complete original dataset,
frozen definitions, action-level pairings, coverage/segments and evaluator code bytes.
The manifest binds semantic and packaging SHA-256 separately.

```sh
roadrunner baseline verify ml/results/phase24-b0-v1
python3 scripts/verify_eta_baseline.py ml/results/phase24-b0-v1
```

There are five committed starts, four exact labels/pairs and one censored action.
All sources share one dependency group. Results are descriptive synthetic engineering
evidence only. Independent holdout, physical generalization and uncertainty are
unavailable. No model was trained or fitted. See [metric contracts and limitations](../../docs/eta-baseline.md).

Published result directories are immutable. Changed inputs/definitions/code require a
new version/destination. Only public synthetic sources are authorized for this example;
raw operational trajectories, credentials and private data do not belong here.
