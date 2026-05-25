use tessera_core::{MutationEnforcementPlanRequest, MutationEnforcementPlanner};
use tessera_protocol::{
    CodingWorkflowId, MutationMode, OsSandboxFilesystem, OsSandboxMode, PolicyDecisionId, TaskId,
};

fn workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from_static("coding_workflow_enforcement")
}

fn task_id() -> TaskId {
    TaskId::from_static("task_mutation_enforcement")
}

#[test]
fn mutation_enforcement_planner_requires_worktree_policy_and_sandbox_before_execution() {
    let planner = MutationEnforcementPlanner::new("/workspace/project");

    let plan = planner
        .plan(MutationEnforcementPlanRequest {
            workflow_id: workflow_id(),
            task_id: task_id(),
            requested_paths: vec!["src/lib.rs".to_string()],
            mutation_mode: None,
            policy_decision_id: None,
            reason: "patch source file".to_string(),
        })
        .expect("default file-changing workflow should produce metadata plan");

    assert_eq!(plan.scope.mutation_mode, MutationMode::WorktreeFirst);
    assert!(plan.scope.worktree_required);
    assert_eq!(plan.scope.allowed_paths, vec!["src/lib.rs"]);
    assert_eq!(plan.sandbox_profile.mode, OsSandboxMode::WorkspaceWrite);
    assert_eq!(
        plan.sandbox_profile.filesystem,
        OsSandboxFilesystem::WorkspaceWrite
    );
    assert!(plan.sandbox_profile.requires_checkpoint);
    assert_eq!(
        plan.sandbox_profile_label.as_deref(),
        Some("workspace_write")
    );
    assert!(plan.executor_blocked);
    assert_eq!(
        plan.executor_block_reason,
        "executor_not_available_until_policy_checkpoint_and_reviewer_gates"
    );
    assert!(plan.policy_decision_id.is_none());

    let error = planner
        .plan(MutationEnforcementPlanRequest {
            workflow_id: workflow_id(),
            task_id: task_id(),
            requested_paths: vec!["src/lib.rs".to_string()],
            mutation_mode: Some(MutationMode::ExplicitLocal),
            policy_decision_id: None,
            reason: "local edit after user approval".to_string(),
        })
        .expect_err("explicit-local mutation requires a policy decision id");

    assert!(error
        .to_string()
        .contains("explicit-local mutation requires policy_decision_id"));

    let policy_decision_id = PolicyDecisionId::from_static("policy_explicit_local");
    let explicit_local = planner
        .plan(MutationEnforcementPlanRequest {
            workflow_id: workflow_id(),
            task_id: task_id(),
            requested_paths: vec!["src/lib.rs".to_string()],
            mutation_mode: Some(MutationMode::ExplicitLocal),
            policy_decision_id: Some(policy_decision_id.clone()),
            reason: "local edit after policy".to_string(),
        })
        .expect("policy-approved explicit-local mutation should still be metadata-only");

    assert_eq!(
        explicit_local.scope.mutation_mode,
        MutationMode::ExplicitLocal
    );
    assert!(!explicit_local.scope.worktree_required);
    assert_eq!(explicit_local.policy_decision_id, Some(policy_decision_id));
    assert!(explicit_local.executor_blocked);

    let error = planner
        .plan(MutationEnforcementPlanRequest {
            workflow_id: workflow_id(),
            task_id: task_id(),
            requested_paths: vec!["../secrets.env".to_string()],
            mutation_mode: None,
            policy_decision_id: None,
            reason: "unsafe path".to_string(),
        })
        .expect_err("unsafe paths must be rejected before planning sandbox execution");

    assert!(error
        .to_string()
        .contains("relative workspace path must not traverse outside workspace"));
}
