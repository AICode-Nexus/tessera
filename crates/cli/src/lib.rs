use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use tessera_client::{ClientMessage, ClientMessageRole, ClientSnapshot, ClientWorkflowInspection};
use tessera_config::{ProviderProfile, TesseraConfig};
use tessera_core::{
    AgentLoop, AgentRunOutcome, AgentRunRequest, ApplyPatchDryRunInput, ApplyPatchDryRunOperation,
    ApplyPatchExecutor, ApplyPatchExecutorContext, ApplyPatchExecutorRootKind, ApplyPatchGate,
    ApplyPatchGateBlocker, ApplyPatchGateRecord, ApplyPatchGateRequest, ApplyPatchGateStatus,
    ApplyPatchIsolatedFileRequest, ConversationEngine, ConversationOutcome, ConversationRequest,
    EventSinkAction, GitWorktreeCommandRunner, InstructionDiscoveryOptions,
    InstructionDiscoveryPlanner, IsolatedWorktreeCreated, IsolatedWorktreeLifecycleRunRequest,
    IsolatedWorktreeLifecycleRunner, IsolatedWorktreePlan, LoadedInstructionSet, LoadedSkillSet,
    MutationEnforcementPlan, MutationEnforcementPlanRequest, MutationEnforcementPlanner,
    ReplayRunner, ReplaySummary, RunCancellationToken, RunControls, RunPauseToken,
    RuntimeEventQuery, RuntimePauseCheckpointSummary, RuntimeReader, RuntimeSessionSummary,
    RuntimeTaskOwnerSummary, RuntimeTaskResumer, RuntimeWorktreeLifecycleSummary,
    SkillActivationRequest, SkillDiscoveryOptions, SkillDiscoveryReport, SkillRuntimeOptions,
    SkillRuntimePlanner, TraceApplyPatchResolveRequest, TraceApplyPatchResolvedEnvelope,
    TraceApplyPatchResolver, TraceApplyPatchSelector, WorktreeRetentionPolicy,
};
use tessera_protocol::{
    AgentHandoffId, AgentProfile, AgentProfileId, AgentRunSummary, ApplyPatchDryRunOperationKind,
    ApplyPatchDryRunOperationSummary, ApplyPatchExecutionBlocker, ApplyPatchExecutionId,
    ApplyPatchExecutionRecord, ApplyPatchExecutionStatus, ApplyPatchPreflightBlocker,
    ApplyPatchPreflightId, ApplyPatchPreflightRecord, ApplyPatchPreflightStatus,
    ArtifactBodyRedactionStatus, ArtifactId, ArtifactKind, CodingWorkflowId, ContextReference,
    EventFrame, HandoffEvidenceRef, InstructionSource, ModelProfileId, MutationMode,
    MutationRequestId, MutationRequestOperationKind, MutationRequestProposal,
    MutationRequestStatus, PatchProposal, PatchProposalId, PolicyDecisionId, PolicyOutcome,
    ProviderId, ResumeMode, ReviewerDecisionKind, ReviewerGateDecision, ReviewerGateId, RunEvent,
    SkillActivation, SkillManifest, SkillReferenceSource, SnapshotId, TaskId, TaskOwnerKind,
    TaskOwnerStatus, TaskReattachMode, TaskStatus, ToolCallId, ToolId, ToolPermission,
    ToolPolicyDecision, ToolSideEffect, TraceRecord, WorkspaceCheckpointLifecycleRecord,
    WorkspaceCheckpointLifecycleStatus, WorkspaceMutationScope, WorkspaceWorktreeId,
    WorkspaceWorktreeLifecycleRecord, WorkspaceWorktreeLifecycleStatus,
};
use tessera_providers::{
    mock::MockProvider, ollama::OllamaProvider, openai_compatible::OpenAiCompatibleProvider,
    ChatProvider, ProviderMessage,
};
use tessera_storage::TraceStore;
use tessera_tui::{ChatViewState, LiveClientEvent};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DoctorReport {
    pub status: String,
    pub data_dir: String,
    pub trace_writable: bool,
    pub sqlite_index_healthy: bool,
    pub provider_profiles: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliSessionSummary {
    pub trace_id: String,
    pub event_count: usize,
    pub updated_at: Option<String>,
    pub last_seq: u64,
    pub last_event_kind: Option<String>,
    pub user_preview: String,
    pub assistant_preview: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliTaskOwnerSummary {
    pub lease_id: String,
    pub task_id: String,
    pub trace_id: String,
    pub runtime_id: String,
    pub client_id: Option<String>,
    pub owner_kind: TaskOwnerKind,
    pub status: TaskOwnerStatus,
    pub reattach_mode: TaskReattachMode,
    pub last_seq: Option<u64>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliTranscript {
    pub trace_id: String,
    pub messages: Vec<ClientMessage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliChatOutput {
    pub trace_id: String,
    pub assistant_text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliAgentRunOutput {
    pub trace_id: String,
    pub task_id: String,
    pub status: TaskStatus,
    pub steps_completed: u32,
    pub summary: AgentRunSummary,
    pub assistant_text: String,
    pub instruction_sources: Vec<InstructionSource>,
    pub instruction_warning_count: usize,
    pub skill_activations: Vec<SkillActivation>,
    pub skill_warning_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliApplyPatchOptions {
    pub trace_id: String,
    pub workflow_id: String,
    pub task_id: String,
    pub request_id: String,
    pub patch_id: String,
    pub preflight_id: String,
    pub execution_id: String,
    pub checkpoint_id: String,
    pub reviewer_gate_id: String,
    pub policy_decision_id: String,
    pub sandbox_profile_label: String,
    pub isolated_root: PathBuf,
    pub root_label: String,
    pub allowed_paths: Vec<String>,
    pub patch_body: String,
    pub dry_run: bool,
    pub operator_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliTraceApplyPatchOptions {
    pub trace_id: String,
    pub workflow_id: Option<String>,
    pub request_id: Option<String>,
    pub patch_id: Option<String>,
    pub patch_artifact_id: Option<String>,
    pub preflight_id: Option<String>,
    pub execution_id: Option<String>,
    pub isolated_root: PathBuf,
    pub root_label: String,
    pub allowed_paths: Vec<String>,
    pub patch_body_override: Option<String>,
    pub dry_run: bool,
    pub operator_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliAutoWorktreeApplyPatchOptions {
    pub trace_id: String,
    pub workflow_id: Option<String>,
    pub request_id: Option<String>,
    pub patch_id: Option<String>,
    pub patch_artifact_id: Option<String>,
    pub preflight_id: Option<String>,
    pub execution_id: Option<String>,
    pub allowed_paths: Vec<String>,
    pub patch_body_override: Option<String>,
    pub dry_run: bool,
    pub operator_label: String,
    pub source_root: Option<PathBuf>,
    pub worktree_base: Option<PathBuf>,
    pub retention_policy: WorktreeRetentionPolicy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CliApplyPatchEnvelope {
    pub trace_id: String,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub request_id: MutationRequestId,
    pub patch_id: PatchProposalId,
    pub preflight_id: ApplyPatchPreflightId,
    pub execution_id: ApplyPatchExecutionId,
    pub mutation_request: MutationRequestProposal,
    pub patch_proposal: PatchProposal,
    pub mutation_scope: WorkspaceMutationScope,
    pub enforcement_plan: MutationEnforcementPlan,
    pub checkpoint_lifecycle: WorkspaceCheckpointLifecycleRecord,
    pub reviewer_decision: ReviewerGateDecision,
    pub policy_decision: ToolPolicyDecision,
    pub sandbox_profile_label: Option<String>,
    pub isolated_root: PathBuf,
    pub root_label: String,
    pub allowed_paths: Vec<String>,
    pub patch_body: String,
    pub dry_run: bool,
    pub operator_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliApplyPatchOutput {
    pub trace_id: String,
    pub preflight_status: String,
    pub executor_blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_status: Option<String>,
    pub affected_paths: Vec<String>,
    pub conflict_paths: Vec<String>,
    pub blockers: Vec<String>,
    pub executor_label: String,
    pub isolated_root_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_lifecycle_status: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliWorktreeCleanupOptions {
    pub trace_id: String,
    pub worktree_id: String,
    pub worktree_path: PathBuf,
    pub source_root: Option<PathBuf>,
    pub dry_run: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliWorktreeListOptions {
    pub trace_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliWorkflowInspectOptions {
    pub trace_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliWorkflowInspectionOutput {
    pub workflow_ref: String,
    pub task_ref: String,
    pub active: bool,
    pub mutation_mode: Option<String>,
    pub worktree_required: bool,
    pub patch_count: usize,
    pub diff_artifact_ref_count: usize,
    pub patches_requiring_review_count: usize,
    pub review_bundle_count: usize,
    pub review_evidence_ref_count: usize,
    pub reviewer_gate_count: usize,
    pub accepted_reviewer_gate_count: usize,
    pub rejected_reviewer_gate_count: usize,
    pub revision_requested_reviewer_gate_count: usize,
    pub pending_reviewer_gate_count: usize,
    pub approval_count: usize,
    pub pending_approval_count: usize,
    pub resolved_approval_count: usize,
    pub apply_patch_preflight_count: usize,
    pub executor_ready_preflight_count: usize,
    pub executor_blocked_preflight_count: usize,
    pub apply_patch_execution_count: usize,
    pub successful_apply_patch_execution_count: usize,
    pub failed_apply_patch_execution_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliWorktreeListOutput {
    pub trace_id: String,
    pub worktree_id: String,
    pub workflow_id: String,
    pub task_id: String,
    pub first_event_seq: u64,
    pub latest_event_seq: u64,
    pub latest_status: String,
    pub source_commit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_branch_label: Option<String>,
    pub worktree_root_label: String,
    pub worktree_base_key: String,
    pub created_for_request_id: Option<String>,
    pub created_for_patch_id: Option<String>,
    pub trace_cleanup_candidate: bool,
    pub trace_cleanup_candidate_note: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliWorktreeCleanupOutput {
    pub trace_id: String,
    pub worktree_id: String,
    pub worktree_path: String,
    pub worktree_lifecycle_status: String,
    pub source_commit: String,
    pub worktree_root_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliInstructionDiscoveryOutput {
    pub source_count: usize,
    pub loaded_count: usize,
    pub warning_count: usize,
    pub sources: Vec<InstructionSource>,
    pub context_references: Vec<ContextReference>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliSkillDiscoveryOutput {
    pub skill_count: usize,
    pub source_count: usize,
    pub warning_count: usize,
    pub skills: Vec<SkillManifest>,
    pub sources: Vec<SkillReferenceSource>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliInstructionContextOptions {
    pub workspace: PathBuf,
    pub target_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliSkillContextOptions {
    pub workspace: PathBuf,
    pub target_dir: Option<PathBuf>,
    pub skills: Vec<String>,
    pub references: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliReplaySummary {
    pub trace_id: String,
    pub event_count: usize,
    pub event_kinds: Vec<String>,
    pub assistant_text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CliEventPage {
    pub trace_id: String,
    pub records: Vec<TraceRecord>,
    pub next_since_seq: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliResumableTaskSummary {
    pub checkpoint_id: String,
    pub task_id: String,
    pub trace_id: String,
    pub event_seq: u64,
    pub last_seq: u64,
    pub provider_id: String,
    pub profile_id: String,
    pub model: String,
    pub resume_mode: ResumeMode,
    pub reason: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliProviderProfile {
    pub id: String,
    pub kind: String,
    pub default_model: String,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliConfigValidationReport {
    pub status: String,
    pub data_dir: String,
    pub profiles: Vec<CliConfigProfileValidation>,
    pub issues: Vec<CliConfigValidationIssue>,
}

impl CliConfigValidationReport {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|issue| issue.severity == "error")
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliConfigProfileValidation {
    pub id: String,
    pub kind: String,
    pub default_model: String,
    pub base_url: Option<String>,
    pub api_key_env: Option<String>,
    pub api_key_env_status: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CliConfigValidationIssue {
    pub severity: String,
    pub message: String,
    pub profile_id: Option<String>,
}

impl From<ConversationOutcome> for CliChatOutput {
    fn from(outcome: ConversationOutcome) -> Self {
        Self {
            trace_id: outcome.trace_id,
            assistant_text: outcome.assistant_text,
        }
    }
}

impl From<AgentRunOutcome> for CliAgentRunOutput {
    fn from(outcome: AgentRunOutcome) -> Self {
        let (instruction_sources, instruction_warning_count) =
            instruction_report_from_trace(&outcome.store, &outcome.trace_id);
        let (skill_activations, skill_warning_count) =
            skill_report_from_trace(&outcome.store, &outcome.trace_id);
        Self {
            trace_id: outcome.trace_id,
            task_id: outcome.task_id.to_string(),
            status: outcome.status,
            steps_completed: outcome.summary.steps_completed,
            summary: outcome.summary,
            assistant_text: outcome.assistant_text,
            instruction_sources,
            instruction_warning_count,
            skill_activations,
            skill_warning_count,
        }
    }
}

impl From<&LoadedInstructionSet> for CliInstructionDiscoveryOutput {
    fn from(set: &LoadedInstructionSet) -> Self {
        Self {
            source_count: set.sources.len(),
            loaded_count: set.loaded.len(),
            warning_count: set.warnings.len(),
            sources: set.sources.clone(),
            context_references: set.context_references.clone(),
            warnings: set.warnings.clone(),
        }
    }
}

impl From<&SkillDiscoveryReport> for CliSkillDiscoveryOutput {
    fn from(report: &SkillDiscoveryReport) -> Self {
        Self {
            skill_count: report.manifests.len(),
            source_count: report.sources.len(),
            warning_count: report.warnings.len(),
            skills: report.manifests.clone(),
            sources: report.sources.clone(),
            warnings: report.warnings.clone(),
        }
    }
}

impl From<ReplaySummary> for CliReplaySummary {
    fn from(summary: ReplaySummary) -> Self {
        Self {
            event_count: summary.event_kinds.len(),
            trace_id: summary.trace_id,
            assistant_text: summary.assistant_text,
            event_kinds: summary.event_kinds,
        }
    }
}

impl From<tessera_core::RuntimeEventPage> for CliEventPage {
    fn from(page: tessera_core::RuntimeEventPage) -> Self {
        Self {
            trace_id: page.trace_id,
            records: page.records,
            next_since_seq: page.next_since_seq,
        }
    }
}

impl From<RuntimeTaskOwnerSummary> for CliTaskOwnerSummary {
    fn from(owner: RuntimeTaskOwnerSummary) -> Self {
        Self {
            lease_id: owner.lease_id.to_string(),
            task_id: owner.task_id.to_string(),
            trace_id: owner.trace_id,
            runtime_id: owner.runtime_id.to_string(),
            client_id: owner.client_id.map(|client_id| client_id.to_string()),
            owner_kind: owner.owner_kind,
            status: owner.status,
            reattach_mode: owner.reattach_mode,
            last_seq: owner.last_seq,
            reason: owner.reason,
        }
    }
}

impl From<RuntimePauseCheckpointSummary> for CliResumableTaskSummary {
    fn from(checkpoint: RuntimePauseCheckpointSummary) -> Self {
        Self {
            checkpoint_id: checkpoint.checkpoint_id.to_string(),
            task_id: checkpoint.task_id.to_string(),
            trace_id: checkpoint.trace_id,
            event_seq: checkpoint.event_seq,
            last_seq: checkpoint.last_seq,
            provider_id: checkpoint.provider_id.to_string(),
            profile_id: checkpoint.profile_id.to_string(),
            model: checkpoint.model,
            resume_mode: checkpoint.resume_mode,
            reason: checkpoint.reason,
            created_at: checkpoint
                .created_at
                .map(|timestamp| timestamp.as_str().to_string()),
        }
    }
}

impl From<&ClientWorkflowInspection> for CliWorkflowInspectionOutput {
    fn from(inspection: &ClientWorkflowInspection) -> Self {
        Self {
            workflow_ref: inspection.workflow_ref.clone(),
            task_ref: inspection.task_ref.clone(),
            active: inspection.active,
            mutation_mode: inspection.mutation_mode.clone(),
            worktree_required: inspection.worktree_required,
            patch_count: inspection.patch_count,
            diff_artifact_ref_count: inspection.diff_artifact_ref_count,
            patches_requiring_review_count: inspection.patches_requiring_review_count,
            review_bundle_count: inspection.review_bundle_count,
            review_evidence_ref_count: inspection.review_evidence_ref_count,
            reviewer_gate_count: inspection.reviewer_gate_count,
            accepted_reviewer_gate_count: inspection.accepted_reviewer_gate_count,
            rejected_reviewer_gate_count: inspection.rejected_reviewer_gate_count,
            revision_requested_reviewer_gate_count: inspection
                .revision_requested_reviewer_gate_count,
            pending_reviewer_gate_count: inspection.pending_reviewer_gate_count,
            approval_count: inspection.approval_count,
            pending_approval_count: inspection.pending_approval_count,
            resolved_approval_count: inspection.resolved_approval_count,
            apply_patch_preflight_count: inspection.apply_patch_preflight_count,
            executor_ready_preflight_count: inspection.executor_ready_preflight_count,
            executor_blocked_preflight_count: inspection.executor_blocked_preflight_count,
            apply_patch_execution_count: inspection.apply_patch_execution_count,
            successful_apply_patch_execution_count: inspection
                .successful_apply_patch_execution_count,
            failed_apply_patch_execution_count: inspection.failed_apply_patch_execution_count,
        }
    }
}

impl From<&ProviderProfile> for CliProviderProfile {
    fn from(profile: &ProviderProfile) -> Self {
        Self {
            id: profile.id.clone(),
            kind: profile.kind.clone(),
            default_model: profile.default_model.clone(),
            base_url: profile.base_url.clone(),
            api_key_env: profile.api_key_env.clone(),
        }
    }
}

impl From<RuntimeSessionSummary> for CliSessionSummary {
    fn from(session: RuntimeSessionSummary) -> Self {
        Self {
            trace_id: session.trace_id,
            event_count: session.event_count,
            updated_at: session
                .updated_at
                .map(|timestamp| timestamp.as_str().to_string()),
            last_seq: session.last_seq,
            last_event_kind: session.last_event_kind,
            user_preview: session.user_preview,
            assistant_preview: session.assistant_preview,
        }
    }
}

pub fn format_doctor_lines(report: &DoctorReport) -> Vec<String> {
    vec![
        format!("status: {}", report.status),
        format!("data_dir: {}", report.data_dir),
        format!("trace_writable: {}", report.trace_writable),
        format!("sqlite_index_healthy: {}", report.sqlite_index_healthy),
        format!(
            "provider_profiles: {}",
            if report.provider_profiles.is_empty() {
                "none".to_string()
            } else {
                report.provider_profiles.join(", ")
            }
        ),
    ]
}

pub type Result<T> = anyhow::Result<T>;

pub const VERSION_TEXT: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (git ",
    env!("TESSERA_GIT_SHA"),
    ")"
);

const APPLY_PATCH_EXECUTOR_LABEL: &str = "core.apply_patch_executor.v1";
const APPLY_PATCH_REQUEST_SOURCE_LABEL: &str = "cli.apply-patch";
const APPLY_PATCH_MAX_BODY_BYTES: usize = 1024 * 1024;

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CliReplCommand {
    Help,
    NewThread,
    Clear,
    Cancel,
    PauseTask(Option<String>),
    ResumeTask(String),
    ResumeTasks,
    Paste,
    Profiles,
    SwitchProfile(String),
    Sessions,
    ResumeSession(String),
    Doctor,
    History,
    Status,
    Export,
    Quit,
    Unknown(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliReplCommandOutcome {
    pub should_quit: bool,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CliReplSession {
    snapshot: ClientSnapshot,
}

impl CliReplSession {
    pub fn new(config: &TesseraConfig, provider_id: &str) -> Result<Self> {
        ensure_provider_profile(config, provider_id)?;
        let profile_ids = config
            .providers
            .iter()
            .map(|profile| profile.id.clone())
            .collect::<Vec<_>>();
        Ok(Self {
            snapshot: ClientSnapshot::with_profiles(provider_id, profile_ids),
        })
    }

    pub fn snapshot(&self) -> &ClientSnapshot {
        &self.snapshot
    }

    pub fn snapshot_mut(&mut self) -> &mut ClientSnapshot {
        &mut self.snapshot
    }

    pub fn handle_command(
        &mut self,
        config: &TesseraConfig,
        command: CliReplCommand,
    ) -> Result<CliReplCommandOutcome> {
        match command {
            CliReplCommand::Help => Ok(CliReplCommandOutcome::continue_with(chat_command_lines())),
            CliReplCommand::NewThread => {
                self.snapshot.start_new_thread();
                Ok(CliReplCommandOutcome::continue_with(["new thread started"]))
            }
            CliReplCommand::Clear => {
                self.snapshot.start_new_thread();
                Ok(CliReplCommandOutcome::continue_with([
                    "current thread cleared",
                ]))
            }
            CliReplCommand::Cancel => Ok(CliReplCommandOutcome::continue_with([
                "no active run to cancel",
            ])),
            CliReplCommand::PauseTask(task_id) => {
                let task_label = task_id.unwrap_or_else(|| "latest running task".to_string());
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "pause requested for {task_label} as metadata-only CLI intent; no runtime execution was invoked"
                )]))
            }
            CliReplCommand::ResumeTask(task_id) => {
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "resume requested for {task_id} as metadata-only CLI intent; no runtime execution was invoked"
                )]))
            }
            CliReplCommand::ResumeTasks => Ok(CliReplCommandOutcome::continue_with([
                "/resume-tasks requires an active data directory".to_string(),
            ])),
            CliReplCommand::Paste => Ok(CliReplCommandOutcome::continue_with([
                "/paste is only available in the interactive REPL".to_string(),
            ])),
            CliReplCommand::Profiles => {
                let lines: Vec<String> = config
                    .providers
                    .iter()
                    .map(|profile| {
                        let marker = if profile.id == self.snapshot.status.active_profile {
                            "*"
                        } else {
                            " "
                        };
                        format!("{marker} {} ({})", profile.id, profile.kind)
                    })
                    .collect();
                Ok(CliReplCommandOutcome::continue_with(lines))
            }
            CliReplCommand::SwitchProfile(profile_id) => {
                ensure_provider_profile(config, &profile_id)?;
                self.snapshot.status.active_profile = profile_id.clone();
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "profile switched to {profile_id}"
                )]))
            }
            CliReplCommand::Sessions => Ok(CliReplCommandOutcome::continue_with([
                "/sessions requires an active data directory".to_string(),
            ])),
            CliReplCommand::ResumeSession(_) => Ok(CliReplCommandOutcome::continue_with([
                "/resume requires an active data directory".to_string(),
            ])),
            CliReplCommand::Doctor => Ok(CliReplCommandOutcome::continue_with([
                "/doctor requires an active data directory".to_string(),
            ])),
            CliReplCommand::Status => {
                Ok(CliReplCommandOutcome::continue_with([self.status_line()]))
            }
            CliReplCommand::History => Ok(CliReplCommandOutcome::continue_with(
                format_history_lines(&self.snapshot.projection.messages),
            )),
            CliReplCommand::Export => Ok(CliReplCommandOutcome::continue_with(
                self.snapshot.export_markdown().lines().map(str::to_string),
            )),
            CliReplCommand::Quit => Ok(CliReplCommandOutcome {
                should_quit: true,
                lines: vec!["bye".to_string()],
            }),
            CliReplCommand::Unknown(command) => {
                Ok(CliReplCommandOutcome::continue_with([format!(
                    "unknown command `{command}`; type /help for commands"
                )]))
            }
        }
    }

    pub fn handle_command_with_data_dir(
        &mut self,
        data_dir: impl AsRef<Path>,
        config: &TesseraConfig,
        command: CliReplCommand,
    ) -> Result<CliReplCommandOutcome> {
        match command {
            CliReplCommand::Sessions => self.list_sessions(data_dir),
            CliReplCommand::ResumeTasks => self.list_resume_tasks(data_dir, config),
            CliReplCommand::ResumeSession(trace_id) => self.resume_session(data_dir, &trace_id),
            CliReplCommand::Doctor => self.doctor(data_dir, config),
            other => self.handle_command(config, other),
        }
    }

    fn status_line(&self) -> String {
        format!(
            "profile {} | {} | {} | {} | {} | {}",
            self.snapshot.status.active_profile,
            self.snapshot.status.task_summary,
            self.snapshot.status.usage_summary,
            self.snapshot.status.cache_summary,
            self.snapshot.status.cost_summary,
            self.snapshot.status.context_summary
        )
    }

    fn list_sessions(&self, data_dir: impl AsRef<Path>) -> Result<CliReplCommandOutcome> {
        Ok(CliReplCommandOutcome::continue_with(format_session_lines(
            &list_sessions(data_dir)?,
        )))
    }

    fn list_resume_tasks(
        &self,
        data_dir: impl AsRef<Path>,
        config: &TesseraConfig,
    ) -> Result<CliReplCommandOutcome> {
        Ok(CliReplCommandOutcome::continue_with(
            format_resume_task_lines(&list_resumable_pause_checkpoints(data_dir, config)?),
        ))
    }

    fn doctor(
        &self,
        data_dir: impl AsRef<Path>,
        config: &TesseraConfig,
    ) -> Result<CliReplCommandOutcome> {
        Ok(CliReplCommandOutcome::continue_with(format_doctor_lines(
            &run_doctor_with_config(data_dir, config)?,
        )))
    }

    fn resume_session(
        &mut self,
        data_dir: impl AsRef<Path>,
        selector: &str,
    ) -> Result<CliReplCommandOutcome> {
        let data_dir = data_dir.as_ref();
        let trace_id = resolve_session_selector(data_dir, selector)?;
        let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
        let page = reader.list_events(RuntimeEventQuery::new(trace_id.as_str()))?;
        if page.records.is_empty() {
            return Err(anyhow::anyhow!("trace not found or empty: {trace_id}"));
        }

        self.snapshot.start_new_thread();
        for record in &page.records {
            self.snapshot.apply_trace_record(record);
        }
        let message_count = self.snapshot.projection.messages.len();
        Ok(CliReplCommandOutcome::continue_with([format!(
            "resumed trace {trace_id} ({message_count} messages)"
        )]))
    }
}

impl CliReplCommandOutcome {
    fn continue_with<I, S>(lines: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            should_quit: false,
            lines: lines.into_iter().map(Into::into).collect(),
        }
    }
}

pub fn parse_repl_command(input: &str) -> Option<CliReplCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let command = trimmed.split_whitespace().next().unwrap_or(trimmed);
    let argument = trimmed[command.len()..].trim();
    Some(match command {
        "/help" | "/commands" | "/?" => CliReplCommand::Help,
        "/new" => CliReplCommand::NewThread,
        "/clear" => CliReplCommand::Clear,
        "/cancel" => CliReplCommand::Cancel,
        "/pause" => CliReplCommand::PauseTask(if argument.is_empty() {
            None
        } else {
            Some(argument.to_string())
        }),
        "/resume-task" if !argument.is_empty() => CliReplCommand::ResumeTask(argument.to_string()),
        "/resume-tasks" => CliReplCommand::ResumeTasks,
        "/paste" => CliReplCommand::Paste,
        "/profiles" => CliReplCommand::Profiles,
        "/profile" if !argument.is_empty() => CliReplCommand::SwitchProfile(argument.to_string()),
        "/sessions" => CliReplCommand::Sessions,
        "/resume" if !argument.is_empty() => CliReplCommand::ResumeSession(argument.to_string()),
        "/doctor" => CliReplCommand::Doctor,
        "/history" => CliReplCommand::History,
        "/status" => CliReplCommand::Status,
        "/export" => CliReplCommand::Export,
        "/quit" | "/exit" => CliReplCommand::Quit,
        _ => CliReplCommand::Unknown(trimmed.to_string()),
    })
}

pub fn list_sessions(data_dir: impl AsRef<Path>) -> Result<Vec<CliSessionSummary>> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    Ok(reader
        .list_sessions()?
        .into_iter()
        .map(CliSessionSummary::from)
        .collect())
}

pub fn latest_session_trace_id(data_dir: impl AsRef<Path>) -> Result<String> {
    list_sessions(data_dir)?
        .into_iter()
        .next()
        .map(|session| session.trace_id)
        .ok_or_else(|| anyhow::anyhow!("no sessions found to continue"))
}

pub fn list_resumable_tasks(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
) -> Result<Vec<CliResumableTaskSummary>> {
    Ok(list_resumable_pause_checkpoints(data_dir, config)?
        .into_iter()
        .map(CliResumableTaskSummary::from)
        .collect())
}

pub fn list_task_owners(
    data_dir: impl AsRef<Path>,
    trace_id: &str,
) -> Result<Vec<CliTaskOwnerSummary>> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    Ok(reader
        .list_task_owners(trace_id)?
        .into_iter()
        .map(CliTaskOwnerSummary::from)
        .collect())
}

pub fn run_workflow_inspect_options(
    data_dir: impl AsRef<Path>,
    options: CliWorkflowInspectOptions,
) -> Result<Vec<CliWorkflowInspectionOutput>> {
    let snapshot = load_workflow_inspect_snapshot(data_dir, &options.trace_id)?;
    Ok(snapshot
        .workflow_inspections
        .iter()
        .map(CliWorkflowInspectionOutput::from)
        .collect())
}

pub fn format_workflow_inspection_lines(
    inspections: &[CliWorkflowInspectionOutput],
) -> Vec<String> {
    if inspections.is_empty() {
        return vec!["workflow inspection records: none".to_string()];
    }

    let mut lines = vec![format!(
        "workflow inspection records: {}",
        inspections.len()
    )];
    lines.extend(inspections.iter().map(|inspection| {
        format!(
            "{} {} active={} mutation={} worktree_required={} patches={} diff_refs={} patches_requiring_review={} review_bundles={} review_evidence_refs={} review_gates={} accepted={} rejected={} revision_requested={} pending={} approvals={} pending_approvals={} resolved_approvals={} preflights={} executor_ready={} executor_blocked={} executions={} successful={} failed={}",
            inspection.workflow_ref,
            inspection.task_ref,
            inspection.active,
            inspection.mutation_mode.as_deref().unwrap_or("none"),
            inspection.worktree_required,
            inspection.patch_count,
            inspection.diff_artifact_ref_count,
            inspection.patches_requiring_review_count,
            inspection.review_bundle_count,
            inspection.review_evidence_ref_count,
            inspection.reviewer_gate_count,
            inspection.accepted_reviewer_gate_count,
            inspection.rejected_reviewer_gate_count,
            inspection.revision_requested_reviewer_gate_count,
            inspection.pending_reviewer_gate_count,
            inspection.approval_count,
            inspection.pending_approval_count,
            inspection.resolved_approval_count,
            inspection.apply_patch_preflight_count,
            inspection.executor_ready_preflight_count,
            inspection.executor_blocked_preflight_count,
            inspection.apply_patch_execution_count,
            inspection.successful_apply_patch_execution_count,
            inspection.failed_apply_patch_execution_count,
        )
    }));
    lines
}

pub fn format_session_lines(sessions: &[CliSessionSummary]) -> Vec<String> {
    if sessions.is_empty() {
        return vec!["no sessions found".to_string()];
    }

    sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let updated_at = session.updated_at.as_deref().unwrap_or("unknown");
            let preview = if !session.user_preview.is_empty() {
                session.user_preview.as_str()
            } else {
                session.assistant_preview.as_str()
            };
            format!(
                "{}. {} | {} events | updated {} | {}",
                index + 1,
                session.trace_id,
                session.event_count,
                updated_at,
                preview
            )
        })
        .collect()
}

pub fn format_task_owner_lines(owners: &[CliTaskOwnerSummary]) -> Vec<String> {
    if owners.is_empty() {
        return vec!["no task owners found".to_string()];
    }

    owners
        .iter()
        .enumerate()
        .map(|(index, owner)| {
            let reason = owner.reason.as_deref().unwrap_or("none");
            let last_seq = owner
                .last_seq
                .map(|seq| seq.to_string())
                .unwrap_or_else(|| "none".to_string());
            format!(
                "{}. {} | lease {} | runtime {} | kind {} | status {} | reattach {} | last_seq {} | reason {}",
                index + 1,
                owner.task_id,
                owner.lease_id,
                owner.runtime_id,
                task_owner_kind_label(owner.owner_kind),
                task_owner_status_label(owner.status),
                task_reattach_mode_label(owner.reattach_mode),
                last_seq,
                reason
            )
        })
        .collect()
}

fn list_resumable_pause_checkpoints(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
) -> Result<Vec<RuntimePauseCheckpointSummary>> {
    let data_dir = data_dir.as_ref();
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let configured_provider_ids = config
        .providers
        .iter()
        .map(|profile| profile.id.as_str())
        .collect::<HashSet<_>>();
    let mut checkpoints = Vec::new();

    for session in reader.list_sessions()? {
        let tasks = reader.list_tasks(&session.trace_id)?;
        for checkpoint in reader.list_pause_checkpoints(&session.trace_id)? {
            let task_is_paused = tasks.iter().any(|task| {
                task.task_id == checkpoint.task_id && task.status == TaskStatus::Paused
            });
            if !task_is_paused
                || checkpoint.resume_mode != ResumeMode::FromTraceProjection
                || !configured_provider_ids.contains(checkpoint.provider_id.as_str())
            {
                continue;
            }
            checkpoints.push(checkpoint);
        }
    }

    checkpoints.sort_by(|left, right| {
        let left_created = left.created_at.as_ref().map(|timestamp| timestamp.as_str());
        let right_created = right
            .created_at
            .as_ref()
            .map(|timestamp| timestamp.as_str());
        right_created
            .cmp(&left_created)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });
    Ok(checkpoints)
}

pub fn format_resumable_task_lines(tasks: &[CliResumableTaskSummary]) -> Vec<String> {
    if tasks.is_empty() {
        return vec!["no resumable paused tasks found".to_string()];
    }

    tasks
        .iter()
        .enumerate()
        .map(|(index, task)| {
            let reason = task.reason.as_deref().unwrap_or("none");
            format!(
                "{}. {} | trace {} | provider {} | checkpoint {} | reason {}",
                index + 1,
                task.task_id,
                task.trace_id,
                task.provider_id,
                task.checkpoint_id,
                reason
            )
        })
        .collect()
}

fn format_resume_task_lines(checkpoints: &[RuntimePauseCheckpointSummary]) -> Vec<String> {
    let tasks = checkpoints
        .iter()
        .cloned()
        .map(CliResumableTaskSummary::from)
        .collect::<Vec<_>>();
    format_resumable_task_lines(&tasks)
}

fn resolve_session_selector(data_dir: &Path, selector: &str) -> Result<String> {
    let Ok(index) = selector.parse::<usize>() else {
        return Ok(selector.to_string());
    };
    if index == 0 {
        return Err(anyhow::anyhow!(
            "session index out of range: 0 (available sessions: {})",
            list_sessions(data_dir)?.len()
        ));
    }

    let sessions = list_sessions(data_dir)?;
    sessions
        .get(index - 1)
        .map(|session| session.trace_id.clone())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "session index out of range: {index} (available sessions: {})",
                sessions.len()
            )
        })
}

pub fn format_history_lines(messages: &[ClientMessage]) -> Vec<String> {
    if messages.is_empty() {
        return vec!["no messages in current thread".to_string()];
    }

    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let role = match message.role {
                ClientMessageRole::System => "system",
                ClientMessageRole::User => "user",
                ClientMessageRole::Assistant => "assistant",
                ClientMessageRole::Reasoning => "reasoning",
            };
            format!(
                "{}. {role}: {}",
                index + 1,
                compact_history_preview(&message.content)
            )
        })
        .collect()
}

fn compact_history_preview(content: &str) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_CHARS: usize = 120;
    if collapsed.chars().count() <= MAX_CHARS {
        return collapsed;
    }

    collapsed
        .chars()
        .take(MAX_CHARS.saturating_sub(3))
        .chain("...".chars())
        .collect()
}

pub fn load_transcript(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<CliTranscript> {
    let snapshot = load_transcript_snapshot(data_dir, trace_id)?;
    Ok(CliTranscript {
        trace_id: trace_id.to_string(),
        messages: snapshot.projection.messages,
    })
}

pub fn export_transcript_markdown(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<String> {
    Ok(load_transcript_snapshot(data_dir, trace_id)?.export_markdown())
}

pub fn replay_trace(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<CliReplaySummary> {
    let store = TraceStore::open(data_dir)?;
    Ok(CliReplaySummary::from(
        ReplayRunner::new(&store).replay(trace_id)?,
    ))
}

pub fn format_replay_summary(summary: &CliReplaySummary) -> String {
    format!(
        "trace: {}\nevents: {}\nassistant:\n{}\n",
        summary.trace_id, summary.event_count, summary.assistant_text
    )
}

pub fn list_events(
    data_dir: impl AsRef<Path>,
    trace_id: &str,
    since_seq: Option<u64>,
    limit: Option<usize>,
) -> Result<CliEventPage> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let mut query = RuntimeEventQuery::new(trace_id);
    if let Some(since_seq) = since_seq {
        query = query.since_seq(since_seq);
    }
    if let Some(limit) = limit {
        query = query.limit(limit);
    }
    Ok(CliEventPage::from(reader.list_events(query)?))
}

pub fn format_event_lines(page: &CliEventPage) -> Vec<String> {
    let mut lines = vec![format!("trace: {}", page.trace_id)];
    lines.extend(page.records.iter().map(|record| {
        format!(
            "{} | {} | {}",
            record.seq,
            record.timestamp.as_str(),
            record.event_kind
        )
    }));
    lines.push(format!(
        "next_since_seq: {}",
        page.next_since_seq
            .map(|seq| seq.to_string())
            .unwrap_or_else(|| "none".to_string())
    ));
    lines
}

pub fn list_profiles(config: &TesseraConfig) -> Vec<CliProviderProfile> {
    config
        .providers
        .iter()
        .map(CliProviderProfile::from)
        .collect()
}

pub fn format_profile_lines(profiles: &[CliProviderProfile]) -> Vec<String> {
    if profiles.is_empty() {
        return vec!["no provider profiles configured".to_string()];
    }

    profiles
        .iter()
        .map(|profile| {
            let base_url = profile.base_url.as_deref().unwrap_or("none");
            let api_key_env = profile.api_key_env.as_deref().unwrap_or("none");
            format!(
                "{} | {} | model {} | base_url {} | api_key_env {}",
                profile.id, profile.kind, profile.default_model, base_url, api_key_env
            )
        })
        .collect()
}

pub fn validate_config(
    config: &TesseraConfig,
    data_dir: impl AsRef<Path>,
) -> CliConfigValidationReport {
    let mut issues = Vec::new();
    let mut seen_profile_ids = HashSet::new();
    let mut profiles = Vec::new();

    if config.providers.is_empty() {
        issues.push(config_validation_error(
            None,
            "at least one provider profile is required",
        ));
    }

    for profile in &config.providers {
        let mut status = "ok".to_string();
        if !seen_profile_ids.insert(profile.id.clone()) {
            status = "error".to_string();
            issues.push(config_validation_error(
                Some(&profile.id),
                format!("duplicate provider id `{}`", profile.id),
            ));
        }

        match profile.kind.as_str() {
            "mock" | "ollama" => {}
            "openai-compatible" | "openai_compatible" => {
                if profile.base_url.is_none() {
                    status = "error".to_string();
                    issues.push(config_validation_error(
                        Some(&profile.id),
                        format!(
                            "provider `{}` kind openai-compatible requires base_url",
                            profile.id
                        ),
                    ));
                }
            }
            other => {
                status = "error".to_string();
                issues.push(config_validation_error(
                    Some(&profile.id),
                    format!(
                        "unsupported provider kind `{other}` for profile `{}`",
                        profile.id
                    ),
                ));
            }
        }

        let api_key_env_status = profile.api_key_env.as_ref().map(|env_name| {
            if std::env::var_os(env_name).is_some() {
                "set".to_string()
            } else {
                status = "error".to_string();
                issues.push(config_validation_error(
                    Some(&profile.id),
                    format!(
                        "provider `{}` api_key_env `{env_name}` is not set",
                        profile.id
                    ),
                ));
                "missing".to_string()
            }
        });

        profiles.push(CliConfigProfileValidation {
            id: profile.id.clone(),
            kind: profile.kind.clone(),
            default_model: profile.default_model.clone(),
            base_url: profile.base_url.clone(),
            api_key_env: profile.api_key_env.clone(),
            api_key_env_status,
            status,
        });
    }

    let status = if issues.iter().any(|issue| issue.severity == "error") {
        "error"
    } else {
        "ok"
    };

    CliConfigValidationReport {
        status: status.to_string(),
        data_dir: data_dir.as_ref().to_string_lossy().to_string(),
        profiles,
        issues,
    }
}

pub fn format_config_validation_lines(report: &CliConfigValidationReport) -> Vec<String> {
    let mut lines = vec![
        format!("status: {}", report.status),
        format!("data_dir: {}", report.data_dir),
    ];

    for profile in &report.profiles {
        let mut details = vec![
            profile.kind.clone(),
            format!("model {}", profile.default_model),
        ];
        if let Some(api_key_env) = &profile.api_key_env {
            let api_key_env_status = profile.api_key_env_status.as_deref().unwrap_or("unknown");
            details.push(format!("api_key_env {api_key_env} {api_key_env_status}"));
        }
        lines.push(format!(
            "profile {}: {} ({})",
            profile.id,
            profile.status,
            details.join(", ")
        ));
    }

    for issue in &report.issues {
        lines.push(format!("{}: {}", issue.severity, issue.message));
    }

    lines
}

fn config_validation_error(
    profile_id: Option<&str>,
    message: impl Into<String>,
) -> CliConfigValidationIssue {
    CliConfigValidationIssue {
        severity: "error".to_string(),
        message: message.into(),
        profile_id: profile_id.map(str::to_string),
    }
}

fn load_transcript_snapshot(data_dir: impl AsRef<Path>, trace_id: &str) -> Result<ClientSnapshot> {
    load_trace_snapshot(data_dir, trace_id, "transcript")
}

fn load_workflow_inspect_snapshot(
    data_dir: impl AsRef<Path>,
    trace_id: &str,
) -> Result<ClientSnapshot> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let page = reader.list_events(RuntimeEventQuery::new(trace_id))?;
    let mut snapshot = ClientSnapshot::new("workflow-inspect");
    for record in &page.records {
        snapshot.apply_trace_record(record);
    }
    Ok(snapshot)
}

fn load_trace_snapshot(
    data_dir: impl AsRef<Path>,
    trace_id: &str,
    projection_profile: &str,
) -> Result<ClientSnapshot> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let page = reader.list_events(RuntimeEventQuery::new(trace_id))?;
    if page.records.is_empty() {
        return Err(anyhow::anyhow!("trace not found or empty: {trace_id}"));
    }

    let mut snapshot = ClientSnapshot::new(projection_profile);
    for record in &page.records {
        snapshot.apply_trace_record(record);
    }
    Ok(snapshot)
}

pub fn provider_history_from_snapshot(snapshot: &ClientSnapshot) -> Vec<ProviderMessage> {
    snapshot
        .projection
        .messages
        .iter()
        .filter(|message| !message.content.trim().is_empty())
        .filter_map(|message| match message.role {
            ClientMessageRole::User => Some(ProviderMessage::user(message.content.clone())),
            ClientMessageRole::Assistant => {
                Some(ProviderMessage::assistant(message.content.clone()))
            }
            ClientMessageRole::System | ClientMessageRole::Reasoning => None,
        })
        .collect()
}

pub async fn run_repl_prompt_with_writer<F>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    prompt: impl Into<String>,
    write_delta: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&str),
{
    run_repl_prompt_with_writer_and_controls(
        data_dir,
        config,
        session,
        prompt,
        RunControls::default(),
        write_delta,
    )
    .await
}

pub async fn run_repl_prompt_with_writer_and_controls<F>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    prompt: impl Into<String>,
    controls: RunControls,
    mut write_delta: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&str),
{
    let provider_id = session.snapshot.status.active_profile.clone();
    let history = provider_history_from_snapshot(&session.snapshot);
    let snapshot = &mut session.snapshot;
    run_chat_with_config_history_controls_and_events(
        data_dir,
        config,
        &provider_id,
        prompt,
        history,
        controls,
        |frame| {
            snapshot.apply_event(frame);
            if let RunEvent::AssistantDelta { text, .. } = &frame.event {
                write_delta(text);
            }
            EventSinkAction::Continue
        },
    )
    .await
}

pub async fn run_chat_repl_with_config(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_chat_repl_with_io_and_resume(
        data_dir,
        config,
        provider_id,
        None,
        BufReader::new(stdin),
        stdout.lock(),
    )
    .await?;
    Ok(())
}

pub async fn run_chat_repl_with_config_and_resume(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
    resume_trace_id: Option<String>,
) -> Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_chat_repl_with_io_and_resume(
        data_dir,
        config,
        provider_id,
        resume_trace_id,
        BufReader::new(stdin),
        stdout.lock(),
    )
    .await?;
    Ok(())
}

pub async fn run_chat_repl_with_io<R, W>(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
    input: R,
    output: W,
) -> Result<ClientSnapshot>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    run_chat_repl_with_io_and_resume(data_dir, config, provider_id, None, input, output).await
}

const REPL_LINE_BUFFER_CAPACITY: usize = 128;

type ReplLineReceiver = mpsc::Receiver<io::Result<String>>;

fn spawn_repl_line_reader<R>(mut input: R) -> ReplLineReceiver
where
    R: BufRead + Send + 'static,
{
    let (sender, receiver) = mpsc::channel(REPL_LINE_BUFFER_CAPACITY);
    thread::spawn(move || loop {
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if sender.blocking_send(Ok(line)).is_err() {
                    break;
                }
            }
            Err(error) => {
                let _ = sender.blocking_send(Err(error));
                break;
            }
        }
    });
    receiver
}

