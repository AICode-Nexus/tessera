use tessera_protocol::{
    ApprovalId, ArtifactId, EventRange, ReviewerGateId, RunEvent, SubagentApprovalForwardingRecord,
    SubagentApprovalForwardingStatus, SubagentInactiveParentAction, SubagentInactivePolicy,
    SubagentInactivePolicyRecord, SubagentRuntimeDecision, SubagentRuntimeDecisionKind,
    SubagentSessionDescriptor, SubagentTranscriptArtifactLifecycleRecord,
    SubagentTranscriptArtifactStatus,
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
