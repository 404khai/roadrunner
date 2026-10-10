# Pre-Phase-20 remediation measurements

These records measure the volatile foundation, graph verification and Phase 15–19
regression scenarios. They make no HTTP, persistence, production throughput or
scalability claim. Each JSON records configuration, hardware, input identity,
raw samples, aggregates and explicit memory-measurement status.

See the [completion report](../../../docs/pre-phase-20-remediation-completion.md)
for commands, prerequisite gates, applicability limits and measured results.

- `operational-boundary.json`: 101 timed evaluation/certification/publication samples.
- `graph-identity.json`: eight Criterion configurations, 20 samples each.
- `phase-15.json` through `phase-19.json`: 11 exact measured replays per configuration.

Run collectors from the repository root after building the release CLI. The report
lists exact commands. Fleet/recovery collectors can use the retained oracle log at
`docs/evidence/pre20-verification/oracles.log` or regenerate it with documented tests.
