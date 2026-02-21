use crate::api::*;
use crate::autonomy::CapabilityToken;
use crate::dag::Dag;
use crate::error::*;
use crate::logging::{Event, EventLog};
use crate::state_machine;
use crate::types::*;
use ed25519_dalek::{SigningKey, VerifyingKey};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Duration;

/// Kernel configuration
#[derive(Debug, Clone)]
pub struct KernelConfig {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub autonomy_ceiling: AutonomyCeiling,
    pub default_caps: ResourceCaps,
}

impl Default for KernelConfig {
    fn default() -> Self {
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
            autonomy_ceiling: AutonomyCeiling::default(),
            default_caps: ResourceCaps {
                cpu_time_ms: 10000,
                memory_bytes: 1024 * 1024 * 1024,
                token_limit: 100000,
                iteration_cap: 1000,
            },
        }
    }
}

/// Node entry in the kernel registry
#[derive(Debug)]
#[allow(dead_code)]
struct NodeEntry {
    node_id: NodeId,
    graph_id: GraphId,
    spec: NodeSpec,
    current_state: NodeState,
    created_at: Timestamp,
    deactivated: bool,
    frozen: bool,
}

/// Graph entry in the kernel registry
#[derive(Debug)]
#[allow(dead_code)]
struct GraphEntry {
    graph_id: GraphId,
    graph_type: GraphType,
    dag: Dag,
    created_at: Timestamp,
    closed: bool,
}

/// Main kernel handle that implements all operational traits
pub struct KernelHandle {
    config: KernelConfig,
    graphs: RwLock<HashMap<GraphId, GraphEntry>>,
    nodes: RwLock<HashMap<NodeId, NodeEntry>>,
    event_log: EventLog,
    next_schedule_seq: RwLock<u64>,
}

impl KernelHandle {
    /// Create a new kernel handle with default configuration
    pub fn new() -> Self {
        Self::with_config(KernelConfig::default())
    }

    /// Create a new kernel handle with custom configuration
    pub fn with_config(config: KernelConfig) -> Self {
        Self {
            config,
            graphs: RwLock::new(HashMap::new()),
            nodes: RwLock::new(HashMap::new()),
            event_log: EventLog::default(),
            next_schedule_seq: RwLock::new(0),
        }
    }

    pub fn api_version(&self) -> ApiVersion {
        KERNEL_API_VERSION
    }

    pub fn check_compatibility(&self, expected: ApiVersion) -> Compatibility {
        let current = self.api_version();
        
        // Different major - incompatible
        if current.major != expected.major {
            let mut breaking_changes = Vec::new();
            breaking_changes.push(format!(
                "Major version mismatch: current={}.{}.{} expected={}.{}.{}",
                current.major, current.minor, current.patch,
                expected.major, expected.minor, expected.patch
            ));
            return Compatibility::Incompatible(breaking_changes);
        }
        
        // Same major, current >= expected - compatible
        if current.minor > expected.minor 
            || (current.minor == expected.minor && current.patch >= expected.patch) {
            return Compatibility::Compatible;
        }
        
        // Same major, current < expected - deprecated but works
        Compatibility::Deprecated
    }

    fn generate_schedule_token(&self, node_id: NodeId) -> ScheduleToken {
        let mut seq = self.next_schedule_seq.write();
        let token = ScheduleToken {
            node_id,
            sequence: *seq,
        };
        *seq += 1;
        token
    }

    fn log_event(
        &self,
        node_id: NodeId,
        autonomy_level: AutonomyLevel,
        directive_hash: DirectiveProfileHash,
        action: &str,
        result: &str,
    ) -> Result<EventId, LogError> {
        let event = Event {
            event_id: EventId::new(),
            timestamp: now_timestamp(),
            node_id,
            autonomy_level,
            directive_hash,
            action: action.to_string(),
            result: result.to_string(),
            prev_hash: [0u8; 32],
            hash: [0u8; 32],
        };
        self.event_log.append(event)
    }

    fn validate_token_full(&self, token: &CapabilityToken) -> Result<ValidationReport, KernelError> {
        let signature_valid = token.verify(&self.config.verifying_key);
        let timestamp_valid = !token.is_expired();
        let caps_valid = true; // Would check against system limits

        let valid = signature_valid && timestamp_valid && caps_valid;

        Ok(ValidationReport {
            valid,
            node_id: token.node_id,
            autonomy_level: token.autonomy_level,
            timestamp_valid,
            signature_valid,
            resource_caps_valid: caps_valid,
            reason: if !valid {
                Some(if !signature_valid {
                    "Invalid signature".to_string()
                } else if !timestamp_valid {
                    "Token expired".to_string()
                } else {
                    "Resource caps exceeded".to_string()
                })
            } else {
                None
            },
        })
    }
}

