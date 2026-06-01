//! UI-neutral client model for Tessera shells.

use std::collections::{BTreeMap, BTreeSet};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tessera_protocol::{
    AgentHandoffId, AgentHandoffStatus, AgentHandoffSummary, ApplyPatchExecutionRecord,
    ApplyPatchExecutionStatus, ApplyPatchPreflightRecord, ApplyPatchPreflightStatus, ApprovalId,
    ApprovalStatus, ArtifactId, ArtifactKind, ClientInstanceId, CodingWorkflowId, ContextId,
    ContextPlacement, ContextReference, ContextSourceKind, EventFrame, EventRange,
    HandoffEvidenceRef, ItemId, MemoryProposal, MemoryProposalId, MemoryProposalStatus,
    MutationMode, MutationRequestProposal, PatchApplicationRecord, PatchProposal,
    RestorePlanRecord, ReviewBundle, ReviewerDecisionKind, ReviewerGateDecision, ReviewerGateId,
    ReviewerGateRequest, RunEvent, RuntimeInstanceId, SubagentApprovalForwardingRecord,
    SubagentApprovalForwardingStatus, SubagentCancellationCascade, SubagentCancellationRecord,
    SubagentInactiveParentAction, SubagentInactivePolicy, SubagentInactivePolicyRecord,
    SubagentRuntimeDecision, SubagentRuntimeDecisionKind, SubagentSessionDescriptor,
    SubagentSessionId, SubagentSessionStatus, SubagentTranscriptArtifactLifecycleRecord,
    SubagentTranscriptArtifactRecord, SubagentTranscriptArtifactStatus, TaskId, TaskKind,
    TaskOwnerHeartbeat, TaskOwnerKind, TaskOwnerLease, TaskOwnerStatus, TaskOwnershipId,
    TaskReattachMode, TaskReattachRecord, TaskStatus, TestEvidenceSummaryRecord, TestPlanRecord,
    TestRunRecord, ThreadId, Timestamp, ToolApproval, ToolCallId, ToolId, ToolPermission,
    ToolPolicyDecision, ToolSideEffect, TraceRecord, TurnId, WorkspaceMutationScope,
    WorkspaceWorktreeId, WorkspaceWorktreeLifecycleRecord, WorkspaceWorktreeLifecycleStatus,
};

/// User intent shared by CLI/TUI/GUI surfaces before it reaches runtime code.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientIntent {
    SubmitPrompt { profile_id: String, prompt: String },
    SwitchProfile { profile_id: String },
    NewThread,
    SaveThread,
    ExportThread,
    CancelTask { task_id: Option<TaskId> },
    PauseTask { task_id: Option<TaskId> },
    ResumeTask { task_id: TaskId },
    ApproveToolCall { approval_id: ApprovalId },
    DenyToolCall { approval_id: ApprovalId },
    AcceptMemoryProposal { proposal_id: MemoryProposalId },
    RejectMemoryProposal { proposal_id: MemoryProposalId },
}

/// UI-neutral message role for client projections.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientMessageRole {
    System,
    User,
    Assistant,
    Reasoning,
}

/// UI-neutral chat message projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientMessage {
    pub role: ClientMessageRole,
    pub content: String,
    pub item_id: Option<ItemId>,
    pub streaming: bool,
}

/// UI-neutral task projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientTask {
    pub task_id: TaskId,
    pub kind: Option<TaskKind>,
    pub status: TaskStatus,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub created_at: Option<Timestamp>,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub cancel_reason: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    #[serde(default)]
    pub owner_lease_id: Option<TaskOwnershipId>,
    #[serde(default)]
    pub owner_runtime_id: Option<RuntimeInstanceId>,
    #[serde(default)]
    pub owner_client_id: Option<ClientInstanceId>,
    #[serde(default)]
    pub owner_kind: Option<TaskOwnerKind>,
    #[serde(default)]
    pub owner_status: Option<TaskOwnerStatus>,
    #[serde(default)]
    pub owner_reattach_mode: Option<TaskReattachMode>,
    #[serde(default)]
    pub owner_last_heartbeat_at: Option<Timestamp>,
    #[serde(default)]
    pub owner_expires_at: Option<Timestamp>,
    #[serde(default)]
    pub owner_last_seq: Option<u64>,
    #[serde(default)]
    pub owner_reason: Option<String>,
}

impl ClientTask {
    fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            kind: None,
            status: TaskStatus::Pending,
            thread_id: None,
            turn_id: None,
            created_at: None,
            started_at: None,
            finished_at: None,
            cancel_reason: None,
            error_code: None,
            error_message: None,
            owner_lease_id: None,
            owner_runtime_id: None,
            owner_client_id: None,
            owner_kind: None,
            owner_status: None,
            owner_reattach_mode: None,
            owner_last_heartbeat_at: None,
            owner_expires_at: None,
            owner_last_seq: None,
            owner_reason: None,
        }
    }

    fn update_scope(&mut self, thread_id: Option<ThreadId>, turn_id: Option<TurnId>) {
        if thread_id.is_some() {
            self.thread_id = thread_id;
        }
        if turn_id.is_some() {
            self.turn_id = turn_id;
        }
    }

    fn apply_owner_lease(&mut self, lease: &TaskOwnerLease) {
        self.owner_lease_id = Some(lease.lease_id.clone());
        self.owner_runtime_id = Some(lease.runtime_id.clone());
        self.owner_client_id = lease.client_id.clone();
        self.owner_kind = Some(lease.owner_kind);
        self.owner_status = Some(lease.status);
        self.owner_reattach_mode = Some(task_owner_default_reattach_mode(lease.status));
        self.owner_last_heartbeat_at = lease.last_heartbeat_at.clone();
        self.owner_expires_at = lease.expires_at.clone();
        self.owner_last_seq = lease.last_seq;
        self.owner_reason = lease.reason.clone();
    }

    fn apply_owner_heartbeat(&mut self, heartbeat: &TaskOwnerHeartbeat) {
        self.owner_lease_id = Some(heartbeat.lease_id.clone());
        self.owner_runtime_id = Some(heartbeat.runtime_id.clone());
        self.owner_status = Some(TaskOwnerStatus::Heartbeat);
        self.owner_reattach_mode = Some(TaskReattachMode::ObserveExistingOwner);
        self.owner_last_heartbeat_at = Some(heartbeat.heartbeat_at.clone());
        self.owner_expires_at = Some(heartbeat.expires_at.clone());
        self.owner_last_seq = Some(heartbeat.last_seq);
    }

    fn apply_owner_status(
        &mut self,
        lease_id: TaskOwnershipId,
        status: TaskOwnerStatus,
        reason: Option<String>,
    ) {
        self.owner_lease_id = Some(lease_id);
        self.owner_status = Some(status);
        self.owner_reattach_mode = Some(task_owner_default_reattach_mode(status));
        self.owner_reason = reason;
    }

    fn apply_reattach_record(&mut self, record: &TaskReattachRecord) {
        self.owner_reattach_mode = Some(record.mode);
        if let Some(lease_id) = record
            .new_lease_id
            .clone()
            .or_else(|| record.previous_lease_id.clone())
        {
            self.owner_lease_id = Some(lease_id);
        }
        if record.since_seq.is_some() {
            self.owner_last_seq = record.since_seq;
        }
        if record.reason.is_some() {
            self.owner_reason = record.reason.clone();
        }
    }

    fn apply_terminal_projection_to_owner(&mut self) {
        if self.owner_status.is_some() {
            self.owner_reattach_mode = Some(TaskReattachMode::TerminalProjection);
        }
    }
}

/// UI-neutral artifact handle projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientArtifact {
    pub artifact_id: ArtifactId,
    pub kind: Option<ArtifactKind>,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub task_id: Option<TaskId>,
    pub item_id: Option<ItemId>,
    pub created_at: Option<Timestamp>,
    pub referenced_by_event_kinds: Vec<String>,
}

impl ClientArtifact {
    fn new(artifact_id: ArtifactId) -> Self {
        Self {
            artifact_id,
            kind: None,
            thread_id: None,
            turn_id: None,
            task_id: None,
            item_id: None,
            created_at: None,
            referenced_by_event_kinds: Vec::new(),
        }
    }

    fn update_scope(
        &mut self,
        thread_id: Option<ThreadId>,
        turn_id: Option<TurnId>,
        task_id: Option<TaskId>,
        item_id: Option<ItemId>,
    ) {
        if thread_id.is_some() {
            self.thread_id = thread_id;
        }
        if turn_id.is_some() {
            self.turn_id = turn_id;
        }
        if task_id.is_some() {
            self.task_id = task_id;
        }
        if item_id.is_some() {
            self.item_id = item_id;
        }
    }

    fn record_reference(&mut self, event_kind: &str) {
        if !self
            .referenced_by_event_kinds
            .iter()
            .any(|existing| existing == event_kind)
        {
            self.referenced_by_event_kinds.push(event_kind.to_string());
        }
    }
}

/// UI-neutral context source kind shared by client shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientContextSourceKind {
    File,
    Directory,
    Workspace,
    Artifact,
    Trace,
    Inline,
    Url,
}

impl From<ContextSourceKind> for ClientContextSourceKind {
    fn from(kind: ContextSourceKind) -> Self {
        match kind {
            ContextSourceKind::File => Self::File,
            ContextSourceKind::Directory => Self::Directory,
            ContextSourceKind::Workspace => Self::Workspace,
            ContextSourceKind::Artifact => Self::Artifact,
            ContextSourceKind::Trace => Self::Trace,
            ContextSourceKind::Inline => Self::Inline,
            ContextSourceKind::Url => Self::Url,
        }
    }
}

/// UI-neutral context placement shared by client shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientContextPlacement {
    StablePrefix,
    AppendOnlyTranscript,
    VolatileScratch,
}

impl From<ContextPlacement> for ClientContextPlacement {
    fn from(placement: ContextPlacement) -> Self {
        match placement {
            ContextPlacement::StablePrefix => Self::StablePrefix,
            ContextPlacement::AppendOnlyTranscript => Self::AppendOnlyTranscript,
            ContextPlacement::VolatileScratch => Self::VolatileScratch,
        }
    }
}

/// Client-side context budget summary kept independent from core crate boundaries.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientContextBudgetSummary {
    pub max_tokens: u64,
    pub reserved_output_tokens: u64,
    pub available_tokens: u64,
    pub used_tokens: u64,
    pub remaining_tokens: u64,
    pub stable_prefix_tokens: u64,
    pub append_only_transcript_tokens: u64,
    pub volatile_scratch_tokens: u64,
    pub over_budget: bool,
}

/// UI-neutral context handle projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientContextHandle {
    pub context_id: ContextId,
    pub source_kind: ClientContextSourceKind,
    pub source_uri: Option<String>,
    pub label: Option<String>,
    pub placement: ClientContextPlacement,
    pub estimated_tokens: u64,
    pub pinned: bool,
    pub summary: Option<String>,
}

impl ClientContextHandle {
    fn from_reference(reference: ContextReference) -> Self {
        Self {
            context_id: reference.id,
            source_kind: reference.source.kind.into(),
            source_uri: reference.source.uri,
            label: reference.source.label,
            placement: reference.placement.into(),
            estimated_tokens: reference.estimated_tokens,
            pinned: reference.pinned,
            summary: reference.summary,
        }
    }
}

/// UI-neutral approval state shared by terminal and future GUI shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientApprovalStatus {
    Pending,
    Approved,
    Denied,
}

impl From<ApprovalStatus> for ClientApprovalStatus {
    fn from(status: ApprovalStatus) -> Self {
        match status {
            ApprovalStatus::Pending => Self::Pending,
            ApprovalStatus::Approved => Self::Approved,
            ApprovalStatus::Denied => Self::Denied,
        }
    }
}

/// UI-neutral approval projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientApproval {
    pub approval_id: ApprovalId,
    pub call_id: ToolCallId,
    pub tool_id: ToolId,
    pub status: ClientApprovalStatus,
    pub reason: Option<String>,
    pub required_permissions: Vec<String>,
    pub side_effects: Vec<String>,
}

impl ClientApproval {
    fn pending_from_decision(decision: &ToolPolicyDecision, approval_id: ApprovalId) -> Self {
        Self {
            approval_id,
            call_id: decision.call_id.clone(),
            tool_id: decision.tool_id.clone(),
            status: ClientApprovalStatus::Pending,
            reason: Some(decision.reason.clone()),
            required_permissions: decision
                .required_permissions
                .iter()
                .map(tool_permission_label)
                .map(str::to_string)
                .collect(),
            side_effects: decision
                .side_effects
                .iter()
                .map(tool_side_effect_label)
                .map(str::to_string)
                .collect(),
        }
    }

    fn from_approval(approval: &ToolApproval) -> Self {
        Self {
            approval_id: approval.approval_id.clone(),
            call_id: approval.call_id.clone(),
            tool_id: approval.tool_id.clone(),
            status: approval.status.into(),
            reason: approval.reason.clone(),
            required_permissions: Vec::new(),
            side_effects: Vec::new(),
        }
    }

    fn update_from_approval(&mut self, approval: &ToolApproval) {
        self.call_id = approval.call_id.clone();
        self.tool_id = approval.tool_id.clone();
        self.status = approval.status.into();
        self.reason = approval.reason.clone();
    }
}

/// UI-neutral memory proposal status shared by terminal and future GUI shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientMemoryProposalStatus {
    Pending,
    Applied,
    Rejected,
}

impl From<MemoryProposalStatus> for ClientMemoryProposalStatus {
    fn from(status: MemoryProposalStatus) -> Self {
        match status {
            MemoryProposalStatus::Pending => Self::Pending,
            MemoryProposalStatus::Applied => Self::Applied,
            MemoryProposalStatus::Rejected => Self::Rejected,
        }
    }
}

/// UI-neutral memory proposal projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientMemoryProposal {
    pub proposal_id: MemoryProposalId,
    pub status: ClientMemoryProposalStatus,
    pub title: String,
    pub summary: String,
    pub source_item_id: Option<ItemId>,
    pub reason: Option<String>,
}

impl ClientMemoryProposal {
    fn from_proposal(proposal: &MemoryProposal) -> Self {
        Self {
            proposal_id: proposal.proposal_id.clone(),
            status: proposal.status.into(),
            title: proposal.title.clone(),
            summary: proposal.summary.clone(),
            source_item_id: proposal.source_item_id.clone(),
            reason: proposal.reason.clone(),
        }
    }

    fn update_from_proposal(&mut self, proposal: &MemoryProposal) {
        self.status = proposal.status.into();
        self.title = proposal.title.clone();
        self.summary = proposal.summary.clone();
        self.source_item_id = proposal.source_item_id.clone();
        self.reason = proposal.reason.clone();
    }
}

/// UI-neutral reviewer gate status shared by terminal and future GUI shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientReviewerGateStatus {
    Pending,
    Accepted,
    Rejected,
    RevisionRequested,
}

