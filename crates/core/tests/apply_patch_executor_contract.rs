use tessera_core::{ApplyPatchExecutor, ApplyPatchInMemoryRequest};
use tessera_protocol::{ApplyPatchExecutionBlocker, ApplyPatchExecutionStatus};

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
