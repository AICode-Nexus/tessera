use tessera_client::{
    ClientApprovalStatus, ClientContextBudgetSummary, ClientContextPlacement,
    ClientContextSourceKind, ClientIntent, ClientMemoryProposalStatus, ClientMessageRole,
    ClientProjection, ClientReviewerGateStatus, ClientSnapshot, ClientStatus,
    ClientSubagentApprovalForwardingStatus, ClientSubagentCancellationCascade,
    ClientSubagentInactiveParentAction, ClientSubagentRuntimeDecisionKind,
    ClientSubagentSessionStatus, ClientSubagentTranscriptArtifactStatus,
};
use tessera_protocol::{
    AgentHandoffId, AgentHandoffMetrics, AgentHandoffStatus, AgentHandoffSummary,
    ApplyPatchDryRunOperationKind, ApplyPatchDryRunOperationSummary, ApplyPatchExecutionBlocker,
    ApplyPatchExecutionId, ApplyPatchExecutionRecord, ApplyPatchExecutionStatus,
    ApplyPatchPreflightBlocker, ApplyPatchPreflightId, ApplyPatchPreflightRecord,
    ApplyPatchPreflightStatus, ApprovalId, ApprovalStatus, ArtifactBodyRecord,
    ArtifactBodyRedactionStatus, ArtifactId, ArtifactKind, ClientInstanceId,
    CodingWorkflowEvidenceRedactionStatus, CodingWorkflowId, ContextId, ContextPlacement,
    ContextReference, ContextSource, ContextSourceKind, CostEstimate, ErrorSource, EventFrame,
    EventRange, HandoffEvidenceKind, HandoffEvidenceRef, ItemId, MemoryProposal, MemoryProposalId,
    MemoryProposalStatus, MutationMode, MutationRequestId, MutationRequestOperationKind,
    MutationRequestProposal, MutationRequestStatus, NormalizedError, PatchApplicationOutcome,
    PatchApplicationRecord, PatchProposal, PatchProposalId, PolicyDecisionId, PolicyOutcome,
    ProviderCapability, ProviderId, RestorePlanId, RestorePlanRecord, ReviewBundle, ReviewBundleId,
    ReviewerDecisionKind, ReviewerGateDecision, ReviewerGateId, ReviewerGateRequest, RunEvent,
    RuntimeInstanceId, SnapshotId, SubagentApprovalForwarding, SubagentApprovalForwardingRecord,
    SubagentApprovalForwardingStatus, SubagentCancellationCascade, SubagentCancellationRecord,
    SubagentInactiveParentAction, SubagentInactivePolicy, SubagentInactivePolicyRecord,
    SubagentRuntimeDecision, SubagentRuntimeDecisionKind, SubagentSessionCaps,
    SubagentSessionDescriptor, SubagentSessionId, SubagentSessionStatus,
    SubagentTranscriptArtifactLifecycleRecord, SubagentTranscriptArtifactRecord,
    SubagentTranscriptArtifactStatus, TaskId, TaskKind, TaskOwnerHeartbeat, TaskOwnerKind,
    TaskOwnerLease, TaskOwnerStatus, TaskOwnershipId, TaskReattachMode, TaskStatus,
    TestEvidenceSummaryId, TestEvidenceSummaryRecord, TestEvidenceSummaryStatus, TestPlanId,
    TestPlanRecord, TestRunId, TestRunRecord, TestRunStatus, Timestamp, ToolApproval, ToolCallId,
    ToolId, ToolPermission, ToolPolicyDecision, ToolSideEffect, WorkspaceMutationScope,
    WorkspaceWorktreeId, WorkspaceWorktreeLifecycleRecord, WorkspaceWorktreeLifecycleStatus,
};

#[test]
fn client_projection_turns_core_events_into_ui_neutral_messages() {
    let mut projection = ClientProjection::new("mock-default");
    let user_item_id = ItemId::from_static("item_user");
    let assistant_item_id = ItemId::from_static("item_assistant");

    projection.apply_event(&EventFrame::new(
        "trace_client",
        1,
        RunEvent::UserMessageRecorded {
            item_id: user_item_id.clone(),
            text: "hello client".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        2,
        RunEvent::AssistantMessageStarted {
            item_id: assistant_item_id.clone(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        3,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "shared ".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        4,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "projection".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        5,
        RunEvent::AssistantMessageCompleted {
            item_id: assistant_item_id,
        },
    ));

    assert_eq!(projection.messages[0].role, ClientMessageRole::User);
    assert_eq!(projection.messages[0].content, "hello client");
    assert_eq!(projection.messages[1].role, ClientMessageRole::Assistant);
    assert_eq!(projection.messages[1].content, "shared projection");
    assert!(!projection.messages[1].streaming);
}

#[test]
fn client_snapshot_keeps_status_intents_and_projection_toolkit_neutral() {
    let mut snapshot = ClientSnapshot::with_profiles("mock-default", ["mock-default", "offline"]);

    assert_eq!(
        snapshot.cycle_profile(1),
        Some(ClientIntent::SwitchProfile {
            profile_id: "offline".to_string(),
        })
    );
    assert_eq!(snapshot.status.active_profile, "offline");
    assert_eq!(snapshot.status.active_profile_position(), (2, 2));

    snapshot.draft_input = "hello gui".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::SubmitPrompt {
            profile_id: "offline".to_string(),
            prompt: "hello gui".to_string(),
        })
    );
    assert_eq!(snapshot.draft_input, "");

    let status = ClientStatus::with_profiles("mock-default", ["mock-default"]);
    assert_eq!(status.active_profile_position(), (1, 1));
}

#[test]
fn client_snapshot_projects_context_handles_and_summary() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.set_context_handles(
        [
            ContextReference {
                id: ContextId::from_static("context_architecture"),
                source: ContextSource {
                    kind: ContextSourceKind::File,
                    uri: Some("docs/technical-architecture.md".to_string()),
                    label: Some("architecture".to_string()),
                },
                placement: ContextPlacement::StablePrefix,
                estimated_tokens: 100,
                pinned: true,
                summary: Some("architecture contract".to_string()),
                metadata: None,
            },
            ContextReference {
                id: ContextId::from_static("context_trace"),
                source: ContextSource {
                    kind: ContextSourceKind::Trace,
                    uri: Some("trace://trace_mock".to_string()),
                    label: Some("transcript".to_string()),
                },
                placement: ContextPlacement::AppendOnlyTranscript,
                estimated_tokens: 50,
                pinned: false,
                summary: None,
                metadata: None,
            },
        ],
        ClientContextBudgetSummary {
            max_tokens: 200,
            reserved_output_tokens: 40,
            available_tokens: 160,
            used_tokens: 150,
            remaining_tokens: 10,
            stable_prefix_tokens: 100,
            append_only_transcript_tokens: 50,
            volatile_scratch_tokens: 0,
            over_budget: false,
        },
    );

    assert_eq!(snapshot.context_handles.len(), 2);
    assert_eq!(
        snapshot.context_handles[0].context_id,
        ContextId::from_static("context_architecture")
    );
    assert_eq!(
        snapshot.context_handles[0].source_kind,
        ClientContextSourceKind::File
    );
    assert_eq!(
        snapshot.context_handles[0].source_uri.as_deref(),
        Some("docs/technical-architecture.md")
    );
    assert_eq!(
        snapshot.context_handles[0].label.as_deref(),
        Some("architecture")
    );
    assert_eq!(
        snapshot.context_handles[0].placement,
        ClientContextPlacement::StablePrefix
    );
    assert_eq!(snapshot.context_handles[0].estimated_tokens, 100);
    assert!(snapshot.context_handles[0].pinned);
    assert_eq!(
        snapshot.context_handles[0].summary.as_deref(),
        Some("architecture contract")
    );
    assert_eq!(
        snapshot.status.context_handles_summary,
        "context 2 handles / 150/160 tokens"
    );
    assert!(!snapshot
        .status
        .context_handles_summary
        .contains("over budget"));

    snapshot.start_new_thread();

    assert!(snapshot.context_handles.is_empty());
    assert_eq!(
        snapshot.status.context_handles_summary,
        "context 0 handles / 0/0 tokens"
    );
}

#[test]
fn client_snapshot_maps_slash_commands_to_ui_neutral_intents() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.draft_input = " /new ".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::NewThread));

    snapshot.draft_input = "/save".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::SaveThread));

    snapshot.draft_input = "/export".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::ExportThread));

    snapshot.draft_input = "/cancel".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::CancelTask { task_id: None })
    );

    snapshot.draft_input = "/approve approval_write_readme".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::ApproveToolCall {
            approval_id: ApprovalId::from_static("approval_write_readme")
        })
    );

    snapshot.draft_input = "/deny approval_write_readme".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::DenyToolCall {
            approval_id: ApprovalId::from_static("approval_write_readme")
        })
    );

    snapshot.draft_input = "/remember memory_proposal_prefers_rust".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::AcceptMemoryProposal {
            proposal_id: MemoryProposalId::from_static("memory_proposal_prefers_rust")
        })
    );

    snapshot.draft_input = "/forget memory_proposal_prefers_rust".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::RejectMemoryProposal {
            proposal_id: MemoryProposalId::from_static("memory_proposal_prefers_rust")
        })
    );

    snapshot.draft_input = "/explain this command".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::SubmitPrompt {
            profile_id: "mock-default".to_string(),
            prompt: "/explain this command".to_string(),
        })
    );
}

#[test]
fn client_snapshot_cancel_command_targets_latest_running_task() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let first_task_id = TaskId::from_static("task_old_completed");
    let running_task_id = TaskId::from_static("task_running_cancel");

    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        1,
        RunEvent::TaskCreated {
            task_id: first_task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        2,
        RunEvent::TaskStarted {
            task_id: first_task_id.clone(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        3,
        RunEvent::TaskCompleted {
            task_id: first_task_id,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        4,
        RunEvent::TaskCreated {
            task_id: running_task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        5,
        RunEvent::TaskStarted {
            task_id: running_task_id.clone(),
        },
    ));

    assert_eq!(
        snapshot.active_cancellable_task_id(),
        Some(running_task_id.clone())
    );

    snapshot.draft_input = "/cancel".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::CancelTask {
            task_id: Some(running_task_id)
        })
    );
}

#[test]
fn client_snapshot_maps_pause_resume_commands_to_ui_neutral_intents() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let running_task_id = TaskId::from_static("task_running_pause");

    snapshot.apply_event(&EventFrame::new(
        "trace_pause_intent",
        1,
        RunEvent::TaskCreated {
            task_id: running_task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_pause_intent",
        2,
        RunEvent::TaskStarted {
            task_id: running_task_id.clone(),
        },
    ));

    snapshot.draft_input = "/pause".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::PauseTask {
            task_id: Some(running_task_id)
        })
    );

    snapshot.draft_input = "/pause task_explicit".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::PauseTask {
            task_id: Some(TaskId::from_static("task_explicit"))
        })
    );

    snapshot.draft_input = "/resume-task task_paused".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::ResumeTask {
            task_id: TaskId::from_static("task_paused")
        })
    );

    snapshot.draft_input = "/resume-task".to_string();
    assert_eq!(snapshot.submit_input(), None);
}

#[test]
fn client_snapshot_projects_pending_and_resolved_tool_approvals() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let approval_id = ApprovalId::from_static("approval_write_readme");
    let call_id = ToolCallId::from_static("tool_call_write_readme");
    let tool_id = ToolId::from_static("tool_workspace_write");

    snapshot.apply_event(&EventFrame::new(
        "trace_approval",
        1,
        RunEvent::ToolPolicyDecisionRecorded {
            decision: ToolPolicyDecision {
                decision_id: PolicyDecisionId::from_static("policy_write_readme"),
                call_id: call_id.clone(),
                tool_id: tool_id.clone(),
                outcome: PolicyOutcome::AskUser,
                reason: "workspace_write_requires_approval".to_string(),
                required_permissions: vec![ToolPermission::FilesystemWrite],
                side_effects: vec![ToolSideEffect::WritesWorkspace],
                approval_id: Some(approval_id.clone()),
            },
        },
    ));

    assert_eq!(snapshot.approvals.len(), 1);
    assert_eq!(snapshot.approvals[0].approval_id, approval_id);
    assert_eq!(snapshot.approvals[0].call_id, call_id);
    assert_eq!(snapshot.approvals[0].tool_id, tool_id);
    assert_eq!(snapshot.approvals[0].status, ClientApprovalStatus::Pending);
    assert_eq!(
        snapshot.approvals[0].required_permissions,
        vec!["filesystem_write"]
    );
    assert_eq!(snapshot.status.approval_summary, "approvals 1 pending");

    snapshot.apply_event(&EventFrame::new(
        "trace_approval",
        2,
        RunEvent::ToolCallApproved {
            approval: ToolApproval {
                approval_id: ApprovalId::from_static("approval_write_readme"),
                call_id: ToolCallId::from_static("tool_call_write_readme"),
                tool_id: ToolId::from_static("tool_workspace_write"),
                status: ApprovalStatus::Approved,
                reason: Some("user approved workspace write".to_string()),
            },
        },
    ));

    assert_eq!(snapshot.approvals[0].status, ClientApprovalStatus::Approved);
    assert_eq!(
        snapshot.approvals[0].reason.as_deref(),
        Some("user approved workspace write")
    );
    assert_eq!(snapshot.status.approval_summary, "approvals 0 pending");
}

