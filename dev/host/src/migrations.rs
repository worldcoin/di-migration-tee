//! The in-flight migration and the results of recent ones; one runs at a time by design.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex, PoisonError},
};

use bytes::Bytes;
use di_dev_enclave_types as enclave_types;
use uuid::Uuid;

use crate::enclave;

/// How many finished migrations stay collectable; each pins a whole PCP, so this bounds memory.
const RETAINED_RESULTS: usize = 4;

/// Why a migration did not produce a PCP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// The enclave could not be reached.
    EnclaveUnreachable(String),
    /// The enclave did not answer within the deadline.
    EnclaveTimeout,
    /// The enclave answered with an error.
    EnclaveRejected(enclave_types::Error),
    /// The task died without an outcome; without this a panic would strand the slot forever.
    Abandoned,
}

/// Where a migration has got to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Still running.
    Running,
    /// Finished; the migrated PCP is ready.
    Succeeded(Bytes),
    /// Finished without a PCP.
    Failed(Failure),
}

#[derive(Debug, Default)]
struct Inner {
    running: Option<Uuid>,
    states: HashMap<Uuid, State>,
    finished: VecDeque<Uuid>,
}

/// The single migration slot and the recent results.
#[derive(Debug, Default)]
pub struct Store {
    inner: Mutex<Inner>,
}

impl Store {
    /// Creates an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claims the slot, returning a guard that releases it, or `None` when one already runs.
    pub fn start(self: &Arc<Self>) -> Option<Slot> {
        let id = Uuid::new_v4();
        let mut inner = self.lock();

        if inner.running.is_some() {
            return None;
        }

        inner.running = Some(id);
        inner.states.insert(id, State::Running);
        drop(inner);

        Some(Slot {
            store: Arc::clone(self),
            id,
            settled: false,
        })
    }

    /// Returns where `id` has got to, or `None` when it is unknown or has aged out.
    #[must_use]
    pub fn state(&self, id: Uuid) -> Option<State> {
        self.lock().states.get(&id).cloned()
    }

    fn settle(&self, id: Uuid, state: State) {
        let mut inner = self.lock();

        inner.states.insert(id, state);
        inner.finished.push_back(id);
        if inner.running == Some(id) {
            inner.running = None;
        }

        while inner.finished.len() > RETAINED_RESULTS {
            if let Some(evicted) = inner.finished.pop_front() {
                inner.states.remove(&evicted);
            }
        }

        drop(inner);
    }

    /// Nothing under the lock can be half-written, so recovering beats refusing every migration.
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// A claim on the migration slot. Releases it on drop, however the task ends.
#[derive(Debug)]
pub struct Slot {
    store: Arc<Store>,
    id: Uuid,
    settled: bool,
}

impl Slot {
    /// The id clients poll for this migration.
    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }

    /// Records the migrated PCP and releases the slot.
    pub fn succeed(mut self, pcp: Bytes) {
        self.settled = true;
        self.store.settle(self.id, State::Succeeded(pcp));
    }

    /// Records a failure and releases the slot.
    pub fn fail(mut self, failure: Failure) {
        self.settled = true;
        self.store.settle(self.id, State::Failed(failure));
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        if self.settled {
            return;
        }

        tracing::error!(
            migration_id = %self.id,
            "migration task ended without an outcome; releasing the slot"
        );
        self.store
            .settle(self.id, State::Failed(Failure::Abandoned));
    }
}

impl From<&enclave::Error> for Failure {
    fn from(error: &enclave::Error) -> Self {
        match error {
            enclave::Error::Timeout => Self::EnclaveTimeout,
            enclave::Error::Transport(detail) => Self::EnclaveUnreachable(detail.clone()),
            enclave::Error::Operation(operation) => Self::EnclaveRejected(*operation),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bytes::Bytes;

    use super::{Failure, RETAINED_RESULTS, State, Store};

    fn store() -> Arc<Store> {
        Arc::new(Store::new())
    }

    #[test]
    fn a_fresh_id_is_unknown() {
        assert_eq!(store().state(uuid::Uuid::new_v4()), None);
    }

    #[test]
    fn only_one_migration_runs_at_a_time() {
        let store = store();
        let first = store.start().expect("slot is free");

        assert!(store.start().is_none(), "second claim should be refused");

        first.succeed(Bytes::from_static(b"pcp"));
        assert!(store.start().is_some(), "slot should be free again");
    }

    #[test]
    fn a_finished_migration_is_collectable() {
        let store = store();
        let slot = store.start().expect("slot is free");
        let id = slot.id();

        assert_eq!(store.state(id), Some(State::Running));
        slot.succeed(Bytes::from_static(b"pcp"));

        assert_eq!(
            store.state(id),
            Some(State::Succeeded(Bytes::from_static(b"pcp")))
        );
    }

    /// A panicking task must not strand the slot, or every later submission is refused.
    #[test]
    fn dropping_a_slot_unsettled_fails_the_migration() {
        let store = store();
        let slot = store.start().expect("slot is free");
        let id = slot.id();

        drop(slot);

        assert_eq!(store.state(id), Some(State::Failed(Failure::Abandoned)));
        assert!(store.start().is_some(), "slot should be free again");
    }

    /// Each outcome keeps its identity so `ApiError` can give the three different statuses.
    #[test]
    fn every_enclave_error_maps_to_its_own_failure() {
        use di_dev_enclave_types as enclave_types;

        use crate::enclave;

        assert_eq!(
            Failure::from(&enclave::Error::Timeout),
            Failure::EnclaveTimeout
        );
        assert_eq!(
            Failure::from(&enclave::Error::Transport("boom".to_owned())),
            Failure::EnclaveUnreachable("boom".to_owned())
        );
        assert_eq!(
            Failure::from(&enclave::Error::Operation(enclave_types::Error::Internal)),
            Failure::EnclaveRejected(enclave_types::Error::Internal)
        );
    }

    #[test]
    fn results_past_the_retention_window_age_out() {
        let store = store();
        let mut ids = Vec::new();

        for _ in 0..=RETAINED_RESULTS {
            let slot = store.start().expect("slot is free");
            ids.push(slot.id());
            slot.succeed(Bytes::from_static(b"pcp"));
        }

        assert_eq!(store.state(ids[0]), None, "oldest should have aged out");
        assert!(store.state(ids[RETAINED_RESULTS]).is_some());
    }
}
