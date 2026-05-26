use std::fs;

use tessera_core::{
    ApplyPatchExecutor, ApplyPatchExecutorRootKind, ApplyPatchInMemoryRequest,
    ApplyPatchIsolatedFileRequest,
};
use tessera_protocol::{
    ApplyPatchExecutionBlocker, ApplyPatchExecutionId, ApplyPatchExecutionStatus,
    ApplyPatchPreflightId, CodingWorkflowId, MutationRequestId, PatchProposalId, PolicyDecisionId,
    ReviewerGateId, SnapshotId, TaskId,
};

fn modify_patch() -> String {
    [
        "diff --git a/docs/README.md b/docs/README.md",
        "index 1111111..2222222 100644",
        "--- a/docs/README.md",
        "+++ b/docs/README.md",
        "@@ -1,3 +1,3 @@",
        " alpha",
        "-old",
        "+new",
        " omega",
        "",
    ]
    .join("\n")
}

fn create_patch(path: &str) -> String {
    [
        format!("diff --git a/{path} b/{path}"),
        "new file mode 100644".to_string(),
        "--- /dev/null".to_string(),
        format!("+++ b/{path}"),
        "@@ -0,0 +1,2 @@".to_string(),
        "+hello".to_string(),
        "+world".to_string(),
        "".to_string(),
    ]
    .join("\n")
}

fn absolute_patch() -> String {
    [
        "diff --git a/docs/README.md b//tmp/evil.md",
        "--- a/docs/README.md",
        "+++ /tmp/evil.md",
        "@@ -1 +1 @@",
        "-old",
        "+evil",
        "",
    ]
    .join("\n")
}

fn isolated_file_request(
    root: &std::path::Path,
    patch_body: String,
) -> ApplyPatchIsolatedFileRequest {
    ApplyPatchIsolatedFileRequest {
        execution_id: ApplyPatchExecutionId::from_static("apply_patch_execution_file_contract"),
        preflight_id: ApplyPatchPreflightId::from_static("apply_patch_preflight_file_contract"),
        workflow_id: CodingWorkflowId::from_static("coding_workflow_file_contract"),
        task_id: TaskId::from_static("task_file_contract"),
        request_id: MutationRequestId::from_static("mutation_request_file_contract"),
        patch_id: PatchProposalId::from_static("patch_proposal_file_contract"),
        checkpoint_id: Some(SnapshotId::from_static("snapshot_file_contract")),
        reviewer_gate_id: Some(ReviewerGateId::from_static("reviewer_gate_file_contract")),
        policy_decision_id: Some(PolicyDecisionId::from_static("policy_file_contract")),
        sandbox_profile_label: Some("workspace_write_isolated".to_string()),
        isolated_root: root.to_path_buf(),
        isolated_root_label: "worktree:apply-patch-file-contract".to_string(),
        root_kind: ApplyPatchExecutorRootKind::IsolatedWorktree,
        executor_label: "core.apply_patch_executor.v1".to_string(),
        allowed_paths: vec![
            "docs/README.md".to_string(),
            "docs/new.md".to_string(),
            "docs/link.md".to_string(),
        ],
        patch_body,
    }
}

#[test]
fn apply_patch_executor_applies_single_file_modify_in_memory() {
    let result = ApplyPatchExecutor.apply_in_memory(ApplyPatchInMemoryRequest {
        patch_body: modify_patch(),
        current_contents: Some("alpha\nold\nomega\n".to_string()),
    });

    assert_eq!(result.status, ApplyPatchExecutionStatus::Applied);
    assert_eq!(result.affected_paths, vec!["docs/README.md"]);
    assert!(result.blockers.is_empty());
    assert_eq!(result.new_contents.as_deref(), Some("alpha\nnew\nomega\n"));
}

#[test]
fn apply_patch_executor_creates_single_file_in_memory() {
    let result = ApplyPatchExecutor.apply_in_memory(ApplyPatchInMemoryRequest {
        patch_body: create_patch("docs/new.md"),
        current_contents: None,
    });

    assert_eq!(result.status, ApplyPatchExecutionStatus::Applied);
    assert_eq!(result.affected_paths, vec!["docs/new.md"]);
    assert!(result.blockers.is_empty());
    assert_eq!(result.new_contents.as_deref(), Some("hello\nworld\n"));
}

