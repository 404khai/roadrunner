# Persistence and durable operational authority

Phase 21 adds `roadrunner-storage` and opt-in persistent operation to the Phase 20 API.
Routing algorithms and transition validity remain in core/dispatch. PostgreSQL
current facts are authoritative; retained records explain their changes. The API cache
is installed only after a successful database COMMIT and independent acknowledgement
watermark fsync. No Kafka, external delivery, event-sourced restoration or automatic
failover is introduced.

## Run with PostgreSQL/PostGIS

Use a dedicated database on a local PostgreSQL installation with PostGIS available.
The CI fixture uses the [official PostGIS image](https://github.com/postgis/docker-postgis)
`postgis/postgis:16-3.5`; the recorded local verification used PostgreSQL 18.4/PostGIS 3.6.4.
The synchronous client uses `NoTls`; the supported binary remains the explicit
trusted-local loopback driver. This phase does not provision deployment credentials
or physical observation trust.

From the repository root:

```bash
createdb roadrunner_local
export ROADRUNNER_DATABASE_URL='host=/tmp dbname=roadrunner_local'
export ROADRUNNER_API_TOKEN="$(openssl rand -hex 32)"
install -d -m 700 "$PWD/.roadrunner-state"
export ROADRUNNER_DURABILITY_ANCHOR="$PWD/.roadrunner-state/local.head"

# Offline, before any writer is started:
cargo +1.99.0 run -p roadrunner-api -- --migrate

cargo +1.99.0 run -p roadrunner-api -- \
  --trusted-local data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph \
  --postgres local
```

`--postgres local` selects a stable database world key. The graph, operational
configuration, original namespace and time domain must match an existing world.
The anchor path must be absolute and retained on an independent durable filesystem.
Open `/swagger-ui/` and use the [Phase 20 HTTP guide](../crates/roadrunner-api/README.md).
The volatile run mode remains available by omitting `--postgres`.

Restart using the same database/world key/anchor/graph/configuration. Tokens may
rotate while retaining the stable principal. The namespace, local identities,
plan/action/generation counters, custody, accepted terms, frozen execution, contexts,
recognition and original results remain exact. A host clock behind the durable floor
rejects restoration; no clock or deadline is silently rebased.

## One authority and publication gate

Each writer holds a PostgreSQL session advisory lock for the world key and an
exclusive lock file beside its independent anchor. The session also holds a shared
migration lock. A new owner acquires the same exclusion, locks the world row and
increments its checked monotonic fence. There is no time-based lease TTL or implicit
leader election. A connection/fence/head failure retires the writer. Its cached state
cannot answer operational reads or publish afterwards.

Publication stages a private complete coordinator. The transaction locks the
namespace row and checks fence, storage revision, expected content digest and the
reconstructed relational before-image. Domain validation checks the complete staged
state. Writes cover facts, effective plans, execution, terms, context revisions,
recognition, command result, history and required outbox intent. They are invisible
until one COMMIT. Failure before COMMIT discards the entire staged replacement.

Routing, search and simulation stay outside the writer. Context adoption and
idempotency reservation are independently valid durable transactions. The final
business effect and its terminal command result commit together. Retries with the
same canonical meaning return the original result; conflicting meaning rejects.
Pending records cannot be mistaken for assignments or custody.

## Time and acknowledgement

The configured static-road certificate still binds the exact evaluated decision and
its source/adoption revisions. After initial relational writes, the storage callback
samples the monotonic clock under the namespace lock and stamps new acceptance,
recovery and publication evidence. Existing accepted terms and completion references
remain unchanged. The final immutable records contain that real gate sample, not the
old evaluation time. This sample is separate from network acknowledgement time and
is not a claim of nanosecond WAL-commit timestamp measurement.

A deferred PostgreSQL constraint trigger checks the database clock at COMMIT against
that certified endpoint and any new acceptance-reference cutoff, and rejects a clock
behind the sampled floor. Expiry while writing therefore rolls back all effects and
recognition. The supported deployment uses aligned UNIX-clock application/database
hosts; regression fails closed. Time-dependent operational certification and unknown
active-prefix proof retain the prerequisite limits.

The session forces `synchronous_commit=on`, requires `fsync=on`, and never acknowledges
before COMMIT succeeds. It then writes the independent head file through a temporary
file, file fsync, atomic rename and parent-directory fsync. Only then does it install
the cache or return success. A COMMIT transport failure or acknowledgement-file failure
is `OutcomeUnknown`; the old writer stops serving authority. It never labels such work
uncommitted or retries business effects automatically.

A fresh exclusive owner validates the current rows and immutable commit-receipt chain
against the independently acknowledged head. A confirmed later database head can
reconcile a lost acknowledgement. Remaining pending commands prove that no atomic
business/terminal publication committed; they become immutable
`interrupted_before_publication` outcomes. Callers use a new key for a new business attempt.

## Facts, spatial values and evidence

Current domain facts have explicit relational ownership:

- `rr_orders`: stable order facts, readiness and fulfillment; pickup/dropoff geography.
- `rr_riders`: profile, operational state, plan revision and location geography.
- `rr_assignments`: order/rider responsibility with foreign keys.
- `rr_deliveries`: immutable authentic acceptance terms; fulfillment stays in orders.
- `rr_plan_stops`, `rr_execution`: ordered pending work and frozen action state.
- `rr_contexts`, `rr_effects`: adopted content/revisions and once-only effect recognition.
- `rr_worlds`: checked allocators/world metadata, adapter projections/predictions,
  clock floor, content fingerprint, durable revision and writer fence.

IDs use a checked `numeric(20,0)` unsigned domain, preserving the complete u64 range.
Point geographies are SRID 4326 with GiST indexes. SQL checks bind indexed positions
exactly to their domain coordinate facts. Plan/assignment foreign keys defend local
integrity; dispatch validates custody, capacity, ordering and whole-state invariants.
There is no separately writable derived delivery/assignment truth.

Immutable records include `rr_history`, `rr_publications`, `rr_commands` terminal
outcomes, `rr_observations`, `rr_graphs`, `rr_traffic`, `rr_routes`,
`rr_execution_routes`, `rr_simulations`, `rr_commits` and `rr_outbox`. Triggers reject
rewriting protected evidence; pending command updates preserve canonical intent.

Route queries return opaque `route_records`. Spatial rows retain the entire original
query response, including departure/objective/traffic provenance and alternatives.
Zero-length wire LineStrings retain their original response and index as valid points.
Started execution retains its original graph/profile/traffic and route independently
of later traffic adoption. Missing/corrupt required graph or frozen traffic artifacts
block writes. Successful simulation scenarios/results are stored in an isolated run
record and do not mutate live domain state.

Domain checkpoint, adapter, graph, migration, policy, algorithm and outbox schemas
have separate versions. Restoration checks invariants, allocators, namespace,
plan/execution ownership, source ordering and canonical fingerprints before enabling
writes. Sealed evaluated proposals do not acquire a public Deserialize bypass.
Recorded decision context and actual accepted/committed evidence remain distinct.
Historical inspection reads retained facts; recomputation requires original retained
artifacts and compatible implementations. No automatic modern-data substitution exists.

## Outbox boundary

Every required obligation is inserted in the same transaction as its originating
commit. Identity is namespace + storage revision + ordinal, with original fence,
operational revision, time domain, actual gate sample, content fingerprints and payload.
Operational payloads retain immutable committed fact changes; command payloads retain
the exact finalized original intent/result. Event delivery can reuse the same identity
and body in Phase 22. Storage revisions may advance for recordings without an event;
they are not a contiguous consumer message sequence.

There is no publisher, broker acknowledgement, consumer inbox, integration side effect
or end-to-end exactly-once claim in this phase.

## Migrations, retention and restoration policy

Migration `0001_operational.sql` is checksum-pinned and transactional. Migrations
require exclusive global exclusion; running writers prevent them. Unknown versions,
modified migration text, database/cluster identity changes or authority-table OID changes
reject. Future upgrades must be explicit offline meaning-preserving migrations with
verified restoration; there is no destructive/down migration or online mixed-writer mode.

Acknowledged recognition and evidence have no automatic expiry or deletion. Keep graph
and traffic artifacts required by active execution. Optional historical replay absence
may be reported narrowly; missing critical restoration input blocks writes.

The independent anchor is part of the acknowledgement contract. Retain it independently
of database backups; do not roll it back with a database snapshot. Deleting it causes
an existing world to refuse writes. An older database head cannot satisfy its newer
acknowledged watermark. Optional `ROADRUNNER_EXPECTED_HEAD` adds an externally retained
`{"revision":123,"digest":"..."}` ancestor requirement; it cannot weaken the anchor.

Supported recovery is a validated single-primary restart or an offline restoration
that proves the acknowledged head and preserves exclusion. Logical database/table
clones reject installation binding. Do not run writable physical clones or copy/roll
back the independent anchor to authorize them. Independent physical installations are
not globally fenced by local PostgreSQL locks; cross-host failover/election and physical
clone orchestration are unsupported. A lost anchor or unprovable backup requires
operator investigation; no fresh namespace can erase acknowledged external effects.

## Verification

Run against a dedicated disposable database, with suites sequential because migrations
and injected SQL triggers require controlled exclusion:

```bash
cargo fmt --all -- --check
cargo +1.99.0 clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo +1.99.0 test --workspace --all-features --locked
export ROADRUNNER_TEST_DATABASE_URL='host=/tmp dbname=roadrunner_test'
cargo +1.99.0 test -p roadrunner-storage --test postgres --locked -- --ignored --test-threads=1
cargo +1.99.0 build -p roadrunner-api --locked
python3 scripts/verify_postgres_runtime.py
cargo +1.99.0 build -p roadrunner-api --release --locked
python3 scripts/collect_http_api_benchmark.py \
  --database-url "$ROADRUNNER_TEST_DATABASE_URL" \
  --output benchmarks/results/phase-21-durable-http.json
```

CI has a real PostgreSQL/PostGIS job that runs the opt-in storage tests and actual-process
fault driver. See [completion/evidence](phase-21-completion.md). Measurements include
client/HTTP/JSON/SQL/WAL/anchor work on a tiny fixture; they are not a production SLO.