async fn next_repl_line(
    input_lines: &mut ReplLineReceiver,
    pending_lines: &mut VecDeque<String>,
) -> Result<Option<String>> {
    if let Some(line) = pending_lines.pop_front() {
        return Ok(Some(line));
    }
    match input_lines.recv().await {
        Some(Ok(line)) => Ok(Some(line)),
        Some(Err(error)) => Err(error.into()),
        None => Ok(None),
    }
}

pub async fn run_chat_repl_with_io_and_resume<R, W>(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
    resume_trace_id: Option<String>,
    input: R,
    mut output: W,
) -> Result<ClientSnapshot>
where
    R: BufRead + Send + 'static,
    W: Write,
{
    let mut input_lines = spawn_repl_line_reader(input);
    let mut pending_lines = VecDeque::new();
    let mut session = CliReplSession::new(&config, &provider_id)?;
    for line in repl_startup_lines(&data_dir, &config, &provider_id) {
        writeln!(output, "{line}")?;
    }
    if let Some(trace_id) = resume_trace_id {
        let outcome = session.handle_command_with_data_dir(
            &data_dir,
            &config,
            CliReplCommand::ResumeSession(trace_id),
        )?;
        for line in outcome.lines {
            writeln!(output, "{line}")?;
        }
    }

    loop {
        write!(
            output,
            "\ntessera({})> ",
            session.snapshot.status.active_profile
        )?;
        output.flush()?;

        let Some(line) = next_repl_line(&mut input_lines, &mut pending_lines).await? else {
            break;
        };
        let user_input = line.trim();
        if user_input.is_empty() {
            continue;
        }

        if let Some(command) = parse_repl_command(user_input) {
            match command {
                CliReplCommand::Paste => {
                    writeln!(output, "paste mode; end with /send or /cancel")?;
                    let mut pasted = String::new();
                    let mut should_quit = false;
                    loop {
                        write!(output, "paste> ")?;
                        output.flush()?;

                        let Some(line) =
                            next_repl_line(&mut input_lines, &mut pending_lines).await?
                        else {
                            should_quit = true;
                            break;
                        };
                        let pasted_line = line.trim_end_matches(['\r', '\n']);
                        match pasted_line {
                            "/send" => {
                                if pasted.trim().is_empty() {
                                    writeln!(output, "paste is empty; nothing sent")?;
                                } else {
                                    run_repl_prompt_and_write(
                                        &data_dir,
                                        &config,
                                        &mut session,
                                        pasted,
                                        &mut output,
                                        &mut input_lines,
                                        &mut pending_lines,
                                    )
                                    .await?;
                                }
                                break;
                            }
                            "/cancel" => {
                                writeln!(output, "paste cancelled")?;
                                break;
                            }
                            _ => {
                                if !pasted.is_empty() {
                                    pasted.push('\n');
                                }
                                pasted.push_str(pasted_line);
                            }
                        }
                    }
                    if should_quit {
                        break;
                    }
                }
                CliReplCommand::ResumeTask(task_id) => {
                    if let Err(error) = resume_repl_task_and_write(
                        &data_dir,
                        &config,
                        &mut session,
                        &task_id,
                        &mut output,
                        &mut input_lines,
                        &mut pending_lines,
                    )
                    .await
                    {
                        writeln!(output, "error: {error}")?;
                    }
                }
                other => match session.handle_command_with_data_dir(&data_dir, &config, other) {
                    Ok(outcome) => {
                        for line in outcome.lines {
                            writeln!(output, "{line}")?;
                        }
                        if outcome.should_quit {
                            break;
                        }
                    }
                    Err(error) => {
                        writeln!(output, "error: {error}")?;
                    }
                },
            }
            continue;
        }

        run_repl_prompt_and_write(
            &data_dir,
            &config,
            &mut session,
            user_input.to_string(),
            &mut output,
            &mut input_lines,
            &mut pending_lines,
        )
        .await?;
    }

    Ok(session.snapshot)
}

