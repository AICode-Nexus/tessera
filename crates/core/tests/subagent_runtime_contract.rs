use tessera_core::{
    SubagentApprovalForwardingRequest, SubagentCancellationRequest, SubagentCompletionRequest,
    SubagentInactivePolicyRequest, SubagentRuntimeCoordinator, SubagentRuntimeStartRequest,
    SubagentTaskOwnerAttachRequest, SubagentTaskOwnerDetachRequest,
    SubagentTaskOwnerHeartbeatRequest, SubagentTaskOwnerLostRequest,
    SubagentTaskOwnerReattachRequest, SubagentTranscriptArtifactLifecycleRequest,
};
use tessera_protocol::{
    AgentProfileId, ApprovalId, ArtifactId, ClientInstanceId, CostEstimate, EventRange,
    ReviewerGateId, RunEvent, RuntimeInstanceId, SubagentApprovalForwarding,
    SubagentApprovalForwardingStatus, SubagentCancellationCascade, SubagentInactiveParentAction,
    SubagentInactivePolicy, SubagentRuntimeDecisionKind, SubagentSessionCaps,
    SubagentSessionDescriptor, SubagentSessionId, SubagentSessionStatus,
    SubagentTranscriptArtifactStatus, TaskId, TaskOwnerKind, TaskOwnerStatus, TaskOwnershipId,
    TaskPauseCheckpointId, TaskReattachMode, Timestamp,
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

#[test]
fn subagent_transcript_artifact_lifecycle_reserves_without_event_range() {
    let coordinator = SubagentRuntimeCoordinator;
    let record =
        coordinator.record_transcript_lifecycle(SubagentTranscriptArtifactLifecycleRequest {
            session: subagent_session(SubagentSessionStatus::Active),
            artifact_id: ArtifactId::from_static("artifact_child_transcript"),
            status: SubagentTranscriptArtifactStatus::Reserved,
            event_range: None,
            summary_label: Some("child transcript handle".to_string()),
            reason: "reserved before child runtime starts".to_string(),
        });

    let record = record.expect("reserved lifecycle should not require an event range");
    assert_eq!(record.status, SubagentTranscriptArtifactStatus::Reserved);
    assert_eq!(record.event_range, None);
    assert_eq!(
        record.session_id,
        SubagentSessionId::from_static("subagent_session_review")
    );
    assert_eq!(
        record.parent_task_id,
        TaskId::from_static("task_parent_subagent")
    );
    assert_eq!(
        record.artifact_id,
        ArtifactId::from_static("artifact_child_transcript")
    );
}

#[test]
fn subagent_transcript_artifact_lifecycle_rejects_published_without_event_range() {
    let coordinator = SubagentRuntimeCoordinator;
    let error =
        coordinator.record_transcript_lifecycle(SubagentTranscriptArtifactLifecycleRequest {
            session: subagent_session(SubagentSessionStatus::Active),
            artifact_id: ArtifactId::from_static("artifact_child_transcript"),
            status: SubagentTranscriptArtifactStatus::Published,
            event_range: None,
            summary_label: Some("child transcript ready".to_string()),
            reason: "published transcript event range".to_string(),
        });

    let error = error.expect_err("published lifecycle should require an event range");
    assert!(error
        .to_string()
        .contains("event_range is required for published transcript lifecycle"));
}

#[test]
fn subagent_transcript_artifact_lifecycle_rejects_empty_published_event_range() {
    let coordinator = SubagentRuntimeCoordinator;
    let error =
        coordinator.record_transcript_lifecycle(SubagentTranscriptArtifactLifecycleRequest {
            session: subagent_session(SubagentSessionStatus::Active),
            artifact_id: ArtifactId::from_static("artifact_child_transcript"),
            status: SubagentTranscriptArtifactStatus::Published,
            event_range: Some(EventRange {
                start_seq: 9,
                end_seq: 8,
            }),
            summary_label: Some("child transcript ready".to_string()),
            reason: "published transcript event range".to_string(),
        });

    let error = error.expect_err("published lifecycle should reject an empty event range");
    assert!(error
        .to_string()
        .contains("event_range end_seq must be greater than or equal to start_seq"));
}

#[test]
fn subagent_transcript_artifact_lifecycle_sealed_event_is_metadata_only() {
    let coordinator = SubagentRuntimeCoordinator;
    let event =
        coordinator.transcript_lifecycle_event(SubagentTranscriptArtifactLifecycleRequest {
            session: subagent_session(SubagentSessionStatus::Completed),
            artifact_id: ArtifactId::from_static("artifact_child_transcript"),
            status: SubagentTranscriptArtifactStatus::Sealed,
            event_range: Some(EventRange {
                start_seq: 11,
                end_seq: 19,
            }),
            summary_label: Some("final child transcript metadata".to_string()),
            reason: "sealed after child completion".to_string(),
        });

    let event = event.expect("sealed lifecycle should emit a metadata event");
    assert_eq!(
        event.kind(),
        "subagent_transcript_artifact_lifecycle_recorded"
    );
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_parent_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["lifecycle"]["status"], "sealed");
    assert_eq!(payload["lifecycle"]["event_range"]["start_seq"], 11);
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("storage_write").is_none());
    assert!(payload.get("ui_action").is_none());

    match event {
        RunEvent::SubagentTranscriptArtifactLifecycleRecorded { lifecycle } => {
            assert_eq!(lifecycle.reason, "sealed after child completion");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn subagent_transcript_artifact_lifecycle_abandoned_preserves_reason_without_body() {
    let coordinator = SubagentRuntimeCoordinator;
    let event =
        coordinator.transcript_lifecycle_event(SubagentTranscriptArtifactLifecycleRequest {
            session: subagent_session(SubagentSessionStatus::Failed),
            artifact_id: ArtifactId::from_static("artifact_child_transcript"),
            status: SubagentTranscriptArtifactStatus::Abandoned,
            event_range: None,
            summary_label: None,
            reason: "abandoned before child runtime persisted transcript body".to_string(),
        });

    let event = event.expect("abandoned lifecycle should allow reason-only metadata");
    let payload = event.payload();
    assert_eq!(
        payload["lifecycle"]["reason"],
        "abandoned before child runtime persisted transcript body"
    );
    assert!(payload["lifecycle"]["event_range"].is_null());
    assert!(payload["lifecycle"].get("transcript").is_none());
    assert!(payload["lifecycle"].get("body").is_none());
    assert!(payload["lifecycle"].get("content").is_none());
    assert!(payload["lifecycle"].get("workspace_diff").is_none());
}

#[test]
fn subagent_approval_policy_queued_forwarding_requires_reviewer_gate() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.record_approval_forwarding(SubagentApprovalForwardingRequest {
        session: subagent_session(SubagentSessionStatus::WaitingForApproval),
        approval_id: ApprovalId::from_static("approval_subagent_review"),
        reviewer_gate_id: None,
        status: SubagentApprovalForwardingStatus::QueuedForReviewer,
        reason: "queued for reviewer".to_string(),
    });

    let error = error.expect_err("queued approval forwarding should require a reviewer gate");
    assert!(error
        .to_string()
        .contains("reviewer_gate_id is required for queued approval forwarding"));
}

#[test]
fn subagent_approval_policy_forwarded_to_parent_preserves_approval_metadata() {
    let coordinator = SubagentRuntimeCoordinator;
    let record = coordinator.record_approval_forwarding(SubagentApprovalForwardingRequest {
        session: subagent_session(SubagentSessionStatus::WaitingForApproval),
        approval_id: ApprovalId::from_static("approval_subagent_review"),
        reviewer_gate_id: Some(ReviewerGateId::from_static("gate_subagent_review")),
        status: SubagentApprovalForwardingStatus::ForwardedToParent,
        reason: "forwarded to parent approval queue".to_string(),
    });

    let record = record.expect("forwarded approval metadata should be recordable");
    assert_eq!(
        record.session_id,
        SubagentSessionId::from_static("subagent_session_review")
    );
    assert_eq!(
        record.parent_task_id,
        TaskId::from_static("task_parent_subagent")
    );
    assert_eq!(
        record.approval_id,
        ApprovalId::from_static("approval_subagent_review")
    );
    assert_eq!(
        record.reviewer_gate_id,
        Some(ReviewerGateId::from_static("gate_subagent_review"))
    );
    assert_eq!(
        record.status,
        SubagentApprovalForwardingStatus::ForwardedToParent
    );
}

#[test]
fn subagent_approval_policy_rejects_forwarding_already_forwarded_from_parent() {
    let coordinator = SubagentRuntimeCoordinator;
    let mut session = subagent_session(SubagentSessionStatus::WaitingForApproval);
    session.approval_forwarding = Some(SubagentApprovalForwarding {
        inactive_policy: SubagentInactivePolicy::RequireReviewer,
        reviewer_gate_id: Some(ReviewerGateId::from_static("gate_subagent_review")),
        approval_id: Some(ApprovalId::from_static("approval_subagent_review")),
        forwarded_from_parent: true,
    });

    let error = coordinator.record_approval_forwarding(SubagentApprovalForwardingRequest {
        session,
        approval_id: ApprovalId::from_static("approval_subagent_review"),
        reviewer_gate_id: Some(ReviewerGateId::from_static("gate_subagent_review")),
        status: SubagentApprovalForwardingStatus::ForwardedToParent,
        reason: "forwarded to parent approval queue".to_string(),
    });

    let error = error.expect_err("forwarding should not loop back to the parent");
    assert!(error
        .to_string()
        .contains("approval was already forwarded from parent"));
}

#[test]
fn subagent_approval_policy_denied_event_is_metadata_only() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.approval_forwarding_event(SubagentApprovalForwardingRequest {
        session: subagent_session(SubagentSessionStatus::Inactive),
        approval_id: ApprovalId::from_static("approval_subagent_review"),
        reviewer_gate_id: None,
        status: SubagentApprovalForwardingStatus::DeniedByPolicy,
        reason: "policy denied forwarding".to_string(),
    });

    let event = event.expect("denied approval forwarding should emit metadata");
    assert_eq!(event.kind(), "subagent_approval_forwarding_recorded");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_parent_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["forwarding"]["status"], "denied_by_policy");
    assert_eq!(payload["forwarding"]["reason"], "policy denied forwarding");
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("scheduler_loop").is_none());
    assert!(payload.get("storage_write").is_none());
    assert!(payload.get("ui_action").is_none());
}

