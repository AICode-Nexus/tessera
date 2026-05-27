use std::path::{Component, Path, PathBuf};
use std::process::Command;

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
pub struct IsolatedWorktreeLifecycleRunRequest {
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
    pub source_root: PathBuf,
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

#[derive(Clone, Debug, PartialEq)]
pub struct IsolatedWorktreeCreated {
    pub plan: IsolatedWorktreePlan,
    pub source_root: PathBuf,
    pub lifecycle_events: Vec<RunEvent>,
    pub created_by_current_invocation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeCommandInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeCommandOutput {
    pub status_success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait WorktreeCommandRunner {
    fn run(
        &mut self,
        invocation: WorktreeCommandInvocation,
    ) -> Result<WorktreeCommandOutput, String>;
}

#[derive(Clone, Debug, Default)]
pub struct GitWorktreeCommandRunner;

impl WorktreeCommandRunner for GitWorktreeCommandRunner {
    fn run(
        &mut self,
        invocation: WorktreeCommandInvocation,
    ) -> Result<WorktreeCommandOutput, String> {
        if !is_allowed_git_invocation(&invocation) {
            return Err("unsupported git worktree command".to_string());
        }
        let output = Command::new(&invocation.program)
            .args(&invocation.args)
            .current_dir(&invocation.cwd)
            .output()
            .map_err(|error| format!("failed to run {}: {error}", invocation.program))?;

        Ok(WorktreeCommandOutput {
            status_success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

fn is_allowed_git_invocation(invocation: &WorktreeCommandInvocation) -> bool {
    if invocation.program != "git" {
        return false;
    }
    match invocation.args.as_slice() {
        [command, revision] if command == "rev-parse" && revision == "HEAD" => true,
        [command, porcelain, untracked]
            if command == "status"
                && porcelain == "--porcelain"
                && untracked == "--untracked-files=no" =>
        {
            true
        }
        [command, subcommand, detach, worktree_path, source_commit]
            if command == "worktree"
                && subcommand == "add"
                && detach == "--detach"
                && !worktree_path.trim().is_empty()
                && !source_commit.trim().is_empty() =>
        {
            true
        }
        [command, subcommand, worktree_path]
            if command == "worktree"
                && subcommand == "remove"
                && !worktree_path.trim().is_empty() =>
        {
            true
        }
        _ => false,
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

#[derive(Clone, Debug, Default)]
pub struct IsolatedWorktreeLifecycleRunner {
    planner: IsolatedWorktreeLifecyclePlanner,
}

impl IsolatedWorktreeLifecycleRunner {
    pub fn create_detached_worktree<R>(
        &self,
        command_runner: &mut R,
        request: IsolatedWorktreeLifecycleRunRequest,
    ) -> Result<IsolatedWorktreeCreated, IsolatedWorktreeLifecycleError>
    where
        R: WorktreeCommandRunner,
    {
        validate_nonempty_path(&request.source_root, "source root")?;
        let source_commit = run_git_text(
            command_runner,
            git_invocation(&request.source_root, ["rev-parse", "HEAD"]),
            "git rev-parse HEAD",
        )?;
        let status = run_git_text(
            command_runner,
            git_invocation(
                &request.source_root,
                ["status", "--porcelain", "--untracked-files=no"],
            ),
            "git status --porcelain --untracked-files=no",
        )?;
        let source_checkout = source_checkout_status(&source_commit, &status);

        let plan = self.planner.plan(IsolatedWorktreePlanRequest {
            workflow_id: request.workflow_id,
            task_id: request.task_id,
            trace_id: request.trace_id,
            request_id: request.request_id,
            patch_id: request.patch_id,
            repo_key: request.repo_key,
            data_dir: request.data_dir,
            worktree_base: request.worktree_base,
            worktree_base_key: request.worktree_base_key,
            requested_root_label: request.requested_root_label,
            source_checkout,
            existing_worktree_leaf_names: request.existing_worktree_leaf_names,
            retention_policy: request.retention_policy,
            reason: request.reason,
            evidence: request.evidence,
        })?;

        run_git_text(
            command_runner,
            git_invocation(
                &request.source_root,
                [
                    "worktree".to_string(),
                    "add".to_string(),
                    "--detach".to_string(),
                    path_to_string(&plan.worktree_path),
                    plan.source_commit.clone(),
                ],
            ),
            "git worktree add --detach",
        )?;

        Ok(IsolatedWorktreeCreated {
            lifecycle_events: vec![
                plan.planned_lifecycle_event(),
                lifecycle_event(
                    &plan,
                    WorkspaceWorktreeLifecycleStatus::Created,
                    "detached worktree created",
                ),
            ],
            plan,
            source_root: request.source_root,
            created_by_current_invocation: true,
        })
    }

    pub fn retained_lifecycle_event(&self, created: &IsolatedWorktreeCreated) -> RunEvent {
        lifecycle_event(
            &created.plan,
            WorkspaceWorktreeLifecycleStatus::Retained,
            "worktree retained for operator inspection",
        )
    }

    pub fn cleanup_created_worktree<R>(
        &self,
        command_runner: &mut R,
        created: &IsolatedWorktreeCreated,
    ) -> RunEvent
    where
        R: WorktreeCommandRunner,
    {
        match self.try_cleanup_created_worktree(command_runner, created) {
            Ok(event) => event,
            Err(error) => lifecycle_event(
                &created.plan,
                WorkspaceWorktreeLifecycleStatus::CleanupFailed,
                error.to_string(),
            ),
        }
    }

    pub fn try_cleanup_created_worktree<R>(
        &self,
        command_runner: &mut R,
        created: &IsolatedWorktreeCreated,
    ) -> Result<RunEvent, IsolatedWorktreeLifecycleError>
    where
        R: WorktreeCommandRunner,
    {
        if !created.created_by_current_invocation {
            return Err(IsolatedWorktreeLifecycleError::new(
                "worktree was not created by this invocation",
            ));
        }

        let invocation = git_invocation(
            &created.source_root,
            [
                "worktree".to_string(),
                "remove".to_string(),
                path_to_string(&created.plan.worktree_path),
            ],
        );
        let output = command_runner
            .run(invocation)
            .map_err(|error| IsolatedWorktreeLifecycleError::new(format!("git failed: {error}")))?;
        if output.status_success {
            Ok(lifecycle_event(
                &created.plan,
                WorkspaceWorktreeLifecycleStatus::CleanupCompleted,
                "generated worktree removed",
            ))
        } else {
            Ok(lifecycle_event(
                &created.plan,
                WorkspaceWorktreeLifecycleStatus::CleanupFailed,
                command_error_summary("git worktree remove", &output),
            ))
        }
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

fn source_checkout_status(source_commit: &str, status_porcelain: &str) -> SourceCheckoutStatus {
    let mut has_tracked_changes = false;
    let mut has_staged_changes = false;
    let mut has_untracked_changes = false;

    for line in status_porcelain.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 2 {
            continue;
        }
        let index = bytes[0] as char;
        let worktree = bytes[1] as char;
        if index == '?' && worktree == '?' {
            has_untracked_changes = true;
            continue;
        }
        if index != ' ' {
            has_staged_changes = true;
        }
        if worktree != ' ' {
            has_tracked_changes = true;
        }
    }

    SourceCheckoutStatus {
        source_commit: source_commit.trim().to_string(),
        source_branch_label: None,
        has_tracked_changes,
        has_staged_changes,
        has_untracked_changes,
    }
}

fn lifecycle_event(
    plan: &IsolatedWorktreePlan,
    status: WorkspaceWorktreeLifecycleStatus,
    reason: impl Into<String>,
) -> RunEvent {
    RunEvent::WorkspaceWorktreeLifecycleRecorded {
        record: WorkspaceWorktreeLifecycleRecord {
            worktree_id: plan.worktree_id.clone(),
            workflow_id: plan.workflow_id.clone(),
            task_id: plan.task_id.clone(),
            trace_id: plan.trace_id.clone(),
            source_commit: plan.source_commit.clone(),
            source_branch_label: plan.source_branch_label.clone(),
            worktree_root_label: plan.worktree_root_label.clone(),
            worktree_base_key: plan.worktree_base_key.clone(),
            lifecycle_status: status,
            reason: reason.into(),
            created_for_request_id: Some(plan.created_for_request_id.clone()),
            created_for_patch_id: Some(plan.created_for_patch_id.clone()),
            evidence: plan.evidence.clone(),
            metadata: None,
        },
    }
}

fn run_git_text<R>(
    command_runner: &mut R,
    invocation: WorktreeCommandInvocation,
    label: &'static str,
) -> Result<String, IsolatedWorktreeLifecycleError>
where
    R: WorktreeCommandRunner,
{
    let output = command_runner
        .run(invocation)
        .map_err(|error| IsolatedWorktreeLifecycleError::new(format!("{label} failed: {error}")))?;
    if output.status_success {
        Ok(output.stdout)
    } else {
        Err(IsolatedWorktreeLifecycleError::new(command_error_summary(
            label, &output,
        )))
    }
}

fn command_error_summary(label: &str, output: &WorktreeCommandOutput) -> String {
    let stderr = output.stderr.trim();
    if stderr.is_empty() {
        format!("{label} failed")
    } else {
        format!("{label} failed: {stderr}")
    }
}

fn git_invocation<I, S>(cwd: &Path, args: I) -> WorktreeCommandInvocation
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    WorktreeCommandInvocation {
        program: "git".to_string(),
        args: args.into_iter().map(Into::into).collect(),
        cwd: cwd.to_path_buf(),
    }
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

fn validate_nonempty_path(
    value: &Path,
    label: &'static str,
) -> Result<(), IsolatedWorktreeLifecycleError> {
    if value.as_os_str().is_empty() {
        return Err(IsolatedWorktreeLifecycleError::new(format!(
            "{label} is required"
        )));
    }
    Ok(())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
