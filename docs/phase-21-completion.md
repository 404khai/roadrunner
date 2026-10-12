# Phase 21 completion

Implemented against merged Phase 20 (`3e0f907`). The opt-in PostgreSQL/PostGIS
adapter persists the operational authority, original command outcomes, observations,
routes, dispatch decisions and isolated simulation runs. See the
[persistence run guide](persistence.md) for deployment and restore requirements.

## Delivered contracts

`roadrunner-storage` owns checksum-pinned offline migrations, a session writer
lease, monotonic fencing, atomic relational publication and restoration. Current
orders, riders, assignments, stops, accepted delivery terms and execution are
relational facts. Spatial positions and persisted route geometry use PostGIS with
GiST indexes. Immutable graph/traffic artifacts, history, publications, command
outcomes, source observations and simulation inputs/results preserve their provenance.

A complete versioned domain checkpoint preserves world identity, revisions,
allocators, accepted terms, active/frozen execution, recovery and effect aliases.
The API restores its private routing projections, policies, forecasts, retained
traffic, command recognition and observation ordering through a separate validated
format. Public decision/proposal types remain sealed. Successful restoration
checks actual relational content against its durable digest and artifact/configuration
bindings before serving requests.

Business effects, original terminal outcomes and immutable outbox obligations commit
in one transaction. A deferred SQL gate rechecks the certified window at COMMIT;
SQL write latency cannot silently consume the publication budget. Outbox records
carry finalized facts and identities. Broker delivery and consumer inboxes remain
Phase 22 work.

An independent locked, fsynced acknowledgement file preserves the last acknowledged
head outside PostgreSQL backups. Missing or regressed authority rejects restoration;
verified later receipts can reconcile a commit whose acknowledgement was lost.
Storage failure retires the writer and refuses cached operational reads. Accepted
workers own their reservations independently of HTTP disconnects. A new exclusive
writer resolves abandoned reservations without inventing an uncommitted success.

The default volatile mode and generated OpenAPI 3.1/vendored Swagger UI remain
available. Durable route queries additionally return persisted route record IDs.
Credentials are supplied afresh and never checkpointed; a rotated credential for
the same principal recognizes the original command outcome.

## Verification

[Check record](evidence/phase-21-verification.json),
[real process evidence](evidence/phase-21-runtime.json) and
[raw measurements](../benchmarks/results/phase-21-durable-http.json) record the results.
The workspace suite retains its previous tests and adds checkpoint validation.
Five explicit real-database tests verify rollback, spatial facts, exact restoration,
lease/fence exclusion, migration exclusion, configuration/head/relational corruption,
independent rollback detection and failure of acknowledgement after a real COMMIT.
They require a disposable database and run explicitly in the new CI persistence job.

Seven actual process/fault cases verify SIGKILL restart with frozen execution and
original terms, effect aliases across new command identities, spatial routes and
isolated simulations, SIGKILL after reservation/partial SQL writes, a lost HTTP
response followed by restart/retry, certificate expiry inside deferred COMMIT, and
writer exclusion/immutable outbox. No injected in-memory model substitutes for those
SQL and process boundaries. The PostgreSQL suite and process driver run sequentially
because the latter installs temporary database fault triggers.

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
export ROADRUNNER_TEST_DATABASE_URL='host=/tmp dbname=roadrunner_test'
cargo +1.99.0 test -p roadrunner-storage --test postgres --locked -- --ignored --test-threads=1
cargo +1.99.0 build -p roadrunner-api --locked
python3 scripts/verify_postgres_runtime.py
cargo +1.99.0 build -p roadrunner-api --release --locked
python3 scripts/collect_http_api_benchmark.py --database-url "$ROADRUNNER_TEST_DATABASE_URL" --output benchmarks/results/phase-21-durable-http.json
```

Local fault evidence uses PostgreSQL 18.4/PostGIS 3.6.4 on Apple M3. CI is configured
for PostgreSQL 16/PostGIS 3.5; its result is not claimed as a local verification.
The release collector records 30 sequential loopback samples per workload on the
58-node/114-edge fixture, including PostgreSQL and acknowledgement-file fsync.
Median/p95/p99 and raw data are recorded; CPU/memory are unmeasured. This is a
reproducible persistence baseline, not a production throughput or scalability claim.

## Supported operational envelope

One PostgreSQL primary and one active writer per configured world; no automatic
failover, reconnect or multi-primary authority. The trusted-local driver uses NoTLS
and requires a separately protected absolute acknowledgement file. Production TLS,
role provisioning, external source attestation and highly available deployment are
outside this implementation. Physical writable clones cannot be globally fenced
by this single-primary protocol and are unsupported. Never copy or roll back the
acknowledgement file with a database backup. The run guide states restore, clone,
retention and migration constraints explicitly. Operational routing retains the
existing static-road certification scope.
