use crate::autonomy::hash_execution_profile_bytes;
use crate::types::{DirectiveProfileHash, DirectiveSet, ExecutionProfile};
use std::collections::BTreeMap;

pub fn compile(directives: &DirectiveSet) -> (ExecutionProfile, DirectiveProfileHash) {
    let profile = ExecutionProfile {
        required_test_coverage_percent: directives
            .directives
            .get("required_test_coverage_percent")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(100) as u8,
        security_scan_depth: directives
            .directives
            .get("security_scan_depth")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(255) as u8,
        max_debate_iterations: directives
            .directives
            .get("max_debate_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            .min(u32::MAX as u64) as u32,
        merge_gating_policy: directives
            .directives
            .get("merge_gating_policy")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        resource_multipliers: directives
            .directives
            .get("resource_multipliers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                let mut map = BTreeMap::new();
                for (k, v) in obj {
                    map.insert(k.clone(), v.clone());
                }
                map
            })
            .unwrap_or_default(),
    };

    let bytes = serde_json::to_vec(&profile).unwrap_or_default();
    let hash = hash_execution_profile_bytes(&bytes);
    (profile, hash)
}
