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
    pub signature: Signature,
}

impl CapabilityToken {
    pub fn sign(
        node_id: NodeId,
        autonomy_level: AutonomyLevel,
        caps: ResourceCaps,
        directive_hash: DirectiveProfileHash,
        signing_key: &SigningKey,
    ) -> Self {
        let message = token_message(node_id, autonomy_level, &caps, directive_hash);
        let sig: Signature = signing_key.sign(&message);
        Self {
            node_id,
            autonomy_level,
            caps,
            directive_hash,
            signature: sig,
        }
    }

    pub fn verify(&self, verifying_key: &VerifyingKey) -> bool {
        let message = token_message(
            self.node_id,
            self.autonomy_level,
            &self.caps,
            self.directive_hash,
        );
        verifying_key.verify(&message, &self.signature).is_ok()
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
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(16 + 1 + 8 * 4 + 32);
    msg.extend_from_slice(node_id.0.as_bytes());
    msg.push(autonomy_level.as_u8());
    msg.extend_from_slice(&caps.cpu_time_ms.to_le_bytes());
    msg.extend_from_slice(&caps.memory_bytes.to_le_bytes());
    msg.extend_from_slice(&caps.token_limit.to_le_bytes());
    msg.extend_from_slice(&caps.iteration_cap.to_le_bytes());
    msg.extend_from_slice(&directive_hash.0);
    msg
}