#[test]
fn apply_patch_executor_reports_conflict_on_context_mismatch() {
    let result = ApplyPatchExecutor.apply_in_memory(ApplyPatchInMemoryRequest {
        patch_body: modify_patch(),
        current_contents: Some("alpha\nchanged\nomega\n".to_string()),
    });

    assert_eq!(result.status, ApplyPatchExecutionStatus::Conflict);
    assert_eq!(result.affected_paths, vec!["docs/README.md"]);
    assert_eq!(result.conflict_paths, vec!["docs/README.md"]);
    assert!(result
        .blockers
        .contains(&ApplyPatchExecutionBlocker::HunkConflict));
    assert!(result.new_contents.is_none());
}

#[test]
fn apply_patch_executor_rejects_delete_rename_binary_and_multi_file_patches() {
    let cases = [
        [
            "diff --git a/docs/remove.md b/docs/remove.md",
            "deleted file mode 100644",
            "--- a/docs/remove.md",
            "+++ /dev/null",
            "",
        ]
        .join("\n"),
        [
            "diff --git a/docs/old.md b/docs/new.md",
            "rename from docs/old.md",
            "rename to docs/new.md",
            "",
        ]
        .join("\n"),
        [
            "diff --git a/assets/logo.png b/assets/logo.png",
            "Binary files a/assets/logo.png and b/assets/logo.png differ",
            "",
        ]
        .join("\n"),
        [
            "diff --git a/docs/one.md b/docs/one.md",
            "--- a/docs/one.md",
            "+++ b/docs/one.md",
            "@@ -1 +1 @@",
            "-one",
            "+ONE",
            "diff --git a/docs/two.md b/docs/two.md",
            "--- a/docs/two.md",
            "+++ b/docs/two.md",
            "@@ -1 +1 @@",
            "-two",
            "+TWO",
            "",
        ]
        .join("\n"),
    ];

    for patch_body in cases {
        let result = ApplyPatchExecutor.apply_in_memory(ApplyPatchInMemoryRequest {
            patch_body,
            current_contents: Some("one\n".to_string()),
        });

        assert_eq!(result.status, ApplyPatchExecutionStatus::Rejected);
        assert!(
            result
                .blockers
                .contains(&ApplyPatchExecutionBlocker::UnsupportedPatchOperation)
                || result
                    .blockers
                    .contains(&ApplyPatchExecutionBlocker::MultipleFilesUnsupported)
        );
        assert!(result.new_contents.is_none());
    }
}

#[test]
fn apply_patch_executor_does_not_require_filesystem_access_for_parse_or_apply() {
    let result = ApplyPatchExecutor.apply_in_memory(ApplyPatchInMemoryRequest {
        patch_body: create_patch("missing/directory/file.md"),
        current_contents: None,
    });

    assert_eq!(result.status, ApplyPatchExecutionStatus::Applied);
    assert_eq!(result.affected_paths, vec!["missing/directory/file.md"]);
    assert_eq!(result.new_contents.as_deref(), Some("hello\nworld\n"));
}

#[test]
fn apply_patch_executor_writes_create_inside_isolated_root() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();

    let result = ApplyPatchExecutor.apply_to_isolated_root(isolated_file_request(
        temp.path(),
        create_patch("docs/new.md"),
    ));

    assert_eq!(result.record.status, ApplyPatchExecutionStatus::Applied);
    assert_eq!(result.record.affected_paths, vec!["docs/new.md"]);
    assert!(result.record.blockers.is_empty());
    assert_eq!(
        fs::read_to_string(temp.path().join("docs/new.md")).unwrap(),
        "hello\nworld\n"
    );
}

#[test]
fn apply_patch_executor_writes_modify_with_temp_file_and_rename() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    fs::write(temp.path().join("docs/README.md"), "alpha\nold\nomega\n").unwrap();

    let result = ApplyPatchExecutor
        .apply_to_isolated_root(isolated_file_request(temp.path(), modify_patch()));

    assert_eq!(result.record.status, ApplyPatchExecutionStatus::Applied);
    assert_eq!(
        fs::read_to_string(temp.path().join("docs/README.md")).unwrap(),
        "alpha\nnew\nomega\n"
    );
    let temp_entries = fs::read_dir(temp.path().join("docs"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains("tessera-tmp"))
        .count();
    assert_eq!(temp_entries, 0);
}

