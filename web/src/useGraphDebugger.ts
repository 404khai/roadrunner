import { useEffect, useState } from "react";
import { apiText, type RouteRequest } from "./contracts";
import {
  decodeGraph,
  type GraphSelection,
  type GraphView,
} from "./graphContracts";
export function useGraphDebugger(digest: string | undefined, token: string) {
  const [enabled, setEnabled] = useState(false),
    [view, setView] = useState<GraphView>(),
    [error, setError] = useState<string>(),
    [busy, setBusy] = useState(false);
  const [objective, setObjective] =
    useState<RouteRequest["objective"]>("travel_time");
  const [offsets, setOffsets] = useState({ node: 0, edge: 0 }),
    [refresh, setRefresh] = useState(0),
    [selection, setSelection] = useState<GraphSelection>();
  const [nodeHistory, setNodeHistory] = useState<number[]>([]),
    [edgeHistory, setEdgeHistory] = useState<number[]>([]);
  const [layers, setLayers] = useState({
    nodes: true,
    edges: true,
    directions: true,
    weights: false,
  });
  useEffect(() => {
    setOffsets({ node: 0, edge: 0 });
    setNodeHistory([]);
    setEdgeHistory([]);
    setSelection(undefined);
    setView(undefined);
  }, [digest, token]);
  useEffect(() => {
    if (!enabled || !digest) {
      setBusy(false);
      if (!digest) {
        setView(undefined);
        setError(undefined);
        setSelection(undefined);
      }
      return;
    }
    const controller = new AbortController();
    setBusy(true);
    setError(undefined);
    setView(undefined);
    setSelection(undefined);
    const q = {
      digest,
      nodeOffset: offsets.node,
      edgeOffset: offsets.edge,
      objective,
    };
    const params = new URLSearchParams({
      graph_digest: digest,
      node_offset: String(q.nodeOffset),
      edge_offset: String(q.edgeOffset),
      objective,
    });
    void apiText(`/v1/visualization/graph?${params}`, token, controller.signal)
      .then((raw) => {
        const next = decodeGraph(raw, q);
        if (!controller.signal.aborted) setView(next);
      })
      .catch((e) => {
        if (!controller.signal.aborted)
          setError(
            e instanceof Error ? e.message : "Graph inspection unavailable.",
          );
      })
      .finally(() => {
        if (!controller.signal.aborted) setBusy(false);
      });
    return () => controller.abort();
  }, [enabled, digest, token, objective, offsets, refresh]);
  function page(kind: "node" | "edge", forward: boolean) {
    const history = kind === "node" ? nodeHistory : edgeHistory,
      setHistory = kind === "node" ? setNodeHistory : setEdgeHistory;
    const next = forward
      ? view?.[kind === "node" ? "node_page" : "edge_page"].next_offset
      : history.at(-1);
    if (next === null || next === undefined) return;
    setHistory(forward ? [...history, offsets[kind]] : history.slice(0, -1));
    setOffsets((o) => ({ ...o, [kind]: next }));
  }
  return {
    enabled,
    setEnabled,
    view,
    error,
    busy,
    objective,
    setObjective,
    offsets,
    selection,
    setSelection,
    layers,
    setLayers,
    page,
    canBackNodes: nodeHistory.length > 0,
    canBackEdges: edgeHistory.length > 0,
    refresh: () => setRefresh((n) => n + 1),
  };
}
export type GraphDebugger = ReturnType<typeof useGraphDebugger>;