impl From<ReviewerDecisionKind> for ClientReviewerGateStatus {
    fn from(decision: ReviewerDecisionKind) -> Self {
        match decision {
            ReviewerDecisionKind::Accept => Self::Accepted,
            ReviewerDecisionKind::Reject => Self::Rejected,
            ReviewerDecisionKind::RequestRevision => Self::RevisionRequested,
        }
    }
}

/// UI-neutral structured handoff projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientAgentHandoff {
    pub handoff_id: AgentHandoffId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub status: AgentHandoffStatus,
    pub objective: String,
    pub summary: String,
    pub evidence: Vec<HandoffEvidenceRef>,
    pub steps_completed: u32,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub estimated_cost: Option<f64>,
    pub cost_currency: Option<String>,
    pub evidence_event_range: Option<EventRange>,
}

impl ClientAgentHandoff {
    fn from_summary(summary: &AgentHandoffSummary) -> Self {
        Self {
            handoff_id: summary.handoff_id.clone(),
            parent_task_id: summary.parent_task_id.clone(),
            child_task_id: summary.child_task_id.clone(),
            status: summary.status,
            objective: summary.objective.clone(),
            summary: summary.summary.clone(),
            evidence: summary.evidence.clone(),
            steps_completed: summary.metrics.steps_completed,
            input_tokens: summary.metrics.input_tokens,
            output_tokens: summary.metrics.output_tokens,
            estimated_cost: summary
                .metrics
                .estimated_cost
                .as_ref()
                .map(|cost| cost.amount),
            cost_currency: summary
                .metrics
                .estimated_cost
                .as_ref()
                .map(|cost| cost.currency.clone()),
            evidence_event_range: summary.evidence_event_range.clone(),
        }
    }
}

/// UI-neutral reviewer gate projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientReviewerGate {
    pub gate_id: ReviewerGateId,
    pub handoff_id: AgentHandoffId,
    pub parent_task_id: Option<TaskId>,
    pub status: ClientReviewerGateStatus,
    pub requested_decisions: Vec<ReviewerDecisionKind>,
    pub decision: Option<ReviewerDecisionKind>,
    pub reviewer: Option<String>,
    pub reason_code: Option<String>,
    pub comment: Option<String>,
    pub evidence: Vec<HandoffEvidenceRef>,
}

impl ClientReviewerGate {
    fn pending_from_request(request: &ReviewerGateRequest) -> Self {
        Self {
            gate_id: request.gate_id.clone(),
            handoff_id: request.handoff_id.clone(),
            parent_task_id: Some(request.parent_task_id.clone()),
            status: ClientReviewerGateStatus::Pending,
            requested_decisions: request.requested_decisions.clone(),
            decision: None,
            reviewer: None,
            reason_code: None,
            comment: None,
            evidence: request.evidence.clone(),
        }
    }

    fn from_decision(decision: &ReviewerGateDecision) -> Self {
        let mut gate = Self {
            gate_id: decision.gate_id.clone(),
            handoff_id: decision.handoff_id.clone(),
            parent_task_id: None,
            status: ClientReviewerGateStatus::from(decision.decision),
            requested_decisions: Vec::new(),
            decision: None,
            reviewer: None,
            reason_code: None,
            comment: None,
            evidence: Vec::new(),
        };
        gate.apply_decision(decision);
        gate
    }

    fn apply_decision(&mut self, decision: &ReviewerGateDecision) {
        self.handoff_id = decision.handoff_id.clone();
        self.status = ClientReviewerGateStatus::from(decision.decision);
        self.decision = Some(decision.decision);
        self.reviewer = Some(decision.reviewer.clone());
        self.reason_code = Some(decision.reason_code.clone());
        self.comment = decision.comment.clone();
    }
}

/// UI-neutral sub-agent session status shared by terminal and future GUI shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientSubagentSessionStatus {
    Planned,
    Active,
    WaitingForApproval,
    Inactive,
    Completed,
    Failed,
    Cancelled,
    HandedOff,
}

impl From<SubagentSessionStatus> for ClientSubagentSessionStatus {
    fn from(status: SubagentSessionStatus) -> Self {
        match status {
            SubagentSessionStatus::Planned => Self::Planned,
            SubagentSessionStatus::Active => Self::Active,
            SubagentSessionStatus::WaitingForApproval => Self::WaitingForApproval,
            SubagentSessionStatus::Inactive => Self::Inactive,
            SubagentSessionStatus::Completed => Self::Completed,
            SubagentSessionStatus::Failed => Self::Failed,
            SubagentSessionStatus::Cancelled => Self::Cancelled,
            SubagentSessionStatus::HandedOff => Self::HandedOff,
        }
    }
}

/// UI-neutral sub-agent session metadata projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSubagentSession {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub profile_id: tessera_protocol::AgentProfileId,
    pub objective: String,
    pub status: ClientSubagentSessionStatus,
    pub scope_labels: Vec<String>,
    pub tool_permission_labels: Vec<String>,
    pub memory_scope_labels: Vec<String>,
    pub transcript_artifact_id: Option<ArtifactId>,
    pub max_steps: u32,
    pub max_depth: u32,
    pub timeout_ms: Option<u64>,
    pub max_child_sessions: u32,
    pub estimated_cost: Option<f64>,
    pub cost_currency: Option<String>,
    pub concurrency_slot: Option<String>,
    pub approval_inactive_policy: Option<String>,
    pub approval_reviewer_gate_id: Option<ReviewerGateId>,
    pub approval_id: Option<ApprovalId>,
    pub approval_forwarded_from_parent: bool,
}

impl ClientSubagentSession {
    fn from_descriptor(session: &SubagentSessionDescriptor) -> Self {
        let approval = session.approval_forwarding.as_ref();
        Self {
            session_id: session.session_id.clone(),
            parent_task_id: session.parent_task_id.clone(),
            child_task_id: session.child_task_id.clone(),
            profile_id: session.profile_id.clone(),
            objective: session.objective.clone(),
            status: session.status.into(),
            scope_labels: session.scope_labels.clone(),
            tool_permission_labels: session.tool_permission_labels.clone(),
            memory_scope_labels: session.memory_scope_labels.clone(),
            transcript_artifact_id: session.transcript_artifact_id.clone(),
            max_steps: session.caps.max_steps,
            max_depth: session.caps.max_depth,
            timeout_ms: session.caps.timeout_ms,
            max_child_sessions: session.caps.max_child_sessions,
            estimated_cost: session
                .caps
                .max_estimated_cost
                .as_ref()
                .map(|cost| cost.amount),
            cost_currency: session
                .caps
                .max_estimated_cost
                .as_ref()
                .map(|cost| cost.currency.clone()),
            concurrency_slot: session.caps.concurrency_slot.clone(),
            approval_inactive_policy: approval
                .map(|forwarding| inactive_policy_label(forwarding.inactive_policy).to_string()),
            approval_reviewer_gate_id: approval
                .and_then(|forwarding| forwarding.reviewer_gate_id.clone()),
            approval_id: approval.and_then(|forwarding| forwarding.approval_id.clone()),
            approval_forwarded_from_parent: approval
                .map(|forwarding| forwarding.forwarded_from_parent)
                .unwrap_or(false),
        }
    }
}

/// UI-neutral sub-agent runtime decision kind shared by client shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientSubagentRuntimeDecisionKind {
    StartAllowed,
    StartDenied,
    QueueOnly,
    RequireReviewer,
}

impl From<SubagentRuntimeDecisionKind> for ClientSubagentRuntimeDecisionKind {
    fn from(kind: SubagentRuntimeDecisionKind) -> Self {
        match kind {
            SubagentRuntimeDecisionKind::StartAllowed => Self::StartAllowed,
            SubagentRuntimeDecisionKind::StartDenied => Self::StartDenied,
            SubagentRuntimeDecisionKind::QueueOnly => Self::QueueOnly,
            SubagentRuntimeDecisionKind::RequireReviewer => Self::RequireReviewer,
        }
    }
}

/// UI-neutral sub-agent runtime decision projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSubagentRuntimeDecision {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub kind: ClientSubagentRuntimeDecisionKind,
    pub reason: String,
    pub max_steps: u32,
    pub max_depth: u32,
    pub timeout_ms: Option<u64>,
    pub max_child_sessions: u32,
    pub estimated_cost: Option<f64>,
    pub cost_currency: Option<String>,
    pub concurrency_slot: Option<String>,
}

impl ClientSubagentRuntimeDecision {
    fn from_decision(decision: &SubagentRuntimeDecision) -> Self {
        Self {
            session_id: decision.session_id.clone(),
            parent_task_id: decision.parent_task_id.clone(),
            child_task_id: decision.child_task_id.clone(),
            kind: decision.kind.into(),
            reason: decision.reason.clone(),
            max_steps: decision.caps_snapshot.max_steps,
            max_depth: decision.caps_snapshot.max_depth,
            timeout_ms: decision.caps_snapshot.timeout_ms,
            max_child_sessions: decision.caps_snapshot.max_child_sessions,
            estimated_cost: decision
                .caps_snapshot
                .max_estimated_cost
                .as_ref()
                .map(|cost| cost.amount),
            cost_currency: decision
                .caps_snapshot
                .max_estimated_cost
                .as_ref()
                .map(|cost| cost.currency.clone()),
            concurrency_slot: decision.caps_snapshot.concurrency_slot.clone(),
        }
    }
}

/// UI-neutral sub-agent transcript artifact projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSubagentTranscriptArtifact {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub artifact_id: ArtifactId,
    pub event_range: EventRange,
    pub summary_label: Option<String>,
}

impl ClientSubagentTranscriptArtifact {
    fn from_record(record: &SubagentTranscriptArtifactRecord) -> Self {
        Self {
            session_id: record.session_id.clone(),
            parent_task_id: record.parent_task_id.clone(),
            child_task_id: record.child_task_id.clone(),
            artifact_id: record.artifact_id.clone(),
            event_range: record.event_range.clone(),
            summary_label: record.summary_label.clone(),
        }
    }
}

/// UI-neutral sub-agent transcript artifact lifecycle status shared by client shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientSubagentTranscriptArtifactStatus {
    Reserved,
    Published,
    Sealed,
    Abandoned,
}

impl From<SubagentTranscriptArtifactStatus> for ClientSubagentTranscriptArtifactStatus {
    fn from(status: SubagentTranscriptArtifactStatus) -> Self {
        match status {
            SubagentTranscriptArtifactStatus::Reserved => Self::Reserved,
            SubagentTranscriptArtifactStatus::Published => Self::Published,
            SubagentTranscriptArtifactStatus::Sealed => Self::Sealed,
            SubagentTranscriptArtifactStatus::Abandoned => Self::Abandoned,
        }
    }
}

/// UI-neutral sub-agent transcript artifact lifecycle projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSubagentTranscriptArtifactLifecycle {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub child_task_id: Option<TaskId>,
    pub artifact_id: ArtifactId,
    pub status: ClientSubagentTranscriptArtifactStatus,
    pub event_range: Option<EventRange>,
    pub summary_label: Option<String>,
    pub reason: String,
}

impl ClientSubagentTranscriptArtifactLifecycle {
    fn from_record(record: &SubagentTranscriptArtifactLifecycleRecord) -> Self {
        Self {
            session_id: record.session_id.clone(),
            parent_task_id: record.parent_task_id.clone(),
            child_task_id: record.child_task_id.clone(),
            artifact_id: record.artifact_id.clone(),
            status: record.status.into(),
            event_range: record.event_range.clone(),
            summary_label: record.summary_label.clone(),
            reason: record.reason.clone(),
        }
    }
}

/// UI-neutral sub-agent approval forwarding status shared by client shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientSubagentApprovalForwardingStatus {
    QueuedForReviewer,
    ForwardedToParent,
    DeniedByPolicy,
}

impl From<SubagentApprovalForwardingStatus> for ClientSubagentApprovalForwardingStatus {
    fn from(status: SubagentApprovalForwardingStatus) -> Self {
        match status {
            SubagentApprovalForwardingStatus::QueuedForReviewer => Self::QueuedForReviewer,
            SubagentApprovalForwardingStatus::ForwardedToParent => Self::ForwardedToParent,
            SubagentApprovalForwardingStatus::DeniedByPolicy => Self::DeniedByPolicy,
        }
    }
}

/// UI-neutral sub-agent approval forwarding projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSubagentApprovalForwarding {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub approval_id: ApprovalId,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub status: ClientSubagentApprovalForwardingStatus,
    pub reason: String,
}

impl ClientSubagentApprovalForwarding {
    fn from_record(record: &SubagentApprovalForwardingRecord) -> Self {
        Self {
            session_id: record.session_id.clone(),
            parent_task_id: record.parent_task_id.clone(),
            approval_id: record.approval_id.clone(),
            reviewer_gate_id: record.reviewer_gate_id.clone(),
            status: record.status.into(),
            reason: record.reason.clone(),
        }
    }
}

/// UI-neutral inactive-child parent action shared by client shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientSubagentInactiveParentAction {
    PauseParent,
    QueueDecision,
    RequireReviewer,
}

impl From<SubagentInactiveParentAction> for ClientSubagentInactiveParentAction {
    fn from(action: SubagentInactiveParentAction) -> Self {
        match action {
            SubagentInactiveParentAction::PauseParent => Self::PauseParent,
            SubagentInactiveParentAction::QueueDecision => Self::QueueDecision,
            SubagentInactiveParentAction::RequireReviewer => Self::RequireReviewer,
        }
    }
}

/// UI-neutral sub-agent inactive policy projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSubagentInactivePolicy {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub policy: Option<String>,
    pub parent_action: ClientSubagentInactiveParentAction,
    pub reason: String,
}

impl ClientSubagentInactivePolicy {
    fn from_record(record: &SubagentInactivePolicyRecord) -> Self {
        Self {
            session_id: record.session_id.clone(),
            parent_task_id: record.parent_task_id.clone(),
            policy: Some(inactive_policy_label(record.policy).to_string()),
            parent_action: record.parent_action.into(),
            reason: record.reason.clone(),
        }
    }
}

/// UI-neutral sub-agent cancellation cascade shared by client shells.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum ClientSubagentCancellationCascade {
    CancelChild,
    ObserveOnly,
    QueueCancellation,
}

impl From<SubagentCancellationCascade> for ClientSubagentCancellationCascade {
    fn from(cascade: SubagentCancellationCascade) -> Self {
        match cascade {
            SubagentCancellationCascade::CancelChild => Self::CancelChild,
            SubagentCancellationCascade::ObserveOnly => Self::ObserveOnly,
            SubagentCancellationCascade::QueueCancellation => Self::QueueCancellation,
        }
    }
}

/// UI-neutral sub-agent cancellation projection.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSubagentCancellation {
    pub session_id: SubagentSessionId,
    pub parent_task_id: TaskId,
    pub source_task_id: TaskId,
    pub reason: String,
    pub cascade: ClientSubagentCancellationCascade,
}

impl ClientSubagentCancellation {
    fn from_record(record: &SubagentCancellationRecord) -> Self {
        Self {
            session_id: record.session_id.clone(),
            parent_task_id: record.parent_task_id.clone(),
            source_task_id: record.source_task_id.clone(),
            reason: record.reason.clone(),
            cascade: record.cascade.into(),
        }
    }
}

