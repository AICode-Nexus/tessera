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
    MissingPatchBody,
    PatchBodyTooLarge,
    UnsafePatchPath,
    UnsupportedPatchOperation,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ApplyPatchDryRunOperation {
    Create,
    #[default]
    Modify,
    DeleteUnsupported,
    RenameUnsupported,
    BinaryUnsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyPatchDryRunInput {
    pub body: String,
    pub max_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyPatchDryRunOperationSummary {
    pub path: String,
    pub operation: ApplyPatchDryRunOperation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyPatchDryRunSummary {
    pub affected_paths: Vec<String>,
    pub operations: Vec<ApplyPatchDryRunOperationSummary>,
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
    pub patch_body: Option<ApplyPatchDryRunInput>,
    pub operator_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplyPatchGateRecord {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub status: ApplyPatchGateStatus,
    pub blockers: Vec<ApplyPatchGateBlocker>,
    pub affected_paths: Vec<String>,
    pub dry_run: Option<ApplyPatchDryRunSummary>,
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
        let dry_run = validate_dry_run(&request, &mut blockers);

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
            affected_paths: dry_run
                .as_ref()
                .map(|summary| summary.affected_paths.clone())
                .unwrap_or(request.patch_proposal.touched_paths),
            dry_run,
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

fn validate_dry_run(
    request: &ApplyPatchGateRequest,
    blockers: &mut Vec<ApplyPatchGateBlocker>,
) -> Option<ApplyPatchDryRunSummary> {
    let Some(input) = &request.patch_body else {
        blockers.push(ApplyPatchGateBlocker::MissingPatchBody);
        return None;
    };

    if input.body.len() > input.max_bytes {
        blockers.push(ApplyPatchGateBlocker::PatchBodyTooLarge);
        return None;
    }

    let mut summary = parse_patch_body(&input.body, blockers);
    for path in &summary.affected_paths {
        if !contains_path(&request.mutation_scope.allowed_paths, path)
            || !contains_path(&request.mutation_request.requested_paths, path)
        {
            push_unique_blocker(blockers, ApplyPatchGateBlocker::ScopeMismatch);
        }
    }

    if summary.affected_paths.is_empty() {
        summary.affected_paths = request.patch_proposal.touched_paths.clone();
    }

    Some(summary)
}

fn parse_patch_body(
    body: &str,
    blockers: &mut Vec<ApplyPatchGateBlocker>,
) -> ApplyPatchDryRunSummary {
    let mut parser = PatchDryRunParser::default();
    for line in body.lines() {
        parser.consume(line, blockers);
    }
    parser.finish(blockers)
}

#[derive(Default)]
struct PatchDryRunParser {
    current_path: Option<String>,
    current_operation: ApplyPatchDryRunOperation,
    affected_paths: Vec<String>,
    operations: Vec<ApplyPatchDryRunOperationSummary>,
}

impl PatchDryRunParser {
    fn consume(&mut self, line: &str, blockers: &mut Vec<ApplyPatchGateBlocker>) {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            self.finish_current(blockers);
            self.current_operation = ApplyPatchDryRunOperation::Modify;
            self.current_path = parse_git_diff_path(rest, blockers);
            return;
        }

        if line.starts_with("Binary files ") {
            self.current_operation = ApplyPatchDryRunOperation::BinaryUnsupported;
            push_unique_blocker(blockers, ApplyPatchGateBlocker::UnsupportedPatchOperation);
            return;
        }

        if let Some(path) = line.strip_prefix("rename to ") {
            self.current_path = normalize_patch_path(path, blockers);
            self.current_operation = ApplyPatchDryRunOperation::RenameUnsupported;
            push_unique_blocker(blockers, ApplyPatchGateBlocker::UnsupportedPatchOperation);
            return;
        }

        if line.starts_with("rename from ") {
            self.current_operation = ApplyPatchDryRunOperation::RenameUnsupported;
            push_unique_blocker(blockers, ApplyPatchGateBlocker::UnsupportedPatchOperation);
            return;
        }

        if line.starts_with("deleted file mode ") {
            self.current_operation = ApplyPatchDryRunOperation::DeleteUnsupported;
            push_unique_blocker(blockers, ApplyPatchGateBlocker::UnsupportedPatchOperation);
            return;
        }

        if line.starts_with("new file mode ") {
            self.current_operation = ApplyPatchDryRunOperation::Create;
            return;
        }

        if let Some(path) = line.strip_prefix("+++ ") {
            if path == "/dev/null" {
                self.current_operation = ApplyPatchDryRunOperation::DeleteUnsupported;
                push_unique_blocker(blockers, ApplyPatchGateBlocker::UnsupportedPatchOperation);
            } else if let Some(path) = normalize_patch_path(path, blockers) {
                self.current_path = Some(path);
            }
        }

        if line == "--- /dev/null" {
            self.current_operation = ApplyPatchDryRunOperation::Create;
        }
    }

    fn finish(mut self, blockers: &mut Vec<ApplyPatchGateBlocker>) -> ApplyPatchDryRunSummary {
        self.finish_current(blockers);
        ApplyPatchDryRunSummary {
            affected_paths: self.affected_paths,
            operations: self.operations,
        }
    }

    fn finish_current(&mut self, blockers: &mut Vec<ApplyPatchGateBlocker>) {
        let Some(path) = self.current_path.take() else {
            return;
        };

        if !is_safe_relative_path(&path) {
            push_unique_blocker(blockers, ApplyPatchGateBlocker::UnsafePatchPath);
            return;
        }

        push_unique_path(&mut self.affected_paths, path.clone());
        self.operations.push(ApplyPatchDryRunOperationSummary {
            path,
            operation: self.current_operation,
        });
    }
}

fn parse_git_diff_path(rest: &str, blockers: &mut Vec<ApplyPatchGateBlocker>) -> Option<String> {
    rest.split_whitespace()
        .nth(1)
        .and_then(|path| normalize_patch_path(path, blockers))
}

fn normalize_patch_path(path: &str, blockers: &mut Vec<ApplyPatchGateBlocker>) -> Option<String> {
    let normalized = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path)
        .to_string();

    if normalized == "/dev/null" {
        return None;
    }

    if !is_safe_relative_path(&normalized) {
        push_unique_blocker(blockers, ApplyPatchGateBlocker::UnsafePatchPath);
        return None;
    }

    Some(normalized)
}

fn contains_path(allowed_paths: &[String], path: &str) -> bool {
    allowed_paths.iter().any(|allowed| allowed == path)
}

fn push_unique_path(paths: &mut Vec<String>, path: String) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

fn push_unique_blocker(blockers: &mut Vec<ApplyPatchGateBlocker>, blocker: ApplyPatchGateBlocker) {
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

fn is_safe_relative_path(path: &str) -> bool {
    let path = path.trim();
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && !Path::new(path)
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
}