#[test]
fn subagent_approval_policy_inactive_require_reviewer_event_is_metadata_only() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.inactive_policy_event(SubagentInactivePolicyRequest {
        session: subagent_session(SubagentSessionStatus::Inactive),
        policy: SubagentInactivePolicy::RequireReviewer,
        parent_action: SubagentInactiveParentAction::RequireReviewer,
        reason: "reviewer must decide inactive child".to_string(),
    });

    let event = event.expect("inactive policy should emit metadata");
    assert_eq!(event.kind(), "subagent_inactive_policy_recorded");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_parent_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["inactive"]["policy"], "require_reviewer");
    assert_eq!(payload["inactive"]["parent_action"], "require_reviewer");
    assert_eq!(
        payload["inactive"]["reason"],
        "reviewer must decide inactive child"
    );
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("scheduler_loop").is_none());
}

#[test]
fn subagent_approval_policy_rejects_mismatched_inactive_policy_action() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.record_inactive_policy(SubagentInactivePolicyRequest {
        session: subagent_session(SubagentSessionStatus::Inactive),
        policy: SubagentInactivePolicy::QueueDecision,
        parent_action: SubagentInactiveParentAction::PauseParent,
        reason: "mismatched inactive handling".to_string(),
    });

    let error = error.expect_err("inactive policy should reject mismatched parent action");
    assert!(error
        .to_string()
        .contains("queue_decision inactive policy requires queue_decision parent action"));
}