async fn resume_repl_task_and_write<W>(
    data_dir: &Path,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    task_id: &str,
    output: &mut W,
    input_lines: &mut ReplLineReceiver,
    pending_lines: &mut VecDeque<String>,
) -> Result<()>
where
    W: Write,
{
    let checkpoint = resolve_resume_task_checkpoint(data_dir, config, task_id)?;
    if checkpoint.resume_mode != ResumeMode::FromTraceProjection {
        return Err(anyhow::anyhow!(
            "unsupported resume mode for task {}: {:?}",
            checkpoint.task_id,
            checkpoint.resume_mode
        ));
    }
    ensure_checkpoint_task_is_paused(data_dir, &checkpoint)?;
    let provider_id = checkpoint.provider_id.to_string();
    let resume_config = config_for_resume_checkpoint(config, &checkpoint)?;

    project_checkpoint_trace_into_session(data_dir, session, &checkpoint)?;
    session.snapshot.status.active_profile = provider_id;
    writeln!(
        output,
        "resuming task {} from trace {} via checkpoint {}",
        checkpoint.task_id, checkpoint.trace_id, checkpoint.checkpoint_id
    )?;
    let outcome = run_repl_prompt_and_write(
        data_dir,
        &resume_config,
        session,
        resume_task_prompt(&checkpoint.task_id),
        output,
        input_lines,
        pending_lines,
    )
    .await?;

    let mut resumer = RuntimeTaskResumer::new(TraceStore::open(data_dir)?);
    resumer.mark_task_resumed(
        &checkpoint,
        Some(format!("chat resume started in trace {}", outcome.trace_id)),
    )?;
    Ok(())
}