/// Safe, UI-neutral inspection projection for read-only workflow review metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientWorkflowInspection {
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

/// UI-neutral, read-only projection for coding-agent workflow metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientCodingWorkflow {
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub objective: Option<String>,
    pub active: bool,
    pub workspace_scope: Option<WorkspaceMutationScope>,
    pub mutation_requests: Vec<MutationRequestProposal>,
    pub apply_patch_preflights: Vec<ApplyPatchPreflightRecord>,
    pub apply_patch_executions: Vec<ApplyPatchExecutionRecord>,
    pub patch_proposals: Vec<PatchProposal>,
    pub patch_applications: Vec<PatchApplicationRecord>,
    pub test_plans: Vec<TestPlanRecord>,
    pub test_runs: Vec<TestRunRecord>,
    pub test_evidence_summaries: Vec<TestEvidenceSummaryRecord>,
    pub review_bundles: Vec<ReviewBundle>,
    pub restore_plans: Vec<RestorePlanRecord>,
}

impl ClientCodingWorkflow {
    fn new(workflow_id: CodingWorkflowId, task_id: TaskId) -> Self {
        Self {
            workflow_id,
            task_id,
            objective: None,
            active: true,
            workspace_scope: None,
            mutation_requests: Vec::new(),
            apply_patch_preflights: Vec::new(),
            apply_patch_executions: Vec::new(),
            patch_proposals: Vec::new(),
            patch_applications: Vec::new(),
            test_plans: Vec::new(),
            test_runs: Vec::new(),
            test_evidence_summaries: Vec::new(),
            review_bundles: Vec::new(),
            restore_plans: Vec::new(),
        }
    }

    fn record_start(&mut self, task_id: TaskId, objective: String) {
        self.task_id = task_id;
        self.objective = Some(objective);
        self.active = true;
    }

    fn record_scope(&mut self, scope: &WorkspaceMutationScope) {
        self.task_id = scope.task_id.clone();
        self.workspace_scope = Some(scope.clone());
    }

    fn record_mutation_request(&mut self, proposal: &MutationRequestProposal) {
        self.task_id = proposal.task_id.clone();
        if let Some(existing) = self
            .mutation_requests
            .iter_mut()
            .find(|existing| existing.request_id == proposal.request_id)
        {
            *existing = proposal.clone();
        } else {
            self.mutation_requests.push(proposal.clone());
        }
    }

    fn record_patch_proposal(&mut self, proposal: &PatchProposal) {
        self.task_id = proposal.task_id.clone();
        if let Some(existing) = self
            .patch_proposals
            .iter_mut()
            .find(|existing| existing.patch_id == proposal.patch_id)
        {
            *existing = proposal.clone();
        } else {
            self.patch_proposals.push(proposal.clone());
        }
    }

    fn record_apply_patch_preflight(&mut self, record: &ApplyPatchPreflightRecord) {
        self.task_id = record.task_id.clone();
        if let Some(existing) = self
            .apply_patch_preflights
            .iter_mut()
            .find(|existing| existing.preflight_id == record.preflight_id)
        {
            *existing = record.clone();
        } else {
            self.apply_patch_preflights.push(record.clone());
        }
    }

    fn record_apply_patch_execution(&mut self, record: &ApplyPatchExecutionRecord) {
        self.task_id = record.task_id.clone();
        if let Some(existing) = self
            .apply_patch_executions
            .iter_mut()
            .find(|existing| existing.execution_id == record.execution_id)
        {
            *existing = record.clone();
        } else {
            self.apply_patch_executions.push(record.clone());
        }
    }

    fn record_patch_application(&mut self, record: &PatchApplicationRecord) {
        self.task_id = record.task_id.clone();
        if let Some(existing) = self
            .patch_applications
            .iter_mut()
            .find(|existing| existing.patch_id == record.patch_id)
        {
            *existing = record.clone();
        } else {
            self.patch_applications.push(record.clone());
        }
    }

    fn record_test_plan(&mut self, plan: &TestPlanRecord) {
        self.task_id = plan.task_id.clone();
        if let Some(existing) = self
            .test_plans
            .iter_mut()
            .find(|existing| existing.test_plan_id == plan.test_plan_id)
        {
            *existing = plan.clone();
        } else {
            self.test_plans.push(plan.clone());
        }
    }

    fn record_test_run(&mut self, record: &TestRunRecord) {
        self.task_id = record.task_id.clone();
        if let Some(existing) = self
            .test_runs
            .iter_mut()
            .find(|existing| existing.test_run_id == record.test_run_id)
        {
            *existing = record.clone();
        } else {
            self.test_runs.push(record.clone());
        }
    }

    fn record_test_evidence_summary(&mut self, record: &TestEvidenceSummaryRecord) {
        self.task_id = record.task_id.clone();
        if let Some(existing) = self
            .test_evidence_summaries
            .iter_mut()
            .find(|existing| existing.summary_id == record.summary_id)
        {
            *existing = record.clone();
        } else {
            self.test_evidence_summaries.push(record.clone());
        }
    }

    fn record_review_bundle(&mut self, bundle: &ReviewBundle) {
        self.task_id = bundle.task_id.clone();
        if let Some(existing) = self
            .review_bundles
            .iter_mut()
            .find(|existing| existing.review_bundle_id == bundle.review_bundle_id)
        {
            *existing = bundle.clone();
        } else {
            self.review_bundles.push(bundle.clone());
        }
    }

    fn record_restore_plan(&mut self, plan: &RestorePlanRecord) {
        self.task_id = plan.task_id.clone();
        if let Some(existing) = self
            .restore_plans
            .iter_mut()
            .find(|existing| existing.restore_plan_id == plan.restore_plan_id)
        {
            *existing = plan.clone();
        } else {
            self.restore_plans.push(plan.clone());
        }
    }
}

/// UI-neutral, read-only projection for isolated worktree lifecycle metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientWorktreeLifecycle {
    pub worktree_id: WorkspaceWorktreeId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub trace_id: String,
    pub source_commit: String,
    pub source_branch_label: Option<String>,
    pub worktree_root_label: String,
    pub worktree_base_key: String,
    pub latest_status: WorkspaceWorktreeLifecycleStatus,
    pub latest_reason: Option<String>,
    pub created_for_request_id: Option<tessera_protocol::MutationRequestId>,
    pub created_for_patch_id: Option<tessera_protocol::PatchProposalId>,
    pub evidence: Vec<HandoffEvidenceRef>,
}

impl ClientWorktreeLifecycle {
    fn from_record(record: &WorkspaceWorktreeLifecycleRecord) -> Self {
        Self {
            worktree_id: record.worktree_id.clone(),
            workflow_id: record.workflow_id.clone(),
            task_id: record.task_id.clone(),
            trace_id: record.trace_id.clone(),
            source_commit: record.source_commit.clone(),
            source_branch_label: record.source_branch_label.clone(),
            worktree_root_label: record.worktree_root_label.clone(),
            worktree_base_key: record.worktree_base_key.clone(),
            latest_status: record.lifecycle_status,
            latest_reason: Some(record.reason.clone()),
            created_for_request_id: record.created_for_request_id.clone(),
            created_for_patch_id: record.created_for_patch_id.clone(),
            evidence: record.evidence.clone(),
        }
    }

    fn update_from_record(&mut self, record: &WorkspaceWorktreeLifecycleRecord) {
        self.workflow_id = record.workflow_id.clone();
        self.task_id = record.task_id.clone();
        self.trace_id = record.trace_id.clone();
        self.source_commit = record.source_commit.clone();
        self.source_branch_label = record.source_branch_label.clone();
        self.worktree_root_label = record.worktree_root_label.clone();
        self.worktree_base_key = record.worktree_base_key.clone();
        self.latest_status = record.lifecycle_status;
        self.latest_reason = Some(record.reason.clone());
        self.created_for_request_id = record.created_for_request_id.clone();
        self.created_for_patch_id = record.created_for_patch_id.clone();
        self.evidence = record.evidence.clone();
    }
}

/// Provider-neutral telemetry projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientTelemetrySummary {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cache_miss_tokens: u64,
    pub cache_total_tokens: u64,
    pub latest_context_tokens: Option<u64>,
    pub max_context_tokens: Option<u64>,
    pub estimated_cost: Option<f64>,
    pub cost_currency: Option<String>,
    pub cost_currency_mixed: bool,
}

#[derive(Clone, Copy, Debug)]
struct UsageTelemetryInput<'a> {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    total_tokens: Option<u64>,
    cache_read_tokens: Option<u64>,
    cache_write_tokens: Option<u64>,
    cache_miss_tokens: Option<u64>,
    cost_amount: Option<f64>,
    cost_currency: Option<&'a str>,
}

impl ClientTelemetrySummary {
    fn record_capability(&mut self, max_context_tokens: Option<u64>) {
        if max_context_tokens.is_some() {
            self.max_context_tokens = max_context_tokens;
        }
    }

    fn record_usage(&mut self, usage: UsageTelemetryInput<'_>) {
        if let Some(input_tokens) = usage.input_tokens {
            self.input_tokens = self.input_tokens.saturating_add(input_tokens);
            self.latest_context_tokens = Some(input_tokens);
        }
        if let Some(output_tokens) = usage.output_tokens {
            self.output_tokens = self.output_tokens.saturating_add(output_tokens);
        }
        let reported_total =
            usage
                .total_tokens
                .or_else(|| match (usage.input_tokens, usage.output_tokens) {
                    (Some(input), Some(output)) => Some(input.saturating_add(output)),
                    (Some(input), None) => Some(input),
                    (None, Some(output)) => Some(output),
                    (None, None) => None,
                });
        if let Some(total_tokens) = reported_total {
            self.total_tokens = self.total_tokens.saturating_add(total_tokens);
        }
        if let Some(cache_read_tokens) = usage.cache_read_tokens {
            self.cache_read_tokens = self.cache_read_tokens.saturating_add(cache_read_tokens);
        }
        if let Some(cache_write_tokens) = usage.cache_write_tokens {
            self.cache_write_tokens = self.cache_write_tokens.saturating_add(cache_write_tokens);
        }
        if let Some(cache_miss_tokens) = usage.cache_miss_tokens {
            self.cache_miss_tokens = self.cache_miss_tokens.saturating_add(cache_miss_tokens);
        }
        if usage.cache_read_tokens.is_some() || usage.cache_miss_tokens.is_some() {
            let cache_read_tokens = usage.cache_read_tokens.unwrap_or_default();
            let cache_miss_tokens = usage.cache_miss_tokens.unwrap_or_default();
            let reported_cache_total = cache_read_tokens.saturating_add(cache_miss_tokens);
            let cache_total_tokens =
                if usage.cache_read_tokens.is_some() && usage.cache_miss_tokens.is_none() {
                    usage.input_tokens.unwrap_or(reported_cache_total)
                } else if reported_cache_total > 0 {
                    reported_cache_total
                } else {
                    usage.input_tokens.unwrap_or(reported_cache_total)
                };
            self.cache_total_tokens = self.cache_total_tokens.saturating_add(cache_total_tokens);
        }
        if let (Some(amount), Some(currency)) = (usage.cost_amount, usage.cost_currency) {
            self.record_cost(amount, currency);
        }
    }

    fn record_cost(&mut self, amount: f64, currency: &str) {
        if let Some(existing_currency) = &self.cost_currency {
            if existing_currency == currency && !self.cost_currency_mixed {
                let total = self.estimated_cost.unwrap_or_default() + amount;
                self.estimated_cost = Some(total);
                return;
            }

            self.cost_currency_mixed = true;
            self.estimated_cost = None;
            return;
        }

        self.cost_currency = Some(currency.to_string());
        self.estimated_cost = Some(amount);
    }

    fn usage_summary(&self) -> String {
        format!(
            "usage in {} / out {} / total {}",
            self.input_tokens, self.output_tokens, self.total_tokens
        )
    }

    fn cache_summary(&self) -> String {
        if self.cache_total_tokens == 0 {
            return "cache 0/0".to_string();
        }

        let percentage = self.cache_read_tokens.saturating_mul(100) / self.cache_total_tokens;
        format!(
            "cache {}/{} ({percentage}%)",
            self.cache_read_tokens, self.cache_total_tokens
        )
    }

    fn cost_summary(&self) -> String {
        if self.cost_currency_mixed {
            return "cost mixed".to_string();
        }

        match (self.estimated_cost, &self.cost_currency) {
            (Some(amount), Some(currency)) => format!("{currency} {amount:.4}"),
            _ => "CNY 0.0000".to_string(),
        }
    }

    fn context_summary(&self) -> String {
        let Some(context_tokens) = self.latest_context_tokens else {
            return "ctx 0 tokens".to_string();
        };

        match self.max_context_tokens {
            Some(max_context_tokens) if max_context_tokens > 0 => {
                let percentage = context_tokens.saturating_mul(100) / max_context_tokens;
                format!("ctx {context_tokens}/{max_context_tokens} ({percentage}%)")
            }
            _ => format!("ctx {context_tokens} tokens"),
        }
    }
}

/// UI-neutral status projection shared by terminal and future GUI shells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientStatus {
    pub active_profile: String,
    pub available_profiles: Vec<String>,
    pub reasoning_visible: bool,
    pub task_summary: String,
    pub artifact_summary: String,
    pub approval_summary: String,
    pub memory_summary: String,
    #[serde(default)]
    pub handoff_summary: String,
    #[serde(default)]
    pub coding_workflow_summary: String,
    #[serde(default)]
    pub workflow_inspection_summary: String,
    #[serde(default)]
    pub worktree_summary: String,
    #[serde(default)]
    pub subagent_summary: String,
    #[serde(default)]
    pub subagent_runtime_summary: String,
    #[serde(default)]
    pub subagent_transcript_lifecycle_summary: String,
    pub usage_summary: String,
    pub cache_summary: String,
    pub cost_summary: String,
    pub context_summary: String,
    #[serde(default)]
    pub context_handles_summary: String,
    #[serde(default)]
    pub telemetry: ClientTelemetrySummary,
}

impl ClientStatus {
    pub fn new(active_profile: impl Into<String>) -> Self {
        let active_profile = active_profile.into();
        Self::with_profiles(active_profile.clone(), [active_profile])
    }

