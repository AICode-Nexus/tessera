use tessera_core::{
    TraceApplyPatchResolveRequest, TraceApplyPatchResolver, TraceApplyPatchSelector,
};
use tessera_protocol::{
    AgentHandoffId, ArtifactBodyRecord, ArtifactBodyRedactionStatus, ArtifactId, ArtifactKind,
    CodingWorkflowId, EventFrame, EventRange, HandoffEvidenceKind, HandoffEvidenceRef,
    MutationMode, MutationRequestId, MutationRequestOperationKind, MutationRequestProposal,
    MutationRequestStatus, PatchProposal, PatchProposalId, PolicyDecisionId, PolicyOutcome,
    ReviewerDecisionKind, ReviewerGateDecision, ReviewerGateId, RunEvent, SnapshotId, TaskId,
    ToolCallId, ToolId, ToolPermission, ToolPolicyDecision, ToolSideEffect, TraceRecord,
    WorkspaceCheckpointLifecycleRecord, WorkspaceCheckpointLifecycleStatus, WorkspaceMutationScope,
};

fn trace_id() -> &'static str {
    "trace_apply_patch_resolution"
}

fn workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from_static("coding_workflow_trace_apply_patch")
}

fn task_id() -> TaskId {
    TaskId::from_static("task_trace_apply_patch")
}

fn request_id() -> MutationRequestId {
    MutationRequestId::from_static("mutation_request_trace_apply_patch")
}

fn patch_id() -> PatchProposalId {
    PatchProposalId::from_static("patch_proposal_trace_apply_patch")
}

fn checkpoint_id() -> SnapshotId {
    SnapshotId::from_static("snapshot_trace_apply_patch")
}

fn reviewer_gate_id() -> ReviewerGateId {
    ReviewerGateId::from_static("reviewer_gate_trace_apply_patch")
}

fn policy_decision_id() -> PolicyDecisionId {
    PolicyDecisionId::from_static("policy_trace_apply_patch")
}

fn patch_artifact_id() -> ArtifactId {
    ArtifactId::from_static("artifact_trace_apply_patch_diff")
}

fn patch_evidence() -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::DiffArtifact,
        artifact_id: Some(patch_artifact_id()),
        trace_id: Some(trace_id().to_string()),
        event_range: Some(EventRange {
            start_seq: 5,
            end_seq: 6,
        }),
        label: Some("reviewed patch artifact".to_string()),
        summary: Some("patch body stored outside trace".to_string()),
    }
}

fn mutation_scope() -> WorkspaceMutationScope {
    WorkspaceMutationScope {
        workflow_id: workflow_id(),
        task_id: task_id(),
        root_label: "project".to_string(),
        allowed_paths: vec!["docs/README.md".to_string(), "docs/guide.md".to_string()],
        denied_paths: vec![".env".to_string()],
        mutation_mode: MutationMode::WorktreeFirst,
        worktree_required: true,
        reason: Some("trace-backed isolated mutation".to_string()),
    }
}

fn mutation_request() -> MutationRequestProposal {
    MutationRequestProposal {
        request_id: request_id(),
        workflow_id: workflow_id(),
        task_id: task_id(),
        operation: MutationRequestOperationKind::PatchApplication,
        status: MutationRequestStatus::Approved,
        summary: "apply reviewed docs patch".to_string(),
        requested_paths: vec!["docs/README.md".to_string()],
        required_checkpoint_id: Some(checkpoint_id()),
        reviewer_gate_id: Some(reviewer_gate_id()),
        policy_decision_id: Some(policy_decision_id()),
        sandbox_profile_label: Some("workspace_write_isolated".to_string()),
        worktree_required: true,
        evidence: vec![patch_evidence()],
    }
}

fn patch_proposal() -> PatchProposal {
    PatchProposal {
        patch_id: patch_id(),
        workflow_id: workflow_id(),
        task_id: task_id(),
        summary: "update docs with reviewed trace patch".to_string(),
        touched_paths: vec!["docs/README.md".to_string()],
        diff_artifacts: vec![patch_evidence()],
        risk_labels: vec!["docs_only".to_string()],
        required_checkpoint_id: Some(checkpoint_id()),
        reviewer_gate_id: Some(reviewer_gate_id()),
    }
}

fn patch_artifact_body() -> ArtifactBodyRecord {
    ArtifactBodyRecord {
        artifact_id: patch_artifact_id(),
        kind: ArtifactKind::Patch,
        task_id: Some(task_id()),
        media_type: "text/x-diff".to_string(),
        byte_len: 128,
        storage_uri: "tessera://artifacts/artifact_trace_apply_patch_diff/body".to_string(),
        redaction_status: ArtifactBodyRedactionStatus::Clean,
        summary: Some("clean reviewed patch body".to_string()),
        metadata: None,
    }
}

