use tessera_core::{
    ApplyPatchGate, ApplyPatchGateBlocker, ApplyPatchGateRequest, ApplyPatchGateStatus,
    MutationEnforcementPlan, MutationEnforcementPlanRequest, MutationEnforcementPlanner,
};
use tessera_protocol::{
    AgentHandoffId, ArtifactId, CodingWorkflowId, HandoffEvidenceKind, HandoffEvidenceRef,
    MutationMode, MutationRequestId, MutationRequestOperationKind, MutationRequestProposal,
    MutationRequestStatus, PatchProposal, PatchProposalId, PolicyDecisionId, PolicyOutcome,
    ReviewerDecisionKind, ReviewerGateDecision, ReviewerGateId, SnapshotId, TaskId, ToolCallId,
    ToolId, ToolPermission, ToolPolicyDecision, ToolSideEffect, WorkspaceCheckpointLifecycleRecord,
    WorkspaceCheckpointLifecycleStatus, WorkspaceMutationScope,
};

fn workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from_static("coding_workflow_apply_patch_gate")
}

fn task_id() -> TaskId {
    TaskId::from_static("task_apply_patch_gate")
}

fn checkpoint_id() -> SnapshotId {
    SnapshotId::from_static("snapshot_apply_patch_gate")
}

fn reviewer_gate_id() -> ReviewerGateId {
    ReviewerGateId::from_static("reviewer_gate_apply_patch")
}

fn policy_decision_id() -> PolicyDecisionId {
    PolicyDecisionId::from_static("policy_apply_patch_gate")
}

fn diff_artifact() -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::DiffArtifact,
        artifact_id: Some(ArtifactId::from_static("artifact_apply_patch_diff")),
        trace_id: None,
        event_range: None,
        label: Some("apply patch diff".to_string()),
        summary: Some("redacted patch diff metadata".to_string()),
    }
}

fn mutation_request() -> MutationRequestProposal {
    MutationRequestProposal {
        request_id: MutationRequestId::from_static("mutation_request_apply_patch"),
        workflow_id: workflow_id(),
        task_id: task_id(),
        operation: MutationRequestOperationKind::PatchApplication,
        status: MutationRequestStatus::Approved,
        summary: "apply a reviewed docs patch".to_string(),
        requested_paths: vec!["docs/README.md".to_string()],
        required_checkpoint_id: Some(checkpoint_id()),
        reviewer_gate_id: Some(reviewer_gate_id()),
        policy_decision_id: Some(policy_decision_id()),
        sandbox_profile_label: Some("workspace_write".to_string()),
        worktree_required: true,
        evidence: vec![diff_artifact()],
    }
}

fn patch_proposal() -> PatchProposal {
    PatchProposal {
        patch_id: PatchProposalId::from_static("patch_proposal_apply_patch"),
        workflow_id: workflow_id(),
        task_id: task_id(),
        summary: "update docs with reviewed patch".to_string(),
        touched_paths: vec!["docs/README.md".to_string()],
        diff_artifacts: vec![diff_artifact()],
        risk_labels: vec!["docs_only".to_string()],
        required_checkpoint_id: Some(checkpoint_id()),
        reviewer_gate_id: Some(reviewer_gate_id()),
    }
}

fn checkpoint_lifecycle() -> WorkspaceCheckpointLifecycleRecord {
    WorkspaceCheckpointLifecycleRecord {
        checkpoint_id: checkpoint_id(),
        task_id: task_id(),
        status: WorkspaceCheckpointLifecycleStatus::Created,
        reason: "pre-mutation checkpoint exists".to_string(),
        restore_plan_id: None,
        execution_blocked: true,
        evidence: Vec::new(),
        metadata: None,
    }
}

fn reviewer_decision() -> ReviewerGateDecision {
    ReviewerGateDecision {
        gate_id: reviewer_gate_id(),
        handoff_id: AgentHandoffId::from_static("handoff_apply_patch_gate"),
        decision: ReviewerDecisionKind::Accept,
        reviewer: "human-reviewer".to_string(),
        reason_code: "approved_for_preflight".to_string(),
        comment: Some("Patch can enter preflight.".to_string()),
    }
}