fn resolve_resume_task_checkpoint(
    data_dir: &Path,
    config: &TesseraConfig,
    selector: &str,
) -> Result<RuntimePauseCheckpointSummary> {
    if let Some(index) = parse_resume_task_index(selector)? {
        let checkpoints = list_resumable_pause_checkpoints(data_dir, config)?;
        if index == 0 {
            return Err(anyhow::anyhow!(
                "resume task index out of range: 0 (available tasks: {})",
                checkpoints.len()
            ));
        }
        return checkpoints.get(index - 1).cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "resume task index out of range: {index} (available tasks: {})",
                checkpoints.len()
            )
        });
    }

    load_pause_checkpoint_for_task(data_dir, selector)
}

fn parse_resume_task_index(selector: &str) -> Result<Option<usize>> {
    let candidate = selector.strip_prefix('#').unwrap_or(selector);
    if candidate.is_empty()
        || !candidate
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Ok(None);
    }
    Ok(Some(candidate.parse()?))
}

fn load_pause_checkpoint_for_task(
    data_dir: &Path,
    task_id: &str,
) -> Result<RuntimePauseCheckpointSummary> {
    let task_id = TaskId::from(task_id);
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    reader
        .find_pause_checkpoint(&task_id)?
        .ok_or_else(|| anyhow::anyhow!("pause checkpoint not found for task: {task_id}"))
}

fn ensure_checkpoint_task_is_paused(
    data_dir: &Path,
    checkpoint: &RuntimePauseCheckpointSummary,
) -> Result<()> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let status = reader
        .list_tasks(&checkpoint.trace_id)?
        .into_iter()
        .find(|task| task.task_id == checkpoint.task_id)
        .map(|task| task.status);
    match status {
        Some(TaskStatus::Paused) => Ok(()),
        Some(status) => Err(anyhow::anyhow!(
            "task {} is not paused (current status: {})",
            checkpoint.task_id,
            task_status_label(status)
        )),
        None => Err(anyhow::anyhow!(
            "task {} is not present in trace {}",
            checkpoint.task_id,
            checkpoint.trace_id
        )),
    }
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::WaitingForApproval => "waiting_for_approval",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
        TaskStatus::Paused => "paused",
    }
}

fn task_owner_kind_label(kind: TaskOwnerKind) -> &'static str {
    match kind {
        TaskOwnerKind::Execution => "execution",
        TaskOwnerKind::Observer => "observer",
    }
}

fn task_owner_status_label(status: TaskOwnerStatus) -> &'static str {
    match status {
        TaskOwnerStatus::Attached => "attached",
        TaskOwnerStatus::Heartbeat => "heartbeat",
        TaskOwnerStatus::Detached => "detached",
        TaskOwnerStatus::Lost => "lost",
    }
}

fn task_reattach_mode_label(mode: TaskReattachMode) -> &'static str {
    match mode {
        TaskReattachMode::ObserveExistingOwner => "observe_existing_owner",
        TaskReattachMode::ResumeFromCheckpoint => "resume_from_checkpoint",
        TaskReattachMode::TerminalProjection => "terminal_projection",
        TaskReattachMode::OwnerLost => "owner_lost",
    }
}

fn snake_json_label<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToString::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn instruction_report_from_trace(
    store: &TraceStore,
    trace_id: &str,
) -> (Vec<InstructionSource>, usize) {
    let sources = store
        .read_trace_records(trace_id)
        .ok()
        .and_then(|records| {
            records
                .into_iter()
                .find(|record| record.event_kind == "instructions_discovered")
                .and_then(|record| {
                    serde_json::from_value::<Vec<InstructionSource>>(
                        record.payload.get("sources")?.clone(),
                    )
                    .ok()
                })
        })
        .unwrap_or_default();
    let warning_count = sources
        .iter()
        .map(|source| source.warnings.len())
        .sum::<usize>();
    (sources, warning_count)
}

fn skill_report_from_trace(store: &TraceStore, trace_id: &str) -> (Vec<SkillActivation>, usize) {
    let activations = store
        .read_trace_records(trace_id)
        .ok()
        .map(|records| {
            records
                .into_iter()
                .filter(|record| record.event_kind == "skill_activated")
                .filter_map(|record| {
                    serde_json::from_value::<SkillActivation>(
                        record.payload.get("activation")?.clone(),
                    )
                    .ok()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let warning_count = activations
        .iter()
        .map(|activation| {
            activation.warnings.len()
                + activation.entrypoint.warnings.len()
                + activation
                    .references
                    .iter()
                    .map(|source| source.warnings.len())
                    .sum::<usize>()
                + activation
                    .steps
                    .iter()
                    .map(|step| step.warnings.len())
                    .sum::<usize>()
        })
        .sum::<usize>();
    (activations, warning_count)
}

fn project_checkpoint_trace_into_session(
    data_dir: &Path,
    session: &mut CliReplSession,
    checkpoint: &RuntimePauseCheckpointSummary,
) -> Result<()> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let page = reader.list_events(RuntimeEventQuery::new(checkpoint.trace_id.as_str()))?;
    let records = page
        .records
        .iter()
        .filter(|record| record.seq <= checkpoint.last_seq)
        .collect::<Vec<_>>();
    if records.is_empty() {
        return Err(anyhow::anyhow!(
            "pause checkpoint has no trace records to project: {}",
            checkpoint.checkpoint_id
        ));
    }

    session.snapshot.start_new_thread();
    for record in records {
        session.snapshot.apply_trace_record(record);
    }
    Ok(())
}

fn provider_history_for_checkpoint(
    data_dir: &Path,
    checkpoint: &RuntimePauseCheckpointSummary,
) -> Result<Vec<ProviderMessage>> {
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let page = reader.list_events(RuntimeEventQuery::new(checkpoint.trace_id.as_str()))?;
    let records = page
        .records
        .iter()
        .filter(|record| record.seq <= checkpoint.last_seq)
        .collect::<Vec<_>>();
    if records.is_empty() {
        return Err(anyhow::anyhow!(
            "pause checkpoint has no trace records to project: {}",
            checkpoint.checkpoint_id
        ));
    }

    let mut snapshot = ClientSnapshot::new("resume-task");
    for record in records {
        snapshot.apply_trace_record(record);
    }
    Ok(provider_history_from_snapshot(&snapshot))
}

fn config_for_resume_checkpoint(
    config: &TesseraConfig,
    checkpoint: &RuntimePauseCheckpointSummary,
) -> Result<TesseraConfig> {
    let provider_id = checkpoint.provider_id.to_string();
    let mut resume_config = config.clone();
    let profile = resume_config
        .providers
        .iter_mut()
        .find(|profile| profile.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("provider profile not found: {provider_id}"))?;
    profile.default_model = checkpoint.model.clone();
    Ok(resume_config)
}

fn resume_task_prompt(task_id: &TaskId) -> String {
    format!("Continue the paused task {task_id} from the saved trace projection.")
}

pub async fn resume_task_with_config<W>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    task_selector: &str,
    output: &mut W,
) -> Result<ConversationOutcome>
where
    W: Write,
{
    let data_dir = data_dir.as_ref();
    let checkpoint = resolve_resume_task_checkpoint(data_dir, config, task_selector)?;
    if checkpoint.resume_mode != ResumeMode::FromTraceProjection {
        return Err(anyhow::anyhow!(
            "unsupported resume mode for task {}: {:?}",
            checkpoint.task_id,
            checkpoint.resume_mode
        ));
    }
    ensure_checkpoint_task_is_paused(data_dir, &checkpoint)?;
    let provider_id = checkpoint.provider_id.to_string();
    let resume_config = config_for_resume_checkpoint(config, &checkpoint)?;
    let history = provider_history_for_checkpoint(data_dir, &checkpoint)?;

    writeln!(
        output,
        "resuming task {} from trace {} via checkpoint {}",
        checkpoint.task_id, checkpoint.trace_id, checkpoint.checkpoint_id
    )?;
    let outcome = run_chat_with_config_history_and_events(
        data_dir,
        &resume_config,
        &provider_id,
        resume_task_prompt(&checkpoint.task_id),
        history,
        |_| EventSinkAction::Continue,
    )
    .await?;
    writeln!(output, "{}", outcome.assistant_text)?;

    let mut resumer = RuntimeTaskResumer::new(TraceStore::open(data_dir)?);
    resumer.mark_task_resumed(
        &checkpoint,
        Some(format!("chat resume started in trace {}", outcome.trace_id)),
    )?;
    Ok(outcome)
}

async fn run_repl_prompt_and_write<W>(
    data_dir: &Path,
    config: &TesseraConfig,
    session: &mut CliReplSession,
    prompt: String,
    output: &mut W,
    input_lines: &mut ReplLineReceiver,
    pending_lines: &mut VecDeque<String>,
) -> Result<ConversationOutcome>
where
    W: Write,
{
    write!(output, "assistant> ")?;
    output.flush()?;

    let cancellation_token = RunCancellationToken::new();
    let pause_token = RunPauseToken::new();
    let controls = RunControls {
        event_timeout: None,
        cancellation_token: Some(cancellation_token.clone()),
        pause_token: Some(pause_token.clone()),
    };
    let (delta_tx, mut delta_rx) = mpsc::unbounded_channel::<String>();
    let run = run_repl_prompt_with_writer_and_controls(
        data_dir,
        config,
        session,
        prompt,
        controls,
        move |delta| {
            let _ = delta_tx.send(delta.to_string());
        },
    );
    tokio::pin!(run);
    let mut input_closed = false;
    let mut cancel_announced = false;
    let mut pause_announced = false;

    loop {
        tokio::select! {
            result = &mut run => {
                while let Ok(delta) = delta_rx.try_recv() {
                    write!(output, "{delta}")?;
                    output.flush()?;
                }
                let outcome = result?;
                writeln!(output)?;
                return Ok(outcome);
            }
            maybe_delta = delta_rx.recv() => {
                if let Some(delta) = maybe_delta {
                    write!(output, "{delta}")?;
                    output.flush()?;
                }
            }
            maybe_line = input_lines.recv(), if !input_closed => {
                match maybe_line {
                    Some(Ok(line)) => {
                        if pending_lines.is_empty() {
                            match parse_repl_command(line.trim()) {
                                Some(CliReplCommand::Cancel) => {
                                    cancellation_token.cancel("cli repl cancel requested");
                                    if !cancel_announced {
                                        writeln!(output, "\ncancel requested")?;
                                        output.flush()?;
                                        cancel_announced = true;
                                    }
                                }
                                Some(CliReplCommand::PauseTask(_)) => {
                                    pause_token.pause("cli repl pause requested");
                                    if !pause_announced {
                                        writeln!(output, "\npause requested")?;
                                        output.flush()?;
                                        pause_announced = true;
                                    }
                                }
                                _ => pending_lines.push_back(line),
                            }
                        } else {
                            pending_lines.push_back(line);
                        }
                    }
                    Some(Err(error)) => return Err(error.into()),
                    None => {
                        input_closed = true;
                    }
                }
            }
        }
    }
}

pub fn repl_startup_lines(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
) -> Vec<String> {
    let available_profiles = config
        .providers
        .iter()
        .map(|profile| profile.id.as_str())
        .collect::<Vec<_>>();

    vec![
        "Tessera CLI interactive chat".to_string(),
        format!("active_profile: {provider_id}"),
        format!("data_dir: {}", data_dir.as_ref().display()),
        format!(
            "available_profiles: {}",
            if available_profiles.is_empty() {
                "none".to_string()
            } else {
                available_profiles.join(", ")
            }
        ),
        "type /help or run `tessera chat --list-commands` for commands; use /doctor for runtime health, /quit to exit".to_string(),
    ]
}

pub fn chat_command_lines() -> Vec<&'static str> {
    vec![
        "commands:",
        "  /help, /commands   show this help",
        "  /new               start a fresh visible thread",
        "  /clear             clear the current visible thread",
        "  /cancel            cancel active paste/run when available",
        "  /pause [task_id]   pause active run when available; otherwise record metadata-only intent",
        "  /resume-task <task_id|#> resume a paused chat task by id or /resume-tasks number",
        "  /resume-tasks      list resumable paused tasks from trace checkpoints",
        "  /paste             enter multiline prompt mode",
        "  /profiles          list configured provider profiles",
        "  /profile <id>      switch active provider profile",
        "  /sessions          list trace-backed sessions",
        "  /resume <trace_id|#> project a trace into this session",
        "  /doctor            show runtime health for this session",
        "  /history           list current visible messages",
        "  /status            show compact runtime status",
        "  /export            print markdown transcript",
        "  /quit, /exit       leave the REPL",
    ]
}

pub fn default_config_template() -> &'static str {
    r#"# Tessera local configuration
# This template stores provider secret *environment variable names* only.
# Do not paste API keys or bearer tokens into this file.

data_dir = "./.tessera"

[[providers]]
id = "mock"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "ollama"
kind = "ollama"
default_model = "llama3"
base_url = "http://localhost:11434"

[[providers]]
id = "openai-compatible"
kind = "openai-compatible"
default_model = "deepseek-chat"
base_url = "https://api.example.com/v1"
api_key_env = "TESSERA_OPENAI_COMPATIBLE_API_KEY"
"#
}

