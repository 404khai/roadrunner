# React Bits Radar

Vendored from https://github.com/DavidHDev/react-bits/tree/c2a1408d9219a5bd7a2d9c93c9c499042f5fa34a/src/ts-default/Backgrounds/Radar
for the user-requested https://reactbits.dev/tools/background-studio?bg=radar effect.
Copyright (c) 2026 David Haz. License: [MIT + Commons Clause](LICENSE.md).
Included as part of Roadrunner's application, with attribution retained.

Source is formatted to this repository's style. The dispatch wrapper disables
generated contacts, clutter and mouse interaction; it never generates rider data.

The cleanup releases GL resources without explicitly losing the canvas context,
so React StrictMode can safely restart the effect on the same canvas.

A controlled `sweepProgress` and `onSweep` notification synchronize this visual
with the presentation timeline. Rider pulses use actual projected tag bearings,
not generated contacts, eligibility discovery order or backend computation progress.