#[test]
fn subagent_cancellation_core_cancel_child_requires_child_task_id() {
    let coordinator = SubagentRuntimeCoordinator;
    let mut session = subagent_session(SubagentSessionStatus::Active);
    session.child_task_id = None;

    let error = coordinator.record_cancellation(SubagentCancellationRequest {
        session,
        source_task_id: TaskId::from_static("task_parent_subagent"),
        cascade: SubagentCancellationCascade::CancelChild,
        reason: "parent requested child cancellation".to_string(),
    });

    let error = error.expect_err("cancel_child should require a child task id");
    assert!(error
        .to_string()
        .contains("child_task_id is required for cancel_child cascade"));
}

#[test]
fn subagent_cancellation_core_cancel_child_record_preserves_source_and_cascade() {
    let coordinator = SubagentRuntimeCoordinator;
    let record = coordinator.record_cancellation(SubagentCancellationRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        source_task_id: TaskId::from_static("task_parent_subagent"),
        cascade: SubagentCancellationCascade::CancelChild,
        reason: "parent interruption should cancel active child".to_string(),
    });

    let record = record.expect("cancel child metadata should be recordable");
    assert_eq!(
        record.session_id,
        SubagentSessionId::from_static("subagent_session_review")
    );
    assert_eq!(
        record.parent_task_id,
        TaskId::from_static("task_parent_subagent")
    );
    assert_eq!(
        record.source_task_id,
        TaskId::from_static("task_parent_subagent")
    );
    assert_eq!(record.cascade, SubagentCancellationCascade::CancelChild);
    assert_eq!(
        record.reason,
        "parent interruption should cancel active child"
    );
}

