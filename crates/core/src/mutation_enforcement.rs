use std::path::{Component, Path};

use serde_json::json;
use tessera_protocol::{
    CodingWorkflowId, MutationMode, OsSandboxMode, OsSandboxProfile, PolicyDecisionId, TaskId,
    ToolDescriptor, ToolId, ToolPermission, ToolSideEffect, WorkspaceMutationScope,
};

use crate::OsSandboxPlanner;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationEnforcementPlanRequest {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub requested_paths: Vec<String>,
    pub mutation_mode: Option<MutationMode>,
    pub policy_decision_id: Option<PolicyDecisionId>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationEnforcementPlan {
    pub scope: WorkspaceMutationScope,
    pub sandbox_profile: OsSandboxProfile,
    pub sandbox_profile_label: Option<String>,
    pub policy_decision_id: Option<PolicyDecisionId>,
    pub executor_blocked: bool,
    pub executor_block_reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationEnforcementError {
    message: String,
}

impl MutationEnforcementError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for MutationEnforcementError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MutationEnforcementError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationEnforcementPlanner {
    workspace_root: String,
}

impl MutationEnforcementPlanner {
    pub fn new(workspace_root: impl Into<String>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    pub fn plan(
        &self,
        request: MutationEnforcementPlanRequest,
    ) -> Result<MutationEnforcementPlan, MutationEnforcementError> {
        validate_requested_paths(&request.requested_paths)?;
        if request.reason.trim().is_empty() {
            return Err(MutationEnforcementError::new(
                "mutation enforcement reason is required",
            ));
        }

        let mutation_mode = request.mutation_mode.unwrap_or(MutationMode::WorktreeFirst);
        if mutation_mode == MutationMode::ExplicitLocal && request.policy_decision_id.is_none() {
            return Err(MutationEnforcementError::new(
                "explicit-local mutation requires policy_decision_id",
            ));
        }

        let worktree_required = mutation_mode == MutationMode::WorktreeFirst;
        let scope = WorkspaceMutationScope {
            workflow_id: request.workflow_id,
            task_id: request.task_id,
            root_label: "project".to_string(),
            allowed_paths: request.requested_paths,
            denied_paths: Vec::new(),
            mutation_mode,
            worktree_required,
            reason: Some(request.reason),
        };

        let sandbox_profile =
            OsSandboxPlanner::new(&self.workspace_root).plan_tool(&workspace_mutation_descriptor());
        let sandbox_profile_label = Some(sandbox_profile_label(&sandbox_profile.mode).to_string());

        Ok(MutationEnforcementPlan {
            scope,
            sandbox_profile,
            sandbox_profile_label,
            policy_decision_id: request.policy_decision_id,
            executor_blocked: true,
            executor_block_reason:
                "executor_not_available_until_policy_checkpoint_and_reviewer_gates".to_string(),
        })
    }
}

fn workspace_mutation_descriptor() -> ToolDescriptor {
    ToolDescriptor {
        id: ToolId::from_static("tool_planned_workspace_mutation"),
        display_name: "Planned workspace mutation".to_string(),
        description: "Metadata-only placeholder for future mutation executor planning".to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object" }),
        required_permissions: vec![ToolPermission::FilesystemWrite],
        side_effects: vec![ToolSideEffect::WritesWorkspace],
        parallel_safe: false,
        metadata: None,
    }
}

fn sandbox_profile_label(mode: &OsSandboxMode) -> &'static str {
    match mode {
        OsSandboxMode::ReadOnly => "read_only",
        OsSandboxMode::WorkspaceWrite => "workspace_write",
        OsSandboxMode::NetworkRequired => "network_required",
        OsSandboxMode::Denied => "denied",
    }
}

fn validate_requested_paths(paths: &[String]) -> Result<(), MutationEnforcementError> {
    if paths.is_empty() {
        return Err(MutationEnforcementError::new("requested_paths is required"));
    }

    for path in paths {
        validate_relative_workspace_path(path)?;
    }
    Ok(())
}

fn validate_relative_workspace_path(path: &str) -> Result<(), MutationEnforcementError> {
    let path = path.trim();
    if path.is_empty() || Path::new(path).is_absolute() {
        return Err(MutationEnforcementError::new(
            "relative workspace path is required",
        ));
    }
    if Path::new(path)
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(MutationEnforcementError::new(
            "relative workspace path must not traverse outside workspace",
        ));
    }
    Ok(())
}
