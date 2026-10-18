# Phase 33 finite optimization evidence brief

Baseline source: `d265c05` (probe tooling on merged Phase 32). The retained raw
[baseline](../../benchmarks/results/phase33-baseline.json) records binary, manifest,
graph, compiler and host identities. No production capacity claim is made.

The tranche contains exactly two equivalent-computation changes:

1. **Snapshot-local eligibility proof reuse.** On the declared 128-rider,
   256-retained-delivered-order case, five exhaustive candidate generations have
   a median combined duration of 88.554 ms; one world validation is 0.150 ms.
   Source attribution is the candidate loop invoking complete validation per rider.
   Reuse only the exact borrowed DispatchSnapshot's proof. Keep arbitrary WorldData
   validation, rider checks, corruption errors, ordering and publication validation.
   Prove candidate equality against the defensive public helper/linear locator and
   retain invalid-world/context/clone/stamping tests. This is candidate generation,
   not total dispatch or a service p95.
2. **Immutable started-action traffic sharing.** The 1,024-node/3,968-edge grid,
   one-rider/128-order run starts 256 actions and retains 1,015,808 logical traffic
   records in 256 vectors, with 16,777,216 bytes of element capacity. This inventory
   excludes allocator headers and all other objects. Share one immutable full vector
   within an unchanged original graph-qualified context. Preserve original factors,
   order, serialization, historical dependencies and independent mutation through
   copy-on-write. Compare complete serialized result hashes, context churn,
   retained buffer sharing and adversarial independent mutation.

Measure full-process RSS separately (includes graph setup and JSON serialization).
Keep the same scenarios, source algorithms, clocks, budgets, evidence and output.
Measure each change independently before combining. Routing scratch reuse, A* setup,
spatial persistence, recovery generators, durable SQL, admission and retention-policy
changes are deferred. No new caching, expiry, archive, dispatch or authority policy.