pub fn write_config_template(path: impl AsRef<Path>, force: bool) -> Result<PathBuf> {
    let path = path.as_ref();
    if path.exists() && !force {
        return Err(anyhow::anyhow!(
            "config file already exists: {} (pass --force to overwrite)",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, default_config_template())?;
    Ok(path.to_path_buf())
}

pub fn run_doctor(data_dir: impl AsRef<Path>) -> Result<DoctorReport> {
    run_doctor_with_config(data_dir, &TesseraConfig::default_with_mock())
}

pub fn run_doctor_with_config(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
) -> Result<DoctorReport> {
    let data_dir = data_dir.as_ref();
    let store = TraceStore::open(data_dir)?;
    let traces_dir = data_dir.join("traces");
    fs::create_dir_all(&traces_dir)?;
    let probe = traces_dir.join(".write-probe");
    let trace_writable = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
        .and_then(|_| fs::remove_file(&probe))
        .is_ok();

    Ok(DoctorReport {
        status: if trace_writable && store.is_healthy() {
            "ok".to_string()
        } else {
            "error".to_string()
        },
        data_dir: data_dir.to_string_lossy().to_string(),
        trace_writable,
        sqlite_index_healthy: store.is_healthy(),
        provider_profiles: config
            .providers
            .iter()
            .map(|profile| profile.id.clone())
            .collect(),
    })
}

pub fn run_apply_patch_with_options(
    data_dir: impl AsRef<Path>,
    options: CliApplyPatchOptions,
) -> Result<CliApplyPatchOutput> {
    validate_apply_patch_options(&options)?;
    let envelope = build_explicit_apply_patch_envelope(options)?;
    run_apply_patch_envelope(data_dir, envelope)
}

pub fn run_apply_patch_from_trace_options(
    data_dir: impl AsRef<Path>,
    options: CliTraceApplyPatchOptions,
) -> Result<CliApplyPatchOutput> {
    validate_trace_apply_patch_options(&options)?;
    let data_dir = data_dir.as_ref();
    let ResolvedTraceApplyPatchInput {
        resolved,
        patch_body,
    } = resolve_trace_apply_patch_input(
        data_dir,
        &options.trace_id,
        trace_apply_patch_selector(
            options.workflow_id.clone(),
            options.request_id.clone(),
            options.patch_id.clone(),
            options.patch_artifact_id.clone(),
            options.allowed_paths.clone(),
        ),
        options.patch_body_override.as_deref(),
    )?;

    let envelope = build_trace_apply_patch_envelope(options, resolved, patch_body)?;
    run_apply_patch_envelope(data_dir, envelope)
}

pub fn run_apply_patch_auto_worktree_options(
    data_dir: impl AsRef<Path>,
    options: CliAutoWorktreeApplyPatchOptions,
) -> Result<CliApplyPatchOutput> {
    validate_auto_worktree_apply_patch_options(&options)?;
    let data_dir = data_dir.as_ref();
    let ResolvedTraceApplyPatchInput {
        resolved,
        patch_body,
    } = resolve_trace_apply_patch_input(
        data_dir,
        &options.trace_id,
        trace_apply_patch_selector(
            options.workflow_id.clone(),
            options.request_id.clone(),
            options.patch_id.clone(),
            options.patch_artifact_id.clone(),
            options.allowed_paths.clone(),
        ),
        options.patch_body_override.as_deref(),
    )?;

    if options.dry_run {
        let envelope = build_trace_apply_patch_envelope(
            CliTraceApplyPatchOptions {
                trace_id: options.trace_id,
                workflow_id: options.workflow_id,
                request_id: options.request_id,
                patch_id: options.patch_id,
                patch_artifact_id: options.patch_artifact_id,
                preflight_id: options.preflight_id,
                execution_id: options.execution_id,
                isolated_root: data_dir
                    .join("worktrees")
                    .join("dry-run-auto-worktree-not-created"),
                root_label: generated_cli_worktree_root_label(
                    &resolved.workflow_id,
                    &resolved.patch_proposal.patch_id,
                ),
                allowed_paths: options.allowed_paths,
                patch_body_override: None,
                dry_run: true,
                operator_label: options.operator_label,
            },
            resolved,
            patch_body,
        )?;
        return run_apply_patch_envelope(data_dir, envelope);
    }

    let source_root = resolve_auto_worktree_source_root(options.source_root.as_deref())?;
    let repo_key = repo_key_from_source_root(&source_root);
    let (worktree_base, worktree_base_key) =
        resolve_auto_worktree_base(data_dir, &repo_key, options.worktree_base.as_deref());
    fs::create_dir_all(&worktree_base)?;
    let existing_worktree_leaf_names = existing_worktree_leaf_names(&worktree_base)?;

    let lifecycle_runner = IsolatedWorktreeLifecycleRunner::default();
    let mut command_runner = GitWorktreeCommandRunner;
    let created = lifecycle_runner.create_detached_worktree(
        &mut command_runner,
        IsolatedWorktreeLifecycleRunRequest {
            workflow_id: resolved.workflow_id.clone(),
            task_id: resolved.task_id.clone(),
            trace_id: resolved.trace_id.clone(),
            request_id: resolved.mutation_request.request_id.clone(),
            patch_id: resolved.patch_proposal.patch_id.clone(),
            repo_key,
            data_dir: data_dir.to_path_buf(),
            worktree_base: Some(worktree_base),
            worktree_base_key: Some(worktree_base_key),
            requested_root_label: None,
            source_root,
            existing_worktree_leaf_names,
            retention_policy: options.retention_policy,
            reason: "trace-driven apply-patch requested an isolated worktree".to_string(),
            evidence: lifecycle_evidence_from_trace(&resolved),
        },
    )?;
    append_lifecycle_events(data_dir, &resolved.trace_id, &created.lifecycle_events)?;

    let envelope = build_trace_apply_patch_envelope(
        CliTraceApplyPatchOptions {
            trace_id: options.trace_id,
            workflow_id: options.workflow_id,
            request_id: options.request_id,
            patch_id: options.patch_id,
            patch_artifact_id: options.patch_artifact_id,
            preflight_id: options.preflight_id,
            execution_id: options.execution_id,
            isolated_root: created.plan.worktree_path.clone(),
            root_label: created.plan.worktree_root_label.clone(),
            allowed_paths: options.allowed_paths,
            patch_body_override: None,
            dry_run: false,
            operator_label: options.operator_label,
        },
        resolved,
        patch_body,
    )?;

    match run_apply_patch_envelope(data_dir, envelope) {
        Ok(output) => {
            let retained = lifecycle_runner.retained_lifecycle_event(&created);
            append_lifecycle_events(data_dir, &created.plan.trace_id, &[retained])?;
            Ok(output.with_auto_worktree(&created, WorkspaceWorktreeLifecycleStatus::Retained))
        }
        Err(error) => {
            if options.retention_policy == WorktreeRetentionPolicy::CleanupOnFailure {
                let cleanup =
                    lifecycle_runner.cleanup_created_worktree(&mut command_runner, &created);
                append_lifecycle_events(data_dir, &created.plan.trace_id, &[cleanup])?;
            } else {
                let retained = lifecycle_runner.retained_lifecycle_event(&created);
                append_lifecycle_events(data_dir, &created.plan.trace_id, &[retained])?;
            }
            Err(error)
        }
    }
}

pub fn run_worktree_cleanup_options(
    data_dir: impl AsRef<Path>,
    options: CliWorktreeCleanupOptions,
) -> Result<CliWorktreeCleanupOutput> {
    validate_worktree_cleanup_options(&options)?;
    let data_dir = data_dir.as_ref();
    let worktree_id = WorkspaceWorktreeId::from(options.worktree_id.clone());
    let source_root = resolve_auto_worktree_source_root(options.source_root.as_deref())?;
    let record = resolve_retained_worktree_record(data_dir, &options.trace_id, &worktree_id)?;
    validate_cleanup_worktree_path(&record, &source_root, &options.worktree_path)?;

    let created = reconstructed_cleanup_worktree(&record, source_root, options.worktree_path)?;
    if options.dry_run {
        return Ok(CliWorktreeCleanupOutput::from_created(
            &created,
            "dry_run_ready",
        ));
    }

    let lifecycle_runner = IsolatedWorktreeLifecycleRunner::default();
    let mut command_runner = GitWorktreeCommandRunner;
    let started = lifecycle_runner.cleanup_started_lifecycle_event(&created);
    append_lifecycle_events(data_dir, &record.trace_id, &[started])?;

    let terminal = lifecycle_runner.cleanup_trace_confirmed_worktree(&mut command_runner, &created);
    append_lifecycle_events(data_dir, &record.trace_id, std::slice::from_ref(&terminal))?;
    let (status, reason) = worktree_cleanup_event_status_and_reason(&terminal)?;
    if status != WorkspaceWorktreeLifecycleStatus::CleanupCompleted {
        anyhow::bail!("worktree cleanup failed: {reason}");
    }

    Ok(CliWorktreeCleanupOutput::from_created(
        &created,
        worktree_lifecycle_status_label(status),
    ))
}

pub fn run_worktree_list_options(
    data_dir: impl AsRef<Path>,
    options: CliWorktreeListOptions,
) -> Result<Vec<CliWorktreeListOutput>> {
    validate_worktree_list_options(&options)?;
    let data_dir = data_dir.as_ref();
    let created_worktree_ids_with_refs =
        worktree_lifecycle_created_ids_with_refs(data_dir, &options.trace_id)?;
    let reader = RuntimeReader::new(TraceStore::open(data_dir)?);
    let lifecycles = reader.list_worktree_lifecycles(&options.trace_id)?;
    Ok(lifecycles
        .iter()
        .map(|lifecycle| {
            CliWorktreeListOutput::from_lifecycle(
                lifecycle,
                created_worktree_ids_with_refs.contains(lifecycle.worktree_id.as_str()),
            )
        })
        .collect())
}

pub fn format_worktree_list_lines(lifecycles: &[CliWorktreeListOutput]) -> Vec<String> {
    if lifecycles.is_empty() {
        return vec!["worktree lifecycle records: none".to_string()];
    }

    lifecycles
        .iter()
        .map(|lifecycle| {
            format!(
                "worktree {} status={} root_label={} base_key={} source_commit={} trace_cleanup_candidate={} ({})",
                lifecycle.worktree_id,
                lifecycle.latest_status,
                lifecycle.worktree_root_label,
                lifecycle.worktree_base_key,
                lifecycle.source_commit,
                lifecycle.trace_cleanup_candidate,
                lifecycle.trace_cleanup_candidate_note,
            )
        })
        .collect()
}

struct ResolvedTraceApplyPatchInput {
    resolved: TraceApplyPatchResolvedEnvelope,
    patch_body: String,
}

fn trace_apply_patch_selector(
    workflow_id: Option<String>,
    request_id: Option<String>,
    patch_id: Option<String>,
    patch_artifact_id: Option<String>,
    allowed_paths: Vec<String>,
) -> TraceApplyPatchSelector {
    TraceApplyPatchSelector {
        workflow_id: workflow_id.map(CodingWorkflowId::from),
        request_id: request_id.map(MutationRequestId::from),
        patch_id: patch_id.map(PatchProposalId::from),
        patch_artifact_id: patch_artifact_id.map(ArtifactId::from),
        allowed_paths,
    }
}

fn resolve_trace_apply_patch_input(
    data_dir: &Path,
    trace_id: &str,
    selector: TraceApplyPatchSelector,
    patch_body_override: Option<&str>,
) -> Result<ResolvedTraceApplyPatchInput> {
    let store = TraceStore::open(data_dir)?;
    let records = store.read_trace_records(trace_id)?;
    let resolved = TraceApplyPatchResolver::resolve(TraceApplyPatchResolveRequest {
        trace_id: trace_id.to_string(),
        records,
        selector,
    })?;
    let patch_body = match patch_body_override {
        Some(body) => body.to_string(),
        None => {
            let artifact_id = resolved
                .patch_artifact_id
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("trace apply-patch requires a patch artifact"))?;
            let stored = store.read_artifact_body(artifact_id)?;
            validate_trace_patch_artifact_body(&stored.record)?;
            String::from_utf8(stored.body)?
        }
    };
    if patch_body.len() > APPLY_PATCH_MAX_BODY_BYTES {
        anyhow::bail!("patch artifact body is too large");
    }

    Ok(ResolvedTraceApplyPatchInput {
        resolved,
        patch_body,
    })
}

fn resolve_retained_worktree_record(
    data_dir: &Path,
    trace_id: &str,
    worktree_id: &WorkspaceWorktreeId,
) -> Result<WorkspaceWorktreeLifecycleRecord> {
    let store = TraceStore::open(data_dir)?;
    let records = store.read_trace_records(trace_id)?;
    let mut saw_created = false;
    let mut saw_created_with_refs = false;
    let mut saw_retained = false;
    let mut latest = None;

    for trace_record in records {
        if trace_record.event_kind != "workspace_worktree_lifecycle_recorded" {
            continue;
        }
        let record: WorkspaceWorktreeLifecycleRecord =
            serde_json::from_value(trace_record.payload["record"].clone())?;
        if &record.worktree_id != worktree_id {
            continue;
        }
        if record.lifecycle_status == WorkspaceWorktreeLifecycleStatus::Created {
            saw_created = true;
            if record.created_for_request_id.is_some() && record.created_for_patch_id.is_some() {
                saw_created_with_refs = true;
            }
        }
        if record.lifecycle_status == WorkspaceWorktreeLifecycleStatus::Retained {
            saw_retained = true;
        }
        latest = Some(record);
    }

    let Some(record) = latest else {
        anyhow::bail!("no matching worktree lifecycle records found");
    };
    if !saw_created {
        anyhow::bail!("worktree lifecycle was never created");
    }
    if !saw_created_with_refs {
        anyhow::bail!("created worktree lifecycle record is missing request or patch refs");
    }
    match record.lifecycle_status {
        WorkspaceWorktreeLifecycleStatus::Retained => {}
        WorkspaceWorktreeLifecycleStatus::CleanupCompleted => {
            anyhow::bail!("worktree lifecycle was already cleaned");
        }
        WorkspaceWorktreeLifecycleStatus::CleanupFailed => {
            anyhow::bail!("worktree lifecycle latest cleanup failed");
        }
        _ => {
            anyhow::bail!("worktree lifecycle is not retained");
        }
    }
    if !saw_retained {
        anyhow::bail!("worktree lifecycle is not retained");
    }
    if !record.worktree_root_label.starts_with("worktree:") {
        anyhow::bail!("retained worktree lifecycle is not generated");
    }
    Ok(record)
}

fn worktree_lifecycle_created_ids_with_refs(
    data_dir: &Path,
    trace_id: &str,
) -> Result<HashSet<String>> {
    let store = TraceStore::open(data_dir)?;
    let records = store.read_trace_records(trace_id)?;
    let mut ids = HashSet::new();

    for trace_record in records {
        if trace_record.event_kind != "workspace_worktree_lifecycle_recorded" {
            continue;
        }
        let record: WorkspaceWorktreeLifecycleRecord =
            serde_json::from_value(trace_record.payload["record"].clone())?;
        if record.lifecycle_status == WorkspaceWorktreeLifecycleStatus::Created
            && record.created_for_request_id.is_some()
            && record.created_for_patch_id.is_some()
        {
            ids.insert(record.worktree_id.to_string());
        }
    }

    Ok(ids)
}

fn resolve_auto_worktree_source_root(source_root: Option<&Path>) -> Result<PathBuf> {
    if let Some(source_root) = source_root {
        return Ok(source_root.to_path_buf());
    }
    let current_dir = std::env::current_dir()?;
    find_git_root(&current_dir)
        .ok_or_else(|| anyhow::anyhow!("--source-root is required outside a Git checkout"))
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn validate_cleanup_worktree_path(
    record: &WorkspaceWorktreeLifecycleRecord,
    source_root: &Path,
    worktree_path: &Path,
) -> Result<()> {
    if !worktree_path.exists() {
        anyhow::bail!("worktree path does not exist");
    }
    if !worktree_path.is_dir() {
        anyhow::bail!("worktree path must be a directory");
    }
    if let (Ok(source_root), Ok(worktree_path)) =
        (source_root.canonicalize(), worktree_path.canonicalize())
    {
        if source_root == worktree_path {
            anyhow::bail!("worktree path must not target source root");
        }
    }

    let actual_leaf = worktree_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("worktree path must include a leaf name"))?;
    let expected_leaf = expected_worktree_leaf_name(record);
    if actual_leaf != expected_leaf {
        anyhow::bail!("worktree path does not match lifecycle metadata");
    }
    Ok(())
}

fn expected_worktree_leaf_name(record: &WorkspaceWorktreeLifecycleRecord) -> String {
    let base = record
        .worktree_root_label
        .strip_prefix("worktree:")
        .unwrap_or(&record.worktree_root_label);
    let source = record.source_commit.chars().take(12).collect::<String>();
    bounded_cli_leaf_name(&format!("{base}-{source}"), 80)
}

fn bounded_cli_leaf_name(value: &str, limit: usize) -> String {
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

fn reconstructed_cleanup_worktree(
    record: &WorkspaceWorktreeLifecycleRecord,
    source_root: PathBuf,
    worktree_path: PathBuf,
) -> Result<IsolatedWorktreeCreated> {
    let created_for_request_id = record
        .created_for_request_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("worktree lifecycle record is missing request id"))?;
    let created_for_patch_id = record
        .created_for_patch_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("worktree lifecycle record is missing patch id"))?;
    let worktree_base = worktree_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("worktree path must have a parent directory"))?
        .to_path_buf();
    let worktree_leaf_name = worktree_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("worktree path must include a leaf name"))?
        .to_string();

    Ok(IsolatedWorktreeCreated {
        plan: IsolatedWorktreePlan {
            worktree_id: record.worktree_id.clone(),
            workflow_id: record.workflow_id.clone(),
            task_id: record.task_id.clone(),
            trace_id: record.trace_id.clone(),
            source_commit: record.source_commit.clone(),
            source_branch_label: record.source_branch_label.clone(),
            worktree_root_label: record.worktree_root_label.clone(),
            worktree_leaf_name,
            worktree_base,
            worktree_path,
            worktree_base_key: record.worktree_base_key.clone(),
            created_for_request_id,
            created_for_patch_id,
            retention_policy: WorktreeRetentionPolicy::RetainOnSuccess,
            source_has_untracked_changes: false,
            reason: "trace-confirmed worktree cleanup requested".to_string(),
            evidence: record.evidence.clone(),
        },
        source_root,
        lifecycle_events: Vec::new(),
        created_by_current_invocation: false,
    })
}

fn worktree_cleanup_event_status_and_reason(
    event: &RunEvent,
) -> Result<(WorkspaceWorktreeLifecycleStatus, String)> {
    match event {
        RunEvent::WorkspaceWorktreeLifecycleRecorded { record } => {
            Ok((record.lifecycle_status, record.reason.clone()))
        }
        _ => anyhow::bail!("unexpected worktree cleanup event"),
    }
}

fn repo_key_from_source_root(source_root: &Path) -> String {
    let label = source_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    format!("{}-auto", sanitize_cli_key_fragment(label))
}

fn resolve_auto_worktree_base(
    data_dir: &Path,
    repo_key: &str,
    worktree_base: Option<&Path>,
) -> (PathBuf, String) {
    match worktree_base {
        Some(base) => {
            let base_key = base
                .file_name()
                .and_then(|name| name.to_str())
                .map(sanitize_cli_key_fragment)
                .unwrap_or_else(|| "custom-worktree-base".to_string());
            (base.to_path_buf(), format!("cli:{base_key}"))
        }
        None => (
            data_dir.join("worktrees").join(repo_key),
            format!("data_dir:worktrees/{repo_key}"),
        ),
    }
}

fn existing_worktree_leaf_names(worktree_base: &Path) -> Result<Vec<String>> {
    if !worktree_base.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(worktree_base)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    Ok(names)
}

fn generated_cli_worktree_root_label(
    workflow_id: &CodingWorkflowId,
    patch_id: &PatchProposalId,
) -> String {
    format!(
        "worktree:{}-{}",
        sanitize_cli_key_fragment(strip_cli_known_prefix(
            workflow_id.as_str(),
            "coding_workflow_"
        )),
        sanitize_cli_key_fragment(strip_cli_known_prefix(patch_id.as_str(), "patch_proposal_"))
    )
}

fn strip_cli_known_prefix<'a>(value: &'a str, prefix: &str) -> &'a str {
    value.strip_prefix(prefix).unwrap_or(value)
}

fn sanitize_cli_key_fragment(value: &str) -> String {
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
    let output = output.trim_matches('-');
    if output.is_empty() {
        "item".to_string()
    } else {
        output.chars().take(32).collect()
    }
}

