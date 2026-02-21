use crate::types::{AutonomyLevel, DirectiveProfileHash, NodeId, ResourceCaps};
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, Signer, Verifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub node_id: NodeId,
    pub autonomy_level: AutonomyLevel,
    pub caps: ResourceCaps,
    pub directive_hash: DirectiveProfileHash,
    /// Unix timestamp when token was issued
    pub issued_at: u64,
    /// Token expiration timestamp (0 = no expiration)
    pub expires_at: u64,
    /// Operation this token is bound to (empty = general purpose)
    pub bound_operation: String,
    pub signature: Signature,
}

impl CapabilityToken {
    pub fn sign(
        node_id: NodeId,
        autonomy_level: AutonomyLevel,
        caps: ResourceCaps,
        directive_hash: DirectiveProfileHash,
        signing_key: &SigningKey,
        expires_at: u64,
        bound_operation: &str,
    ) -> Self {
        let issued_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let message = token_message(
            node_id, 
            autonomy_level, 
            &caps, 
            directive_hash,
            issued_at,
            expires_at,
            bound_operation,
        );
        let sig: Signature = signing_key.sign(&message);
        Self {
            node_id,
            autonomy_level,
            caps,
            directive_hash,
            issued_at,
            expires_at,
            bound_operation: bound_operation.to_string(),
            signature: sig,
        }
    }

    pub fn verify(&self, verifying_key: &VerifyingKey) -> bool {
        let message = token_message(
            self.node_id,
            self.autonomy_level,
            &self.caps,
            self.directive_hash,
            self.issued_at,
            self.expires_at,
            &self.bound_operation,
        );
        verifying_key.verify(&message, &self.signature).is_ok()
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now > self.expires_at
    }

    /// Check if token is bound to a specific operation
    pub fn is_bound_to(&self, operation: &str) -> bool {
        self.bound_operation.is_empty() || self.bound_operation == operation
    }
}

pub fn hash_execution_profile_bytes(profile_bytes: &[u8]) -> DirectiveProfileHash {
    let mut hasher = Sha256::new();
    hasher.update(profile_bytes);
    let out = hasher.finalize();
    DirectiveProfileHash(out.into())
}

fn token_message(
    node_id: NodeId,
    autonomy_level: AutonomyLevel,
    caps: &ResourceCaps,
    directive_hash: DirectiveProfileHash,
    issued_at: u64,
    expires_at: u64,
    bound_operation: &str,
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(16 + 1 + 8 * 4 + 32 + 8 + 8 + bound_operation.len());
    msg.extend_from_slice(node_id.0.as_bytes());
    msg.push(autonomy_level.as_u8());
    msg.extend_from_slice(&caps.cpu_time_ms.to_le_bytes());
    msg.extend_from_slice(&caps.memory_bytes.to_le_bytes());
    msg.extend_from_slice(&caps.token_limit.to_le_bytes());
    msg.extend_from_slice(&caps.iteration_cap.to_le_bytes());
    msg.extend_from_slice(&directive_hash.0);
    msg.extend_from_slice(&issued_at.to_le_bytes());
    msg.extend_from_slice(&expires_at.to_le_bytes());
    msg.extend_from_slice(bound_operation.as_bytes());
    msg
}
