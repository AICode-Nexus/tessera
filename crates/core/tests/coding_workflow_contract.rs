use tessera_core::{
    CodingWorkflowArtifactBodyRequest, CodingWorkflowCheckpointLifecycleRequest,
    CodingWorkflowCoordinator, CodingWorkflowPatchProposalRequest, CodingWorkflowStartRequest,
    CodingWorkflowTestRunRequest, CodingWorkflowWorkspaceScopeRequest,
};
use tessera_protocol::{
    ArtifactBodyRecord, ArtifactBodyRedactionStatus, ArtifactId, ArtifactKind,
    CodingWorkflowEvidenceRedactionStatus, CodingWorkflowId, HandoffEvidenceKind,
    HandoffEvidenceRef, MutationMode, PatchProposal, PatchProposalId, RestorePlanId,
    RestorePlanRecord, ReviewerGateId, RunEvent, SnapshotId, TaskId, TestRunId, TestRunRecord,
    TestRunStatus, WorkspaceCheckpointLifecycleRecord, WorkspaceCheckpointLifecycleStatus,
};

fn workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from_static("coding_workflow_core")
}

fn task_id() -> TaskId {
    TaskId::from_static("task_coding_workflow_core")
}

fn diff_artifact() -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::DiffArtifact,
        artifact_id: Some(ArtifactId::from_static("artifact_patch_diff")),
        trace_id: None,
        event_range: None,
        label: Some("patch diff".to_string()),
        summary: Some("diff artifact metadata".to_string()),
    }
}

fn patch_proposal() -> PatchProposal {
    PatchProposal {
        patch_id: PatchProposalId::from_static("patch_proposal_core"),
        workflow_id: workflow_id(),
        task_id: task_id(),
        summary: "metadata-only patch proposal".to_string(),
        touched_paths: vec!["docs/README.md".to_string()],
        diff_artifacts: vec![diff_artifact()],
        risk_labels: vec!["docs_only".to_string()],
        required_checkpoint_id: Some(SnapshotId::from_static("snapshot_before_patch")),
        reviewer_gate_id: Some(ReviewerGateId::from_static("reviewer_gate_patch")),
    }
}

#[test]
fn coding_workflow_coordinator_start_requires_task_id() {
    let coordinator = CodingWorkflowCoordinator;
    let error = coordinator.start_event(CodingWorkflowStartRequest {
        workflow_id: workflow_id(),
        task_id: None,
        objective: "prepare patch".to_string(),
    });

    let error = error.expect_err("workflow start should require task id");
    assert!(error.to_string().contains("task_id is required"));
}

#[test]
fn coding_workflow_scope_defaults_to_worktree_first_and_rejects_unsafe_paths() {
    let coordinator = CodingWorkflowCoordinator;
    let event = coordinator.workspace_scope_event(CodingWorkflowWorkspaceScopeRequest {
        workflow_id: workflow_id(),
        task_id: task_id(),
        root_label: "project".to_string(),
        allowed_paths: vec!["docs/**".to_string()],
        denied_paths: vec![".env".to_string()],
        mutation_mode: None,
        worktree_required: true,
        reason: Some("default to isolated mutation".to_string()),
    });

    let event = event.expect("safe scope should produce metadata event");
    assert_eq!(event.kind(), "workspace_mutation_scope_recorded");
    assert_eq!(event.task_id(), Some(task_id()));
    match event {
        RunEvent::WorkspaceMutationScopeRecorded { scope } => {
            assert_eq!(scope.mutation_mode, MutationMode::WorktreeFirst);
            assert!(scope.worktree_required);
            assert_eq!(scope.allowed_paths, vec!["docs/**"]);
        }
        other => panic!("unexpected event: {other:?}"),
    }

    let error = coordinator.workspace_scope_event(CodingWorkflowWorkspaceScopeRequest {
        workflow_id: workflow_id(),
        task_id: task_id(),
        root_label: "project".to_string(),
        allowed_paths: vec!["../secrets".to_string()],
        denied_paths: Vec::new(),
        mutation_mode: Some(MutationMode::ExplicitLocal),
        worktree_required: false,
        reason: None,
    });

    let error = error.expect_err("scope should reject parent traversal");
    assert!(error.to_string().contains("relative workspace path"));
}

#[test]
fn coding_workflow_patch_proposal_requires_diff_and_review_gates() {
    let coordinator = CodingWorkflowCoordinator;
    let mut proposal = patch_proposal();
    proposal.diff_artifacts.clear();

    let error = coordinator.patch_proposal_event(CodingWorkflowPatchProposalRequest {
        proposal,
        mutation_ready: false,
    });

    let error = error.expect_err("patch proposal should require diff artifact refs");
    assert!(error.to_string().contains("diff_artifacts is required"));

    let mut proposal = patch_proposal();
    proposal.required_checkpoint_id = None;
    proposal.reviewer_gate_id = None;
    let error = coordinator.patch_proposal_event(CodingWorkflowPatchProposalRequest {
        proposal,
        mutation_ready: true,
    });

    let error = error.expect_err("mutation-ready proposal should require gates");
    assert!(error
        .to_string()
        .contains("mutation-ready patch proposal requires checkpoint and reviewer gate"));
}

