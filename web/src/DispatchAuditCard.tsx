import { useEffect, useState } from "react";
import type { Scene } from "./deliveryScene";
import type { Position } from "./geometry";
export type AuditFocus = { identity: string; center: Position; label: string };
export function DispatchAuditCard({
  scene,
  onFocus,
}: {
  scene?: Scene;
  onFocus: (focus: AuditFocus) => void;
}) {
  const [kind, setKind] = useState("fleet"),
    [index, setIndex] = useState(0),
    [replay, setReplay] = useState(0);
  useEffect(() => {
    setIndex(0);
    setReplay(0);
  }, [scene?.run_identity]);
  const records =
    scene?.dispatch_audit?.[
      kind as "fleet" | "insertion" | "recovery" | "basic" | "failures"
    ] ?? [];
  const record = records[index] as Record<string, unknown> | undefined;
  const decision = (record?.decision ?? record) as
    Record<string, unknown> | undefined;
  const evidence = (decision?.evidence ?? decision) as
    Record<string, unknown> | undefined;
  const coverage = (evidence?.coverage ?? evidence) as
    Record<string, unknown> | undefined;
  const order = String(
    evidence?.order ??
      (Array.isArray(evidence?.batch) ? evidence.batch[0] : ""),
  );
  const anchor = scene?.frames
    .flatMap((f) => f.orders)
    .find((o) => o.id === order);
  const committed =
    typeof record?.committed === "boolean" ? record.committed : undefined;
  return (
    <details className="delivery-dispatch-audit floating-dispatch-card">
      <summary>Dispatch audit · retained engine results</summary>
      <div className="dispatch-card-body">
        <label>
          Decision family
          <select
            value={kind}
            onChange={(e) => {
              setKind(e.target.value);
              setIndex(0);
            }}
          >
            <option value="fleet">Fleet</option>
            <option value="insertion">Insertion</option>
            <option value="recovery">Recovery</option>
            <option value="basic">Basic</option>
            <option value="failures">Unavailable predictions</option>
          </select>
        </label>
        <label>
          Retained record
          <select
            value={index}
            onChange={(e) => setIndex(Number(e.target.value))}
          >
            {records.map((_, i) => (
              <option value={i} key={i}>
                {i + 1}
              </option>
            ))}
          </select>
        </label>
        {!record ? (
          <p>
            No retained {kind} decision in this run. No candidates or selection
            are inferred.
          </p>
        ) : (
          <>
            <p>
              {kind === "basic"
                ? "Basic policy association requires its originating snapshot; use Dispatch visualization for the supported radius demo."
                : "Nonradial policy · decorative presentation only. No radius coverage or candidate discovery order."}
            </p>
            {kind === "failures" && (
              <p>
                Unavailable prediction input · no feasibility or selected
                courier is inferred.
              </p>
            )}
            <p>
              Proposal:{" "}
              {decision?.proposal
                ? "retained proposal (see original evidence)"
                : "no retained proposal"}
              . No fastest-courier claim is inferred.
            </p>
            <p>
              Publication:{" "}
              {committed === undefined
                ? "unavailable in this record"
                : committed
                  ? "committed in simulation"
                  : "not committed"}
              . Active execution is shown separately by rider ActionId.
            </p>
            <p>
              Input complete:{" "}
              {typeof coverage?.input_complete === "boolean"
                ? String(coverage.input_complete)
                : "unavailable"}{" "}
              · Search complete:{" "}
              {typeof coverage?.search_complete === "boolean"
                ? String(coverage.search_complete)
                : "unavailable"}
            </p>
            <p>
              Termination:{" "}
              {JSON.stringify(evidence?.termination ?? "unavailable")}
            </p>
            <p>
              Per-candidate hypothetical geometry/score is unavailable in
              aggregate search evidence. Rider health records are not candidate
              rankings.
            </p>
            <button
              disabled={!anchor || kind === "basic"}
              onClick={() => {
                if (!anchor || !scene) return;
                setReplay((v) => v + 1);
                onFocus({
                  identity: `${scene.run_identity}:${kind}:${index}:${replay}`,
                  center: [anchor.pickup.longitude, anchor.pickup.latitude],
                  label: `${kind} presentation anchor: order ${order} pickup · decorative, no search radius`,
                });
              }}
            >
              Present nonradial search metaphor
            </button>
            {!anchor && (
              <p>
                No associated pickup anchor retained; geographic pulse
                unavailable.
              </p>
            )}
            <details>
              <summary>Original evaluation / publication receipt</summary>
              <pre>
                {JSON.stringify(
                  record,
                  (_, v) => (typeof v === "bigint" ? String(v) : v),
                  2,
                )}
              </pre>
            </details>
          </>
        )}
      </div>
    </details>
  );
}
