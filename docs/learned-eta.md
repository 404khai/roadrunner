# Offline learned route ETA

Roadrunner can fit and reproduce a small learned correction to its original
whole-route B0. This is a synthetic research tool. It does not change routing,
dispatch, API predictions, accepted terms or operational authority.
[ADR 0022](adr/0022-offline-learned-route-eta.md) freezes the experiment and gates;
the [completion report](phase-25-completion.md) records measured results.

The target is accepted synthetic arrival minus original logical start, for each
committed started road leg. It is not an edge label or physical travel measurement.
The model uses exactly two original start-time values: B0 seconds and distance
meters. No scenario, seed, source identity, calendar, future delay, traffic update,
arrival or retrospective segment membership enters fitting or inference.

The transparent model is regularized linear residual regression:

```text
x = [1, (original_B0 - Train_mean_B0) / Train_scale_B0,
        (original_distance - Train_mean_distance) / Train_scale_distance]
minimize sum_eligible_Train (observed_duration - original_B0 - x·beta)^2
         + sum_j beta_j^2
predicted_whole_route_seconds = original_B0 + x·beta
```

Train scales are population standard deviations; constant columns use 1. The
intercept is also regularized. A deterministic three-variable Cholesky solver uses
Python standard-library binary64 arithmetic. There is one fixed model and no
hyperparameter or checkpoint search. Validation is reported without tuning; Test
remains excluded from all fitting and preprocessing. No third-party ML runtime,
model download or Rust-to-PyTorch dependency is needed.

## Use

Python 3.9+ and the repository's Rust toolchain suffice:

```bash
cargo build -p roadrunner-cli --locked
# Export only after complete Rust source/census/feature/split verification.
target/debug/roadrunner learned export ml/data/phase23-representative-v1

# Existing representative data: explicitly unavailable for fitting.
python3 ml/eta_engine.py run ml/data/phase23-representative-v1 /tmp/eta-unavailable-new

# Controlled null study: build into a NEW destination, then fit.
target/debug/roadrunner dataset build data/fixtures/phase-25/request.json /tmp/eta-controlled-new
python3 ml/eta_engine.py run /tmp/eta-controlled-new /tmp/eta-research-new
python3 ml/eta_engine.py verify /tmp/eta-research-new
python3 scripts/verify_learned_eta.py /tmp/eta-research-new

# Verify the actual committed publications.
python3 ml/eta_engine.py verify ml/results/route-ridge-null-v1
python3 ml/eta_engine.py verify ml/results/route-ridge-unavailable-v1
python3 -m unittest discover -s ml/tests -v
```

All publications refuse overwriting an existing destination. Rebuilding with
changed source, compiler or retained evidence has a new identity, even when metric
values coincide. `scripts/generate_eta_research_fixtures.py NEW_DIRECTORY` reproduces
the checked-in request, geometry/topology fixtures and design declaration exactly.

## Inference boundary

`eta_engine.predict(model, request, expected_request_sha256)` accepts an original
pinned directed route, departure reference, routing context, start-order witness,
two-value feature snapshot and content-bound model. The caller supplies the
expected request digest from the verified export. The returned record binds the
logical action, model and request, exact effective binary64 values/hex, correction,
expected duration, or explicit failure. The predictor has no label argument.

The command `python3 ml/eta_engine.py predict RESULT REQUEST.json` first verifies
the publication and accepts only a request exactly present in its verified
`research.json` export. It rejects unverified future/counterfactual requests.
The module interface is for offline research callers honoring the same verified
input boundary; it grants no operational authority or freshness guarantee.

Unsupported profile/traffic/units/schema, changed request context, missing or
corrupt model, nonfinite values, out-of-Train-range features, and negative/nonfinite
predictions fail explicitly. Predicted values are not clamped. Point estimates
are not uncertainty intervals or physical upper bounds.

## Comparisons and fallback

Every committed start remains in the census, including incomplete and Excluded
rows. Only admissible exact Observed, non-Excluded synthetic labels enter fitting
or exact metrics; legitimate zero is eligible. Bounds and missing labels are
never zero filled. Protected dependency components and original B0 are immutable.

Reports retain the original baseline report and every predeclared distance,
estimate, cohort, graph, profile, traffic, family, generator, policy, source,
dependency-group, partition and retrospective intervention slice, including empty
slices. Candidate and B0 matched metrics use exactly the same successful action
keys. Full-population B0 and coverage are shown independently of candidate success.
Each reported slice is an offline batch: any included request failure causes
original-B0 fallback for the **entire** slice. This reports a coherent fallback
comparison without mixing learned and deterministic predictions in one batch.
It is not an operational planner fallback implementation.

Metrics use label minus prediction, equally weighted actions, MAE, RMSE, signed
mean/midpoint median, nearest-rank absolute/positive p50/p95/p99 and underprediction
frequency/severity. Coverage fractions include explicit numerators/denominators;
empty denominator ratios and empty exact metric populations are unavailable.
Candidate prediction coverage uses included starts, with Excluded counts retained.
No MAPE, row-bootstrap confidence intervals or physical claims are produced.

A protected Test comparison requires complete coverage, MAE improvement exceeding
1e-9 seconds and no RMSE deterioration. The margin protects against declaring
numeric roundoff a gain; it is not statistical significance. Operational admission
is always unavailable in this tool, regardless of a research comparison result.

## Actual evidence and limitations

`ml/results/route-ridge-unavailable-v1` preserves the existing one-component dataset:
5 starts, 4 exact labels, one censored start. It fits no model. All original B0
coverage and errors remain visible under declared fallback.

`ml/results/route-ridge-null-v1` contains its own verified dataset: six explicitly
constructed geometry/topology variants, eight runs (including a horizon descendant
and a no-start descendant), 13 starts and 12 exact labels. Six Train labels, two
Validation labels and four Test labels follow protected component assignments.
This is a controlled within-generator engineering design, not six sampled
independent physical worlds. Graph, scenario, trace, ancestry and assignment bytes
are retained. All no-delay source durations equal original B0. All coefficients
are zero and Test MAE/RMSE are zero for both B0 and candidate; improvement is false.
The correct result is a negative research experiment, not fabricated superiority.
The hypothetical positive unit fixture verifies the comparison gate mathematically;
it is not a measured model improvement claim.

Immutable publications retain `dataset/{bundle,manifest}.json`, `research.json`
and an outer manifest binding exact bytes. The dataset contains recoverable Rust
source and original source bundles. The research publication contains recoverable
Python source, actual runtime environment, protocol, model/preprocessing and input,
label and split identities, all requests, predictions, failures and comparisons.
The outer manifest is an integrity binding, not a signature or external trust root.
A crash before manifest completion leaves an unavailable publication, not a valid
partial result. Verification replays sources and refits/rescores, not merely hashes.

For standalone reproduction, restore each safe relative path from
`dataset/bundle.json` → `reproduction.files` after verifying its SHA-256, build the
CLI with retained Cargo.lock, restore `research.json` → `source.eta_engine.py`,
then run that script's `verify` with `--verifier /restored/target/debug/roadrunner`.
Retained output inspection can survive a missing model; inference reproduction
requires compatible model/source/runtime; training reproduction additionally
requires original eligible labels, inputs and split. The committed reproduction
evidence performed the full fresh-source rebuild outside the repository.

Before shadow or operational use, a separate proposal must establish representative
labels/residuals, supported applicability, dependence-aware uncertainty and relevant
whole-proposal gains, resource measurements, isolation, freshness/adoption and
atomic fallback. Started-route accuracy alone validates none of edge costs,
unchosen paths, learned routing or hard feasibility guarantees.
