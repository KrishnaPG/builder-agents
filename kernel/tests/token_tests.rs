use cog_kernel::autonomy::CapabilityToken;
use cog_kernel::types::{AutonomyLevel, DirectiveProfileHash, NodeId, ResourceCaps};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;

#[test]
fn test_token_signing_and_verification() {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = VerifyingKey::from(&signing_key);
    
    let node_id = NodeId::new();
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024,
        token_limit: 100,
        iteration_cap: 10,
    };
    let hash = DirectiveProfileHash([0u8; 32]);
    
    let token = CapabilityToken::sign(
        node_id, 
        AutonomyLevel::L1, 
        caps, 
        hash, 
        &signing_key
    );
    
    assert!(token.verify(&verifying_key));
}

#[test]
fn test_token_forgery_fails() {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = VerifyingKey::from(&signing_key);
    
    let node_id = NodeId::new();
    let caps = ResourceCaps {
        cpu_time_ms: 1000,
        memory_bytes: 1024,
        token_limit: 100,
        iteration_cap: 10,
    };
    let hash = DirectiveProfileHash([0u8; 32]);
    
    let mut token = CapabilityToken::sign(
        node_id, 
        AutonomyLevel::L1, 
        caps, 
        hash, 
        &signing_key
    );
    
    // Tamper with data
    token.autonomy_level = AutonomyLevel::L5;
    
    assert!(!token.verify(&verifying_key));
}
