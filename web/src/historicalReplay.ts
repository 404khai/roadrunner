import { parseExact } from "./contracts";
import { decodeScene, type Scene } from "./deliveryScene";

export const MAX_REPLAY_BYTES = 8 * 1024 * 1024;

// A saved scene carries its own graph and directed geometry. Do not bind it to
// whichever (possibly different) graph happens to be connected today.
export function decodeReplay(raw: string): Scene {
  if (new TextEncoder().encode(raw).byteLength > MAX_REPLAY_BYTES)
    throw Error("Replay exceeds the 8 MiB scene limit");
  const header = parseExact(raw) as { graph_digest?: unknown } | null;
  if (!header || typeof header.graph_digest !== "string")
    throw Error(
      "Unsupported replay: expected downloaded delivery scene evidence",
    );
  const scene = decodeScene(raw, header.graph_digest);
  if (scene.frames[0].at !== scene.start)
    throw Error("Replay is missing its initial boundary");
  return scene;
}

export function replayEvents(scene: Scene) {
  const events: { at: number; label: string }[] = [];
  scene.frames.forEach((frame, i) => {
    const previous = scene.frames[i - 1];
    const add = (label: string) => events.push({ at: frame.at, label });
    for (const order of frame.orders) {
      const before = previous?.orders.find((o) => o.id === order.id);
      if (!before) add(`Order ${order.id} created`);
      if (order.rider !== null && order.rider !== before?.rider)
        add(`Order ${order.id} assigned to rider ${order.rider}`);
      if (before && order.rider === null && before.rider !== null)
        add(`Order ${order.id} unassigned`);
      if (order.status !== before?.status && order.status !== "awaiting_pickup")
        add(`Order ${order.id} ${order.status}`);
    }
    if (
      previous &&
      scene.trafficSupported &&
      frame.traffic_digest !== previous.traffic_digest
    )
      add("Synthetic traffic context changed");
    for (const rider of frame.riders) {
      const before = previous?.riders.find((r) => r.id === rider.id);
      if (before && rider.plan_revision !== before.plan_revision)
        add(`Rider ${rider.id} plan revised`);
      if (
        rider.execution &&
        rider.execution.action_id !== before?.execution?.action_id
      )
        add(`Rider ${rider.id} started action ${rider.execution.action_id}`);
      if (
        before?.execution &&
        rider.execution?.action_id === before.execution.action_id &&
        rider.execution.generation !== before.execution.generation
      )
        add(`Rider ${rider.id} action schedule changed`);
    }
  });
  return events;
}
