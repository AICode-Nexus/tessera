use std::path::PathBuf;

use tessera_core::{
    GitWorktreeCommandRunner, IsolatedWorktreeLifecyclePlanner,
    IsolatedWorktreeLifecycleRunRequest, IsolatedWorktreeLifecycleRunner,
    IsolatedWorktreePlanRequest, SourceCheckoutStatus, WorktreeCommandInvocation,
    WorktreeCommandOutput, WorktreeCommandRunner, WorktreeRetentionPolicy,
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

fn run_request() -> IsolatedWorktreeLifecycleRunRequest {
    IsolatedWorktreeLifecycleRunRequest {
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
        source_root: PathBuf::from("/repo/tessera"),
        existing_worktree_leaf_names: Vec::new(),
        retention_policy: WorktreeRetentionPolicy::RetainOnSuccess,
        reason: "trace-driven apply-patch requested an isolated worktree".to_string(),
        evidence: vec![trace_evidence()],
    }
}

#[derive(Default)]
struct FakeCommandRunner {
    outputs: std::collections::VecDeque<WorktreeCommandOutput>,
    invocations: Vec<WorktreeCommandInvocation>,
}

impl FakeCommandRunner {
    fn with_outputs(outputs: impl IntoIterator<Item = WorktreeCommandOutput>) -> Self {
        Self {
            outputs: outputs.into_iter().collect(),
            invocations: Vec::new(),
        }
    }
}

impl WorktreeCommandRunner for FakeCommandRunner {
    fn run(
        &mut self,
        invocation: WorktreeCommandInvocation,
    ) -> Result<WorktreeCommandOutput, String> {
        self.invocations.push(invocation);
        self.outputs
            .pop_front()
            .ok_or_else(|| "missing fake command output".to_string())
    }
}

fn command_output(stdout: &str) -> WorktreeCommandOutput {
    WorktreeCommandOutput {
        status_success: true,
        stdout: stdout.to_string(),
        stderr: String::new(),
    }
}

fn command_failure(stderr: &str) -> WorktreeCommandOutput {
    WorktreeCommandOutput {
        status_success: false,
        stdout: String::new(),
        stderr: stderr.to_string(),
    }
}

fn lifecycle_status(event: &RunEvent) -> WorkspaceWorktreeLifecycleStatus {
    match event {
        RunEvent::WorkspaceWorktreeLifecycleRecorded { record } => record.lifecycle_status,
        other => panic!("unexpected lifecycle event: {other:?}"),
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
fn worktree_runner_uses_fixed_git_args_and_records_created_lifecycle() {
    let mut commands = FakeCommandRunner::with_outputs([
        command_output("17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1\n"),
        command_output(""),
        command_output(""),
    ]);
    let runner = IsolatedWorktreeLifecycleRunner::default();

    let created = runner
        .create_detached_worktree(&mut commands, run_request())
        .expect("clean source should create a detached worktree");

    assert_eq!(created.plan.source_commit, clean_source().source_commit);
    assert_eq!(
        created
            .lifecycle_events
            .iter()
            .map(lifecycle_status)
            .collect::<Vec<_>>(),
        vec![
            WorkspaceWorktreeLifecycleStatus::Planned,
            WorkspaceWorktreeLifecycleStatus::Created
        ]
    );

    assert_eq!(commands.invocations.len(), 3);
    assert_eq!(commands.invocations[0].program, "git");
    assert_eq!(commands.invocations[0].cwd, PathBuf::from("/repo/tessera"));
    assert_eq!(commands.invocations[0].args, vec!["rev-parse", "HEAD"]);
    assert_eq!(
        commands.invocations[1].args,
        vec!["status", "--porcelain", "--untracked-files=no"]
    );
    assert_eq!(
        commands.invocations[2].args,
        vec![
            "worktree",
            "add",
            "--detach",
            "/var/tmp/tessera-data/worktrees/tessera-abcd1234/auto-patch-review-docs-readme-17bd0f1c0ffe",
            "17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1"
        ]
    );
}

#[test]
fn worktree_runner_aborts_dirty_status_before_worktree_add() {
    let runner = IsolatedWorktreeLifecycleRunner::default();
    let mut commands = FakeCommandRunner::with_outputs([
        command_output("17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1\n"),
        command_output(" M crates/core/src/lib.rs\n"),
    ]);

    let error = runner
        .create_detached_worktree(&mut commands, run_request())
        .expect_err("tracked source changes should abort before worktree add");

    assert!(error
        .to_string()
        .contains("source checkout has tracked changes"));
    assert_eq!(commands.invocations.len(), 2);

    let mut commands = FakeCommandRunner::with_outputs([
        command_output("17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1\n"),
        command_output("M  crates/core/src/lib.rs\n"),
    ]);
    let error = runner
        .create_detached_worktree(&mut commands, run_request())
        .expect_err("staged source changes should abort before worktree add");

    assert!(error
        .to_string()
        .contains("source checkout has staged changes"));
    assert_eq!(commands.invocations.len(), 2);
}

#[test]
fn worktree_runner_retains_and_cleans_only_current_invocation_without_force() {
    let runner = IsolatedWorktreeLifecycleRunner::default();
    let mut commands = FakeCommandRunner::with_outputs([
        command_output("17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1\n"),
        command_output(""),
        command_output(""),
        command_output(""),
    ]);

    let created = runner
        .create_detached_worktree(&mut commands, run_request())
        .expect("clean source should create a detached worktree");
    let retained = runner.retained_lifecycle_event(&created);
    assert_eq!(
        lifecycle_status(&retained),
        WorkspaceWorktreeLifecycleStatus::Retained
    );
    assert_eq!(commands.invocations.len(), 3);

    let cleanup = runner.cleanup_created_worktree(&mut commands, &created);
    assert_eq!(
        lifecycle_status(&cleanup),
        WorkspaceWorktreeLifecycleStatus::CleanupCompleted
    );
    assert_eq!(
        commands.invocations[3].args,
        vec![
            "worktree",
            "remove",
            "/var/tmp/tessera-data/worktrees/tessera-abcd1234/auto-patch-review-docs-readme-17bd0f1c0ffe"
        ]
    );
    assert!(!commands.invocations[3]
        .args
        .iter()
        .any(|arg| arg == "--force"));

    let mut not_created = created.clone();
    not_created.created_by_current_invocation = false;
    let before = commands.invocations.len();
    let error = runner
        .try_cleanup_created_worktree(&mut commands, &not_created)
        .expect_err("cleanup must reject worktrees not created by this invocation");
    assert!(error
        .to_string()
        .contains("worktree was not created by this invocation"));
    assert_eq!(commands.invocations.len(), before);
}

#[test]
fn worktree_runner_records_cleanup_failed_without_force() {
    let runner = IsolatedWorktreeLifecycleRunner::default();
    let mut commands = FakeCommandRunner::with_outputs([
        command_output("17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1\n"),
        command_output(""),
        command_output(""),
        command_failure("worktree /repo/tessera-worktree contains local changes"),
    ]);
    let created = runner
        .create_detached_worktree(&mut commands, run_request())
        .expect("clean source should create a detached worktree");

    let cleanup = runner.cleanup_created_worktree(&mut commands, &created);
    assert_eq!(
        lifecycle_status(&cleanup),
        WorkspaceWorktreeLifecycleStatus::CleanupFailed
    );
    assert!(!commands.invocations[3]
        .args
        .iter()
        .any(|arg| arg == "--force"));
    let payload = cleanup.payload().to_string();
    assert!(payload.contains("worktree removal failed without force"));
    assert!(!payload.contains("local changes"));
    assert!(!payload.contains("/repo/tessera"));
}

#[test]
fn worktree_runner_default_rejects_non_fixed_invocations_before_process_spawn() {
    let mut runner = GitWorktreeCommandRunner;
    let error = runner
        .run(WorktreeCommandInvocation {
            program: "definitely-not-git".to_string(),
            args: vec!["commit".to_string()],
            cwd: PathBuf::from("/repo/tessera"),
        })
        .expect_err("default runner must not accept arbitrary programs");

    assert!(error.contains("unsupported git worktree command"));
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