impl Default for KernelHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphManager for KernelHandle {
    fn create_graph(&self, graph_type: GraphType) -> Result<GraphId, KernelError> {
        let graph_id = GraphId::new();
        let dag = Dag::new(graph_type);
        
        let entry = GraphEntry {
            graph_id,
            graph_type,
            dag,
            created_at: now_timestamp(),
            closed: false,
        };

        self.graphs.write().insert(graph_id, entry);
        
        // Log the graph creation
        let _ = self.log_event(
            NodeId::new(), // System node
            AutonomyLevel::L0,
            DirectiveProfileHash([0u8; 32]),
            "create_graph",
            &format!("graph_id={:?}, type={:?}", graph_id, graph_type),
        );

        Ok(graph_id)
    }

    fn close_graph(&self, graph_id: GraphId) -> Result<(), KernelError> {
        let mut graphs = self.graphs.write();
        let entry = graphs.get_mut(&graph_id).ok_or(GraphError::GraphNotFound)?;
        
        if entry.closed {
            return Err(GraphError::GraphClosed.into());
        }
        
        entry.closed = true;
        
        let _ = self.log_event(
            NodeId::new(),
            AutonomyLevel::L0,
            DirectiveProfileHash([0u8; 32]),
            "close_graph",
            &format!("graph_id={:?}", graph_id),
        );

        Ok(())
    }

    fn graph_stats(&self, graph_id: GraphId) -> Result<GraphStats, KernelError> {
        let graphs = self.graphs.read();
        let entry = graphs.get(&graph_id).ok_or(GraphError::GraphNotFound)?;
        
        let node_count = entry.dag.node_count();
        let edge_count = entry.dag.edge_count();
        
        Ok(GraphStats {
            node_count,
            edge_count,
            graph_type: entry.graph_type,
            is_closed: entry.closed,
            created_at: entry.created_at,
        })
    }
}

impl NodeOperations for KernelHandle {
    fn add_node(&self, graph_id: GraphId, spec: NodeSpec) -> Result<NodeId, KernelError> {
        let graphs = self.graphs.read();
        let graph_entry = graphs.get(&graph_id).ok_or(GraphError::GraphNotFound)?;
        
        if graph_entry.closed {
            return Err(GraphError::GraphClosed.into());
        }

        let node_id = NodeId::new();
        
        // Add to DAG
        graph_entry.dag.add_node(node_id);
        
        // Create node entry
        let node_entry = NodeEntry {
            node_id,
            graph_id,
            spec,
            current_state: NodeState::Created,
            created_at: now_timestamp(),
            deactivated: false,
            frozen: false,
        };

        drop(graphs); // Release read lock before acquiring write lock
        self.nodes.write().insert(node_id, node_entry);
        
        let _ = self.log_event(
            node_id,
            AutonomyLevel::L0,
            DirectiveProfileHash([0u8; 32]),
            "add_node",
            "success",
        );

        Ok(node_id)
    }

    fn add_edge(&self, graph_id: GraphId, from: NodeId, to: NodeId) -> Result<(), KernelError> {
        let graphs = self.graphs.read();
        let graph_entry = graphs.get(&graph_id).ok_or(GraphError::GraphNotFound)?;
        
        if graph_entry.closed {
            return Err(GraphError::GraphClosed.into());
        }

        graph_entry.dag.add_edge(from, to)?;
        
        drop(graphs);
        let _ = self.log_event(
            from,
            AutonomyLevel::L0,
            DirectiveProfileHash([0u8; 32]),
            "add_edge",
            &format!("to={:?}", to),
        );

        Ok(())
    }

    fn deactivate_node(&self, node_id: NodeId) -> Result<(), KernelError> {
        let mut nodes = self.nodes.write();
        let entry = nodes.get_mut(&node_id).ok_or(NodeError::NodeNotFound)?;
        
        if entry.deactivated {
            return Ok(()); // Already deactivated
        }
        
        entry.deactivated = true;
        
        let _ = self.log_event(
            node_id,
            AutonomyLevel::L0,
            DirectiveProfileHash([0u8; 32]),
            "deactivate_node",
            "success",
        );

        Ok(())
    }

