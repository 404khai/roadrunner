# Machine learning and ETA research

Roadrunner's production-facing algorithms remain deterministic and operate without
an ML runtime. The repository includes offline synthetic ETA research, not a deployed
learned routing cost or physical-telemetry model.

```text
retained synthetic starts + accepted outcomes
                     |
          source/census/identity verification
                     |
          leakage-safe Train / Validation / Test
                     |
       deterministic baselines -> learned comparison
                     |
          gated offline evidence (no publication)
```

[ETA dataset](eta-dataset.md) documents provenance, observation eligibility and split
rules. A route start's original features must not be replaced by later traffic or
arrival facts. Synthetic arrival duration is not GPS-observed ground truth. Source
identities and scenario dependencies must survive export and independent verification.

[Baseline research](eta-baseline.md) defines deterministic estimates and evaluation.
[Learned ETA](learned-eta.md) implements a small regularized linear residual correction:

```text
prediction = original_B0 + standardized_original_features dot beta
loss = sum_Train (observed_duration - prediction)^2 + sum_j beta_j^2
```

Only training data determines preprocessing and coefficients; validation/test never
enter fitting. The target is a whole started route leg, not an edge label. Metrics
must disclose eligible populations, missing/unavailable predictions and supported
error denominators rather than reporting only favorable rows.

The retained controlled study is a negative experiment; another retained population
is explicitly unavailable for fitting. Neither establishes improvement sufficient to
integrate learned estimates into routing, dispatch, API predictions or accepted terms.
See [Phase 25 completion](phase-25-completion.md) and the reproducible commands in
[learned ETA](learned-eta.md). No PyTorch dependency or external model download is
required. Future integration needs a separate authorization and must preserve graph,
route, context, uncertainty and publication contracts.
