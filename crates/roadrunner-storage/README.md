# roadrunner-storage

Phase 21 PostgreSQL/PostGIS adapter for Roadrunner's operational authority.
It persists normalized current facts, checked restoration metadata, immutable
history, routes, simulation runs, command/observation recognition and outbox obligations.
It does not deliver events or implement a broker.

See [persistence contracts and run guide](../../docs/persistence.md) and
[Phase 21 completion](../../docs/phase-21-completion.md).

The public `PgStore` is synchronous. Use it on a blocking thread. Its session
lease and independent acknowledgement-anchor lock are retained until the store
is dropped or retired. No reconnection can reuse a failed writer's fence.

```bash
cargo +1.99.0 test -p roadrunner-storage --locked
ROADRUNNER_TEST_DATABASE_URL='host=/tmp dbname=roadrunner_test' \
  cargo +1.99.0 test -p roadrunner-storage --test postgres --locked \
  -- --ignored --test-threads=1
```

The opt-in suite requires a dedicated disposable PostgreSQL/PostGIS database and
runs actual rollback, writer-exclusion, restoration, corruption, rollback-watermark
and post-COMMIT acknowledgement-failure checks. CI runs it explicitly.

Phase 22 adds the public `events` module and additive delivery migration. Role-specific
leases, inbox/effects, retained ordering/reconciliation and acknowledgement anchors
support [local event processing](../../docs/events.md). Real event tests run explicitly:

```bash
cargo +1.99.0 test -p roadrunner-storage --test events --locked -- --ignored --test-threads=1
```

The upgrade fixture needs CREATE DATABASE privileges and PostgreSQL keyword connection
syntax; it creates and removes its own uniquely named disposable database.
