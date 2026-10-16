# Client evidence fixtures

`catalog.json`, `forward.json`, `reverse.json`, and `no-travel.json` were captured
from the real `roadrunner-api` portable producer using the checked-in
`data/fixtures/phase-10/snapshot.semantic-v2/graph.rr-graph`. Their corresponding
request files pin the graph, exact anchors, A*, free-flow travel time and
`2026-10-16T12:00:00Z`. They contain no credentials or operational rider/order data.
Formatting of the retained JSON is normalized for review; tests preserve whatever
source bytes they consume. Forward 0→35 and reverse 35→0 independently expose the
same curved road sequence in opposite directions, with all duplicated joints.

`contracts.test.ts` exercises actual producer/client compatibility and adversarial
mutations. `presentation.test.tsx` drives a controlled animation clock.
`App.test.tsx` uses the real evidence fixtures and a controlled transport/map boundary
for lifecycle and noninterference tests. Mocked map tests do not establish rendered
fidelity; actual browser screenshots and live-engine checks are recorded separately.
