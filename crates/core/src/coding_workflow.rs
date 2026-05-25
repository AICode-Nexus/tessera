use std::path::{Component, Path};

use tessera_protocol::{
    ArtifactBodyRecord, CodingWorkflowId, MutationMode, PatchApplicationRecord, PatchProposal,
    RestorePlanRecord, ReviewBundle, RunEvent, TaskId, TestPlanRecord, TestRunRecord,
    WorkspaceCheckpointLifecycleRecord, WorkspaceCheckpointLifecycleStatus, WorkspaceMutationScope,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowStartRequest {
    pub workflow_id: CodingWorkflowId,
    pub task_id: Option<TaskId>,
    pub objective: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowWorkspaceScopeRequest {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub root_label: String,
    pub allowed_paths: Vec<String>,
    pub denied_paths: Vec<String>,
    pub mutation_mode: Option<MutationMode>,
    pub worktree_required: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowPatchProposalRequest {
    pub proposal: PatchProposal,
    pub mutation_ready: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowPatchApplicationRequest {
    pub record: PatchApplicationRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowTestPlanRequest {
    pub plan: TestPlanRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowTestRunRequest {
    pub record: TestRunRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowArtifactBodyRequest {
    pub record: ArtifactBodyRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowCheckpointLifecycleRequest {
    pub lifecycle: WorkspaceCheckpointLifecycleRecord,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodingWorkflowReviewBundleRequest {
    pub bundle: ReviewBundle,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodingWorkflowError {
    message: String,
}

impl CodingWorkflowError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CodingWorkflowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CodingWorkflowError {}

#[derive(Clone, Debug, Default)]
pub struct CodingWorkflowCoordinator;

impl CodingWorkflowCoordinator {
    pub fn start_event(
        &self,
        request: CodingWorkflowStartRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        let task_id = request
            .task_id
            .ok_or_else(|| CodingWorkflowError::new("task_id is required"))?;
        if request.objective.trim().is_empty() {
            return Err(CodingWorkflowError::new("objective is required"));
        }

        Ok(RunEvent::CodingWorkflowStarted {
            workflow_id: request.workflow_id,
            task_id,
            objective: request.objective,
        })
    }

    pub fn workspace_scope_event(
        &self,
        request: CodingWorkflowWorkspaceScopeRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        validate_scope_paths(&request.allowed_paths)?;
        validate_scope_paths(&request.denied_paths)?;
        if request.root_label.trim().is_empty() {
            return Err(CodingWorkflowError::new("root_label is required"));
        }

        Ok(RunEvent::WorkspaceMutationScopeRecorded {
            scope: WorkspaceMutationScope {
                workflow_id: request.workflow_id,
                task_id: request.task_id,
                root_label: request.root_label,
                allowed_paths: request.allowed_paths,
                denied_paths: request.denied_paths,
                mutation_mode: request.mutation_mode.unwrap_or(MutationMode::WorktreeFirst),
                worktree_required: request.worktree_required,
                reason: request.reason,
            },
        })
    }

    pub fn patch_proposal_event(
        &self,
        request: CodingWorkflowPatchProposalRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        validate_patch_proposal(&request.proposal, request.mutation_ready)?;
        Ok(RunEvent::PatchProposalRecorded {
            proposal: request.proposal,
        })
    }

    pub fn patch_application_event(
        &self,
        request: CodingWorkflowPatchApplicationRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        validate_scope_paths(&request.record.conflict_paths)?;
        validate_scope_paths(&request.record.applied_paths)?;
        Ok(RunEvent::PatchApplicationRecorded {
            record: request.record,
        })
    }

    pub fn test_plan_event(
        &self,
        request: CodingWorkflowTestPlanRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        validate_scope_paths(&request.plan.affected_paths)?;
        if request.plan.command_labels.is_empty() {
            return Err(CodingWorkflowError::new("command_labels is required"));
        }
        Ok(RunEvent::TestPlanRecorded { plan: request.plan })
    }

    pub fn test_run_event(
        &self,
        request: CodingWorkflowTestRunRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        if request.record.command_label.trim().is_empty() {
            return Err(CodingWorkflowError::new("command_label is required"));
        }
        if request.record.stdout_artifact_id.is_none()
            && request.record.stderr_artifact_id.is_none()
        {
            return Err(CodingWorkflowError::new(
                "test run output must use artifact references",
            ));
        }
        Ok(RunEvent::TestRunRecorded {
            record: request.record,
        })
    }

    pub fn artifact_body_event(
        &self,
        request: CodingWorkflowArtifactBodyRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        validate_artifact_body_record(&request.record)?;
        Ok(RunEvent::ArtifactBodyRecorded {
            record: request.record,
        })
    }

    pub fn checkpoint_lifecycle_event(
        &self,
        request: CodingWorkflowCheckpointLifecycleRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        validate_checkpoint_lifecycle(&request.lifecycle)?;
        Ok(RunEvent::SnapshotLifecycleRecorded {
            lifecycle: request.lifecycle,
        })
    }

    pub fn review_bundle_event(
        &self,
        request: CodingWorkflowReviewBundleRequest,
    ) -> Result<RunEvent, CodingWorkflowError> {
        if request.bundle.patch_ids.is_empty() && request.bundle.test_run_ids.is_empty() {
            return Err(CodingWorkflowError::new(
                "review bundle requires patch or test evidence",
            ));
        }
        if request.bundle.summary.trim().is_empty() {
            return Err(CodingWorkflowError::new(
                "review bundle summary is required",
            ));
        }
        Ok(RunEvent::ReviewBundleRecorded {
            bundle: request.bundle,
        })
    }

    pub fn restore_plan_event(
        &self,
        plan: RestorePlanRecord,
    ) -> Result<RunEvent, CodingWorkflowError> {
        validate_scope_paths(&plan.target_paths)?;
        if plan.reason.trim().is_empty() {
            return Err(CodingWorkflowError::new("restore plan reason is required"));
        }
        if !plan.execution_blocked {
            return Err(CodingWorkflowError::new(
                "restore execution must remain blocked",
            ));
        }
        Ok(RunEvent::RestorePlanRecorded { plan })
    }
}

fn validate_patch_proposal(
    proposal: &PatchProposal,
    mutation_ready: bool,
) -> Result<(), CodingWorkflowError> {
    if proposal.touched_paths.is_empty() {
        return Err(CodingWorkflowError::new("touched_paths is required"));
    }
    validate_scope_paths(&proposal.touched_paths)?;
    if proposal.diff_artifacts.is_empty() {
        return Err(CodingWorkflowError::new("diff_artifacts is required"));
    }
    if mutation_ready
        && (proposal.required_checkpoint_id.is_none() || proposal.reviewer_gate_id.is_none())
    {
        return Err(CodingWorkflowError::new(
            "mutation-ready patch proposal requires checkpoint and reviewer gate",
        ));
    }
    Ok(())
}

fn validate_artifact_body_record(record: &ArtifactBodyRecord) -> Result<(), CodingWorkflowError> {
    if record.media_type.trim().is_empty() {
        return Err(CodingWorkflowError::new("artifact media_type is required"));
    }
    if record.byte_len == 0 {
        return Err(CodingWorkflowError::new(
            "artifact body byte_len is required",
        ));
    }
    if !record.storage_uri.starts_with("tessera://artifacts/") {
        return Err(CodingWorkflowError::new(
            "artifact body storage_uri must use tessera artifact storage",
        ));
    }
    if record.storage_uri.chars().any(char::is_whitespace) {
        return Err(CodingWorkflowError::new(
            "artifact body storage_uri must not contain whitespace",
        ));
    }
    Ok(())
}

fn validate_checkpoint_lifecycle(
    lifecycle: &WorkspaceCheckpointLifecycleRecord,
) -> Result<(), CodingWorkflowError> {
    if lifecycle.reason.trim().is_empty() {
        return Err(CodingWorkflowError::new(
            "checkpoint lifecycle reason is required",
        ));
    }
    if !lifecycle.execution_blocked {
        return Err(CodingWorkflowError::new(
            "checkpoint lifecycle execution must remain blocked",
        ));
    }
    if matches!(
        lifecycle.status,
        WorkspaceCheckpointLifecycleStatus::RestoreBlocked
    ) && lifecycle.restore_plan_id.is_none()
    {
        return Err(CodingWorkflowError::new(
            "restore-blocked checkpoint lifecycle requires restore_plan_id",
        ));
    }
    Ok(())
}

fn validate_scope_paths(paths: &[String]) -> Result<(), CodingWorkflowError> {
    for path in paths {
        validate_relative_workspace_path(path)?;
    }
    Ok(())
}

fn validate_relative_workspace_path(path: &str) -> Result<(), CodingWorkflowError> {
    let path = path.trim();
    if path.is_empty() || Path::new(path).is_absolute() {
        return Err(CodingWorkflowError::new(
            "relative workspace path is required",
        ));
    }
    if Path::new(path)
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(CodingWorkflowError::new(
            "relative workspace path must not traverse outside workspace",
        ));
    }
    Ok(())
}
