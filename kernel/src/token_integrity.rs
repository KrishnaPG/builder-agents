//! Token Integrity Verification (v2.0)
//!
//! This module provides runtime integrity verification for capability tokens.
//! IMPORTANT: This is NOT policy validation - all policy checks happen at construction time.
//!
//! Runtime checks performed:
//! - Cryptographic signature verification
//! - Token expiration
//! - Node binding (token is for the correct node)

use crate::autonomy::CapabilityToken;
use crate::error::ExecutionError;
use crate::types::v2::IntegrityVerification;
use crate::types::NodeId;
use ed25519_dalek::VerifyingKey;

/// Token integrity verifier
///
/// Performs cryptographic and temporal integrity checks only.
/// No policy validation - that happened at construction time.
pub struct TokenIntegrity;

impl TokenIntegrity {
    /// Verify token integrity
    ///
    /// Checks:
    /// 1. Cryptographic signature valid
    /// 2. Token not expired
    ///
    /// Does NOT check:
    /// - Policy compliance (done at construction)
    /// - Resource limits (enforced by container)
    /// - Autonomy level (encoded in ValidatedGraph)
    pub fn verify_integrity(
        token: &CapabilityToken,
        verifying_key: &VerifyingKey,
    ) -> Result<IntegrityVerification, ExecutionError> {
        // Cryptographic signature check
        let signature_valid = token.verify(verifying_key);
        
        // Expiration check
        let not_expired = !token.is_expired();
        
        if signature_valid && not_expired {
            Ok(IntegrityVerification {
                valid: true,
                node_binding_valid: true, // Will be checked separately
                not_expired: true,
            })
        } else if !signature_valid {
            Err(ExecutionError::TokenIntegrityFailure)
        } else {
            Err(ExecutionError::TokenExpired)
        }
    }
    
    /// Verify token is bound to specific node
    ///
    /// This is an integrity check, not policy validation.
    pub fn verify_node_binding(
        token: &CapabilityToken,
        expected_node_id: NodeId,
    ) -> Result<(), ExecutionError> {
        if token.node_id == expected_node_id {
            Ok(())
        } else {
            Err(ExecutionError::TokenBindingFailure)
        }
    }
    
    /// Verify token is bound to specific operation
    ///
    /// This is an integrity check, not policy validation.
    pub fn verify_operation_binding(
        token: &CapabilityToken,
        operation: &str,
    ) -> Result<(), ExecutionError> {
        if token.is_bound_to(operation) {
            Ok(())
        } else {
            Err(ExecutionError::TokenBindingFailure)
        }
    }
    
    /// Full verification: integrity + node binding
    ///
    /// Convenience method that performs all runtime integrity checks.
    pub fn verify_full(
        token: &CapabilityToken,
        verifying_key: &VerifyingKey,
        expected_node_id: NodeId,
        operation: Option<&str>,
    ) -> Result<IntegrityVerification, ExecutionError> {
        // First verify basic integrity
        let result = Self::verify_integrity(token, verifying_key)?;
        
        // Verify node binding
        Self::verify_node_binding(token, expected_node_id)?;
        
        // Verify operation binding if specified
        if let Some(op) = operation {
            Self::verify_operation_binding(token, op)?;
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AutonomyLevel, DirectiveProfileHash, ResourceCaps};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn create_test_token(
        signing_key: &SigningKey,
        node_id: NodeId,
        expires_at: u64,
    ) -> CapabilityToken {
        CapabilityToken::sign(
            node_id,
            AutonomyLevel::L3,
            ResourceCaps {
                cpu_time_ms: 1000,
                memory_bytes: 1024 * 1024,
                token_limit: 1000,
                iteration_cap: 100,
            },
            DirectiveProfileHash([0u8; 32]),
            signing_key,
            expires_at,
            "test_operation",
        )
    }

    #[test]
    fn test_valid_token_passes_integrity() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId::new();
        
        let token = create_test_token(&signing_key, node_id, 0); // No expiration
        
        let result = TokenIntegrity::verify_integrity(&token, &verifying_key);
        assert!(result.is_ok());
        
        let verification = result.unwrap();
        assert!(verification.valid);
        assert!(verification.not_expired);
    }

    #[test]
    fn test_expired_token_fails() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId::new();
        
        // Token expired 1 hour ago
        let expired_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 3600;
        
        let token = create_test_token(&signing_key, node_id, expired_at);
        
        let result = TokenIntegrity::verify_integrity(&token, &verifying_key);
        assert!(matches!(result, Err(ExecutionError::TokenExpired)));
    }

    #[test]
    fn test_node_binding_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let correct_node_id = NodeId::new();
        let wrong_node_id = NodeId::new();
        
        let token = create_test_token(&signing_key, correct_node_id, 0);
        
        // Correct binding
        assert!(TokenIntegrity::verify_node_binding(&token, correct_node_id).is_ok());
        
        // Wrong binding
        assert!(TokenIntegrity::verify_node_binding(&token, wrong_node_id).is_err());
    }

    #[test]
    fn test_full_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId::new();
        
        let token = create_test_token(&signing_key, node_id, 0);
        
        let result = TokenIntegrity::verify_full(&token, &verifying_key, node_id, Some("test_operation"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_wrong_operation_binding_fails() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        let node_id = NodeId::new();
        
        let token = create_test_token(&signing_key, node_id, 0);
        
        let result = TokenIntegrity::verify_full(&token, &verifying_key, node_id, Some("wrong_operation"));
        assert!(matches!(result, Err(ExecutionError::TokenBindingFailure)));
    }
}
