use tessera_protocol::{
    ApprovalId, ArtifactId, ClientInstanceId, EventRange, ReviewerGateId, RunEvent,
    RuntimeInstanceId, SubagentApprovalForwardingRecord, SubagentApprovalForwardingStatus,
    SubagentCancellationCascade, SubagentCancellationRecord, SubagentInactiveParentAction,
    SubagentInactivePolicy, SubagentInactivePolicyRecord, SubagentRuntimeDecision,
    SubagentRuntimeDecisionKind, SubagentSessionDescriptor, SubagentSessionStatus,
    SubagentTranscriptArtifactLifecycleRecord, SubagentTranscriptArtifactStatus, TaskId,
    TaskOwnerHeartbeat, TaskOwnerKind, TaskOwnerLease, TaskOwnerStatus, TaskOwnershipId,
    TaskPauseCheckpointId, TaskReattachMode, TaskReattachRecord, Timestamp,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentRuntimeStartRequest {
    pub session: SubagentSessionDescriptor,
    pub current_depth: u32,
    pub requested_child_sessions: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentCompletionRequest {
    pub session: SubagentSessionDescriptor,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentTranscriptArtifactLifecycleRequest {
    pub session: SubagentSessionDescriptor,
    pub artifact_id: ArtifactId,
    pub status: SubagentTranscriptArtifactStatus,
    pub event_range: Option<EventRange>,
    pub summary_label: Option<String>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentApprovalForwardingRequest {
    pub session: SubagentSessionDescriptor,
    pub approval_id: ApprovalId,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub status: SubagentApprovalForwardingStatus,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentInactivePolicyRequest {
    pub session: SubagentSessionDescriptor,
    pub policy: SubagentInactivePolicy,
    pub parent_action: SubagentInactiveParentAction,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentCancellationRequest {
    pub session: SubagentSessionDescriptor,
    pub source_task_id: TaskId,
    pub cascade: SubagentCancellationCascade,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentTaskOwnerAttachRequest {
    pub session: SubagentSessionDescriptor,
    pub trace_id: String,
    pub runtime_id: RuntimeInstanceId,
    pub client_id: Option<ClientInstanceId>,
    pub owner_kind: TaskOwnerKind,
    pub last_seq: Option<u64>,
    pub heartbeat_interval_ms: u64,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentTaskOwnerDetachRequest {
    pub session: SubagentSessionDescriptor,
    pub lease_id: TaskOwnershipId,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentTaskOwnerHeartbeatRequest {
    pub session: SubagentSessionDescriptor,
    pub lease_id: TaskOwnershipId,
    pub runtime_id: RuntimeInstanceId,
    pub heartbeat_at: Timestamp,
    pub expires_at: Timestamp,
    pub last_seq: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentTaskOwnerLostRequest {
    pub session: SubagentSessionDescriptor,
    pub lease_id: TaskOwnershipId,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SubagentTaskOwnerReattachRequest {
    pub session: SubagentSessionDescriptor,
    pub mode: TaskReattachMode,
    pub previous_lease_id: Option<TaskOwnershipId>,
    pub new_lease_id: Option<TaskOwnershipId>,
    pub checkpoint_id: Option<TaskPauseCheckpointId>,
    pub since_seq: Option<u64>,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubagentRuntimeError {
    message: String,
}

impl SubagentRuntimeError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SubagentRuntimeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SubagentRuntimeError {}

#[derive(Clone, Debug, Default)]
pub struct SubagentRuntimeCoordinator;

impl SubagentRuntimeCoordinator {
    pub fn plan_start(&self, request: SubagentRuntimeStartRequest) -> SubagentRuntimeDecision {
        let session = request.session;
        let kind_and_reason = if session.caps.max_steps == 0 {
            (
                SubagentRuntimeDecisionKind::StartDenied,
                "max_steps must be greater than 0".to_string(),
            )
        } else if request.current_depth > session.caps.max_depth {
            (
                SubagentRuntimeDecisionKind::StartDenied,
                format!(
                    "max_depth exceeded: current depth {} is greater than cap {}",
                    request.current_depth, session.caps.max_depth
                ),
            )
        } else if request.requested_child_sessions > session.caps.max_child_sessions {
            (
                SubagentRuntimeDecisionKind::StartDenied,
                format!(
                    "max_child_sessions exceeded: requested {} is greater than cap {}",
                    request.requested_child_sessions, session.caps.max_child_sessions
                ),
            )
        } else if requires_reviewer_gate(&session) {
            (
                SubagentRuntimeDecisionKind::RequireReviewer,
                "reviewer_gate_id is required for require_reviewer inactive policy".to_string(),
            )
        } else {
            (
                SubagentRuntimeDecisionKind::StartAllowed,
                "sub-agent runtime metadata is within caps".to_string(),
            )
        };

        decision_from_session(&session, kind_and_reason.0, kind_and_reason.1)
    }

    pub fn plan_start_event(&self, request: SubagentRuntimeStartRequest) -> RunEvent {
        RunEvent::SubagentRuntimeDecisionRecorded {
            decision: self.plan_start(request),
        }
    }

    pub fn plan_completion(&self, request: SubagentCompletionRequest) -> SubagentRuntimeDecision {
        let session = request.session;
        if session.transcript_artifact_id.is_none() {
            return decision_from_session(
                &session,
                SubagentRuntimeDecisionKind::StartDenied,
                "transcript_artifact_id is required before sub-agent completion".to_string(),
            );
        }

        decision_from_session(
            &session,
            SubagentRuntimeDecisionKind::StartAllowed,
            "sub-agent completion has transcript artifact metadata".to_string(),
        )
    }

    pub fn record_transcript_lifecycle(
        &self,
        request: SubagentTranscriptArtifactLifecycleRequest,
    ) -> Result<SubagentTranscriptArtifactLifecycleRecord, SubagentRuntimeError> {
        validate_transcript_lifecycle_event_range(request.status, request.event_range.as_ref())?;

        Ok(SubagentTranscriptArtifactLifecycleRecord {
            session_id: request.session.session_id,
            parent_task_id: request.session.parent_task_id,
            child_task_id: request.session.child_task_id,
            artifact_id: request.artifact_id,
            status: request.status,
            event_range: request.event_range,
            summary_label: request.summary_label,
            reason: request.reason,
        })
    }

    pub fn transcript_lifecycle_event(
        &self,
        request: SubagentTranscriptArtifactLifecycleRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        self.record_transcript_lifecycle(request)
            .map(|lifecycle| RunEvent::SubagentTranscriptArtifactLifecycleRecorded { lifecycle })
    }

    pub fn record_approval_forwarding(
        &self,
        request: SubagentApprovalForwardingRequest,
    ) -> Result<SubagentApprovalForwardingRecord, SubagentRuntimeError> {
        validate_approval_forwarding(&request)?;

        Ok(SubagentApprovalForwardingRecord {
            session_id: request.session.session_id,
            parent_task_id: request.session.parent_task_id,
            approval_id: request.approval_id,
            reviewer_gate_id: request.reviewer_gate_id,
            status: request.status,
            reason: request.reason,
        })
    }

    pub fn approval_forwarding_event(
        &self,
        request: SubagentApprovalForwardingRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        self.record_approval_forwarding(request)
            .map(|forwarding| RunEvent::SubagentApprovalForwardingRecorded { forwarding })
    }

    pub fn record_inactive_policy(
        &self,
        request: SubagentInactivePolicyRequest,
    ) -> Result<SubagentInactivePolicyRecord, SubagentRuntimeError> {
        validate_inactive_policy_action(request.policy, request.parent_action)?;

        Ok(SubagentInactivePolicyRecord {
            session_id: request.session.session_id,
            parent_task_id: request.session.parent_task_id,
            policy: request.policy,
            parent_action: request.parent_action,
            reason: request.reason,
        })
    }

    pub fn inactive_policy_event(
        &self,
        request: SubagentInactivePolicyRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        self.record_inactive_policy(request)
            .map(|inactive| RunEvent::SubagentInactivePolicyRecorded { inactive })
    }

    pub fn record_cancellation(
        &self,
        request: SubagentCancellationRequest,
    ) -> Result<SubagentCancellationRecord, SubagentRuntimeError> {
        validate_cancellation(&request)?;

        Ok(SubagentCancellationRecord {
            session_id: request.session.session_id,
            parent_task_id: request.session.parent_task_id,
            source_task_id: request.source_task_id,
            reason: request.reason,
            cascade: request.cascade,
        })
    }

    pub fn cancellation_event(
        &self,
        request: SubagentCancellationRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        self.record_cancellation(request)
            .map(|cancellation| RunEvent::SubagentCancellationRecorded { cancellation })
    }

    pub fn task_owner_lease(
        &self,
        request: SubagentTaskOwnerAttachRequest,
    ) -> Result<TaskOwnerLease, SubagentRuntimeError> {
        let child_task_id = validate_task_owner_attach(&request)?;

        Ok(TaskOwnerLease {
            lease_id: TaskOwnershipId::new(),
            task_id: child_task_id,
            trace_id: request.trace_id,
            runtime_id: request.runtime_id,
            client_id: request.client_id,
            owner_kind: request.owner_kind,
            status: TaskOwnerStatus::Attached,
            acquired_at: Timestamp::now_utc(),
            heartbeat_interval_ms: request.heartbeat_interval_ms,
            expires_at: None,
            last_heartbeat_at: None,
            last_seq: request.last_seq,
            reason: Some(request.reason),
        })
    }

    pub fn task_owner_attach_event(
        &self,
        request: SubagentTaskOwnerAttachRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        self.task_owner_lease(request)
            .map(|lease| RunEvent::TaskOwnerAttached {
                lease: Box::new(lease),
            })
    }

    pub fn task_owner_detach_event(
        &self,
        request: SubagentTaskOwnerDetachRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        let child_task_id = request.session.child_task_id.ok_or_else(|| {
            SubagentRuntimeError::new("child_task_id is required for sub-agent task owner")
        })?;

        Ok(RunEvent::TaskOwnerDetached {
            lease_id: request.lease_id,
            task_id: child_task_id,
            reason: request.reason,
        })
    }

    pub fn task_owner_heartbeat_event(
        &self,
        request: SubagentTaskOwnerHeartbeatRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        let child_task_id = validate_task_owner_heartbeat(&request)?;

        Ok(RunEvent::TaskOwnerHeartbeat {
            heartbeat: TaskOwnerHeartbeat {
                lease_id: request.lease_id,
                task_id: child_task_id,
                runtime_id: request.runtime_id,
                heartbeat_at: request.heartbeat_at,
                expires_at: request.expires_at,
                last_seq: request.last_seq,
            },
        })
    }

    pub fn task_owner_lost_event(
        &self,
        request: SubagentTaskOwnerLostRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        let child_task_id = required_child_task_id(&request.session)?;
        validate_non_empty_reason(
            &request.reason,
            "reason is required for sub-agent task owner lost",
        )?;

        Ok(RunEvent::TaskOwnerLost {
            lease_id: request.lease_id,
            task_id: child_task_id,
            reason: Some(request.reason),
        })
    }

    pub fn task_owner_reattach_event(
        &self,
        request: SubagentTaskOwnerReattachRequest,
    ) -> Result<RunEvent, SubagentRuntimeError> {
        let child_task_id = validate_task_owner_reattach(&request)?;

        Ok(RunEvent::TaskReattachRecorded {
            record: TaskReattachRecord {
                task_id: child_task_id,
                mode: request.mode,
                previous_lease_id: request.previous_lease_id,
                new_lease_id: request.new_lease_id,
                checkpoint_id: request.checkpoint_id,
                since_seq: request.since_seq,
                reason: Some(request.reason),
            },
        })
    }
}

fn validate_task_owner_heartbeat(
    request: &SubagentTaskOwnerHeartbeatRequest,
) -> Result<TaskId, SubagentRuntimeError> {
    let child_task_id = required_child_task_id(&request.session)?;
    if is_terminal_subagent_status(request.session.status) {
        return Err(SubagentRuntimeError::new(
            "terminal sub-agent session cannot heartbeat task owner",
        ));
    }

    Ok(child_task_id)
}

fn validate_task_owner_reattach(
    request: &SubagentTaskOwnerReattachRequest,
) -> Result<TaskId, SubagentRuntimeError> {
    let child_task_id = required_child_task_id(&request.session)?;
    validate_non_empty_reason(
        &request.reason,
        "reason is required for sub-agent task owner reattach",
    )?;

    match request.mode {
        TaskReattachMode::ObserveExistingOwner => {
            if request.new_lease_id.is_none() {
                return Err(SubagentRuntimeError::new(
                    "observe_existing_owner reattach requires new_lease_id",
                ));
            }
        }
        TaskReattachMode::ResumeFromCheckpoint => {
            if request.checkpoint_id.is_none() {
                return Err(SubagentRuntimeError::new(
                    "resume_from_checkpoint reattach requires checkpoint_id",
                ));
            }
        }
        TaskReattachMode::OwnerLost => {
            if request.previous_lease_id.is_none() {
                return Err(SubagentRuntimeError::new(
                    "owner_lost reattach requires previous_lease_id",
                ));
            }
        }
        TaskReattachMode::TerminalProjection => {}
    }

    Ok(child_task_id)
}

fn required_child_task_id(
    session: &SubagentSessionDescriptor,
) -> Result<TaskId, SubagentRuntimeError> {
    session.child_task_id.clone().ok_or_else(|| {
        SubagentRuntimeError::new("child_task_id is required for sub-agent task owner")
    })
}

fn validate_non_empty_reason(
    reason: &str,
    message: &'static str,
) -> Result<(), SubagentRuntimeError> {
    if reason.trim().is_empty() {
        return Err(SubagentRuntimeError::new(message));
    }

    Ok(())
}

fn validate_task_owner_attach(
    request: &SubagentTaskOwnerAttachRequest,
) -> Result<TaskId, SubagentRuntimeError> {
    let child_task_id = required_child_task_id(&request.session)?;

    if request.trace_id.trim().is_empty() {
        return Err(SubagentRuntimeError::new(
            "trace_id is required for sub-agent task owner",
        ));
    }

    validate_non_empty_reason(
        &request.reason,
        "reason is required for sub-agent task owner",
    )?;

    if request.heartbeat_interval_ms == 0 {
        return Err(SubagentRuntimeError::new(
            "heartbeat_interval_ms must be greater than 0",
        ));
    }

    if is_terminal_subagent_status(request.session.status) {
        return Err(SubagentRuntimeError::new(
            "terminal sub-agent session cannot attach task owner",
        ));
    }

    if request.owner_kind == TaskOwnerKind::Execution
        && request.session.status != SubagentSessionStatus::Active
    {
        return Err(SubagentRuntimeError::new(
            "execution task owner requires active sub-agent session",
        ));
    }

    Ok(child_task_id)
}

fn is_terminal_subagent_status(status: SubagentSessionStatus) -> bool {
    matches!(
        status,
        SubagentSessionStatus::Completed
            | SubagentSessionStatus::Failed
            | SubagentSessionStatus::Cancelled
    )
}

fn validate_cancellation(
    request: &SubagentCancellationRequest,
) -> Result<(), SubagentRuntimeError> {
    if request.reason.trim().is_empty() {
        return Err(SubagentRuntimeError::new(
            "reason is required for sub-agent cancellation",
        ));
    }

    let source_matches_parent = request.source_task_id == request.session.parent_task_id;
    let source_matches_child = request
        .session
        .child_task_id
        .as_ref()
        .map(|child_task_id| request.source_task_id == *child_task_id)
        .unwrap_or(false);

    if !source_matches_parent && !source_matches_child {
        return Err(SubagentRuntimeError::new(
            "source_task_id must match parent_task_id or child_task_id",
        ));
    }

    if request.cascade == SubagentCancellationCascade::CancelChild
        && request.session.child_task_id.is_none()
    {
        return Err(SubagentRuntimeError::new(
            "child_task_id is required for cancel_child cascade",
        ));
    }

    Ok(())
}

fn validate_approval_forwarding(
    request: &SubagentApprovalForwardingRequest,
) -> Result<(), SubagentRuntimeError> {
    if request.approval_id.as_str().trim().is_empty() {
        return Err(SubagentRuntimeError::new(
            "approval_id is required for approval forwarding",
        ));
    }

    match request.status {
        SubagentApprovalForwardingStatus::QueuedForReviewer => {
            if request.reviewer_gate_id.is_none() {
                return Err(SubagentRuntimeError::new(
                    "reviewer_gate_id is required for queued approval forwarding",
                ));
            }
        }
        SubagentApprovalForwardingStatus::ForwardedToParent => {
            let already_forwarded_from_parent = request
                .session
                .approval_forwarding
                .as_ref()
                .map(|forwarding| forwarding.forwarded_from_parent)
                .unwrap_or(false);
            if already_forwarded_from_parent {
                return Err(SubagentRuntimeError::new(
                    "approval was already forwarded from parent",
                ));
            }
        }
        SubagentApprovalForwardingStatus::DeniedByPolicy => {}
    }

    Ok(())
}

fn validate_inactive_policy_action(
    policy: SubagentInactivePolicy,
    parent_action: SubagentInactiveParentAction,
) -> Result<(), SubagentRuntimeError> {
    match (policy, parent_action) {
        (
            SubagentInactivePolicy::RequireReviewer,
            SubagentInactiveParentAction::RequireReviewer | SubagentInactiveParentAction::PauseParent,
        )
        | (SubagentInactivePolicy::QueueDecision, SubagentInactiveParentAction::QueueDecision)
        | (SubagentInactivePolicy::PauseParent, SubagentInactiveParentAction::PauseParent) => {
            Ok(())
        }
        (SubagentInactivePolicy::QueueDecision, _) => Err(SubagentRuntimeError::new(
            "queue_decision inactive policy requires queue_decision parent action",
        )),
        (SubagentInactivePolicy::PauseParent, _) => Err(SubagentRuntimeError::new(
            "pause_parent inactive policy requires pause_parent parent action",
        )),
        (SubagentInactivePolicy::RequireReviewer, _) => Err(SubagentRuntimeError::new(
            "require_reviewer inactive policy requires require_reviewer or pause_parent parent action",
        )),
    }
}

fn validate_transcript_lifecycle_event_range(
    status: SubagentTranscriptArtifactStatus,
    event_range: Option<&EventRange>,
) -> Result<(), SubagentRuntimeError> {
    match status {
        SubagentTranscriptArtifactStatus::Published | SubagentTranscriptArtifactStatus::Sealed => {
            let event_range = event_range.ok_or_else(|| {
                SubagentRuntimeError::new(format!(
                    "event_range is required for {} transcript lifecycle",
                    transcript_lifecycle_status_label(status)
                ))
            })?;
            validate_non_empty_event_range(event_range)
        }
        SubagentTranscriptArtifactStatus::Reserved
        | SubagentTranscriptArtifactStatus::Abandoned => {
            if let Some(event_range) = event_range {
                validate_non_empty_event_range(event_range)?;
            }
            Ok(())
        }
    }
}

fn validate_non_empty_event_range(event_range: &EventRange) -> Result<(), SubagentRuntimeError> {
    if event_range.end_seq < event_range.start_seq {
        return Err(SubagentRuntimeError::new(
            "event_range end_seq must be greater than or equal to start_seq",
        ));
    }

    Ok(())
}

fn transcript_lifecycle_status_label(status: SubagentTranscriptArtifactStatus) -> &'static str {
    match status {
        SubagentTranscriptArtifactStatus::Reserved => "reserved",
        SubagentTranscriptArtifactStatus::Published => "published",
        SubagentTranscriptArtifactStatus::Sealed => "sealed",
        SubagentTranscriptArtifactStatus::Abandoned => "abandoned",
    }
}

fn requires_reviewer_gate(session: &SubagentSessionDescriptor) -> bool {
    session
        .approval_forwarding
        .as_ref()
        .map(|forwarding| {
            forwarding.inactive_policy == SubagentInactivePolicy::RequireReviewer
                && forwarding.reviewer_gate_id.is_none()
        })
        .unwrap_or(false)
}

fn decision_from_session(
    session: &SubagentSessionDescriptor,
    kind: SubagentRuntimeDecisionKind,
    reason: String,
) -> SubagentRuntimeDecision {
    SubagentRuntimeDecision {
        session_id: session.session_id.clone(),
        parent_task_id: session.parent_task_id.clone(),
        child_task_id: session.child_task_id.clone(),
        kind,
        reason,
        caps_snapshot: session.caps.clone(),
    }
}