fn checkpoint_lifecycle() -> WorkspaceCheckpointLifecycleRecord {
    WorkspaceCheckpointLifecycleRecord {
        checkpoint_id: checkpoint_id(),
        task_id: task_id(),
        status: WorkspaceCheckpointLifecycleStatus::Created,
        reason: "checkpoint exists before trace-driven apply".to_string(),
        restore_plan_id: None,
        execution_blocked: true,
        evidence: Vec::new(),
        metadata: None,
    }
}

fn reviewer_decision() -> ReviewerGateDecision {
    ReviewerGateDecision {
        gate_id: reviewer_gate_id(),
        handoff_id: AgentHandoffId::from_static("handoff_trace_apply_patch"),
        decision: ReviewerDecisionKind::Accept,
        reviewer: "human-reviewer".to_string(),
        reason_code: "accepted_for_trace_apply".to_string(),
        comment: Some("Reviewed bundle can be applied.".to_string()),
    }
}

fn policy_decision() -> ToolPolicyDecision {
    ToolPolicyDecision {
        decision_id: policy_decision_id(),
        call_id: ToolCallId::from_static("tool_call_trace_apply_patch"),
        tool_id: ToolId::from_static("tool_trace_apply_patch"),
        outcome: PolicyOutcome::Allow,
        reason: "policy allows isolated apply-patch".to_string(),
        required_permissions: vec![ToolPermission::FilesystemWrite],
        side_effects: vec![ToolSideEffect::WritesWorkspace],
        approval_id: None,
    }
}

fn record(seq: u64, event: RunEvent) -> TraceRecord {
    EventFrame::new(trace_id(), seq, event).to_trace_record()
}

fn reviewed_bundle_records() -> Vec<TraceRecord> {
    vec![
        record(
            1,
            RunEvent::CodingWorkflowStarted {
                workflow_id: workflow_id(),
                task_id: task_id(),
                objective: "trace-backed docs patch".to_string(),
            },
        ),
        record(
            2,
            RunEvent::WorkspaceMutationScopeRecorded {
                scope: mutation_scope(),
            },
        ),
        record(
            3,
            RunEvent::MutationRequestProposalRecorded {
                proposal: mutation_request(),
            },
        ),
        record(
            4,
            RunEvent::PatchProposalRecorded {
                proposal: patch_proposal(),
            },
        ),
        record(
            5,
            RunEvent::ArtifactBodyRecorded {
                record: patch_artifact_body(),
            },
        ),
        record(
            6,
            RunEvent::SnapshotLifecycleRecorded {
                lifecycle: checkpoint_lifecycle(),
            },
        ),
        record(
            7,
            RunEvent::ReviewerGateResolved {
                decision: reviewer_decision(),
            },
        ),
        record(
            8,
            RunEvent::ToolPolicyDecisionRecorded {
                decision: policy_decision(),
            },
        ),
    ]
}

fn resolve(
    records: Vec<TraceRecord>,
) -> Result<tessera_core::TraceApplyPatchResolvedEnvelope, String> {
    TraceApplyPatchResolver::resolve(TraceApplyPatchResolveRequest {
        trace_id: trace_id().to_string(),
        records,
        selector: TraceApplyPatchSelector::default(),
    })
    .map_err(|error| error.to_string())
}

#[test]
fn trace_apply_patch_resolver_builds_executor_envelope_from_reviewed_bundle() {
    let envelope = resolve(reviewed_bundle_records()).expect("reviewed bundle should resolve");

    assert_eq!(envelope.trace_id, trace_id());
    assert_eq!(envelope.workflow_id, workflow_id());
    assert_eq!(envelope.task_id, task_id());
    assert_eq!(envelope.mutation_request.request_id, request_id());
    assert_eq!(envelope.patch_proposal.patch_id, patch_id());
    assert_eq!(
        envelope.mutation_scope.allowed_paths,
        vec!["docs/README.md"]
    );
    assert_eq!(envelope.checkpoint_lifecycle.checkpoint_id, checkpoint_id());
    assert_eq!(envelope.reviewer_decision.gate_id, reviewer_gate_id());
    assert_eq!(envelope.policy_decision.decision_id, policy_decision_id());
    assert_eq!(
        envelope.sandbox_profile_label.as_deref(),
        Some("workspace_write_isolated")
    );
    assert_eq!(envelope.patch_artifact_id, Some(patch_artifact_id()));
    assert_eq!(
        envelope.source_event_range,
        EventRange {
            start_seq: 1,
            end_seq: 8,
        }
    );
    assert!(envelope
        .evidence
        .iter()
        .any(|evidence| evidence.kind == HandoffEvidenceKind::TraceRange
            && evidence.trace_id.as_deref() == Some(trace_id())));
}

#[test]
fn trace_apply_patch_resolver_rejects_ambiguous_workflow_without_selector() {
    let mut records = reviewed_bundle_records();
    records.push(record(
        9,
        RunEvent::CodingWorkflowStarted {
            workflow_id: CodingWorkflowId::from_static("coding_workflow_second"),
            task_id: task_id(),
            objective: "another workflow".to_string(),
        },
    ));

    let error = resolve(records).expect_err("ambiguous workflow should fail");

    assert!(error.contains("ambiguous workflow"));
}