#[test]
fn client_snapshot_projects_memory_proposals_for_ui_review() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let proposal_id = MemoryProposalId::from_static("memory_proposal_prefers_rust");
    let pending = MemoryProposal {
        proposal_id: proposal_id.clone(),
        status: MemoryProposalStatus::Pending,
        title: "Preferred language".to_string(),
        summary: "User prefers Rust-first implementations.".to_string(),
        source_item_id: Some(ItemId::from_static("item_memory_source")),
        reason: Some("explicit preference".to_string()),
        metadata: None,
    };

    snapshot.apply_event(&EventFrame::new(
        "trace_memory_client",
        1,
        RunEvent::MemoryWriteProposed {
            proposal: pending.clone(),
        },
    ));

    assert_eq!(snapshot.memory_proposals.len(), 1);
    assert_eq!(snapshot.memory_proposals[0].proposal_id, proposal_id);
    assert_eq!(
        snapshot.memory_proposals[0].status,
        ClientMemoryProposalStatus::Pending
    );
    assert_eq!(
        snapshot.memory_proposals[0].summary,
        "User prefers Rust-first implementations."
    );
    assert_eq!(snapshot.status.memory_summary, "memory 1 pending");

    snapshot.apply_event(&EventFrame::new(
        "trace_memory_client",
        2,
        RunEvent::MemoryWriteApplied {
            proposal: MemoryProposal {
                status: MemoryProposalStatus::Applied,
                ..pending.clone()
            },
        },
    ));

    assert_eq!(
        snapshot.memory_proposals[0].status,
        ClientMemoryProposalStatus::Applied
    );
    assert_eq!(snapshot.status.memory_summary, "memory 0 pending");

    let mut replayed = ClientSnapshot::new("mock-default");
    replayed.apply_trace_record(
        &EventFrame::new(
            "trace_memory_client",
            3,
            RunEvent::MemoryWriteRejected {
                proposal: MemoryProposal {
                    status: MemoryProposalStatus::Rejected,
                    reason: Some("user rejected".to_string()),
                    ..pending
                },
            },
        )
        .to_trace_record(),
    );

    assert_eq!(
        replayed.memory_proposals[0].status,
        ClientMemoryProposalStatus::Rejected
    );
    assert_eq!(
        replayed.memory_proposals[0].reason.as_deref(),
        Some("user rejected")
    );
}

fn coding_workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from_static("coding_workflow_client")
}

fn coding_task_id() -> TaskId {
    TaskId::from_static("task_coding_workflow_client")
}

fn coding_diff_evidence() -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::DiffArtifact,
        artifact_id: Some(ArtifactId::from_static("artifact_patch_diff")),
        trace_id: None,
        event_range: None,
        label: Some("patch diff".to_string()),
        summary: Some("metadata-only diff artifact".to_string()),
    }
}

fn coding_test_evidence() -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::TestOutputArtifact,
        artifact_id: Some(ArtifactId::from_static("artifact_test_stdout")),
        trace_id: None,
        event_range: None,
        label: Some("cargo test".to_string()),
        summary: Some("failed focused client projection test".to_string()),
    }
}

fn coding_scope() -> WorkspaceMutationScope {
    WorkspaceMutationScope {
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        root_label: "repo".to_string(),
        allowed_paths: vec![
            "crates/client/src/lib.rs".to_string(),
            "crates/client/tests/client_contract.rs".to_string(),
        ],
        denied_paths: vec![".env".to_string()],
        mutation_mode: MutationMode::WorktreeFirst,
        worktree_required: true,
        reason: Some("client projection metadata only".to_string()),
    }
}

fn coding_patch_proposal() -> PatchProposal {
    PatchProposal {
        patch_id: PatchProposalId::from_static("patch_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        summary: "Project coding workflow metadata into client snapshot.".to_string(),
        touched_paths: vec![
            "crates/client/src/lib.rs".to_string(),
            "crates/client/tests/client_contract.rs".to_string(),
        ],
        diff_artifacts: vec![coding_diff_evidence()],
        risk_labels: vec!["client_projection".to_string()],
        required_checkpoint_id: Some(SnapshotId::from_static("snapshot_before_client_patch")),
        reviewer_gate_id: Some(ReviewerGateId::from_static("gate_coding_review")),
    }
}

fn coding_patch_application() -> PatchApplicationRecord {
    PatchApplicationRecord {
        patch_id: PatchProposalId::from_static("patch_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        checkpoint_id: Some(SnapshotId::from_static("snapshot_before_client_patch")),
        outcome: PatchApplicationOutcome::Planned,
        conflict_paths: Vec::new(),
        applied_paths: vec!["crates/client/src/lib.rs".to_string()],
        artifact_refs: vec![ArtifactId::from_static("artifact_patch_diff")],
    }
}

fn coding_test_plan() -> TestPlanRecord {
    TestPlanRecord {
        test_plan_id: TestPlanId::from_static("test_plan_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        command_labels: vec!["cargo test -p tessera-client coding_workflow".to_string()],
        affected_paths: vec!["crates/client/src/lib.rs".to_string()],
        required_artifact_kinds: vec![ArtifactKind::TestReport],
    }
}

fn coding_test_run(status: TestRunStatus) -> TestRunRecord {
    TestRunRecord {
        test_run_id: TestRunId::from_static("test_run_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        test_plan_id: Some(TestPlanId::from_static("test_plan_client_projection")),
        command_label: "cargo test -p tessera-client coding_workflow".to_string(),
        status,
        exit_code: Some(101),
        duration_ms: Some(1200),
        stdout_artifact_id: Some(ArtifactId::from_static("artifact_test_stdout")),
        stderr_artifact_id: Some(ArtifactId::from_static("artifact_test_stderr")),
        diagnostics: vec![coding_test_evidence()],
        redaction_status: CodingWorkflowEvidenceRedactionStatus::Redacted,
    }
}

fn coding_test_evidence_summary(status: TestEvidenceSummaryStatus) -> TestEvidenceSummaryRecord {
    TestEvidenceSummaryRecord {
        summary_id: TestEvidenceSummaryId::from_static("test_evidence_summary_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        test_plan_ids: vec![TestPlanId::from_static("test_plan_client_projection")],
        test_run_ids: vec![TestRunId::from_static("test_run_client_projection")],
        status,
        total_runs: 1,
        passed_runs: if status == TestEvidenceSummaryStatus::Passed {
            1
        } else {
            0
        },
        failed_runs: if status == TestEvidenceSummaryStatus::Failed {
            1
        } else {
            0
        },
        cancelled_runs: if status == TestEvidenceSummaryStatus::Cancelled {
            1
        } else {
            0
        },
        error_runs: if status == TestEvidenceSummaryStatus::Error {
            1
        } else {
            0
        },
        required_artifact_kinds: vec![ArtifactKind::TestReport],
        artifact_refs: vec![
            ArtifactId::from_static("artifact_test_stdout"),
            ArtifactId::from_static("artifact_test_stderr"),
        ],
        diagnostics: vec![coding_test_evidence()],
        redaction_status: CodingWorkflowEvidenceRedactionStatus::Redacted,
        summary: "Focused client projection test evidence is ready for review.".to_string(),
        execution_blocked: true,
    }
}

fn coding_review_bundle() -> ReviewBundle {
    ReviewBundle {
        review_bundle_id: ReviewBundleId::from_static("review_bundle_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        reviewer_gate_id: ReviewerGateId::from_static("gate_coding_review"),
        patch_ids: vec![PatchProposalId::from_static("patch_client_projection")],
        test_run_ids: vec![TestRunId::from_static("test_run_client_projection")],
        evidence: vec![coding_diff_evidence(), coding_test_evidence()],
        summary: "Client projection patch is ready for reviewer inspection.".to_string(),
    }
}

fn coding_restore_plan() -> RestorePlanRecord {
    RestorePlanRecord {
        restore_plan_id: RestorePlanId::from_static("restore_plan_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        checkpoint_id: SnapshotId::from_static("snapshot_before_client_patch"),
        target_paths: vec!["crates/client/src/lib.rs".to_string()],
        reason: "restore is represented as metadata only".to_string(),
        execution_blocked: true,
    }
}

fn client_worktree_lifecycle_record(
    status: WorkspaceWorktreeLifecycleStatus,
    reason: &str,
) -> WorkspaceWorktreeLifecycleRecord {
    WorkspaceWorktreeLifecycleRecord {
        worktree_id: WorkspaceWorktreeId::from_static("workspace_worktree_client_projection"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        trace_id: "trace_worktree_lifecycle_client".to_string(),
        source_commit: "17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1".to_string(),
        source_branch_label: Some("main".to_string()),
        worktree_root_label: "worktree:client-projection".to_string(),
        worktree_base_key: "data_dir:worktrees".to_string(),
        lifecycle_status: status,
        reason: reason.to_string(),
        created_for_request_id: Some(MutationRequestId::from_static(
            "mutation_request_client_apply",
        )),
        created_for_patch_id: Some(PatchProposalId::from_static("patch_proposal_client")),
        evidence: vec![HandoffEvidenceRef {
            kind: HandoffEvidenceKind::TraceRange,
            artifact_id: None,
            trace_id: Some("trace_worktree_lifecycle_client".to_string()),
            event_range: Some(EventRange {
                start_seq: 3,
                end_seq: 8,
            }),
            label: Some("approved isolated worktree request".to_string()),
            summary: Some("metadata-only lifecycle evidence".to_string()),
        }],
        metadata: None,
    }
}

fn coding_reviewer_gate_request() -> ReviewerGateRequest {
    ReviewerGateRequest {
        gate_id: ReviewerGateId::from_static("gate_coding_review"),
        handoff_id: AgentHandoffId::from_static("handoff_coding_review"),
        parent_task_id: coding_task_id(),
        requested_decisions: vec![
            ReviewerDecisionKind::Accept,
            ReviewerDecisionKind::Reject,
            ReviewerDecisionKind::RequestRevision,
        ],
        evidence: vec![coding_diff_evidence(), coding_test_evidence()],
    }
}

fn coding_workflow_events(test_status: TestRunStatus) -> Vec<RunEvent> {
    vec![
        RunEvent::CodingWorkflowStarted {
            workflow_id: coding_workflow_id(),
            task_id: coding_task_id(),
            objective: "project metadata-only coding workflow state".to_string(),
        },
        RunEvent::WorkspaceMutationScopeRecorded {
            scope: coding_scope(),
        },
        RunEvent::PatchProposalRecorded {
            proposal: coding_patch_proposal(),
        },
        RunEvent::PatchApplicationRecorded {
            record: coding_patch_application(),
        },
        RunEvent::TestPlanRecorded {
            plan: coding_test_plan(),
        },
        RunEvent::TestRunRecorded {
            record: coding_test_run(test_status),
        },
        RunEvent::ReviewerGateRequested {
            request: coding_reviewer_gate_request(),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: coding_review_bundle(),
        },
        RunEvent::RestorePlanRecorded {
            plan: coding_restore_plan(),
        },
    ]
}

fn mutation_request(status: MutationRequestStatus) -> MutationRequestProposal {
    MutationRequestProposal {
        request_id: MutationRequestId::from_static("mutation_request_client_apply"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        operation: MutationRequestOperationKind::PatchApplication,
        status,
        summary: "Request apply-patch execution after gates.".to_string(),
        requested_paths: vec!["crates/client/src/lib.rs".to_string()],
        required_checkpoint_id: Some(SnapshotId::from_static("snapshot_before_client_patch")),
        reviewer_gate_id: Some(ReviewerGateId::from_static("gate_coding_review")),
        policy_decision_id: Some(PolicyDecisionId::from_static("policy_client_apply")),
        sandbox_profile_label: Some("workspace_write".to_string()),
        worktree_required: true,
        evidence: vec![coding_diff_evidence()],
    }
}

fn apply_patch_preflight(status: ApplyPatchPreflightStatus) -> ApplyPatchPreflightRecord {
    ApplyPatchPreflightRecord {
        preflight_id: ApplyPatchPreflightId::from_static("apply_patch_preflight_client"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        request_id: MutationRequestId::from_static("mutation_request_client_apply"),
        patch_id: PatchProposalId::from_static("patch_proposal_client"),
        status,
        blockers: vec![ApplyPatchPreflightBlocker::ExecutorUnavailable],
        affected_paths: vec!["crates/client/src/lib.rs".to_string()],
        operations: vec![ApplyPatchDryRunOperationSummary {
            path: "crates/client/src/lib.rs".to_string(),
            operation: ApplyPatchDryRunOperationKind::Modify,
        }],
        executor_blocked: true,
        executor_block_reason: "apply_patch_executor_not_implemented".to_string(),
        evidence: vec![coding_diff_evidence()],
    }
}

fn apply_patch_execution(status: ApplyPatchExecutionStatus) -> ApplyPatchExecutionRecord {
    ApplyPatchExecutionRecord {
        execution_id: ApplyPatchExecutionId::from_static("apply_patch_execution_client"),
        preflight_id: ApplyPatchPreflightId::from_static("apply_patch_preflight_client"),
        workflow_id: coding_workflow_id(),
        task_id: coding_task_id(),
        request_id: MutationRequestId::from_static("mutation_request_client_apply"),
        patch_id: PatchProposalId::from_static("patch_proposal_client"),
        checkpoint_id: Some(SnapshotId::from_static("snapshot_before_client_patch")),
        reviewer_gate_id: Some(ReviewerGateId::from_static("gate_coding_review")),
        policy_decision_id: Some(PolicyDecisionId::from_static("policy_client_apply")),
        sandbox_profile_label: Some("workspace_write_isolated".to_string()),
        isolated_root_label: "worktree:client-apply-patch".to_string(),
        executor_label: "core.apply_patch_executor.v1".to_string(),
        status,
        blockers: match status {
            ApplyPatchExecutionStatus::Conflict => vec![ApplyPatchExecutionBlocker::HunkConflict],
            ApplyPatchExecutionStatus::Rejected => vec![ApplyPatchExecutionBlocker::UnsafePath],
            _ => Vec::new(),
        },
        affected_paths: vec!["crates/client/src/lib.rs".to_string()],
        conflict_paths: match status {
            ApplyPatchExecutionStatus::Conflict => vec!["crates/client/src/lib.rs".to_string()],
            _ => Vec::new(),
        },
        artifact_refs: vec![ArtifactId::from_static("artifact_apply_patch_execution")],
        evidence: vec![coding_diff_evidence()],
    }
}

fn workflow_inspection_workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from("/Users/admin/work/tessera/.env::sk-secret-workflow-id")
}

fn workflow_inspection_task_id() -> TaskId {
    TaskId::from("/Users/admin/work/tessera/.env::sk-secret-task-id")
}

fn workflow_inspection_gate_id(suffix: &str) -> ReviewerGateId {
    ReviewerGateId::from(format!(
        "/Users/admin/work/tessera/.env::sk-secret-gate-{suffix}"
    ))
}

fn workflow_inspection_evidence(label: &str, summary: &str) -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::DiffArtifact,
        artifact_id: Some(ArtifactId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-artifact-{label}"
        ))),
        trace_id: Some(format!(
            "/Users/admin/work/tessera/.env::sk-secret-trace-{label}"
        )),
        event_range: Some(EventRange {
            start_seq: 3,
            end_seq: 8,
        }),
        label: Some(label.to_string()),
        summary: Some(summary.to_string()),
    }
}

fn workflow_inspection_scope() -> WorkspaceMutationScope {
    WorkspaceMutationScope {
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        root_label: "/Users/admin/work/tessera/.env::sk-secret-root-label".to_string(),
        allowed_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-allowed".to_string()],
        denied_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-denied".to_string()],
        mutation_mode: MutationMode::WorktreeFirst,
        worktree_required: true,
        reason: Some("/Users/admin/work/tessera/.env::sk-secret-scope-reason".to_string()),
    }
}

fn workflow_inspection_patch_proposal() -> PatchProposal {
    PatchProposal {
        patch_id: PatchProposalId::from("/Users/admin/work/tessera/.env::sk-secret-patch-id"),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        summary: "/Users/admin/work/tessera/.env::sk-secret-patch-summary".to_string(),
        touched_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-touched-path".to_string()],
        diff_artifacts: vec![
            workflow_inspection_evidence(
                "/Users/admin/work/tessera/.env::sk-secret-diff-label",
                "/Users/admin/work/tessera/.env::sk-secret-diff-summary",
            ),
            HandoffEvidenceRef {
                kind: HandoffEvidenceKind::TraceRange,
                artifact_id: None,
                trace_id: Some(
                    "/Users/admin/work/tessera/.env::sk-secret-trace-only-diff-trace".to_string(),
                ),
                event_range: Some(EventRange {
                    start_seq: 21,
                    end_seq: 34,
                }),
                label: Some(
                    "/Users/admin/work/tessera/.env::sk-secret-trace-only-diff-label".to_string(),
                ),
                summary: Some(
                    "/Users/admin/work/tessera/.env::sk-secret-trace-only-diff-summary".to_string(),
                ),
            },
        ],
        risk_labels: vec!["/Users/admin/work/tessera/.env::sk-secret-risk-label".to_string()],
        required_checkpoint_id: Some(SnapshotId::from(
            "/Users/admin/work/tessera/.env::sk-secret-checkpoint-id",
        )),
        reviewer_gate_id: Some(workflow_inspection_gate_id("accepted")),
    }
}

fn workflow_inspection_mutation_request() -> MutationRequestProposal {
    MutationRequestProposal {
        request_id: MutationRequestId::from(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-request-id",
        ),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        operation: MutationRequestOperationKind::PatchApplication,
        status: MutationRequestStatus::ReviewerPending,
        summary: "/Users/admin/work/tessera/.env::sk-secret-mutation-summary".to_string(),
        requested_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-requested-path".to_string()
        ],
        required_checkpoint_id: Some(SnapshotId::from(
            "/Users/admin/work/tessera/.env::sk-secret-checkpoint-id",
        )),
        reviewer_gate_id: Some(workflow_inspection_gate_id("accepted")),
        policy_decision_id: Some(PolicyDecisionId::from(
            "/Users/admin/work/tessera/.env::sk-secret-policy-id",
        )),
        sandbox_profile_label: Some(
            "/Users/admin/work/tessera/.env::sk-secret-sandbox-label".to_string(),
        ),
        worktree_required: true,
        evidence: vec![workflow_inspection_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-mutation-evidence-summary",
        )],
    }
}

fn workflow_inspection_review_bundle(gate_suffix: &str) -> ReviewBundle {
    ReviewBundle {
        review_bundle_id: ReviewBundleId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-review-bundle-{gate_suffix}"
        )),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        reviewer_gate_id: workflow_inspection_gate_id(gate_suffix),
        patch_ids: vec![PatchProposalId::from(
            "/Users/admin/work/tessera/.env::sk-secret-patch-id",
        )],
        test_run_ids: vec![TestRunId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-run-id",
        )],
        evidence: vec![workflow_inspection_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-review-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-review-evidence-summary",
        )],
        summary: "/Users/admin/work/tessera/.env::sk-secret-review-summary".to_string(),
    }
}

