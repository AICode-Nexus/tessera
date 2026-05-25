use std::path::{Component, Path};

use tessera_protocol::{
    CodingWorkflowId, MutationMode, MutationRequestOperationKind, MutationRequestProposal,
    MutationRequestStatus, PatchProposal, PolicyOutcome, ReviewerDecisionKind,
    ReviewerGateDecision, TaskId, ToolPolicyDecision, WorkspaceCheckpointLifecycleRecord,
    WorkspaceCheckpointLifecycleStatus, WorkspaceMutationScope,
};

use crate::MutationEnforcementPlan;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyPatchGateStatus {
    Blocked,
    PreflightReady,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplyPatchGateBlocker {
    WorkflowMismatch,
    TaskMismatch,
    OperationNotPatchApplication,
    MutationRequestNotApproved,
    ScopeMismatch,
    WorktreeIsolationRequired,
    MissingPolicyDecision,
    PolicyNotAllowed,
    MissingReviewerDecision,
    ReviewerNotAccepted,
    MissingCheckpointLifecycle,
    CheckpointNotCreated,
    MissingSandboxProfile,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyPatchGateRequest {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub mutation_request: MutationRequestProposal,
    pub patch_proposal: PatchProposal,
    pub mutation_scope: WorkspaceMutationScope,
    pub enforcement_plan: MutationEnforcementPlan,
    pub checkpoint_lifecycle: Option<WorkspaceCheckpointLifecycleRecord>,
    pub reviewer_decision: Option<ReviewerGateDecision>,
    pub policy_decision: Option<ToolPolicyDecision>,
    pub operator_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyPatchGateRecord {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub status: ApplyPatchGateStatus,
    pub blockers: Vec<ApplyPatchGateBlocker>,
    pub affected_paths: Vec<String>,
    pub executor_blocked: bool,
    pub executor_block_reason: String,
    pub operator_label: String,
}

#[derive(Clone, Debug, Default)]
pub struct ApplyPatchGate;

impl ApplyPatchGate {
    pub fn evaluate(&self, request: ApplyPatchGateRequest) -> ApplyPatchGateRecord {
        let mut blockers = Vec::new();

        validate_workflow_and_task(&request, &mut blockers);
        validate_mutation_request(&request, &mut blockers);
        validate_scope(&request, &mut blockers);
        validate_policy(&request, &mut blockers);
        validate_reviewer(&request, &mut blockers);
        validate_checkpoint(&request, &mut blockers);
        validate_sandbox(&request, &mut blockers);

        let status = if blockers.is_empty() {
            ApplyPatchGateStatus::PreflightReady
        } else {
            ApplyPatchGateStatus::Blocked
        };

        ApplyPatchGateRecord {
            workflow_id: request.workflow_id,
            task_id: request.task_id,
            status,
            blockers,
            affected_paths: request.patch_proposal.touched_paths,
            executor_blocked: true,
            executor_block_reason: "apply_patch_executor_not_implemented".to_string(),
            operator_label: request.operator_label,
        }
    }
}

fn validate_workflow_and_task(
    request: &ApplyPatchGateRequest,
    blockers: &mut Vec<ApplyPatchGateBlocker>,
) {
    if request.mutation_request.workflow_id != request.workflow_id
        || request.patch_proposal.workflow_id != request.workflow_id
        || request.mutation_scope.workflow_id != request.workflow_id
        || request.enforcement_plan.scope.workflow_id != request.workflow_id
    {
        blockers.push(ApplyPatchGateBlocker::WorkflowMismatch);
    }

    if request.mutation_request.task_id != request.task_id
        || request.patch_proposal.task_id != request.task_id
        || request.mutation_scope.task_id != request.task_id
        || request.enforcement_plan.scope.task_id != request.task_id
    {
        blockers.push(ApplyPatchGateBlocker::TaskMismatch);
    }
}

fn validate_mutation_request(
    request: &ApplyPatchGateRequest,
    blockers: &mut Vec<ApplyPatchGateBlocker>,
) {
    if request.mutation_request.operation != MutationRequestOperationKind::PatchApplication {
        blockers.push(ApplyPatchGateBlocker::OperationNotPatchApplication);
    }
    if request.mutation_request.status != MutationRequestStatus::Approved {
        blockers.push(ApplyPatchGateBlocker::MutationRequestNotApproved);
    }
}

fn validate_scope(request: &ApplyPatchGateRequest, blockers: &mut Vec<ApplyPatchGateBlocker>) {
    let paths_in_scope = request.patch_proposal.touched_paths.iter().all(|path| {
        is_safe_relative_path(path)
            && contains_path(&request.mutation_scope.allowed_paths, path)
            && contains_path(&request.mutation_request.requested_paths, path)
    });

    if !paths_in_scope {
        blockers.push(ApplyPatchGateBlocker::ScopeMismatch);
    }

    if request.mutation_scope.mutation_mode != MutationMode::WorktreeFirst
        || !request.mutation_scope.worktree_required
        || !request.mutation_request.worktree_required
        || request.mutation_scope.root_label.trim().is_empty()
        || request.mutation_scope.root_label == "project"
    {
        blockers.push(ApplyPatchGateBlocker::WorktreeIsolationRequired);
    }
}

fn validate_policy(request: &ApplyPatchGateRequest, blockers: &mut Vec<ApplyPatchGateBlocker>) {
    let Some(policy_decision) = &request.policy_decision else {
        blockers.push(ApplyPatchGateBlocker::MissingPolicyDecision);
        return;
    };

    if request.mutation_request.policy_decision_id.as_ref() != Some(&policy_decision.decision_id)
        || request.enforcement_plan.policy_decision_id.as_ref()
            != Some(&policy_decision.decision_id)
    {
        blockers.push(ApplyPatchGateBlocker::MissingPolicyDecision);
    }

    if policy_decision.outcome != PolicyOutcome::Allow {
        blockers.push(ApplyPatchGateBlocker::PolicyNotAllowed);
    }
}

fn validate_reviewer(request: &ApplyPatchGateRequest, blockers: &mut Vec<ApplyPatchGateBlocker>) {
    let Some(reviewer_decision) = &request.reviewer_decision else {
        blockers.push(ApplyPatchGateBlocker::MissingReviewerDecision);
        return;
    };

    if request.mutation_request.reviewer_gate_id.as_ref() != Some(&reviewer_decision.gate_id)
        || request.patch_proposal.reviewer_gate_id.as_ref() != Some(&reviewer_decision.gate_id)
    {
        blockers.push(ApplyPatchGateBlocker::MissingReviewerDecision);
    }

    if reviewer_decision.decision != ReviewerDecisionKind::Accept {
        blockers.push(ApplyPatchGateBlocker::ReviewerNotAccepted);
    }
}

fn validate_checkpoint(request: &ApplyPatchGateRequest, blockers: &mut Vec<ApplyPatchGateBlocker>) {
    let Some(checkpoint_lifecycle) = &request.checkpoint_lifecycle else {
        blockers.push(ApplyPatchGateBlocker::MissingCheckpointLifecycle);
        return;
    };

    if request.mutation_request.required_checkpoint_id.as_ref()
        != Some(&checkpoint_lifecycle.checkpoint_id)
        || request.patch_proposal.required_checkpoint_id.as_ref()
            != Some(&checkpoint_lifecycle.checkpoint_id)
    {
        blockers.push(ApplyPatchGateBlocker::MissingCheckpointLifecycle);
    }

    if checkpoint_lifecycle.status != WorkspaceCheckpointLifecycleStatus::Created {
        blockers.push(ApplyPatchGateBlocker::CheckpointNotCreated);
    }
}

fn validate_sandbox(request: &ApplyPatchGateRequest, blockers: &mut Vec<ApplyPatchGateBlocker>) {
    if request.mutation_request.sandbox_profile_label.is_none()
        || request.enforcement_plan.sandbox_profile_label.is_none()
    {
        blockers.push(ApplyPatchGateBlocker::MissingSandboxProfile);
    }
}

fn contains_path(allowed_paths: &[String], path: &str) -> bool {
    allowed_paths.iter().any(|allowed| allowed == path)
}

fn is_safe_relative_path(path: &str) -> bool {
    let path = path.trim();
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && !Path::new(path)
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
}
