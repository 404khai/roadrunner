# roadrunner-events

Phase 22 local asynchronous publisher and transactional integration consumers.
Uses PostgreSQL's immutable stream, checked projection and durable local notification
obligations. No broker or remote notification provider is required.

See [event contracts and run guide](../../docs/events.md) and
[completion/evidence](../../docs/phase-22-completion.md).

```bash
export ROADRUNNER_DATABASE_URL='host=/tmp dbname=roadrunner_local'
export ROADRUNNER_EVENT_ANCHORS="$PWD/.roadrunner-state/events-local"
cargo +1.99.0 run -p roadrunner-events -- --run local
cargo +1.99.0 run -p roadrunner-events -- --inspect local
```

Run the offline API migration first. Preserve the absolute event-anchor directory
independently of database backup rollback, alongside the Phase 21 operational anchor.
