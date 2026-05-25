use tessera_core::{
    SubagentCompletionRequest, SubagentRuntimeCoordinator, SubagentRuntimeStartRequest,
};
use tessera_protocol::{
    AgentProfileId, ApprovalId, ArtifactId, CostEstimate, ReviewerGateId,
    SubagentApprovalForwarding, SubagentInactivePolicy, SubagentRuntimeDecisionKind,
    SubagentSessionCaps, SubagentSessionDescriptor, SubagentSessionId, SubagentSessionStatus,
    TaskId,
};

fn subagent_session(status: SubagentSessionStatus) -> SubagentSessionDescriptor {
    SubagentSessionDescriptor {
        session_id: SubagentSessionId::from_static("subagent_session_review"),
        parent_task_id: TaskId::from_static("task_parent_subagent"),
        child_task_id: Some(TaskId::from_static("task_child_subagent")),
        profile_id: AgentProfileId::from_static("agent_profile_reviewer"),
        objective: "review protocol metadata".to_string(),
        status,
        scope_labels: vec!["workspace:read".to_string()],
        tool_permission_labels: vec!["filesystem_read".to_string()],
        memory_scope_labels: vec!["none".to_string()],
        transcript_artifact_id: Some(ArtifactId::from_static("artifact_child_transcript")),
        caps: SubagentSessionCaps {
            max_steps: 4,
            max_depth: 1,
            timeout_ms: Some(30_000),
            max_child_sessions: 0,
            max_estimated_cost: Some(CostEstimate {
                amount: 0.02,
                currency: "USD".to_string(),
                input_cost: Some(0.008),
                output_cost: Some(0.012),
                cache_read_cost: None,
                cache_write_cost: None,
            }),
            concurrency_slot: Some("reviewer-1".to_string()),
        },
        approval_forwarding: Some(SubagentApprovalForwarding {
            inactive_policy: SubagentInactivePolicy::RequireReviewer,
            reviewer_gate_id: Some(ReviewerGateId::from_static("gate_subagent_review")),
            approval_id: Some(ApprovalId::from_static("approval_subagent_review")),
            forwarded_from_parent: false,
        }),
    }
}

#[test]
fn subagent_runtime_coordinator_rejects_sessions_over_depth_caps() {
    let coordinator = SubagentRuntimeCoordinator;
    let decision = coordinator.plan_start(SubagentRuntimeStartRequest {
        session: subagent_session(SubagentSessionStatus::Planned),
        current_depth: 2,
        requested_child_sessions: 0,
    });

    assert_eq!(decision.kind, SubagentRuntimeDecisionKind::StartDenied);
    assert!(decision.reason.contains("max_depth"));
    assert_eq!(
        decision.parent_task_id,
        TaskId::from_static("task_parent_subagent")
    );
}

#[test]
fn subagent_runtime_coordinator_records_decision_events_without_provider_or_tool_execution() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.plan_start_event(SubagentRuntimeStartRequest {
        session: subagent_session(SubagentSessionStatus::Planned),
        current_depth: 1,
        requested_child_sessions: 0,
    });

    assert_eq!(event.kind(), "subagent_runtime_decision_recorded");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_parent_subagent"))
    );
    assert_eq!(event.payload()["decision"]["kind"], "start_allowed");
    assert!(event.payload().get("provider_request").is_none());
    assert!(event.payload().get("tool_call").is_none());
    assert!(event.payload().get("scheduler_loop").is_none());
}

#[test]
fn subagent_runtime_coordinator_requires_transcript_artifact_before_completion() {
    let coordinator = SubagentRuntimeCoordinator;
    let mut session = subagent_session(SubagentSessionStatus::Completed);
    session.transcript_artifact_id = None;
    let decision = coordinator.plan_completion(SubagentCompletionRequest { session });

    assert_eq!(decision.kind, SubagentRuntimeDecisionKind::StartDenied);
    assert!(decision.reason.contains("transcript_artifact_id"));
}

#[test]
fn subagent_runtime_coordinator_rejects_missing_reviewer_gate_when_review_is_required() {
    let coordinator = SubagentRuntimeCoordinator;
    let mut session = subagent_session(SubagentSessionStatus::Planned);
    session.approval_forwarding = Some(SubagentApprovalForwarding {
        inactive_policy: SubagentInactivePolicy::RequireReviewer,
        reviewer_gate_id: None,
        approval_id: Some(ApprovalId::from_static("approval_subagent_review")),
        forwarded_from_parent: false,
    });

    let decision = coordinator.plan_start(SubagentRuntimeStartRequest {
        session,
        current_depth: 1,
        requested_child_sessions: 0,
    });

    assert_eq!(decision.kind, SubagentRuntimeDecisionKind::RequireReviewer);
    assert!(decision.reason.contains("reviewer_gate_id"));
}
