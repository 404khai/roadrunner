//! Independently versioned storage reconstruction; never a public writable state clone.
use super::{
    ActionId, ActiveExecution, AdoptedContext, AppliedEffect, AppliedExecutionEffectId,
    OperationalRevision, OperationalState, OperationalTransitionRecord, PlanRevision, RiderId,
};
use crate::{CommitError, DispatchInstant, PublicationProvenance, World, WorldData, WorldVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Corrupt, unsupported or inconsistent durable authority. Restoration fails closed.
#[derive(Debug, thiserror::Error)]
pub enum CheckpointError {
    /// Malformed versioned record.
    #[error("invalid operational checkpoint: {0}")]
    Encoding(#[from] serde_json::Error),
    /// Schema, digest, compatibility mode, history or allocators are invalid.
    #[error("unsupported or corrupt operational checkpoint")]
    Invalid,
    /// Domain/active execution invariants reject restoration.
    #[error(transparent)]
    Domain(#[from] CommitError),
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredWorld {
    identity: u64,
    version: WorldVersion,
    data: WorldData,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredState {
    world: StoredWorld,
    revision: OperationalRevision,
    plans: BTreeMap<RiderId, PlanRevision>,
    execution: BTreeMap<RiderId, ActiveExecution>,
    contexts: BTreeMap<String, AdoptedContext>,
    effects: BTreeMap<AppliedExecutionEffectId, AppliedEffect>,
    next_action: ActionId,
    next_attempt: u64,
    next_effect: AppliedExecutionEffectId,
    next_order: u64,
    next_rider: u64,
    recovery_at: Option<DispatchInstant>,
    history: Vec<OperationalTransitionRecord>,
    publications: Vec<PublicationProvenance>,
}
impl OperationalState {
    /// Exports the complete version-1 durable storage value, including allocators,
    /// recognition, frozen work and immutable publication/history evidence.
    /// This is a trusted storage format, never an HTTP input or an event replay.
    /// # Errors
    /// Rejects compatibility/simulation authority or invalid state.
    pub fn checkpoint(&self) -> Result<serde_json::Value, CheckpointError> {
        if self.compatibility {
            return Err(CheckpointError::Invalid);
        }
        self.validate()?;
        let state = StoredState {
            world: StoredWorld {
                identity: self.world.identity(),
                version: self.world.version(),
                data: self.data().clone(),
            },
            revision: self.revision,
            plans: self.plans.clone(),
            execution: self.execution.clone(),
            contexts: self.contexts.clone(),
            effects: self.effects.clone(),
            next_action: self.next_action,
            next_attempt: self.next_attempt,
            next_effect: self.next_effect,
            next_order: self.next_order,
            next_rider: self.next_rider,
            recovery_at: self.recovery_at,
            history: self.history.clone(),
            publications: self.publications.clone(),
        };
        let payload = serde_json::to_value(state)?;
        Ok(
            serde_json::json!({"schema_version":1,"semantics":"roadrunner.operational-storage/v1","state":payload}),
        )
    }
    /// Restores a complete storage record and validates domain, execution, counter
    /// and history invariants before exposing an operational authority.
    /// The storage adapter separately verifies transaction/fence/content integrity.
    /// # Errors
    /// Rejects unknown schema/semantics, corrupt state or allocator reuse.
    pub fn restore_checkpoint(record: &serde_json::Value) -> Result<Self, CheckpointError> {
        if record["schema_version"] != 1
            || record["semantics"] != "roadrunner.operational-storage/v1"
        {
            return Err(CheckpointError::Invalid);
        }
        let s: StoredState = serde_json::from_value(record["state"].clone())?;
        let state = Self {
            world: World::restore(s.world.identity, s.world.version, s.world.data)?,
            compatibility: false,
            revision: s.revision,
            plans: s.plans,
            execution: s.execution,
            contexts: s.contexts,
            effects: s.effects,
            next_action: s.next_action,
            next_attempt: s.next_attempt,
            next_effect: s.next_effect,
            next_order: s.next_order,
            next_rider: s.next_rider,
            recovery_at: s.recovery_at,
            history: s.history,
            publications: s.publications,
        };
        state.validate()?;
        let data = state.data();
        if data.orders.keys().any(|id| id.value() >= state.next_order)
            || data
                .profiles
                .keys()
                .any(|id| id.value() >= state.next_rider)
            || state.plans.keys().ne(data.profiles.keys())
            || state
                .execution
                .values()
                .any(|a| a.action_id >= state.next_action || a.id >= state.next_attempt)
            || state.effects.iter().any(|(id, e)| {
                *id != e.id || *id >= state.next_effect || e.action >= state.next_action
            })
            || state.contexts.iter().any(|(name, c)| {
                name != &c.identity.category
                    || c.identity.content.is_empty()
                    || c.revision.value() == 0
            })
        {
            return Err(CheckpointError::Invalid);
        }
        let mut revision = 0;
        for h in &state.history {
            if h.before.namespace != state.revision.namespace
                || h.after.namespace != state.revision.namespace
                || h.before.number != revision
                || h.after.number != revision.checked_add(1).ok_or(CheckpointError::Invalid)?
            {
                return Err(CheckpointError::Invalid);
            }
            revision = h.after.number;
        }
        if revision != state.revision.number
            || state.publications.iter().any(|p| {
                p.schema_version != 1
                    || p.validity_policy != crate::STATIC_MONOTONE_POLICY
                    || p.committed_revision.namespace != state.revision.namespace
                    || p.committed_revision.number > state.revision.number
                    || p.committed_at > p.valid_until
                    || p.committed_at < p.evaluated_at
            })
        {
            return Err(CheckpointError::Invalid);
        }
        Ok(state)
    }
    /// Stamps staged new acceptance and publication records with the storage gate
    /// clock sample. The original reference/policy and existing acceptance stay fixed.
    /// Must run under the storage namespace lock before durable publication.
    /// # Errors
    /// Rejects changed namespace, time regression or certificate/reference expiry.
    pub fn stamp_storage_commit(
        &mut self,
        previous: &Self,
        at: DispatchInstant,
    ) -> Result<(), CheckpointError> {
        if self.revision.namespace != previous.revision.namespace
            || self.publications.len() < previous.publications.len()
        {
            return Err(CheckpointError::Invalid);
        }
        if self.world.identity() != previous.world.identity()
            || !self.history.starts_with(&previous.history)
            || !self.publications.starts_with(&previous.publications)
            || previous
                .data()
                .accepted
                .iter()
                .any(|(id, terms)| self.data().accepted.get(id) != Some(terms))
        {
            return Err(CheckpointError::Invalid);
        }
        if self.publications.len() == previous.publications.len()
            && self.data().accepted.len() != previous.data().accepted.len()
        {
            return Err(CheckpointError::Invalid);
        }
        for p in &self.publications[previous.publications.len()..] {
            if p.source != previous.revision || at < p.committed_at || at > p.valid_until {
                return Err(CheckpointError::Invalid);
            }
        }
        self.world.stamp_new_acceptance(previous.data(), at)?;
        if self.recovery_at != previous.recovery_at && self.recovery_at.is_some() {
            self.recovery_at = Some(at);
        }
        let payload = serde_json::json!({"world":self.data(),"execution":self.execution(),"plan_revisions":self.plan_revisions(),"recovery_at":self.recovery_at()});
        for p in &mut self.publications[previous.publications.len()..] {
            p.committed_at = at;
            p.commitment_record.committed_at = Some(at);
            p.decision_record.committed_at = Some(at);
            p.commitment_record.payload = payload.clone();
        }
        self.validate()?;
        Ok(())
    }
}