#[test]
fn coding_workflow_test_run_requires_output_artifact_refs() {
    let coordinator = CodingWorkflowCoordinator;
    let error = coordinator.test_run_event(CodingWorkflowTestRunRequest {
        record: TestRunRecord {
            test_run_id: TestRunId::from_static("test_run_missing_artifact"),
            workflow_id: workflow_id(),
            task_id: task_id(),
            test_plan_id: None,
            command_label: "cargo test".to_string(),
            status: TestRunStatus::Passed,
            exit_code: Some(0),
            duration_ms: Some(10),
            stdout_artifact_id: None,
            stderr_artifact_id: None,
            diagnostics: Vec::new(),
            redaction_status: CodingWorkflowEvidenceRedactionStatus::Clean,
        },
    });

    let error = error.expect_err("test output should be artifact-backed");
    assert!(error
        .to_string()
        .contains("test run output must use artifact references"));
}

#[test]
fn coding_workflow_artifact_body_and_checkpoint_lifecycle_contracts_are_metadata_only() {
    let coordinator = CodingWorkflowCoordinator;
    let artifact_event = coordinator
        .artifact_body_event(CodingWorkflowArtifactBodyRequest {
            record: ArtifactBodyRecord {
                artifact_id: ArtifactId::from_static("artifact_core_diff_body"),
                kind: ArtifactKind::Patch,
                task_id: Some(task_id()),
                media_type: "text/x-diff".to_string(),
                byte_len: 64,
                storage_uri: "tessera://artifacts/artifact_core_diff_body/body".to_string(),
                redaction_status: ArtifactBodyRedactionStatus::Redacted,
                summary: Some("diff body stored via storage API".to_string()),
                metadata: None,
            },
        })
        .expect("artifact body metadata should become a trace event");

    assert_eq!(artifact_event.kind(), "artifact_body_recorded");
    let payload = artifact_event.payload();
    assert_eq!(payload["record"]["redaction_status"], "redacted");
    assert!(payload.get("body").is_none());
    assert!(payload.get("raw_body").is_none());

    let lifecycle_event = coordinator
        .checkpoint_lifecycle_event(CodingWorkflowCheckpointLifecycleRequest {
            lifecycle: WorkspaceCheckpointLifecycleRecord {
                checkpoint_id: SnapshotId::from_static("snapshot_core_lifecycle"),
                task_id: task_id(),
                status: WorkspaceCheckpointLifecycleStatus::RestoreBlocked,
                reason: "restore remains blocked until executor gate".to_string(),
                restore_plan_id: Some(RestorePlanId::from_static("restore_plan_core_lifecycle")),
                execution_blocked: true,
                evidence: Vec::new(),
                metadata: None,
            },
        })
        .expect("checkpoint lifecycle should be trace-backed metadata");

    assert_eq!(lifecycle_event.kind(), "snapshot_lifecycle_recorded");
    let payload = lifecycle_event.payload();
    assert_eq!(payload["lifecycle"]["execution_blocked"], true);
    assert!(payload.get("restore_command").is_none());

    let error = coordinator
        .checkpoint_lifecycle_event(CodingWorkflowCheckpointLifecycleRequest {
            lifecycle: WorkspaceCheckpointLifecycleRecord {
                checkpoint_id: SnapshotId::from_static("snapshot_core_lifecycle_unblocked"),
                task_id: task_id(),
                status: WorkspaceCheckpointLifecycleStatus::RestoreBlocked,
                reason: "restore should not execute in foundation".to_string(),
                restore_plan_id: Some(RestorePlanId::from_static(
                    "restore_plan_core_lifecycle_unblocked",
                )),
                execution_blocked: false,
                evidence: Vec::new(),
                metadata: None,
            },
        })
        .expect_err("checkpoint restore lifecycle must remain blocked");

    assert!(error
        .to_string()
        .contains("checkpoint lifecycle execution must remain blocked"));
}

#[test]
fn coding_workflow_restore_plan_is_metadata_only() {
    let coordinator = CodingWorkflowCoordinator;
    let event = coordinator.restore_plan_event(RestorePlanRecord {
        restore_plan_id: RestorePlanId::from_static("restore_plan_core"),
        workflow_id: workflow_id(),
        task_id: task_id(),
        checkpoint_id: SnapshotId::from_static("snapshot_before_patch"),
        target_paths: vec!["docs/README.md".to_string()],
        reason: "review rejected patch".to_string(),
        execution_blocked: true,
    });

    let event = event.expect("restore plan should be metadata-only event");
    assert_eq!(event.kind(), "restore_plan_recorded");
    assert_eq!(event.task_id(), Some(task_id()));
    let payload = event.payload();
    assert_eq!(payload["plan"]["execution_blocked"], true);
    assert!(payload.get("restore_command").is_none());
    assert!(payload.get("revert_command").is_none());

    let error = coordinator.restore_plan_event(RestorePlanRecord {
        restore_plan_id: RestorePlanId::from_static("restore_plan_executing"),
        workflow_id: workflow_id(),
        task_id: task_id(),
        checkpoint_id: SnapshotId::from_static("snapshot_before_patch"),
        target_paths: vec!["docs/README.md".to_string()],
        reason: "should not execute restore".to_string(),
        execution_blocked: false,
    });

    let error = error.expect_err("restore execution must remain blocked");
    assert!(error
        .to_string()
        .contains("restore execution must remain blocked"));
}