    pub fn with_profiles<I, S>(active_profile: impl Into<String>, profiles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let active_profile = active_profile.into();
        let mut available_profiles = Vec::new();
        for profile in profiles {
            let profile = profile.into();
            if !profile.trim().is_empty() && !available_profiles.contains(&profile) {
                available_profiles.push(profile);
            }
        }
        if !available_profiles.contains(&active_profile) {
            available_profiles.insert(0, active_profile.clone());
        }

        Self {
            active_profile,
            available_profiles,
            reasoning_visible: false,
            task_summary: "task idle".to_string(),
            artifact_summary: "artifacts 0".to_string(),
            approval_summary: "approvals 0 pending".to_string(),
            memory_summary: "memory 0 pending".to_string(),
            handoff_summary: "handoffs 0 / reviews 0 pending".to_string(),
            coding_workflow_summary:
                "coding workflows 0 / patches 0 / tests 0 / reviews 0 / mutation requests 0 / blocked restores 0"
                    .to_string(),
            workflow_inspection_summary:
                "inspection workflows 0 / review gates 0 accepted 0 rejected 0 revision_requested 0 pending 0 / approvals 0 pending / diff refs 0"
                    .to_string(),
            worktree_summary: worktree_lifecycle_summary(&[]),
            subagent_summary: "subagents 0 / active 0 / waiting 0 / inactive 0".to_string(),
            subagent_runtime_summary:
                "subagent runtime decisions 0 / transcripts 0 / forwarding queued 0 / inactive require_reviewer 0 / cancellations 0"
                    .to_string(),
            subagent_transcript_lifecycle_summary:
                "transcript lifecycles reserved 0 / published 0 / sealed 0 / abandoned 0"
                    .to_string(),
            usage_summary: "usage in 0 / out 0 / total 0".to_string(),
            cache_summary: "cache 0/0".to_string(),
            cost_summary: "CNY 0.0000".to_string(),
            context_summary: "ctx 0 tokens".to_string(),
            context_handles_summary: context_handles_summary(
                0,
                &ClientContextBudgetSummary::default(),
            ),
            telemetry: ClientTelemetrySummary::default(),
        }
    }

    pub fn active_profile_position(&self) -> (usize, usize) {
        let total = self.available_profiles.len().max(1);
        let index = self
            .available_profiles
            .iter()
            .position(|profile| profile == &self.active_profile)
            .map(|index| index + 1)
            .unwrap_or(1);
        (index, total)
    }

    pub fn cycle_profile(&mut self, offset: isize) -> Option<ClientIntent> {
        let total = self.available_profiles.len();
        if total <= 1 {
            return None;
        }
        let current = self
            .available_profiles
            .iter()
            .position(|profile| profile == &self.active_profile)
            .unwrap_or(0);
        let next = (current as isize + offset).rem_euclid(total as isize) as usize;
        self.active_profile = self.available_profiles[next].clone();
        Some(ClientIntent::SwitchProfile {
            profile_id: self.active_profile.clone(),
        })
    }

    fn update_provider_capability(&mut self, max_context_tokens: Option<u64>) {
        self.telemetry.record_capability(max_context_tokens);
        self.refresh_telemetry_summaries();
    }

    fn update_usage(&mut self, usage: UsageTelemetryInput<'_>) {
        self.telemetry.record_usage(usage);
        self.refresh_telemetry_summaries();
    }

    fn reset_telemetry(&mut self) {
        self.telemetry = ClientTelemetrySummary::default();
        self.refresh_telemetry_summaries();
    }

    fn refresh_telemetry_summaries(&mut self) {
        self.usage_summary = self.telemetry.usage_summary();
        self.cache_summary = self.telemetry.cache_summary();
        self.cost_summary = self.telemetry.cost_summary();
        self.context_summary = self.telemetry.context_summary();
    }

    fn update_task_summary(&mut self, tasks: &[ClientTask]) {
        self.task_summary = latest_task_summary(tasks);
    }

    fn update_artifact_summary(&mut self, artifacts: &[ClientArtifact]) {
        self.artifact_summary = format!("artifacts {}", artifacts.len());
    }

    fn update_approval_summary(&mut self, approvals: &[ClientApproval]) {
        let pending = approvals
            .iter()
            .filter(|approval| approval.status == ClientApprovalStatus::Pending)
            .count();
        self.approval_summary = format!("approvals {pending} pending");
    }

    fn update_memory_summary(&mut self, proposals: &[ClientMemoryProposal]) {
        let pending = proposals
            .iter()
            .filter(|proposal| proposal.status == ClientMemoryProposalStatus::Pending)
            .count();
        self.memory_summary = format!("memory {pending} pending");
    }

    fn update_handoff_summary(
        &mut self,
        handoffs: &[ClientAgentHandoff],
        gates: &[ClientReviewerGate],
    ) {
        let pending = gates
            .iter()
            .filter(|gate| gate.status == ClientReviewerGateStatus::Pending)
            .count();
        self.handoff_summary = format!("handoffs {} / reviews {pending} pending", handoffs.len());
    }

    fn update_coding_workflow_summary(&mut self, workflows: &[ClientCodingWorkflow]) {
        let patches = workflows
            .iter()
            .map(|workflow| workflow.patch_proposals.len())
            .sum::<usize>();
        let tests = workflows
            .iter()
            .map(|workflow| workflow.test_runs.len())
            .sum::<usize>();
        let test_evidence_summaries = workflows
            .iter()
            .map(|workflow| workflow.test_evidence_summaries.len())
            .sum::<usize>();
        let reviews = workflows
            .iter()
            .map(|workflow| workflow.review_bundles.len())
            .sum::<usize>();
        let mutation_requests = workflows
            .iter()
            .map(|workflow| workflow.mutation_requests.len())
            .sum::<usize>();
        let apply_patch_preflights = workflows
            .iter()
            .map(|workflow| workflow.apply_patch_preflights.len())
            .sum::<usize>();
        let apply_patch_executions = workflows
            .iter()
            .map(|workflow| workflow.apply_patch_executions.len())
            .sum::<usize>();
        let blocked_restores = workflows
            .iter()
            .flat_map(|workflow| workflow.restore_plans.iter())
            .filter(|plan| plan.execution_blocked)
            .count();
        self.coding_workflow_summary = format!(
            "coding workflows {} / patches {patches} / tests {tests} / test evidence summaries {test_evidence_summaries} / reviews {reviews} / mutation requests {mutation_requests} / apply-patch preflights {apply_patch_preflights} / apply-patch executions {apply_patch_executions} / blocked restores {blocked_restores}",
            workflows.len()
        );
    }

    fn update_workflow_inspection_summary(
        &mut self,
        inspections: &[ClientWorkflowInspection],
        approvals: &[ClientApproval],
    ) {
        let accepted = inspections
            .iter()
            .map(|inspection| inspection.accepted_reviewer_gate_count)
            .sum::<usize>();
        let rejected = inspections
            .iter()
            .map(|inspection| inspection.rejected_reviewer_gate_count)
            .sum::<usize>();
        let revision_requested = inspections
            .iter()
            .map(|inspection| inspection.revision_requested_reviewer_gate_count)
            .sum::<usize>();
        let pending_gates = inspections
            .iter()
            .map(|inspection| inspection.pending_reviewer_gate_count)
            .sum::<usize>();
        let reviewer_gates = inspections
            .iter()
            .map(|inspection| inspection.reviewer_gate_count)
            .sum::<usize>();
        let diff_refs = inspections
            .iter()
            .map(|inspection| inspection.diff_artifact_ref_count)
            .sum::<usize>();
        let pending_approvals = approvals
            .iter()
            .filter(|approval| approval.status == ClientApprovalStatus::Pending)
            .count();

        self.workflow_inspection_summary = format!(
            "inspection workflows {} / review gates {reviewer_gates} accepted {accepted} rejected {rejected} revision_requested {revision_requested} pending {pending_gates} / approvals {pending_approvals} pending / diff refs {diff_refs}",
            inspections.len()
        );
    }

    fn update_worktree_summary(&mut self, lifecycles: &[ClientWorktreeLifecycle]) {
        self.worktree_summary = worktree_lifecycle_summary(lifecycles);
    }

    fn update_subagent_summary(&mut self, sessions: &[ClientSubagentSession]) {
        let active = sessions
            .iter()
            .filter(|session| session.status == ClientSubagentSessionStatus::Active)
            .count();
        let waiting = sessions
            .iter()
            .filter(|session| session.status == ClientSubagentSessionStatus::WaitingForApproval)
            .count();
        let inactive = sessions
            .iter()
            .filter(|session| session.status == ClientSubagentSessionStatus::Inactive)
            .count();
        self.subagent_summary = format!(
            "subagents {} / active {active} / waiting {waiting} / inactive {inactive}",
            sessions.len()
        );
    }

    fn update_subagent_runtime_summary(
        &mut self,
        decisions: &[ClientSubagentRuntimeDecision],
        transcripts: &[ClientSubagentTranscriptArtifact],
        forwarding: &[ClientSubagentApprovalForwarding],
        inactive: &[ClientSubagentInactivePolicy],
        cancellations: &[ClientSubagentCancellation],
    ) {
        let forwarding_queued = forwarding
            .iter()
            .filter(|record| {
                record.status == ClientSubagentApprovalForwardingStatus::QueuedForReviewer
            })
            .count();
        let inactive_require_reviewer = inactive
            .iter()
            .filter(|record| record.policy.as_deref() == Some("require_reviewer"))
            .count();
        self.subagent_runtime_summary = format!(
            "subagent runtime decisions {} / transcripts {} / forwarding queued {forwarding_queued} / inactive require_reviewer {inactive_require_reviewer} / cancellations {}",
            decisions.len(),
            transcripts.len(),
            cancellations.len()
        );
    }

    fn update_subagent_transcript_lifecycle_summary(
        &mut self,
        lifecycles: &[ClientSubagentTranscriptArtifactLifecycle],
    ) {
        let reserved = lifecycles
            .iter()
            .filter(|record| record.status == ClientSubagentTranscriptArtifactStatus::Reserved)
            .count();
        let published = lifecycles
            .iter()
            .filter(|record| record.status == ClientSubagentTranscriptArtifactStatus::Published)
            .count();
        let sealed = lifecycles
            .iter()
            .filter(|record| record.status == ClientSubagentTranscriptArtifactStatus::Sealed)
            .count();
        let abandoned = lifecycles
            .iter()
            .filter(|record| record.status == ClientSubagentTranscriptArtifactStatus::Abandoned)
            .count();
        self.subagent_transcript_lifecycle_summary = format!(
            "transcript lifecycles reserved {reserved} / published {published} / sealed {sealed} / abandoned {abandoned}"
        );
    }

    fn update_context_handles_summary(
        &mut self,
        handles: &[ClientContextHandle],
        summary: &ClientContextBudgetSummary,
    ) {
        self.context_handles_summary = context_handles_summary(handles.len(), summary);
    }
}

/// UI-neutral message projection built from runtime events or trace records.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientProjection {
    pub messages: Vec<ClientMessage>,
    pub reasoning_visible: bool,
}

impl ClientProjection {
    pub fn new(_active_profile: impl Into<String>) -> Self {
        Self {
            messages: Vec::new(),
            reasoning_visible: false,
        }
    }

    pub fn apply_event(&mut self, frame: &EventFrame) {
        match &frame.event {
            RunEvent::UserMessageRecorded { item_id, text } => {
                self.messages.push(ClientMessage {
                    role: ClientMessageRole::User,
                    content: text.clone(),
                    item_id: Some(item_id.clone()),
                    streaming: false,
                });
            }
            RunEvent::AssistantMessageStarted { item_id } => {
                self.push_empty_streaming_message(ClientMessageRole::Assistant, item_id.clone());
            }
            RunEvent::AssistantDelta { item_id, text } => {
                self.append_to_streaming_message(ClientMessageRole::Assistant, item_id, text);
            }
            RunEvent::AssistantReasoningDelta { item_id, text } if self.reasoning_visible => {
                self.append_to_streaming_message(ClientMessageRole::Reasoning, item_id, text);
            }
            RunEvent::AssistantMessageCompleted { item_id } => {
                self.complete_assistant_item(item_id);
            }
            _ => {}
        }
    }

