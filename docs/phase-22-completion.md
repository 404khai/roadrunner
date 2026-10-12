# Phase 22 completion

Implemented against merged Phase 21 (`33cddf8`). The synchronous operational boundary
remains authoritative. Phase 22 adds a local asynchronous PostgreSQL event transport,
transactional projection/notification consumers, ordering/quarantine/reconciliation,
restart-safe acknowledgement and isolated event-processing replay.

The [event run guide](events.md) states wire categories, identity/effect scopes,
upgrade/retention/restore constraints and the supported operational envelope.

## Delivered gate

- `roadrunner-events` polls the outbox on Tokio and keeps synchronous SQL work/client
  teardown on blocking workers. Serial role leases exclude duplicate publishers/consumers.
- Additive checksum-pinned migration `0002_events.sql` installs immutable stream,
  acknowledgement, attempt, effect and reconciliation evidence plus current inbox/cursors
  and projection. It leaves original operational data/checkpoint/anchor formats unchanged.
- At-least-once publication reuses original identity/position/body after append loses its
  acknowledgement. Source sequences and predecessors are explicit and independent of
  operational/storage revisions.
- Inbox meaning, source ordering, local business effect and original terminal receipt
  commit atomically. Transport acknowledgement and committed application remain separate.
  Message aliases recognize an existing effect; changed meaning preserves the original result.
- Gaps defer, poison/superseded meaning quarantines, and explicit principal/reason/fingerprint
  reconciliation records an omission. Runtime backs off blocked deliveries while other
  consumers progress. Read-only inspection works with live writers.
- Independent role acknowledgement files, immutable receipt ancestry and content gates detect
  missing/regressed/corrupt state and reconcile lost post-COMMIT acknowledgement. Writers do
  not reconnect after uncertain outcomes. SQL durability and file/directory fsync precede ACK.
- Projection checks actual recorded fact before-images. Notifications are durable local
  obligations. Neither is another operational authority, physical observation or remote send.
- Replay executes recorded message/attempt order and approved skips in isolated memory,
  verifies inbox/outcomes/cursors/effects/projection, and suppresses SQL/external effects.
  Incompatible/missing original inputs fail instead of using modern data.

No broker was introduced: the selected local transport establishes the required failure
boundaries without a new distributed dependency. This does not claim PostgreSQL is a
universal replacement for Kafka or prove broker throughput comparisons.

## Real fault evidence

[Verification record](evidence/phase-22-verification.json) and
[process evidence](evidence/phase-22-runtime.json) record passing checks.
Seven real PostgreSQL tests cover append/application lost acknowledgement and restart,
distinct message identities carrying one effect, same-commit fanout, source gap deferral, poison/conflict,
operator reconciliation, partial SQL rollback, post-COMMIT anchor failure and rollback
watermarks. An owned disposable-database upgrade test restores schema 1, applies the
additive migration and proves exact original authority/anchor preservation and old
outbox delivery. These opt-in tests run explicitly in CI; none of the existing tests
were disabled or converted to ignored tests.

Five actual-process histories inject SQL sleep barriers and SIGKILL: before publisher
append COMMIT, after append/before transport ACK, after partial consumer effect/before
inbox COMMIT, and after effect/inbox COMMIT/before delivery ACK. Restart drains each
original obligation with one local effect and intact operational state. The fifth case
verifies asynchronous live delivery, concurrent publisher exclusion, live inspection,
graceful SIGINT teardown and isolated compatible replay.

The existing five Phase 21 real storage tests and seven Phase 21 process cases also
run against the new schema/binaries. They preserve frozen execution/authentic terms,
recognition across distinct operational commands, lost response recovery, simulation
isolation and deferred temporal COMMIT rejection. CI runs all database/process suites
sequentially because fault hooks and migrations require controlled exclusion.

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
export ROADRUNNER_TEST_DATABASE_URL='host=/tmp dbname=roadrunner_test'
cargo +1.99.0 test -p roadrunner-storage --test postgres --locked -- --ignored --test-threads=1
cargo +1.99.0 test -p roadrunner-storage --test events --locked -- --ignored --test-threads=1
cargo +1.99.0 build -p roadrunner-api -p roadrunner-events --locked
python3 scripts/verify_postgres_runtime.py --output /tmp/phase-21-regression.json
python3 scripts/verify_event_runtime.py
cargo +1.99.0 build -p roadrunner-api -p roadrunner-events --release --locked
python3 scripts/collect_event_benchmark.py
```

## Measured delivery and limits

[Raw release measurements](../benchmarks/results/phase-22-event-delivery.json) record
hardware, compiler/binary hashes, verified graph/hash/size, transport/consumer algorithm,
SQL/fsync configuration, three warmups, 30 raw samples and median/p95/p99. Each sample
measures from original HTTP commit acknowledgement until both consumers' delivery ACKs
are observed. Psql process launch and 40 ms observation polling are included, so these
are observed upper-bound samples, not precise internal service latency. CPU/memory are
explicitly unmeasured. A tiny sequential fixture establishes a reproducible baseline;
it does not establish a production throughput/SLO or scalability result.

Local verification uses PostgreSQL 18.4/PostGIS 3.6.4 on Apple M3; CI is configured for
PostgreSQL 16/PostGIS 3.5, with its result reported by the PR checks. Full evidence-content
hashing grows with retained history and is intentionally unoptimized. Profile before
optimizing it; no scalability claim hides this cost.

Single primary, trusted-local NoTLS/database operator, compatible retained semantics,
empty API genesis and separately protected operational/event anchors remain required.
Remote notification delivery, automatic event-triggered dispatch, public broker input,
physical-device attestation, online mixed versions, parallel consumers, automatic
failover, writable physical clones and exactly-once remote effects remain unsupported.