#[test]
fn subagent_cancellation_core_rejects_unrelated_source_task() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.record_cancellation(SubagentCancellationRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        source_task_id: TaskId::from_static("task_unrelated"),
        cascade: SubagentCancellationCascade::ObserveOnly,
        reason: "foreign task should not affect this sub-agent session".to_string(),
    });

    let error = error.expect_err("unrelated source task should be rejected");
    assert!(error
        .to_string()
        .contains("source_task_id must match parent_task_id or child_task_id"));
}

#[test]
fn subagent_cancellation_core_observe_only_event_is_metadata_only() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.cancellation_event(SubagentCancellationRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        source_task_id: TaskId::from_static("task_child_subagent"),
        cascade: SubagentCancellationCascade::ObserveOnly,
        reason: "child reported cancellation already handled externally".to_string(),
    });

    let event = event.expect("observe-only cancellation should emit metadata");
    assert_eq!(event.kind(), "subagent_cancellation_recorded");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_parent_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["cancellation"]["cascade"], "observe_only");
    assert_eq!(
        payload["cancellation"]["source_task_id"],
        "task_child_subagent"
    );
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("scheduler_loop").is_none());
    assert!(payload.get("storage_write").is_none());
    assert!(payload.get("ui_action").is_none());

    match event {
        RunEvent::SubagentCancellationRecorded { cancellation } => {
            assert_eq!(
                cancellation.reason,
                "child reported cancellation already handled externally"
            );
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn subagent_cancellation_core_queue_cancellation_preserves_reason() {
    let coordinator = SubagentRuntimeCoordinator;
    let record = coordinator.record_cancellation(SubagentCancellationRequest {
        session: subagent_session(SubagentSessionStatus::WaitingForApproval),
        source_task_id: TaskId::from_static("task_parent_subagent"),
        cascade: SubagentCancellationCascade::QueueCancellation,
        reason: "queue cancellation until reviewer decision resolves".to_string(),
    });

    let record = record.expect("queued cancellation metadata should be recordable");
    assert_eq!(
        record.reason,
        "queue cancellation until reviewer decision resolves"
    );
    assert_eq!(
        record.cascade,
        SubagentCancellationCascade::QueueCancellation
    );
}

#[test]
fn subagent_cancellation_core_rejects_empty_reason() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.record_cancellation(SubagentCancellationRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        source_task_id: TaskId::from_static("task_parent_subagent"),
        cascade: SubagentCancellationCascade::ObserveOnly,
        reason: "   ".to_string(),
    });

    let error = error.expect_err("cancellation metadata should require a reason");
    assert!(error
        .to_string()
        .contains("reason is required for sub-agent cancellation"));
}

#[test]
fn subagent_task_owner_bridge_attach_requires_child_task_id() {
    let coordinator = SubagentRuntimeCoordinator;
    let mut session = subagent_session(SubagentSessionStatus::Planned);
    session.child_task_id = None;

    let error = coordinator.task_owner_lease(SubagentTaskOwnerAttachRequest {
        session,
        trace_id: "trace_subagent_child_owner".to_string(),
        runtime_id: RuntimeInstanceId::from_static("runtime_subagent_child"),
        client_id: None,
        owner_kind: TaskOwnerKind::Observer,
        last_seq: Some(7),
        heartbeat_interval_ms: 10_000,
        reason: "observe planned child task".to_string(),
    });

    let error = error.expect_err("attach should require a child task id");
    assert!(error
        .to_string()
        .contains("child_task_id is required for sub-agent task owner"));
}

