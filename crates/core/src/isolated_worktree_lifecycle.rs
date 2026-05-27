use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};
use tessera_protocol::{
    CodingWorkflowId, HandoffEvidenceRef, MutationRequestId, PatchProposalId, RunEvent, TaskId,
    WorkspaceWorktreeId, WorkspaceWorktreeLifecycleRecord, WorkspaceWorktreeLifecycleStatus,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorktreeRetentionPolicy {
    RetainOnSuccess,
    CleanupOnFailure,
    RetainAlways,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCheckoutStatus {
    pub source_commit: String,
    pub source_branch_label: Option<String>,
    pub has_tracked_changes: bool,
    pub has_staged_changes: bool,
    pub has_untracked_changes: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsolatedWorktreePlanRequest {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub trace_id: String,
    pub request_id: MutationRequestId,
    pub patch_id: PatchProposalId,
    pub repo_key: String,
    pub data_dir: PathBuf,
    pub worktree_base: Option<PathBuf>,
    pub worktree_base_key: Option<String>,
    pub requested_root_label: Option<String>,
    pub source_checkout: SourceCheckoutStatus,
    pub existing_worktree_leaf_names: Vec<String>,
    pub retention_policy: WorktreeRetentionPolicy,
    pub reason: String,
    pub evidence: Vec<HandoffEvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsolatedWorktreePlan {
    pub worktree_id: WorkspaceWorktreeId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub trace_id: String,
    pub source_commit: String,
    pub source_branch_label: Option<String>,
    pub worktree_root_label: String,
    pub worktree_leaf_name: String,
    pub worktree_base: PathBuf,
    pub worktree_path: PathBuf,
    pub worktree_base_key: String,
    pub created_for_request_id: MutationRequestId,
    pub created_for_patch_id: PatchProposalId,
    pub retention_policy: WorktreeRetentionPolicy,
    pub source_has_untracked_changes: bool,
    pub reason: String,
    pub evidence: Vec<HandoffEvidenceRef>,
}

impl IsolatedWorktreePlan {
    pub fn planned_lifecycle_event(&self) -> RunEvent {
        RunEvent::WorkspaceWorktreeLifecycleRecorded {
            record: WorkspaceWorktreeLifecycleRecord {
                worktree_id: self.worktree_id.clone(),
                workflow_id: self.workflow_id.clone(),
                task_id: self.task_id.clone(),
                trace_id: self.trace_id.clone(),
                source_commit: self.source_commit.clone(),
                source_branch_label: self.source_branch_label.clone(),
                worktree_root_label: self.worktree_root_label.clone(),
                worktree_base_key: self.worktree_base_key.clone(),
                lifecycle_status: WorkspaceWorktreeLifecycleStatus::Planned,
                reason: self.reason.clone(),
                created_for_request_id: Some(self.created_for_request_id.clone()),
                created_for_patch_id: Some(self.created_for_patch_id.clone()),
                evidence: self.evidence.clone(),
                metadata: None,
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsolatedWorktreeLifecycleError {
    message: String,
}

impl IsolatedWorktreeLifecycleError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for IsolatedWorktreeLifecycleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IsolatedWorktreeLifecycleError {}

#[derive(Clone, Debug, Default)]
pub struct IsolatedWorktreeLifecyclePlanner;

impl IsolatedWorktreeLifecyclePlanner {
    pub fn plan(
        &self,
        request: IsolatedWorktreePlanRequest,
    ) -> Result<IsolatedWorktreePlan, IsolatedWorktreeLifecycleError> {
        validate_nonempty(&request.trace_id, "trace_id")?;
        validate_nonempty(&request.reason, "reason")?;
        validate_source_checkout(&request.source_checkout)?;
        validate_trace_safe_key(&request.repo_key, "repo key")?;

        let worktree_root_label = request
            .requested_root_label
            .unwrap_or_else(|| generated_root_label(&request.workflow_id, &request.patch_id));
        validate_worktree_root_label(&worktree_root_label)?;

        let worktree_base_key = request
            .worktree_base_key
            .unwrap_or_else(|| format!("data_dir:worktrees/{}", request.repo_key));
        validate_trace_safe_key(&worktree_base_key, "worktree base key")?;

        let worktree_leaf_name =
            worktree_leaf_name(&worktree_root_label, &request.source_checkout.source_commit);
        if request
            .existing_worktree_leaf_names
            .iter()
            .any(|existing| existing == &worktree_leaf_name)
        {
            return Err(IsolatedWorktreeLifecycleError::new(
                "worktree path collision before execution",
            ));
        }

        let worktree_base = request
            .worktree_base
            .unwrap_or_else(|| request.data_dir.join("worktrees").join(&request.repo_key));
        if worktree_base.as_os_str().is_empty() {
            return Err(IsolatedWorktreeLifecycleError::new(
                "worktree base is required",
            ));
        }
        let worktree_path = worktree_base.join(&worktree_leaf_name);

        Ok(IsolatedWorktreePlan {
            worktree_id: deterministic_worktree_id(
                &request.workflow_id,
                &request.patch_id,
                &request.source_checkout.source_commit,
                &request.trace_id,
            ),
            workflow_id: request.workflow_id,
            task_id: request.task_id,
            trace_id: request.trace_id,
            source_commit: request.source_checkout.source_commit,
            source_branch_label: request.source_checkout.source_branch_label,
            worktree_root_label,
            worktree_leaf_name,
            worktree_base,
            worktree_path,
            worktree_base_key,
            created_for_request_id: request.request_id,
            created_for_patch_id: request.patch_id,
            retention_policy: request.retention_policy,
            source_has_untracked_changes: request.source_checkout.has_untracked_changes,
            reason: request.reason,
            evidence: request.evidence,
        })
    }
}

fn validate_source_checkout(
    source: &SourceCheckoutStatus,
) -> Result<(), IsolatedWorktreeLifecycleError> {
    validate_nonempty(&source.source_commit, "source commit")?;
    validate_trace_safe_key(&source.source_commit, "source commit")?;
    if source.has_staged_changes {
        return Err(IsolatedWorktreeLifecycleError::new(
            "source checkout has staged changes",
        ));
    }
    if source.has_tracked_changes {
        return Err(IsolatedWorktreeLifecycleError::new(
            "source checkout has tracked changes",
        ));
    }
    Ok(())
}

fn generated_root_label(workflow_id: &CodingWorkflowId, patch_id: &PatchProposalId) -> String {
    format!(
        "worktree:{}-{}",
        sanitize_fragment(strip_known_prefix(workflow_id.as_str(), "coding_workflow_")),
        sanitize_fragment(strip_known_prefix(patch_id.as_str(), "patch_proposal_"))
    )
}

fn worktree_leaf_name(root_label: &str, source_commit: &str) -> String {
    let base = root_label
        .strip_prefix("worktree:")
        .unwrap_or(root_label)
        .to_string();
    let source = source_commit.chars().take(12).collect::<String>();
    bounded_leaf_name(&format!("{base}-{source}"), 80)
}

fn deterministic_worktree_id(
    workflow_id: &CodingWorkflowId,
    patch_id: &PatchProposalId,
    source_commit: &str,
    trace_id: &str,
) -> WorkspaceWorktreeId {
    let mut hasher = Sha256::new();
    hasher.update(workflow_id.as_str());
    hasher.update(b"\0");
    hasher.update(patch_id.as_str());
    hasher.update(b"\0");
    hasher.update(source_commit);
    hasher.update(b"\0");
    hasher.update(trace_id);
    let digest = hasher.finalize();
    let suffix = digest
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    WorkspaceWorktreeId::from(format!("workspace_worktree_{suffix}"))
}

fn strip_known_prefix<'a>(value: &'a str, prefix: &str) -> &'a str {
    value.strip_prefix(prefix).unwrap_or(value)
}

fn sanitize_fragment(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            output.push('-');
            last_was_dash = true;
        }
    }
    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "item".to_string()
    } else {
        bounded_leaf_name(&output, 32)
    }
}

fn bounded_leaf_name(value: &str, limit: usize) -> String {
    if value.len() <= limit {
        return value.to_string();
    }
    let mut output = String::new();
    for ch in value.chars() {
        if output.len() + ch.len_utf8() > limit {
            break;
        }
        output.push(ch);
    }
    output.trim_matches('-').to_string()
}

fn validate_worktree_root_label(label: &str) -> Result<(), IsolatedWorktreeLifecycleError> {
    let label = label.trim();
    validate_nonempty(label, "worktree root label")?;
    if matches!(
        label,
        "project" | "local" | "worktree:project" | "worktree:local"
    ) {
        return Err(IsolatedWorktreeLifecycleError::new(
            "worktree root label must not target primary root",
        ));
    }
    validate_trace_safe_key(label, "worktree root label")
}

fn validate_trace_safe_key(
    value: &str,
    label: &'static str,
) -> Result<(), IsolatedWorktreeLifecycleError> {
    let value = value.trim();
    validate_nonempty(value, label)?;
    if value.contains('\\')
        || value.chars().any(char::is_whitespace)
        || Path::new(value).is_absolute()
        || Path::new(value)
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(IsolatedWorktreeLifecycleError::new(format!(
            "{label} must be trace-safe"
        )));
    }
    Ok(())
}

fn validate_nonempty(
    value: &str,
    label: &'static str,
) -> Result<(), IsolatedWorktreeLifecycleError> {
    if value.trim().is_empty() {
        return Err(IsolatedWorktreeLifecycleError::new(format!(
            "{label} is required"
        )));
    }
    Ok(())
}