    pub fn apply_trace_record(&mut self, record: &TraceRecord) {
        let item_id = trace_record_item_id(record);
        match record.event_kind.as_str() {
            "user_message_recorded" => {
                let Some(text) = record.payload.get("text").and_then(|value| value.as_str()) else {
                    return;
                };
                self.messages.push(ClientMessage {
                    role: ClientMessageRole::User,
                    content: text.to_string(),
                    item_id,
                    streaming: false,
                });
            }
            "assistant_message_started" => {
                let Some(item_id) = item_id else {
                    return;
                };
                self.push_empty_streaming_message(ClientMessageRole::Assistant, item_id);
            }
            "assistant_delta" => {
                let (Some(item_id), Some(text)) = (
                    item_id.as_ref(),
                    record.payload.get("text").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                self.append_to_streaming_message(ClientMessageRole::Assistant, item_id, text);
            }
            "assistant_reasoning_delta" if self.reasoning_visible => {
                let (Some(item_id), Some(text)) = (
                    item_id.as_ref(),
                    record.payload.get("text").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                self.append_to_streaming_message(ClientMessageRole::Reasoning, item_id, text);
            }
            "assistant_message_completed" => {
                let Some(item_id) = item_id else {
                    return;
                };
                self.complete_assistant_item(&item_id);
            }
            _ => {}
        }
    }

    fn push_empty_streaming_message(&mut self, role: ClientMessageRole, item_id: ItemId) {
        self.messages.push(ClientMessage {
            role,
            content: String::new(),
            item_id: Some(item_id),
            streaming: true,
        });
    }

    fn append_to_streaming_message(
        &mut self,
        role: ClientMessageRole,
        item_id: &ItemId,
        text: &str,
    ) {
        if let Some(message) = self.message_by_item_id_and_role_mut(item_id, &role) {
            message.content.push_str(text);
            message.streaming = true;
            return;
        }

        self.messages.push(ClientMessage {
            role,
            content: text.to_string(),
            item_id: Some(item_id.clone()),
            streaming: true,
        });
    }

    fn message_by_item_id_and_role_mut(
        &mut self,
        item_id: &ItemId,
        role: &ClientMessageRole,
    ) -> Option<&mut ClientMessage> {
        self.messages
            .iter_mut()
            .rev()
            .find(|message| message.item_id.as_ref() == Some(item_id) && message.role == *role)
    }

    fn complete_assistant_item(&mut self, item_id: &ItemId) {
        for message in self.messages.iter_mut().filter(|message| {
            message.item_id.as_ref() == Some(item_id)
                && matches!(
                    message.role,
                    ClientMessageRole::Assistant | ClientMessageRole::Reasoning
                )
        }) {
            message.streaming = false;
        }
    }
}

/// Complete client-side snapshot for a shell render pass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(ts_rs::TS))]
pub struct ClientSnapshot {
    pub status: ClientStatus,
    pub projection: ClientProjection,
    pub tasks: Vec<ClientTask>,
    pub artifacts: Vec<ClientArtifact>,
    pub approvals: Vec<ClientApproval>,
    pub memory_proposals: Vec<ClientMemoryProposal>,
    #[serde(default)]
    pub handoffs: Vec<ClientAgentHandoff>,
    #[serde(default)]
    pub reviewer_gates: Vec<ClientReviewerGate>,
    #[serde(default)]
    pub coding_workflows: Vec<ClientCodingWorkflow>,
    #[serde(default)]
    pub workflow_inspections: Vec<ClientWorkflowInspection>,
    #[serde(default)]
    pub worktree_lifecycles: Vec<ClientWorktreeLifecycle>,
    #[serde(default)]
    pub subagent_sessions: Vec<ClientSubagentSession>,
    #[serde(default)]
    pub subagent_runtime_decisions: Vec<ClientSubagentRuntimeDecision>,
    #[serde(default)]
    pub subagent_transcripts: Vec<ClientSubagentTranscriptArtifact>,
    #[serde(default)]
    pub subagent_transcript_lifecycles: Vec<ClientSubagentTranscriptArtifactLifecycle>,
    #[serde(default)]
    pub subagent_approval_forwarding: Vec<ClientSubagentApprovalForwarding>,
    #[serde(default)]
    pub subagent_inactive_policies: Vec<ClientSubagentInactivePolicy>,
    #[serde(default)]
    pub subagent_cancellations: Vec<ClientSubagentCancellation>,
    #[serde(default)]
    pub context_handles: Vec<ClientContextHandle>,
    pub draft_input: String,
}

impl ClientSnapshot {
    pub fn new(active_profile: impl Into<String>) -> Self {
        let active_profile = active_profile.into();
        Self::with_profiles(active_profile.clone(), [active_profile])
    }

    pub fn with_profiles<I, S>(active_profile: impl Into<String>, profiles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let active_profile = active_profile.into();
        Self {
            status: ClientStatus::with_profiles(active_profile.clone(), profiles),
            projection: ClientProjection::new(active_profile),
            tasks: Vec::new(),
            artifacts: Vec::new(),
            approvals: Vec::new(),
            memory_proposals: Vec::new(),
            handoffs: Vec::new(),
            reviewer_gates: Vec::new(),
            coding_workflows: Vec::new(),
            workflow_inspections: Vec::new(),
            worktree_lifecycles: Vec::new(),
            subagent_sessions: Vec::new(),
            subagent_runtime_decisions: Vec::new(),
            subagent_transcripts: Vec::new(),
            subagent_transcript_lifecycles: Vec::new(),
            subagent_approval_forwarding: Vec::new(),
            subagent_inactive_policies: Vec::new(),
            subagent_cancellations: Vec::new(),
            context_handles: Vec::new(),
            draft_input: String::new(),
        }
    }

    pub fn set_input(&mut self, input: impl Into<String>) {
        self.draft_input = input.into();
    }

    pub fn submit_input(&mut self) -> Option<ClientIntent> {
        let prompt = self.draft_input.trim().to_string();
        if prompt.is_empty() {
            return None;
        }
        self.draft_input.clear();
        match prompt.as_str() {
            "/new" => Some(ClientIntent::NewThread),
            "/save" => Some(ClientIntent::SaveThread),
            "/export" => Some(ClientIntent::ExportThread),
            "/cancel" => Some(ClientIntent::CancelTask {
                task_id: self.active_cancellable_task_id(),
            }),
            "/pause" => Some(ClientIntent::PauseTask {
                task_id: self.active_pausable_task_id(),
            }),
            "/resume-task" => None,
            _ if prompt.starts_with("/pause ") => pause_intent(&prompt),
            _ if prompt.starts_with("/resume-task ") => resume_intent(&prompt),
            _ if prompt.starts_with("/approve ") => approval_intent(&prompt, "/approve ", true),
            _ if prompt.starts_with("/deny ") => approval_intent(&prompt, "/deny ", false),
            _ if prompt.starts_with("/remember ") => memory_intent(&prompt, "/remember ", true),
            _ if prompt.starts_with("/forget ") => memory_intent(&prompt, "/forget ", false),
            _ => Some(ClientIntent::SubmitPrompt {
                profile_id: self.status.active_profile.clone(),
                prompt,
            }),
        }
    }

    pub fn active_profile_position(&self) -> (usize, usize) {
        self.status.active_profile_position()
    }

    pub fn active_cancellable_task_id(&self) -> Option<TaskId> {
        self.tasks
            .iter()
            .rev()
            .find(|task| task.status == TaskStatus::Running)
            .map(|task| task.task_id.clone())
    }

    fn active_pausable_task_id(&self) -> Option<TaskId> {
        self.tasks
            .iter()
            .rev()
            .find(|task| task.status == TaskStatus::Running)
            .map(|task| task.task_id.clone())
    }

    pub fn cycle_profile(&mut self, offset: isize) -> Option<ClientIntent> {
        self.status.cycle_profile(offset)
    }

    pub fn set_context_handles<I>(&mut self, references: I, summary: ClientContextBudgetSummary)
    where
        I: IntoIterator<Item = ContextReference>,
    {
        self.context_handles = references
            .into_iter()
            .map(ClientContextHandle::from_reference)
            .collect();
        self.status
            .update_context_handles_summary(&self.context_handles, &summary);
    }

    pub fn apply_event(&mut self, frame: &EventFrame) {
        match &frame.event {
            RunEvent::TaskCreated { task_id, kind } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.kind = Some(kind.clone());
                task.status = TaskStatus::Pending;
                task.created_at = Some(timestamp);
                task.finished_at = None;
                task.cancel_reason = None;
                task.error_code = None;
                task.error_message = None;
                task.update_scope(thread_id, turn_id);
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskStarted { task_id } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Running;
                task.started_at = Some(timestamp);
                task.update_scope(thread_id, turn_id);
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskCompleted { task_id } => {
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Completed;
                task.finished_at = Some(timestamp);
                task.apply_terminal_projection_to_owner();
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskFailed { task_id, error } => {
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Failed;
                task.finished_at = Some(timestamp);
                task.error_code = Some(error.code.clone());
                task.error_message = Some(error.message.clone());
                task.apply_terminal_projection_to_owner();
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskCancelled { task_id, reason } => {
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Cancelled;
                task.finished_at = Some(timestamp);
                task.cancel_reason = reason.clone();
                task.apply_terminal_projection_to_owner();
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskPaused { task_id, .. } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Paused;
                task.update_scope(thread_id, turn_id);
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::TaskResumed { task_id, .. } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let timestamp = frame.timestamp.clone();
                let task = self.task_mut_or_insert(task_id);
                task.status = TaskStatus::Running;
                if task.started_at.is_none() {
                    task.started_at = Some(timestamp);
                }
                task.finished_at = None;
                task.update_scope(thread_id, turn_id);
                self.status.update_task_summary(&self.tasks);
            }
            RunEvent::ArtifactCreated { artifact_id, kind } => {
                let thread_id = frame.thread_id.clone();
                let turn_id = frame.turn_id.clone();
                let task_id = frame.task_id.clone();
                let item_id = frame.item_id.clone();
                let timestamp = frame.timestamp.clone();
                let artifact = self.artifact_mut_or_insert(artifact_id);
                artifact.kind = Some(kind.clone());
                artifact.created_at = Some(timestamp);
                artifact.update_scope(thread_id, turn_id, task_id, item_id);
                self.status.update_artifact_summary(&self.artifacts);
            }
            RunEvent::ToolPolicyDecisionRecorded { decision } => {
                if let Some(approval_id) = &decision.approval_id {
                    self.record_pending_approval(decision, approval_id.clone());
                }
            }
            RunEvent::ToolCallApproved { approval } | RunEvent::ToolCallDenied { approval } => {
                self.record_resolved_approval(approval);
            }
            RunEvent::MemoryWriteProposed { proposal }
            | RunEvent::MemoryWriteApplied { proposal }
            | RunEvent::MemoryWriteRejected { proposal } => {
                self.record_memory_proposal(proposal);
            }
            RunEvent::AgentHandoffRecorded { summary } => {
                self.record_handoff_summary(summary);
            }
            RunEvent::ReviewerGateRequested { request } => {
                self.record_reviewer_gate_request(request);
            }
            RunEvent::ReviewerGateResolved { decision } => {
                self.record_reviewer_gate_decision(decision);
            }
            RunEvent::CodingWorkflowStarted {
                workflow_id,
                task_id,
                objective,
            } => {
                self.record_coding_workflow_started(workflow_id, task_id, objective);
            }
            RunEvent::WorkspaceMutationScopeRecorded { scope } => {
                self.record_coding_workflow_scope(scope);
            }
            RunEvent::MutationRequestProposalRecorded { proposal } => {
                self.record_coding_workflow_mutation_request(proposal);
            }
            RunEvent::ApplyPatchPreflightRecorded { record } => {
                self.record_coding_workflow_apply_patch_preflight(record);
            }
            RunEvent::ApplyPatchExecutionRecorded { record } => {
                self.record_coding_workflow_apply_patch_execution(record);
            }
            RunEvent::PatchProposalRecorded { proposal } => {
                self.record_coding_workflow_patch_proposal(proposal);
            }
            RunEvent::PatchApplicationRecorded { record } => {
                self.record_coding_workflow_patch_application(record);
            }
            RunEvent::TestPlanRecorded { plan } => {
                self.record_coding_workflow_test_plan(plan);
            }
            RunEvent::TestRunRecorded { record } => {
                self.record_coding_workflow_test_run(record);
            }
            RunEvent::TestEvidenceSummaryRecorded { record } => {
                self.record_coding_workflow_test_evidence_summary(record);
            }
            RunEvent::ReviewBundleRecorded { bundle } => {
                self.record_coding_workflow_review_bundle(bundle);
            }
            RunEvent::RestorePlanRecorded { plan } => {
                self.record_coding_workflow_restore_plan(plan);
            }
            RunEvent::WorkspaceWorktreeLifecycleRecorded { record } => {
                self.record_worktree_lifecycle(record);
            }
            RunEvent::SubagentSessionPlanned { session }
            | RunEvent::SubagentSessionStarted { session }
            | RunEvent::SubagentSessionWaitingForApproval { session }
            | RunEvent::SubagentSessionInactive { session }
            | RunEvent::SubagentSessionCompleted { session } => {
                self.record_subagent_session(session);
            }
            RunEvent::SubagentRuntimeDecisionRecorded { decision } => {
                self.record_subagent_runtime_decision(decision);
            }
            RunEvent::SubagentTranscriptArtifactRecorded { transcript } => {
                self.record_subagent_transcript(transcript);
            }
            RunEvent::SubagentTranscriptArtifactLifecycleRecorded { lifecycle } => {
                self.record_subagent_transcript_lifecycle(lifecycle);
            }
            RunEvent::SubagentApprovalForwardingRecorded { forwarding } => {
                self.record_subagent_approval_forwarding(forwarding);
            }
            RunEvent::SubagentInactivePolicyRecorded { inactive } => {
                self.record_subagent_inactive_policy(inactive);
            }
            RunEvent::SubagentCancellationRecorded { cancellation } => {
                self.record_subagent_cancellation(cancellation);
            }
            RunEvent::TaskOwnerAttached { lease } => {
                let task = self.task_mut_or_insert(&lease.task_id);
                task.apply_owner_lease(lease);
            }
            RunEvent::TaskOwnerHeartbeat { heartbeat } => {
                let task = self.task_mut_or_insert(&heartbeat.task_id);
                task.apply_owner_heartbeat(heartbeat);
            }
            RunEvent::TaskOwnerDetached {
                lease_id,
                task_id,
                reason,
            } => {
                let task = self.task_mut_or_insert(task_id);
                task.apply_owner_status(
                    lease_id.clone(),
                    TaskOwnerStatus::Detached,
                    reason.clone(),
                );
            }
            RunEvent::TaskOwnerLost {
                lease_id,
                task_id,
                reason,
            } => {
                let task = self.task_mut_or_insert(task_id);
                task.apply_owner_status(lease_id.clone(), TaskOwnerStatus::Lost, reason.clone());
            }
            RunEvent::TaskReattachRecorded { record } => {
                let task = self.task_mut_or_insert(&record.task_id);
                task.apply_reattach_record(record);
            }
            RunEvent::ProviderCapabilityReported { capability, .. } => self
                .status
                .update_provider_capability(capability.max_context_tokens),
            RunEvent::UsageReported {
                input_tokens,
                output_tokens,
                total_tokens,
                cache_read_tokens,
                cache_write_tokens,
                cache_miss_tokens,
                estimated_cost,
                ..
            } => {
                self.status.update_usage(UsageTelemetryInput {
                    input_tokens: *input_tokens,
                    output_tokens: *output_tokens,
                    total_tokens: *total_tokens,
                    cache_read_tokens: *cache_read_tokens,
                    cache_write_tokens: *cache_write_tokens,
                    cache_miss_tokens: *cache_miss_tokens,
                    cost_amount: estimated_cost.as_ref().map(|cost| cost.amount),
                    cost_currency: estimated_cost.as_ref().map(|cost| cost.currency.as_str()),
                });
            }
            _ => {}
        }
        self.apply_artifact_refs_from_frame(frame);
        self.projection.reasoning_visible = self.status.reasoning_visible;
        self.projection.apply_event(frame);
    }

    pub fn apply_trace_record(&mut self, record: &TraceRecord) {
        match record.event_kind.as_str() {
            "task_created" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let kind = record
                    .payload
                    .get("kind")
                    .and_then(|value| value.as_str())
                    .and_then(TaskKind::from_snake_case);
                let task = self.task_mut_or_insert(&task_id);
                task.kind = kind;
                task.status = TaskStatus::Pending;
                task.created_at = Some(record.timestamp.clone());
                task.finished_at = None;
                task.cancel_reason = None;
                task.error_code = None;
                task.error_message = None;
                task.update_scope(record.thread_id.clone(), record.turn_id.clone());
                self.status.update_task_summary(&self.tasks);
            }
            "task_started" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Running;
                task.started_at = Some(record.timestamp.clone());
                task.update_scope(record.thread_id.clone(), record.turn_id.clone());
                self.status.update_task_summary(&self.tasks);
            }
            "task_completed" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Completed;
                task.finished_at = Some(record.timestamp.clone());
                task.apply_terminal_projection_to_owner();
                self.status.update_task_summary(&self.tasks);
            }
            "task_failed" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Failed;
                task.finished_at = Some(record.timestamp.clone());
                task.error_code = record
                    .payload
                    .get("error")
                    .and_then(|error| error.get("code"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                task.error_message = record
                    .payload
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                task.apply_terminal_projection_to_owner();
                self.status.update_task_summary(&self.tasks);
            }
            "task_cancelled" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Cancelled;
                task.finished_at = Some(record.timestamp.clone());
                task.cancel_reason = record
                    .payload
                    .get("reason")
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                task.apply_terminal_projection_to_owner();
                self.status.update_task_summary(&self.tasks);
            }
            "task_paused" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Paused;
                task.update_scope(record.thread_id.clone(), record.turn_id.clone());
                self.status.update_task_summary(&self.tasks);
            }
            "task_resumed" => {
                let Some(task_id) = trace_record_task_id(record) else {
                    return;
                };
                let task = self.task_mut_or_insert(&task_id);
                task.status = TaskStatus::Running;
                if task.started_at.is_none() {
                    task.started_at = Some(record.timestamp.clone());
                }
                task.finished_at = None;
                task.update_scope(record.thread_id.clone(), record.turn_id.clone());
                self.status.update_task_summary(&self.tasks);
            }
            "artifact_created" => {
                let Some(artifact_id) = trace_record_artifact_id(record) else {
                    return;
                };
                let kind = record
                    .payload
                    .get("kind")
                    .and_then(|value| value.as_str())
                    .and_then(ArtifactKind::from_snake_case);
                let artifact = self.artifact_mut_or_insert(&artifact_id);
                artifact.kind = kind;
                artifact.created_at = Some(record.timestamp.clone());
                artifact.update_scope(
                    record.thread_id.clone(),
                    record.turn_id.clone(),
                    record.task_id.clone(),
                    record.item_id.clone(),
                );
                self.status.update_artifact_summary(&self.artifacts);
            }
            "tool_policy_decision_recorded" => {
                let Some(decision) = record.payload.get("decision") else {
                    return;
                };
                let (Some(approval_id), Some(call_id), Some(tool_id)) = (
                    decision.get("approval_id").and_then(|value| value.as_str()),
                    decision.get("call_id").and_then(|value| value.as_str()),
                    decision.get("tool_id").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                let reason = decision
                    .get("reason")
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                let required_permissions = decision
                    .get("required_permissions")
                    .and_then(|value| value.as_array())
                    .map(|values| string_values(values))
                    .unwrap_or_default();
                let side_effects = decision
                    .get("side_effects")
                    .and_then(|value| value.as_array())
                    .map(|values| string_values(values))
                    .unwrap_or_default();
                self.record_pending_approval_parts(PendingApprovalParts {
                    approval_id: ApprovalId::from(approval_id),
                    call_id: ToolCallId::from(call_id),
                    tool_id: ToolId::from(tool_id),
                    reason,
                    required_permissions,
                    side_effects,
                });
            }
            "tool_call_approved" | "tool_call_denied" => {
                let Some(approval) = record.payload.get("approval") else {
                    return;
                };
                let (Some(approval_id), Some(call_id), Some(tool_id)) = (
                    approval.get("approval_id").and_then(|value| value.as_str()),
                    approval.get("call_id").and_then(|value| value.as_str()),
                    approval.get("tool_id").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                let status = approval
                    .get("status")
                    .and_then(|value| value.as_str())
                    .and_then(client_approval_status_from_str)
                    .unwrap_or(if record.event_kind == "tool_call_approved" {
                        ClientApprovalStatus::Approved
                    } else {
                        ClientApprovalStatus::Denied
                    });
                let reason = approval
                    .get("reason")
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                self.record_resolved_approval_parts(ResolvedApprovalParts {
                    approval_id: ApprovalId::from(approval_id),
                    call_id: ToolCallId::from(call_id),
                    tool_id: ToolId::from(tool_id),
                    status,
                    reason,
                });
            }
            "memory_write_proposed" | "memory_write_applied" | "memory_write_rejected" => {
                let Some(proposal) = record.payload.get("proposal") else {
                    return;
                };
                let (Some(proposal_id), Some(title), Some(summary)) = (
                    proposal.get("proposal_id").and_then(|value| value.as_str()),
                    proposal.get("title").and_then(|value| value.as_str()),
                    proposal.get("summary").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                let status = proposal
                    .get("status")
                    .and_then(|value| value.as_str())
                    .and_then(client_memory_proposal_status_from_str)
                    .unwrap_or(match record.event_kind.as_str() {
                        "memory_write_applied" => ClientMemoryProposalStatus::Applied,
                        "memory_write_rejected" => ClientMemoryProposalStatus::Rejected,
                        _ => ClientMemoryProposalStatus::Pending,
                    });
                let source_item_id = proposal
                    .get("source_item_id")
                    .and_then(|value| value.as_str())
                    .map(ItemId::from);
                let reason = proposal
                    .get("reason")
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                self.record_memory_proposal_parts(MemoryProposalParts {
                    proposal_id: MemoryProposalId::from(proposal_id),
                    status,
                    title: title.to_string(),
                    summary: summary.to_string(),
                    source_item_id,
                    reason,
                });
            }
            "agent_handoff_recorded" => {
                let Some(summary) =
                    trace_payload::<AgentHandoffSummary>(record.payload.get("summary"))
                else {
                    return;
                };
                self.record_handoff_summary(&summary);
            }
            "reviewer_gate_requested" => {
                let Some(request) =
                    trace_payload::<ReviewerGateRequest>(record.payload.get("request"))
                else {
                    return;
                };
                self.record_reviewer_gate_request(&request);
            }
            "reviewer_gate_resolved" => {
                let Some(decision) =
                    trace_payload::<ReviewerGateDecision>(record.payload.get("decision"))
                else {
                    return;
                };
                self.record_reviewer_gate_decision(&decision);
            }
            "coding_workflow_started" => {
                let (Some(workflow_id), Some(task_id), Some(objective)) = (
                    trace_record_coding_workflow_id(record),
                    trace_record_task_id(record),
                    record
                        .payload
                        .get("objective")
                        .and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                self.record_coding_workflow_started(&workflow_id, &task_id, objective);
            }
            "workspace_mutation_scope_recorded" => {
                let Some(scope) =
                    trace_payload::<WorkspaceMutationScope>(record.payload.get("scope"))
                else {
                    return;
                };
                self.record_coding_workflow_scope(&scope);
            }
            "mutation_request_proposal_recorded" => {
                let Some(proposal) =
                    trace_payload::<MutationRequestProposal>(record.payload.get("proposal"))
                else {
                    return;
                };
                self.record_coding_workflow_mutation_request(&proposal);
            }
            "apply_patch_preflight_recorded" => {
                let Some(record) =
                    trace_payload::<ApplyPatchPreflightRecord>(record.payload.get("record"))
                else {
                    return;
                };
                self.record_coding_workflow_apply_patch_preflight(&record);
            }
            "apply_patch_execution_recorded" => {
                let Some(record) =
                    trace_payload::<ApplyPatchExecutionRecord>(record.payload.get("record"))
                else {
                    return;
                };
                self.record_coding_workflow_apply_patch_execution(&record);
            }
            "patch_proposal_recorded" => {
                let Some(proposal) = trace_payload::<PatchProposal>(record.payload.get("proposal"))
                else {
                    return;
                };
                self.record_coding_workflow_patch_proposal(&proposal);
            }
            "patch_application_recorded" => {
                let Some(record) =
                    trace_payload::<PatchApplicationRecord>(record.payload.get("record"))
                else {
                    return;
                };
                self.record_coding_workflow_patch_application(&record);
            }
            "test_plan_recorded" => {
                let Some(plan) = trace_payload::<TestPlanRecord>(record.payload.get("plan")) else {
                    return;
                };
                self.record_coding_workflow_test_plan(&plan);
            }
            "test_run_recorded" => {
                let Some(record) = trace_payload::<TestRunRecord>(record.payload.get("record"))
                else {
                    return;
                };
                self.record_coding_workflow_test_run(&record);
            }
            "test_evidence_summary_recorded" => {
                let Some(record) =
                    trace_payload::<TestEvidenceSummaryRecord>(record.payload.get("record"))
                else {
                    return;
                };
                self.record_coding_workflow_test_evidence_summary(&record);
            }
            "review_bundle_recorded" => {
                let Some(bundle) = trace_payload::<ReviewBundle>(record.payload.get("bundle"))
                else {
                    return;
                };
                self.record_coding_workflow_review_bundle(&bundle);
            }
            "restore_plan_recorded" => {
                let Some(plan) = trace_payload::<RestorePlanRecord>(record.payload.get("plan"))
                else {
                    return;
                };
                self.record_coding_workflow_restore_plan(&plan);
            }
            "workspace_worktree_lifecycle_recorded" => {
                let Some(record) =
                    trace_payload::<WorkspaceWorktreeLifecycleRecord>(record.payload.get("record"))
                else {
                    return;
                };
                self.record_worktree_lifecycle(&record);
            }
            "subagent_session_planned"
            | "subagent_session_started"
            | "subagent_session_waiting_for_approval"
            | "subagent_session_inactive"
            | "subagent_session_completed" => {
                let Some(session) =
                    trace_payload::<SubagentSessionDescriptor>(record.payload.get("session"))
                else {
                    return;
                };
                self.record_subagent_session(&session);
            }
            "subagent_runtime_decision_recorded" => {
                let Some(decision) =
                    trace_payload::<SubagentRuntimeDecision>(record.payload.get("decision"))
                else {
                    return;
                };
                self.record_subagent_runtime_decision(&decision);
            }
            "subagent_transcript_artifact_recorded" => {
                let Some(transcript) = trace_payload::<SubagentTranscriptArtifactRecord>(
                    record.payload.get("transcript"),
                ) else {
                    return;
                };
                self.record_subagent_transcript(&transcript);
            }
            "subagent_transcript_artifact_lifecycle_recorded" => {
                let Some(lifecycle) = trace_payload::<SubagentTranscriptArtifactLifecycleRecord>(
                    record.payload.get("lifecycle"),
                ) else {
                    return;
                };
                self.record_subagent_transcript_lifecycle(&lifecycle);
            }
            "subagent_approval_forwarding_recorded" => {
                let Some(forwarding) = trace_payload::<SubagentApprovalForwardingRecord>(
                    record.payload.get("forwarding"),
                ) else {
                    return;
                };
                self.record_subagent_approval_forwarding(&forwarding);
            }
            "subagent_inactive_policy_recorded" => {
                let Some(inactive) =
                    trace_payload::<SubagentInactivePolicyRecord>(record.payload.get("inactive"))
                else {
                    return;
                };
                self.record_subagent_inactive_policy(&inactive);
            }
            "subagent_cancellation_recorded" => {
                let Some(cancellation) =
                    trace_payload::<SubagentCancellationRecord>(record.payload.get("cancellation"))
                else {
                    return;
                };
                self.record_subagent_cancellation(&cancellation);
            }
            "task_owner_attached" => {
                let Some(lease) = trace_payload::<TaskOwnerLease>(record.payload.get("lease"))
                else {
                    return;
                };
                let task = self.task_mut_or_insert(&lease.task_id);
                task.apply_owner_lease(&lease);
            }
            "task_owner_heartbeat" => {
                let Some(heartbeat) =
                    trace_payload::<TaskOwnerHeartbeat>(record.payload.get("heartbeat"))
                else {
                    return;
                };
                let task = self.task_mut_or_insert(&heartbeat.task_id);
                task.apply_owner_heartbeat(&heartbeat);
            }
            "task_owner_detached" | "task_owner_lost" => {
                let (Some(task_id), Some(lease_id)) = (
                    trace_record_task_id(record),
                    trace_record_task_ownership_id(record),
                ) else {
                    return;
                };
                let reason = record
                    .payload
                    .get("reason")
                    .and_then(|value| value.as_str())
                    .map(str::to_string);
                let status = if record.event_kind == "task_owner_detached" {
                    TaskOwnerStatus::Detached
                } else {
                    TaskOwnerStatus::Lost
                };
                let task = self.task_mut_or_insert(&task_id);
                task.apply_owner_status(lease_id, status, reason);
            }
            "task_reattach_recorded" => {
                let Some(reattach) =
                    trace_payload::<TaskReattachRecord>(record.payload.get("record"))
                else {
                    return;
                };
                let task = self.task_mut_or_insert(&reattach.task_id);
                task.apply_reattach_record(&reattach);
            }
            "provider_capability_reported" => self.status.update_provider_capability(
                record
                    .payload
                    .get("capability")
                    .and_then(|value| value.get("max_context_tokens"))
                    .and_then(|value| value.as_u64()),
            ),
            "usage_reported" => {
                let estimated_cost = record.payload.get("estimated_cost");
                self.status.update_usage(UsageTelemetryInput {
                    input_tokens: record
                        .payload
                        .get("input_tokens")
                        .and_then(|value| value.as_u64()),
                    output_tokens: record
                        .payload
                        .get("output_tokens")
                        .and_then(|value| value.as_u64()),
                    total_tokens: record
                        .payload
                        .get("total_tokens")
                        .and_then(|value| value.as_u64()),
                    cache_read_tokens: record
                        .payload
                        .get("cache_read_tokens")
                        .and_then(|value| value.as_u64()),
                    cache_write_tokens: record
                        .payload
                        .get("cache_write_tokens")
                        .and_then(|value| value.as_u64()),
                    cache_miss_tokens: record
                        .payload
                        .get("cache_miss_tokens")
                        .and_then(|value| value.as_u64()),
                    cost_amount: estimated_cost
                        .and_then(|value| value.get("amount"))
                        .and_then(|value| value.as_f64()),
                    cost_currency: estimated_cost
                        .and_then(|value| value.get("currency"))
                        .and_then(|value| value.as_str()),
                });
            }
            _ => {}
        }
        self.apply_artifact_refs_from_record(record);
        self.projection.reasoning_visible = self.status.reasoning_visible;
        self.projection.apply_trace_record(record);
    }

    pub fn start_new_thread(&mut self) {
        self.projection = ClientProjection::new(self.status.active_profile.clone());
        self.tasks.clear();
        self.artifacts.clear();
        self.approvals.clear();
        self.memory_proposals.clear();
        self.handoffs.clear();
        self.reviewer_gates.clear();
        self.coding_workflows.clear();
        self.workflow_inspections.clear();
        self.worktree_lifecycles.clear();
        self.subagent_sessions.clear();
        self.subagent_runtime_decisions.clear();
        self.subagent_transcripts.clear();
        self.subagent_transcript_lifecycles.clear();
        self.subagent_approval_forwarding.clear();
        self.subagent_inactive_policies.clear();
        self.subagent_cancellations.clear();
        self.context_handles.clear();
        self.draft_input.clear();
        self.status.reset_telemetry();
        self.status.update_task_summary(&self.tasks);
        self.status.update_artifact_summary(&self.artifacts);
        self.status.update_approval_summary(&self.approvals);
        self.status.update_memory_summary(&self.memory_proposals);
        self.status
            .update_handoff_summary(&self.handoffs, &self.reviewer_gates);
        self.status
            .update_coding_workflow_summary(&self.coding_workflows);
        self.status
            .update_workflow_inspection_summary(&self.workflow_inspections, &self.approvals);
        self.status
            .update_worktree_summary(&self.worktree_lifecycles);
        self.status.update_subagent_summary(&self.subagent_sessions);
        self.refresh_subagent_runtime_summary();
        self.status
            .update_subagent_transcript_lifecycle_summary(&self.subagent_transcript_lifecycles);
        self.status.update_context_handles_summary(
            &self.context_handles,
            &ClientContextBudgetSummary::default(),
        );
    }

    pub fn push_notice(&mut self, content: impl Into<String>) {
        self.projection.messages.push(ClientMessage {
            role: ClientMessageRole::System,
            content: content.into(),
            item_id: None,
            streaming: false,
        });
    }

    pub fn export_markdown(&self) -> String {
        let mut output = String::from("# Tessera Export\n\n");
        if self.projection.messages.is_empty() {
            output.push_str("_No messages._\n");
            return output;
        }

        for message in &self.projection.messages {
            let role = match message.role {
                ClientMessageRole::System => "System",
                ClientMessageRole::User => "User",
                ClientMessageRole::Assistant => "Assistant",
                ClientMessageRole::Reasoning => "Reasoning",
            };
            output.push_str("## ");
            output.push_str(role);
            output.push_str("\n\n");
            output.push_str(&message.content);
            output.push_str("\n\n");
        }

        output
    }

    fn task_mut_or_insert(&mut self, task_id: &TaskId) -> &mut ClientTask {
        if let Some(index) = self.tasks.iter().position(|task| &task.task_id == task_id) {
            return &mut self.tasks[index];
        }

        self.tasks.push(ClientTask::new(task_id.clone()));
        self.tasks
            .last_mut()
            .expect("task was just inserted into non-empty registry")
    }

    fn artifact_mut_or_insert(&mut self, artifact_id: &ArtifactId) -> &mut ClientArtifact {
        if let Some(index) = self
            .artifacts
            .iter()
            .position(|artifact| &artifact.artifact_id == artifact_id)
        {
            return &mut self.artifacts[index];
        }

        self.artifacts
            .push(ClientArtifact::new(artifact_id.clone()));
        self.artifacts
            .last_mut()
            .expect("artifact was just inserted into non-empty registry")
    }

    fn coding_workflow_mut_or_insert(
        &mut self,
        workflow_id: &CodingWorkflowId,
        task_id: &TaskId,
    ) -> &mut ClientCodingWorkflow {
        if let Some(index) = self
            .coding_workflows
            .iter()
            .position(|workflow| &workflow.workflow_id == workflow_id)
        {
            return &mut self.coding_workflows[index];
        }

        self.coding_workflows.push(ClientCodingWorkflow::new(
            workflow_id.clone(),
            task_id.clone(),
        ));
        self.coding_workflows
            .last_mut()
            .expect("workflow was just inserted into non-empty registry")
    }

    fn record_pending_approval(&mut self, decision: &ToolPolicyDecision, approval_id: ApprovalId) {
        let approval = ClientApproval::pending_from_decision(decision, approval_id.clone());
        if let Some(existing) = self
            .approvals
            .iter_mut()
            .find(|existing| existing.approval_id == approval_id)
        {
            *existing = approval;
        } else {
            self.approvals.push(approval);
        }
        self.status.update_approval_summary(&self.approvals);
        self.refresh_workflow_inspections();
    }

    fn record_pending_approval_parts(&mut self, parts: PendingApprovalParts) {
        let approval = ClientApproval {
            approval_id: parts.approval_id.clone(),
            call_id: parts.call_id,
            tool_id: parts.tool_id,
            status: ClientApprovalStatus::Pending,
            reason: parts.reason,
            required_permissions: parts.required_permissions,
            side_effects: parts.side_effects,
        };
        if let Some(existing) = self
            .approvals
            .iter_mut()
            .find(|existing| existing.approval_id == parts.approval_id)
        {
            *existing = approval;
        } else {
            self.approvals.push(approval);
        }
        self.status.update_approval_summary(&self.approvals);
        self.refresh_workflow_inspections();
    }

    fn record_resolved_approval(&mut self, approval: &ToolApproval) {
        if let Some(existing) = self
            .approvals
            .iter_mut()
            .find(|existing| existing.approval_id == approval.approval_id)
        {
            existing.update_from_approval(approval);
        } else {
            self.approvals.push(ClientApproval::from_approval(approval));
        }
        self.status.update_approval_summary(&self.approvals);
        self.refresh_workflow_inspections();
    }

    fn record_resolved_approval_parts(&mut self, parts: ResolvedApprovalParts) {
        if let Some(existing) = self
            .approvals
            .iter_mut()
            .find(|existing| existing.approval_id == parts.approval_id)
        {
            existing.call_id = parts.call_id;
            existing.tool_id = parts.tool_id;
            existing.status = parts.status;
            existing.reason = parts.reason;
        } else {
            self.approvals.push(ClientApproval {
                approval_id: parts.approval_id,
                call_id: parts.call_id,
                tool_id: parts.tool_id,
                status: parts.status,
                reason: parts.reason,
                required_permissions: Vec::new(),
                side_effects: Vec::new(),
            });
        }
        self.status.update_approval_summary(&self.approvals);
        self.refresh_workflow_inspections();
    }

    fn record_memory_proposal(&mut self, proposal: &MemoryProposal) {
        if let Some(existing) = self
            .memory_proposals
            .iter_mut()
            .find(|existing| existing.proposal_id == proposal.proposal_id)
        {
            existing.update_from_proposal(proposal);
        } else {
            self.memory_proposals
                .push(ClientMemoryProposal::from_proposal(proposal));
        }
        self.status.update_memory_summary(&self.memory_proposals);
    }

    fn record_memory_proposal_parts(&mut self, parts: MemoryProposalParts) {
        let proposal = ClientMemoryProposal {
            proposal_id: parts.proposal_id.clone(),
            status: parts.status,
            title: parts.title,
            summary: parts.summary,
            source_item_id: parts.source_item_id,
            reason: parts.reason,
        };
        if let Some(existing) = self
            .memory_proposals
            .iter_mut()
            .find(|existing| existing.proposal_id == parts.proposal_id)
        {
            *existing = proposal;
        } else {
            self.memory_proposals.push(proposal);
        }
        self.status.update_memory_summary(&self.memory_proposals);
    }

    fn record_handoff_summary(&mut self, summary: &AgentHandoffSummary) {
        let handoff = ClientAgentHandoff::from_summary(summary);
        if let Some(existing) = self
            .handoffs
            .iter_mut()
            .find(|existing| existing.handoff_id == summary.handoff_id)
        {
            *existing = handoff;
        } else {
            self.handoffs.push(handoff);
        }
        self.status
            .update_handoff_summary(&self.handoffs, &self.reviewer_gates);
    }

    fn record_reviewer_gate_request(&mut self, request: &ReviewerGateRequest) {
        let gate = ClientReviewerGate::pending_from_request(request);
        if let Some(existing) = self
            .reviewer_gates
            .iter_mut()
            .find(|existing| existing.gate_id == request.gate_id)
        {
            *existing = gate;
        } else {
            self.reviewer_gates.push(gate);
        }
        self.status
            .update_handoff_summary(&self.handoffs, &self.reviewer_gates);
        self.refresh_workflow_inspections();
    }

    fn record_reviewer_gate_decision(&mut self, decision: &ReviewerGateDecision) {
        if let Some(existing) = self
            .reviewer_gates
            .iter_mut()
            .find(|existing| existing.gate_id == decision.gate_id)
        {
            existing.apply_decision(decision);
        } else {
            self.reviewer_gates
                .push(ClientReviewerGate::from_decision(decision));
        }
        self.status
            .update_handoff_summary(&self.handoffs, &self.reviewer_gates);
        self.refresh_workflow_inspections();
    }

    fn record_coding_workflow_started(
        &mut self,
        workflow_id: &CodingWorkflowId,
        task_id: &TaskId,
        objective: &str,
    ) {
        {
            let workflow = self.coding_workflow_mut_or_insert(workflow_id, task_id);
            workflow.record_start(task_id.clone(), objective.to_string());
        }
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_scope(&mut self, scope: &WorkspaceMutationScope) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&scope.workflow_id, &scope.task_id);
            workflow.record_scope(scope);
        }
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_mutation_request(&mut self, proposal: &MutationRequestProposal) {
        {
            let workflow =
                self.coding_workflow_mut_or_insert(&proposal.workflow_id, &proposal.task_id);
            workflow.record_mutation_request(proposal);
        }
        self.record_evidence_artifacts(
            &proposal.evidence,
            ArtifactKind::Patch,
            "mutation_request_proposal_recorded",
        );
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_patch_proposal(&mut self, proposal: &PatchProposal) {
        {
            let workflow =
                self.coding_workflow_mut_or_insert(&proposal.workflow_id, &proposal.task_id);
            workflow.record_patch_proposal(proposal);
        }
        self.record_evidence_artifacts(
            &proposal.diff_artifacts,
            ArtifactKind::Patch,
            "patch_proposal_recorded",
        );
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_apply_patch_preflight(&mut self, record: &ApplyPatchPreflightRecord) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&record.workflow_id, &record.task_id);
            workflow.record_apply_patch_preflight(record);
        }
        self.record_evidence_artifacts(
            &record.evidence,
            ArtifactKind::Patch,
            "apply_patch_preflight_recorded",
        );
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_apply_patch_execution(&mut self, record: &ApplyPatchExecutionRecord) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&record.workflow_id, &record.task_id);
            workflow.record_apply_patch_execution(record);
        }
        for artifact_id in &record.artifact_refs {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.kind = Some(ArtifactKind::Patch);
            artifact.record_reference("apply_patch_execution_recorded");
        }
        self.record_evidence_artifacts(
            &record.evidence,
            ArtifactKind::Patch,
            "apply_patch_execution_recorded",
        );
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_patch_application(&mut self, record: &PatchApplicationRecord) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&record.workflow_id, &record.task_id);
            workflow.record_patch_application(record);
        }
        for artifact_id in &record.artifact_refs {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.kind = Some(ArtifactKind::Patch);
            artifact.record_reference("patch_application_recorded");
        }
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_test_plan(&mut self, plan: &TestPlanRecord) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&plan.workflow_id, &plan.task_id);
            workflow.record_test_plan(plan);
        }
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_test_run(&mut self, record: &TestRunRecord) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&record.workflow_id, &record.task_id);
            workflow.record_test_run(record);
        }
        for artifact_id in record
            .stdout_artifact_id
            .iter()
            .chain(record.stderr_artifact_id.iter())
        {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.kind = Some(ArtifactKind::TestReport);
            artifact.record_reference("test_run_recorded");
        }
        self.record_evidence_artifacts(
            &record.diagnostics,
            ArtifactKind::TestReport,
            "test_run_recorded",
        );
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_test_evidence_summary(&mut self, record: &TestEvidenceSummaryRecord) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&record.workflow_id, &record.task_id);
            workflow.record_test_evidence_summary(record);
        }
        for artifact_id in &record.artifact_refs {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.kind = Some(ArtifactKind::TestReport);
            artifact.record_reference("test_evidence_summary_recorded");
        }
        self.record_evidence_artifacts(
            &record.diagnostics,
            ArtifactKind::TestReport,
            "test_evidence_summary_recorded",
        );
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_review_bundle(&mut self, bundle: &ReviewBundle) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&bundle.workflow_id, &bundle.task_id);
            workflow.record_review_bundle(bundle);
        }
        self.refresh_coding_workflow_summary();
    }

    fn record_coding_workflow_restore_plan(&mut self, plan: &RestorePlanRecord) {
        {
            let workflow = self.coding_workflow_mut_or_insert(&plan.workflow_id, &plan.task_id);
            workflow.record_restore_plan(plan);
        }
        self.refresh_coding_workflow_summary();
    }

    fn record_worktree_lifecycle(&mut self, record: &WorkspaceWorktreeLifecycleRecord) {
        if let Some(existing) = self
            .worktree_lifecycles
            .iter_mut()
            .find(|existing| existing.worktree_id == record.worktree_id)
        {
            existing.update_from_record(record);
        } else {
            self.worktree_lifecycles
                .push(ClientWorktreeLifecycle::from_record(record));
        }
        self.status
            .update_worktree_summary(&self.worktree_lifecycles);
    }

    fn record_subagent_session(&mut self, session: &SubagentSessionDescriptor) {
        let projected = ClientSubagentSession::from_descriptor(session);
        if let Some(existing) = self
            .subagent_sessions
            .iter_mut()
            .find(|existing| existing.session_id == session.session_id)
        {
            *existing = projected;
        } else {
            self.subagent_sessions.push(projected);
        }
        self.status.update_subagent_summary(&self.subagent_sessions);
    }

    fn record_subagent_runtime_decision(&mut self, decision: &SubagentRuntimeDecision) {
        self.subagent_runtime_decisions
            .push(ClientSubagentRuntimeDecision::from_decision(decision));
        self.refresh_subagent_runtime_summary();
    }

    fn record_subagent_transcript(&mut self, transcript: &SubagentTranscriptArtifactRecord) {
        let projected = ClientSubagentTranscriptArtifact::from_record(transcript);
        if let Some(existing) = self.subagent_transcripts.iter_mut().find(|existing| {
            existing.session_id == transcript.session_id
                && existing.artifact_id == transcript.artifact_id
        }) {
            *existing = projected;
        } else {
            self.subagent_transcripts.push(projected);
        }

        let artifact = self.artifact_mut_or_insert(&transcript.artifact_id);
        artifact.kind = Some(ArtifactKind::AgentTranscript);
        artifact.update_scope(
            None,
            None,
            transcript
                .child_task_id
                .clone()
                .or_else(|| Some(transcript.parent_task_id.clone())),
            None,
        );
        artifact.record_reference("subagent_transcript_artifact_recorded");
        self.status.update_artifact_summary(&self.artifacts);
        self.refresh_subagent_runtime_summary();
    }

    fn record_subagent_transcript_lifecycle(
        &mut self,
        lifecycle: &SubagentTranscriptArtifactLifecycleRecord,
    ) {
        self.subagent_transcript_lifecycles.push(
            ClientSubagentTranscriptArtifactLifecycle::from_record(lifecycle),
        );

        let artifact = self.artifact_mut_or_insert(&lifecycle.artifact_id);
        artifact.kind = Some(ArtifactKind::AgentTranscript);
        artifact.update_scope(
            None,
            None,
            lifecycle
                .child_task_id
                .clone()
                .or_else(|| Some(lifecycle.parent_task_id.clone())),
            None,
        );
        artifact.record_reference("subagent_transcript_artifact_lifecycle_recorded");
        self.status.update_artifact_summary(&self.artifacts);
        self.status
            .update_subagent_transcript_lifecycle_summary(&self.subagent_transcript_lifecycles);
    }

    fn record_subagent_approval_forwarding(
        &mut self,
        forwarding: &SubagentApprovalForwardingRecord,
    ) {
        self.subagent_approval_forwarding
            .push(ClientSubagentApprovalForwarding::from_record(forwarding));
        self.refresh_subagent_runtime_summary();
        self.refresh_workflow_inspections();
    }

    fn record_subagent_inactive_policy(&mut self, inactive: &SubagentInactivePolicyRecord) {
        self.subagent_inactive_policies
            .push(ClientSubagentInactivePolicy::from_record(inactive));
        self.refresh_subagent_runtime_summary();
    }

    fn record_subagent_cancellation(&mut self, cancellation: &SubagentCancellationRecord) {
        self.subagent_cancellations
            .push(ClientSubagentCancellation::from_record(cancellation));
        self.refresh_subagent_runtime_summary();
    }

    fn refresh_coding_workflow_summary(&mut self) {
        self.status
            .update_coding_workflow_summary(&self.coding_workflows);
        self.refresh_workflow_inspections();
    }

    fn refresh_workflow_inspections(&mut self) {
        self.workflow_inspections = project_workflow_inspections(
            &self.coding_workflows,
            &self.reviewer_gates,
            &self.approvals,
            &self.subagent_approval_forwarding,
        );
        self.status
            .update_workflow_inspection_summary(&self.workflow_inspections, &self.approvals);
    }

    fn refresh_subagent_runtime_summary(&mut self) {
        self.status.update_subagent_runtime_summary(
            &self.subagent_runtime_decisions,
            &self.subagent_transcripts,
            &self.subagent_approval_forwarding,
            &self.subagent_inactive_policies,
            &self.subagent_cancellations,
        );
    }

    fn record_evidence_artifacts(
        &mut self,
        evidence: &[HandoffEvidenceRef],
        kind: ArtifactKind,
        event_kind: &str,
    ) {
        for evidence in evidence {
            let Some(artifact_id) = &evidence.artifact_id else {
                continue;
            };
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.kind = Some(kind.clone());
            artifact.record_reference(event_kind);
        }
    }

    fn apply_artifact_refs_from_frame(&mut self, frame: &EventFrame) {
        if frame.artifact_refs.is_empty() {
            return;
        }

        let event_kind = frame.event.kind().to_string();
        let thread_id = frame.thread_id.clone();
        let turn_id = frame.turn_id.clone();
        let task_id = frame.task_id.clone();
        let item_id = frame.item_id.clone();
        for artifact_id in &frame.artifact_refs {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.update_scope(
                thread_id.clone(),
                turn_id.clone(),
                task_id.clone(),
                item_id.clone(),
            );
            artifact.record_reference(&event_kind);
        }
        self.status.update_artifact_summary(&self.artifacts);
    }

    fn apply_artifact_refs_from_record(&mut self, record: &TraceRecord) {
        if record.artifact_refs.is_empty() {
            return;
        }

        let event_kind = record.event_kind.clone();
        let thread_id = record.thread_id.clone();
        let turn_id = record.turn_id.clone();
        let task_id = record.task_id.clone();
        let item_id = record.item_id.clone();
        for artifact_id in &record.artifact_refs {
            let artifact = self.artifact_mut_or_insert(artifact_id);
            artifact.update_scope(
                thread_id.clone(),
                turn_id.clone(),
                task_id.clone(),
                item_id.clone(),
            );
            artifact.record_reference(&event_kind);
        }
        self.status.update_artifact_summary(&self.artifacts);
    }
}