#[test]
fn apply_patch_executor_rejects_primary_root_label() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    let mut request = isolated_file_request(temp.path(), create_patch("docs/new.md"));
    request.root_kind = ApplyPatchExecutorRootKind::PrimaryProject;
    request.isolated_root_label = "project".to_string();

    let result = ApplyPatchExecutor.apply_to_isolated_root(request);

    assert_eq!(result.record.status, ApplyPatchExecutionStatus::Rejected);
    assert!(result
        .record
        .blockers
        .contains(&ApplyPatchExecutionBlocker::PrimaryRootRejected));
    assert!(!temp.path().join("docs/new.md").exists());
}

#[test]
fn apply_patch_executor_rejects_absolute_parent_and_out_of_scope_paths() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();

    let absolute_result = ApplyPatchExecutor
        .apply_to_isolated_root(isolated_file_request(temp.path(), absolute_patch()));
    assert_eq!(
        absolute_result.record.status,
        ApplyPatchExecutionStatus::Rejected
    );
    assert!(absolute_result
        .record
        .blockers
        .contains(&ApplyPatchExecutionBlocker::UnsafePath));

    let mut out_of_scope = isolated_file_request(temp.path(), create_patch("src/lib.rs"));
    out_of_scope.allowed_paths = vec!["docs/README.md".to_string()];
    let out_of_scope_result = ApplyPatchExecutor.apply_to_isolated_root(out_of_scope);

    assert_eq!(
        out_of_scope_result.record.status,
        ApplyPatchExecutionStatus::Rejected
    );
    assert!(out_of_scope_result
        .record
        .blockers
        .contains(&ApplyPatchExecutionBlocker::UnsafePath));
    assert!(!temp.path().join("src/lib.rs").exists());
}

#[test]
#[cfg(unix)]
fn apply_patch_executor_rejects_symlinked_parent_or_target() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    symlink(outside.path(), temp.path().join("linked_docs")).unwrap();

    let mut parent_request = isolated_file_request(temp.path(), create_patch("linked_docs/new.md"));
    parent_request
        .allowed_paths
        .push("linked_docs/new.md".to_string());
    let parent_result = ApplyPatchExecutor.apply_to_isolated_root(parent_request);

    assert_eq!(
        parent_result.record.status,
        ApplyPatchExecutionStatus::Rejected
    );
    assert!(parent_result
        .record
        .blockers
        .contains(&ApplyPatchExecutionBlocker::SymlinkRejected));

    fs::write(outside.path().join("target.md"), "alpha\nold\nomega\n").unwrap();
    symlink(
        outside.path().join("target.md"),
        temp.path().join("docs/link.md"),
    )
    .unwrap();
    let target_result = ApplyPatchExecutor.apply_to_isolated_root(isolated_file_request(
        temp.path(),
        [
            "diff --git a/docs/link.md b/docs/link.md",
            "--- a/docs/link.md",
            "+++ b/docs/link.md",
            "@@ -1,3 +1,3 @@",
            " alpha",
            "-old",
            "+new",
            " omega",
            "",
        ]
        .join("\n"),
    ));

    assert_eq!(
        target_result.record.status,
        ApplyPatchExecutionStatus::Rejected
    );
    assert!(target_result
        .record
        .blockers
        .contains(&ApplyPatchExecutionBlocker::SymlinkRejected));
}

#[test]
fn apply_patch_executor_does_not_leave_partial_file_on_conflict() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("docs")).unwrap();
    fs::write(
        temp.path().join("docs/README.md"),
        "alpha\nchanged\nomega\n",
    )
    .unwrap();

    let result = ApplyPatchExecutor
        .apply_to_isolated_root(isolated_file_request(temp.path(), modify_patch()));

    assert_eq!(result.record.status, ApplyPatchExecutionStatus::Conflict);
    assert!(result
        .record
        .blockers
        .contains(&ApplyPatchExecutionBlocker::HunkConflict));
    assert_eq!(
        fs::read_to_string(temp.path().join("docs/README.md")).unwrap(),
        "alpha\nchanged\nomega\n"
    );
    let temp_entries = fs::read_dir(temp.path().join("docs"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains("tessera-tmp"))
        .count();
    assert_eq!(temp_entries, 0);
}
