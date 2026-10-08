# Phase 19 dynamic recovery scenarios (schema 4)

Every fixture uses Roadrunner's inline synthetic graph. Run from repository root:

```bash
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/offline-recovery.json
cargo +1.99.0 run -p roadrunner-cli --locked -- simulate data/fixtures/phase-19/budget-exhaustion.json --json
```

| Fixture | Expected behavior |
| --- | --- |
| offline-recovery | A's active pickup stays with rider 1; unstarted B moves to rider 2; both deliver |
| optional-recovery | Healthy committed B moves to a newly available rider for positive penalized road saving |
| churn-suppression | Same physical opportunity with explicit large churn penalties keeps original owners |
| frozen-wait / frozen-service | Active A waiting/service stays pinned while B changes owner |
| cancellation | Active A cancellation refused; unstarted B cancelled; onboard A cancellation refused |
| no-recovery | No available recipient; scoped NoRecovery, existing obligations continue |
| observed-road-delay | Pinned path/destination delayed; old arrival superseded; cumulative realized breach recorded |
| hard-deadline-breach | Valid original hard admission followed by an observed delay; no safe repair; realized breach |
| unavailable-readiness | Active prediction no longer usable; typed recovery failure, actual execution continues |
| forecast-update | New forecast has fresh generation time; actual readiness remains authoritative |
| traffic-change | New routing context triggers recovery; departed leg remains pinned |
| budget-exhaustion | Initial admission completes; later recovery exhausts budget with no incumbent publication |

`RecoveryPolicy` values are explicit scenario inputs, never universal defaults.
Offline means unavailable for new work; these scenarios do not model a physically
immobilized vehicle. Terminal cancelled orders have no stops/assignment and retain
historical acceptance records if previously admitted. They are not delivered,
unassigned, or outstanding. Schema 4 utilization uses full configured fleet-window
responsibility exposure. These tiny graphs establish correctness and reproducibility,
not production scale or arbitrary-resequencing global optimality.
