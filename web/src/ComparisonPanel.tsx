import { useEffect, useState } from "react";
import { ChevronLeft, ChevronRight, Eye, EyeOff } from "lucide-react";
import type { AlternativeMember, ComparisonResult } from "./contracts";
import { distanceLabel, durationLabel } from "./geometry";
export const routeName = (rank: number) =>
  `Route ${String.fromCharCode(64 + rank)}`;
export function ComparisonPanel({
  comparison,
  selected,
  visible,
  onSelect,
  onToggle,
}: {
  comparison: ComparisonResult;
  selected: string;
  visible: Set<string>;
  onSelect: (id: string) => void;
  onToggle: (id: string) => void;
}) {
  const primary = comparison.members[0].route;
  const { summary } = comparison;
  return (
    <section className="comparison" aria-label="Alternative route comparison">
      <div className="section-label">
        COMPARE ROUTES{" "}
        <span>
          {comparison.members.length} / {summary.options.max_routes} RETURNED
        </span>
      </div>
      <div className="route-cards">
        {comparison.members.map((m) => {
          const r = m.route,
            active = selected === m.member_id,
            shown = visible.has(m.member_id);
          const delta = r.objective.value - primary.objective.value;
          return (
            <div
              className={`comparison-card ${active ? "selected" : ""} ${shown ? "" : "route-hidden"}`}
              key={m.member_id}
            >
              <button
                className="route-choice"
                aria-label={`Select ${routeName(m.rank)}`}
                aria-pressed={active}
                onClick={() => onSelect(m.member_id)}
              >
                <span className={`route-swatch swatch-${m.rank}`}>
                  {String.fromCharCode(64 + m.rank)}
                </span>
                <span>
                  <strong>{routeName(m.rank)}</strong>
                  <small>
                    {m.rank === 1
                      ? "ENGINE PRIMARY"
                      : `+${delta.toLocaleString(undefined, { maximumFractionDigits: 1 })} ${r.objective_unit} objective`}
                  </small>
                </span>
                {active && (
                  <span className="selected-dot" aria-label="Selected" />
                )}
              </button>
              <button
                className="visibility-button"
                aria-label={`${shown ? "Hide" : "Show"} ${routeName(m.rank)}`}
                aria-pressed={shown}
                disabled={shown && visible.size === 1}
                onClick={() => onToggle(m.member_id)}
              >
                {shown ? <Eye size={16} /> : <EyeOff size={16} />}
              </button>
              <div className="comparison-metrics">
                <span>
                  {durationLabel(r.predicted_elapsed_seconds)}
                  <small>Road ETA</small>
                </span>
                <span>
                  {distanceLabel(r.distance_meters)}
                  <small>Distance</small>
                </span>
              </div>
              <dl className="comparison-facts">
                <dt>Objective cost</dt>
                <dd>
                  {r.objective.value.toLocaleString(undefined, {
                    maximumFractionDigits: 2,
                  })}{" "}
                  {r.objective_unit}
                </dd>
                <dt>Traffic delay</dt>
                <dd>
                  {m.inspection?.traffic_delay_seconds != null
                    ? durationLabel(m.inspection.traffic_delay_seconds)
                    : r.provenance.traffic_digest
                      ? "Unavailable"
                      : "Not applied"}
                </dd>
                <dt>Algorithm</dt>
                <dd>Yen · Dijkstra</dd>
              </dl>
            </div>
          );
        })}
      </div>
      <p className="comparison-explanation">
        Route A has the lowest{" "}
        {primary.objective.kind === "distance" ? "distance" : "time"} objective
        among legal loopless paths. Your selection changes only the view.
      </p>
      <details className="comparison-scope">
        <summary>Selection rules & search limits</summary>
        <p>
          Additional routes cost at most {summary.options.max_cost_factor}× the
          primary and share at most{" "}
          {Math.round(summary.options.max_shared_distance_ratio * 100)}% of the
          shorter route’s physical distance with earlier returned routes. The
          overlap shown is physical, not directed-edge similarity.
        </p>
        <p role="status">
          {summary.truncated
            ? "Partial result: search budget reached. More alternatives may exist."
            : summary.termination === "exhausted"
              ? "Enumeration exhausted: no further legal loopless paths."
              : summary.termination === "cost_limit"
                ? "Remaining paths exceed the cost limit."
                : "Requested number of diverse routes returned."}
        </p>
        <p>
          {String(summary.ranked_paths)} complete paths ranked ·{" "}
          {String(summary.search_states)} loopless path pops across root and
          spur searches. These are not visited-node counts. Rejected candidate
          history is not retained.
        </p>
        <code>{comparison.result_set_id}</code>
      </details>
      {summary.truncated && (
        <p className="partial-result" role="status">
          Partial comparison · search budget reached
        </p>
      )}
      {comparison.members.length < summary.options.max_routes && (
        <p className="form-note">
          Fewer routes returned: {summary.termination.replaceAll("_", " ")}. No
          alternatives were fabricated.
        </p>
      )}
    </section>
  );
}
export function SegmentPanel({
  member,
  ordinal,
  onChange,
}: {
  member: AlternativeMember;
  ordinal?: number;
  onChange: (n: number | undefined) => void;
}) {
  const current = ordinal ?? 0;
  const [draft, setDraft] = useState(String(current + 1));
  useEffect(() => setDraft(String(current + 1)), [current]);
  const r = member.route,
    o = r.geometry.occurrences[current],
    d = member.inspection?.segments[current];
  return (
    <details
      className="segment-panel"
      open={ordinal !== undefined}
      onToggle={(e) => {
        onChange(e.currentTarget.open ? current : undefined);
      }}
    >
      <summary>
        Inspect selected route segments{" "}
        <span>{r.geometry.occurrences.length}</span>
      </summary>
      {!o ? (
        <p>No travel · no directed segments.</p>
      ) : (
        <>
          <div className="segment-navigation">
            <button
              aria-label="Previous segment"
              disabled={current === 0}
              onClick={() => onChange(current - 1)}
            >
              <ChevronLeft size={16} />
            </button>
            <label>
              Traversal{" "}
              <input
                aria-label="Traversal number"
                type="number"
                min={1}
                max={r.geometry.occurrences.length}
                value={draft}
                onBlur={() => setDraft(String(current + 1))}
                onChange={(e) => {
                  setDraft(e.target.value);
                  const n = Number(e.target.value);
                  if (
                    Number.isInteger(n) &&
                    n >= 1 &&
                    n <= r.geometry.occurrences.length
                  )
                    onChange(n - 1);
                }}
              />{" "}
              / {r.geometry.occurrences.length}
            </label>
            <button
              aria-label="Next segment"
              disabled={current === r.geometry.occurrences.length - 1}
              onClick={() => onChange(current + 1)}
            >
              <ChevronRight size={16} />
            </button>
          </div>
          <dl>
            <dt>Directed edge / physical segment</dt>
            <dd>
              {o.edge} / {o.segment}
            </dd>
            <dt>Direction</dt>
            <dd>
              {o.from} → {o.to} · {o.orientation}
            </dd>
            <dt>Distance</dt>
            <dd>
              {o.distance_meters.toLocaleString(undefined, {
                maximumFractionDigits: 3,
              })}{" "}
              m
            </dd>
            <dt>Objective weight</dt>
            <dd>
              {d
                ? `${d.objective_cost.toLocaleString(undefined, { maximumFractionDigits: 4 })} ${r.objective_unit}`
                : "Unavailable"}
            </dd>
            <dt>Base / evaluated road seconds</dt>
            <dd>
              {d
                ? `${d.base_seconds.toFixed(4)} / ${d.predicted_elapsed_seconds.toFixed(4)}`
                : "Unavailable"}
            </dd>
            <dt>Traffic factor</dt>
            <dd>
              {d?.traffic_factor != null
                ? `${d.traffic_factor}×`
                : r.provenance.traffic_digest
                  ? "Unavailable"
                  : "Not applied"}
            </dd>
            <dt>Geometry point interval</dt>
            <dd>
              [{o.point_start}, {o.point_end}) · directed E7
            </dd>
          </dl>
          <p>
            Selected-route occurrence only.{" "}
            {d
              ? "Weights are evaluated under this result’s original pinned static context; they are not retained search events."
              : "This producer did not expose supported per-segment evaluation evidence."}
          </p>
        </>
      )}
    </details>
  );
}