fn workflow_inspection_gate_request(gate_suffix: &str) -> ReviewerGateRequest {
    ReviewerGateRequest {
        gate_id: workflow_inspection_gate_id(gate_suffix),
        handoff_id: AgentHandoffId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-handoff-{gate_suffix}"
        )),
        parent_task_id: workflow_inspection_task_id(),
        requested_decisions: vec![
            ReviewerDecisionKind::Accept,
            ReviewerDecisionKind::Reject,
            ReviewerDecisionKind::RequestRevision,
        ],
        evidence: vec![workflow_inspection_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-gate-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-gate-evidence-summary",
        )],
    }
}

fn workflow_inspection_gate_decision(
    gate_suffix: &str,
    decision: ReviewerDecisionKind,
) -> ReviewerGateDecision {
    ReviewerGateDecision {
        gate_id: workflow_inspection_gate_id(gate_suffix),
        handoff_id: AgentHandoffId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-handoff-{gate_suffix}"
        )),
        decision,
        reviewer: "/Users/admin/work/tessera/.env::sk-secret-reviewer".to_string(),
        reason_code: "/Users/admin/work/tessera/.env::sk-secret-reviewer-reason".to_string(),
        comment: Some("/Users/admin/work/tessera/.env::sk-secret-reviewer-comment".to_string()),
    }
}

fn workflow_inspection_pending_approval_event() -> RunEvent {
    RunEvent::ToolPolicyDecisionRecorded {
        decision: ToolPolicyDecision {
            decision_id: PolicyDecisionId::from(
                "/Users/admin/work/tessera/.env::sk-secret-policy-id",
            ),
            call_id: ToolCallId::from("/Users/admin/work/tessera/.env::sk-secret-tool-call-id"),
            tool_id: ToolId::from("/Users/admin/work/tessera/.env::sk-secret-tool-id"),
            outcome: PolicyOutcome::AskUser,
            reason: "/Users/admin/work/tessera/.env::sk-secret-approval-reason".to_string(),
            required_permissions: vec![ToolPermission::FilesystemWrite],
            side_effects: vec![ToolSideEffect::WritesWorkspace],
            approval_id: Some(ApprovalId::from(
                "/Users/admin/work/tessera/.env::sk-secret-approval-id",
            )),
        },
    }
}

fn workflow_inspection_approval_forwarding_event() -> RunEvent {
    RunEvent::SubagentApprovalForwardingRecorded {
        forwarding: SubagentApprovalForwardingRecord {
            session_id: SubagentSessionId::from(
                "/Users/admin/work/tessera/.env::sk-secret-session-id",
            ),
            parent_task_id: workflow_inspection_task_id(),
            approval_id: ApprovalId::from("/Users/admin/work/tessera/.env::sk-secret-approval-id"),
            reviewer_gate_id: Some(workflow_inspection_gate_id("accepted")),
            status: SubagentApprovalForwardingStatus::QueuedForReviewer,
            reason: "/Users/admin/work/tessera/.env::sk-secret-forwarding-reason".to_string(),
        },
    }
}

fn workflow_inspection_apply_patch_preflight() -> ApplyPatchPreflightRecord {
    ApplyPatchPreflightRecord {
        preflight_id: ApplyPatchPreflightId::from(
            "/Users/admin/work/tessera/.env::sk-secret-preflight-id",
        ),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        request_id: MutationRequestId::from(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-request-id",
        ),
        patch_id: PatchProposalId::from("/Users/admin/work/tessera/.env::sk-secret-patch-id"),
        status: ApplyPatchPreflightStatus::Blocked,
        blockers: vec![ApplyPatchPreflightBlocker::ExecutorUnavailable],
        affected_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-preflight-affected-path".to_string(),
        ],
        operations: vec![ApplyPatchDryRunOperationSummary {
            path: "/Users/admin/work/tessera/.env::sk-secret-preflight-operation-path".to_string(),
            operation: ApplyPatchDryRunOperationKind::Modify,
        }],
        executor_blocked: true,
        executor_block_reason: "/Users/admin/work/tessera/.env::sk-secret-executor-block-reason"
            .to_string(),
        evidence: vec![workflow_inspection_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-preflight-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-preflight-evidence-summary",
        )],
    }
}

fn workflow_inspection_apply_patch_execution() -> ApplyPatchExecutionRecord {
    ApplyPatchExecutionRecord {
        execution_id: ApplyPatchExecutionId::from(
            "/Users/admin/work/tessera/.env::sk-secret-execution-id",
        ),
        preflight_id: ApplyPatchPreflightId::from(
            "/Users/admin/work/tessera/.env::sk-secret-preflight-id",
        ),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        request_id: MutationRequestId::from(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-request-id",
        ),
        patch_id: PatchProposalId::from("/Users/admin/work/tessera/.env::sk-secret-patch-id"),
        checkpoint_id: Some(SnapshotId::from(
            "/Users/admin/work/tessera/.env::sk-secret-checkpoint-id",
        )),
        reviewer_gate_id: Some(workflow_inspection_gate_id("accepted")),
        policy_decision_id: Some(PolicyDecisionId::from(
            "/Users/admin/work/tessera/.env::sk-secret-policy-id",
        )),
        sandbox_profile_label: Some(
            "/Users/admin/work/tessera/.env::sk-secret-sandbox-label".to_string(),
        ),
        isolated_root_label: "/Users/admin/work/tessera/.env::sk-secret-isolated-root".to_string(),
        executor_label: "/Users/admin/work/tessera/.env::sk-secret-executor-label".to_string(),
        status: ApplyPatchExecutionStatus::Failed,
        blockers: vec![ApplyPatchExecutionBlocker::HunkConflict],
        affected_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-execution-affected-path".to_string(),
        ],
        conflict_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-execution-conflict-path".to_string(),
        ],
        artifact_refs: vec![ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-execution-artifact",
        )],
        evidence: vec![workflow_inspection_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-execution-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-execution-evidence-summary",
        )],
    }
}

fn workflow_inspection_restore_plan() -> RestorePlanRecord {
    RestorePlanRecord {
        restore_plan_id: RestorePlanId::from(
            "/Users/admin/work/tessera/.env::sk-secret-restore-plan-id",
        ),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        checkpoint_id: SnapshotId::from("/Users/admin/work/tessera/.env::sk-secret-checkpoint-id"),
        target_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-restore-target-path".to_string(),
        ],
        reason: "/Users/admin/work/tessera/.env::sk-secret-restore-reason".to_string(),
        execution_blocked: true,
    }
}

