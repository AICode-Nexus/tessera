use std::fs;
use std::path::{Component, Path, PathBuf};

use tessera_protocol::{
    ApplyPatchExecutionBlocker, ApplyPatchExecutionId, ApplyPatchExecutionRecord,
    ApplyPatchExecutionStatus, ApplyPatchPreflightId, ArtifactId, CodingWorkflowId,
    MutationRequestId, PatchProposalId, PolicyDecisionId, ReviewerGateId, SnapshotId, TaskId,
};

use crate::apply_patch_gate::ApplyPatchExecutorRootKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyPatchInMemoryRequest {
    pub patch_body: String,
    pub current_contents: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyPatchInMemoryResult {
    pub status: ApplyPatchExecutionStatus,
    pub affected_paths: Vec<String>,
    pub conflict_paths: Vec<String>,
    pub blockers: Vec<ApplyPatchExecutionBlocker>,
    pub new_contents: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyPatchIsolatedFileRequest {
    pub execution_id: ApplyPatchExecutionId,
    pub preflight_id: ApplyPatchPreflightId,
    pub workflow_id: CodingWorkflowId,
    pub task_id: TaskId,
    pub request_id: MutationRequestId,
    pub patch_id: PatchProposalId,
    pub checkpoint_id: Option<SnapshotId>,
    pub reviewer_gate_id: Option<ReviewerGateId>,
    pub policy_decision_id: Option<PolicyDecisionId>,
    pub sandbox_profile_label: Option<String>,
    pub isolated_root: PathBuf,
    pub isolated_root_label: String,
    pub root_kind: ApplyPatchExecutorRootKind,
    pub executor_label: String,
    pub allowed_paths: Vec<String>,
    pub patch_body: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyPatchIsolatedFileResult {
    pub record: ApplyPatchExecutionRecord,
}

#[derive(Clone, Debug, Default)]
pub struct ApplyPatchExecutor;

impl ApplyPatchExecutor {
    pub fn apply_in_memory(&self, request: ApplyPatchInMemoryRequest) -> ApplyPatchInMemoryResult {
        match parse_patch_document(&request.patch_body) {
            Ok(document) => apply_document(document, request.current_contents),
            Err(result) => result,
        }
    }

    pub fn apply_to_isolated_root(
        &self,
        request: ApplyPatchIsolatedFileRequest,
    ) -> ApplyPatchIsolatedFileResult {
        let document = match parse_patch_document(&request.patch_body) {
            Ok(document) => document,
            Err(result) => return isolated_result(&request, result),
        };

        let target_path = match validate_isolated_target(&request, &document.path) {
            Ok(target_path) => target_path,
            Err(result) => return isolated_result(&request, result),
        };

        let current_contents = match document.operation {
            PatchOperation::Create => {
                if target_path.exists() {
                    return isolated_result(
                        &request,
                        rejected(
                            vec![document.path],
                            vec![ApplyPatchExecutionBlocker::UnsupportedPatchOperation],
                        ),
                    );
                }
                None
            }
            PatchOperation::Modify => match fs::read_to_string(&target_path) {
                Ok(contents) => Some(contents),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return isolated_result(&request, conflict(vec![document.path]));
                }
                Err(_) => {
                    return isolated_result(
                        &request,
                        failed(
                            vec![document.path],
                            vec![ApplyPatchExecutionBlocker::WriteFailed],
                        ),
                    );
                }
            },
        };

        let result = apply_document(document, current_contents);
        let Some(new_contents) = result.new_contents.as_deref() else {
            return isolated_result(&request, result);
        };

        let write_result = match write_atomically(&target_path, new_contents) {
            Ok(()) => result,
            Err(_) => failed(
                result.affected_paths,
                vec![ApplyPatchExecutionBlocker::WriteFailed],
            ),
        };
        isolated_result(&request, write_result)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PatchOperation {
    Create,
    Modify,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PatchDocument {
    path: String,
    operation: PatchOperation,
    hunks: Vec<PatchHunk>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PatchHunk {
    old_lines: Vec<String>,
    new_lines: Vec<String>,
}

fn parse_patch_document(body: &str) -> Result<PatchDocument, ApplyPatchInMemoryResult> {
    if body.contains("\ndiff --git ") {
        return Err(rejected(
            Vec::new(),
            vec![ApplyPatchExecutionBlocker::MultipleFilesUnsupported],
        ));
    }

    if body.contains("\nBinary files ")
        || body.starts_with("Binary files ")
        || body.contains("\nrename from ")
        || body.contains("\nrename to ")
        || body.starts_with("rename from ")
        || body.starts_with("rename to ")
        || body.contains("\ndeleted file mode ")
        || body.starts_with("deleted file mode ")
        || body.contains("\n+++ /dev/null")
    {
        return Err(rejected(
            Vec::new(),
            vec![ApplyPatchExecutionBlocker::UnsupportedPatchOperation],
        ));
    }

    let mut path = None;
    let mut operation = PatchOperation::Modify;
    let mut hunks = Vec::new();
    let mut current_hunk: Option<PatchHunk> = None;
    let mut in_hunk = false;

    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            path = parse_git_diff_path(rest);
            continue;
        }

        if line.starts_with("new file mode ") || line == "--- /dev/null" {
            operation = PatchOperation::Create;
            continue;
        }

        if let Some(rest) = line.strip_prefix("+++ ") {
            if rest != "/dev/null" {
                path = normalize_patch_path(rest);
            }
            continue;
        }

        if line.starts_with("@@ ") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
            current_hunk = Some(PatchHunk {
                old_lines: Vec::new(),
                new_lines: Vec::new(),
            });
            in_hunk = true;
            continue;
        }

        if !in_hunk {
            continue;
        }

        let Some(hunk) = current_hunk.as_mut() else {
            continue;
        };

        if let Some(line) = line.strip_prefix(' ') {
            hunk.old_lines.push(line.to_string());
            hunk.new_lines.push(line.to_string());
        } else if let Some(line) = line.strip_prefix('-') {
            hunk.old_lines.push(line.to_string());
        } else if let Some(line) = line.strip_prefix('+') {
            hunk.new_lines.push(line.to_string());
        }
    }

    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    let Some(path) = path else {
        return Err(rejected(
            Vec::new(),
            vec![ApplyPatchExecutionBlocker::UnsafePath],
        ));
    };

    if !is_safe_relative_path(&path) {
        return Err(rejected(
            vec![path],
            vec![ApplyPatchExecutionBlocker::UnsafePath],
        ));
    }

    if hunks.is_empty() {
        return Err(rejected(
            vec![path],
            vec![ApplyPatchExecutionBlocker::UnsupportedPatchOperation],
        ));
    }

    Ok(PatchDocument {
        path,
        operation,
        hunks,
    })
}

fn apply_document(
    document: PatchDocument,
    current_contents: Option<String>,
) -> ApplyPatchInMemoryResult {
    match document.operation {
        PatchOperation::Create => apply_create(document, current_contents),
        PatchOperation::Modify => apply_modify(document, current_contents),
    }
}

fn apply_create(
    document: PatchDocument,
    current_contents: Option<String>,
) -> ApplyPatchInMemoryResult {
    if current_contents.is_some() {
        return rejected(
            vec![document.path],
            vec![ApplyPatchExecutionBlocker::UnsupportedPatchOperation],
        );
    }

    let mut lines = Vec::new();
    for hunk in document.hunks {
        lines.extend(hunk.new_lines);
    }

    applied(vec![document.path], join_lines(&lines))
}

fn apply_modify(
    document: PatchDocument,
    current_contents: Option<String>,
) -> ApplyPatchInMemoryResult {
    let Some(current_contents) = current_contents else {
        return conflict(vec![document.path]);
    };

    let mut current_lines = split_contents(&current_contents);
    let mut search_start = 0;
    for hunk in document.hunks {
        let Some(index) = find_sequence(&current_lines, &hunk.old_lines, search_start) else {
            return conflict(vec![document.path]);
        };
        current_lines.splice(index..index + hunk.old_lines.len(), hunk.new_lines.clone());
        search_start = index + hunk.new_lines.len();
    }

    applied(vec![document.path], join_lines(&current_lines))
}

fn validate_isolated_target(
    request: &ApplyPatchIsolatedFileRequest,
    relative_path: &str,
) -> Result<PathBuf, ApplyPatchInMemoryResult> {
    if request.root_kind != ApplyPatchExecutorRootKind::IsolatedWorktree
        || request.isolated_root_label.trim().is_empty()
        || request.isolated_root_label == "project"
        || request.isolated_root_label == "local"
    {
        return Err(rejected(
            vec![relative_path.to_string()],
            vec![ApplyPatchExecutionBlocker::PrimaryRootRejected],
        ));
    }

    if !is_safe_relative_path(relative_path)
        || !request
            .allowed_paths
            .iter()
            .any(|path| path == relative_path)
    {
        return Err(rejected(
            vec![relative_path.to_string()],
            vec![ApplyPatchExecutionBlocker::UnsafePath],
        ));
    }

    let root = fs::canonicalize(&request.isolated_root).map_err(|_| {
        rejected(
            vec![relative_path.to_string()],
            vec![ApplyPatchExecutionBlocker::MissingIsolatedRoot],
        )
    })?;
    reject_symlinked_components(&root, relative_path)?;

    let target = root.join(relative_path);
    let Some(parent) = target.parent() else {
        return Err(rejected(
            vec![relative_path.to_string()],
            vec![ApplyPatchExecutionBlocker::UnsafePath],
        ));
    };
    let canonical_parent = fs::canonicalize(parent).map_err(|_| {
        failed(
            vec![relative_path.to_string()],
            vec![ApplyPatchExecutionBlocker::WriteFailed],
        )
    })?;

    if !canonical_parent.starts_with(&root) {
        return Err(rejected(
            vec![relative_path.to_string()],
            vec![ApplyPatchExecutionBlocker::UnsafePath],
        ));
    }

    Ok(target)
}

fn reject_symlinked_components(
    root: &Path,
    relative_path: &str,
) -> Result<(), ApplyPatchInMemoryResult> {
    let relative = Path::new(relative_path);
    let components = relative.components().collect::<Vec<_>>();
    let mut cursor = root.to_path_buf();

    for (index, component) in components.iter().enumerate() {
        cursor.push(component.as_os_str());
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(rejected(
                    vec![relative_path.to_string()],
                    vec![ApplyPatchExecutionBlocker::SymlinkRejected],
                ));
            }
            Ok(_) => {}
            Err(error)
                if index + 1 == components.len()
                    && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(failed(
                    vec![relative_path.to_string()],
                    vec![ApplyPatchExecutionBlocker::WriteFailed],
                ));
            }
        }
    }

    Ok(())
}

