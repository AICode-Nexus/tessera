use tessera_protocol::{
    RunEvent, SubagentInactivePolicy, SubagentRuntimeDecision, SubagentRuntimeDecisionKind,
    SubagentSessionDescriptor,
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