fn workflow_inspection_test_plan() -> TestPlanRecord {
    TestPlanRecord {
        test_plan_id: TestPlanId::from("/Users/admin/work/tessera/.env::sk-secret-test-plan-id"),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        command_labels: vec![
            "/Users/admin/work/tessera/.env::sk-secret-test-command-label".to_string(),
        ],
        affected_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-test-path".to_string()],
        required_artifact_kinds: vec![ArtifactKind::TestReport],
    }
}

fn workflow_inspection_test_run() -> TestRunRecord {
    TestRunRecord {
        test_run_id: TestRunId::from("/Users/admin/work/tessera/.env::sk-secret-test-run-id"),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        test_plan_id: Some(TestPlanId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-plan-id",
        )),
        command_label: "/Users/admin/work/tessera/.env::sk-secret-test-run-command".to_string(),
        status: TestRunStatus::Failed,
        exit_code: Some(101),
        duration_ms: Some(500),
        stdout_artifact_id: Some(ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-stdout-artifact-label",
        )),
        stderr_artifact_id: Some(ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-stderr-artifact-label",
        )),
        diagnostics: vec![workflow_inspection_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-diagnostic-label",
            "/Users/admin/work/tessera/.env::sk-secret-diagnostic-summary",
        )],
        redaction_status: CodingWorkflowEvidenceRedactionStatus::Redacted,
    }
}

fn workflow_inspection_test_evidence_summary() -> TestEvidenceSummaryRecord {
    TestEvidenceSummaryRecord {
        summary_id: TestEvidenceSummaryId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-summary-id",
        ),
        workflow_id: workflow_inspection_workflow_id(),
        task_id: workflow_inspection_task_id(),
        test_plan_ids: vec![TestPlanId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-plan-id",
        )],
        test_run_ids: vec![TestRunId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-run-id",
        )],
        status: TestEvidenceSummaryStatus::Failed,
        total_runs: 1,
        passed_runs: 0,
        failed_runs: 1,
        cancelled_runs: 0,
        error_runs: 0,
        required_artifact_kinds: vec![ArtifactKind::TestReport],
        artifact_refs: vec![ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-artifact-body-text",
        )],
        diagnostics: vec![workflow_inspection_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-test-evidence-diagnostic-label",
            "/Users/admin/work/tessera/.env::sk-secret-test-evidence-diagnostic-summary",
        )],
        redaction_status: CodingWorkflowEvidenceRedactionStatus::Redacted,
        summary: "/Users/admin/work/tessera/.env::sk-secret-test-evidence-summary".to_string(),
        execution_blocked: true,
    }
}

fn workflow_inspection_artifact_body_event() -> RunEvent {
    RunEvent::ArtifactBodyRecorded {
        record: ArtifactBodyRecord {
            artifact_id: ArtifactId::from(
                "/Users/admin/work/tessera/.env::sk-secret-artifact-body-id",
            ),
            kind: ArtifactKind::Patch,
            task_id: Some(workflow_inspection_task_id()),
            media_type: "text/x-patch".to_string(),
            byte_len: 42,
            storage_uri: "/Users/admin/work/tessera/.env::sk-secret-artifact-body-storage"
                .to_string(),
            redaction_status: ArtifactBodyRedactionStatus::Redacted,
            summary: Some(
                "/Users/admin/work/tessera/.env::sk-secret-artifact-body-summary".to_string(),
            ),
            metadata: None,
        },
    }
}

fn workflow_inspection_events() -> Vec<RunEvent> {
    vec![
        RunEvent::CodingWorkflowStarted {
            workflow_id: workflow_inspection_workflow_id(),
            task_id: workflow_inspection_task_id(),
            objective: "/Users/admin/work/tessera/.env::sk-secret-objective".to_string(),
        },
        RunEvent::WorkspaceMutationScopeRecorded {
            scope: workflow_inspection_scope(),
        },
        RunEvent::PatchProposalRecorded {
            proposal: workflow_inspection_patch_proposal(),
        },
        RunEvent::MutationRequestProposalRecorded {
            proposal: workflow_inspection_mutation_request(),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspection_review_bundle("accepted"),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspection_review_bundle("rejected"),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspection_review_bundle("revision"),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspection_review_bundle("pending"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspection_gate_request("accepted"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspection_gate_request("rejected"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspection_gate_request("revision"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspection_gate_request("pending"),
        },
        RunEvent::ReviewerGateResolved {
            decision: workflow_inspection_gate_decision("accepted", ReviewerDecisionKind::Accept),
        },
        RunEvent::ReviewerGateResolved {
            decision: workflow_inspection_gate_decision("rejected", ReviewerDecisionKind::Reject),
        },
        RunEvent::ReviewerGateResolved {
            decision: workflow_inspection_gate_decision(
                "revision",
                ReviewerDecisionKind::RequestRevision,
            ),
        },
        workflow_inspection_pending_approval_event(),
        workflow_inspection_approval_forwarding_event(),
        RunEvent::ApplyPatchPreflightRecorded {
            record: workflow_inspection_apply_patch_preflight(),
        },
        RunEvent::ApplyPatchExecutionRecorded {
            record: workflow_inspection_apply_patch_execution(),
        },
        RunEvent::RestorePlanRecorded {
            plan: workflow_inspection_restore_plan(),
        },
        RunEvent::TestPlanRecorded {
            plan: workflow_inspection_test_plan(),
        },
        RunEvent::TestRunRecorded {
            record: workflow_inspection_test_run(),
        },
        RunEvent::TestEvidenceSummaryRecorded {
            record: workflow_inspection_test_evidence_summary(),
        },
        workflow_inspection_artifact_body_event(),
    ]
}

fn assert_workflow_inspection_projection(snapshot: &ClientSnapshot) {
    assert_eq!(snapshot.workflow_inspections.len(), 1);
    let row = &snapshot.workflow_inspections[0];
    assert_eq!(row.workflow_ref, "workflow:1");
    assert_eq!(row.task_ref, "task:1");
    assert!(row.active);
    assert_eq!(row.mutation_mode.as_deref(), Some("worktree_first"));
    assert!(row.worktree_required);
    assert_eq!(row.patch_count, 1);
    assert_eq!(row.diff_artifact_ref_count, 1);
    assert_eq!(row.patches_requiring_review_count, 1);
    assert_eq!(row.review_bundle_count, 4);
    assert_eq!(row.review_evidence_ref_count, 4);
    assert_eq!(row.reviewer_gate_count, 4);
    assert_eq!(row.accepted_reviewer_gate_count, 1);
    assert_eq!(row.rejected_reviewer_gate_count, 1);
    assert_eq!(row.revision_requested_reviewer_gate_count, 1);
    assert_eq!(row.pending_reviewer_gate_count, 1);
    assert_eq!(row.approval_count, 1);
    assert_eq!(row.pending_approval_count, 1);
    assert_eq!(row.resolved_approval_count, 0);
    assert_eq!(row.apply_patch_preflight_count, 1);
    assert_eq!(row.executor_ready_preflight_count, 0);
    assert_eq!(row.executor_blocked_preflight_count, 1);
    assert_eq!(row.apply_patch_execution_count, 1);
    assert_eq!(row.successful_apply_patch_execution_count, 0);
    assert_eq!(row.failed_apply_patch_execution_count, 1);
    assert_eq!(
        snapshot.status.workflow_inspection_summary,
        "inspection workflows 1 / review gates 4 accepted 1 rejected 1 revision_requested 1 pending 1 / approvals 1 pending / diff refs 1"
    );

    let inspection_json = serde_json::to_string(&snapshot.workflow_inspections).unwrap();
    for leaked in [
        "/Users/admin/work/tessera/.env",
        "sk-secret",
        "root-label",
        "allowed",
        "denied",
        "objective",
        "patch-summary",
        "mutation-summary",
        "requested-path",
        "review-summary",
        "diff-label",
        "diff-summary",
        "trace-only-diff-label",
        "trace-only-diff-summary",
        "review-evidence-label",
        "review-evidence-summary",
        "reviewer-comment",
        "reviewer-reason",
        "test-command-label",
        "stdout-artifact-label",
        "stderr-artifact-label",
        "preflight-affected-path",
        "preflight-operation-path",
        "execution-affected-path",
        "execution-conflict-path",
        "restore-target-path",
        "restore-reason",
        "diagnostic",
        "test-artifact-body-text",
        "artifact-body-storage",
        "artifact-body-summary",
        "executor-block-reason",
    ] {
        assert!(
            !inspection_json.contains(leaked),
            "workflow inspection JSON leaked {leaked}: {inspection_json}"
        );
    }
}

#[test]
fn workflow_inspection_projects_safe_metadata_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    for (index, event) in workflow_inspection_events().into_iter().enumerate() {
        snapshot.apply_event(&EventFrame::new(
            "trace_workflow_inspection_live",
            index as u64 + 1,
            event,
        ));
    }

    assert_workflow_inspection_projection(&snapshot);
}

#[test]
fn workflow_inspection_projects_safe_metadata_from_replayed_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    for (index, event) in workflow_inspection_events().into_iter().enumerate() {
        snapshot.apply_trace_record(
            &EventFrame::new("trace_workflow_inspection_replay", index as u64 + 1, event)
                .to_trace_record(),
        );
    }

    assert_workflow_inspection_projection(&snapshot);
}

#[test]
fn client_snapshot_projects_worktree_lifecycle_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for (seq, status, reason) in [
        (
            1,
            WorkspaceWorktreeLifecycleStatus::Planned,
            "isolated worktree planned",
        ),
        (
            2,
            WorkspaceWorktreeLifecycleStatus::Created,
            "isolated worktree created",
        ),
        (
            3,
            WorkspaceWorktreeLifecycleStatus::Retained,
            "retained for reviewer",
        ),
    ] {
        snapshot.apply_event(&EventFrame::new(
            "trace_worktree_lifecycle_client",
            seq,
            RunEvent::WorkspaceWorktreeLifecycleRecorded {
                record: client_worktree_lifecycle_record(status, reason),
            },
        ));
    }

    assert_eq!(snapshot.worktree_lifecycles.len(), 1);
    let lifecycle = &snapshot.worktree_lifecycles[0];
    assert_eq!(
        lifecycle.worktree_id,
        WorkspaceWorktreeId::from_static("workspace_worktree_client_projection")
    );
    assert_eq!(lifecycle.workflow_id, coding_workflow_id());
    assert_eq!(lifecycle.task_id, coding_task_id());
    assert_eq!(
        lifecycle.latest_status,
        WorkspaceWorktreeLifecycleStatus::Retained
    );
    assert_eq!(
        lifecycle.latest_reason.as_deref(),
        Some("retained for reviewer")
    );
    assert_eq!(lifecycle.worktree_root_label, "worktree:client-projection");
    assert_eq!(lifecycle.worktree_base_key, "data_dir:worktrees");
    assert!(!lifecycle.worktree_root_label.contains('/'));
    assert!(!lifecycle.worktree_base_key.contains('/'));
    assert_eq!(
        lifecycle.created_for_request_id,
        Some(MutationRequestId::from_static(
            "mutation_request_client_apply"
        ))
    );
    assert_eq!(
        lifecycle.created_for_patch_id,
        Some(PatchProposalId::from_static("patch_proposal_client"))
    );
    assert_eq!(lifecycle.evidence.len(), 1);
    assert_eq!(snapshot.status.worktree_summary, "worktrees 1 / retained 1");
}

#[test]
fn client_snapshot_projects_worktree_lifecycle_from_replayed_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for (seq, status, reason) in [
        (
            1,
            WorkspaceWorktreeLifecycleStatus::Created,
            "isolated worktree created",
        ),
        (
            2,
            WorkspaceWorktreeLifecycleStatus::CleanupFailed,
            "git worktree remove failed",
        ),
    ] {
        snapshot.apply_trace_record(
            &EventFrame::new(
                "trace_worktree_lifecycle_client",
                seq,
                RunEvent::WorkspaceWorktreeLifecycleRecorded {
                    record: client_worktree_lifecycle_record(status, reason),
                },
            )
            .to_trace_record(),
        );
    }

    assert_eq!(snapshot.worktree_lifecycles.len(), 1);
    assert_eq!(
        snapshot.worktree_lifecycles[0].latest_status,
        WorkspaceWorktreeLifecycleStatus::CleanupFailed
    );
    assert_eq!(
        snapshot.worktree_lifecycles[0].latest_reason.as_deref(),
        Some("git worktree remove failed")
    );
    assert_eq!(
        snapshot.worktree_lifecycles[0].worktree_root_label,
        "worktree:client-projection"
    );
    assert_eq!(
        snapshot.worktree_lifecycles[0].worktree_base_key,
        "data_dir:worktrees"
    );
    assert_eq!(
        snapshot.status.worktree_summary,
        "worktrees 1 / cleanup_failed 1"
    );
}

