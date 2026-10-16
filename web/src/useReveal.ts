import { useEffect, useRef, useState } from "react";
export function useReducedMotion() {
  const [reduced, setReduced] = useState(
    () => window.matchMedia("(prefers-reduced-motion: reduce)").matches,
  );
  useEffect(() => {
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const onChange = () => setReduced(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);
  return reduced;
}
export function useReveal(
  identity: string | undefined,
  noTravel: boolean,
  reduced: boolean,
) {
  const [progress, setProgress] = useState(1),
    [playing, setPlaying] = useState(false),
    [speed, setSpeed] = useState(1);
  const [activeIdentity, setActiveIdentity] = useState(identity);
  const elapsed = useRef(0),
    frame = useRef(0);
  useEffect(() => {
    elapsed.current = 0;
    setActiveIdentity(identity);
    setProgress(reduced || noTravel || !identity ? 1 : 0);
    setPlaying(!!identity && !reduced && !noTravel);
  }, [identity, noTravel, reduced]);
  useEffect(() => {
    if (!playing || reduced) return;
    let last: number | undefined;
    const tick = (now: number) => {
      if (last !== undefined) elapsed.current += (now - last) * speed;
      last = now;
      const p = Math.min(1, elapsed.current / 2600);
      setProgress(p);
      if (p < 1) frame.current = requestAnimationFrame(tick);
      else setPlaying(false);
    };
    frame.current = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame.current);
  }, [playing, speed, reduced, identity]);
  return {
    progress:
      activeIdentity === identity
        ? progress
        : reduced || noTravel || !identity
          ? 1
          : 0,
    playing,
    speed,
    setSpeed,
    pause: () => setPlaying(false),
    resume: () => setPlaying(true),
    skip: () => {
      elapsed.current = 2600;
      setPlaying(false);
      setProgress(1);
    },
    replay: () => {
      elapsed.current = 0;
      setProgress(reduced || noTravel ? 1 : 0);
      setPlaying(!reduced && !noTravel);
    },
  };
}