fn lifecycle_evidence_from_trace(
    resolved: &TraceApplyPatchResolvedEnvelope,
) -> Vec<HandoffEvidenceRef> {
    let mut evidence = resolved.mutation_request.evidence.clone();
    evidence.extend(resolved.patch_proposal.diff_artifacts.clone());
    evidence.truncate(8);
    evidence
}

fn append_lifecycle_events(data_dir: &Path, trace_id: &str, events: &[RunEvent]) -> Result<()> {
    let mut store = TraceStore::open(data_dir)?;
    for event in events {
        let seq = next_trace_seq(&store, trace_id)?;
        store.append(&EventFrame::new(trace_id.to_string(), seq, event.clone()))?;
    }
    Ok(())
}

pub fn run_apply_patch_envelope(
    data_dir: impl AsRef<Path>,
    envelope: CliApplyPatchEnvelope,
) -> Result<CliApplyPatchOutput> {
    let gate_request = build_apply_patch_gate_request(&envelope);
    let gate_record = ApplyPatchGate.evaluate(gate_request);
    let preflight = apply_patch_preflight_record_from_gate(&envelope, &gate_record);

    let mut store = TraceStore::open(data_dir)?;
    let preflight_seq = next_trace_seq(&store, &envelope.trace_id)?;
    store.append(&EventFrame::new(
        envelope.trace_id.clone(),
        preflight_seq,
        RunEvent::ApplyPatchPreflightRecorded {
            record: preflight.clone(),
        },
    ))?;

    if envelope.dry_run {
        if gate_record.status != ApplyPatchGateStatus::PreflightReady {
            anyhow::bail!(
                "apply-patch dry-run blocked: {}",
                format_apply_patch_gate_blockers(&gate_record.blockers).join(", ")
            );
        }
        return Ok(CliApplyPatchOutput::from_records(
            envelope.trace_id,
            envelope.root_label,
            &preflight,
            None,
        ));
    }

    if gate_record.status != ApplyPatchGateStatus::ExecutorReady {
        anyhow::bail!(
            "apply-patch execution blocked: {}",
            format_apply_patch_gate_blockers(&gate_record.blockers).join(", ")
        );
    }

    let execution = ApplyPatchExecutor
        .apply_to_isolated_root(ApplyPatchIsolatedFileRequest {
            execution_id: envelope.execution_id.clone(),
            preflight_id: envelope.preflight_id.clone(),
            workflow_id: envelope.workflow_id.clone(),
            task_id: envelope.task_id.clone(),
            request_id: envelope.request_id.clone(),
            patch_id: envelope.patch_id.clone(),
            checkpoint_id: Some(envelope.checkpoint_lifecycle.checkpoint_id.clone()),
            reviewer_gate_id: Some(envelope.reviewer_decision.gate_id.clone()),
            policy_decision_id: Some(envelope.policy_decision.decision_id.clone()),
            sandbox_profile_label: envelope.sandbox_profile_label.clone(),
            isolated_root: envelope.isolated_root.clone(),
            isolated_root_label: envelope.root_label.clone(),
            root_kind: ApplyPatchExecutorRootKind::IsolatedWorktree,
            executor_label: APPLY_PATCH_EXECUTOR_LABEL.to_string(),
            allowed_paths: envelope.allowed_paths.clone(),
            patch_body: envelope.patch_body.clone(),
        })
        .record;

    let execution_seq = next_trace_seq(&store, &envelope.trace_id)?;
    store.append(&EventFrame::new(
        envelope.trace_id.clone(),
        execution_seq,
        RunEvent::ApplyPatchExecutionRecorded {
            record: execution.clone(),
        },
    ))?;

    if execution.status != ApplyPatchExecutionStatus::Applied {
        anyhow::bail!(
            "apply-patch execution did not apply: {}",
            format_apply_patch_execution_blockers(&execution.blockers).join(", ")
        );
    }

    Ok(CliApplyPatchOutput::from_records(
        envelope.trace_id,
        envelope.root_label,
        &preflight,
        Some(&execution),
    ))
}

fn validate_apply_patch_options(options: &CliApplyPatchOptions) -> Result<()> {
    if options.trace_id.trim().is_empty() {
        anyhow::bail!("--trace-id must not be empty");
    }
    if options.allowed_paths.is_empty() {
        anyhow::bail!("--allowed-path is required");
    }
    if options.sandbox_profile_label.trim().is_empty() {
        anyhow::bail!("--sandbox-profile is required");
    }
    for path in &options.allowed_paths {
        validate_cli_relative_path(path)?;
    }
    Ok(())
}

fn validate_trace_apply_patch_options(options: &CliTraceApplyPatchOptions) -> Result<()> {
    if options.trace_id.trim().is_empty() {
        anyhow::bail!("--from-trace must not be empty");
    }
    if options.root_label.trim().is_empty() {
        anyhow::bail!("--root-label must not be empty");
    }
    for path in &options.allowed_paths {
        validate_cli_relative_path(path)?;
    }
    Ok(())
}

fn validate_auto_worktree_apply_patch_options(
    options: &CliAutoWorktreeApplyPatchOptions,
) -> Result<()> {
    if options.trace_id.trim().is_empty() {
        anyhow::bail!("--from-trace must not be empty");
    }
    for path in &options.allowed_paths {
        validate_cli_relative_path(path)?;
    }
    Ok(())
}

fn validate_worktree_cleanup_options(options: &CliWorktreeCleanupOptions) -> Result<()> {
    if options.trace_id.trim().is_empty() {
        anyhow::bail!("--from-trace must not be empty");
    }
    if options.worktree_id.trim().is_empty() {
        anyhow::bail!("--worktree-id must not be empty");
    }
    if options.worktree_path.as_os_str().is_empty() {
        anyhow::bail!("--worktree-path must not be empty");
    }
    Ok(())
}

fn validate_worktree_list_options(options: &CliWorktreeListOptions) -> Result<()> {
    if options.trace_id.trim().is_empty() {
        anyhow::bail!("--trace must not be empty");
    }
    Ok(())
}

pub fn parse_worktree_retention_policy(value: Option<&str>) -> Result<WorktreeRetentionPolicy> {
    match value.unwrap_or("retain-on-success") {
        "retain-on-success" => Ok(WorktreeRetentionPolicy::RetainOnSuccess),
        "retain-always" => Ok(WorktreeRetentionPolicy::RetainAlways),
        "cleanup-on-failure" => Ok(WorktreeRetentionPolicy::CleanupOnFailure),
        other => anyhow::bail!(
            "invalid --worktree-retention `{other}`; expected retain-on-success, retain-always, or cleanup-on-failure"
        ),
    }
}

fn validate_trace_patch_artifact_body(record: &tessera_protocol::ArtifactBodyRecord) -> Result<()> {
    if record.kind != ArtifactKind::Patch {
        anyhow::bail!("patch artifact is not a patch");
    }
    if record.redaction_status != ArtifactBodyRedactionStatus::Clean {
        anyhow::bail!("patch artifact is not clean");
    }
    if record.byte_len == 0 {
        anyhow::bail!("patch artifact body is empty");
    }
    if record.byte_len > APPLY_PATCH_MAX_BODY_BYTES as u64 {
        anyhow::bail!("patch artifact body is too large");
    }
    Ok(())
}

fn validate_cli_relative_path(path: &str) -> Result<()> {
    let path = path.trim();
    if path.is_empty() || Path::new(path).is_absolute() {
        anyhow::bail!("--allowed-path must be a relative workspace path");
    }
    if Path::new(path).components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        anyhow::bail!("--allowed-path must not traverse outside the workspace");
    }
    Ok(())
}

fn next_trace_seq(store: &TraceStore, trace_id: &str) -> Result<u64> {
    if !store.list_trace_ids()?.iter().any(|id| id == trace_id) {
        return Ok(1);
    }
    Ok(store
        .read_trace_records(trace_id)?
        .iter()
        .map(|record| record.seq)
        .max()
        .unwrap_or(0)
        + 1)
}

fn build_explicit_apply_patch_envelope(
    options: CliApplyPatchOptions,
) -> Result<CliApplyPatchEnvelope> {
    let workflow_id = CodingWorkflowId::from(options.workflow_id.clone());
    let task_id = TaskId::from(options.task_id.clone());
    let request_id = MutationRequestId::from(options.request_id.clone());
    let patch_id = PatchProposalId::from(options.patch_id.clone());
    let checkpoint_id = SnapshotId::from(options.checkpoint_id.clone());
    let reviewer_gate_id = ReviewerGateId::from(options.reviewer_gate_id.clone());
    let policy_decision_id = PolicyDecisionId::from(options.policy_decision_id.clone());

    let mutation_request = MutationRequestProposal {
        request_id: request_id.clone(),
        workflow_id: workflow_id.clone(),
        task_id: task_id.clone(),
        operation: MutationRequestOperationKind::PatchApplication,
        status: MutationRequestStatus::Approved,
        summary: "explicit CLI apply-patch request".to_string(),
        requested_paths: options.allowed_paths.clone(),
        required_checkpoint_id: Some(checkpoint_id.clone()),
        reviewer_gate_id: Some(reviewer_gate_id.clone()),
        policy_decision_id: Some(policy_decision_id.clone()),
        sandbox_profile_label: Some(options.sandbox_profile_label.clone()),
        worktree_required: true,
        evidence: Vec::<HandoffEvidenceRef>::new(),
    };
    let patch_proposal = PatchProposal {
        patch_id: patch_id.clone(),
        workflow_id: workflow_id.clone(),
        task_id: task_id.clone(),
        summary: "explicit CLI apply-patch patch body".to_string(),
        touched_paths: options.allowed_paths.clone(),
        diff_artifacts: Vec::new(),
        risk_labels: vec!["explicit_cli_apply_patch".to_string()],
        required_checkpoint_id: Some(checkpoint_id.clone()),
        reviewer_gate_id: Some(reviewer_gate_id.clone()),
    };
    let enforcement_plan = MutationEnforcementPlanner::new(
        options.isolated_root.display().to_string(),
    )
    .plan(MutationEnforcementPlanRequest {
        workflow_id: workflow_id.clone(),
        task_id: task_id.clone(),
        requested_paths: options.allowed_paths.clone(),
        mutation_mode: Some(MutationMode::WorktreeFirst),
        policy_decision_id: Some(policy_decision_id.clone()),
        reason: "explicit CLI apply-patch".to_string(),
    })?;
    let mutation_scope = WorkspaceMutationScope {
        root_label: options.root_label.clone(),
        allowed_paths: options.allowed_paths.clone(),
        ..enforcement_plan.scope.clone()
    };
    let checkpoint_lifecycle = WorkspaceCheckpointLifecycleRecord {
        checkpoint_id,
        task_id: task_id.clone(),
        status: WorkspaceCheckpointLifecycleStatus::Created,
        reason: "explicit CLI checkpoint reference".to_string(),
        restore_plan_id: None,
        execution_blocked: true,
        evidence: Vec::new(),
        metadata: None,
    };
    let reviewer_decision = ReviewerGateDecision {
        gate_id: reviewer_gate_id,
        handoff_id: AgentHandoffId::from_static("handoff_cli_apply_patch"),
        decision: ReviewerDecisionKind::Accept,
        reviewer: options.operator_label.clone(),
        reason_code: "explicit_cli_accept".to_string(),
        comment: Some("operator supplied reviewer gate reference".to_string()),
    };
    let policy_decision = ToolPolicyDecision {
        decision_id: policy_decision_id,
        call_id: ToolCallId::from_static("tool_call_cli_apply_patch"),
        tool_id: ToolId::from_static("tool_cli_apply_patch"),
        outcome: PolicyOutcome::Allow,
        reason: "operator supplied policy decision reference".to_string(),
        required_permissions: vec![ToolPermission::FilesystemWrite],
        side_effects: vec![ToolSideEffect::WritesWorkspace],
        approval_id: None,
    };
    let enforcement_plan = MutationEnforcementPlan {
        sandbox_profile_label: Some(options.sandbox_profile_label.clone()),
        ..enforcement_plan
    };

    Ok(CliApplyPatchEnvelope {
        trace_id: options.trace_id,
        workflow_id,
        task_id,
        request_id,
        patch_id,
        preflight_id: ApplyPatchPreflightId::from(options.preflight_id),
        execution_id: ApplyPatchExecutionId::from(options.execution_id),
        mutation_request,
        patch_proposal,
        mutation_scope,
        enforcement_plan,
        checkpoint_lifecycle,
        reviewer_decision,
        policy_decision,
        sandbox_profile_label: Some(options.sandbox_profile_label),
        isolated_root: options.isolated_root,
        root_label: options.root_label,
        allowed_paths: options.allowed_paths,
        patch_body: options.patch_body,
        dry_run: options.dry_run,
        operator_label: options.operator_label,
    })
}

fn build_trace_apply_patch_envelope(
    options: CliTraceApplyPatchOptions,
    resolved: tessera_core::TraceApplyPatchResolvedEnvelope,
    patch_body: String,
) -> Result<CliApplyPatchEnvelope> {
    let allowed_paths = resolved.mutation_scope.allowed_paths.clone();
    let enforcement_plan = MutationEnforcementPlanner::new(
        options.isolated_root.display().to_string(),
    )
    .plan(MutationEnforcementPlanRequest {
        workflow_id: resolved.workflow_id.clone(),
        task_id: resolved.task_id.clone(),
        requested_paths: allowed_paths.clone(),
        mutation_mode: Some(resolved.mutation_scope.mutation_mode),
        policy_decision_id: Some(resolved.policy_decision.decision_id.clone()),
        reason: "trace-driven CLI apply-patch".to_string(),
    })?;
    let mutation_scope = WorkspaceMutationScope {
        root_label: options.root_label.clone(),
        allowed_paths: allowed_paths.clone(),
        ..resolved.mutation_scope
    };
    let enforcement_plan = MutationEnforcementPlan {
        scope: mutation_scope.clone(),
        sandbox_profile_label: resolved.sandbox_profile_label.clone(),
        policy_decision_id: Some(resolved.policy_decision.decision_id.clone()),
        ..enforcement_plan
    };

    Ok(CliApplyPatchEnvelope {
        trace_id: resolved.trace_id,
        workflow_id: resolved.workflow_id,
        task_id: resolved.task_id.clone(),
        request_id: resolved.mutation_request.request_id.clone(),
        patch_id: resolved.patch_proposal.patch_id.clone(),
        preflight_id: options
            .preflight_id
            .map(ApplyPatchPreflightId::from)
            .unwrap_or_default(),
        execution_id: options
            .execution_id
            .map(ApplyPatchExecutionId::from)
            .unwrap_or_default(),
        mutation_request: resolved.mutation_request,
        patch_proposal: resolved.patch_proposal,
        mutation_scope,
        enforcement_plan,
        checkpoint_lifecycle: resolved.checkpoint_lifecycle,
        reviewer_decision: resolved.reviewer_decision,
        policy_decision: resolved.policy_decision,
        sandbox_profile_label: resolved.sandbox_profile_label,
        isolated_root: options.isolated_root,
        root_label: options.root_label,
        allowed_paths,
        patch_body,
        dry_run: options.dry_run,
        operator_label: options.operator_label,
    })
}

fn build_apply_patch_gate_request(envelope: &CliApplyPatchEnvelope) -> ApplyPatchGateRequest {
    let patch_body = (!envelope.patch_body.trim().is_empty()).then(|| ApplyPatchDryRunInput {
        body: envelope.patch_body.clone(),
        max_bytes: APPLY_PATCH_MAX_BODY_BYTES,
    });

    ApplyPatchGateRequest {
        workflow_id: envelope.workflow_id.clone(),
        task_id: envelope.task_id.clone(),
        mutation_request: envelope.mutation_request.clone(),
        patch_proposal: envelope.patch_proposal.clone(),
        mutation_scope: envelope.mutation_scope.clone(),
        enforcement_plan: envelope.enforcement_plan.clone(),
        checkpoint_lifecycle: Some(envelope.checkpoint_lifecycle.clone()),
        reviewer_decision: Some(envelope.reviewer_decision.clone()),
        policy_decision: Some(envelope.policy_decision.clone()),
        patch_body,
        executor_context: (!envelope.dry_run).then(|| ApplyPatchExecutorContext {
            isolated_root_label: envelope.root_label.clone(),
            root_kind: ApplyPatchExecutorRootKind::IsolatedWorktree,
            executor_label: APPLY_PATCH_EXECUTOR_LABEL.to_string(),
            executor_available: true,
            request_source_label: APPLY_PATCH_REQUEST_SOURCE_LABEL.to_string(),
        }),
        operator_label: envelope.operator_label.clone(),
    }
}