fn project_workflow_inspections(
    workflows: &[ClientCodingWorkflow],
    reviewer_gates: &[ClientReviewerGate],
    approvals: &[ClientApproval],
    approval_forwarding: &[ClientSubagentApprovalForwarding],
) -> Vec<ClientWorkflowInspection> {
    let mut sorted_workflows = workflows.iter().collect::<Vec<_>>();
    sorted_workflows.sort_by(|left, right| {
        left.workflow_id
            .cmp(&right.workflow_id)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });

    let mut sorted_task_ids = workflows
        .iter()
        .map(|workflow| workflow.task_id.clone())
        .collect::<Vec<_>>();
    sorted_task_ids.sort();
    sorted_task_ids.dedup();
    let task_refs = sorted_task_ids
        .into_iter()
        .enumerate()
        .map(|(index, task_id)| (task_id, format!("task:{}", index + 1)))
        .collect::<BTreeMap<_, _>>();

    let reviewer_gate_by_id = reviewer_gates
        .iter()
        .map(|gate| (gate.gate_id.clone(), gate))
        .collect::<BTreeMap<_, _>>();
    let approvals_by_id = approvals
        .iter()
        .map(|approval| (approval.approval_id.clone(), approval))
        .collect::<BTreeMap<_, _>>();

    sorted_workflows
        .into_iter()
        .enumerate()
        .map(|(index, workflow)| {
            let gate_ids = workflow_gate_ids(workflow);
            let matched_gates = gate_ids
                .iter()
                .filter_map(|gate_id| reviewer_gate_by_id.get(gate_id).copied())
                .collect::<Vec<_>>();
            let approval_ids = approval_forwarding
                .iter()
                .filter(|forwarding| {
                    forwarding
                        .reviewer_gate_id
                        .as_ref()
                        .map(|gate_id| gate_ids.contains(gate_id))
                        .unwrap_or(false)
                })
                .map(|forwarding| forwarding.approval_id.clone())
                .collect::<BTreeSet<_>>();
            let matched_approvals = approval_ids
                .iter()
                .filter_map(|approval_id| approvals_by_id.get(approval_id).copied())
                .collect::<Vec<_>>();

            ClientWorkflowInspection {
                workflow_ref: format!("workflow:{}", index + 1),
                task_ref: task_refs
                    .get(&workflow.task_id)
                    .cloned()
                    .unwrap_or_else(|| format!("task:{}", index + 1)),
                active: workflow.active,
                mutation_mode: workflow
                    .workspace_scope
                    .as_ref()
                    .map(|scope| mutation_mode_label(scope.mutation_mode).to_string()),
                worktree_required: workflow
                    .workspace_scope
                    .as_ref()
                    .map(|scope| scope.worktree_required)
                    .unwrap_or_else(|| {
                        workflow
                            .mutation_requests
                            .iter()
                            .any(|request| request.worktree_required)
                    }),
                patch_count: workflow.patch_proposals.len(),
                diff_artifact_ref_count: workflow
                    .patch_proposals
                    .iter()
                    .flat_map(|proposal| proposal.diff_artifacts.iter())
                    .filter(|evidence| evidence.artifact_id.is_some())
                    .count(),
                patches_requiring_review_count: workflow
                    .patch_proposals
                    .iter()
                    .filter(|proposal| proposal.reviewer_gate_id.is_some())
                    .count(),
                review_bundle_count: workflow.review_bundles.len(),
                review_evidence_ref_count: workflow
                    .review_bundles
                    .iter()
                    .map(|bundle| bundle.evidence.len())
                    .sum(),
                reviewer_gate_count: matched_gates.len(),
                accepted_reviewer_gate_count: matched_gates
                    .iter()
                    .filter(|gate| gate.status == ClientReviewerGateStatus::Accepted)
                    .count(),
                rejected_reviewer_gate_count: matched_gates
                    .iter()
                    .filter(|gate| gate.status == ClientReviewerGateStatus::Rejected)
                    .count(),
                revision_requested_reviewer_gate_count: matched_gates
                    .iter()
                    .filter(|gate| gate.status == ClientReviewerGateStatus::RevisionRequested)
                    .count(),
                pending_reviewer_gate_count: matched_gates
                    .iter()
                    .filter(|gate| gate.status == ClientReviewerGateStatus::Pending)
                    .count(),
                approval_count: matched_approvals.len(),
                pending_approval_count: matched_approvals
                    .iter()
                    .filter(|approval| approval.status == ClientApprovalStatus::Pending)
                    .count(),
                resolved_approval_count: matched_approvals
                    .iter()
                    .filter(|approval| approval.status != ClientApprovalStatus::Pending)
                    .count(),
                apply_patch_preflight_count: workflow.apply_patch_preflights.len(),
                executor_ready_preflight_count: workflow
                    .apply_patch_preflights
                    .iter()
                    .filter(|preflight| {
                        preflight.status == ApplyPatchPreflightStatus::ExecutorReady
                    })
                    .count(),
                executor_blocked_preflight_count: workflow
                    .apply_patch_preflights
                    .iter()
                    .filter(|preflight| preflight.executor_blocked)
                    .count(),
                apply_patch_execution_count: workflow.apply_patch_executions.len(),
                successful_apply_patch_execution_count: workflow
                    .apply_patch_executions
                    .iter()
                    .filter(|execution| execution.status == ApplyPatchExecutionStatus::Applied)
                    .count(),
                failed_apply_patch_execution_count: workflow
                    .apply_patch_executions
                    .iter()
                    .filter(|execution| {
                        matches!(
                            execution.status,
                            ApplyPatchExecutionStatus::Conflict
                                | ApplyPatchExecutionStatus::Rejected
                                | ApplyPatchExecutionStatus::Failed
                        )
                    })
                    .count(),
            }
        })
        .collect()
}