#[test]
fn subagent_task_owner_bridge_active_execution_attach_targets_child_task() {
    let coordinator = SubagentRuntimeCoordinator;
    let lease = coordinator.task_owner_lease(SubagentTaskOwnerAttachRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        trace_id: "trace_subagent_child_owner".to_string(),
        runtime_id: RuntimeInstanceId::from_static("runtime_subagent_child"),
        client_id: Some(ClientInstanceId::from_static("client_subagent_shell")),
        owner_kind: TaskOwnerKind::Execution,
        last_seq: Some(12),
        heartbeat_interval_ms: 15_000,
        reason: "active child task owner metadata".to_string(),
    });

    let lease = lease.expect("active sub-agent should allow execution owner metadata");
    assert_eq!(lease.task_id, TaskId::from_static("task_child_subagent"));
    assert_eq!(lease.trace_id, "trace_subagent_child_owner");
    assert_eq!(
        lease.runtime_id,
        RuntimeInstanceId::from_static("runtime_subagent_child")
    );
    assert_eq!(
        lease.client_id,
        Some(ClientInstanceId::from_static("client_subagent_shell"))
    );
    assert_eq!(lease.owner_kind, TaskOwnerKind::Execution);
    assert_eq!(lease.status, TaskOwnerStatus::Attached);
    assert_eq!(lease.heartbeat_interval_ms, 15_000);
    assert_eq!(lease.last_seq, Some(12));
    assert_eq!(
        lease.reason,
        Some("active child task owner metadata".to_string())
    );
}

#[test]
fn subagent_task_owner_bridge_observer_attach_event_is_metadata_only() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.task_owner_attach_event(SubagentTaskOwnerAttachRequest {
        session: subagent_session(SubagentSessionStatus::WaitingForApproval),
        trace_id: "trace_subagent_child_owner".to_string(),
        runtime_id: RuntimeInstanceId::from_static("runtime_subagent_observer"),
        client_id: None,
        owner_kind: TaskOwnerKind::Observer,
        last_seq: None,
        heartbeat_interval_ms: 20_000,
        reason: "observe child while approval is pending".to_string(),
    });

    let event = event.expect("waiting sub-agent should allow observer owner metadata");
    assert_eq!(event.kind(), "task_owner_attached");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_child_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["lease"]["task_id"], "task_child_subagent");
    assert_eq!(payload["lease"]["owner_kind"], "observer");
    assert_eq!(payload["lease"]["status"], "attached");
    assert_eq!(
        payload["lease"]["reason"],
        "observe child while approval is pending"
    );
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("scheduler_loop").is_none());
    assert!(payload.get("storage_write").is_none());
    assert!(payload.get("ui_action").is_none());
}

#[test]
fn subagent_task_owner_bridge_rejects_execution_owner_for_waiting_session() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.task_owner_lease(SubagentTaskOwnerAttachRequest {
        session: subagent_session(SubagentSessionStatus::WaitingForApproval),
        trace_id: "trace_subagent_child_owner".to_string(),
        runtime_id: RuntimeInstanceId::from_static("runtime_subagent_child"),
        client_id: None,
        owner_kind: TaskOwnerKind::Execution,
        last_seq: Some(12),
        heartbeat_interval_ms: 10_000,
        reason: "waiting child should not claim execution".to_string(),
    });

    let error = error.expect_err("waiting session should not attach execution owner");
    assert!(error
        .to_string()
        .contains("execution task owner requires active sub-agent session"));
}

#[test]
fn subagent_task_owner_bridge_rejects_terminal_session_attach() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.task_owner_lease(SubagentTaskOwnerAttachRequest {
        session: subagent_session(SubagentSessionStatus::Completed),
        trace_id: "trace_subagent_child_owner".to_string(),
        runtime_id: RuntimeInstanceId::from_static("runtime_subagent_child"),
        client_id: None,
        owner_kind: TaskOwnerKind::Observer,
        last_seq: Some(12),
        heartbeat_interval_ms: 10_000,
        reason: "completed child should not attach a new owner".to_string(),
    });

    let error = error.expect_err("terminal session should not attach a new owner");
    assert!(error
        .to_string()
        .contains("terminal sub-agent session cannot attach task owner"));
}

