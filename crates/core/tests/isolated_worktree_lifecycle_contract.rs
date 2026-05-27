use std::path::PathBuf;

use tessera_core::{
    IsolatedWorktreeLifecyclePlanner, IsolatedWorktreePlanRequest, SourceCheckoutStatus,
    WorktreeRetentionPolicy,
};
use tessera_protocol::{
    CodingWorkflowId, EventRange, HandoffEvidenceKind, HandoffEvidenceRef, MutationRequestId,
    PatchProposalId, RunEvent, TaskId, WorkspaceWorktreeLifecycleStatus,
};

fn workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from_static("coding_workflow_auto_patch_review")
}

fn task_id() -> TaskId {
    TaskId::from_static("task_auto_worktree")
}

fn patch_id() -> PatchProposalId {
    PatchProposalId::from_static("patch_proposal_docs_readme")
}

fn mutation_request_id() -> MutationRequestId {
    MutationRequestId::from_static("mutation_request_docs_readme")
}

fn clean_source() -> SourceCheckoutStatus {
    SourceCheckoutStatus {
        source_commit: "17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1".to_string(),
        source_branch_label: Some("main".to_string()),
        has_tracked_changes: false,
        has_staged_changes: false,
        has_untracked_changes: true,
    }
}

fn trace_evidence() -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::TraceRange,
        artifact_id: None,
        trace_id: Some("trace_apply_patch_review".to_string()),
        event_range: Some(EventRange {
            start_seq: 10,
            end_seq: 18,
        }),
        label: Some("reviewed apply-patch bundle".to_string()),
        summary: Some("review/checkpoint/policy evidence refs".to_string()),
    }
}

fn plan_request() -> IsolatedWorktreePlanRequest {
    IsolatedWorktreePlanRequest {
        workflow_id: workflow_id(),
        task_id: task_id(),
        trace_id: "trace_apply_patch_review".to_string(),
        request_id: mutation_request_id(),
        patch_id: patch_id(),
        repo_key: "tessera-abcd1234".to_string(),
        data_dir: PathBuf::from("/var/tmp/tessera-data"),
        worktree_base: None,
        worktree_base_key: None,
        requested_root_label: None,
        source_checkout: clean_source(),
        existing_worktree_leaf_names: Vec::new(),
        retention_policy: WorktreeRetentionPolicy::RetainOnSuccess,
        reason: "trace-driven apply-patch requested an isolated worktree".to_string(),
        evidence: vec![trace_evidence()],
    }
}

#[test]
fn isolated_worktree_planner_generates_redacted_labels_and_default_base() {
    let planner = IsolatedWorktreeLifecyclePlanner;
    let plan = planner
        .plan(plan_request())
        .expect("clean source should produce a metadata-only plan");

    assert_eq!(
        plan.worktree_root_label,
        "worktree:auto-patch-review-docs-readme"
    );
    assert_eq!(
        plan.worktree_leaf_name,
        "auto-patch-review-docs-readme-17bd0f1c0ffe"
    );
    assert!(plan
        .worktree_leaf_name
        .chars()
        .all(|ch: char| ch.is_ascii_alphanumeric() || ch == '-'));
    assert!(!plan.worktree_leaf_name.contains('/'));
    assert!(!plan.worktree_leaf_name.contains(':'));
    assert!(plan.worktree_leaf_name.len() <= 80);
    assert_eq!(
        plan.worktree_base,
        PathBuf::from("/var/tmp/tessera-data/worktrees/tessera-abcd1234")
    );
    assert_eq!(
        plan.worktree_path,
        PathBuf::from(
            "/var/tmp/tessera-data/worktrees/tessera-abcd1234/auto-patch-review-docs-readme-17bd0f1c0ffe"
        )
    );
    assert_eq!(
        plan.worktree_base_key,
        "data_dir:worktrees/tessera-abcd1234"
    );
    assert_eq!(
        plan.retention_policy,
        WorktreeRetentionPolicy::RetainOnSuccess
    );
    assert!(plan.source_has_untracked_changes);
    assert_eq!(plan.created_for_request_id, mutation_request_id());
    assert_eq!(plan.created_for_patch_id, patch_id());
}

#[test]
fn isolated_worktree_plan_builds_planned_lifecycle_event_without_paths() {
    let planner = IsolatedWorktreeLifecyclePlanner;
    let plan = planner
        .plan(plan_request())
        .expect("clean source should produce a metadata-only plan");
    let event = plan.planned_lifecycle_event();

    assert_eq!(event.kind(), "workspace_worktree_lifecycle_recorded");
    assert_eq!(event.task_id(), Some(task_id()));
    match event {
        RunEvent::WorkspaceWorktreeLifecycleRecorded { record } => {
            assert_eq!(
                record.lifecycle_status,
                WorkspaceWorktreeLifecycleStatus::Planned
            );
            assert_eq!(record.source_commit, clean_source().source_commit);
            assert_eq!(
                record.worktree_root_label,
                "worktree:auto-patch-review-docs-readme"
            );
            assert_eq!(
                record.worktree_base_key,
                "data_dir:worktrees/tessera-abcd1234"
            );
            assert_eq!(record.created_for_request_id, Some(mutation_request_id()));
            assert_eq!(record.created_for_patch_id, Some(patch_id()));
            assert_eq!(record.evidence[0].kind, HandoffEvidenceKind::TraceRange);

            let payload = serde_json::to_string(&record).unwrap();
            assert!(!payload.contains("/var/tmp/tessera-data"));
            assert!(!payload.contains("/Users/admin/work/tessera"));
            assert!(!payload.contains("git worktree"));
            assert!(!payload.contains("stdout"));
            assert!(!payload.contains("stderr"));
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn isolated_worktree_planner_rejects_unsafe_labels_and_trace_path_keys() {
    let planner = IsolatedWorktreeLifecyclePlanner;

    let mut request = plan_request();
    request.requested_root_label = Some("project".to_string());
    let error = planner
        .plan(request)
        .expect_err("primary root labels must never be planned");
    assert!(error
        .to_string()
        .contains("worktree root label must not target primary root"));

    let mut request = plan_request();
    request.requested_root_label = Some("/tmp/worktree".to_string());
    let error = planner
        .plan(request)
        .expect_err("absolute-looking trace labels must be rejected");
    assert!(error
        .to_string()
        .contains("worktree root label must be trace-safe"));

    let mut request = plan_request();
    request.worktree_base_key = Some("/Users/admin/work/tessera/.worktrees".to_string());
    let error = planner
        .plan(request)
        .expect_err("absolute paths must not become trace keys");
    assert!(error
        .to_string()
        .contains("worktree base key must be trace-safe"));
}

#[test]
fn isolated_worktree_planner_rejects_collisions_and_dirty_source_status() {
    let planner = IsolatedWorktreeLifecyclePlanner;

    let mut request = plan_request();
    request.existing_worktree_leaf_names =
        vec!["auto-patch-review-docs-readme-17bd0f1c0ffe".to_string()];
    let error = planner
        .plan(request)
        .expect_err("colliding worktree leaves must fail before execution");
    assert!(error.to_string().contains("worktree path collision"));

    let mut request = plan_request();
    request.source_checkout.has_tracked_changes = true;
    let error = planner
        .plan(request)
        .expect_err("tracked changes block automatic worktree planning");
    assert!(error
        .to_string()
        .contains("source checkout has tracked changes"));

    let mut request = plan_request();
    request.source_checkout.has_staged_changes = true;
    let error = planner
        .plan(request)
        .expect_err("staged changes block automatic worktree planning");
    assert!(error
        .to_string()
        .contains("source checkout has staged changes"));
}
