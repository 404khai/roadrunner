# Historical replay (Phase 32)

Phase 30 already implemented deterministic scene sampling, play/pause, restart,
next-event stepping, logical-time scrubbing, riders, orders, traffic and accepted
execution. Phase 31 supplied retained dispatch inspection. Phase 32 reuses these
producers and closes the missing saved-run workflow.

## Use

Open `/workspace?mode=delivery`. Run a simulation and download **Scene evidence**.
Later, expand **Delivery simulation → Open saved replay** and select that JSON.
An engine connection is unnecessary for opening a saved run. Its embedded road graph,
original directed routes and graph identity are used even if the current server has
a different graph. The contextual basemap still needs network access.

Imported runs start paused at their initial boundary. New runs and restarts reset
to **1×**. Playback offers **1× / 2× / 5× / 10×**, pause, restart, next-event,
a scrubber and exact logical seconds. Play at the end restarts at 1×. Pause/resume
preserves a speed explicitly chosen during the current playback. The landing page's
delivery demo also advances logical time at 1×; route and dispatch line reveals
remain presentation animations.

Expand **Orders → Replay events** for the whole-run index. Selecting a change
seeks to its retained timestamp and pauses. It lists order creation, assignment,
unassignment, custody/status changes and delivery, synthetic traffic-context
changes, plan revisions, action starts and schedule changes. Plan revisions are
not automatically labeled reroutes: a suffix edit may preserve the active action.
Movement and changed subsequent routes are sampled by the existing map.

## Determinism and evidence limits

Replay is read-only sampling of a captured finite simulation, with no API request
on import, seek or playback. It does not rerun dispatch or reconstruct decisions
from current configuration. The original source bytes remain downloadable.
Scene/graph/time/motion versions, directed geometry, action identity and causal
schedule lineage pass the existing decoder; saved runs must include the initial
boundary. Files exceeding the producer's 8 MiB response limit reject before reading.
Malformed/unsupported imports show an error and retain the previous scene, paused.
Imported JSON is evidence supplied by the user, not a cryptographically authenticated
engine artifact.

At equal timestamps the last source frame wins. The index uses captured vector order,
but clicking any change at that timestamp shows the coherent final boundary, not an
individual intermediate queue entry. The index includes future changes and is labeled
whole-run; dispatch audit inspection likewise covers the whole retained run.
Accepted arrival and completion remain authoritative; interpolation never creates
fulfillment. Delay anchors preserve earlier progress when seeking backward.
Traffic is the retained synthetic context, separate from frozen action costs and
cosmetic ambient actors. Missing geometry remains unavailable.

This supports previously exported `synthetic-delivery-scene/v1` simulation runs,
including captured dispatch history. Bare scenario JSON, ordinary simulation summaries,
standalone dispatch responses and arbitrary operational sessions do not contain enough
movement history and are not accepted. No server-side historical session store,
live GPS recording, civil-time playback or event ingestion service is introduced.
See the [producer and motion contract](live-delivery-visualization.md).

## Verification

`npm --prefix web test -- --maxWorkers=1 --testTimeout=30000` passes all
88 tests across 22 files. `npm --prefix web run build`,
`npm --prefix web run lint`, targeted Prettier checks and `git diff --check` pass.
An earlier default-worker run passed 86 tests before the final two regression
cases were added; later default/2-worker runs hit 5-second UI timeouts under
host contention. The final serial run passed without assertion changes. Replay tests reopen real engine fixtures, sample
backward/forward across delay and suffix boundaries, compare identical-time results,
check traffic transitions and reject oversized, graph-inconsistent and incomplete
artifacts. UI tests cover disconnected import, paused 1× initialization, speed reset,
scrubbing and failed-import preservation.

Running Chrome verification on localhost:5173 used the actual engine delay/suffix
scenario (nodes 0/35/22): autoplay showed 1×, seeking to 125s showed accepted service
and zero deliveries, and the 19-change event index exposed assignment, delay and
plan revision history. Collaborative preview snapshot failed twice with
`Preview automation snapshot timed out after 15000ms`; Chrome served as fallback.
Saved-file upload could not be visually exercised because the browser extension
requires its file-URL access setting; it is covered by the automated UI tests.