#[test]
fn client_snapshot_projects_coding_workflow_metadata_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for (index, event) in coding_workflow_events(TestRunStatus::Failed)
        .into_iter()
        .enumerate()
    {
        snapshot.apply_event(&EventFrame::new(
            "trace_coding_workflow_live",
            index as u64 + 1,
            event,
        ));
    }

    assert_eq!(snapshot.coding_workflows.len(), 1);
    let workflow = &snapshot.coding_workflows[0];
    assert_eq!(workflow.workflow_id, coding_workflow_id());
    assert_eq!(workflow.task_id, coding_task_id());
    assert!(workflow.active);
    assert_eq!(
        workflow.objective.as_deref(),
        Some("project metadata-only coding workflow state")
    );

    let scope = workflow.workspace_scope.as_ref().expect("scope projected");
    assert_eq!(scope.mutation_mode, MutationMode::WorktreeFirst);
    assert!(scope.worktree_required);
    assert_eq!(
        scope.allowed_paths,
        vec![
            "crates/client/src/lib.rs".to_string(),
            "crates/client/tests/client_contract.rs".to_string()
        ]
    );

    assert_eq!(workflow.patch_proposals.len(), 1);
    assert_eq!(
        workflow.patch_proposals[0].summary,
        "Project coding workflow metadata into client snapshot."
    );
    assert_eq!(
        workflow.patch_proposals[0].touched_paths,
        vec![
            "crates/client/src/lib.rs".to_string(),
            "crates/client/tests/client_contract.rs".to_string()
        ]
    );
    assert_eq!(workflow.patch_applications.len(), 1);
    assert_eq!(
        workflow.patch_applications[0].outcome,
        PatchApplicationOutcome::Planned
    );

    assert_eq!(workflow.test_plans.len(), 1);
    assert_eq!(workflow.test_runs.len(), 1);
    assert_eq!(workflow.test_runs[0].status, TestRunStatus::Failed);
    assert_eq!(
        workflow.test_runs[0].stdout_artifact_id,
        Some(ArtifactId::from_static("artifact_test_stdout"))
    );
    assert_eq!(
        workflow.test_runs[0].redaction_status,
        CodingWorkflowEvidenceRedactionStatus::Redacted
    );

    assert_eq!(workflow.review_bundles.len(), 1);
    assert_eq!(snapshot.reviewer_gates.len(), 1);
    assert_eq!(
        workflow.review_bundles[0].reviewer_gate_id,
        snapshot.reviewer_gates[0].gate_id
    );

    assert_eq!(workflow.restore_plans.len(), 1);
    assert!(workflow.restore_plans[0].execution_blocked);
    assert_eq!(
        snapshot.status.coding_workflow_summary,
        "coding workflows 1 / patches 1 / tests 1 / test evidence summaries 0 / reviews 1 / mutation requests 0 / apply-patch preflights 0 / apply-patch executions 0 / blocked restores 1"
    );
}

#[test]
fn client_snapshot_projects_test_evidence_summary_from_live_and_replayed_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    snapshot.apply_event(&EventFrame::new(
        "trace_test_evidence_summary_live",
        1,
        RunEvent::TestEvidenceSummaryRecorded {
            record: coding_test_evidence_summary(TestEvidenceSummaryStatus::Failed),
        },
    ));

    assert_eq!(snapshot.coding_workflows.len(), 1);
    let workflow = &snapshot.coding_workflows[0];
    assert_eq!(workflow.test_evidence_summaries.len(), 1);
    assert_eq!(
        workflow.test_evidence_summaries[0].summary_id,
        TestEvidenceSummaryId::from_static("test_evidence_summary_client_projection")
    );
    assert_eq!(
        workflow.test_evidence_summaries[0].status,
        TestEvidenceSummaryStatus::Failed
    );
    assert_eq!(workflow.test_evidence_summaries[0].total_runs, 1);
    assert!(workflow.test_evidence_summaries[0].execution_blocked);
    assert!(snapshot.artifacts.iter().any(|artifact| artifact
        .referenced_by_event_kinds
        .contains(&"test_evidence_summary_recorded".to_string())));
    assert_eq!(
        snapshot.status.coding_workflow_summary,
        "coding workflows 1 / patches 0 / tests 0 / test evidence summaries 1 / reviews 0 / mutation requests 0 / apply-patch preflights 0 / apply-patch executions 0 / blocked restores 0"
    );

    let mut replayed = ClientSnapshot::new("mock-default");
    let record = EventFrame::new(
        "trace_test_evidence_summary_replay",
        1,
        RunEvent::TestEvidenceSummaryRecorded {
            record: coding_test_evidence_summary(TestEvidenceSummaryStatus::Passed),
        },
    )
    .to_trace_record();
    replayed.apply_trace_record(&record);

    assert_eq!(replayed.coding_workflows.len(), 1);
    assert_eq!(
        replayed.coding_workflows[0].test_evidence_summaries[0].status,
        TestEvidenceSummaryStatus::Passed
    );
    assert_eq!(
        replayed.coding_workflows[0].test_evidence_summaries[0].artifact_refs,
        vec![
            ArtifactId::from_static("artifact_test_stdout"),
            ArtifactId::from_static("artifact_test_stderr")
        ]
    );
}

#[test]
fn client_snapshot_projects_mutation_request_proposals_from_live_and_replayed_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    snapshot.apply_event(&EventFrame::new(
        "trace_mutation_request_live",
        1,
        RunEvent::CodingWorkflowStarted {
            workflow_id: coding_workflow_id(),
            task_id: coding_task_id(),
            objective: "request gated mutation".to_string(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_mutation_request_live",
        2,
        RunEvent::MutationRequestProposalRecorded {
            proposal: mutation_request(MutationRequestStatus::ReviewerPending),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_mutation_request_live",
        3,
        RunEvent::MutationRequestProposalRecorded {
            proposal: mutation_request(MutationRequestStatus::Approved),
        },
    ));

    assert_eq!(snapshot.coding_workflows.len(), 1);
    let workflow = &snapshot.coding_workflows[0];
    assert_eq!(workflow.mutation_requests.len(), 1);
    assert_eq!(
        workflow.mutation_requests[0].request_id,
        MutationRequestId::from_static("mutation_request_client_apply")
    );
    assert_eq!(
        workflow.mutation_requests[0].operation,
        MutationRequestOperationKind::PatchApplication
    );
    assert_eq!(
        workflow.mutation_requests[0].status,
        MutationRequestStatus::Approved
    );
    assert_eq!(
        workflow.mutation_requests[0].requested_paths,
        vec!["crates/client/src/lib.rs".to_string()]
    );
    assert!(workflow.mutation_requests[0].worktree_required);
    assert_eq!(
        workflow.mutation_requests[0]
            .sandbox_profile_label
            .as_deref(),
        Some("workspace_write")
    );
    assert_eq!(
        snapshot.status.coding_workflow_summary,
        "coding workflows 1 / patches 0 / tests 0 / test evidence summaries 0 / reviews 0 / mutation requests 1 / apply-patch preflights 0 / apply-patch executions 0 / blocked restores 0"
    );

    let mut replayed = ClientSnapshot::new("mock-default");
    let record = EventFrame::new(
        "trace_mutation_request_replay",
        1,
        RunEvent::MutationRequestProposalRecorded {
            proposal: mutation_request(MutationRequestStatus::PolicyBlocked),
        },
    )
    .to_trace_record();
    replayed.apply_trace_record(&record);

    assert_eq!(replayed.coding_workflows.len(), 1);
    assert_eq!(
        replayed.coding_workflows[0].mutation_requests[0].status,
        MutationRequestStatus::PolicyBlocked
    );
    assert_eq!(
        replayed.coding_workflows[0].mutation_requests[0].policy_decision_id,
        Some(PolicyDecisionId::from_static("policy_client_apply"))
    );
}

#[test]
fn client_snapshot_projects_apply_patch_preflight_from_live_and_replayed_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    snapshot.apply_event(&EventFrame::new(
        "trace_apply_patch_preflight_live",
        1,
        RunEvent::ApplyPatchPreflightRecorded {
            record: apply_patch_preflight(ApplyPatchPreflightStatus::DryRunReady),
        },
    ));

    assert_eq!(snapshot.coding_workflows.len(), 1);
    let workflow = &snapshot.coding_workflows[0];
    assert_eq!(workflow.apply_patch_preflights.len(), 1);
    assert_eq!(
        workflow.apply_patch_preflights[0].status,
        ApplyPatchPreflightStatus::DryRunReady
    );
    assert_eq!(
        workflow.apply_patch_preflights[0].blockers,
        vec![ApplyPatchPreflightBlocker::ExecutorUnavailable]
    );
    assert_eq!(
        workflow.apply_patch_preflights[0].affected_paths,
        vec!["crates/client/src/lib.rs".to_string()]
    );
    assert_eq!(
        workflow.apply_patch_preflights[0].operations[0].operation,
        ApplyPatchDryRunOperationKind::Modify
    );
    assert!(workflow.apply_patch_preflights[0].executor_blocked);
    assert_eq!(
        snapshot.status.coding_workflow_summary,
        "coding workflows 1 / patches 0 / tests 0 / test evidence summaries 0 / reviews 0 / mutation requests 0 / apply-patch preflights 1 / apply-patch executions 0 / blocked restores 0"
    );

    let mut replayed = ClientSnapshot::new("mock-default");
    let record = EventFrame::new(
        "trace_apply_patch_preflight_replay",
        1,
        RunEvent::ApplyPatchPreflightRecorded {
            record: apply_patch_preflight(ApplyPatchPreflightStatus::Blocked),
        },
    )
    .to_trace_record();
    replayed.apply_trace_record(&record);

    assert_eq!(
        replayed.coding_workflows[0].apply_patch_preflights[0].status,
        ApplyPatchPreflightStatus::Blocked
    );
    assert_eq!(
        replayed.coding_workflows[0].apply_patch_preflights[0].operations[0].path,
        "crates/client/src/lib.rs"
    );
}

#[test]
fn client_snapshot_projects_apply_patch_execution_from_live_and_replayed_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    snapshot.apply_event(&EventFrame::new(
        "trace_apply_patch_execution_live",
        1,
        RunEvent::ApplyPatchExecutionRecorded {
            record: apply_patch_execution(ApplyPatchExecutionStatus::Applied),
        },
    ));

    assert_eq!(snapshot.coding_workflows.len(), 1);
    let workflow = &snapshot.coding_workflows[0];
    assert_eq!(workflow.apply_patch_executions.len(), 1);
    assert_eq!(
        workflow.apply_patch_executions[0].status,
        ApplyPatchExecutionStatus::Applied
    );
    assert_eq!(
        workflow.apply_patch_executions[0].affected_paths,
        vec!["crates/client/src/lib.rs".to_string()]
    );
    assert!(workflow.apply_patch_executions[0].blockers.is_empty());
    assert_eq!(
        workflow.apply_patch_executions[0].artifact_refs,
        vec![ArtifactId::from_static("artifact_apply_patch_execution")]
    );
    assert!(snapshot.artifacts.iter().any(|artifact| artifact
        .referenced_by_event_kinds
        .contains(&"apply_patch_execution_recorded".to_string())));
    assert_eq!(
        snapshot.status.coding_workflow_summary,
        "coding workflows 1 / patches 0 / tests 0 / test evidence summaries 0 / reviews 0 / mutation requests 0 / apply-patch preflights 0 / apply-patch executions 1 / blocked restores 0"
    );

    let mut replayed = ClientSnapshot::new("mock-default");
    let record = EventFrame::new(
        "trace_apply_patch_execution_replay",
        1,
        RunEvent::ApplyPatchExecutionRecorded {
            record: apply_patch_execution(ApplyPatchExecutionStatus::Conflict),
        },
    )
    .to_trace_record();
    replayed.apply_trace_record(&record);

    assert_eq!(replayed.coding_workflows.len(), 1);
    let execution = &replayed.coding_workflows[0].apply_patch_executions[0];
    assert_eq!(execution.status, ApplyPatchExecutionStatus::Conflict);
    assert_eq!(
        execution.blockers,
        vec![ApplyPatchExecutionBlocker::HunkConflict]
    );
    assert_eq!(
        execution.conflict_paths,
        vec!["crates/client/src/lib.rs".to_string()]
    );
}

#[test]
fn client_snapshot_projects_coding_workflow_metadata_from_replayed_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for (index, event) in coding_workflow_events(TestRunStatus::Passed)
        .into_iter()
        .enumerate()
    {
        let record = EventFrame::new("trace_coding_workflow_replay", index as u64 + 1, event)
            .to_trace_record();
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.coding_workflows.len(), 1);
    let workflow = &snapshot.coding_workflows[0];
    assert_eq!(
        workflow.workspace_scope.as_ref().unwrap().denied_paths,
        vec![".env"]
    );
    assert_eq!(
        workflow.patch_proposals[0].diff_artifacts[0],
        coding_diff_evidence()
    );
    assert_eq!(workflow.test_runs[0].status, TestRunStatus::Passed);
    assert_eq!(
        workflow.review_bundles[0].patch_ids,
        vec![PatchProposalId::from_static("patch_client_projection")]
    );
    assert_eq!(
        workflow.restore_plans[0].checkpoint_id,
        SnapshotId::from_static("snapshot_before_client_patch")
    );
    assert_eq!(
        snapshot.status.coding_workflow_summary,
        "coding workflows 1 / patches 1 / tests 1 / test evidence summaries 0 / reviews 1 / mutation requests 0 / apply-patch preflights 0 / apply-patch executions 0 / blocked restores 1"
    );
}