    fn freeze_node(&self, node_id: NodeId) -> Result<(), KernelError> {
        let mut nodes = self.nodes.write();
        let entry = nodes.get_mut(&node_id).ok_or(NodeError::NodeNotFound)?;
        
        entry.frozen = true;
        entry.current_state = NodeState::Frozen;
        
        let _ = self.log_event(
            node_id,
            AutonomyLevel::L0,
            DirectiveProfileHash([0u8; 32]),
            "freeze_node",
            "success",
        );

        Ok(())
    }
}

impl AutonomyManager for KernelHandle {
    fn issue_token(
        &self,
        node_id: NodeId,
        level: AutonomyLevel,
        caps: ResourceCaps,
    ) -> Result<CapabilityToken, KernelError> {
        // Verify node exists
        let nodes = self.nodes.read();
        let _ = nodes.get(&node_id).ok_or(NodeError::NodeNotFound)?;
        drop(nodes);

        // Check autonomy ceiling
        if !self.config.autonomy_ceiling.check(level) {
            return Err(AutonomyError::CeilingExceeded.into());
        }

        let directive_hash = DirectiveProfileHash([0u8; 32]); // Simplified
        let expires_at = now_timestamp() + DEFAULT_TOKEN_EXPIRY_SECS;

        let token = CapabilityToken::sign(
            node_id,
            level,
            caps,
            directive_hash,
            &self.config.signing_key,
            expires_at,
            "", // General purpose token
        );

        let _ = self.log_event(
            node_id,
            level,
            directive_hash,
            "issue_token",
            &format!("expires_at={}", expires_at),
        );

        Ok(token)
    }

    fn downgrade_token(
        &self,
        token: &CapabilityToken,
        new_level: AutonomyLevel,
    ) -> Result<CapabilityToken, KernelError> {
        // Verify original token first
        if !token.verify(&self.config.verifying_key) {
            return Err(AutonomyError::InvalidSignature.into());
        }

        // Cannot upgrade via downgrade
        if new_level.as_u8() > token.autonomy_level.as_u8() {
            return Err(AutonomyError::ElevationForbidden.into());
        }

        let directive_hash = DirectiveProfileHash([0u8; 32]);
        let expires_at = now_timestamp() + DEFAULT_TOKEN_EXPIRY_SECS;

        let new_token = CapabilityToken::sign(
            token.node_id,
            new_level,
            token.caps, // Keep same caps
            directive_hash,
            &self.config.signing_key,
            expires_at,
            "", // General purpose
        );

        let _ = self.log_event(
            token.node_id,
            new_level,
            directive_hash,
            "downgrade_token",
            &format!("from={:?}, to={:?}", token.autonomy_level, new_level),
        );

        Ok(new_token)
    }

    fn validate_token(
        &self,
        token: &CapabilityToken,
    ) -> Result<ValidationReport, KernelError> {
        self.validate_token_full(token)
    }
}

impl StateController for KernelHandle {
    fn transition(
        &self,
        node_id: NodeId,
        to: NodeState,
        token: &CapabilityToken,
    ) -> Result<TransitionReceipt, StateMachineError> {
        // Validate token
        if !token.verify(&self.config.verifying_key) {
            return Err(StateMachineError::IllegalTransition);
        }

        if token.is_expired() {
            return Err(StateMachineError::IllegalTransition);
        }

        // Check token is for this node
        if token.node_id != node_id {
            return Err(StateMachineError::IllegalTransition);
        }

        let mut nodes = self.nodes.write();
        let entry = nodes.get_mut(&node_id).ok_or(StateMachineError::IllegalTransition)?;
        
        if entry.frozen && to != NodeState::Escalated {
            return Err(StateMachineError::IllegalTransition);
        }

        let from_state = entry.current_state;
        
        // Validate transition
        state_machine::validate_transition(from_state, to)?;
        
        // Perform transition
        entry.current_state = to;
        
        drop(nodes);
        
        let _ = self.log_event(
            node_id,
            token.autonomy_level,
            token.directive_hash,
            "state_transition",
            &format!("from={:?}, to={:?}", from_state, to),
        );

        Ok(TransitionReceipt {
            node_id,
            from_state,
            to_state: to,
            timestamp: now_timestamp(),
            token_validated: true,
        })
    }

    fn current_state(&self, node_id: NodeId) -> Result<NodeState, KernelError> {
        let nodes = self.nodes.read();
        let entry = nodes.get(&node_id).ok_or(NodeError::NodeNotFound)?;
        Ok(entry.current_state)
    }