#[test]
fn subagent_task_owner_bridge_detach_event_targets_child_task() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.task_owner_detach_event(SubagentTaskOwnerDetachRequest {
        session: subagent_session(SubagentSessionStatus::Completed),
        lease_id: TaskOwnershipId::from_static("task_owner_subagent_child"),
        reason: Some("child session reached terminal metadata".to_string()),
    });

    let event = event.expect("detach event should target child task metadata");
    assert_eq!(event.kind(), "task_owner_detached");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_child_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["lease_id"], "task_owner_subagent_child");
    assert_eq!(payload["task_id"], "task_child_subagent");
    assert_eq!(payload["reason"], "child session reached terminal metadata");
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("storage_write").is_none());
    assert!(payload.get("ui_action").is_none());
}

#[test]
fn subagent_owner_heartbeat_bridge_heartbeat_targets_child_task() {
    let coordinator = SubagentRuntimeCoordinator;
    let heartbeat_at = Timestamp::now_utc();
    let expires_at = Timestamp::now_utc();
    let event = coordinator.task_owner_heartbeat_event(SubagentTaskOwnerHeartbeatRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        lease_id: TaskOwnershipId::from_static("task_owner_subagent_child"),
        runtime_id: RuntimeInstanceId::from_static("runtime_subagent_child"),
        heartbeat_at: heartbeat_at.clone(),
        expires_at: expires_at.clone(),
        last_seq: 33,
    });

    let event = event.expect("active sub-agent should emit child heartbeat metadata");
    assert_eq!(event.kind(), "task_owner_heartbeat");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_child_subagent"))
    );
    let payload = event.payload();
    assert_eq!(
        payload["heartbeat"]["lease_id"],
        "task_owner_subagent_child"
    );
    assert_eq!(payload["heartbeat"]["task_id"], "task_child_subagent");
    assert_eq!(payload["heartbeat"]["last_seq"], 33);

    match event {
        RunEvent::TaskOwnerHeartbeat { heartbeat } => {
            assert_eq!(
                heartbeat.lease_id,
                TaskOwnershipId::from_static("task_owner_subagent_child")
            );
            assert_eq!(
                heartbeat.runtime_id,
                RuntimeInstanceId::from_static("runtime_subagent_child")
            );
            assert_eq!(heartbeat.heartbeat_at, heartbeat_at);
            assert_eq!(heartbeat.expires_at, expires_at);
            assert_eq!(heartbeat.last_seq, 33);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn subagent_owner_heartbeat_bridge_rejects_terminal_heartbeat() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.task_owner_heartbeat_event(SubagentTaskOwnerHeartbeatRequest {
        session: subagent_session(SubagentSessionStatus::Completed),
        lease_id: TaskOwnershipId::from_static("task_owner_subagent_child"),
        runtime_id: RuntimeInstanceId::from_static("runtime_subagent_child"),
        heartbeat_at: Timestamp::now_utc(),
        expires_at: Timestamp::now_utc(),
        last_seq: 34,
    });

    let error = error.expect_err("terminal session should not emit a fresh heartbeat");
    assert!(error
        .to_string()
        .contains("terminal sub-agent session cannot heartbeat task owner"));
}

#[test]
fn subagent_owner_heartbeat_bridge_lost_requires_child_task_id() {
    let coordinator = SubagentRuntimeCoordinator;
    let mut session = subagent_session(SubagentSessionStatus::Active);
    session.child_task_id = None;

    let error = coordinator.task_owner_lost_event(SubagentTaskOwnerLostRequest {
        session,
        lease_id: TaskOwnershipId::from_static("task_owner_subagent_child"),
        reason: "lost owner metadata".to_string(),
    });

    let error = error.expect_err("lost owner metadata should require child task id");
    assert!(error
        .to_string()
        .contains("child_task_id is required for sub-agent task owner"));
}

#[test]
fn subagent_owner_heartbeat_bridge_lost_rejects_empty_reason() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.task_owner_lost_event(SubagentTaskOwnerLostRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        lease_id: TaskOwnershipId::from_static("task_owner_subagent_child"),
        reason: "  ".to_string(),
    });

    let error = error.expect_err("lost owner metadata should require a reason");
    assert!(error
        .to_string()
        .contains("reason is required for sub-agent task owner lost"));
}

