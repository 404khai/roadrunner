//! Explicit trusted-local asynchronous event runtime. No remote notification sending.
use roadrunner_storage::events::{ConsumerKind, EventStore, Role};
use std::{
    error::Error,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

struct Consumer {
    store: EventStore,
    next_poll: Instant,
    backoff: Duration,
}
impl Consumer {
    fn new(store: EventStore) -> Self {
        Self {
            store,
            next_poll: Instant::now(),
            backoff: Duration::from_secs(1),
        }
    }
}
struct Runtime {
    publisher: EventStore,
    projection: Consumer,
    notifications: Consumer,
}
impl Runtime {
    fn open(
        url: &str,
        key: &str,
        directory: &Path,
    ) -> Result<Self, roadrunner_storage::StorageError> {
        Ok(Self {
            publisher: EventStore::open(
                url,
                key,
                Role::Publisher,
                &directory.join("publisher.head"),
            )?,
            projection: Consumer::new(EventStore::open(
                url,
                key,
                Role::Consumer {
                    name: "projection-v1".into(),
                    kind: ConsumerKind::Projection,
                },
                &directory.join("projection-v1.head"),
            )?),
            notifications: Consumer::new(EventStore::open(
                url,
                key,
                Role::Consumer {
                    name: "notifications-v1".into(),
                    kind: ConsumerKind::Notification,
                },
                &directory.join("notifications-v1.head"),
            )?),
        })
    }
    fn tick(&mut self) -> Result<usize, roadrunner_storage::StorageError> {
        let mut work = 0;
        if let Some(delivery) = self.publisher.publish()? {
            self.publisher.acknowledge_publish(&delivery)?;
            tracing::info!(event_id=%delivery.message_id,sequence=%delivery.sequence,"event transport accepted");
            work += 1;
        }
        for consumer in [&mut self.projection, &mut self.notifications] {
            if Instant::now() < consumer.next_poll {
                continue;
            }
            if let Some(delivery) = consumer.store.next_delivery()? {
                let receipt = consumer.store.receive(&delivery)?;
                if receipt.terminal() {
                    consumer.store.acknowledge_delivery(&delivery)?;
                    consumer.backoff = Duration::from_secs(1);
                    tracing::info!(event_id=%delivery.message_id,sequence=%delivery.sequence,outcome=%receipt.state,"consumer durable outcome acknowledged");
                    work += 1;
                } else {
                    consumer.next_poll = Instant::now() + consumer.backoff;
                    consumer.backoff = (consumer.backoff * 2).min(Duration::from_secs(30));
                    tracing::warn!(event_id=%delivery.message_id,sequence=%delivery.sequence,outcome=%receipt.state,"delivery requires dependency or operator reconciliation");
                }
            }
        }
        Ok(work)
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 2
        || !matches!(
            args[0].as_str(),
            "--run" | "--once" | "--inspect" | "--replay" | "--skip"
        )
    {
        return Err("usage: roadrunner-events --run|--once|--inspect <world-key>; --replay <world-key> <projection-v1|notifications-v1>; --skip <world-key> <consumer> <message-id> <fingerprint> <reason>; ROADRUNNER_DATABASE_URL and absolute ROADRUNNER_EVENT_ANCHORS required (trusted local operator)".into());
    }
    let url = std::env::var("ROADRUNNER_DATABASE_URL")?;
    if args[0] == "--inspect" && args.len() == 2 {
        let key = args[1].clone();
        let status = tokio::task::spawn_blocking(move || EventStore::inspect(&url, &key)).await??;
        println!("{}", serde_json::to_string_pretty(&status)?);
        return Ok(());
    }
    let directory = PathBuf::from(std::env::var("ROADRUNNER_EVENT_ANCHORS")?);
    if !directory.is_absolute() {
        return Err("ROADRUNNER_EVENT_ANCHORS must be absolute and independently retained".into());
    }
    let key = args[1].clone();
    if matches!(args[0].as_str(), "--replay" | "--skip") {
        if (args[0] == "--replay" && args.len() != 3) || (args[0] == "--skip" && args.len() != 6) {
            return Err("invalid replay/reconciliation arguments".into());
        }
        let principal =
            if args[0] == "--skip" {
                Some(std::env::var("ROADRUNNER_EVENT_OPERATOR").map_err(
                    |_| "ROADRUNNER_EVENT_OPERATOR required for explicit reconciliation",
                )?)
            } else {
                None
            };
        let name = args.get(2).ok_or("consumer required")?.clone();
        let kind = match name.as_str() {
            "projection-v1" => ConsumerKind::Projection,
            "notifications-v1" => ConsumerKind::Notification,
            _ => return Err("unknown consumer semantics".into()),
        };
        let path = directory.join(format!("{name}.head"));
        let mut consumer = tokio::task::spawn_blocking(move || {
            EventStore::open(&url, &key, Role::Consumer { name, kind }, &path)
        })
        .await??;
        if args[0] == "--replay" && args.len() == 3 {
            let result = tokio::task::spawn_blocking(move || consumer.replay()).await??;
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else if args[0] == "--skip" && args.len() == 6 {
            let principal = principal.unwrap_or_default();
            let result = tokio::task::spawn_blocking(move || {
                consumer.skip(&args[3], &args[4], &principal, &args[5])
            })
            .await??;
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            return Err("invalid replay/reconciliation arguments".into());
        }
        return Ok(());
    }
    if args.len() != 2 {
        return Err("unexpected arguments".into());
    }
    let mut runtime =
        tokio::task::spawn_blocking(move || Runtime::open(&url, &key, &directory)).await??;
    if args[0] == "--once" {
        let status = tokio::task::spawn_blocking(move || {
            while runtime.tick()? > 0 {}
            runtime.publisher.status()
        })
        .await??;
        println!("{}", serde_json::to_string_pretty(&status)?);
        return Ok(());
    }
    let mut interval = tokio::time::interval(Duration::from_millis(100));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);
    let stopped = loop {
        tokio::select! {
            biased;
            result=&mut shutdown=>{break result},
            _=interval.tick()=>{
                runtime=tokio::task::spawn_blocking(move||{runtime.tick().map(|_|runtime)}).await??;
            }
        }
    };
    // postgres synchronous connections must also be dropped on a blocking thread.
    tokio::task::spawn_blocking(move || drop(runtime)).await?;
    stopped?;
    Ok(())
}