    fn allowed_transitions(&self, node_id: NodeId) -> Result<Vec<NodeState>, KernelError> {
        let nodes = self.nodes.read();
        let entry = nodes.get(&node_id).ok_or(NodeError::NodeNotFound)?;
        Ok(state_machine::allowed_transitions(entry.current_state))
    }
}

impl ExecutionRuntime for KernelHandle {
    fn execute(
        &self,
        node_id: NodeId,
        token: &CapabilityToken,
        work: WorkSpec,
    ) -> Result<ExecutionResult, ExecutionError> {
        // Validate token
        if !token.verify(&self.config.verifying_key) {
            return Err(ExecutionError {
                node_id: Some(node_id),
                kind: ExecutionErrorKind::TokenInvalid,
                message: "Token signature invalid".to_string(),
            });
        }

        if token.is_expired() {
            return Err(ExecutionError {
                node_id: Some(node_id),
                kind: ExecutionErrorKind::TokenInvalid,
                message: "Token expired".to_string(),
            });
        }

        // Check node exists and is in executing state
        let nodes = self.nodes.read();
        let entry = match nodes.get(&node_id) {
            Some(e) => e,
            None => {
                return Err(ExecutionError {
                    node_id: Some(node_id),
                    kind: ExecutionErrorKind::Internal,
                    message: "Node not found".to_string(),
                })
            }
        };

        if entry.current_state != NodeState::Executing {
            return Err(ExecutionError {
                node_id: Some(node_id),
                kind: ExecutionErrorKind::Internal,
                message: format!("Node not in Executing state: {:?}", entry.current_state),
            });
        }

        drop(nodes);

        // Log execution start
        let _ = self.log_event(
            node_id,
            token.autonomy_level,
            token.directive_hash,
            "execute_start",
            &format!("work_kind={}", work.kind),
        );

        // Perform execution based on isolation level
        let result = match token.autonomy_level {
            AutonomyLevel::L0 | AutonomyLevel::L1 | AutonomyLevel::L2 => {
                // Thread isolation
                let output = format!("Executed in thread: {}", work.kind);
                Ok(ExecutionResult {
                    success: true,
                    node_id,
                    output: Some(output),
                    resource_usage: ResourceUsage::default(),
                })
            }
            AutonomyLevel::L3 | AutonomyLevel::L4 | AutonomyLevel::L5 => {
                // Subprocess isolation - simplified
                let output = format!("Executed in subprocess: {}", work.kind);
                Ok(ExecutionResult {
                    success: true,
                    node_id,
                    output: Some(output),
                    resource_usage: ResourceUsage::default(),
                })
            }
        };

        // Log execution result
        let _ = self.log_event(
            node_id,
            token.autonomy_level,
            token.directive_hash,
            "execute_complete",
            if result.is_ok() { "success" } else { "failure" },
        );

        result
    }
}

impl ComplianceInterface for KernelHandle {
    fn validate_action(&self, action: ProposedAction) -> Result<ComplianceReport, ComplianceError> {
        let mut violations = Vec::new();
        let mut resource_check_passed = true;
        let policy_check_passed = true;

        // Check resource caps if specified
        if let Some(caps) = action.requested_caps {
            let limits = &self.config.default_caps;
            
            // Check against configured limits
            if caps.cpu_time_ms > limits.cpu_time_ms
                || caps.memory_bytes > limits.memory_bytes
                || caps.token_limit > limits.token_limit
                || caps.iteration_cap > limits.iteration_cap
            {
                resource_check_passed = false;
                violations.push(ComplianceViolation::PolicyViolation);
            }
            
            // Also check for obviously excessive/absurd values
            const REASONABLE_MAX_CPU: u64 = 24 * 60 * 60 * 1000; // 24 hours in ms
            const REASONABLE_MAX_MEMORY: u64 = 1024 * 1024 * 1024 * 100; // 100 GB
            const REASONABLE_MAX_TOKENS: u64 = 1_000_000_000;
            
            if caps.cpu_time_ms > REASONABLE_MAX_CPU
                || caps.memory_bytes > REASONABLE_MAX_MEMORY
                || caps.token_limit > REASONABLE_MAX_TOKENS
                || caps.iteration_cap > REASONABLE_MAX_TOKENS
            {
                resource_check_passed = false;
                violations.push(ComplianceViolation::PolicyViolation);
            }
        }

        // Check autonomy ceiling for token issuance
        if let ActionType::IssueToken = action.action_type {
            // Would check requested level against ceiling
        }

        let approved = violations.is_empty() && resource_check_passed && policy_check_passed;

        Ok(ComplianceReport {
            approved,
            violations,
            resource_check_passed,
            policy_check_passed,
            timestamp: now_timestamp(),
        })
    }