fn handoff_summary(parent_task_id: TaskId, handoff_id: AgentHandoffId) -> AgentHandoffSummary {
    AgentHandoffSummary {
        handoff_id,
        parent_task_id,
        child_task_id: Some(TaskId::from_static("task_child_review")),
        status: AgentHandoffStatus::Completed,
        objective: "review protocol changes".to_string(),
        summary: "Protocol changes are bounded to handoff metadata.".to_string(),
        evidence: vec![HandoffEvidenceRef {
            kind: HandoffEvidenceKind::TraceRange,
            artifact_id: None,
            trace_id: Some("trace_child_review".to_string()),
            event_range: Some(EventRange {
                start_seq: 10,
                end_seq: 20,
            }),
            label: Some("child trace".to_string()),
            summary: Some("child result evidence".to_string()),
        }],
        metrics: AgentHandoffMetrics {
            steps_completed: 2,
            input_tokens: Some(500),
            output_tokens: Some(120),
            estimated_cost: Some(CostEstimate {
                amount: 0.01,
                currency: "USD".to_string(),
                input_cost: Some(0.004),
                output_cost: Some(0.006),
                cache_read_cost: None,
                cache_write_cost: None,
            }),
        },
        evidence_event_range: Some(EventRange {
            start_seq: 10,
            end_seq: 20,
        }),
    }
}

#[test]
fn client_snapshot_projects_handoff_reviewer_state_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let parent_task_id = TaskId::from_static("task_parent_review");
    let handoff_id = AgentHandoffId::from_static("handoff_review");
    let gate_id = ReviewerGateId::from_static("gate_review");
    let summary = handoff_summary(parent_task_id.clone(), handoff_id.clone());
    let request = ReviewerGateRequest {
        gate_id: gate_id.clone(),
        handoff_id: handoff_id.clone(),
        parent_task_id: parent_task_id.clone(),
        requested_decisions: vec![
            ReviewerDecisionKind::Accept,
            ReviewerDecisionKind::Reject,
            ReviewerDecisionKind::RequestRevision,
        ],
        evidence: summary.evidence.clone(),
    };

    snapshot.apply_event(&EventFrame::new(
        "trace_handoff_live",
        1,
        RunEvent::AgentHandoffRecorded {
            summary: summary.clone(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_handoff_live",
        2,
        RunEvent::ReviewerGateRequested {
            request: request.clone(),
        },
    ));

    assert_eq!(snapshot.handoffs.len(), 1);
    assert_eq!(snapshot.handoffs[0].handoff_id, handoff_id);
    assert_eq!(snapshot.handoffs[0].parent_task_id, parent_task_id);
    assert_eq!(snapshot.handoffs[0].status, AgentHandoffStatus::Completed);
    assert_eq!(snapshot.handoffs[0].evidence.len(), 1);
    assert_eq!(snapshot.handoffs[0].steps_completed, 2);
    assert_eq!(snapshot.handoffs[0].estimated_cost, Some(0.01));

    assert_eq!(snapshot.reviewer_gates.len(), 1);
    assert_eq!(snapshot.reviewer_gates[0].gate_id, gate_id);
    assert_eq!(
        snapshot.reviewer_gates[0].status,
        ClientReviewerGateStatus::Pending
    );
    assert_eq!(
        snapshot.status.handoff_summary,
        "handoffs 1 / reviews 1 pending"
    );

    snapshot.apply_event(&EventFrame::new(
        "trace_handoff_live",
        3,
        RunEvent::ReviewerGateResolved {
            decision: ReviewerGateDecision {
                gate_id: ReviewerGateId::from_static("gate_review"),
                handoff_id: AgentHandoffId::from_static("handoff_review"),
                decision: ReviewerDecisionKind::Accept,
                reviewer: "user".to_string(),
                reason_code: "evidence_sufficient".to_string(),
                comment: Some("accept bounded handoff".to_string()),
            },
        },
    ));

    assert_eq!(
        snapshot.reviewer_gates[0].status,
        ClientReviewerGateStatus::Accepted
    );
    assert_eq!(snapshot.reviewer_gates[0].reviewer.as_deref(), Some("user"));
    assert_eq!(
        snapshot.status.handoff_summary,
        "handoffs 1 / reviews 0 pending"
    );
}

#[test]
fn client_snapshot_projects_handoff_reviewer_state_from_replayed_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let parent_task_id = TaskId::from_static("task_parent_replay_review");
    let handoff_id = AgentHandoffId::from_static("handoff_replay_review");
    let gate_id = ReviewerGateId::from_static("gate_replay_review");
    let summary = handoff_summary(parent_task_id.clone(), handoff_id.clone());
    let request = ReviewerGateRequest {
        gate_id: gate_id.clone(),
        handoff_id: handoff_id.clone(),
        parent_task_id,
        requested_decisions: vec![ReviewerDecisionKind::RequestRevision],
        evidence: summary.evidence.clone(),
    };
    let decision = ReviewerGateDecision {
        gate_id: gate_id.clone(),
        handoff_id: handoff_id.clone(),
        decision: ReviewerDecisionKind::RequestRevision,
        reviewer: "reviewer".to_string(),
        reason_code: "needs_more_tests".to_string(),
        comment: Some("attach test output evidence".to_string()),
    };

    for record in [
        EventFrame::new(
            "trace_handoff_replay",
            1,
            RunEvent::AgentHandoffRecorded { summary },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_handoff_replay",
            2,
            RunEvent::ReviewerGateRequested { request },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_handoff_replay",
            3,
            RunEvent::ReviewerGateResolved { decision },
        )
        .to_trace_record(),
    ] {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.handoffs[0].handoff_id, handoff_id);
    assert_eq!(snapshot.reviewer_gates[0].gate_id, gate_id);
    assert_eq!(
        snapshot.reviewer_gates[0].status,
        ClientReviewerGateStatus::RevisionRequested
    );
    assert_eq!(
        snapshot.reviewer_gates[0].reason_code.as_deref(),
        Some("needs_more_tests")
    );
}

fn subagent_session(status: SubagentSessionStatus) -> SubagentSessionDescriptor {
    SubagentSessionDescriptor {
        session_id: SubagentSessionId::from_static("subagent_session_review"),
        parent_task_id: TaskId::from_static("task_parent_subagent"),
        child_task_id: Some(TaskId::from_static("task_child_subagent")),
        profile_id: tessera_protocol::AgentProfileId::from_static("agent_profile_reviewer"),
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

fn subagent_runtime_decision(kind: SubagentRuntimeDecisionKind) -> SubagentRuntimeDecision {
    let session = subagent_session(SubagentSessionStatus::Planned);
    SubagentRuntimeDecision {
        session_id: session.session_id,
        parent_task_id: session.parent_task_id,
        child_task_id: session.child_task_id,
        kind,
        reason: "caps and reviewer gate are satisfied".to_string(),
        caps_snapshot: session.caps,
    }
}

fn subagent_transcript_record() -> SubagentTranscriptArtifactRecord {
    SubagentTranscriptArtifactRecord {
        session_id: SubagentSessionId::from_static("subagent_session_review"),
        parent_task_id: TaskId::from_static("task_parent_subagent"),
        child_task_id: Some(TaskId::from_static("task_child_subagent")),
        artifact_id: ArtifactId::from_static("artifact_child_transcript"),
        event_range: EventRange {
            start_seq: 11,
            end_seq: 19,
        },
        summary_label: Some("child transcript summary".to_string()),
    }
}

fn subagent_transcript_lifecycle(
    status: SubagentTranscriptArtifactStatus,
    event_range: Option<EventRange>,
    reason: &str,
) -> SubagentTranscriptArtifactLifecycleRecord {
    SubagentTranscriptArtifactLifecycleRecord {
        session_id: SubagentSessionId::from_static("subagent_session_review"),
        parent_task_id: TaskId::from_static("task_parent_subagent"),
        child_task_id: Some(TaskId::from_static("task_child_subagent")),
        artifact_id: ArtifactId::from_static("artifact_child_transcript_lifecycle"),
        status,
        event_range,
        summary_label: Some("child transcript lifecycle".to_string()),
        reason: reason.to_string(),
    }
}

fn subagent_forwarding_record() -> SubagentApprovalForwardingRecord {
    SubagentApprovalForwardingRecord {
        session_id: SubagentSessionId::from_static("subagent_session_review"),
        parent_task_id: TaskId::from_static("task_parent_subagent"),
        approval_id: ApprovalId::from_static("approval_subagent_review"),
        reviewer_gate_id: Some(ReviewerGateId::from_static("gate_subagent_review")),
        status: SubagentApprovalForwardingStatus::QueuedForReviewer,
        reason: "child task is inactive".to_string(),
    }
}

fn subagent_inactive_record() -> SubagentInactivePolicyRecord {
    SubagentInactivePolicyRecord {
        session_id: SubagentSessionId::from_static("subagent_session_review"),
        parent_task_id: TaskId::from_static("task_parent_subagent"),
        policy: SubagentInactivePolicy::RequireReviewer,
        parent_action: SubagentInactiveParentAction::PauseParent,
        reason: "reviewer must inspect inactive child".to_string(),
    }
}

fn subagent_cancellation_record() -> SubagentCancellationRecord {
    SubagentCancellationRecord {
        session_id: SubagentSessionId::from_static("subagent_session_review"),
        parent_task_id: TaskId::from_static("task_parent_subagent"),
        source_task_id: TaskId::from_static("task_parent_subagent"),
        reason: "parent cancelled".to_string(),
        cascade: SubagentCancellationCascade::CancelChild,
    }
}

#[test]
fn client_snapshot_projects_subagent_session_metadata_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_live",
        1,
        RunEvent::SubagentSessionPlanned {
            session: subagent_session(SubagentSessionStatus::Planned),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_live",
        2,
        RunEvent::SubagentSessionStarted {
            session: subagent_session(SubagentSessionStatus::Active),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_live",
        3,
        RunEvent::SubagentSessionWaitingForApproval {
            session: subagent_session(SubagentSessionStatus::WaitingForApproval),
        },
    ));

    assert_eq!(snapshot.subagent_sessions.len(), 1);
    assert_eq!(
        snapshot.subagent_sessions[0].session_id,
        SubagentSessionId::from_static("subagent_session_review")
    );
    assert_eq!(
        snapshot.subagent_sessions[0].status,
        ClientSubagentSessionStatus::WaitingForApproval
    );
    assert_eq!(snapshot.subagent_sessions[0].max_steps, 4);
    assert_eq!(snapshot.subagent_sessions[0].max_depth, 1);
    assert_eq!(snapshot.subagent_sessions[0].estimated_cost, Some(0.02));
    assert_eq!(
        snapshot.subagent_sessions[0].transcript_artifact_id,
        Some(ArtifactId::from_static("artifact_child_transcript"))
    );
    assert_eq!(
        snapshot.status.subagent_summary,
        "subagents 1 / active 0 / waiting 1 / inactive 0"
    );
}

#[test]
fn client_snapshot_projects_subagent_session_metadata_from_replayed_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for record in [
        EventFrame::new(
            "trace_subagent_replay",
            1,
            RunEvent::SubagentSessionPlanned {
                session: subagent_session(SubagentSessionStatus::Planned),
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_subagent_replay",
            2,
            RunEvent::SubagentSessionInactive {
                session: subagent_session(SubagentSessionStatus::Inactive),
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_subagent_replay",
            3,
            RunEvent::SubagentSessionCompleted {
                session: subagent_session(SubagentSessionStatus::HandedOff),
            },
        )
        .to_trace_record(),
    ] {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.subagent_sessions.len(), 1);
    assert_eq!(
        snapshot.subagent_sessions[0].status,
        ClientSubagentSessionStatus::HandedOff
    );
    assert_eq!(
        snapshot.subagent_sessions[0]
            .approval_inactive_policy
            .as_deref(),
        Some("require_reviewer")
    );
    assert_eq!(
        snapshot.status.subagent_summary,
        "subagents 1 / active 0 / waiting 0 / inactive 0"
    );
}

#[test]
fn client_snapshot_projects_subagent_transcript_artifact_lifecycle_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for (seq, lifecycle) in [
        (
            1,
            subagent_transcript_lifecycle(
                SubagentTranscriptArtifactStatus::Reserved,
                None,
                "reserved before child runtime starts",
            ),
        ),
        (
            2,
            subagent_transcript_lifecycle(
                SubagentTranscriptArtifactStatus::Published,
                Some(EventRange {
                    start_seq: 11,
                    end_seq: 19,
                }),
                "published transcript event range",
            ),
        ),
        (
            3,
            subagent_transcript_lifecycle(
                SubagentTranscriptArtifactStatus::Sealed,
                Some(EventRange {
                    start_seq: 11,
                    end_seq: 22,
                }),
                "sealed after child completion",
            ),
        ),
        (
            4,
            subagent_transcript_lifecycle(
                SubagentTranscriptArtifactStatus::Abandoned,
                None,
                "abandoned without transcript body",
            ),
        ),
    ] {
        snapshot.apply_event(&EventFrame::new(
            "trace_subagent_transcript_lifecycle_live",
            seq,
            RunEvent::SubagentTranscriptArtifactLifecycleRecorded { lifecycle },
        ));
    }

    assert_eq!(snapshot.subagent_transcript_lifecycles.len(), 4);
    assert_eq!(
        snapshot.subagent_transcript_lifecycles[0].status,
        ClientSubagentTranscriptArtifactStatus::Reserved
    );
    assert_eq!(
        snapshot.subagent_transcript_lifecycles[1].event_range,
        Some(EventRange {
            start_seq: 11,
            end_seq: 19,
        })
    );
    assert_eq!(
        snapshot.subagent_transcript_lifecycles[3].reason,
        "abandoned without transcript body"
    );

    let lifecycle_artifact = snapshot
        .artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_id == ArtifactId::from_static("artifact_child_transcript_lifecycle")
        })
        .expect("lifecycle projects an agent transcript artifact handle");
    assert_eq!(lifecycle_artifact.kind, Some(ArtifactKind::AgentTranscript));
    assert!(lifecycle_artifact
        .referenced_by_event_kinds
        .contains(&"subagent_transcript_artifact_lifecycle_recorded".to_string()));
    assert_eq!(
        snapshot.status.subagent_transcript_lifecycle_summary,
        "transcript lifecycles reserved 1 / published 1 / sealed 1 / abandoned 1"
    );
}