fn workflow_gate_ids(workflow: &ClientCodingWorkflow) -> BTreeSet<ReviewerGateId> {
    let mut gate_ids = BTreeSet::new();
    for gate_id in workflow
        .patch_proposals
        .iter()
        .filter_map(|proposal| proposal.reviewer_gate_id.clone())
    {
        gate_ids.insert(gate_id);
    }
    for gate_id in workflow
        .review_bundles
        .iter()
        .map(|bundle| bundle.reviewer_gate_id.clone())
    {
        gate_ids.insert(gate_id);
    }
    gate_ids
}

fn mutation_mode_label(mode: MutationMode) -> &'static str {
    match mode {
        MutationMode::WorktreeFirst => "worktree_first",
        MutationMode::ExplicitLocal => "explicit_local",
        MutationMode::ReadOnlyProposal => "read_only_proposal",
    }
}

fn trace_record_item_id(record: &TraceRecord) -> Option<ItemId> {
    record.item_id.clone().or_else(|| {
        record
            .payload
            .get("item_id")
            .and_then(|value| value.as_str())
            .map(ItemId::from)
    })
}

fn trace_record_task_id(record: &TraceRecord) -> Option<TaskId> {
    record.task_id.clone().or_else(|| {
        record
            .payload
            .get("task_id")
            .and_then(|value| value.as_str())
            .map(TaskId::from)
    })
}

