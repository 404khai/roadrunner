# Phase 23 adversarial proof mapping

This maps Phase 23 obligations in the signed audit Section 23 to executable evidence.
Test expectations here are not test execution claims. Actual runs and counts are in
[completion evidence](evidence/phase-23-verification.json).

| Audit cases | Executable evidence | Publication rule |
|---|---|---|
| 1, 2, 16, 22, 30, 32: future/adoption/readiness/aggregate leakage | `future_traffic_readiness_delay_arrival_and_horizon_do_not_change_original_features`, `same_second_later_adoption_and_later_generation_cannot_impersonate_original_inputs`; typed `extract_features(StartedAction, original_graph)` boundary; complete source replay rejects injected source fields | No scenario, label, schedule, latest state or aggregate input exists on the predictor boundary; captured original traffic digest must match |
| 3, 18: route-generated label versus physical evidence | `independent_all_started_oracle_includes_two_legs_and_original_b0`, `incompatible_clocks_and_physical_silence_never_get_synthetic_labels_or_bounds` | SyntheticControlled cohort; physical evaluation unavailable |
| 4, 28: duplicates, effect aliases, stale generations | `duplicate_arrival_aliases_do_not_multiply_census`, `wrong_owner_hole_and_pending_evidence_fail_census`; existing simulator `repeated_current_move_cannot_double_distance_or_advance_stage` and obsolete-token test; dispatch/runtime suites | One canonical action; one accepted endpoint; stale attempts remain pending evidence without becoming labels |
| 5, 7, 23, 24, 29: corrections, cancellation, late/invalid/zero endpoints | `rejected_correction_does_not_replace_original_endpoint_and_new_dispute_version_is_explicit`, `cancellation_cannot_erase_already_started_action`, `label_partition_zero_negative_pending_missing_dispute_and_late` | Immutable as-of assessments; original accepted endpoints persist; no zero imputation/clamping |
| 6: no committed start | `never_started_and_future_orders_do_not_create_fake_legs`; extended simulator repeated-move test explicitly attempts an invalid second road start and checks unchanged allocator/start ledger | No fabricated ActionId or counterfactual leg |
| 8, 10, 12: original B0, delays, frozen suffix | `delay_generations_preserve_original_ten_second_b0`, `reassignment_and_frozen_replacement_retain_actual_started_owner`, `future_traffic_readiness_delay_arrival_and_horizon_do_not_change_original_features`; baseline behavior comparison | Original route remains unchanged; later generations are lineage only |
| 9, 15: ownership/policy changes | reassignment test above; source policy coverage and split tests; Phase 19 fixture replay | Join actual committed owner; selected policy is metadata, not a predictor or causal label |
| 11, 31: graph semantics/archival loss | `source_bytes_missing_graph_and_legacy_semantics_fail_publication`, `archival_loss_is_not_predictor_time_absence_or_modern_backfill` | Typed missing optional features; corrupt/missing required source artifact blocks source completeness; never modern rerouting |
| 12–14, 20, 25, 27 in decision register; proof cases 12–15: ancestry/episodes/replay | `dependency_closure_rejects_policy_replay_episode_and_transitive_leakage`, `reused_namespace_and_replay_are_run_qualified_but_not_independent`, `exact_split_reproducibility_and_exclusions_require_reasons` | Exact whole-component assignments; insufficient groups explicitly unavailable; common generator cannot support a mechanism-shift claim |
| 21: incompatible clocks/physical start unsupported | incompatible clock test above | No physical/report label or silence-derived censor bound |
| 25: active/pending horizon | `horizon_start_denominator_is_not_completed_movements`, `arrival_without_pickup_or_fulfillment_is_an_exact_label` | All starts count; replay/queue/active evidence supports synthetic-only censoring |
| 26: restart while active | bundle publication/load roundtrip preserves saved source action/run/namespace; existing storage/process SIGKILL restoration suites preserve action/generation/original route | No simulator mid-run resume capability is claimed; retained complete source can replay without changing identity |
| 27: reused fixture namespace | `reused_namespace_and_replay_are_run_qualified_but_not_independent` | Run-qualified identities; related runs grouped |
| Reproducibility, source corruption, incomplete census | `source_corruption_blocks_even_when_digest_is_recomputed`, `semantic_reproducibility_and_changed_boundaries_are_distinct_versions`, `corrupted_features_quality_splits_and_label_cannot_publish`, `immutable_publication_roundtrip_rejects_overwrite_and_corruption`; independent Python source oracle | Verify raw bytes, source semantics/replay and every derived component before publication |
| Source schema/units/numeric corruptions relevant to dataset | `malformed_units_nan_infinity_and_unsupported_source_schema_reject` | Reject invalid typed values and unsupported source schema |

The independent Python oracle reads retained source starts, allocator, effects, pending
queue and original route, then checks dataset joins/counts/B0/endpoints and hashes. The
small Rust graph oracle asserts hand-known start/arrival times (0→10 and 25→55), not an
expected dataset generated by the extractor. The behavior verifier compares every old
field with a binary built directly from audited commit `acfd809`; it removes only the
new top-level trace and never regenerates goldens.

Proof cases 17, 19 and evaluator-only portions of 29–30 belong to Phase 24 or later.
Model-admission cases 20 and 33–40 are deferred with their introducing capabilities.
No evaluator, learned inference, shadow runtime, physical source adapter or causal policy
evaluation is implemented here. Unsupported physical-clock behavior is tested as a refusal,
not as a successful physical measurement capability.
