use tessera_core::RuntimeReader;
use tessera_protocol::{
    CodingWorkflowId, EventFrame, EventRange, HandoffEvidenceKind, HandoffEvidenceRef,
    MutationRequestId, PatchProposalId, RunEvent, TaskId, WorkspaceWorktreeId,
    WorkspaceWorktreeLifecycleRecord, WorkspaceWorktreeLifecycleStatus,
};
use tessera_storage::TraceStore;

fn lifecycle_record(
    worktree_id: WorkspaceWorktreeId,
    status: WorkspaceWorktreeLifecycleStatus,
    reason: &str,
) -> WorkspaceWorktreeLifecycleRecord {
    WorkspaceWorktreeLifecycleRecord {
        worktree_id,
        workflow_id: CodingWorkflowId::from_static("coding_workflow_reader"),
        task_id: TaskId::from_static("task_reader_worktree"),
        trace_id: "trace_reader_worktree_lifecycle".to_string(),
        source_commit: "17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1".to_string(),
        source_branch_label: Some("main".to_string()),
        worktree_root_label: "worktree:reader-projection".to_string(),
        worktree_base_key: "data_dir:worktrees".to_string(),
        lifecycle_status: status,
        reason: reason.to_string(),
        created_for_request_id: Some(MutationRequestId::from_static(
            "mutation_request_reader_worktree",
        )),
        created_for_patch_id: Some(PatchProposalId::from_static("patch_reader_worktree")),
        evidence: vec![HandoffEvidenceRef {
            kind: HandoffEvidenceKind::TraceRange,
            artifact_id: None,
            trace_id: Some("trace_reader_worktree_lifecycle".to_string()),
            event_range: Some(EventRange {
                start_seq: 1,
                end_seq: 3,
            }),
            label: Some("reader lifecycle evidence".to_string()),
            summary: Some("redacted refs only".to_string()),
        }],
        metadata: None,
    }
}

#[test]
fn runtime_reader_lists_worktree_lifecycle_summaries_from_trace() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();
    let trace_id = "trace_reader_worktree_lifecycle";
    let worktree_id = WorkspaceWorktreeId::from_static("workspace_worktree_reader");

    for (seq, status, reason) in [
        (
            1,
            WorkspaceWorktreeLifecycleStatus::Planned,
            "isolated worktree planned",
        ),
        (
            3,
            WorkspaceWorktreeLifecycleStatus::CleanupFailed,
            "git worktree remove failed",
        ),
    ] {
        store
            .append(&EventFrame::new(
                trace_id,
                seq,
                RunEvent::WorkspaceWorktreeLifecycleRecorded {
                    record: lifecycle_record(worktree_id.clone(), status, reason),
                },
            ))
            .unwrap();
    }

    let reader = RuntimeReader::new(store);
    let lifecycles = reader.list_worktree_lifecycles(trace_id).unwrap();

    assert_eq!(lifecycles.len(), 1);
    let lifecycle = &lifecycles[0];
    assert_eq!(lifecycle.worktree_id, worktree_id);
    assert_eq!(
        lifecycle.workflow_id,
        CodingWorkflowId::from_static("coding_workflow_reader")
    );
    assert_eq!(
        lifecycle.task_id,
        TaskId::from_static("task_reader_worktree")
    );
    assert_eq!(lifecycle.trace_id, trace_id);
    assert_eq!(lifecycle.first_event_seq, 1);
    assert_eq!(lifecycle.latest_event_seq, 3);
    assert_eq!(
        lifecycle.source_commit,
        "17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1"
    );
    assert_eq!(lifecycle.source_branch_label.as_deref(), Some("main"));
    assert_eq!(lifecycle.worktree_root_label, "worktree:reader-projection");
    assert_eq!(lifecycle.worktree_base_key, "data_dir:worktrees");
    assert!(!lifecycle.worktree_root_label.contains('/'));
    assert!(!lifecycle.worktree_base_key.contains('/'));
    assert_eq!(
        lifecycle.latest_status,
        WorkspaceWorktreeLifecycleStatus::CleanupFailed
    );
    assert_eq!(
        lifecycle.latest_reason.as_deref(),
        Some("git worktree remove failed")
    );
    assert_eq!(
        lifecycle.created_for_request_id,
        Some(MutationRequestId::from_static(
            "mutation_request_reader_worktree"
        ))
    );
    assert_eq!(
        lifecycle.created_for_patch_id,
        Some(PatchProposalId::from_static("patch_reader_worktree"))
    );
    assert_eq!(lifecycle.evidence.len(), 1);
}