fn policy_decision() -> ToolPolicyDecision {
    ToolPolicyDecision {
        decision_id: policy_decision_id(),
        call_id: ToolCallId::from_static("tool_call_apply_patch_gate"),
        tool_id: ToolId::from_static("tool_apply_patch_gate"),
        outcome: PolicyOutcome::Allow,
        reason: "policy permits preflight only".to_string(),
        required_permissions: vec![ToolPermission::FilesystemWrite],
        side_effects: vec![ToolSideEffect::WritesWorkspace],
        approval_id: None,
    }
}

fn enforcement_plan() -> MutationEnforcementPlan {
    MutationEnforcementPlanner::new("/workspace/project")
        .plan(MutationEnforcementPlanRequest {
            workflow_id: workflow_id(),
            task_id: task_id(),
            requested_paths: vec!["docs/README.md".to_string()],
            mutation_mode: None,
            policy_decision_id: Some(policy_decision_id()),
            reason: "apply reviewed docs patch".to_string(),
        })
        .expect("safe mutation metadata should produce enforcement plan")
}

fn valid_request() -> ApplyPatchGateRequest {
    let enforcement_plan = enforcement_plan();
    let mutation_scope = WorkspaceMutationScope {
        root_label: "worktree".to_string(),
        ..enforcement_plan.scope.clone()
    };
    ApplyPatchGateRequest {
        workflow_id: workflow_id(),
        task_id: task_id(),
        mutation_request: mutation_request(),
        patch_proposal: patch_proposal(),
        mutation_scope,
        enforcement_plan,
        checkpoint_lifecycle: Some(checkpoint_lifecycle()),
        reviewer_decision: Some(reviewer_decision()),
        policy_decision: Some(policy_decision()),
        operator_label: "coding-agent".to_string(),
    }
}

#[test]
fn apply_patch_gate_rejects_missing_policy_reviewer_checkpoint_or_sandbox() {
    let gate = ApplyPatchGate;
    let mut request = valid_request();
    request.checkpoint_lifecycle = None;
    request.reviewer_decision = None;
    request.policy_decision = None;
    request.enforcement_plan.sandbox_profile_label = None;

    let record = gate.evaluate(request);

    assert_eq!(record.status, ApplyPatchGateStatus::Blocked);
    assert!(record
        .blockers
        .contains(&ApplyPatchGateBlocker::MissingCheckpointLifecycle));
    assert!(record
        .blockers
        .contains(&ApplyPatchGateBlocker::MissingReviewerDecision));
    assert!(record
        .blockers
        .contains(&ApplyPatchGateBlocker::MissingPolicyDecision));
    assert!(record
        .blockers
        .contains(&ApplyPatchGateBlocker::MissingSandboxProfile));
    assert!(record.executor_blocked);
}

#[test]
fn apply_patch_gate_rejects_scope_mismatch_and_local_root_execution() {
    let gate = ApplyPatchGate;
    let mut request = valid_request();
    request.patch_proposal.touched_paths = vec!["src/lib.rs".to_string()];
    request.mutation_scope = WorkspaceMutationScope {
        mutation_mode: MutationMode::ExplicitLocal,
        worktree_required: false,
        root_label: "project".to_string(),
        allowed_paths: vec!["docs/README.md".to_string()],
        ..request.mutation_scope
    };

    let record = gate.evaluate(request);

    assert_eq!(record.status, ApplyPatchGateStatus::Blocked);
    assert!(record
        .blockers
        .contains(&ApplyPatchGateBlocker::ScopeMismatch));
    assert!(record
        .blockers
        .contains(&ApplyPatchGateBlocker::WorktreeIsolationRequired));
    assert!(record.executor_blocked);
}

#[test]
fn apply_patch_gate_reports_executor_blocked_even_when_preflight_is_ready() {
    let gate = ApplyPatchGate;

    let record = gate.evaluate(valid_request());

    assert_eq!(record.status, ApplyPatchGateStatus::PreflightReady);
    assert!(record.blockers.is_empty());
    assert_eq!(record.affected_paths, vec!["docs/README.md"]);
    assert!(record.executor_blocked);
    assert_eq!(
        record.executor_block_reason,
        "apply_patch_executor_not_implemented"
    );
}