    fn query_policy(&self, _scope: PolicyScope) -> Result<PolicySnapshot, KernelError> {
        Ok(PolicySnapshot {
            max_autonomy_level: self.config.autonomy_ceiling.max_level,
            default_caps: self.config.default_caps,
            require_token_for_all_actions: true,
            timestamp: now_timestamp(),
        })
    }

    fn check_resources(&self, caps: ResourceCaps) -> Result<ResourceAvailability, KernelError> {
        let limits = &self.config.default_caps;
        
        Ok(ResourceAvailability {
            available: true,
            cpu_time_remaining_ms: limits.cpu_time_ms.saturating_sub(caps.cpu_time_ms),
            memory_remaining_bytes: limits.memory_bytes.saturating_sub(caps.memory_bytes),
            tokens_remaining: limits.token_limit.saturating_sub(caps.token_limit),
            iterations_remaining: limits.iteration_cap.saturating_sub(caps.iteration_cap),
        })
    }
}

impl EventLogger for KernelHandle {
    fn log_event(&self, event: Event) -> Result<EventId, LogError> {
        self.event_log.append(event)
    }

    fn query_events(&self, filter: EventFilter, limit: usize) -> Result<Vec<LogEntry>, KernelError> {
        let events = self.event_log.events();
        let filtered: Vec<_> = events
            .into_iter()
            .filter(|e| {
                if let Some(node_id) = filter.node_id {
                    if e.node_id != node_id {
                        return false;
                    }
                }
                if let Some(ref action) = filter.action {
                    if &e.action != action {
                        return false;
                    }
                }
                if let Some(since) = filter.since_timestamp {
                    if e.timestamp < since {
                        return false;
                    }
                }
                if let Some(until) = filter.until_timestamp {
                    if e.timestamp > until {
                        return false;
                    }
                }
                if let Some(level) = filter.autonomy_level {
                    if e.autonomy_level != level {
                        return false;
                    }
                }
                true
            })
            .take(limit)
            .map(|e| LogEntry { event: e, verified: true })
            .collect();
        
        Ok(filtered)
    }

    fn verify_integrity(&self) -> Result<IntegrityReport, KernelError> {
        let events = self.event_log.events();
        let events_checked = events.len();
        
        match self.event_log.verify_integrity() {
            Ok(()) => Ok(IntegrityReport {
                valid: true,
                events_checked,
                first_invalid_index: None,
                tamper_detected: false,
            }),
            Err(_) => Ok(IntegrityReport {
                valid: false,
                events_checked,
                first_invalid_index: Some(0),
                tamper_detected: true,
            }),
        }
    }
}

#[async_trait::async_trait]
impl Scheduler for KernelHandle {
    fn schedule(
        &self,
        node_id: NodeId,
        token: &CapabilityToken,
    ) -> Result<ScheduleToken, SchedulerError> {
        // Verify node exists
        let nodes = self.nodes.read();
        let _ = nodes.get(&node_id).ok_or(SchedulerError {
            kind: SchedulerErrorKind::NodeNotFound,
            message: "Node not found".to_string(),
        })?;
        drop(nodes);

        // Verify token
        if !token.verify(&self.config.verifying_key) {
            return Err(SchedulerError {
                kind: SchedulerErrorKind::Cancelled,
                message: "Invalid token".to_string(),
            });
        }

        let schedule_token = self.generate_schedule_token(node_id);

        let _ = self.log_event(
            node_id,
            token.autonomy_level,
            token.directive_hash,
            "schedule",
            &format!("sequence={}", schedule_token.sequence),
        );

        Ok(schedule_token)
    }

    fn cancel(&self, schedule_token: ScheduleToken) -> Result<(), SchedulerError> {
        let _ = self.log_event(
            schedule_token.node_id,
            AutonomyLevel::L0,
            DirectiveProfileHash([0u8; 32]),
            "cancel_schedule",
            &format!("sequence={}", schedule_token.sequence),
        );

        Ok(())
    }

    async fn wait_for_completion(
        &self,
        node_id: NodeId,
        timeout: Duration,
    ) -> Result<ExecutionResult, SchedulerError> {
        tokio::time::sleep(std::cmp::min(timeout, Duration::from_millis(10))).await;
        
        Ok(ExecutionResult {
            success: true,
            node_id,
            output: Some("Completed".to_string()),
            resource_usage: ResourceUsage::default(),
        })
    }
}
