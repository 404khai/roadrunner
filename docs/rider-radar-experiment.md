# Rider radar experiment

Branch: `feat/web-radar-rider-selection`.

The dispatch seeking presentation uses the [React Bits Radar](https://reactbits.dev/tools/background-studio?bg=radar)
component, anchored at the submitted pickup. Its generated contacts/clutter and mouse
interaction are disabled. Geographic eligibility and engine recommendations remain
retained evidence; the sweep is a presentation metaphor.

A controlled sweep completes during the first 75% of the existing presentation.
Each actual projected rider tag emits a 550 ms ring when crossed, once per replay.
The remaining presentation interval lets those pulses finish before the picked
rider's star appears directly above their tag. The star persists after completion;
manual inspection does not change which rider was recommended. Pause/replay/skip
follow the existing controls. Reduced motion skips the sweep and reveals the result.
A result with no selected rider never receives a star.

The vendored component retains its license and source attribution in
`web/src/vendor/reactbits/`. Application adaptations provide controlled progress and
sweep notifications, and release GL resources without explicitly losing a context
that React StrictMode immediately reuses.

Browser verification on the real local dispatch demo: all six rider tags emitted
pulses with zero recommendation stars present; after completion, rider 2 alone had
a visible gold star whose bounding box was entirely above the rider tag. The shader
context remains valid after StrictMode mounting. Production build/lint and the web
regression suite are checked separately.
