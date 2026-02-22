//! Functional tests for security tokens, autonomy ceilings, and validation tokens.
//!
//! These tests focus on the security-related types and invariants:
//! - AutonomyCeiling enforces maximum autonomy levels.
//! - ValidationToken carries cryptographic proof and expiry.
//! - SystemLimits encode global bounds for construction validation.

use coa_kernel::types::{AutonomyCeiling, AutonomyLevel, ResourceCaps, SystemLimits};
use coa_kernel::types::v2::ValidationToken;
use ed25519_dalek::{Signature, VerifyingKey};

/// Tenet: autonomy ceiling restricts allowed autonomy levels.
///
/// Levels greater than the configured ceiling must be rejected by `check`.
#[test]
fn autonomy_ceiling_restricts_levels() {
    let ceiling = AutonomyCeiling {
        max_level: AutonomyLevel::L3,
    };

    assert!(ceiling.check(AutonomyLevel::L0));
    assert!(ceiling.check(AutonomyLevel::L3));
    assert!(!ceiling.check(AutonomyLevel::L4));
    assert!(!ceiling.check(AutonomyLevel::L5));
}

/// Tenet: default system limits provide generous but finite bounds.
///
/// This test does not enforce specific numeric values beyond sanity checks,
/// but it ensures the defaults are internally consistent and non-zero where
/// expected.
#[test]
fn default_system_limits_are_sane() {
    let limits = SystemLimits::default();

    assert!(limits.max_nodes > 0);
    assert!(limits.max_edges > 0);
    assert!(limits.max_resources.cpu_time_ms > 0);
    assert!(limits.max_resources.memory_bytes > 0);
    assert!(limits.max_resources.token_limit > 0);
}

/// Tenet: validation tokens can represent non-expiring and expiring proofs.
///
/// A token with expires_at = 0 is considered non-expiring; others should be
/// treated as expired once the system clock passes `expires_at`.
#[test]
fn validation_token_expiry_semantics() {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();

    let non_expiring = ValidationToken {
        graph_id: coa_kernel::types::GraphId(uuid::Uuid::new_v4()),
        validation_hash: [0u8; 32],
        timestamp: now,
        expires_at: 0,
        signature: dummy_signature(),
    };

    assert!(!non_expiring.is_expired());

    let expired = ValidationToken {
        graph_id: coa_kernel::types::GraphId(uuid::Uuid::new_v4()),
        validation_hash: [0u8; 32],
        timestamp: now.saturating_sub(3600),
        expires_at: now.saturating_sub(1),
        signature: dummy_signature(),
    };

    assert!(expired.is_expired());
}

/// Helper: construct a dummy ed25519 signature.
///
/// The kernel treats ValidationToken as a carrier of signatures; tests here do
/// not verify signature correctness, only that the type can be constructed and
/// used without panicking.
fn dummy_signature() -> Signature {
    Signature::from_bytes(&[0u8; 64])
}