fn write_atomically(target_path: &Path, contents: &str) -> std::io::Result<()> {
    let parent = target_path
        .parent()
        .ok_or_else(|| std::io::Error::other("target path has no parent"))?;
    let temp_path = parent.join(format!(
        ".tessera-tmp-{}",
        target_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("apply-patch")
    ));

    if let Err(error) = fs::write(&temp_path, contents) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    if let Err(error) = fs::rename(&temp_path, target_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    Ok(())
}

fn parse_git_diff_path(rest: &str) -> Option<String> {
    rest.split_whitespace()
        .nth(1)
        .and_then(normalize_patch_path)
}

fn normalize_patch_path(path: &str) -> Option<String> {
    Some(
        path.strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .unwrap_or(path)
            .to_string(),
    )
}

fn is_safe_relative_path(path: &str) -> bool {
    let path = path.trim();
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && !Path::new(path).components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn split_contents(contents: &str) -> Vec<String> {
    let mut lines = contents
        .split('\n')
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if contents.ends_with('\n') {
        lines.pop();
    }
    lines
}

fn join_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn find_sequence(lines: &[String], needle: &[String], start: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(start.min(lines.len()));
    }
    lines
        .windows(needle.len())
        .enumerate()
        .skip(start)
        .find_map(|(index, window)| (window == needle).then_some(index))
}

fn applied(affected_paths: Vec<String>, new_contents: String) -> ApplyPatchInMemoryResult {
    ApplyPatchInMemoryResult {
        status: ApplyPatchExecutionStatus::Applied,
        affected_paths,
        conflict_paths: Vec::new(),
        blockers: Vec::new(),
        new_contents: Some(new_contents),
    }
}

fn rejected(
    affected_paths: Vec<String>,
    blockers: Vec<ApplyPatchExecutionBlocker>,
) -> ApplyPatchInMemoryResult {
    ApplyPatchInMemoryResult {
        status: ApplyPatchExecutionStatus::Rejected,
        affected_paths,
        conflict_paths: Vec::new(),
        blockers,
        new_contents: None,
    }
}

fn failed(
    affected_paths: Vec<String>,
    blockers: Vec<ApplyPatchExecutionBlocker>,
) -> ApplyPatchInMemoryResult {
    ApplyPatchInMemoryResult {
        status: ApplyPatchExecutionStatus::Failed,
        affected_paths,
        conflict_paths: Vec::new(),
        blockers,
        new_contents: None,
    }
}

fn conflict(conflict_paths: Vec<String>) -> ApplyPatchInMemoryResult {
    ApplyPatchInMemoryResult {
        status: ApplyPatchExecutionStatus::Conflict,
        affected_paths: conflict_paths.clone(),
        conflict_paths,
        blockers: vec![ApplyPatchExecutionBlocker::HunkConflict],
        new_contents: None,
    }
}

fn isolated_result(
    request: &ApplyPatchIsolatedFileRequest,
    result: ApplyPatchInMemoryResult,
) -> ApplyPatchIsolatedFileResult {
    ApplyPatchIsolatedFileResult {
        record: ApplyPatchExecutionRecord {
            execution_id: request.execution_id.clone(),
            preflight_id: request.preflight_id.clone(),
            workflow_id: request.workflow_id.clone(),
            task_id: request.task_id.clone(),
            request_id: request.request_id.clone(),
            patch_id: request.patch_id.clone(),
            checkpoint_id: request.checkpoint_id.clone(),
            reviewer_gate_id: request.reviewer_gate_id.clone(),
            policy_decision_id: request.policy_decision_id.clone(),
            sandbox_profile_label: request.sandbox_profile_label.clone(),
            isolated_root_label: request.isolated_root_label.clone(),
            executor_label: request.executor_label.clone(),
            status: result.status,
            blockers: result.blockers,
            affected_paths: result.affected_paths,
            conflict_paths: result.conflict_paths,
            artifact_refs: Vec::<ArtifactId>::new(),
            evidence: Vec::new(),
        },
    }
}
