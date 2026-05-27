use std::path::{Component, Path};

use serde::de::DeserializeOwned;
use tessera_protocol::{
    ArtifactBodyRecord, ArtifactBodyRedactionStatus, ArtifactId, ArtifactKind, CodingWorkflowId,
    EventRange, HandoffEvidenceKind, HandoffEvidenceRef, MutationMode, MutationRequestId,
    MutationRequestOperationKind, MutationRequestProposal, MutationRequestStatus, PatchProposal,
    PatchProposalId, PolicyOutcome, ReviewerDecisionKind, ReviewerGateDecision, SnapshotId, TaskId,
    ToolPolicyDecision, TraceRecord, WorkspaceCheckpointLifecycleRecord,
    WorkspaceCheckpointLifecycleStatus, WorkspaceMutationScope,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceApplyPatchSelector {
    pub workflow_id: Option<CodingWorkflowId>,
    pub request_id: Option<MutationRequestId>,
    pub patch_id: Option<PatchProposalId>,
    pub patch_artifact_id: Option<ArtifactId>,
    pub allowed_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraceApplyPatchResolveRequest {
    pub trace_id: String,
    pub records: Vec<TraceRecord>,
    pub selector: TraceApplyPatchSelector,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraceApplyPatchResolvedEnvelope {
    pub trace_id: String,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub mutation_request: MutationRequestProposal,
    pub patch_proposal: PatchProposal,
    pub mutation_scope: WorkspaceMutationScope,
    pub checkpoint_lifecycle: WorkspaceCheckpointLifecycleRecord,
    pub reviewer_decision: ReviewerGateDecision,
    pub policy_decision: ToolPolicyDecision,
    pub sandbox_profile_label: Option<String>,
    pub patch_artifact_id: Option<ArtifactId>,
    pub patch_artifact: Option<ArtifactBodyRecord>,
    pub source_event_range: EventRange,
    pub evidence: Vec<HandoffEvidenceRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceApplyPatchResolutionError {
    message: String,
}

impl TraceApplyPatchResolutionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TraceApplyPatchResolutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TraceApplyPatchResolutionError {}

#[derive(Clone, Debug, Default)]
pub struct TraceApplyPatchResolver;

impl TraceApplyPatchResolver {
    pub fn resolve(
        request: TraceApplyPatchResolveRequest,
    ) -> Result<TraceApplyPatchResolvedEnvelope, TraceApplyPatchResolutionError> {
        if request.trace_id.trim().is_empty() {
            return Err(TraceApplyPatchResolutionError::new("trace_id is required"));
        }

        let mut records = request
            .records
            .into_iter()
            .filter(|record| record.trace_id == request.trace_id)
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.seq);
        if records.is_empty() {
            return Err(TraceApplyPatchResolutionError::new(
                "trace has no records for apply-patch resolution",
            ));
        }

        let workflow = select_workflow(&records, request.selector.workflow_id.as_ref())?;
        let scope = select_scope(&records, &workflow.workflow_id, &workflow.task_id)?;
        validate_scope(&scope)?;
        let mutation_request = select_mutation_request(
            &records,
            &workflow.workflow_id,
            &workflow.task_id,
            request.selector.request_id.as_ref(),
        )?;
        validate_mutation_request(&mutation_request, &scope)?;
        let patch_proposal = select_patch_proposal(
            &records,
            &workflow.workflow_id,
            &workflow.task_id,
            request.selector.patch_id.as_ref(),
        )?;
        validate_patch_proposal(&patch_proposal, &mutation_request, &scope)?;
        let allowed_paths = resolve_allowed_paths(
            &scope.allowed_paths,
            &mutation_request.requested_paths,
            &patch_proposal.touched_paths,
            &request.selector.allowed_paths,
        )?;
        let mutation_scope = WorkspaceMutationScope {
            allowed_paths,
            ..scope
        };

        let checkpoint_id = mutation_request
            .required_checkpoint_id
            .clone()
            .ok_or_else(|| TraceApplyPatchResolutionError::new("missing checkpoint reference"))?;
        let reviewer_gate_id = mutation_request
            .reviewer_gate_id
            .clone()
            .ok_or_else(|| TraceApplyPatchResolutionError::new("missing reviewer gate"))?;
        let policy_decision_id = mutation_request.policy_decision_id.clone().ok_or_else(|| {
            TraceApplyPatchResolutionError::new("missing policy decision reference")
        })?;

        let checkpoint_lifecycle =
            select_checkpoint_lifecycle(&records, &checkpoint_id, &workflow.task_id)?;
        let reviewer_decision = select_reviewer_decision(&records, &reviewer_gate_id)?;
        let policy_decision = select_policy_decision(&records, &policy_decision_id)?;
        let patch_artifact_id =
            select_patch_artifact_id(&patch_proposal, request.selector.patch_artifact_id.as_ref())?;
        let patch_artifact = match patch_artifact_id.as_ref() {
            Some(artifact_id) => Some(select_patch_artifact_record(&records, artifact_id)?),
            None => None,
        };
        let sandbox_profile_label = mutation_request.sandbox_profile_label.clone();

        let source_event_range = EventRange {
            start_seq: records.first().map(|record| record.seq).unwrap_or(0),
            end_seq: records.last().map(|record| record.seq).unwrap_or(0),
        };
        let evidence = vec![HandoffEvidenceRef {
            kind: HandoffEvidenceKind::TraceRange,
            artifact_id: patch_artifact_id.clone(),
            trace_id: Some(request.trace_id.clone()),
            event_range: Some(source_event_range.clone()),
            label: Some("trace-driven apply-patch bundle".to_string()),
            summary: Some("reviewed mutation bundle resolved from trace".to_string()),
        }];

        Ok(TraceApplyPatchResolvedEnvelope {
            trace_id: request.trace_id,
            workflow_id: workflow.workflow_id,
            task_id: workflow.task_id,
            mutation_request,
            patch_proposal,
            mutation_scope,
            checkpoint_lifecycle,
            reviewer_decision,
            policy_decision,
            sandbox_profile_label,
            patch_artifact_id,
            patch_artifact,
            source_event_range,
            evidence,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WorkflowSelection {
    workflow_id: CodingWorkflowId,
    task_id: TaskId,
}

fn select_workflow(
    records: &[TraceRecord],
    selector: Option<&CodingWorkflowId>,
) -> Result<WorkflowSelection, TraceApplyPatchResolutionError> {
    let mut workflows = Vec::new();
    for record in records
        .iter()
        .filter(|record| record.event_kind == "coding_workflow_started")
    {
        let workflow_id = payload_field::<CodingWorkflowId>(record, "workflow_id")?;
        let task_id = payload_field::<TaskId>(record, "task_id")?;
        if workflows
            .iter()
            .all(|workflow: &WorkflowSelection| workflow.workflow_id != workflow_id)
        {
            workflows.push(WorkflowSelection {
                workflow_id,
                task_id,
            });
        }
    }

    if let Some(selector) = selector {
        return workflows
            .into_iter()
            .find(|workflow| &workflow.workflow_id == selector)
            .ok_or_else(|| TraceApplyPatchResolutionError::new("selected workflow not found"));
    }

    match workflows.len() {
        0 => Err(TraceApplyPatchResolutionError::new("missing workflow")),
        1 => Ok(workflows.remove(0)),
        _ => Err(TraceApplyPatchResolutionError::new(
            "ambiguous workflow; provide workflow selector",
        )),
    }
}

fn select_scope(
    records: &[TraceRecord],
    workflow_id: &CodingWorkflowId,
    task_id: &TaskId,
) -> Result<WorkspaceMutationScope, TraceApplyPatchResolutionError> {
    let scopes = records
        .iter()
        .filter(|record| record.event_kind == "workspace_mutation_scope_recorded")
        .map(|record| payload_field::<WorkspaceMutationScope>(record, "scope"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|scope| &scope.workflow_id == workflow_id && &scope.task_id == task_id)
        .collect::<Vec<_>>();

    one_or_error(scopes, "missing mutation scope", "ambiguous mutation scope")
}

fn select_mutation_request(
    records: &[TraceRecord],
    workflow_id: &CodingWorkflowId,
    task_id: &TaskId,
    selector: Option<&MutationRequestId>,
) -> Result<MutationRequestProposal, TraceApplyPatchResolutionError> {
    let requests = records
        .iter()
        .filter(|record| record.event_kind == "mutation_request_proposal_recorded")
        .map(|record| payload_field::<MutationRequestProposal>(record, "proposal"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|proposal| {
            &proposal.workflow_id == workflow_id
                && &proposal.task_id == task_id
                && proposal.operation == MutationRequestOperationKind::PatchApplication
        })
        .collect::<Vec<_>>();

    if let Some(selector) = selector {
        return requests
            .into_iter()
            .find(|proposal| &proposal.request_id == selector)
            .ok_or_else(|| {
                TraceApplyPatchResolutionError::new("selected mutation request not found")
            });
    }

    one_or_error(
        requests,
        "missing mutation request",
        "ambiguous mutation request",
    )
}

fn select_patch_proposal(
    records: &[TraceRecord],
    workflow_id: &CodingWorkflowId,
    task_id: &TaskId,
    selector: Option<&PatchProposalId>,
) -> Result<PatchProposal, TraceApplyPatchResolutionError> {
    let proposals = records
        .iter()
        .filter(|record| record.event_kind == "patch_proposal_recorded")
        .map(|record| payload_field::<PatchProposal>(record, "proposal"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|proposal| &proposal.workflow_id == workflow_id && &proposal.task_id == task_id)
        .collect::<Vec<_>>();

    if let Some(selector) = selector {
        return proposals
            .into_iter()
            .find(|proposal| &proposal.patch_id == selector)
            .ok_or_else(|| {
                TraceApplyPatchResolutionError::new("selected patch proposal not found")
            });
    }

    one_or_error(
        proposals,
        "missing patch proposal",
        "ambiguous patch proposal",
    )
}

fn select_checkpoint_lifecycle(
    records: &[TraceRecord],
    checkpoint_id: &SnapshotId,
    task_id: &TaskId,
) -> Result<WorkspaceCheckpointLifecycleRecord, TraceApplyPatchResolutionError> {
    let lifecycles = records
        .iter()
        .filter(|record| record.event_kind == "snapshot_lifecycle_recorded")
        .map(|record| payload_field::<WorkspaceCheckpointLifecycleRecord>(record, "lifecycle"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|lifecycle| {
            &lifecycle.checkpoint_id == checkpoint_id && &lifecycle.task_id == task_id
        })
        .collect::<Vec<_>>();

    if lifecycles.is_empty() {
        return Err(TraceApplyPatchResolutionError::new(
            "missing checkpoint lifecycle",
        ));
    }
    lifecycles
        .into_iter()
        .find(|lifecycle| lifecycle.status == WorkspaceCheckpointLifecycleStatus::Created)
        .ok_or_else(|| TraceApplyPatchResolutionError::new("checkpoint was not created"))
}

fn select_reviewer_decision(
    records: &[TraceRecord],
    reviewer_gate_id: &tessera_protocol::ReviewerGateId,
) -> Result<ReviewerGateDecision, TraceApplyPatchResolutionError> {
    let decision = records
        .iter()
        .filter(|record| record.event_kind == "reviewer_gate_resolved")
        .map(|record| payload_field::<ReviewerGateDecision>(record, "decision"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .rev()
        .find(|decision| &decision.gate_id == reviewer_gate_id)
        .ok_or_else(|| TraceApplyPatchResolutionError::new("missing reviewer decision"))?;

    if decision.decision != ReviewerDecisionKind::Accept {
        return Err(TraceApplyPatchResolutionError::new(
            "reviewer gate is not accepted",
        ));
    }
    Ok(decision)
}

fn select_policy_decision(
    records: &[TraceRecord],
    policy_decision_id: &tessera_protocol::PolicyDecisionId,
) -> Result<ToolPolicyDecision, TraceApplyPatchResolutionError> {
    let decision = records
        .iter()
        .filter(|record| record.event_kind == "tool_policy_decision_recorded")
        .map(|record| payload_field::<ToolPolicyDecision>(record, "decision"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .rev()
        .find(|decision| &decision.decision_id == policy_decision_id)
        .ok_or_else(|| TraceApplyPatchResolutionError::new("missing policy decision"))?;

    if decision.outcome != PolicyOutcome::Allow {
        return Err(TraceApplyPatchResolutionError::new(
            "policy decision is not allowed",
        ));
    }
    Ok(decision)
}

fn select_patch_artifact_id(
    patch_proposal: &PatchProposal,
    selector: Option<&ArtifactId>,
) -> Result<Option<ArtifactId>, TraceApplyPatchResolutionError> {
    let mut artifact_ids = patch_proposal
        .diff_artifacts
        .iter()
        .filter_map(|evidence| evidence.artifact_id.clone())
        .collect::<Vec<_>>();
    artifact_ids.sort();
    artifact_ids.dedup();

    if let Some(selector) = selector {
        return artifact_ids
            .into_iter()
            .find(|artifact_id| artifact_id == selector)
            .map(Some)
            .ok_or_else(|| {
                TraceApplyPatchResolutionError::new("selected patch artifact not found")
            });
    }

    match artifact_ids.len() {
        0 => Err(TraceApplyPatchResolutionError::new(
            "missing patch artifact",
        )),
        1 => Ok(artifact_ids.pop()),
        _ => Err(TraceApplyPatchResolutionError::new(
            "ambiguous patch artifact",
        )),
    }
}

fn select_patch_artifact_record(
    records: &[TraceRecord],
    artifact_id: &ArtifactId,
) -> Result<ArtifactBodyRecord, TraceApplyPatchResolutionError> {
    let artifacts = records
        .iter()
        .filter(|record| record.event_kind == "artifact_body_recorded")
        .map(|record| payload_field::<ArtifactBodyRecord>(record, "record"))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|record| &record.artifact_id == artifact_id)
        .collect::<Vec<_>>();
    let artifact = one_or_error(
        artifacts,
        "missing patch artifact metadata",
        "ambiguous patch artifact metadata",
    )?;

    if artifact.kind != ArtifactKind::Patch {
        return Err(TraceApplyPatchResolutionError::new(
            "patch artifact is not a patch",
        ));
    }
    if artifact.redaction_status != ArtifactBodyRedactionStatus::Clean {
        return Err(TraceApplyPatchResolutionError::new(
            "patch artifact is not clean",
        ));
    }
    if artifact.byte_len == 0 {
        return Err(TraceApplyPatchResolutionError::new(
            "patch artifact body is empty",
        ));
    }
    if !artifact.storage_uri.starts_with("tessera://artifacts/") {
        return Err(TraceApplyPatchResolutionError::new(
            "patch artifact storage URI is not tessera artifact storage",
        ));
    }
    Ok(artifact)
}

fn validate_scope(scope: &WorkspaceMutationScope) -> Result<(), TraceApplyPatchResolutionError> {
    if scope.mutation_mode != MutationMode::WorktreeFirst || !scope.worktree_required {
        return Err(TraceApplyPatchResolutionError::new(
            "worktree-first mutation scope is required",
        ));
    }
    validate_paths(&scope.allowed_paths, "mutation scope allowed paths")?;
    validate_paths(&scope.denied_paths, "mutation scope denied paths")
}

fn validate_mutation_request(
    request: &MutationRequestProposal,
    scope: &WorkspaceMutationScope,
) -> Result<(), TraceApplyPatchResolutionError> {
    if request.status != MutationRequestStatus::Approved {
        return Err(TraceApplyPatchResolutionError::new(
            "mutation request is not approved",
        ));
    }
    if request.required_checkpoint_id.is_none() {
        return Err(TraceApplyPatchResolutionError::new(
            "missing checkpoint reference",
        ));
    }
    if request.reviewer_gate_id.is_none() {
        return Err(TraceApplyPatchResolutionError::new("missing reviewer gate"));
    }
    if request.policy_decision_id.is_none() {
        return Err(TraceApplyPatchResolutionError::new(
            "missing policy decision reference",
        ));
    }
    if request.sandbox_profile_label.is_none() {
        return Err(TraceApplyPatchResolutionError::new(
            "missing sandbox profile",
        ));
    }
    if !request.worktree_required {
        return Err(TraceApplyPatchResolutionError::new(
            "mutation request must require worktree isolation",
        ));
    }
    validate_paths(&request.requested_paths, "mutation request paths")?;
    if !request
        .requested_paths
        .iter()
        .all(|path| scope.allowed_paths.contains(path))
    {
        return Err(TraceApplyPatchResolutionError::new(
            "mutation request paths outside mutation scope",
        ));
    }
    Ok(())
}

fn validate_patch_proposal(
    proposal: &PatchProposal,
    request: &MutationRequestProposal,
    scope: &WorkspaceMutationScope,
) -> Result<(), TraceApplyPatchResolutionError> {
    validate_paths(&proposal.touched_paths, "patch proposal paths")?;
    if !proposal
        .touched_paths
        .iter()
        .all(|path| scope.allowed_paths.contains(path) && request.requested_paths.contains(path))
    {
        return Err(TraceApplyPatchResolutionError::new(
            "patch paths outside mutation scope",
        ));
    }
    if let Some(proposal_checkpoint) = &proposal.required_checkpoint_id {
        if request.required_checkpoint_id.as_ref() != Some(proposal_checkpoint) {
            return Err(TraceApplyPatchResolutionError::new(
                "patch checkpoint does not match mutation request",
            ));
        }
    }
    if let Some(proposal_gate) = &proposal.reviewer_gate_id {
        if request.reviewer_gate_id.as_ref() != Some(proposal_gate) {
            return Err(TraceApplyPatchResolutionError::new(
                "patch reviewer gate does not match mutation request",
            ));
        }
    }
    Ok(())
}

fn resolve_allowed_paths(
    scope_paths: &[String],
    request_paths: &[String],
    patch_paths: &[String],
    selector_paths: &[String],
) -> Result<Vec<String>, TraceApplyPatchResolutionError> {
    validate_paths(selector_paths, "selected allowed paths")?;
    let mut paths = patch_paths
        .iter()
        .filter(|path| scope_paths.contains(path) && request_paths.contains(path))
        .cloned()
        .collect::<Vec<_>>();
    if !selector_paths.is_empty() {
        if !selector_paths.iter().all(|path| paths.contains(path)) {
            return Err(TraceApplyPatchResolutionError::new(
                "selected allowed paths are outside resolved patch scope",
            ));
        }
        paths = selector_paths.to_vec();
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        return Err(TraceApplyPatchResolutionError::new(
            "no allowed paths resolved for patch",
        ));
    }
    Ok(paths)
}

fn payload_field<T: DeserializeOwned>(
    record: &TraceRecord,
    key: &str,
) -> Result<T, TraceApplyPatchResolutionError> {
    let value = record.payload.get(key).ok_or_else(|| {
        TraceApplyPatchResolutionError::new(format!(
            "trace record {} missing payload field {key}",
            record.event_kind
        ))
    })?;
    serde_json::from_value(value.clone()).map_err(|error| {
        TraceApplyPatchResolutionError::new(format!(
            "trace record {} has invalid payload field {key}: {error}",
            record.event_kind
        ))
    })
}

fn one_or_error<T>(
    mut values: Vec<T>,
    missing: &'static str,
    ambiguous: &'static str,
) -> Result<T, TraceApplyPatchResolutionError> {
    match values.len() {
        0 => Err(TraceApplyPatchResolutionError::new(missing)),
        1 => Ok(values.remove(0)),
        _ => Err(TraceApplyPatchResolutionError::new(ambiguous)),
    }
}

fn validate_paths(
    paths: &[String],
    label: &'static str,
) -> Result<(), TraceApplyPatchResolutionError> {
    for path in paths {
        validate_relative_workspace_path(path, label)?;
    }
    Ok(())
}

fn validate_relative_workspace_path(
    path: &str,
    label: &'static str,
) -> Result<(), TraceApplyPatchResolutionError> {
    let path = path.trim();
    if path.is_empty() || Path::new(path).is_absolute() {
        return Err(TraceApplyPatchResolutionError::new(format!(
            "{label} must be relative workspace paths"
        )));
    }
    if Path::new(path)
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(TraceApplyPatchResolutionError::new(format!(
            "{label} must not traverse outside workspace"
        )));
    }
    Ok(())
}