fn trace_record_coding_workflow_id(record: &TraceRecord) -> Option<CodingWorkflowId> {
    record
        .payload
        .get("workflow_id")
        .and_then(|value| value.as_str())
        .map(CodingWorkflowId::from)
}

fn trace_record_task_ownership_id(record: &TraceRecord) -> Option<TaskOwnershipId> {
    record
        .payload
        .get("lease_id")
        .and_then(|value| value.as_str())
        .map(TaskOwnershipId::from)
}

fn trace_record_artifact_id(record: &TraceRecord) -> Option<ArtifactId> {
    record
        .payload
        .get("artifact_id")
        .and_then(|value| value.as_str())
        .map(ArtifactId::from)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingApprovalParts {
    approval_id: ApprovalId,
    call_id: ToolCallId,
    tool_id: ToolId,
    reason: Option<String>,
    required_permissions: Vec<String>,
    side_effects: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ResolvedApprovalParts {
    approval_id: ApprovalId,
    call_id: ToolCallId,
    tool_id: ToolId,
    status: ClientApprovalStatus,
    reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MemoryProposalParts {
    proposal_id: MemoryProposalId,
    status: ClientMemoryProposalStatus,
    title: String,
    summary: String,
    source_item_id: Option<ItemId>,
    reason: Option<String>,
}

fn approval_intent(prompt: &str, prefix: &str, approve: bool) -> Option<ClientIntent> {
    let approval_id = prompt.strip_prefix(prefix)?.trim();
    if approval_id.is_empty() {
        return None;
    }

    let approval_id = ApprovalId::from(approval_id);
    if approve {
        Some(ClientIntent::ApproveToolCall { approval_id })
    } else {
        Some(ClientIntent::DenyToolCall { approval_id })
    }
}

fn memory_intent(prompt: &str, prefix: &str, accept: bool) -> Option<ClientIntent> {
    let proposal_id = prompt.strip_prefix(prefix)?.trim();
    if proposal_id.is_empty() {
        return None;
    }

    let proposal_id = MemoryProposalId::from(proposal_id);
    if accept {
        Some(ClientIntent::AcceptMemoryProposal { proposal_id })
    } else {
        Some(ClientIntent::RejectMemoryProposal { proposal_id })
    }
}

fn pause_intent(prompt: &str) -> Option<ClientIntent> {
    let task_id = prompt.strip_prefix("/pause ")?.trim();
    if task_id.is_empty() {
        return None;
    }

    Some(ClientIntent::PauseTask {
        task_id: Some(TaskId::from(task_id)),
    })
}

fn resume_intent(prompt: &str) -> Option<ClientIntent> {
    let task_id = prompt.strip_prefix("/resume-task ")?.trim();
    if task_id.is_empty() {
        return None;
    }

    Some(ClientIntent::ResumeTask {
        task_id: TaskId::from(task_id),
    })
}

fn tool_permission_label(permission: &ToolPermission) -> &'static str {
    match permission {
        ToolPermission::FilesystemRead => "filesystem_read",
        ToolPermission::FilesystemWrite => "filesystem_write",
        ToolPermission::Network => "network",
        ToolPermission::Shell => "shell",
        ToolPermission::Git => "git",
        ToolPermission::EnvRead => "env_read",
    }
}

fn tool_side_effect_label(side_effect: &ToolSideEffect) -> &'static str {
    match side_effect {
        ToolSideEffect::ReadOnly => "read_only",
        ToolSideEffect::WritesWorkspace => "writes_workspace",
        ToolSideEffect::WritesOutsideWorkspace => "writes_outside_workspace",
        ToolSideEffect::Network => "network",
        ToolSideEffect::Shell => "shell",
        ToolSideEffect::PersistentState => "persistent_state",
    }
}

fn client_approval_status_from_str(value: &str) -> Option<ClientApprovalStatus> {
    match value {
        "pending" => Some(ClientApprovalStatus::Pending),
        "approved" => Some(ClientApprovalStatus::Approved),
        "denied" => Some(ClientApprovalStatus::Denied),
        _ => None,
    }
}

fn client_memory_proposal_status_from_str(value: &str) -> Option<ClientMemoryProposalStatus> {
    match value {
        "pending" => Some(ClientMemoryProposalStatus::Pending),
        "applied" => Some(ClientMemoryProposalStatus::Applied),
        "rejected" => Some(ClientMemoryProposalStatus::Rejected),
        _ => None,
    }
}

fn inactive_policy_label(policy: SubagentInactivePolicy) -> &'static str {
    match policy {
        SubagentInactivePolicy::PauseParent => "pause_parent",
        SubagentInactivePolicy::QueueDecision => "queue_decision",
        SubagentInactivePolicy::RequireReviewer => "require_reviewer",
    }
}

fn trace_payload<T: DeserializeOwned>(value: Option<&serde_json::Value>) -> Option<T> {
    value
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

fn string_values(values: &[serde_json::Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn latest_task_summary(tasks: &[ClientTask]) -> String {
    let Some(task) = tasks.last() else {
        return "task idle".to_string();
    };

    format!("task {}", task_status_label(&task.status))
}

fn context_handles_summary(handle_count: usize, summary: &ClientContextBudgetSummary) -> String {
    let mut label = format!(
        "context {handle_count} handles / {}/{} tokens",
        summary.used_tokens, summary.available_tokens
    );
    if summary.over_budget {
        label.push_str(" over budget");
    }
    label
}

fn worktree_lifecycle_summary(lifecycles: &[ClientWorktreeLifecycle]) -> String {
    let retained = lifecycles
        .iter()
        .filter(|lifecycle| lifecycle.latest_status == WorkspaceWorktreeLifecycleStatus::Retained)
        .count();
    let cleanup_failed = lifecycles
        .iter()
        .filter(|lifecycle| {
            lifecycle.latest_status == WorkspaceWorktreeLifecycleStatus::CleanupFailed
        })
        .count();
    if cleanup_failed > 0 {
        format!(
            "worktrees {} / cleanup_failed {cleanup_failed}",
            lifecycles.len()
        )
    } else {
        format!("worktrees {} / retained {retained}", lifecycles.len())
    }
}

fn task_status_label(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::WaitingForApproval => "waiting",
        TaskStatus::Paused => "paused",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

fn task_owner_default_reattach_mode(status: TaskOwnerStatus) -> TaskReattachMode {
    match status {
        TaskOwnerStatus::Attached | TaskOwnerStatus::Heartbeat => {
            TaskReattachMode::ObserveExistingOwner
        }
        TaskOwnerStatus::Detached | TaskOwnerStatus::Lost => TaskReattachMode::OwnerLost,
    }
}
