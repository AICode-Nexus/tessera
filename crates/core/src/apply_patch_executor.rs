use std::path::{Component, Path};

use tessera_protocol::{ApplyPatchExecutionBlocker, ApplyPatchExecutionStatus};

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

#[derive(Clone, Debug, Default)]
pub struct ApplyPatchExecutor;

impl ApplyPatchExecutor {
    pub fn apply_in_memory(&self, request: ApplyPatchInMemoryRequest) -> ApplyPatchInMemoryResult {
        match parse_patch_document(&request.patch_body) {
            Ok(document) => apply_document(document, request.current_contents),
            Err(result) => result,
        }
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

fn conflict(conflict_paths: Vec<String>) -> ApplyPatchInMemoryResult {
    ApplyPatchInMemoryResult {
        status: ApplyPatchExecutionStatus::Conflict,
        affected_paths: conflict_paths.clone(),
        conflict_paths,
        blockers: vec![ApplyPatchExecutionBlocker::HunkConflict],
        new_contents: None,
    }
}
