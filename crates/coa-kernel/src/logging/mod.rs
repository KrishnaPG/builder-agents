use crate::error::LogError;
use crate::types::{AutonomyLevel, DirectiveProfileHash, EventId, NodeId};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: EventId,
    pub timestamp: u64,
    pub node_id: NodeId,
    pub autonomy_level: AutonomyLevel,
    pub directive_hash: DirectiveProfileHash,
    pub action: String,
    pub result: String,
    pub prev_hash: [u8; 32],
    pub hash: [u8; 32],
}

#[derive(Debug, Default)]
pub struct EventLog {
    inner: Mutex<Vec<Event>>,
}

impl EventLog {
    pub fn append(&self, mut event: Event) -> Result<EventId, LogError> {
        let mut guard = self.inner.lock();
        let prev_hash = guard.last().map(|e| e.hash).unwrap_or([0u8; 32]);
        event.prev_hash = prev_hash;
        event.hash = compute_hash(&event);
        guard.push(event.clone());
        Ok(event.event_id)
    }

    pub fn events(&self) -> Vec<Event> {
        self.inner.lock().clone()
    }

    pub fn verify_integrity(&self) -> Result<(), LogError> {
        let guard = self.inner.lock();
        let mut prev = [0u8; 32];
        for e in guard.iter() {
            if e.prev_hash != prev {
                return Err(LogError::IntegrityViolation);
            }
            let expected = compute_hash(e);
            if e.hash != expected {
                return Err(LogError::IntegrityViolation);
            }
            prev = e.hash;
        }
        Ok(())
    }
}

fn compute_hash(event: &Event) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(event.event_id.0.as_bytes());
    hasher.update(&event.timestamp.to_le_bytes());
    hasher.update(event.node_id.0.as_bytes());
    hasher.update([event.autonomy_level.as_u8()]);
    hasher.update(&event.directive_hash.0);
    hasher.update(event.action.as_bytes());
    hasher.update([0]);
    hasher.update(event.result.as_bytes());
    hasher.update([0]);
    hasher.update(&event.prev_hash);
    let out = hasher.finalize();
    out.into()
}