fn apply_patch_preflight_record_from_gate(
    envelope: &CliApplyPatchEnvelope,
    gate: &ApplyPatchGateRecord,
) -> ApplyPatchPreflightRecord {
    ApplyPatchPreflightRecord {
        preflight_id: envelope.preflight_id.clone(),
        workflow_id: envelope.workflow_id.clone(),
        task_id: envelope.task_id.clone(),
        request_id: envelope.request_id.clone(),
        patch_id: envelope.patch_id.clone(),
        status: match gate.status {
            ApplyPatchGateStatus::Blocked => ApplyPatchPreflightStatus::Blocked,
            ApplyPatchGateStatus::PreflightReady => ApplyPatchPreflightStatus::DryRunReady,
            ApplyPatchGateStatus::ExecutorReady => ApplyPatchPreflightStatus::ExecutorReady,
        },
        blockers: gate
            .blockers
            .iter()
            .map(map_apply_patch_gate_blocker)
            .collect(),
        affected_paths: gate.affected_paths.clone(),
        operations: gate
            .dry_run
            .as_ref()
            .map(|dry_run| {
                dry_run
                    .operations
                    .iter()
                    .map(|operation| ApplyPatchDryRunOperationSummary {
                        path: operation.path.clone(),
                        operation: match operation.operation {
                            ApplyPatchDryRunOperation::Create => {
                                ApplyPatchDryRunOperationKind::Create
                            }
                            ApplyPatchDryRunOperation::Modify => {
                                ApplyPatchDryRunOperationKind::Modify
                            }
                            ApplyPatchDryRunOperation::DeleteUnsupported => {
                                ApplyPatchDryRunOperationKind::DeleteUnsupported
                            }
                            ApplyPatchDryRunOperation::RenameUnsupported => {
                                ApplyPatchDryRunOperationKind::RenameUnsupported
                            }
                            ApplyPatchDryRunOperation::BinaryUnsupported => {
                                ApplyPatchDryRunOperationKind::BinaryUnsupported
                            }
                        },
                    })
                    .collect()
            })
            .unwrap_or_default(),
        executor_blocked: gate.executor_blocked,
        executor_block_reason: gate.executor_block_reason.clone(),
        evidence: Vec::new(),
    }
}

fn map_apply_patch_gate_blocker(blocker: &ApplyPatchGateBlocker) -> ApplyPatchPreflightBlocker {
    match blocker {
        ApplyPatchGateBlocker::WorkflowMismatch
        | ApplyPatchGateBlocker::TaskMismatch
        | ApplyPatchGateBlocker::OperationNotPatchApplication => {
            ApplyPatchPreflightBlocker::OperationMismatch
        }
        ApplyPatchGateBlocker::MutationRequestNotApproved => {
            ApplyPatchPreflightBlocker::MutationRequestNotApproved
        }
        ApplyPatchGateBlocker::ScopeMismatch => ApplyPatchPreflightBlocker::ScopeMismatch,
        ApplyPatchGateBlocker::WorktreeIsolationRequired
        | ApplyPatchGateBlocker::MissingExecutorContext
        | ApplyPatchGateBlocker::PrimaryRootRejected => {
            ApplyPatchPreflightBlocker::WorktreeIsolationRequired
        }
        ApplyPatchGateBlocker::MissingPolicyDecision => {
            ApplyPatchPreflightBlocker::MissingPolicyDecision
        }
        ApplyPatchGateBlocker::PolicyNotAllowed => ApplyPatchPreflightBlocker::PolicyNotAllowed,
        ApplyPatchGateBlocker::MissingReviewerDecision => {
            ApplyPatchPreflightBlocker::MissingReviewerDecision
        }
        ApplyPatchGateBlocker::ReviewerNotAccepted => {
            ApplyPatchPreflightBlocker::ReviewerNotAccepted
        }
        ApplyPatchGateBlocker::MissingCheckpointLifecycle => {
            ApplyPatchPreflightBlocker::MissingCheckpointLifecycle
        }
        ApplyPatchGateBlocker::CheckpointNotCreated => {
            ApplyPatchPreflightBlocker::CheckpointNotCreated
        }
        ApplyPatchGateBlocker::MissingSandboxProfile => {
            ApplyPatchPreflightBlocker::MissingSandboxProfile
        }
        ApplyPatchGateBlocker::MissingPatchBody => ApplyPatchPreflightBlocker::MissingPatchBody,
        ApplyPatchGateBlocker::PatchBodyTooLarge => ApplyPatchPreflightBlocker::PatchBodyTooLarge,
        ApplyPatchGateBlocker::UnsafePatchPath => ApplyPatchPreflightBlocker::UnsafePatchPath,
        ApplyPatchGateBlocker::UnsupportedPatchOperation => {
            ApplyPatchPreflightBlocker::UnsupportedPatchOperation
        }
        ApplyPatchGateBlocker::ExecutorUnavailable => {
            ApplyPatchPreflightBlocker::ExecutorUnavailable
        }
    }
}

fn format_apply_patch_gate_blockers(blockers: &[ApplyPatchGateBlocker]) -> Vec<String> {
    blockers
        .iter()
        .map(apply_patch_gate_blocker_label)
        .collect()
}

fn apply_patch_gate_blocker_label(blocker: &ApplyPatchGateBlocker) -> String {
    match blocker {
        ApplyPatchGateBlocker::WorkflowMismatch => "workflow_mismatch",
        ApplyPatchGateBlocker::TaskMismatch => "task_mismatch",
        ApplyPatchGateBlocker::OperationNotPatchApplication => "operation_not_patch_application",
        ApplyPatchGateBlocker::MutationRequestNotApproved => "mutation_request_not_approved",
        ApplyPatchGateBlocker::ScopeMismatch => "scope_mismatch",
        ApplyPatchGateBlocker::WorktreeIsolationRequired => "worktree_isolation_required",
        ApplyPatchGateBlocker::MissingPolicyDecision => "missing_policy_decision",
        ApplyPatchGateBlocker::PolicyNotAllowed => "policy_not_allowed",
        ApplyPatchGateBlocker::MissingReviewerDecision => "missing_reviewer_decision",
        ApplyPatchGateBlocker::ReviewerNotAccepted => "reviewer_not_accepted",
        ApplyPatchGateBlocker::MissingCheckpointLifecycle => "missing_checkpoint_lifecycle",
        ApplyPatchGateBlocker::CheckpointNotCreated => "checkpoint_not_created",
        ApplyPatchGateBlocker::MissingSandboxProfile => "missing_sandbox_profile",
        ApplyPatchGateBlocker::MissingPatchBody => "missing_patch_body",
        ApplyPatchGateBlocker::PatchBodyTooLarge => "patch_body_too_large",
        ApplyPatchGateBlocker::UnsafePatchPath => "unsafe_patch_path",
        ApplyPatchGateBlocker::UnsupportedPatchOperation => "unsupported_patch_operation",
        ApplyPatchGateBlocker::MissingExecutorContext => "missing_executor_context",
        ApplyPatchGateBlocker::ExecutorUnavailable => "executor_unavailable",
        ApplyPatchGateBlocker::PrimaryRootRejected => "primary_root_rejected",
    }
    .to_string()
}

fn format_apply_patch_execution_blockers(blockers: &[ApplyPatchExecutionBlocker]) -> Vec<String> {
    blockers
        .iter()
        .map(apply_patch_execution_blocker_label)
        .collect()
}

fn apply_patch_execution_blocker_label(blocker: &ApplyPatchExecutionBlocker) -> String {
    match blocker {
        ApplyPatchExecutionBlocker::PreflightNotReady => "preflight_not_ready",
        ApplyPatchExecutionBlocker::ExecutorUnavailable => "executor_unavailable",
        ApplyPatchExecutionBlocker::PrimaryRootRejected => "primary_root_rejected",
        ApplyPatchExecutionBlocker::MissingCheckpointLifecycle => "missing_checkpoint_lifecycle",
        ApplyPatchExecutionBlocker::CheckpointNotCreated => "checkpoint_not_created",
        ApplyPatchExecutionBlocker::MissingPolicyDecision => "missing_policy_decision",
        ApplyPatchExecutionBlocker::PolicyNotAllowed => "policy_not_allowed",
        ApplyPatchExecutionBlocker::MissingReviewerDecision => "missing_reviewer_decision",
        ApplyPatchExecutionBlocker::ReviewerNotAccepted => "reviewer_not_accepted",
        ApplyPatchExecutionBlocker::MissingSandboxProfile => "missing_sandbox_profile",
        ApplyPatchExecutionBlocker::MissingIsolatedRoot => "missing_isolated_root",
        ApplyPatchExecutionBlocker::UnsafePath => "unsafe_path",
        ApplyPatchExecutionBlocker::SymlinkRejected => "symlink_rejected",
        ApplyPatchExecutionBlocker::UnsupportedPatchOperation => "unsupported_patch_operation",
        ApplyPatchExecutionBlocker::MultipleFilesUnsupported => "multiple_files_unsupported",
        ApplyPatchExecutionBlocker::HunkConflict => "hunk_conflict",
        ApplyPatchExecutionBlocker::WriteFailed => "write_failed",
    }
    .to_string()
}

fn apply_patch_preflight_status_label(status: ApplyPatchPreflightStatus) -> &'static str {
    match status {
        ApplyPatchPreflightStatus::Blocked => "blocked",
        ApplyPatchPreflightStatus::DryRunReady => "dry_run_ready",
        ApplyPatchPreflightStatus::ExecutorReady => "executor_ready",
    }
}

fn apply_patch_execution_status_label(status: ApplyPatchExecutionStatus) -> &'static str {
    match status {
        ApplyPatchExecutionStatus::Planned => "planned",
        ApplyPatchExecutionStatus::Applied => "applied",
        ApplyPatchExecutionStatus::Conflict => "conflict",
        ApplyPatchExecutionStatus::Rejected => "rejected",
        ApplyPatchExecutionStatus::Failed => "failed",
    }
}

fn apply_patch_preflight_blocker_label(blocker: &ApplyPatchPreflightBlocker) -> String {
    match blocker {
        ApplyPatchPreflightBlocker::MissingMutationRequest => "missing_mutation_request",
        ApplyPatchPreflightBlocker::MissingPatchProposal => "missing_patch_proposal",
        ApplyPatchPreflightBlocker::OperationMismatch => "operation_mismatch",
        ApplyPatchPreflightBlocker::MutationRequestNotApproved => "mutation_request_not_approved",
        ApplyPatchPreflightBlocker::ScopeMismatch => "scope_mismatch",
        ApplyPatchPreflightBlocker::WorktreeIsolationRequired => "worktree_isolation_required",
        ApplyPatchPreflightBlocker::MissingPolicyDecision => "missing_policy_decision",
        ApplyPatchPreflightBlocker::PolicyNotAllowed => "policy_not_allowed",
        ApplyPatchPreflightBlocker::MissingReviewerDecision => "missing_reviewer_decision",
        ApplyPatchPreflightBlocker::ReviewerNotAccepted => "reviewer_not_accepted",
        ApplyPatchPreflightBlocker::MissingCheckpointLifecycle => "missing_checkpoint_lifecycle",
        ApplyPatchPreflightBlocker::CheckpointNotCreated => "checkpoint_not_created",
        ApplyPatchPreflightBlocker::MissingSandboxProfile => "missing_sandbox_profile",
        ApplyPatchPreflightBlocker::MissingPatchBody => "missing_patch_body",
        ApplyPatchPreflightBlocker::PatchBodyTooLarge => "patch_body_too_large",
        ApplyPatchPreflightBlocker::UnsafePatchPath => "unsafe_patch_path",
        ApplyPatchPreflightBlocker::UnsupportedPatchOperation => "unsupported_patch_operation",
        ApplyPatchPreflightBlocker::ExecutorUnavailable => "executor_unavailable",
    }
    .to_string()
}

impl CliApplyPatchOutput {
    fn from_records(
        trace_id: String,
        isolated_root_label: String,
        preflight: &ApplyPatchPreflightRecord,
        execution: Option<&ApplyPatchExecutionRecord>,
    ) -> Self {
        Self {
            trace_id,
            preflight_status: apply_patch_preflight_status_label(preflight.status).to_string(),
            executor_blocked: preflight.executor_blocked,
            execution_status: execution
                .map(|record| apply_patch_execution_status_label(record.status).to_string()),
            affected_paths: execution
                .map(|record| record.affected_paths.clone())
                .unwrap_or_else(|| preflight.affected_paths.clone()),
            conflict_paths: execution
                .map(|record| record.conflict_paths.clone())
                .unwrap_or_default(),
            blockers: execution
                .map(|record| format_apply_patch_execution_blockers(&record.blockers))
                .unwrap_or_else(|| {
                    preflight
                        .blockers
                        .iter()
                        .map(apply_patch_preflight_blocker_label)
                        .collect()
                }),
            executor_label: execution
                .map(|record| record.executor_label.clone())
                .unwrap_or_else(|| APPLY_PATCH_EXECUTOR_LABEL.to_string()),
            isolated_root_label: execution
                .map(|record| record.isolated_root_label.clone())
                .unwrap_or(isolated_root_label),
            worktree_id: None,
            source_commit: None,
            worktree_path: None,
            worktree_lifecycle_status: None,
        }
    }

    fn with_auto_worktree(
        mut self,
        created: &tessera_core::IsolatedWorktreeCreated,
        status: WorkspaceWorktreeLifecycleStatus,
    ) -> Self {
        self.worktree_id = Some(created.plan.worktree_id.to_string());
        self.source_commit = Some(created.plan.source_commit.clone());
        self.worktree_path = Some(created.plan.worktree_path.display().to_string());
        self.worktree_lifecycle_status = Some(worktree_lifecycle_status_label(status).to_string());
        self
    }
}

impl CliWorktreeCleanupOutput {
    fn from_created(created: &IsolatedWorktreeCreated, status: &str) -> Self {
        Self {
            trace_id: created.plan.trace_id.clone(),
            worktree_id: created.plan.worktree_id.to_string(),
            worktree_path: created.plan.worktree_path.display().to_string(),
            worktree_lifecycle_status: status.to_string(),
            source_commit: created.plan.source_commit.clone(),
            worktree_root_label: created.plan.worktree_root_label.clone(),
        }
    }
}

impl CliWorktreeListOutput {
    fn from_lifecycle(lifecycle: &RuntimeWorktreeLifecycleSummary, saw_created: bool) -> Self {
        let trace_cleanup_candidate = saw_created
            && lifecycle.latest_status == WorkspaceWorktreeLifecycleStatus::Retained
            && lifecycle.worktree_root_label.starts_with("worktree:")
            && lifecycle.created_for_request_id.is_some()
            && lifecycle.created_for_patch_id.is_some();

        Self {
            trace_id: lifecycle.trace_id.clone(),
            worktree_id: lifecycle.worktree_id.to_string(),
            workflow_id: lifecycle.workflow_id.to_string(),
            task_id: lifecycle.task_id.to_string(),
            first_event_seq: lifecycle.first_event_seq,
            latest_event_seq: lifecycle.latest_event_seq,
            latest_status: worktree_lifecycle_status_label(lifecycle.latest_status).to_string(),
            source_commit: lifecycle.source_commit.clone(),
            source_branch_label: lifecycle.source_branch_label.clone(),
            worktree_root_label: lifecycle.worktree_root_label.clone(),
            worktree_base_key: lifecycle.worktree_base_key.clone(),
            created_for_request_id: lifecycle
                .created_for_request_id
                .as_ref()
                .map(ToString::to_string),
            created_for_patch_id: lifecycle
                .created_for_patch_id
                .as_ref()
                .map(ToString::to_string),
            trace_cleanup_candidate,
            trace_cleanup_candidate_note:
                "trace evidence only; cleanup still requires explicit --worktree-path validation"
                    .to_string(),
        }
    }
}

fn worktree_lifecycle_status_label(status: WorkspaceWorktreeLifecycleStatus) -> &'static str {
    match status {
        WorkspaceWorktreeLifecycleStatus::Planned => "planned",
        WorkspaceWorktreeLifecycleStatus::Created => "created",
        WorkspaceWorktreeLifecycleStatus::CreationFailed => "creation_failed",
        WorkspaceWorktreeLifecycleStatus::Retained => "retained",
        WorkspaceWorktreeLifecycleStatus::CleanupStarted => "cleanup_started",
        WorkspaceWorktreeLifecycleStatus::CleanupCompleted => "cleanup_completed",
        WorkspaceWorktreeLifecycleStatus::CleanupFailed => "cleanup_failed",
    }
}

fn ensure_provider_profile(config: &TesseraConfig, provider_id: &str) -> Result<()> {
    if config
        .providers
        .iter()
        .any(|profile| profile.id == provider_id)
    {
        return Ok(());
    }

    Err(anyhow::anyhow!("provider profile not found: {provider_id}"))
}

pub async fn run_chat_mock(
    data_dir: impl AsRef<Path>,
    prompt: impl Into<String>,
) -> Result<ConversationOutcome> {
    let store = TraceStore::open(data_dir)?;
    let engine = ConversationEngine::new(MockProvider::default(), store);
    let outcome = engine.run_chat(ConversationRequest::mock(prompt)).await?;
    Ok(outcome)
}

pub async fn run_chat_with_config(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
) -> Result<ConversationOutcome> {
    run_chat_with_config_and_events(data_dir, config, provider_id, prompt, |_| {}).await
}

pub async fn run_chat_with_config_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    run_chat_with_config_and_controls_and_events(
        data_dir,
        config,
        provider_id,
        prompt,
        RunControls::default(),
        event_sink,
    )
    .await
}

pub async fn run_chat_with_config_and_controls_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    controls: RunControls,
    event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    run_chat_with_config_history_controls_and_events(
        data_dir,
        config,
        provider_id,
        prompt,
        Vec::new(),
        controls,
        event_sink,
    )
    .await
}