#[test]
fn client_snapshot_projects_subagent_transcript_artifact_lifecycle_from_replayed_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for record in [
        EventFrame::new(
            "trace_subagent_transcript_lifecycle_replay",
            1,
            RunEvent::SubagentTranscriptArtifactLifecycleRecorded {
                lifecycle: subagent_transcript_lifecycle(
                    SubagentTranscriptArtifactStatus::Reserved,
                    None,
                    "reserved before replay",
                ),
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_subagent_transcript_lifecycle_replay",
            2,
            RunEvent::SubagentTranscriptArtifactLifecycleRecorded {
                lifecycle: subagent_transcript_lifecycle(
                    SubagentTranscriptArtifactStatus::Sealed,
                    Some(EventRange {
                        start_seq: 21,
                        end_seq: 29,
                    }),
                    "sealed in replay",
                ),
            },
        )
        .to_trace_record(),
    ] {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.subagent_transcript_lifecycles.len(), 2);
    assert_eq!(
        snapshot.subagent_transcript_lifecycles[1].status,
        ClientSubagentTranscriptArtifactStatus::Sealed
    );
    assert_eq!(
        snapshot.subagent_transcript_lifecycles[1].event_range,
        Some(EventRange {
            start_seq: 21,
            end_seq: 29,
        })
    );
    assert_eq!(
        snapshot.status.subagent_transcript_lifecycle_summary,
        "transcript lifecycles reserved 1 / published 0 / sealed 1 / abandoned 0"
    );
    assert!(snapshot.artifacts.iter().any(|artifact| {
        artifact.kind == Some(ArtifactKind::AgentTranscript)
            && artifact
                .referenced_by_event_kinds
                .contains(&"subagent_transcript_artifact_lifecycle_recorded".to_string())
    }));
}