#[test]
fn subagent_owner_heartbeat_bridge_lost_event_is_metadata_only() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.task_owner_lost_event(SubagentTaskOwnerLostRequest {
        session: subagent_session(SubagentSessionStatus::Inactive),
        lease_id: TaskOwnershipId::from_static("task_owner_subagent_child"),
        reason: "child owner heartbeat expired".to_string(),
    });

    let event = event.expect("lost owner metadata should target child task");
    assert_eq!(event.kind(), "task_owner_lost");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_child_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["lease_id"], "task_owner_subagent_child");
    assert_eq!(payload["task_id"], "task_child_subagent");
    assert_eq!(payload["reason"], "child owner heartbeat expired");
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("scheduler_loop").is_none());
    assert!(payload.get("storage_write").is_none());
    assert!(payload.get("ui_action").is_none());
}

#[test]
fn subagent_owner_heartbeat_bridge_observe_reattach_requires_new_lease() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.task_owner_reattach_event(SubagentTaskOwnerReattachRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        mode: TaskReattachMode::ObserveExistingOwner,
        previous_lease_id: Some(TaskOwnershipId::from_static("task_owner_previous")),
        new_lease_id: None,
        checkpoint_id: None,
        since_seq: Some(40),
        reason: "observe an existing child owner".to_string(),
    });

    let error = error.expect_err("observe reattach should require a new lease");
    assert!(error
        .to_string()
        .contains("observe_existing_owner reattach requires new_lease_id"));
}

#[test]
fn subagent_owner_heartbeat_bridge_resume_reattach_requires_checkpoint() {
    let coordinator = SubagentRuntimeCoordinator;
    let error = coordinator.task_owner_reattach_event(SubagentTaskOwnerReattachRequest {
        session: subagent_session(SubagentSessionStatus::Active),
        mode: TaskReattachMode::ResumeFromCheckpoint,
        previous_lease_id: Some(TaskOwnershipId::from_static("task_owner_previous")),
        new_lease_id: Some(TaskOwnershipId::from_static("task_owner_new")),
        checkpoint_id: None,
        since_seq: Some(41),
        reason: "resume child from checkpoint".to_string(),
    });

    let error = error.expect_err("resume reattach should require a checkpoint");
    assert!(error
        .to_string()
        .contains("resume_from_checkpoint reattach requires checkpoint_id"));
}

#[test]
fn subagent_owner_heartbeat_bridge_terminal_projection_is_metadata_only() {
    let coordinator = SubagentRuntimeCoordinator;
    let event = coordinator.task_owner_reattach_event(SubagentTaskOwnerReattachRequest {
        session: subagent_session(SubagentSessionStatus::Completed),
        mode: TaskReattachMode::TerminalProjection,
        previous_lease_id: Some(TaskOwnershipId::from_static("task_owner_previous")),
        new_lease_id: None,
        checkpoint_id: Some(TaskPauseCheckpointId::from_static(
            "task_pause_checkpoint_child",
        )),
        since_seq: Some(42),
        reason: "project completed child owner from trace".to_string(),
    });

    let event = event.expect("terminal projection should emit child reattach metadata");
    assert_eq!(event.kind(), "task_reattach_recorded");
    assert_eq!(
        event.task_id(),
        Some(TaskId::from_static("task_child_subagent"))
    );
    let payload = event.payload();
    assert_eq!(payload["record"]["task_id"], "task_child_subagent");
    assert_eq!(payload["record"]["mode"], "terminal_projection");
    assert_eq!(payload["record"]["since_seq"], 42);
    assert_eq!(
        payload["record"]["reason"],
        "project completed child owner from trace"
    );
    assert!(payload.get("provider_request").is_none());
    assert!(payload.get("tool_call").is_none());
    assert!(payload.get("scheduler_loop").is_none());
    assert!(payload.get("storage_write").is_none());
    assert!(payload.get("ui_action").is_none());
}