pub async fn run_chat_with_config_history_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    history: Vec<ProviderMessage>,
    event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    run_chat_with_config_history_controls_and_events(
        data_dir,
        config,
        provider_id,
        prompt,
        history,
        RunControls::default(),
        event_sink,
    )
    .await
}

pub async fn run_chat_with_config_history_controls_and_events<F, R>(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    prompt: impl Into<String>,
    history: Vec<ProviderMessage>,
    controls: RunControls,
    mut event_sink: F,
) -> Result<ConversationOutcome>
where
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    let profile = config
        .providers
        .iter()
        .find(|profile| profile.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("provider profile not found: {provider_id}"))?;

    match profile.kind.as_str() {
        "mock" => {
            run_chat_for_provider_with_events(
                data_dir,
                profile,
                MockProvider::default(),
                prompt,
                history,
                controls,
                &mut event_sink,
            )
            .await
        }
        "openai-compatible" | "openai_compatible" => {
            let base_url = profile.base_url.as_deref().ok_or_else(|| {
                anyhow::anyhow!("provider profile `{}` requires base_url", profile.id)
            })?;
            let api_key = read_api_key(profile)?;
            let provider = OpenAiCompatibleProvider::new(
                base_url,
                api_key,
                ProviderId::from(profile.id.as_str()),
            );
            run_chat_for_provider_with_events(
                data_dir,
                profile,
                provider,
                prompt,
                history,
                controls,
                &mut event_sink,
            )
            .await
        }
        "ollama" => {
            let base_url = profile
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            let provider = OllamaProvider::new(base_url, ProviderId::from(profile.id.as_str()));
            run_chat_for_provider_with_events(
                data_dir,
                profile,
                provider,
                prompt,
                history,
                controls,
                &mut event_sink,
            )
            .await
        }
        other => Err(anyhow::anyhow!(
            "unsupported provider kind `{other}` for profile `{}`",
            profile.id
        )),
    }
}

pub async fn run_agent_with_config(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    goal: impl Into<String>,
) -> Result<AgentRunOutcome> {
    run_agent_with_config_and_instruction_options(data_dir, config, provider_id, goal, None, None)
        .await
}

pub fn inspect_instructions(
    workspace: impl AsRef<Path>,
    target_dir: Option<PathBuf>,
) -> Result<LoadedInstructionSet> {
    let workspace = workspace.as_ref().to_path_buf();
    let target_dir = target_dir.unwrap_or_else(|| workspace.clone());
    let options = InstructionDiscoveryOptions::new(workspace, target_dir);
    InstructionDiscoveryPlanner
        .discover(options)
        .map_err(Into::into)
}

pub fn format_instruction_discovery_lines(set: &LoadedInstructionSet) -> Vec<String> {
    let mut lines = vec![format!(
        "instructions: {} loaded / {} sources",
        set.loaded.len(),
        set.sources.len()
    )];

    for source in &set.sources {
        lines.push(format!(
            "{} {} {} ({}/{})",
            snake_json_label(&source.status),
            snake_json_label(&source.placement),
            source.relative_path,
            source.loaded_bytes,
            source.original_bytes
        ));
    }
    if !set.warnings.is_empty() {
        lines.push(format!("warnings: {}", set.warnings.len()));
    }

    lines
}

pub fn inspect_skills(
    workspace: impl AsRef<Path>,
    target_dir: Option<PathBuf>,
) -> Result<SkillDiscoveryReport> {
    let workspace = workspace.as_ref().to_path_buf();
    let target_dir = target_dir.unwrap_or_else(|| workspace.clone());
    let options = SkillDiscoveryOptions::new(workspace, target_dir);
    SkillRuntimePlanner.discover(options).map_err(Into::into)
}

pub fn format_skill_discovery_lines(report: &SkillDiscoveryReport) -> Vec<String> {
    let mut lines = vec![format!(
        "skills: {} loadable / {} sources",
        report.manifests.len(),
        report.sources.len()
    )];

    for source in &report.sources {
        let manifest = report
            .manifests
            .iter()
            .find(|manifest| manifest.source.uri.as_deref() == Some(source.relative_path.as_str()));
        if let Some(manifest) = manifest {
            lines.push(format!(
                "- {} {} {} {}",
                manifest.id.as_str(),
                manifest.name,
                source.relative_path,
                snake_json_label(&source.status)
            ));
        } else {
            lines.push(format!(
                "- {} {}",
                source.relative_path,
                snake_json_label(&source.status)
            ));
        }
    }
    if !report.warnings.is_empty() {
        lines.push(format!("warnings: {}", report.warnings.len()));
    }

    lines
}

pub fn load_skill_context(options: CliSkillContextOptions) -> Result<LoadedSkillSet> {
    let requested_skills = options.skills.iter().cloned().collect::<HashSet<_>>();
    for (skill, _) in &options.references {
        if !requested_skills.contains(skill) {
            return Err(anyhow::anyhow!(
                "--skill-reference references `{skill}` but that skill was not requested"
            ));
        }
    }

    let requests = options
        .skills
        .into_iter()
        .map(|skill| {
            let references = options
                .references
                .iter()
                .filter_map(|(reference_skill, reference)| {
                    if reference_skill == &skill {
                        Some(reference.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            SkillActivationRequest { skill, references }
        })
        .collect::<Vec<_>>();
    let target_dir = options
        .target_dir
        .clone()
        .unwrap_or_else(|| options.workspace.clone());
    let runtime_options = SkillRuntimeOptions::new(options.workspace, target_dir, requests);
    SkillRuntimePlanner
        .activate(runtime_options)
        .map_err(Into::into)
}

pub async fn run_agent_with_config_and_instruction_options(
    data_dir: impl AsRef<Path>,
    config: &TesseraConfig,
    provider_id: &str,
    goal: impl Into<String>,
    instruction_options: Option<CliInstructionContextOptions>,
    skill_options: Option<CliSkillContextOptions>,
) -> Result<AgentRunOutcome> {
    let goal = goal.into();
    let profile = config
        .providers
        .iter()
        .find(|profile| profile.id == provider_id)
        .ok_or_else(|| anyhow::anyhow!("provider profile not found: {provider_id}"))?;
    let instruction_context = match instruction_options {
        Some(options) => Some(inspect_instructions(options.workspace, options.target_dir)?),
        None => None,
    };
    let skill_context = match skill_options {
        Some(options) => Some(load_skill_context(options)?),
        None => None,
    };

    match profile.kind.as_str() {
        "mock" => {
            run_agent_for_provider(
                data_dir,
                profile,
                MockProvider::default(),
                goal,
                instruction_context,
                skill_context,
            )
            .await
        }
        "openai-compatible" | "openai_compatible" => {
            let base_url = profile.base_url.as_deref().ok_or_else(|| {
                anyhow::anyhow!("provider profile `{}` requires base_url", profile.id)
            })?;
            let api_key = read_api_key(profile)?;
            let provider = OpenAiCompatibleProvider::new(
                base_url,
                api_key,
                ProviderId::from(profile.id.as_str()),
            );
            run_agent_for_provider(
                data_dir,
                profile,
                provider,
                goal,
                instruction_context,
                skill_context,
            )
            .await
        }
        "ollama" => {
            let base_url = profile
                .base_url
                .as_deref()
                .unwrap_or("http://localhost:11434");
            let provider = OllamaProvider::new(base_url, ProviderId::from(profile.id.as_str()));
            run_agent_for_provider(
                data_dir,
                profile,
                provider,
                goal,
                instruction_context,
                skill_context,
            )
            .await
        }
        other => Err(anyhow::anyhow!(
            "unsupported provider kind `{other}` for profile `{}`",
            profile.id
        )),
    }
}

pub fn format_agent_run_lines(outcome: &AgentRunOutcome) -> Vec<String> {
    let mut lines = vec![
        format!("agent task {}", outcome.task_id),
        format!("trace {}", outcome.trace_id),
        format!("status {}", task_status_label(outcome.status.clone())),
        format!("steps {}", outcome.summary.steps_completed),
    ];
    if !outcome.assistant_text.is_empty() {
        lines.push(outcome.assistant_text.clone());
    }
    let (instruction_sources, instruction_warning_count) =
        instruction_report_from_trace(&outcome.store, &outcome.trace_id);
    if !instruction_sources.is_empty() {
        lines.push(format!("instruction_sources {}", instruction_sources.len()));
        lines.push(format!("instruction_warnings {instruction_warning_count}"));
    }
    let (skill_activations, skill_warning_count) =
        skill_report_from_trace(&outcome.store, &outcome.trace_id);
    if !skill_activations.is_empty() {
        lines.push(format!("skills {}", skill_activations.len()));
        lines.push(format!("skill_warnings {skill_warning_count}"));
        for activation in skill_activations {
            lines.push(format!(
                "- {} {}",
                activation.skill_id.as_str(),
                snake_json_label(&activation.status)
            ));
        }
    }
    lines
}

pub async fn run_tui_with_config(
    data_dir: PathBuf,
    config: TesseraConfig,
    provider_id: String,
) -> Result<()> {
    let state = build_tui_state_with_config(&config, &provider_id)?;
    let active_cancellation_token: Arc<Mutex<Option<RunCancellationToken>>> =
        Arc::new(Mutex::new(None));
    let active_pause_token: Arc<Mutex<Option<RunPauseToken>>> = Arc::new(Mutex::new(None));
    let submit_cancellation_token = Arc::clone(&active_cancellation_token);
    let submit_pause_token = Arc::clone(&active_pause_token);
    let cancel_cancellation_token = Arc::clone(&active_cancellation_token);
    let pause_pause_token = Arc::clone(&active_pause_token);

    tessera_tui::run_terminal_chat_with_runtime_handlers(
        state,
        move |selected_provider_id, prompt, live_events| {
            let data_dir = data_dir.clone();
            let config = config.clone();
            let active_cancellation_token = Arc::clone(&submit_cancellation_token);
            let active_pause_token = Arc::clone(&submit_pause_token);
            async move {
                let cancellation_token = RunCancellationToken::new();
                let pause_token = RunPauseToken::new();
                {
                    let mut active = active_cancellation_token
                        .lock()
                        .map_err(|_| "active cancellation token lock poisoned".to_string())?;
                    *active = Some(cancellation_token.clone());
                }
                {
                    let mut active = active_pause_token
                        .lock()
                        .map_err(|_| "active pause token lock poisoned".to_string())?;
                    *active = Some(pause_token.clone());
                }

                let controls = RunControls {
                    event_timeout: None,
                    cancellation_token: Some(cancellation_token.clone()),
                    pause_token: Some(pause_token.clone()),
                };
                let result = run_chat_with_config_and_controls_and_events(
                    data_dir,
                    &config,
                    &selected_provider_id,
                    prompt,
                    controls,
                    {
                        let live_events = live_events.clone();
                        move |frame| match live_events
                            .try_send(LiveClientEvent::Frame(Box::new(frame.clone())))
                        {
                            Ok(()) => EventSinkAction::Continue,
                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                EventSinkAction::Cancel("live event channel closed".to_string())
                            }
                            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                                EventSinkAction::Cancel("live event channel full".to_string())
                            }
                        }
                    },
                )
                .await
                .map(|_| ())
                .map_err(|error| error.to_string());

                let mut active = active_cancellation_token
                    .lock()
                    .map_err(|_| "active cancellation token lock poisoned".to_string())?;
                if active
                    .as_ref()
                    .is_some_and(|current| current.is_same_handle(&cancellation_token))
                {
                    *active = None;
                }
                let mut active = active_pause_token
                    .lock()
                    .map_err(|_| "active pause token lock poisoned".to_string())?;
                if active
                    .as_ref()
                    .is_some_and(|current| current.is_same_handle(&pause_token))
                {
                    *active = None;
                }

                result
            }
        },
        move |_task_id| {
            let token = cancel_cancellation_token
                .lock()
                .map_err(|_| "active cancellation token lock poisoned".to_string())?
                .clone();
            match token {
                Some(token) => {
                    token.cancel("tui cancel requested");
                    Ok("cancel requested".to_string())
                }
                None => Err("no active run to cancel".to_string()),
            }
        },
        move |_task_id| {
            let token = pause_pause_token
                .lock()
                .map_err(|_| "active pause token lock poisoned".to_string())?
                .clone();
            match token {
                Some(token) => {
                    token.pause("tui pause requested");
                    Ok("pause requested".to_string())
                }
                None => Err("no active run to pause".to_string()),
            }
        },
    )
    .await?;
    Ok(())
}

pub fn build_tui_state_with_config(
    config: &TesseraConfig,
    provider_id: &str,
) -> Result<ChatViewState> {
    if !config
        .providers
        .iter()
        .any(|profile| profile.id == provider_id)
    {
        return Err(anyhow::anyhow!("provider profile not found: {provider_id}"));
    }

    let profile_ids = config
        .providers
        .iter()
        .map(|profile| profile.id.clone())
        .collect::<Vec<_>>();
    Ok(ChatViewState::with_profiles(provider_id, profile_ids))
}

async fn run_chat_for_provider_with_events<P, F, R>(
    data_dir: impl AsRef<Path>,
    profile: &ProviderProfile,
    provider: P,
    prompt: impl Into<String>,
    history: Vec<ProviderMessage>,
    controls: RunControls,
    event_sink: &mut F,
) -> Result<ConversationOutcome>
where
    P: ChatProvider,
    F: FnMut(&EventFrame) -> R,
    R: Into<EventSinkAction>,
{
    let store = TraceStore::open(data_dir)?;
    let engine = ConversationEngine::new(provider, store);
    let outcome = engine
        .run_chat_with_controls_and_event_sink(
            ConversationRequest {
                trace_id: next_trace_id(&profile.id),
                provider_id: ProviderId::from(profile.id.as_str()),
                profile_id: ModelProfileId::from(profile.id.as_str()),
                model: profile.default_model.clone(),
                prompt: prompt.into(),
                history,
            },
            controls,
            event_sink,
        )
        .await?;
    Ok(outcome)
}

async fn run_agent_for_provider<P>(
    data_dir: impl AsRef<Path>,
    profile: &ProviderProfile,
    provider: P,
    goal: String,
    instruction_context: Option<LoadedInstructionSet>,
    skill_context: Option<LoadedSkillSet>,
) -> Result<AgentRunOutcome>
where
    P: ChatProvider,
{
    let store = TraceStore::open(data_dir)?;
    let agent_profile = agent_profile_from_provider_profile(profile);
    let loop_runtime = AgentLoop::new(provider, store);
    let outcome = loop_runtime
        .run_agent(AgentRunRequest {
            trace_id: next_trace_id(&profile.id),
            provider_id: ProviderId::from(profile.id.as_str()),
            profile_id: ModelProfileId::from(profile.id.as_str()),
            agent_profile: agent_profile.clone(),
            model: profile.default_model.clone(),
            objective: goal,
            context_references: Vec::new(),
            instruction_context,
            skill_context,
            history: Vec::new(),
            max_steps: agent_profile.max_steps,
        })
        .await?;
    Ok(outcome)
}

fn agent_profile_from_provider_profile(profile: &ProviderProfile) -> AgentProfile {
    AgentProfile {
        id: AgentProfileId::from(format!("agent_profile_{}", profile.id)),
        name: format!("{} agent", profile.id),
        role: "no-tool assistant".to_string(),
        model_profile: ModelProfileId::from(profile.id.as_str()),
        skills: Vec::new(),
        memory_scopes: Vec::new(),
        context_scopes: Vec::new(),
        tool_permissions: Vec::new(),
        max_steps: 1,
        metadata: None,
    }
}

fn read_api_key(profile: &ProviderProfile) -> Result<Option<String>> {
    let Some(env_name) = &profile.api_key_env else {
        return Ok(None);
    };
    let value = std::env::var(env_name)
        .map_err(|_| anyhow::anyhow!("environment variable `{env_name}` is not set"))?;
    Ok(Some(value))
}

pub fn resolve_data_dir(explicit: Option<PathBuf>) -> Result<PathBuf> {
    resolve_data_dir_with_config(explicit, &TesseraConfig::default_with_mock())
}

pub fn resolve_data_dir_with_config(
    explicit: Option<PathBuf>,
    config: &TesseraConfig,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Some(data_dir) = &config.data_dir {
        return Ok(PathBuf::from(data_dir));
    }
    tessera_config::default_data_dir().ok_or_else(|| anyhow::anyhow!("cannot resolve data dir"))
}

pub fn resolve_config(explicit: Option<PathBuf>) -> Result<TesseraConfig> {
    if let Some(path) = explicit {
        return Ok(TesseraConfig::load_from_path(path)?);
    }

    if let Ok(path) = std::env::var("TESSERA_CONFIG") {
        if !path.trim().is_empty() {
            return Ok(TesseraConfig::load_from_path(path)?);
        }
    }

    let default_path = PathBuf::from("tessera.toml");
    if default_path.is_file() {
        return Ok(TesseraConfig::load_from_path(default_path)?);
    }

    Ok(TesseraConfig::default_with_mock())
}

fn next_trace_id(provider_id: &str) -> String {
    let provider = provider_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let counter = TRACE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("trace_{provider}_{timestamp}_{counter}")
}