#[test]
fn client_snapshot_projects_subagent_runtime_ownership_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_runtime_live",
        1,
        RunEvent::SubagentRuntimeDecisionRecorded {
            decision: subagent_runtime_decision(SubagentRuntimeDecisionKind::StartAllowed),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_runtime_live",
        2,
        RunEvent::SubagentTranscriptArtifactRecorded {
            transcript: subagent_transcript_record(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_runtime_live",
        3,
        RunEvent::SubagentApprovalForwardingRecorded {
            forwarding: subagent_forwarding_record(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_runtime_live",
        4,
        RunEvent::SubagentInactivePolicyRecorded {
            inactive: subagent_inactive_record(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_subagent_runtime_live",
        5,
        RunEvent::SubagentCancellationRecorded {
            cancellation: subagent_cancellation_record(),
        },
    ));

    assert_eq!(snapshot.subagent_runtime_decisions.len(), 1);
    assert_eq!(
        snapshot.subagent_runtime_decisions[0].kind,
        ClientSubagentRuntimeDecisionKind::StartAllowed
    );
    assert_eq!(snapshot.subagent_runtime_decisions[0].max_steps, 4);
    assert_eq!(snapshot.subagent_transcripts.len(), 1);
    assert_eq!(
        snapshot.subagent_transcripts[0].artifact_id,
        ArtifactId::from_static("artifact_child_transcript")
    );
    assert_eq!(snapshot.subagent_approval_forwarding.len(), 1);
    assert_eq!(
        snapshot.subagent_approval_forwarding[0].status,
        ClientSubagentApprovalForwardingStatus::QueuedForReviewer
    );
    assert_eq!(snapshot.subagent_inactive_policies.len(), 1);
    assert_eq!(
        snapshot.subagent_inactive_policies[0].parent_action,
        ClientSubagentInactiveParentAction::PauseParent
    );
    assert_eq!(snapshot.subagent_cancellations.len(), 1);
    assert_eq!(
        snapshot.subagent_cancellations[0].cascade,
        ClientSubagentCancellationCascade::CancelChild
    );

    let transcript_artifact = snapshot
        .artifacts
        .iter()
        .find(|artifact| {
            artifact.artifact_id == ArtifactId::from_static("artifact_child_transcript")
        })
        .expect("transcript artifact is projected as an artifact handle");
    assert_eq!(
        transcript_artifact.kind,
        Some(ArtifactKind::AgentTranscript)
    );
    assert!(transcript_artifact
        .referenced_by_event_kinds
        .contains(&"subagent_transcript_artifact_recorded".to_string()));
    assert_eq!(
        snapshot.status.subagent_runtime_summary,
        "subagent runtime decisions 1 / transcripts 1 / forwarding queued 1 / inactive require_reviewer 1 / cancellations 1"
    );
}

#[test]
fn client_snapshot_projects_subagent_runtime_ownership_from_replayed_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    for record in [
        EventFrame::new(
            "trace_subagent_runtime_replay",
            1,
            RunEvent::SubagentRuntimeDecisionRecorded {
                decision: subagent_runtime_decision(SubagentRuntimeDecisionKind::RequireReviewer),
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_subagent_runtime_replay",
            2,
            RunEvent::SubagentTranscriptArtifactRecorded {
                transcript: subagent_transcript_record(),
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_subagent_runtime_replay",
            3,
            RunEvent::SubagentApprovalForwardingRecorded {
                forwarding: subagent_forwarding_record(),
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_subagent_runtime_replay",
            4,
            RunEvent::SubagentInactivePolicyRecorded {
                inactive: subagent_inactive_record(),
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_subagent_runtime_replay",
            5,
            RunEvent::SubagentCancellationRecorded {
                cancellation: subagent_cancellation_record(),
            },
        )
        .to_trace_record(),
    ] {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(
        snapshot.subagent_runtime_decisions[0].kind,
        ClientSubagentRuntimeDecisionKind::RequireReviewer
    );
    assert_eq!(
        snapshot.subagent_transcripts[0].summary_label.as_deref(),
        Some("child transcript summary")
    );
    assert_eq!(
        snapshot.subagent_inactive_policies[0].policy.as_deref(),
        Some("require_reviewer")
    );
    assert_eq!(
        snapshot.status.subagent_runtime_summary,
        "subagent runtime decisions 1 / transcripts 1 / forwarding queued 1 / inactive require_reviewer 1 / cancellations 1"
    );
}

#[test]
fn client_snapshot_resets_thread_and_exports_markdown_projection() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    snapshot.apply_event(&EventFrame::new(
        "trace_export",
        1,
        RunEvent::UserMessageRecorded {
            item_id: ItemId::from_static("item_user_export"),
            text: "hello export".to_string(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_export",
        2,
        RunEvent::AssistantDelta {
            item_id: ItemId::from_static("item_assistant_export"),
            text: "exported answer".to_string(),
        },
    ));

    let markdown = snapshot.export_markdown();
    assert!(markdown.contains("# Tessera Export"));
    assert!(markdown.contains("## User"));
    assert!(markdown.contains("hello export"));
    assert!(markdown.contains("## Assistant"));
    assert!(markdown.contains("exported answer"));

    snapshot.start_new_thread();

    assert!(snapshot.projection.messages.is_empty());
    assert_eq!(snapshot.draft_input, "");
    assert_eq!(snapshot.status.active_profile, "mock-default");
}

#[test]
fn client_snapshot_updates_status_from_live_usage_reported_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.apply_event(&EventFrame::new(
        "trace_usage",
        1,
        RunEvent::ProviderCapabilityReported {
            provider_id: ProviderId::from_static("mock"),
            capability: ProviderCapability {
                provider_id: ProviderId::from_static("mock"),
                supports_streaming: true,
                supports_reasoning_delta: true,
                supports_cache_telemetry: true,
                supports_cost_estimate: true,
                supports_tool_calling: false,
                max_context_tokens: Some(4_000),
                extension: None,
            },
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_usage",
        2,
        RunEvent::UsageReported {
            input_tokens: Some(1_000),
            output_tokens: Some(200),
            total_tokens: Some(1_200),
            cache_read_tokens: Some(800),
            cache_write_tokens: None,
            cache_miss_tokens: Some(200),
            estimated_cost: Some(CostEstimate {
                amount: 0.0123,
                currency: "USD".to_string(),
                input_cost: Some(0.0100),
                output_cost: Some(0.0023),
                cache_read_cost: Some(0.0010),
                cache_write_cost: None,
            }),
            latency_ms: Some(42),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_usage",
        3,
        RunEvent::UsageReported {
            input_tokens: Some(500),
            output_tokens: Some(50),
            total_tokens: Some(550),
            cache_read_tokens: Some(300),
            cache_write_tokens: None,
            cache_miss_tokens: Some(200),
            estimated_cost: Some(CostEstimate {
                amount: 0.0057,
                currency: "USD".to_string(),
                input_cost: Some(0.0040),
                output_cost: Some(0.0017),
                cache_read_cost: Some(0.0003),
                cache_write_cost: None,
            }),
            latency_ms: Some(24),
        },
    ));

    assert_eq!(
        snapshot.status.usage_summary,
        "usage in 1500 / out 250 / total 1750"
    );
    assert_eq!(snapshot.status.cache_summary, "cache 1100/1500 (73%)");
    assert_eq!(snapshot.status.cost_summary, "USD 0.0180");
    assert_eq!(snapshot.status.context_summary, "ctx 500/4000 (12%)");
}

#[test]
fn client_snapshot_updates_status_from_replayed_usage_reported_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let record = EventFrame::new(
        "trace_usage_replay",
        1,
        RunEvent::UsageReported {
            input_tokens: Some(2_000),
            output_tokens: Some(300),
            total_tokens: Some(2_300),
            cache_read_tokens: Some(1_500),
            cache_write_tokens: None,
            cache_miss_tokens: Some(500),
            estimated_cost: Some(CostEstimate {
                amount: 0.0456,
                currency: "CNY".to_string(),
                input_cost: Some(0.0300),
                output_cost: Some(0.0156),
                cache_read_cost: Some(0.0030),
                cache_write_cost: None,
            }),
            latency_ms: Some(84),
        },
    )
    .to_trace_record();

    snapshot.apply_trace_record(&record);

    assert_eq!(
        snapshot.status.usage_summary,
        "usage in 2000 / out 300 / total 2300"
    );
    assert_eq!(snapshot.status.cache_summary, "cache 1500/2000 (75%)");
    assert_eq!(snapshot.status.cost_summary, "CNY 0.0456");
    assert_eq!(snapshot.status.context_summary, "ctx 2000 tokens");
}

#[test]
fn client_snapshot_uses_input_tokens_as_cache_denominator_when_miss_tokens_are_absent() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.apply_event(&EventFrame::new(
        "trace_cache_denominator",
        1,
        RunEvent::UsageReported {
            input_tokens: Some(1_000),
            output_tokens: Some(200),
            total_tokens: Some(1_200),
            cache_read_tokens: Some(800),
            cache_write_tokens: None,
            cache_miss_tokens: None,
            estimated_cost: None,
            latency_ms: None,
        },
    ));

    assert_eq!(snapshot.status.cache_summary, "cache 800/1000 (80%)");
}

#[test]
fn client_snapshot_updates_context_summary_from_replayed_provider_capability_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let capability_record = EventFrame::new(
        "trace_context_replay",
        1,
        RunEvent::ProviderCapabilityReported {
            provider_id: ProviderId::from_static("mock"),
            capability: ProviderCapability {
                provider_id: ProviderId::from_static("mock"),
                supports_streaming: true,
                supports_reasoning_delta: true,
                supports_cache_telemetry: true,
                supports_cost_estimate: true,
                supports_tool_calling: false,
                max_context_tokens: Some(8_000),
                extension: None,
            },
        },
    )
    .to_trace_record();
    let usage_record = EventFrame::new(
        "trace_context_replay",
        2,
        RunEvent::UsageReported {
            input_tokens: Some(2_000),
            output_tokens: Some(300),
            total_tokens: Some(2_300),
            cache_read_tokens: None,
            cache_write_tokens: None,
            cache_miss_tokens: None,
            estimated_cost: None,
            latency_ms: None,
        },
    )
    .to_trace_record();

    snapshot.apply_trace_record(&capability_record);
    snapshot.apply_trace_record(&usage_record);

    assert_eq!(snapshot.status.context_summary, "ctx 2000/8000 (25%)");
}

#[test]
fn client_snapshot_updates_task_registry_from_live_task_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let task_id = TaskId::from_static("task_live");

    snapshot.apply_event(&EventFrame::new(
        "trace_task_live",
        1,
        RunEvent::TaskCreated {
            task_id: task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_task_live",
        2,
        RunEvent::TaskStarted {
            task_id: task_id.clone(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_task_live",
        3,
        RunEvent::TaskCompleted {
            task_id: task_id.clone(),
        },
    ));

    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(snapshot.tasks[0].task_id, task_id);
    assert_eq!(snapshot.tasks[0].kind, Some(TaskKind::Chat));
    assert_eq!(snapshot.tasks[0].status, TaskStatus::Completed);
    assert!(snapshot.tasks[0].created_at.is_some());
    assert!(snapshot.tasks[0].started_at.is_some());
    assert!(snapshot.tasks[0].finished_at.is_some());
    assert_eq!(snapshot.status.task_summary, "task completed");
}

fn client_owner_lease(trace_id: &str, task_id: TaskId, lease_id: &'static str) -> TaskOwnerLease {
    TaskOwnerLease {
        lease_id: TaskOwnershipId::from_static(lease_id),
        task_id,
        trace_id: trace_id.to_string(),
        runtime_id: RuntimeInstanceId::from_static("runtime_client_owner"),
        client_id: Some(ClientInstanceId::from_static("client_client_owner")),
        owner_kind: TaskOwnerKind::Execution,
        status: TaskOwnerStatus::Attached,
        acquired_at: Timestamp::now_utc(),
        heartbeat_interval_ms: 5_000,
        expires_at: None,
        last_heartbeat_at: None,
        last_seq: None,
        reason: Some("client owner attached".to_string()),
    }
}

#[test]
fn client_snapshot_projects_live_task_owner_status_without_runtime_work() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let trace_id = "trace_client_owner_live";
    let task_id = TaskId::from_static("task_client_owner_live");
    let lease = client_owner_lease(trace_id, task_id.clone(), "task_owner_client_live");
    let heartbeat_at = Timestamp::now_utc();
    let expires_at = Timestamp::now_utc();

    snapshot.apply_event(&EventFrame::new(
        trace_id,
        1,
        RunEvent::TaskCreated {
            task_id: task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        trace_id,
        2,
        RunEvent::TaskOwnerAttached {
            lease: Box::new(lease.clone()),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        trace_id,
        3,
        RunEvent::TaskOwnerHeartbeat {
            heartbeat: TaskOwnerHeartbeat {
                lease_id: lease.lease_id.clone(),
                task_id: task_id.clone(),
                runtime_id: lease.runtime_id.clone(),
                heartbeat_at: heartbeat_at.clone(),
                expires_at: expires_at.clone(),
                last_seq: 9,
            },
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        trace_id,
        4,
        RunEvent::TaskOwnerLost {
            lease_id: lease.lease_id.clone(),
            task_id: task_id.clone(),
            reason: Some("heartbeat expired".to_string()),
        },
    ));

    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(snapshot.tasks[0].owner_lease_id, Some(lease.lease_id));
    assert_eq!(
        snapshot.tasks[0].owner_runtime_id,
        Some(RuntimeInstanceId::from_static("runtime_client_owner"))
    );
    assert_eq!(snapshot.tasks[0].owner_status, Some(TaskOwnerStatus::Lost));
    assert_eq!(
        snapshot.tasks[0].owner_reattach_mode,
        Some(TaskReattachMode::OwnerLost)
    );
    assert_eq!(
        snapshot.tasks[0].owner_last_heartbeat_at,
        Some(heartbeat_at)
    );
    assert_eq!(snapshot.tasks[0].owner_expires_at, Some(expires_at));
    assert_eq!(snapshot.tasks[0].owner_last_seq, Some(9));
    assert_eq!(
        snapshot.tasks[0].owner_reason.as_deref(),
        Some("heartbeat expired")
    );
}

#[test]
fn client_snapshot_projects_replayed_task_owner_detach_and_terminal_mode() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let trace_id = "trace_client_owner_replay";
    let task_id = TaskId::from_static("task_client_owner_replay");
    let lease = client_owner_lease(trace_id, task_id.clone(), "task_owner_client_replay");

    for frame in [
        EventFrame::new(
            trace_id,
            1,
            RunEvent::TaskCreated {
                task_id: task_id.clone(),
                kind: TaskKind::Chat,
            },
        ),
        EventFrame::new(
            trace_id,
            2,
            RunEvent::TaskOwnerAttached {
                lease: Box::new(lease.clone()),
            },
        ),
        EventFrame::new(
            trace_id,
            3,
            RunEvent::TaskOwnerDetached {
                lease_id: lease.lease_id.clone(),
                task_id: task_id.clone(),
                reason: Some("terminal cleanup".to_string()),
            },
        ),
        EventFrame::new(
            trace_id,
            4,
            RunEvent::TaskCompleted {
                task_id: task_id.clone(),
            },
        ),
    ] {
        snapshot.apply_trace_record(&frame.to_trace_record());
    }

    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(
        snapshot.tasks[0].owner_status,
        Some(TaskOwnerStatus::Detached)
    );
    assert_eq!(
        snapshot.tasks[0].owner_reattach_mode,
        Some(TaskReattachMode::TerminalProjection)
    );
    assert_eq!(
        snapshot.tasks[0].owner_reason.as_deref(),
        Some("terminal cleanup")
    );
}

#[test]
fn client_snapshot_updates_task_registry_from_replayed_failed_and_cancelled_tasks() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let failed_task_id = TaskId::from_static("task_failed_replay");
    let cancelled_task_id = TaskId::from_static("task_cancelled_replay");

    let records = [
        EventFrame::new(
            "trace_task_replay",
            1,
            RunEvent::TaskCreated {
                task_id: failed_task_id.clone(),
                kind: TaskKind::Chat,
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_task_replay",
            2,
            RunEvent::TaskFailed {
                task_id: failed_task_id.clone(),
                error: NormalizedError {
                    code: "provider_rate_limited".to_string(),
                    message: "provider rate limit reached".to_string(),
                    retryable: true,
                    source: ErrorSource::Provider,
                    details: None,
                },
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_task_replay",
            3,
            RunEvent::TaskCreated {
                task_id: cancelled_task_id.clone(),
                kind: TaskKind::Replay,
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_task_replay",
            4,
            RunEvent::TaskCancelled {
                task_id: cancelled_task_id.clone(),
                reason: Some("client stopped".to_string()),
            },
        )
        .to_trace_record(),
    ];

    for record in records {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.tasks.len(), 2);
    assert_eq!(snapshot.tasks[0].task_id, failed_task_id);
    assert_eq!(snapshot.tasks[0].status, TaskStatus::Failed);
    assert_eq!(
        snapshot.tasks[0].error_code.as_deref(),
        Some("provider_rate_limited")
    );
    assert_eq!(
        snapshot.tasks[0].error_message.as_deref(),
        Some("provider rate limit reached")
    );
    assert_eq!(snapshot.tasks[1].task_id, cancelled_task_id);
    assert_eq!(snapshot.tasks[1].status, TaskStatus::Cancelled);
    assert_eq!(
        snapshot.tasks[1].cancel_reason.as_deref(),
        Some("client stopped")
    );
    assert_eq!(snapshot.status.task_summary, "task cancelled");
}

#[test]
fn client_snapshot_projects_paused_and_resumed_tasks() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let task_id = TaskId::from_static("task_pause_projection");

    snapshot.apply_event(&EventFrame::new(
        "trace_task_pause_live",
        1,
        RunEvent::TaskCreated {
            task_id: task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_task_pause_live",
        2,
        RunEvent::TaskStarted {
            task_id: task_id.clone(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_task_pause_live",
        3,
        RunEvent::TaskPaused {
            task_id: task_id.clone(),
            reason: Some("user requested pause".to_string()),
        },
    ));

    assert_eq!(snapshot.tasks[0].status, TaskStatus::Paused);
    assert_eq!(snapshot.status.task_summary, "task paused");

    snapshot.apply_event(&EventFrame::new(
        "trace_task_pause_live",
        4,
        RunEvent::TaskResumed {
            task_id: task_id.clone(),
            reason: Some("user requested resume".to_string()),
        },
    ));

    assert_eq!(snapshot.tasks[0].status, TaskStatus::Running);
    assert_eq!(snapshot.status.task_summary, "task running");

    let mut replayed = ClientSnapshot::new("mock-default");
    replayed.apply_trace_record(
        &EventFrame::new(
            "trace_task_pause_replay",
            1,
            RunEvent::TaskPaused {
                task_id: task_id.clone(),
                reason: Some("trace replay pause".to_string()),
            },
        )
        .to_trace_record(),
    );

    assert_eq!(replayed.tasks[0].task_id, task_id);
    assert_eq!(replayed.tasks[0].status, TaskStatus::Paused);
    assert_eq!(replayed.status.task_summary, "task paused");
}

#[test]
fn client_snapshot_projects_artifact_handles_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let artifact_id = ArtifactId::from_static("artifact_live");
    let task_id = TaskId::from_static("task_artifact_live");
    let item_id = ItemId::from_static("item_artifact_live");

    snapshot.apply_event(
        &EventFrame::new(
            "trace_artifact_live",
            1,
            RunEvent::ArtifactCreated {
                artifact_id: artifact_id.clone(),
                kind: ArtifactKind::Export,
            },
        )
        .with_task_id(task_id.clone()),
    );
    snapshot.apply_event(
        &EventFrame::new(
            "trace_artifact_live",
            2,
            RunEvent::AssistantDelta {
                item_id: item_id.clone(),
                text: "see artifact".to_string(),
            },
        )
        .with_task_id(task_id.clone())
        .with_item_id(item_id.clone())
        .with_artifact_ref(artifact_id.clone()),
    );

    assert_eq!(snapshot.artifacts.len(), 1);
    assert_eq!(snapshot.artifacts[0].artifact_id, artifact_id);
    assert_eq!(snapshot.artifacts[0].kind, Some(ArtifactKind::Export));
    assert_eq!(snapshot.artifacts[0].task_id, Some(task_id));
    assert_eq!(snapshot.artifacts[0].item_id, Some(item_id));
    assert!(snapshot.artifacts[0].created_at.is_some());
    assert_eq!(
        snapshot.artifacts[0].referenced_by_event_kinds,
        vec!["assistant_delta"]
    );
    assert_eq!(snapshot.status.artifact_summary, "artifacts 1");
}

#[test]
fn client_snapshot_projects_artifact_handles_from_replayed_trace_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let artifact_id = ArtifactId::from_static("artifact_replay");
    let task_id = TaskId::from_static("task_artifact_replay");

    let records = [
        EventFrame::new(
            "trace_artifact_replay",
            1,
            RunEvent::AssistantDelta {
                item_id: ItemId::from_static("item_artifact_replay"),
                text: "artifact ref first".to_string(),
            },
        )
        .with_task_id(task_id.clone())
        .with_artifact_ref(artifact_id.clone())
        .to_trace_record(),
        EventFrame::new(
            "trace_artifact_replay",
            2,
            RunEvent::ArtifactCreated {
                artifact_id: artifact_id.clone(),
                kind: ArtifactKind::TestReport,
            },
        )
        .with_task_id(task_id.clone())
        .to_trace_record(),
    ];

    for record in records {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.artifacts.len(), 1);
    assert_eq!(snapshot.artifacts[0].artifact_id, artifact_id);
    assert_eq!(snapshot.artifacts[0].kind, Some(ArtifactKind::TestReport));
    assert_eq!(snapshot.artifacts[0].task_id, Some(task_id));
    assert_eq!(
        snapshot.artifacts[0].referenced_by_event_kinds,
        vec!["assistant_delta"]
    );
    assert_eq!(snapshot.status.artifact_summary, "artifacts 1");
}