#[test]
fn trace_apply_patch_resolver_rejects_unapproved_mutation_request() {
    let mut records = reviewed_bundle_records();
    let mut proposal = mutation_request();
    proposal.status = MutationRequestStatus::ReviewerPending;
    records[2] = record(3, RunEvent::MutationRequestProposalRecorded { proposal });

    let error = resolve(records).expect_err("unapproved request should fail");

    assert!(error.contains("mutation request is not approved"));
}

#[test]
fn trace_apply_patch_resolver_rejects_missing_created_checkpoint() {
    let records = reviewed_bundle_records()
        .into_iter()
        .filter(|record| record.event_kind != "snapshot_lifecycle_recorded")
        .collect();

    let error = resolve(records).expect_err("missing checkpoint lifecycle should fail");

    assert!(error.contains("missing checkpoint lifecycle"));
}

#[test]
fn trace_apply_patch_resolver_rejects_checkpoint_that_is_not_created() {
    let mut records = reviewed_bundle_records();
    let mut lifecycle = checkpoint_lifecycle();
    lifecycle.status = WorkspaceCheckpointLifecycleStatus::RestoreBlocked;
    records[5] = record(6, RunEvent::SnapshotLifecycleRecorded { lifecycle });

    let error = resolve(records).expect_err("non-created checkpoint should fail");

    assert!(error.contains("checkpoint was not created"));
}

#[test]
fn trace_apply_patch_resolver_rejects_reviewer_decision_that_is_not_accept() {
    let mut records = reviewed_bundle_records();
    let mut decision = reviewer_decision();
    decision.decision = ReviewerDecisionKind::RequestRevision;
    records[6] = record(7, RunEvent::ReviewerGateResolved { decision });

    let error = resolve(records).expect_err("non-accepted review should fail");

    assert!(error.contains("reviewer gate is not accepted"));
}

#[test]
fn trace_apply_patch_resolver_rejects_policy_decision_that_is_not_allow() {
    let mut records = reviewed_bundle_records();
    let mut decision = policy_decision();
    decision.outcome = PolicyOutcome::Deny;
    records[7] = record(8, RunEvent::ToolPolicyDecisionRecorded { decision });

    let error = resolve(records).expect_err("denied policy should fail");

    assert!(error.contains("policy decision is not allowed"));
}

#[test]
fn trace_apply_patch_resolver_rejects_missing_sandbox_profile_label() {
    let mut records = reviewed_bundle_records();
    let mut proposal = mutation_request();
    proposal.sandbox_profile_label = None;
    records[2] = record(3, RunEvent::MutationRequestProposalRecorded { proposal });

    let error = resolve(records).expect_err("missing sandbox profile should fail");

    assert!(error.contains("missing sandbox profile"));
}

#[test]
fn trace_apply_patch_resolver_rejects_patch_paths_outside_scope() {
    let mut records = reviewed_bundle_records();
    let mut proposal = patch_proposal();
    proposal.touched_paths = vec!["src/lib.rs".to_string()];
    records[3] = record(4, RunEvent::PatchProposalRecorded { proposal });

    let error = resolve(records).expect_err("path outside scope should fail");

    assert!(error.contains("patch paths outside mutation scope"));
}

#[test]
fn trace_apply_patch_resolver_rejects_missing_patch_artifact_metadata() {
    let records = reviewed_bundle_records()
        .into_iter()
        .filter(|record| record.event_kind != "artifact_body_recorded")
        .collect();

    let error = resolve(records).expect_err("missing patch artifact metadata should fail");

    assert!(error.contains("missing patch artifact metadata"));
}

#[test]
fn trace_apply_patch_resolver_rejects_redacted_patch_artifact() {
    let mut records = reviewed_bundle_records();
    let mut artifact = patch_artifact_body();
    artifact.redaction_status = ArtifactBodyRedactionStatus::Redacted;
    records[4] = record(5, RunEvent::ArtifactBodyRecorded { record: artifact });

    let error = resolve(records).expect_err("redacted patch artifact should fail");

    assert!(error.contains("patch artifact is not clean"));
}

#[test]
fn trace_apply_patch_resolver_rejects_non_patch_artifact_metadata() {
    let mut records = reviewed_bundle_records();
    let mut artifact = patch_artifact_body();
    artifact.kind = ArtifactKind::TestReport;
    records[4] = record(5, RunEvent::ArtifactBodyRecorded { record: artifact });

    let error = resolve(records).expect_err("non-patch artifact metadata should fail");

    assert!(error.contains("patch artifact is not a patch"));
}

#[test]
fn trace_apply_patch_resolver_rejects_ambiguous_patch_artifact_metadata() {
    let mut records = reviewed_bundle_records();
    records.push(record(
        9,
        RunEvent::ArtifactBodyRecorded {
            record: patch_artifact_body(),
        },
    ));

    let error = resolve(records).expect_err("ambiguous artifact metadata should fail");

    assert!(error.contains("ambiguous patch artifact metadata"));
}
