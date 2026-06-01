use std::{
    collections::VecDeque,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use tessera_cli::{
    build_tui_state_with_config, format_resumable_task_lines, format_task_owner_lines, list_events,
    list_resumable_tasks, list_sessions, list_task_owners, parse_repl_command, resolve_config,
    resolve_data_dir_with_config, run_chat_mock, run_chat_repl_with_io_and_resume,
    run_chat_with_config, run_chat_with_config_and_controls_and_events,
    run_chat_with_config_and_events, run_doctor, run_repl_prompt_with_writer,
    write_config_template, CliReplCommand, CliReplSession, DoctorReport,
};
use tessera_config::{ProviderProfile, TesseraConfig};
use tessera_core::{
    EventSinkAction, RunCancellationToken, RunControls, RunPauseToken, RuntimeReader,
};
use tessera_protocol::{
    AgentHandoffId, ApplyPatchDryRunOperationKind, ApplyPatchDryRunOperationSummary,
    ApplyPatchExecutionBlocker, ApplyPatchExecutionId, ApplyPatchExecutionRecord,
    ApplyPatchExecutionStatus, ApplyPatchPreflightBlocker, ApplyPatchPreflightId,
    ApplyPatchPreflightRecord, ApplyPatchPreflightStatus, ApprovalId, ArtifactBodyRecord,
    ArtifactBodyRedactionStatus, ArtifactId, ArtifactKind, ClientInstanceId,
    CodingWorkflowEvidenceRedactionStatus, CodingWorkflowId, EventFrame, EventRange,
    HandoffEvidenceKind, HandoffEvidenceRef, MutationMode, MutationRequestId,
    MutationRequestOperationKind, MutationRequestProposal, MutationRequestStatus, PatchProposal,
    PatchProposalId, PolicyDecisionId, PolicyOutcome, RestorePlanId, RestorePlanRecord,
    ReviewBundle, ReviewBundleId, ReviewerDecisionKind, ReviewerGateDecision, ReviewerGateId,
    ReviewerGateRequest, RunEvent, RuntimeInstanceId, SnapshotId, SubagentApprovalForwardingRecord,
    SubagentApprovalForwardingStatus, SubagentSessionId, TaskId, TaskOwnerKind, TaskOwnerLease,
    TaskOwnerStatus, TaskOwnershipId, TaskStatus, TestEvidenceSummaryId, TestEvidenceSummaryRecord,
    TestEvidenceSummaryStatus, TestPlanId, TestPlanRecord, TestRunId, TestRunRecord, TestRunStatus,
    Timestamp, ToolCallId, ToolId, ToolPermission, ToolPolicyDecision, ToolSideEffect,
    WorkspaceCheckpointLifecycleRecord, WorkspaceCheckpointLifecycleStatus, WorkspaceMutationScope,
    WorkspaceWorktreeId, WorkspaceWorktreeLifecycleRecord, WorkspaceWorktreeLifecycleStatus,
};
use tessera_storage::{ArtifactBodyWrite, TraceStore};

struct DelayedLineReader {
    lines: VecDeque<DelayedLine>,
    buffer: Vec<u8>,
    offset: usize,
}

enum DelayedLine {
    After {
        delay: Duration,
        line: String,
    },
    AfterTraceEvent {
        data_dir: PathBuf,
        event_kind: &'static str,
        line: String,
    },
}

impl DelayedLine {
    fn after(delay: Duration, line: impl Into<String>) -> Self {
        Self::After {
            delay,
            line: line.into(),
        }
    }

    fn after_trace_event(
        data_dir: PathBuf,
        event_kind: &'static str,
        line: impl Into<String>,
    ) -> Self {
        Self::AfterTraceEvent {
            data_dir,
            event_kind,
            line: line.into(),
        }
    }
}

impl DelayedLineReader {
    fn with_steps<I>(lines: I) -> Self
    where
        I: IntoIterator<Item = DelayedLine>,
    {
        Self {
            lines: lines.into_iter().collect(),
            buffer: Vec::new(),
            offset: 0,
        }
    }

    fn refill(&mut self) {
        if self.offset < self.buffer.len() {
            return;
        }
        self.buffer.clear();
        self.offset = 0;
        let Some(line) = self.lines.pop_front() else {
            return;
        };
        let line = match line {
            DelayedLine::After { delay, line } => {
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
                line
            }
            DelayedLine::AfterTraceEvent {
                data_dir,
                event_kind,
                line,
            } => {
                wait_for_trace_event(&data_dir, event_kind);
                line
            }
        };
        self.buffer.extend_from_slice(line.as_bytes());
    }
}

fn wait_for_trace_event(data_dir: &Path, event_kind: &str) {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(2) {
        if trace_contains_event(data_dir, event_kind) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn trace_contains_event(data_dir: &Path, event_kind: &str) -> bool {
    let Ok(sessions) = list_sessions(data_dir) else {
        return false;
    };

    sessions.iter().any(|session| {
        let Ok(page) = list_events(data_dir, &session.trace_id, None, None) else {
            return false;
        };
        page.records
            .iter()
            .any(|record| record.event_kind == event_kind)
    })
}

impl Read for DelayedLineReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        self.refill();
        if self.offset >= self.buffer.len() {
            return Ok(0);
        }
        let len = output.len().min(self.buffer.len() - self.offset);
        output[..len].copy_from_slice(&self.buffer[self.offset..self.offset + len]);
        self.offset += len;
        Ok(len)
    }
}

impl io::BufRead for DelayedLineReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.refill();
        Ok(&self.buffer[self.offset..])
    }

    fn consume(&mut self, amount: usize) {
        self.offset = (self.offset + amount).min(self.buffer.len());
    }
}

fn apply_patch_modify_body() -> String {
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

fn apply_patch_args(
    data_dir: &std::path::Path,
    isolated_root: &std::path::Path,
    patch_file: &std::path::Path,
) -> Vec<String> {
    vec![
        "apply-patch".to_string(),
        "--data-dir".to_string(),
        data_dir.display().to_string(),
        "--trace-id".to_string(),
        "trace_cli_apply_patch".to_string(),
        "--workflow-id".to_string(),
        "coding_workflow_cli_apply_patch".to_string(),
        "--task-id".to_string(),
        "task_cli_apply_patch".to_string(),
        "--request-id".to_string(),
        "mutation_request_cli_apply_patch".to_string(),
        "--patch-id".to_string(),
        "patch_proposal_cli_apply_patch".to_string(),
        "--preflight-id".to_string(),
        "apply_patch_preflight_cli".to_string(),
        "--execution-id".to_string(),
        "apply_patch_execution_cli".to_string(),
        "--checkpoint-id".to_string(),
        "snapshot_cli_apply_patch".to_string(),
        "--reviewer-gate-id".to_string(),
        "reviewer_gate_cli_apply_patch".to_string(),
        "--policy-decision-id".to_string(),
        "policy_cli_apply_patch".to_string(),
        "--sandbox-profile".to_string(),
        "workspace_write_isolated".to_string(),
        "--isolated-root".to_string(),
        isolated_root.display().to_string(),
        "--root-label".to_string(),
        "worktree:cli-apply-patch".to_string(),
        "--allowed-path".to_string(),
        "docs/README.md".to_string(),
        "--patch-file".to_string(),
        patch_file.display().to_string(),
    ]
}

fn trace_apply_patch_args(
    data_dir: &std::path::Path,
    isolated_root: &std::path::Path,
) -> Vec<String> {
    vec![
        "apply-patch".to_string(),
        "--data-dir".to_string(),
        data_dir.display().to_string(),
        "--from-trace".to_string(),
        "trace_cli_trace_apply_patch".to_string(),
        "--isolated-root".to_string(),
        isolated_root.display().to_string(),
        "--root-label".to_string(),
        "worktree:cli-trace-apply-patch".to_string(),
        "--json".to_string(),
    ]
}

fn trace_auto_worktree_apply_patch_args(
    data_dir: &std::path::Path,
    source_root: &std::path::Path,
    worktree_base: &std::path::Path,
) -> Vec<String> {
    vec![
        "apply-patch".to_string(),
        "--data-dir".to_string(),
        data_dir.display().to_string(),
        "--from-trace".to_string(),
        "trace_cli_trace_apply_patch".to_string(),
        "--auto-worktree".to_string(),
        "--source-root".to_string(),
        source_root.display().to_string(),
        "--worktree-base".to_string(),
        worktree_base.display().to_string(),
        "--json".to_string(),
    ]
}

fn worktree_cleanup_args(
    data_dir: &std::path::Path,
    source_root: &std::path::Path,
    worktree_id: &str,
    worktree_path: &std::path::Path,
) -> Vec<String> {
    vec![
        "worktree".to_string(),
        "cleanup".to_string(),
        "--data-dir".to_string(),
        data_dir.display().to_string(),
        "--from-trace".to_string(),
        "trace_cli_trace_apply_patch".to_string(),
        "--worktree-id".to_string(),
        worktree_id.to_string(),
        "--worktree-path".to_string(),
        worktree_path.display().to_string(),
        "--source-root".to_string(),
        source_root.display().to_string(),
        "--json".to_string(),
    ]
}

fn worktree_list_args(data_dir: &std::path::Path, json: bool) -> Vec<String> {
    let mut args = vec![
        "worktree".to_string(),
        "list".to_string(),
        "--data-dir".to_string(),
        data_dir.display().to_string(),
        "--trace".to_string(),
        "trace_cli_trace_apply_patch".to_string(),
    ];
    if json {
        args.push("--json".to_string());
    }
    args
}

fn workflow_inspect_args(data_dir: &std::path::Path, json: bool) -> Vec<String> {
    let mut args = vec![
        "workflow".to_string(),
        "inspect".to_string(),
        "--data-dir".to_string(),
        data_dir.display().to_string(),
        "--trace".to_string(),
        "trace_cli_workflow_inspect".to_string(),
    ];
    if json {
        args.push("--json".to_string());
    }
    args
}

fn trace_workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from_static("coding_workflow_cli_trace_apply_patch")
}

fn workflow_inspect_workflow_id() -> CodingWorkflowId {
    CodingWorkflowId::from("/Users/admin/work/tessera/.env::sk-secret-workflow-id")
}

fn workflow_inspect_task_id() -> TaskId {
    TaskId::from("/Users/admin/work/tessera/.env::sk-secret-task-id")
}

fn workflow_inspect_gate_id(suffix: &str) -> ReviewerGateId {
    ReviewerGateId::from(format!(
        "/Users/admin/work/tessera/.env::sk-secret-gate-{suffix}"
    ))
}

fn workflow_inspect_evidence(label: &str, summary: &str) -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::DiffArtifact,
        artifact_id: Some(ArtifactId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-artifact-{label}"
        ))),
        trace_id: Some(format!(
            "/Users/admin/work/tessera/.env::sk-secret-trace-{label}"
        )),
        event_range: Some(EventRange {
            start_seq: 3,
            end_seq: 8,
        }),
        label: Some(label.to_string()),
        summary: Some(summary.to_string()),
    }
}

fn workflow_inspect_scope() -> WorkspaceMutationScope {
    WorkspaceMutationScope {
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        root_label: "/Users/admin/work/tessera/.env::sk-secret-root-label".to_string(),
        allowed_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-allowed".to_string()],
        denied_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-denied".to_string()],
        mutation_mode: MutationMode::WorktreeFirst,
        worktree_required: true,
        reason: Some("/Users/admin/work/tessera/.env::sk-secret-scope-reason".to_string()),
    }
}

fn workflow_inspect_patch_proposal() -> PatchProposal {
    PatchProposal {
        patch_id: PatchProposalId::from("/Users/admin/work/tessera/.env::sk-secret-patch-id"),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        summary: "/Users/admin/work/tessera/.env::sk-secret-patch-summary".to_string(),
        touched_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-touched-path".to_string()],
        diff_artifacts: vec![
            workflow_inspect_evidence(
                "/Users/admin/work/tessera/.env::sk-secret-diff-label",
                "/Users/admin/work/tessera/.env::sk-secret-diff-summary",
            ),
            HandoffEvidenceRef {
                kind: HandoffEvidenceKind::TraceRange,
                artifact_id: None,
                trace_id: Some(
                    "/Users/admin/work/tessera/.env::sk-secret-trace-only-diff-trace".to_string(),
                ),
                event_range: Some(EventRange {
                    start_seq: 21,
                    end_seq: 34,
                }),
                label: Some(
                    "/Users/admin/work/tessera/.env::sk-secret-trace-only-diff-label".to_string(),
                ),
                summary: Some(
                    "/Users/admin/work/tessera/.env::sk-secret-trace-only-diff-summary".to_string(),
                ),
            },
        ],
        risk_labels: vec!["/Users/admin/work/tessera/.env::sk-secret-risk-label".to_string()],
        required_checkpoint_id: Some(SnapshotId::from(
            "/Users/admin/work/tessera/.env::sk-secret-checkpoint-id",
        )),
        reviewer_gate_id: Some(workflow_inspect_gate_id("accepted")),
    }
}

fn workflow_inspect_mutation_request() -> MutationRequestProposal {
    MutationRequestProposal {
        request_id: MutationRequestId::from(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-request-id",
        ),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        operation: MutationRequestOperationKind::PatchApplication,
        status: MutationRequestStatus::ReviewerPending,
        summary: "/Users/admin/work/tessera/.env::sk-secret-mutation-summary".to_string(),
        requested_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-requested-path".to_string()
        ],
        required_checkpoint_id: Some(SnapshotId::from(
            "/Users/admin/work/tessera/.env::sk-secret-checkpoint-id",
        )),
        reviewer_gate_id: Some(workflow_inspect_gate_id("accepted")),
        policy_decision_id: Some(PolicyDecisionId::from(
            "/Users/admin/work/tessera/.env::sk-secret-policy-id",
        )),
        sandbox_profile_label: Some(
            "/Users/admin/work/tessera/.env::sk-secret-sandbox-label".to_string(),
        ),
        worktree_required: true,
        evidence: vec![workflow_inspect_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-mutation-evidence-summary",
        )],
    }
}

fn workflow_inspect_review_bundle(gate_suffix: &str) -> ReviewBundle {
    ReviewBundle {
        review_bundle_id: ReviewBundleId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-review-bundle-{gate_suffix}"
        )),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        reviewer_gate_id: workflow_inspect_gate_id(gate_suffix),
        patch_ids: vec![PatchProposalId::from(
            "/Users/admin/work/tessera/.env::sk-secret-patch-id",
        )],
        test_run_ids: vec![TestRunId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-run-id",
        )],
        evidence: vec![workflow_inspect_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-review-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-review-evidence-summary",
        )],
        summary: "/Users/admin/work/tessera/.env::sk-secret-review-summary".to_string(),
    }
}

fn workflow_inspect_gate_request(gate_suffix: &str) -> ReviewerGateRequest {
    ReviewerGateRequest {
        gate_id: workflow_inspect_gate_id(gate_suffix),
        handoff_id: AgentHandoffId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-handoff-{gate_suffix}"
        )),
        parent_task_id: workflow_inspect_task_id(),
        requested_decisions: vec![
            ReviewerDecisionKind::Accept,
            ReviewerDecisionKind::Reject,
            ReviewerDecisionKind::RequestRevision,
        ],
        evidence: vec![workflow_inspect_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-gate-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-gate-evidence-summary",
        )],
    }
}

fn workflow_inspect_gate_decision(
    gate_suffix: &str,
    decision: ReviewerDecisionKind,
) -> ReviewerGateDecision {
    ReviewerGateDecision {
        gate_id: workflow_inspect_gate_id(gate_suffix),
        handoff_id: AgentHandoffId::from(format!(
            "/Users/admin/work/tessera/.env::sk-secret-handoff-{gate_suffix}"
        )),
        decision,
        reviewer: "/Users/admin/work/tessera/.env::sk-secret-reviewer".to_string(),
        reason_code: "/Users/admin/work/tessera/.env::sk-secret-reviewer-reason".to_string(),
        comment: Some("/Users/admin/work/tessera/.env::sk-secret-reviewer-comment".to_string()),
    }
}

fn workflow_inspect_pending_approval_event() -> RunEvent {
    RunEvent::ToolPolicyDecisionRecorded {
        decision: ToolPolicyDecision {
            decision_id: PolicyDecisionId::from(
                "/Users/admin/work/tessera/.env::sk-secret-policy-id",
            ),
            call_id: ToolCallId::from("/Users/admin/work/tessera/.env::sk-secret-tool-call-id"),
            tool_id: ToolId::from("/Users/admin/work/tessera/.env::sk-secret-tool-id"),
            outcome: PolicyOutcome::AskUser,
            reason: "/Users/admin/work/tessera/.env::sk-secret-approval-reason".to_string(),
            required_permissions: vec![ToolPermission::FilesystemWrite],
            side_effects: vec![ToolSideEffect::WritesWorkspace],
            approval_id: Some(ApprovalId::from(
                "/Users/admin/work/tessera/.env::sk-secret-approval-id",
            )),
        },
    }
}

fn workflow_inspect_approval_forwarding_event() -> RunEvent {
    RunEvent::SubagentApprovalForwardingRecorded {
        forwarding: SubagentApprovalForwardingRecord {
            session_id: SubagentSessionId::from(
                "/Users/admin/work/tessera/.env::sk-secret-session-id",
            ),
            parent_task_id: workflow_inspect_task_id(),
            approval_id: ApprovalId::from("/Users/admin/work/tessera/.env::sk-secret-approval-id"),
            reviewer_gate_id: Some(workflow_inspect_gate_id("accepted")),
            status: SubagentApprovalForwardingStatus::QueuedForReviewer,
            reason: "/Users/admin/work/tessera/.env::sk-secret-forwarding-reason".to_string(),
        },
    }
}

fn workflow_inspect_apply_patch_preflight() -> ApplyPatchPreflightRecord {
    ApplyPatchPreflightRecord {
        preflight_id: ApplyPatchPreflightId::from(
            "/Users/admin/work/tessera/.env::sk-secret-preflight-id",
        ),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        request_id: MutationRequestId::from(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-request-id",
        ),
        patch_id: PatchProposalId::from("/Users/admin/work/tessera/.env::sk-secret-patch-id"),
        status: ApplyPatchPreflightStatus::Blocked,
        blockers: vec![ApplyPatchPreflightBlocker::ExecutorUnavailable],
        affected_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-preflight-affected-path".to_string(),
        ],
        operations: vec![ApplyPatchDryRunOperationSummary {
            path: "/Users/admin/work/tessera/.env::sk-secret-preflight-operation-path".to_string(),
            operation: ApplyPatchDryRunOperationKind::Modify,
        }],
        executor_blocked: true,
        executor_block_reason: "/Users/admin/work/tessera/.env::sk-secret-executor-block-reason"
            .to_string(),
        evidence: vec![workflow_inspect_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-preflight-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-preflight-evidence-summary",
        )],
    }
}

fn workflow_inspect_apply_patch_execution() -> ApplyPatchExecutionRecord {
    ApplyPatchExecutionRecord {
        execution_id: ApplyPatchExecutionId::from(
            "/Users/admin/work/tessera/.env::sk-secret-execution-id",
        ),
        preflight_id: ApplyPatchPreflightId::from(
            "/Users/admin/work/tessera/.env::sk-secret-preflight-id",
        ),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        request_id: MutationRequestId::from(
            "/Users/admin/work/tessera/.env::sk-secret-mutation-request-id",
        ),
        patch_id: PatchProposalId::from("/Users/admin/work/tessera/.env::sk-secret-patch-id"),
        checkpoint_id: Some(SnapshotId::from(
            "/Users/admin/work/tessera/.env::sk-secret-checkpoint-id",
        )),
        reviewer_gate_id: Some(workflow_inspect_gate_id("accepted")),
        policy_decision_id: Some(PolicyDecisionId::from(
            "/Users/admin/work/tessera/.env::sk-secret-policy-id",
        )),
        sandbox_profile_label: Some(
            "/Users/admin/work/tessera/.env::sk-secret-sandbox-label".to_string(),
        ),
        isolated_root_label: "/Users/admin/work/tessera/.env::sk-secret-isolated-root".to_string(),
        executor_label: "/Users/admin/work/tessera/.env::sk-secret-executor-label".to_string(),
        status: ApplyPatchExecutionStatus::Failed,
        blockers: vec![ApplyPatchExecutionBlocker::HunkConflict],
        affected_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-execution-affected-path".to_string(),
        ],
        conflict_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-execution-conflict-path".to_string(),
        ],
        artifact_refs: vec![ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-execution-artifact",
        )],
        evidence: vec![workflow_inspect_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-execution-evidence-label",
            "/Users/admin/work/tessera/.env::sk-secret-execution-evidence-summary",
        )],
    }
}

fn workflow_inspect_restore_plan() -> RestorePlanRecord {
    RestorePlanRecord {
        restore_plan_id: RestorePlanId::from(
            "/Users/admin/work/tessera/.env::sk-secret-restore-plan-id",
        ),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        checkpoint_id: SnapshotId::from("/Users/admin/work/tessera/.env::sk-secret-checkpoint-id"),
        target_paths: vec![
            "/Users/admin/work/tessera/.env::sk-secret-restore-target-path".to_string(),
        ],
        reason: "/Users/admin/work/tessera/.env::sk-secret-restore-reason".to_string(),
        execution_blocked: true,
    }
}

fn workflow_inspect_test_plan() -> TestPlanRecord {
    TestPlanRecord {
        test_plan_id: TestPlanId::from("/Users/admin/work/tessera/.env::sk-secret-test-plan-id"),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        command_labels: vec![
            "/Users/admin/work/tessera/.env::sk-secret-test-command-label".to_string(),
        ],
        affected_paths: vec!["/Users/admin/work/tessera/.env::sk-secret-test-path".to_string()],
        required_artifact_kinds: vec![ArtifactKind::TestReport],
    }
}

fn workflow_inspect_test_run() -> TestRunRecord {
    TestRunRecord {
        test_run_id: TestRunId::from("/Users/admin/work/tessera/.env::sk-secret-test-run-id"),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        test_plan_id: Some(TestPlanId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-plan-id",
        )),
        command_label: "/Users/admin/work/tessera/.env::sk-secret-test-run-command".to_string(),
        status: TestRunStatus::Failed,
        exit_code: Some(101),
        duration_ms: Some(500),
        stdout_artifact_id: Some(ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-stdout-artifact-label",
        )),
        stderr_artifact_id: Some(ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-stderr-artifact-label",
        )),
        diagnostics: vec![workflow_inspect_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-diagnostic-label",
            "/Users/admin/work/tessera/.env::sk-secret-diagnostic-summary",
        )],
        redaction_status: CodingWorkflowEvidenceRedactionStatus::Redacted,
    }
}

fn workflow_inspect_test_evidence_summary() -> TestEvidenceSummaryRecord {
    TestEvidenceSummaryRecord {
        summary_id: TestEvidenceSummaryId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-summary-id",
        ),
        workflow_id: workflow_inspect_workflow_id(),
        task_id: workflow_inspect_task_id(),
        test_plan_ids: vec![TestPlanId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-plan-id",
        )],
        test_run_ids: vec![TestRunId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-run-id",
        )],
        status: TestEvidenceSummaryStatus::Failed,
        total_runs: 1,
        passed_runs: 0,
        failed_runs: 1,
        cancelled_runs: 0,
        error_runs: 0,
        required_artifact_kinds: vec![ArtifactKind::TestReport],
        artifact_refs: vec![ArtifactId::from(
            "/Users/admin/work/tessera/.env::sk-secret-test-artifact-body-text",
        )],
        diagnostics: vec![workflow_inspect_evidence(
            "/Users/admin/work/tessera/.env::sk-secret-test-evidence-diagnostic-label",
            "/Users/admin/work/tessera/.env::sk-secret-test-evidence-diagnostic-summary",
        )],
        redaction_status: CodingWorkflowEvidenceRedactionStatus::Redacted,
        summary: "/Users/admin/work/tessera/.env::sk-secret-test-evidence-summary".to_string(),
        execution_blocked: true,
    }
}

fn workflow_inspect_artifact_body_event() -> RunEvent {
    RunEvent::ArtifactBodyRecorded {
        record: ArtifactBodyRecord {
            artifact_id: ArtifactId::from(
                "/Users/admin/work/tessera/.env::sk-secret-artifact-body-id",
            ),
            kind: ArtifactKind::Patch,
            task_id: Some(workflow_inspect_task_id()),
            media_type: "text/x-patch".to_string(),
            byte_len: 42,
            storage_uri: "/Users/admin/work/tessera/.env::sk-secret-artifact-body-storage"
                .to_string(),
            redaction_status: ArtifactBodyRedactionStatus::Redacted,
            summary: Some(
                "/Users/admin/work/tessera/.env::sk-secret-artifact-body-summary".to_string(),
            ),
            metadata: None,
        },
    }
}

fn write_workflow_inspection_trace(data_dir: &Path) {
    let mut store = TraceStore::open(data_dir).unwrap();
    for (index, event) in [
        RunEvent::CodingWorkflowStarted {
            workflow_id: workflow_inspect_workflow_id(),
            task_id: workflow_inspect_task_id(),
            objective: "/Users/admin/work/tessera/.env::sk-secret-objective".to_string(),
        },
        RunEvent::WorkspaceMutationScopeRecorded {
            scope: workflow_inspect_scope(),
        },
        RunEvent::PatchProposalRecorded {
            proposal: workflow_inspect_patch_proposal(),
        },
        RunEvent::MutationRequestProposalRecorded {
            proposal: workflow_inspect_mutation_request(),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspect_review_bundle("accepted"),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspect_review_bundle("rejected"),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspect_review_bundle("revision"),
        },
        RunEvent::ReviewBundleRecorded {
            bundle: workflow_inspect_review_bundle("pending"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspect_gate_request("accepted"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspect_gate_request("rejected"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspect_gate_request("revision"),
        },
        RunEvent::ReviewerGateRequested {
            request: workflow_inspect_gate_request("pending"),
        },
        RunEvent::ReviewerGateResolved {
            decision: workflow_inspect_gate_decision("accepted", ReviewerDecisionKind::Accept),
        },
        RunEvent::ReviewerGateResolved {
            decision: workflow_inspect_gate_decision("rejected", ReviewerDecisionKind::Reject),
        },
        RunEvent::ReviewerGateResolved {
            decision: workflow_inspect_gate_decision(
                "revision",
                ReviewerDecisionKind::RequestRevision,
            ),
        },
        workflow_inspect_pending_approval_event(),
        workflow_inspect_approval_forwarding_event(),
        RunEvent::ApplyPatchPreflightRecorded {
            record: workflow_inspect_apply_patch_preflight(),
        },
        RunEvent::ApplyPatchExecutionRecorded {
            record: workflow_inspect_apply_patch_execution(),
        },
        RunEvent::RestorePlanRecorded {
            plan: workflow_inspect_restore_plan(),
        },
        RunEvent::TestPlanRecorded {
            plan: workflow_inspect_test_plan(),
        },
        RunEvent::TestRunRecorded {
            record: workflow_inspect_test_run(),
        },
        RunEvent::TestEvidenceSummaryRecorded {
            record: workflow_inspect_test_evidence_summary(),
        },
        workflow_inspect_artifact_body_event(),
    ]
    .into_iter()
    .enumerate()
    {
        store
            .append(&EventFrame::new(
                "trace_cli_workflow_inspect",
                index as u64 + 1,
                event,
            ))
            .unwrap();
    }
}

fn write_empty_workflow_inspection_trace(data_dir: &Path) {
    TraceStore::open(data_dir).unwrap();
    std::fs::write(data_dir.join("traces/trace_cli_workflow_inspect.jsonl"), "").unwrap();
}

fn assert_workflow_inspect_output_is_safe(output: &str) {
    for leaked in [
        "/Users/admin/work/tessera/.env",
        "sk-secret",
        "root-label",
        "allowed",
        "denied",
        "objective",
        "patch-summary",
        "mutation-summary",
        "requested-path",
        "touched-path",
        "review-summary",
        "diff-label",
        "diff-summary",
        "trace-only-diff-label",
        "trace-only-diff-summary",
        "review-evidence-label",
        "review-evidence-summary",
        "reviewer-comment",
        "reviewer-reason",
        "approval-reason",
        "forwarding-reason",
        "scope-reason",
        "test-command-label",
        "test-run-command",
        "stdout-artifact-label",
        "stderr-artifact-label",
        "preflight-affected-path",
        "preflight-operation-path",
        "execution-affected-path",
        "execution-conflict-path",
        "restore-target-path",
        "restore-reason",
        "diagnostic",
        "test-artifact-body-text",
        "artifact-body-storage",
        "artifact-body-summary",
        "executor-block-reason",
        "workflow-id",
        "task-id",
        "gate-accepted",
        "mutation-request-id",
        "patch-id",
        "policy-id",
        "sandbox-label",
        "isolated-root",
        "executor-label",
    ] {
        assert!(
            !output.contains(leaked),
            "workflow inspect output leaked {leaked}: {output}"
        );
    }
}

fn trace_task_id() -> TaskId {
    TaskId::from_static("task_cli_trace_apply_patch")
}

fn trace_request_id() -> MutationRequestId {
    MutationRequestId::from_static("mutation_request_cli_trace_apply_patch")
}

fn trace_patch_id() -> PatchProposalId {
    PatchProposalId::from_static("patch_proposal_cli_trace_apply_patch")
}

fn trace_checkpoint_id() -> SnapshotId {
    SnapshotId::from_static("snapshot_cli_trace_apply_patch")
}

fn trace_reviewer_gate_id() -> ReviewerGateId {
    ReviewerGateId::from_static("reviewer_gate_cli_trace_apply_patch")
}

fn trace_policy_decision_id() -> PolicyDecisionId {
    PolicyDecisionId::from_static("policy_cli_trace_apply_patch")
}

fn trace_patch_artifact_id() -> ArtifactId {
    ArtifactId::from_static("artifact_cli_trace_apply_patch")
}

fn trace_worktree_id() -> WorkspaceWorktreeId {
    WorkspaceWorktreeId::from_static("workspace_worktree_cli_trace_apply_patch")
}

fn trace_patch_evidence() -> HandoffEvidenceRef {
    HandoffEvidenceRef {
        kind: HandoffEvidenceKind::DiffArtifact,
        artifact_id: Some(trace_patch_artifact_id()),
        trace_id: Some("trace_cli_trace_apply_patch".to_string()),
        event_range: Some(EventRange {
            start_seq: 4,
            end_seq: 5,
        }),
        label: Some("reviewed patch artifact".to_string()),
        summary: Some("patch body stored out of trace".to_string()),
    }
}

fn trace_worktree_lifecycle_record(
    status: WorkspaceWorktreeLifecycleStatus,
    reason: &str,
) -> WorkspaceWorktreeLifecycleRecord {
    WorkspaceWorktreeLifecycleRecord {
        worktree_id: trace_worktree_id(),
        workflow_id: trace_workflow_id(),
        task_id: trace_task_id(),
        trace_id: "trace_cli_trace_apply_patch".to_string(),
        source_commit: "17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1".to_string(),
        source_branch_label: Some("main".to_string()),
        worktree_root_label: "worktree:cli-trace-apply-patch".to_string(),
        worktree_base_key: "data_dir:worktrees/cli-source".to_string(),
        lifecycle_status: status,
        reason: reason.to_string(),
        created_for_request_id: Some(trace_request_id()),
        created_for_patch_id: Some(trace_patch_id()),
        evidence: vec![HandoffEvidenceRef {
            kind: HandoffEvidenceKind::TraceRange,
            artifact_id: None,
            trace_id: Some("trace_cli_trace_apply_patch".to_string()),
            event_range: Some(EventRange {
                start_seq: 1,
                end_seq: 3,
            }),
            label: Some("trace worktree lifecycle evidence".to_string()),
            summary: Some("metadata-only generated worktree refs".to_string()),
        }],
        metadata: None,
    }
}

fn trace_worktree_lifecycle_record_without_refs(
    status: WorkspaceWorktreeLifecycleStatus,
    reason: &str,
) -> WorkspaceWorktreeLifecycleRecord {
    let mut record = trace_worktree_lifecycle_record(status, reason);
    record.created_for_request_id = None;
    record.created_for_patch_id = None;
    record
}

fn write_trace_worktree_lifecycle_bundle(
    data_dir: &Path,
    statuses: &[WorkspaceWorktreeLifecycleStatus],
) {
    let mut store = TraceStore::open(data_dir).unwrap();
    let trace_id = "trace_cli_trace_apply_patch";
    for (offset, status) in statuses.iter().enumerate() {
        store
            .append(&EventFrame::new(
                trace_id,
                100 + offset as u64,
                RunEvent::WorkspaceWorktreeLifecycleRecorded {
                    record: trace_worktree_lifecycle_record(
                        *status,
                        worktree_lifecycle_test_reason(*status),
                    ),
                },
            ))
            .unwrap();
    }
}

fn write_trace_worktree_lifecycle_records(
    data_dir: &Path,
    records: Vec<WorkspaceWorktreeLifecycleRecord>,
) {
    let mut store = TraceStore::open(data_dir).unwrap();
    let trace_id = "trace_cli_trace_apply_patch";
    for (offset, record) in records.into_iter().enumerate() {
        store
            .append(&EventFrame::new(
                trace_id,
                100 + offset as u64,
                RunEvent::WorkspaceWorktreeLifecycleRecorded { record },
            ))
            .unwrap();
    }
}

fn worktree_lifecycle_test_reason(status: WorkspaceWorktreeLifecycleStatus) -> &'static str {
    match status {
        WorkspaceWorktreeLifecycleStatus::Planned => "isolated worktree planned",
        WorkspaceWorktreeLifecycleStatus::Created => "isolated worktree created",
        WorkspaceWorktreeLifecycleStatus::CreationFailed => "isolated worktree creation failed",
        WorkspaceWorktreeLifecycleStatus::Retained => "isolated worktree retained for review",
        WorkspaceWorktreeLifecycleStatus::CleanupStarted => "trace-confirmed cleanup started",
        WorkspaceWorktreeLifecycleStatus::CleanupCompleted => "trace-confirmed cleanup completed",
        WorkspaceWorktreeLifecycleStatus::CleanupFailed => "trace-confirmed cleanup failed",
    }
}

fn write_trace_apply_patch_bundle(
    data_dir: &Path,
    include_artifact_body: bool,
    redaction_status: ArtifactBodyRedactionStatus,
) {
    write_trace_apply_patch_bundle_with_body(
        data_dir,
        include_artifact_body,
        redaction_status,
        apply_patch_modify_body(),
    );
}

fn write_trace_apply_patch_bundle_with_body(
    data_dir: &Path,
    include_artifact_body: bool,
    redaction_status: ArtifactBodyRedactionStatus,
    patch_body: String,
) {
    let mut store = TraceStore::open(data_dir).unwrap();
    let trace_id = "trace_cli_trace_apply_patch";
    let artifact_record = if include_artifact_body {
        Some(
            store
                .write_artifact_body(ArtifactBodyWrite {
                    artifact_id: trace_patch_artifact_id(),
                    kind: ArtifactKind::Patch,
                    task_id: Some(trace_task_id()),
                    media_type: "text/x-diff".to_string(),
                    redaction_status,
                    summary: Some("clean reviewed patch body".to_string()),
                    body: patch_body.into_bytes(),
                })
                .unwrap(),
        )
    } else {
        None
    };

    let mut seq = 1;
    let mut append = |event: RunEvent| {
        store
            .append(&EventFrame::new(trace_id, seq, event))
            .unwrap();
        seq += 1;
    };
    append(RunEvent::CodingWorkflowStarted {
        workflow_id: trace_workflow_id(),
        task_id: trace_task_id(),
        objective: "trace-backed CLI apply-patch".to_string(),
    });
    append(RunEvent::WorkspaceMutationScopeRecorded {
        scope: WorkspaceMutationScope {
            workflow_id: trace_workflow_id(),
            task_id: trace_task_id(),
            root_label: "project".to_string(),
            allowed_paths: vec!["docs/README.md".to_string()],
            denied_paths: Vec::new(),
            mutation_mode: MutationMode::WorktreeFirst,
            worktree_required: true,
            reason: Some("reviewed trace mutation scope".to_string()),
        },
    });
    append(RunEvent::MutationRequestProposalRecorded {
        proposal: MutationRequestProposal {
            request_id: trace_request_id(),
            workflow_id: trace_workflow_id(),
            task_id: trace_task_id(),
            operation: MutationRequestOperationKind::PatchApplication,
            status: MutationRequestStatus::Approved,
            summary: "apply reviewed trace patch".to_string(),
            requested_paths: vec!["docs/README.md".to_string()],
            required_checkpoint_id: Some(trace_checkpoint_id()),
            reviewer_gate_id: Some(trace_reviewer_gate_id()),
            policy_decision_id: Some(trace_policy_decision_id()),
            sandbox_profile_label: Some("workspace_write_isolated".to_string()),
            worktree_required: true,
            evidence: vec![trace_patch_evidence()],
        },
    });
    append(RunEvent::PatchProposalRecorded {
        proposal: PatchProposal {
            patch_id: trace_patch_id(),
            workflow_id: trace_workflow_id(),
            task_id: trace_task_id(),
            summary: "update docs from trace artifact".to_string(),
            touched_paths: vec!["docs/README.md".to_string()],
            diff_artifacts: vec![trace_patch_evidence()],
            risk_labels: vec!["docs_only".to_string()],
            required_checkpoint_id: Some(trace_checkpoint_id()),
            reviewer_gate_id: Some(trace_reviewer_gate_id()),
        },
    });
    if let Some(record) = artifact_record {
        append(RunEvent::ArtifactBodyRecorded { record });
    }
    append(RunEvent::SnapshotLifecycleRecorded {
        lifecycle: WorkspaceCheckpointLifecycleRecord {
            checkpoint_id: trace_checkpoint_id(),
            task_id: trace_task_id(),
            status: WorkspaceCheckpointLifecycleStatus::Created,
            reason: "checkpoint created before trace apply".to_string(),
            restore_plan_id: None,
            execution_blocked: true,
            evidence: Vec::new(),
            metadata: None,
        },
    });
    append(RunEvent::ReviewerGateResolved {
        decision: ReviewerGateDecision {
            gate_id: trace_reviewer_gate_id(),
            handoff_id: AgentHandoffId::from_static("handoff_cli_trace_apply_patch"),
            decision: ReviewerDecisionKind::Accept,
            reviewer: "human-reviewer".to_string(),
            reason_code: "accepted_for_trace_apply".to_string(),
            comment: Some("reviewed for trace apply".to_string()),
        },
    });
    append(RunEvent::ToolPolicyDecisionRecorded {
        decision: ToolPolicyDecision {
            decision_id: trace_policy_decision_id(),
            call_id: ToolCallId::from_static("tool_call_cli_trace_apply_patch"),
            tool_id: ToolId::from_static("tool_cli_trace_apply_patch"),
            outcome: PolicyOutcome::Allow,
            reason: "policy allows trace apply".to_string(),
            required_permissions: vec![ToolPermission::FilesystemWrite],
            side_effects: vec![ToolSideEffect::WritesWorkspace],
            approval_id: None,
        },
    });
}

fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_apply_patch_source_repo(root: &Path) {
    std::fs::create_dir_all(root.join("docs")).unwrap();
    std::fs::write(root.join("docs/README.md"), "alpha\nold\nomega\n").unwrap();
    git(root, &["init"]);
    git(root, &["config", "user.email", "tessera@example.invalid"]);
    git(root, &["config", "user.name", "Tessera CLI Test"]);
    git(root, &["add", "docs/README.md"]);
    git(root, &["commit", "-m", "initial"]);
}

fn worktree_base_entries(worktree_base: &Path) -> Vec<std::path::PathBuf> {
    if !worktree_base.exists() {
        return Vec::new();
    }
    std::fs::read_dir(worktree_base)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect()
}

#[test]
fn version_output_reports_crate_version_and_git_sha() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let git_sha = option_env!("TESSERA_GIT_SHA").unwrap_or("unknown");

    assert_ne!(git_sha, "unknown");
    assert_eq!(git_sha.len(), 40);
    assert!(git_sha
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    assert!(stdout.contains(git_sha));
}

#[test]
fn chat_help_lists_resume_option() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--resume <RESUME>"));
    assert!(stdout.contains("--resume-task <RESUME_TASK>"));
    assert!(stdout.contains("--stdin"));
    assert!(stdout.contains("--file <FILE>"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--continue"));
    assert!(stdout.contains("--list-commands"));
}

#[test]
fn apply_patch_command_help_lists_explicit_isolated_root_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["apply-patch", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--isolated-root"));
    assert!(stdout.contains("--root-label"));
    assert!(stdout.contains("--allowed-path"));
    assert!(stdout.contains("--patch-file"));
    assert!(stdout.contains("--stdin"));
    assert!(stdout.contains("--dry-run"));
}

#[test]
fn apply_patch_command_help_lists_trace_driven_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["apply-patch", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--from-trace"));
    assert!(stdout.contains("--patch-artifact-id"));
    assert!(stdout.contains("--workflow-id"));
    assert!(stdout.contains("--request-id"));
    assert!(stdout.contains("--patch-id"));
}

#[test]
fn apply_patch_command_help_lists_auto_worktree_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["apply-patch", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--auto-worktree"));
    assert!(stdout.contains("--worktree-base"));
    assert!(stdout.contains("--worktree-retention"));
    assert!(stdout.contains("--source-root"));
}

#[test]
fn apply_patch_command_auto_worktree_requires_from_trace() {
    let temp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["apply-patch", "--data-dir"])
        .arg(temp.path().join("data"))
        .args(["--auto-worktree", "--patch-file"])
        .arg(temp.path().join("missing.diff"))
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--auto-worktree requires --from-trace"));
}

#[test]
fn apply_patch_command_auto_worktree_rejects_manual_isolated_root_options() {
    let temp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["apply-patch", "--data-dir"])
        .arg(temp.path().join("data"))
        .args([
            "--from-trace",
            "trace_cli_trace_apply_patch",
            "--auto-worktree",
            "--isolated-root",
        ])
        .arg(temp.path().join("manual"))
        .args(["--root-label", "worktree:manual"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--auto-worktree cannot be combined with --isolated-root"));
    assert!(stderr.contains("--root-label"));
}

#[test]
fn apply_patch_command_applies_single_file_inside_isolated_root_and_records_trace() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    let patch_file = temp.path().join("patch.diff");
    std::fs::write(&patch_file, apply_patch_modify_body()).unwrap();

    let mut args = apply_patch_args(&data_dir, &isolated_root, &patch_file);
    args.push("--json".to_string());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nnew\nomega\n"
    );
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["trace_id"], "trace_cli_apply_patch");
    assert_eq!(stdout["preflight_status"], "executor_ready");
    assert_eq!(stdout["execution_status"], "applied");
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_apply_patch.jsonl")).unwrap();
    assert!(trace.contains("apply_patch_preflight_recorded"));
    assert!(trace.contains("apply_patch_execution_recorded"));
    assert!(!trace.contains("alpha\\nnew\\nomega"));
    assert!(!trace.contains(&isolated_root.display().to_string()));
}

#[test]
fn apply_patch_command_dry_run_records_preflight_without_writing_file() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    let patch_file = temp.path().join("patch.diff");
    std::fs::write(&patch_file, apply_patch_modify_body()).unwrap();

    let mut args = apply_patch_args(&data_dir, &isolated_root, &patch_file);
    args.push("--dry-run".to_string());
    args.push("--json".to_string());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["preflight_status"], "dry_run_ready");
    assert_eq!(stdout["executor_blocked"], true);
    assert!(stdout.get("execution_status").is_none());
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_apply_patch.jsonl")).unwrap();
    assert!(trace.contains("apply_patch_preflight_recorded"));
    assert!(!trace.contains("apply_patch_execution_recorded"));
}

#[test]
fn apply_patch_command_rejects_primary_root_label_without_writing_file() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    let patch_file = temp.path().join("patch.diff");
    std::fs::write(&patch_file, apply_patch_modify_body()).unwrap();

    let mut args = apply_patch_args(&data_dir, &isolated_root, &patch_file);
    let root_label_index = args.iter().position(|arg| arg == "--root-label").unwrap() + 1;
    args[root_label_index] = "project".to_string();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("primary_root_rejected") || stderr.contains("PrimaryRootRejected"));
}

#[test]
fn apply_patch_command_from_trace_applies_patch_artifact_and_records_trace() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_apply_patch_args(&data_dir, &isolated_root))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nnew\nomega\n"
    );
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["trace_id"], "trace_cli_trace_apply_patch");
    assert_eq!(stdout["preflight_status"], "executor_ready");
    assert_eq!(stdout["execution_status"], "applied");
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(trace.contains("apply_patch_preflight_recorded"));
    assert!(trace.contains("apply_patch_execution_recorded"));
    assert!(!trace.contains("alpha\\nnew\\nomega"));
    assert!(!trace.contains(&isolated_root.display().to_string()));
}

#[test]
fn apply_patch_command_from_trace_dry_run_records_preflight_without_writing_file() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let mut args = trace_apply_patch_args(&data_dir, &isolated_root);
    args.push("--dry-run".to_string());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["preflight_status"], "dry_run_ready");
    assert_eq!(stdout["executor_blocked"], true);
    assert!(stdout.get("execution_status").is_none());
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(trace.contains("apply_patch_preflight_recorded"));
    assert!(!trace.contains("apply_patch_execution_recorded"));
}

#[test]
fn apply_patch_command_from_trace_missing_artifact_metadata_fails_before_preflight() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    write_trace_apply_patch_bundle(&data_dir, false, ArtifactBodyRedactionStatus::Clean);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_apply_patch_args(&data_dir, &isolated_root))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing patch artifact metadata"));
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("apply_patch_preflight_recorded"));
}

#[test]
fn apply_patch_command_from_trace_redacted_artifact_fails_before_preflight() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Redacted);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_apply_patch_args(&data_dir, &isolated_root))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("patch artifact is not clean"));
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("apply_patch_preflight_recorded"));
}

#[test]
fn apply_patch_command_from_trace_oversized_artifact_fails_before_preflight() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    write_trace_apply_patch_bundle_with_body(
        &data_dir,
        true,
        ArtifactBodyRedactionStatus::Clean,
        "x".repeat(1024 * 1024 + 1),
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_apply_patch_args(&data_dir, &isolated_root))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("patch artifact body is too large"));
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("apply_patch_preflight_recorded"));
}

#[test]
fn apply_patch_command_from_trace_rejects_patch_artifact_id_with_patch_file() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    let patch_file = temp.path().join("patch.diff");
    std::fs::write(&patch_file, apply_patch_modify_body()).unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let mut args = trace_apply_patch_args(&data_dir, &isolated_root);
    args.push("--patch-artifact-id".to_string());
    args.push(trace_patch_artifact_id().to_string());
    args.push("--patch-file".to_string());
    args.push(patch_file.display().to_string());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("at most one of --patch-artifact-id, --patch-file, or --stdin"));
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("apply_patch_preflight_recorded"));
}

#[test]
fn apply_patch_command_from_trace_rejects_primary_root_label_before_writing_file() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let isolated_root = temp.path().join("isolated");
    let docs_dir = isolated_root.join("docs");
    std::fs::create_dir_all(&docs_dir).unwrap();
    std::fs::write(docs_dir.join("README.md"), "alpha\nold\nomega\n").unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let mut args = trace_apply_patch_args(&data_dir, &isolated_root);
    let root_label_index = args.iter().position(|arg| arg == "--root-label").unwrap() + 1;
    args[root_label_index] = "project".to_string();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(docs_dir.join("README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("primary_root_rejected") || stderr.contains("PrimaryRootRejected"));
}

#[test]
fn apply_patch_command_auto_worktree_dry_run_does_not_create_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_base = temp.path().join("worktrees");
    init_apply_patch_source_repo(&source_root);
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let mut args = trace_auto_worktree_apply_patch_args(&data_dir, &source_root, &worktree_base);
    args.push("--dry-run".to_string());
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(source_root.join("docs/README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    assert!(worktree_base_entries(&worktree_base).is_empty());
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["preflight_status"], "dry_run_ready");
    assert!(stdout.get("worktree_path").is_none());
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(trace.contains("apply_patch_preflight_recorded"));
    assert!(!trace.contains("workspace_worktree_lifecycle_recorded"));
    assert!(!trace.contains("apply_patch_execution_recorded"));
}

#[test]
fn apply_patch_command_auto_worktree_creates_detached_worktree_and_records_redacted_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_base = temp.path().join("worktrees");
    init_apply_patch_source_repo(&source_root);
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_auto_worktree_apply_patch_args(
            &data_dir,
            &source_root,
            &worktree_base,
        ))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(source_root.join("docs/README.md")).unwrap(),
        "alpha\nold\nomega\n"
    );
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(stdout["trace_id"], "trace_cli_trace_apply_patch");
    assert_eq!(stdout["preflight_status"], "executor_ready");
    assert_eq!(stdout["execution_status"], "applied");
    assert_eq!(stdout["worktree_lifecycle_status"], "retained");
    let worktree_path = std::path::PathBuf::from(stdout["worktree_path"].as_str().unwrap());
    assert!(worktree_path.exists());
    assert_eq!(
        std::fs::read_to_string(worktree_path.join("docs/README.md")).unwrap(),
        "alpha\nnew\nomega\n"
    );

    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    let planned = trace.find("\"lifecycle_status\":\"planned\"").unwrap();
    let created = trace.find("\"lifecycle_status\":\"created\"").unwrap();
    let preflight = trace.find("apply_patch_preflight_recorded").unwrap();
    let execution = trace.find("apply_patch_execution_recorded").unwrap();
    let retained = trace.find("\"lifecycle_status\":\"retained\"").unwrap();
    assert!(planned < created);
    assert!(created < preflight);
    assert!(preflight < execution);
    assert!(execution < retained);
    assert!(!trace.contains(&source_root.display().to_string()));
    assert!(!trace.contains(&worktree_base.display().to_string()));
    assert!(!trace.contains(&worktree_path.display().to_string()));
}

#[test]
fn apply_patch_command_auto_worktree_rejects_dirty_source_before_preflight() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_base = temp.path().join("worktrees");
    init_apply_patch_source_repo(&source_root);
    std::fs::write(source_root.join("docs/README.md"), "alpha\ndirty\nomega\n").unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_auto_worktree_apply_patch_args(
            &data_dir,
            &source_root,
            &worktree_base,
        ))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(worktree_base_entries(&worktree_base).is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("source checkout has tracked changes"));
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("workspace_worktree_lifecycle_recorded"));
    assert!(!trace.contains("apply_patch_preflight_recorded"));
    assert!(!trace.contains("apply_patch_execution_recorded"));
}

#[test]
fn worktree_cleanup_command_help_lists_trace_and_path_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["worktree", "cleanup", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--from-trace"));
    assert!(stdout.contains("--worktree-id"));
    assert!(stdout.contains("--worktree-path"));
    assert!(stdout.contains("--source-root"));
    assert!(stdout.contains("--dry-run"));
}

#[test]
fn worktree_list_command_reports_trace_lifecycle_without_local_paths() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let generated_worktree_path = temp
        .path()
        .join("worktrees")
        .join("cli-trace-apply-patch-generated");
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_bundle(
        &data_dir,
        &[
            WorkspaceWorktreeLifecycleStatus::Planned,
            WorkspaceWorktreeLifecycleStatus::Created,
            WorkspaceWorktreeLifecycleStatus::Retained,
        ],
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_list_args(&data_dir, false))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("workspace_worktree_cli_trace_apply_patch"));
    assert!(stdout.contains("retained"));
    assert!(stdout.contains("worktree:cli-trace-apply-patch"));
    assert!(stdout.contains("data_dir:worktrees/cli-source"));
    assert!(stdout.contains("17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1"));
    assert!(stdout.contains("trace_cleanup_candidate"));
    assert!(!stdout.contains(&generated_worktree_path.display().to_string()));
    assert!(!stdout.contains(&temp.path().display().to_string()));
}

#[test]
fn worktree_list_command_emits_json_for_trace_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_bundle(
        &data_dir,
        &[
            WorkspaceWorktreeLifecycleStatus::Planned,
            WorkspaceWorktreeLifecycleStatus::Created,
            WorkspaceWorktreeLifecycleStatus::Retained,
        ],
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_list_args(&data_dir, true))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let lifecycles = stdout.as_array().unwrap();
    assert_eq!(lifecycles.len(), 1);
    let lifecycle = &lifecycles[0];
    assert_eq!(
        lifecycle["worktree_id"],
        "workspace_worktree_cli_trace_apply_patch"
    );
    assert_eq!(lifecycle["latest_status"], "retained");
    assert_eq!(lifecycle["trace_cleanup_candidate"], true);
    assert_eq!(
        lifecycle["trace_cleanup_candidate_note"],
        "trace evidence only; cleanup still requires explicit --worktree-path validation"
    );
    assert!(lifecycle.get("worktree_path").is_none());
}

#[test]
fn worktree_list_command_omits_free_form_lifecycle_reasons_from_text_and_json() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let sensitive_path = "/Users/admin/work/tessera/.env";
    let sensitive_token = "sk-secret-list-reason";
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_records(
        &data_dir,
        vec![
            trace_worktree_lifecycle_record(
                WorkspaceWorktreeLifecycleStatus::Planned,
                "planning generated worktree",
            ),
            trace_worktree_lifecycle_record(
                WorkspaceWorktreeLifecycleStatus::Created,
                "created generated worktree",
            ),
            trace_worktree_lifecycle_record(
                WorkspaceWorktreeLifecycleStatus::Retained,
                &format!("retained at {sensitive_path} with token {sensitive_token}"),
            ),
        ],
    );

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_list_args(&data_dir, false))
        .output()
        .unwrap();

    assert!(
        text_output.status.success(),
        "{}",
        String::from_utf8_lossy(&text_output.stderr)
    );
    let text_stdout = String::from_utf8(text_output.stdout).unwrap();
    assert!(!text_stdout.contains(sensitive_path));
    assert!(!text_stdout.contains(sensitive_token));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_list_args(&data_dir, true))
        .output()
        .unwrap();

    assert!(
        json_output.status.success(),
        "{}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let json_stdout = String::from_utf8(json_output.stdout).unwrap();
    assert!(!json_stdout.contains("latest_reason"));
    assert!(!json_stdout.contains(sensitive_path));
    assert!(!json_stdout.contains(sensitive_token));
    let lifecycles: serde_json::Value = serde_json::from_str(&json_stdout).unwrap();
    assert!(lifecycles.as_array().unwrap()[0]
        .get("latest_reason")
        .is_none());
}

#[test]
fn worktree_list_command_requires_created_record_refs_for_cleanup_candidate() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_records(
        &data_dir,
        vec![
            trace_worktree_lifecycle_record(
                WorkspaceWorktreeLifecycleStatus::Planned,
                "isolated worktree planned",
            ),
            trace_worktree_lifecycle_record_without_refs(
                WorkspaceWorktreeLifecycleStatus::Created,
                "isolated worktree created without request refs",
            ),
            trace_worktree_lifecycle_record(
                WorkspaceWorktreeLifecycleStatus::Retained,
                "isolated worktree retained with latest refs only",
            ),
        ],
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_list_args(&data_dir, true))
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let lifecycle = &stdout.as_array().unwrap()[0];
    assert_eq!(lifecycle["latest_status"], "retained");
    assert_eq!(lifecycle["trace_cleanup_candidate"], false);
    assert!(lifecycle.get("worktree_path").is_none());
}

#[test]
fn worktree_cleanup_command_requires_created_record_refs_before_dry_run() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_leaf = format!(
        "cli-trace-apply-patch-{}",
        "17bd0f1c0ffee17bd0f1c0ffee17bd0f1c0ffee17bd0f1"
            .chars()
            .take(12)
            .collect::<String>()
    );
    let worktree_path = temp.path().join("worktrees").join(worktree_leaf);
    init_apply_patch_source_repo(&source_root);
    std::fs::create_dir_all(&worktree_path).unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_records(
        &data_dir,
        vec![
            trace_worktree_lifecycle_record(
                WorkspaceWorktreeLifecycleStatus::Planned,
                "isolated worktree planned",
            ),
            trace_worktree_lifecycle_record_without_refs(
                WorkspaceWorktreeLifecycleStatus::Created,
                "isolated worktree created without request refs",
            ),
            trace_worktree_lifecycle_record(
                WorkspaceWorktreeLifecycleStatus::Retained,
                "isolated worktree retained with latest refs only",
            ),
        ],
    );

    let mut args = worktree_cleanup_args(
        &data_dir,
        &source_root,
        trace_worktree_id().as_str(),
        &worktree_path,
    );
    args.push("--dry-run".to_string());
    let cleanup_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(!cleanup_output.status.success());
    assert!(worktree_path.exists());
    let stderr = String::from_utf8_lossy(&cleanup_output.stderr);
    assert!(stderr.contains("created worktree lifecycle record is missing request or patch refs"));
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("\"lifecycle_status\":\"cleanup_started\""));
}

#[test]
fn worktree_cleanup_command_dry_run_validates_without_removing_or_appending_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_base = temp.path().join("worktrees");
    init_apply_patch_source_repo(&source_root);
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let create_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_auto_worktree_apply_patch_args(
            &data_dir,
            &source_root,
            &worktree_base,
        ))
        .output()
        .unwrap();
    assert!(
        create_output.status.success(),
        "{}",
        String::from_utf8_lossy(&create_output.stderr)
    );
    let create_stdout: serde_json::Value = serde_json::from_slice(&create_output.stdout).unwrap();
    let worktree_id = create_stdout["worktree_id"].as_str().unwrap();
    let worktree_path = std::path::PathBuf::from(create_stdout["worktree_path"].as_str().unwrap());

    let mut args = worktree_cleanup_args(&data_dir, &source_root, worktree_id, &worktree_path);
    args.push("--dry-run".to_string());
    let cleanup_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(args)
        .output()
        .unwrap();

    assert!(
        cleanup_output.status.success(),
        "{}",
        String::from_utf8_lossy(&cleanup_output.stderr)
    );
    assert!(worktree_path.exists());
    let stdout: serde_json::Value = serde_json::from_slice(&cleanup_output.stdout).unwrap();
    assert_eq!(stdout["trace_id"], "trace_cli_trace_apply_patch");
    assert_eq!(stdout["worktree_id"], worktree_id);
    assert_eq!(stdout["worktree_lifecycle_status"], "dry_run_ready");
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("\"lifecycle_status\":\"cleanup_started\""));
    assert!(!trace.contains("\"lifecycle_status\":\"cleanup_completed\""));
}

#[test]
fn worktree_cleanup_command_removes_retained_worktree_and_records_redacted_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_base = temp.path().join("worktrees");
    init_apply_patch_source_repo(&source_root);
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let create_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_auto_worktree_apply_patch_args(
            &data_dir,
            &source_root,
            &worktree_base,
        ))
        .output()
        .unwrap();
    assert!(
        create_output.status.success(),
        "{}",
        String::from_utf8_lossy(&create_output.stderr)
    );
    let create_stdout: serde_json::Value = serde_json::from_slice(&create_output.stdout).unwrap();
    let worktree_id = create_stdout["worktree_id"].as_str().unwrap();
    let worktree_path = std::path::PathBuf::from(create_stdout["worktree_path"].as_str().unwrap());
    git(&worktree_path, &["restore", "docs/README.md"]);

    let cleanup_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_cleanup_args(
            &data_dir,
            &source_root,
            worktree_id,
            &worktree_path,
        ))
        .output()
        .unwrap();

    assert!(
        cleanup_output.status.success(),
        "{}",
        String::from_utf8_lossy(&cleanup_output.stderr)
    );
    assert!(!worktree_path.exists());
    let stdout: serde_json::Value = serde_json::from_slice(&cleanup_output.stdout).unwrap();
    assert_eq!(stdout["worktree_lifecycle_status"], "cleanup_completed");
    assert_eq!(stdout["worktree_path"], worktree_path.display().to_string());
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    let retained = trace.find("\"lifecycle_status\":\"retained\"").unwrap();
    let cleanup_started = trace
        .find("\"lifecycle_status\":\"cleanup_started\"")
        .unwrap();
    let cleanup_completed = trace
        .find("\"lifecycle_status\":\"cleanup_completed\"")
        .unwrap();
    assert!(retained < cleanup_started);
    assert!(cleanup_started < cleanup_completed);
    assert!(!trace.contains(&source_root.display().to_string()));
    assert!(!trace.contains(&worktree_base.display().to_string()));
    assert!(!trace.contains(&worktree_path.display().to_string()));
    assert!(!trace.contains("git worktree remove"));
}

#[test]
fn worktree_cleanup_command_dirty_worktree_failure_records_redacted_lifecycle() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_base = temp.path().join("worktrees");
    init_apply_patch_source_repo(&source_root);
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let create_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_auto_worktree_apply_patch_args(
            &data_dir,
            &source_root,
            &worktree_base,
        ))
        .output()
        .unwrap();
    assert!(
        create_output.status.success(),
        "{}",
        String::from_utf8_lossy(&create_output.stderr)
    );
    let create_stdout: serde_json::Value = serde_json::from_slice(&create_output.stdout).unwrap();
    let worktree_id = create_stdout["worktree_id"].as_str().unwrap();
    let worktree_path = std::path::PathBuf::from(create_stdout["worktree_path"].as_str().unwrap());

    let cleanup_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_cleanup_args(
            &data_dir,
            &source_root,
            worktree_id,
            &worktree_path,
        ))
        .output()
        .unwrap();

    assert!(!cleanup_output.status.success());
    assert!(worktree_path.exists());
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(trace.contains("\"lifecycle_status\":\"cleanup_started\""));
    assert!(trace.contains("\"lifecycle_status\":\"cleanup_failed\""));
    assert!(!trace.contains(&source_root.display().to_string()));
    assert!(!trace.contains(&worktree_base.display().to_string()));
    assert!(!trace.contains(&worktree_path.display().to_string()));
    assert!(!trace.contains("git worktree remove"));
}

#[test]
fn worktree_cleanup_command_rejects_missing_lifecycle_evidence_before_git() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let arbitrary_path = temp.path().join("not-a-tessera-worktree");
    init_apply_patch_source_repo(&source_root);
    std::fs::create_dir_all(&arbitrary_path).unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_cleanup_args(
            &data_dir,
            &source_root,
            "workspace_worktree_missing",
            &arbitrary_path,
        ))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(arbitrary_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("no matching worktree lifecycle records found"));
    let trace =
        std::fs::read_to_string(data_dir.join("traces/trace_cli_trace_apply_patch.jsonl")).unwrap();
    assert!(!trace.contains("\"lifecycle_status\":\"cleanup_started\""));
}

#[test]
fn worktree_cleanup_rejects_already_cleaned_lifecycle_with_specific_message() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let arbitrary_path = temp.path().join("cli-trace-apply-patch");
    init_apply_patch_source_repo(&source_root);
    std::fs::create_dir_all(&arbitrary_path).unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_bundle(
        &data_dir,
        &[
            WorkspaceWorktreeLifecycleStatus::Planned,
            WorkspaceWorktreeLifecycleStatus::Created,
            WorkspaceWorktreeLifecycleStatus::Retained,
            WorkspaceWorktreeLifecycleStatus::CleanupCompleted,
        ],
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_cleanup_args(
            &data_dir,
            &source_root,
            trace_worktree_id().as_str(),
            &arbitrary_path,
        ))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(arbitrary_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("already cleaned"));
}

#[test]
fn worktree_cleanup_rejects_never_retained_lifecycle_with_specific_message() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let arbitrary_path = temp.path().join("cli-trace-apply-patch");
    init_apply_patch_source_repo(&source_root);
    std::fs::create_dir_all(&arbitrary_path).unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_bundle(
        &data_dir,
        &[
            WorkspaceWorktreeLifecycleStatus::Planned,
            WorkspaceWorktreeLifecycleStatus::Created,
        ],
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_cleanup_args(
            &data_dir,
            &source_root,
            trace_worktree_id().as_str(),
            &arbitrary_path,
        ))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(arbitrary_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not retained"));
}

#[test]
fn worktree_cleanup_rejects_cleanup_failed_without_retained_with_specific_message() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let arbitrary_path = temp.path().join("cli-trace-apply-patch");
    init_apply_patch_source_repo(&source_root);
    std::fs::create_dir_all(&arbitrary_path).unwrap();
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);
    write_trace_worktree_lifecycle_bundle(
        &data_dir,
        &[
            WorkspaceWorktreeLifecycleStatus::Created,
            WorkspaceWorktreeLifecycleStatus::CleanupFailed,
        ],
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_cleanup_args(
            &data_dir,
            &source_root,
            trace_worktree_id().as_str(),
            &arbitrary_path,
        ))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(arbitrary_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("latest cleanup failed"));
}

#[test]
fn workflow_inspect_command_reports_safe_text_without_raw_ids_or_paths() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    write_workflow_inspection_trace(&data_dir);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(workflow_inspect_args(&data_dir, false))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("workflow inspection records: 1"));
    assert!(stdout.contains("workflow:1"));
    assert!(stdout.contains("task:1"));
    assert!(stdout.contains("active=true"));
    assert!(stdout.contains("mutation=worktree_first"));
    assert!(stdout.contains("worktree_required=true"));
    assert!(stdout.contains("patches=1"));
    assert!(stdout.contains("diff_refs=1"));
    assert!(stdout.contains("review_bundles=4"));
    assert!(stdout.contains("review_evidence_refs=4"));
    assert!(stdout.contains("review_gates=4"));
    assert!(stdout.contains("accepted=1"));
    assert!(stdout.contains("rejected=1"));
    assert!(stdout.contains("revision_requested=1"));
    assert!(stdout.contains("pending=1"));
    assert!(stdout.contains("approvals=1"));
    assert!(stdout.contains("pending_approvals=1"));
    assert!(stdout.contains("resolved_approvals=0"));
    assert!(stdout.contains("preflights=1"));
    assert!(stdout.contains("executor_ready=0"));
    assert!(stdout.contains("executor_blocked=1"));
    assert!(stdout.contains("executions=1"));
    assert!(stdout.contains("successful=0"));
    assert!(stdout.contains("failed=1"));
    assert_workflow_inspect_output_is_safe(&stdout);
}

#[test]
fn workflow_inspect_command_emits_safe_json_dto_fields() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    write_workflow_inspection_trace(&data_dir);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(workflow_inspect_args(&data_dir, true))
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_workflow_inspect_output_is_safe(&stdout);
    let records: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(records.as_array().unwrap().len(), 1);
    let row = &records[0];
    assert_eq!(row["workflow_ref"], "workflow:1");
    assert_eq!(row["task_ref"], "task:1");
    assert_eq!(row["active"], true);
    assert_eq!(row["mutation_mode"], "worktree_first");
    assert_eq!(row["worktree_required"], true);
    assert_eq!(row["patch_count"], 1);
    assert_eq!(row["diff_artifact_ref_count"], 1);
    assert_eq!(row["patches_requiring_review_count"], 1);
    assert_eq!(row["review_bundle_count"], 4);
    assert_eq!(row["review_evidence_ref_count"], 4);
    assert_eq!(row["reviewer_gate_count"], 4);
    assert_eq!(row["accepted_reviewer_gate_count"], 1);
    assert_eq!(row["rejected_reviewer_gate_count"], 1);
    assert_eq!(row["revision_requested_reviewer_gate_count"], 1);
    assert_eq!(row["pending_reviewer_gate_count"], 1);
    assert_eq!(row["approval_count"], 1);
    assert_eq!(row["pending_approval_count"], 1);
    assert_eq!(row["resolved_approval_count"], 0);
    assert_eq!(row["apply_patch_preflight_count"], 1);
    assert_eq!(row["executor_ready_preflight_count"], 0);
    assert_eq!(row["executor_blocked_preflight_count"], 1);
    assert_eq!(row["apply_patch_execution_count"], 1);
    assert_eq!(row["successful_apply_patch_execution_count"], 0);
    assert_eq!(row["failed_apply_patch_execution_count"], 1);

    for forbidden_key in [
        "workflow_id",
        "task_id",
        "root_label",
        "allowed_paths",
        "denied_paths",
        "objective",
        "summary",
        "comment",
        "reason",
        "touched_paths",
        "requested_paths",
        "target_paths",
        "command_label",
        "stdout_artifact_id",
        "stderr_artifact_id",
        "evidence",
        "artifact_body",
    ] {
        assert!(
            row.get(forbidden_key).is_none(),
            "workflow inspect JSON exposed unsafe field {forbidden_key}: {stdout}"
        );
    }
}

#[test]
fn workflow_inspect_command_reports_empty_trace_as_none() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    write_empty_workflow_inspection_trace(&data_dir);

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(workflow_inspect_args(&data_dir, false))
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let stdout = String::from_utf8(text_output.stdout).unwrap();
    assert_eq!(stdout.trim(), "workflow inspection records: none");

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(workflow_inspect_args(&data_dir, true))
        .output()
        .unwrap();

    assert!(json_output.status.success());
    assert_eq!(String::from_utf8(json_output.stdout).unwrap().trim(), "[]");
}

#[test]
fn workflow_inspect_help_lists_trace_json_and_data_dir() {
    let top_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .arg("--help")
        .output()
        .unwrap();

    assert!(top_output.status.success());
    let top_stdout = String::from_utf8(top_output.stdout).unwrap();
    assert!(top_stdout.contains("workflow"));

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["workflow", "inspect", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("workflow inspect") || stdout.contains("Inspect"));
    assert!(stdout.contains("--trace"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--data-dir"));
}

#[test]
fn worktree_cleanup_command_rejects_path_mismatch_before_removing_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let source_root = temp.path().join("source");
    let worktree_base = temp.path().join("worktrees");
    init_apply_patch_source_repo(&source_root);
    write_trace_apply_patch_bundle(&data_dir, true, ArtifactBodyRedactionStatus::Clean);

    let create_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(trace_auto_worktree_apply_patch_args(
            &data_dir,
            &source_root,
            &worktree_base,
        ))
        .output()
        .unwrap();
    assert!(
        create_output.status.success(),
        "{}",
        String::from_utf8_lossy(&create_output.stderr)
    );
    let create_stdout: serde_json::Value = serde_json::from_slice(&create_output.stdout).unwrap();
    let worktree_id = create_stdout["worktree_id"].as_str().unwrap();
    let worktree_path = std::path::PathBuf::from(create_stdout["worktree_path"].as_str().unwrap());

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(worktree_cleanup_args(
            &data_dir,
            &source_root,
            worktree_id,
            &source_root,
        ))
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(worktree_path.exists());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("worktree path must not target source root"));
}

#[test]
fn chat_list_commands_prints_repl_commands_without_runtime_config() {
    let temp = tempfile::tempdir().unwrap();
    let missing_config = temp.path().join("missing.toml");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--list-commands", "--config"])
        .arg(&missing_config)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("commands:"));
    assert!(stdout.contains("/help"));
    assert!(stdout.contains("/cancel"));
    assert!(stdout.contains("/pause [task_id]"));
    assert!(stdout.contains("/resume-task <task_id|#>"));
    assert!(stdout.contains("/clear"));
    assert!(stdout.contains("/paste"));
    assert!(stdout.contains("/profiles"));
    assert!(stdout.contains("/history"));
    assert!(stdout.contains("/resume <trace_id|#>"));
    assert!(stdout.contains("/doctor"));
    assert!(stdout.contains("/quit"));
    assert!(!stdout.contains("Tessera CLI interactive chat"));
}

#[test]
fn bare_tessera_starts_interactive_chat_repl() {
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(b"/quit\n").unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Tessera CLI interactive chat"));
    assert!(stdout.contains("active_profile: mock"));
    assert!(stdout.contains("tessera(mock)> "));
}

#[test]
fn sessions_help_lists_json_and_data_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["sessions", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--data-dir"));
}

#[test]
fn tasks_help_lists_json_config_and_data_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["tasks", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--data-dir"));
}

#[test]
fn profiles_help_lists_json_and_config_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["profiles", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--config"));
}

#[test]
fn config_validate_help_lists_json_config_and_data_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--config"));
    assert!(stdout.contains("--data-dir"));
}

#[test]
fn transcript_help_lists_trace_id_and_json_option() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["transcript", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<TRACE_ID>"));
    assert!(stdout.contains("--json"));
}

#[test]
fn replay_help_lists_trace_id_and_json_option() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["replay", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<TRACE_ID>"));
    assert!(stdout.contains("--json"));
}

#[test]
fn events_help_lists_pagination_and_json_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["events", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<TRACE_ID>"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--since <SINCE>"));
    assert!(stdout.contains("--limit <LIMIT>"));
}

#[tokio::test]
async fn doctor_json_reports_trace_and_sqlite_health() {
    let temp = tempfile::tempdir().unwrap();
    let report: DoctorReport = run_doctor(temp.path()).unwrap();

    assert_eq!(report.status, "ok");
    assert!(report.trace_writable);
    assert!(report.sqlite_index_healthy);
    assert!(report
        .provider_profiles
        .iter()
        .any(|profile| profile == "mock"));
}

#[test]
fn doctor_text_reports_runtime_health_details() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "local"
kind = "ollama"
default_model = "llama3"
base_url = "http://localhost:11434"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["doctor", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("status: ok"));
    assert!(stdout.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(stdout.contains("trace_writable: true"));
    assert!(stdout.contains("sqlite_index_healthy: true"));
    assert!(stdout.contains("provider_profiles: offline, local"));
}

#[tokio::test]
async fn chat_command_path_runs_mock_provider() {
    let temp = tempfile::tempdir().unwrap();
    let output = run_chat_mock(temp.path(), "hello").await.unwrap();

    assert!(output.assistant_text.contains("mock response"));
    assert_eq!(output.trace_id, "trace_mock");
}

#[tokio::test]
async fn chat_command_path_routes_to_configured_mock_profile() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };

    let output = run_chat_with_config(temp.path(), &config, "offline", "hello")
        .await
        .unwrap();

    assert!(output.assistant_text.contains("mock response"));
    let events = output.store.list_events(&output.trace_id).unwrap();
    assert!(events.contains(&"route_decision_recorded".to_string()));
}

#[tokio::test]
async fn config_routed_chat_can_stream_live_event_frames_without_reading_trace_back() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let mut live_events = Vec::new();

    let output =
        run_chat_with_config_and_events(temp.path(), &config, "offline", "hello", |frame| {
            live_events.push(frame.clone());
        })
        .await
        .unwrap();

    assert!(live_events
        .iter()
        .any(|frame| matches!(frame.event, RunEvent::AssistantDelta { .. })));
    assert_eq!(live_events.last().unwrap().event.kind(), "done");
    assert_eq!(
        live_events
            .iter()
            .map(|frame| frame.event.kind().to_string())
            .collect::<Vec<_>>(),
        output.store.list_events(&output.trace_id).unwrap()
    );
}

#[tokio::test]
async fn config_routed_chat_records_cancellation_when_event_sink_stops() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };

    let output =
        run_chat_with_config_and_events(temp.path(), &config, "offline", "hello", |frame| {
            match frame.event {
                RunEvent::AssistantMessageStarted { .. } => {
                    EventSinkAction::Cancel("cli sink closed".to_string())
                }
                _ => EventSinkAction::Continue,
            }
        })
        .await
        .unwrap();

    let events = output.store.list_events(&output.trace_id).unwrap();
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
}

#[tokio::test]
async fn config_routed_chat_honors_pre_cancelled_run_controls() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let cancellation_token = RunCancellationToken::new();
    cancellation_token.cancel("cli cancel before provider request");

    let output = run_chat_with_config_and_controls_and_events(
        temp.path(),
        &config,
        "offline",
        "hello",
        RunControls {
            event_timeout: None,
            cancellation_token: Some(cancellation_token),
            pause_token: None,
        },
        |_| {},
    )
    .await
    .unwrap();

    let events = output.store.list_events(&output.trace_id).unwrap();
    assert!(events.contains(&"user_message_recorded".to_string()));
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"provider_request_started".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
}

#[tokio::test]
async fn config_routed_chat_uses_unique_trace_ids_across_runs() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };

    let first = run_chat_with_config(temp.path(), &config, "offline", "hello")
        .await
        .unwrap();
    let second = run_chat_with_config(temp.path(), &config, "offline", "hello again")
        .await
        .unwrap();

    assert_ne!(first.trace_id, second.trace_id);
    assert!(first.trace_id.starts_with("trace_offline_"));
    assert!(second.trace_id.starts_with("trace_offline_"));
}

#[tokio::test]
async fn chat_command_path_rejects_unknown_provider_profile() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig::default_with_mock();

    let error = match run_chat_with_config(temp.path(), &config, "missing", "hello").await {
        Ok(_) => panic!("expected missing provider profile to fail"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("provider profile not found"));
}

#[test]
fn agent_run_json_command_emits_trace_task_status_and_summary() {
    let temp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "mock",
            "--goal",
            "summarize",
            "--json",
            "--data-dir",
        ])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert!(payload["trace_id"]
        .as_str()
        .unwrap()
        .starts_with("trace_mock_"));
    assert!(payload["task_id"].as_str().unwrap().starts_with("task_"));
    assert_eq!(payload["status"], "completed");
    assert_eq!(payload["steps_completed"], 1);
    assert!(payload["assistant_text"]
        .as_str()
        .unwrap()
        .contains("mock response"));
    assert_eq!(payload["summary"]["status"], "completed");
    assert!(payload["summary"]["final_text"]
        .as_str()
        .unwrap()
        .contains("mock response"));
}

#[test]
fn agent_run_text_command_reports_task_trace_and_status() {
    let temp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "mock",
            "--goal",
            "summarize",
            "--data-dir",
        ])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("agent task "));
    assert!(stdout.contains("trace "));
    assert!(stdout.contains("status completed"));
    assert!(stdout.contains("mock response"));
}

#[test]
fn instructions_inspect_json_reports_sources_without_content() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join("AGENTS.md"),
        "Prefer concise answers.\napi_key = sk-test\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["instructions", "inspect", "--workspace"])
        .arg(workspace.path())
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(payload["source_count"], 1);
    assert_eq!(payload["loaded_count"], 1);
    assert_eq!(payload["sources"][0]["relative_path"], "AGENTS.md");
    assert_eq!(payload["sources"][0]["status"], "loaded");
    assert_eq!(payload["warning_count"], 1);
    assert!(!stdout.contains("Prefer concise answers"));
    assert!(!stdout.contains("sk-test"));
    assert!(!stdout.contains("api_key"));
}

#[test]
fn skills_inspect_json_reports_manifests_without_content() {
    let workspace = tempfile::tempdir().unwrap();
    let skill_dir = workspace.path().join(".tessera/skills/reviewer");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: reviewer\ndescription: Review safely\n---\n\nUse this skill.\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["skills", "inspect", "--workspace"])
        .arg(workspace.path())
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(payload["skill_count"], 1);
    assert_eq!(payload["source_count"], 1);
    assert_eq!(payload["skills"][0]["id"], "skill_reviewer");
    assert_eq!(payload["skills"][0]["name"], "reviewer");
    assert_eq!(payload["sources"][0]["status"], "loaded");
    assert!(!stdout.contains("Use this skill."));
}

#[test]
fn skills_inspect_text_summarizes_sources_without_content() {
    let workspace = tempfile::tempdir().unwrap();
    let skill_dir = workspace.path().join(".tessera/skills/reviewer");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: reviewer\ndescription: Review safely\n---\n\nUse this skill.\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["skills", "inspect", "--workspace"])
        .arg(workspace.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("skills: 1 loadable / 1 sources"));
    assert!(stdout.contains("- skill_reviewer reviewer .tessera/skills/reviewer/SKILL.md loaded"));
    assert!(!stdout.contains("Use this skill."));
}

#[test]
fn instructions_inspect_text_summarizes_sources_without_content() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("AGENTS.md"), "Local rules\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["instructions", "inspect", "--workspace"])
        .arg(workspace.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("instructions: 1 loaded / 1 sources"));
    assert!(stdout.contains("loaded stable_prefix AGENTS.md"));
    assert!(!stdout.contains("Local rules"));
}

#[test]
fn instructions_inspect_rejects_outside_target() {
    let workspace = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["instructions", "inspect", "--workspace"])
        .arg(workspace.path())
        .arg("--target-dir")
        .arg(outside.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("target_dir must be within workspace_root"));
}

#[test]
fn agent_run_with_instructions_reports_sources_and_traces_metadata() {
    let workspace = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    std::fs::write(
        workspace.path().join("AGENTS.md"),
        "Prefer concise answers.\napi_key = sk-test\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "mock",
            "--goal",
            "summarize",
            "--instructions",
            "--workspace",
        ])
        .arg(workspace.path())
        .args(["--json", "--data-dir"])
        .arg(data_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let trace_id = payload["trace_id"].as_str().unwrap();

    assert_eq!(
        payload["instruction_sources"][0]["relative_path"],
        "AGENTS.md"
    );
    assert_eq!(payload["instruction_warning_count"], 1);
    assert!(!stdout.contains("Prefer concise answers"));
    assert!(!stdout.contains("sk-test"));
    assert!(!stdout.contains("api_key"));

    let event_page = list_events(data_dir.path(), trace_id, None, None).unwrap();
    let record = event_page
        .records
        .iter()
        .find(|record| record.event_kind == "instructions_discovered")
        .unwrap();
    assert_eq!(record.payload["sources"][0]["relative_path"], "AGENTS.md");
    let encoded_payload = serde_json::to_string(&record.payload).unwrap();
    assert!(!encoded_payload.contains("Prefer concise answers"));
    assert!(!encoded_payload.contains("sk-test"));
}

#[test]
fn agent_run_with_skill_reports_activation_and_traces_metadata() {
    let workspace = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    let skill_dir = workspace.path().join(".tessera/skills/reviewer");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: reviewer\ndescription: Review safely\n---\n\nUse this skill.\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "mock",
            "--goal",
            "Review this repository",
            "--skill",
            "skill_reviewer",
            "--workspace",
        ])
        .arg(workspace.path())
        .args(["--json", "--data-dir"])
        .arg(data_dir.path())
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let trace_id = payload["trace_id"].as_str().unwrap();

    assert_eq!(
        payload["skill_activations"][0]["skill_id"],
        "skill_reviewer"
    );
    assert_eq!(payload["skill_warning_count"], 0);
    assert!(!stdout.contains("Use this skill."));

    let event_page = list_events(data_dir.path(), trace_id, None, None).unwrap();
    let record = event_page
        .records
        .iter()
        .find(|record| record.event_kind == "skill_activated")
        .unwrap();
    assert_eq!(record.payload["activation"]["skill_id"], "skill_reviewer");
    let encoded_payload = serde_json::to_string(&record.payload).unwrap();
    assert!(!encoded_payload.contains("Use this skill."));
}

#[test]
fn agent_run_rejects_workspace_without_instructions() {
    let workspace = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "mock",
            "--goal",
            "summarize",
            "--workspace",
        ])
        .arg(workspace.path())
        .args(["--data-dir"])
        .arg(data_dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--workspace and --target-dir require --instructions"));
    assert!(!data_dir.path().join("traces").exists());
}

#[test]
fn agent_run_rejects_skill_reference_without_skill() {
    let workspace = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "mock",
            "--goal",
            "summarize",
            "--skill-reference",
            "skill_reviewer:references/checklist.md",
            "--workspace",
        ])
        .arg(workspace.path())
        .args(["--data-dir"])
        .arg(data_dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--skill-reference requires --skill"));
    assert!(!data_dir.path().join("traces").exists());
}

#[test]
fn agent_run_rejects_missing_skill_before_provider_execution() {
    let workspace = tempfile::tempdir().unwrap();
    let data_dir = tempfile::tempdir().unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "mock",
            "--goal",
            "summarize",
            "--skill",
            "skill_missing",
            "--workspace",
        ])
        .arg(workspace.path())
        .args(["--data-dir"])
        .arg(data_dir.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("requested skill not found: skill_missing"));
    assert!(!data_dir.path().join("traces").exists());
}

#[test]
fn agent_run_requires_goal() {
    let temp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["agent", "run", "--provider", "mock", "--json", "--data-dir"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--goal <GOAL>"));
}

#[test]
fn agent_run_unknown_provider_fails_before_trace_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args([
            "agent",
            "run",
            "--provider",
            "missing",
            "--goal",
            "summarize",
            "--data-dir",
        ])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("provider profile not found: missing"));
    assert!(!temp.path().join("traces").exists());
}

#[test]
fn tui_state_uses_configured_profiles_for_switching() {
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![
            ProviderProfile {
                id: "offline".to_string(),
                kind: "mock".to_string(),
                default_model: "mock-chat".to_string(),
                base_url: None,
                api_key_env: None,
            },
            ProviderProfile {
                id: "local".to_string(),
                kind: "ollama".to_string(),
                default_model: "llama3".to_string(),
                base_url: None,
                api_key_env: None,
            },
        ],
    };

    let state = build_tui_state_with_config(&config, "local").unwrap();

    assert_eq!(state.status.active_profile, "local");
    assert_eq!(state.status.available_profiles, vec!["offline", "local"]);
}

#[test]
fn tui_state_rejects_unknown_initial_profile() {
    let config = TesseraConfig::default_with_mock();

    let error = build_tui_state_with_config(&config, "missing")
        .unwrap_err()
        .to_string();

    assert!(error.contains("provider profile not found"));
}

#[test]
fn repl_parser_recognizes_local_slash_commands() {
    assert_eq!(parse_repl_command("hello repl"), None);
    assert_eq!(parse_repl_command("/help"), Some(CliReplCommand::Help));
    assert_eq!(parse_repl_command("/commands"), Some(CliReplCommand::Help));
    assert_eq!(parse_repl_command("/new"), Some(CliReplCommand::NewThread));
    assert_eq!(parse_repl_command("/clear"), Some(CliReplCommand::Clear));
    assert_eq!(parse_repl_command("/cancel"), Some(CliReplCommand::Cancel));
    assert_eq!(
        parse_repl_command("/pause"),
        Some(CliReplCommand::PauseTask(None))
    );
    assert_eq!(
        parse_repl_command("/pause task_cli_pause"),
        Some(CliReplCommand::PauseTask(Some(
            "task_cli_pause".to_string()
        )))
    );
    assert_eq!(
        parse_repl_command("/resume-task task_cli_pause"),
        Some(CliReplCommand::ResumeTask("task_cli_pause".to_string()))
    );
    assert_eq!(
        parse_repl_command("/resume-tasks"),
        Some(CliReplCommand::ResumeTasks)
    );
    assert_eq!(parse_repl_command("/paste"), Some(CliReplCommand::Paste));
    assert_eq!(
        parse_repl_command("/profiles"),
        Some(CliReplCommand::Profiles)
    );
    assert_eq!(
        parse_repl_command("/history"),
        Some(CliReplCommand::History)
    );
    assert_eq!(parse_repl_command("/status"), Some(CliReplCommand::Status));
    assert_eq!(parse_repl_command("/export"), Some(CliReplCommand::Export));
    assert_eq!(parse_repl_command("/quit"), Some(CliReplCommand::Quit));
    assert_eq!(parse_repl_command("/exit"), Some(CliReplCommand::Quit));
    assert_eq!(
        parse_repl_command("/profile offline"),
        Some(CliReplCommand::SwitchProfile("offline".to_string()))
    );
    assert_eq!(
        parse_repl_command("/does-not-exist"),
        Some(CliReplCommand::Unknown("/does-not-exist".to_string()))
    );
    assert_eq!(
        parse_repl_command("/sessions"),
        Some(CliReplCommand::Sessions)
    );
    assert_eq!(parse_repl_command("/doctor"), Some(CliReplCommand::Doctor));
    assert_eq!(
        parse_repl_command("/resume trace_123"),
        Some(CliReplCommand::ResumeSession("trace_123".to_string()))
    );
}

#[test]
fn repl_session_accepts_pause_resume_commands_without_runtime_execution() {
    let config = TesseraConfig::default_with_mock();
    let mut session = CliReplSession::new(&config, "mock").unwrap();

    let pause = session
        .handle_command(
            &config,
            CliReplCommand::PauseTask(Some("task_cli_pause".to_string())),
        )
        .unwrap();
    let resume = session
        .handle_command(
            &config,
            CliReplCommand::ResumeTask("task_cli_pause".to_string()),
        )
        .unwrap();

    assert!(!pause.should_quit);
    assert!(pause.lines.join("\n").contains("metadata-only"));
    assert!(pause.lines.join("\n").contains("no runtime execution"));
    assert!(!resume.should_quit);
    assert!(resume.lines.join("\n").contains("metadata-only"));
    assert!(resume.lines.join("\n").contains("no runtime execution"));
}

#[test]
fn repl_session_switches_profiles_and_rejects_unknown_profiles() {
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![
            ProviderProfile {
                id: "offline".to_string(),
                kind: "mock".to_string(),
                default_model: "mock-chat".to_string(),
                base_url: None,
                api_key_env: None,
            },
            ProviderProfile {
                id: "local".to_string(),
                kind: "ollama".to_string(),
                default_model: "llama3".to_string(),
                base_url: None,
                api_key_env: None,
            },
        ],
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();

    let outcome = session
        .handle_command(&config, CliReplCommand::SwitchProfile("local".to_string()))
        .unwrap();

    assert!(!outcome.should_quit);
    assert_eq!(session.snapshot().status.active_profile, "local");
    assert!(outcome
        .lines
        .join("\n")
        .contains("profile switched to local"));

    let error = session
        .handle_command(
            &config,
            CliReplCommand::SwitchProfile("missing".to_string()),
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("provider profile not found"));
    assert_eq!(session.snapshot().status.active_profile, "local");
}

#[test]
fn repl_session_handles_local_commands_without_runtime_work() {
    let config = TesseraConfig::default_with_mock();
    let mut session = CliReplSession::new(&config, "mock").unwrap();
    session.snapshot_mut().push_notice("temporary note");

    let status = session
        .handle_command(&config, CliReplCommand::Status)
        .unwrap();
    assert!(status.lines.join("\n").contains("profile mock"));

    let export = session
        .handle_command(&config, CliReplCommand::Export)
        .unwrap();
    assert!(export.lines.join("\n").contains("temporary note"));

    let new_thread = session
        .handle_command(&config, CliReplCommand::NewThread)
        .unwrap();
    assert!(new_thread.lines.join("\n").contains("new thread"));
    assert!(session.snapshot().projection.messages.is_empty());

    let unknown = session
        .handle_command(&config, CliReplCommand::Unknown("/danger".to_string()))
        .unwrap();
    assert!(unknown.lines.join("\n").contains("unknown command"));

    let cancel = session
        .handle_command(&config, CliReplCommand::Cancel)
        .unwrap();
    assert!(cancel.lines.join("\n").contains("no active run to cancel"));

    let quit = session
        .handle_command(&config, CliReplCommand::Quit)
        .unwrap();
    assert!(quit.should_quit);
}

#[test]
fn repl_session_lists_and_clears_visible_history_without_runtime_work() {
    let config = TesseraConfig::default_with_mock();
    let mut session = CliReplSession::new(&config, "mock").unwrap();

    let empty_history = session
        .handle_command(&config, CliReplCommand::History)
        .unwrap();
    assert!(empty_history
        .lines
        .join("\n")
        .contains("no messages in current thread"));

    session.snapshot_mut().push_notice("temporary note");
    let history = session
        .handle_command(&config, CliReplCommand::History)
        .unwrap();
    assert!(history
        .lines
        .join("\n")
        .contains("1. system: temporary note"));

    let clear = session
        .handle_command(&config, CliReplCommand::Clear)
        .unwrap();
    assert!(clear.lines.join("\n").contains("current thread cleared"));
    assert!(session.snapshot().projection.messages.is_empty());
}

#[test]
fn repl_doctor_reports_runtime_health_for_active_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![
            ProviderProfile {
                id: "offline".to_string(),
                kind: "mock".to_string(),
                default_model: "mock-chat".to_string(),
                base_url: None,
                api_key_env: None,
            },
            ProviderProfile {
                id: "local".to_string(),
                kind: "ollama".to_string(),
                default_model: "llama3".to_string(),
                base_url: None,
                api_key_env: None,
            },
        ],
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();

    let outcome = session
        .handle_command_with_data_dir(&data_dir, &config, CliReplCommand::Doctor)
        .unwrap();

    let lines = outcome.lines.join("\n");
    assert!(!outcome.should_quit);
    assert!(lines.contains("status: ok"));
    assert!(lines.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(lines.contains("trace_writable: true"));
    assert!(lines.contains("sqlite_index_healthy: true"));
    assert!(lines.contains("provider_profiles: offline, local"));
}

#[tokio::test]
async fn repl_startup_prints_runtime_context_before_first_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![
            ProviderProfile {
                id: "offline".to_string(),
                kind: "mock".to_string(),
                default_model: "mock-chat".to_string(),
                base_url: None,
                api_key_env: None,
            },
            ProviderProfile {
                id: "local".to_string(),
                kind: "ollama".to_string(),
                default_model: "llama3".to_string(),
                base_url: None,
                api_key_env: None,
            },
        ],
    };
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        data_dir.clone(),
        config,
        "local".to_string(),
        None,
        "/quit\n".as_bytes(),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert_eq!(snapshot.status.active_profile, "local");
    assert!(stdout.contains("Tessera CLI interactive chat"));
    assert!(stdout.contains("active_profile: local"));
    assert!(stdout.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(stdout.contains("available_profiles: offline, local"));
    assert!(stdout.contains("type /help or run `tessera chat --list-commands`"));
    assert!(stdout.contains("/doctor"));
}

#[tokio::test]
async fn repl_prompt_streams_live_events_into_client_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();
    let mut streamed_text = String::new();

    let outcome =
        run_repl_prompt_with_writer(temp.path(), &config, &mut session, "hello repl", |delta| {
            streamed_text.push_str(delta)
        })
        .await
        .unwrap();

    assert!(streamed_text.contains("mock response"));
    assert_eq!(outcome.assistant_text, streamed_text);
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello repl"));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content.contains("mock response")));

    let mut follow_up_text = String::new();
    run_repl_prompt_with_writer(
        temp.path(),
        &config,
        &mut session,
        "continue from that",
        |delta| follow_up_text.push_str(delta),
    )
    .await
    .unwrap();

    assert!(follow_up_text.contains("history messages: 3"));
}

#[tokio::test]
async fn repl_paste_mode_submits_multiline_prompt_and_can_cancel() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        "/paste\nfirst pasted line\nsecond pasted line\n/send\n/paste\nignored pasted line\n/cancel\n/history\n/quit\n".as_bytes(),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("paste mode; end with /send or /cancel"));
    assert!(stdout.contains("paste cancelled"));
    assert!(stdout.contains("assistant> mock response to: first pasted line"));
    assert!(stdout.contains("1. user: first pasted line second pasted line"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "first pasted line\nsecond pasted line"));
    assert!(!snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content.contains("ignored pasted line")));
}

#[tokio::test]
async fn repl_cancel_interrupts_active_run_and_records_cancelled_trace() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-slow".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        DelayedLineReader::with_steps([
            DelayedLine::after(Duration::ZERO, "cancel this slow run\n"),
            DelayedLine::after_trace_event(
                temp.path().to_path_buf(),
                "provider_request_started",
                "/cancel\n",
            ),
            DelayedLine::after(Duration::ZERO, "/quit\n"),
        ]),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("cancel requested"));
    assert!(!stdout.contains("no active run to cancel"));
    assert_eq!(snapshot.status.task_summary, "task cancelled");

    let sessions = list_sessions(temp.path()).unwrap();
    assert_eq!(sessions.len(), 1);
    let event_page = list_events(temp.path(), &sessions[0].trace_id, None, None).unwrap();
    let event_kinds = event_page
        .records
        .iter()
        .map(|record| record.event_kind.as_str())
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"provider_request_started"));
    assert!(event_kinds.contains(&"task_cancelled"));
    assert!(!event_kinds.contains(&"task_completed"));
}

#[tokio::test]
async fn repl_pause_interrupts_active_run_and_records_paused_trace() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-slow".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        DelayedLineReader::with_steps([
            DelayedLine::after(Duration::ZERO, "pause this slow run\n"),
            DelayedLine::after_trace_event(
                temp.path().to_path_buf(),
                "provider_request_started",
                "/pause\n",
            ),
            DelayedLine::after(Duration::ZERO, "/quit\n"),
        ]),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("pause requested"));
    assert!(!stdout.contains("metadata-only CLI intent"));
    assert_eq!(snapshot.status.task_summary, "task paused");

    let sessions = list_sessions(temp.path()).unwrap();
    assert_eq!(sessions.len(), 1);
    let event_page = list_events(temp.path(), &sessions[0].trace_id, None, None).unwrap();
    let event_kinds = event_page
        .records
        .iter()
        .map(|record| record.event_kind.as_str())
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"provider_request_started"));
    assert!(event_kinds.contains(&"task_paused"));
    assert!(!event_kinds.contains(&"task_cancelled"));
    assert!(!event_kinds.contains(&"task_completed"));
}

#[tokio::test]
async fn repl_resume_task_runs_chat_from_pause_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before resume");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        temp.path(),
        &config,
        "offline",
        "hello paused task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();
    let input = format!("/resume-task {paused_task_id}\n/quit\n");
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        Cursor::new(input.into_bytes()),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains(&format!(
        "resuming task {paused_task_id} from trace {}",
        paused.trace_id
    )));
    assert!(!stdout.contains("metadata-only CLI intent"));
    assert!(stdout.contains("assistant> mock response to: Continue the paused task"));
    assert!(stdout.contains("history messages: 2"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello paused task"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content.contains("mock response")));

    let event_page = list_events(temp.path(), &paused.trace_id, None, None).unwrap();
    let event_kinds = event_page
        .records
        .iter()
        .map(|record| record.event_kind.as_str())
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"task_resumed"));

    let reader = RuntimeReader::new(TraceStore::open(temp.path()).unwrap());
    let tasks = reader.list_tasks(&paused.trace_id).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_id, paused_task_id);
    assert_eq!(tasks[0].status, TaskStatus::Running);
}

#[tokio::test]
async fn repl_resume_task_rejects_task_that_is_no_longer_paused() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before duplicate resume");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        temp.path(),
        &config,
        "offline",
        "hello duplicate resume",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();
    let input = format!("/resume-task {paused_task_id}\n/resume-task {paused_task_id}\n/quit\n");
    let mut output = Vec::new();

    run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        Cursor::new(input.into_bytes()),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert_eq!(stdout.matches("resuming task ").count(), 1);
    assert!(stdout.contains(&format!("task {paused_task_id} is not paused")));
    assert!(stdout.contains("current status: running"));

    let event_page = list_events(temp.path(), &paused.trace_id, None, None).unwrap();
    let resumed_count = event_page
        .records
        .iter()
        .filter(|record| record.event_kind == "task_resumed")
        .count();
    assert_eq!(resumed_count, 1);
    assert_eq!(list_sessions(temp.path()).unwrap().len(), 2);
}

#[tokio::test]
async fn repl_resume_task_missing_provider_profile_does_not_project_trace() {
    let temp = tempfile::tempdir().unwrap();
    let pause_config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before missing profile");
    let mut paused_task_id = None;
    run_chat_with_config_and_controls_and_events(
        temp.path(),
        &pause_config,
        "offline",
        "hello missing provider profile",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();
    let resume_config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "other".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let input = format!("/resume-task {paused_task_id}\n/quit\n");
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        resume_config,
        "other".to_string(),
        None,
        Cursor::new(input.into_bytes()),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("error: provider profile not found: offline"));
    assert!(!stdout.contains("resuming task "));
    assert!(!stdout.contains("assistant>"));
    assert!(!snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello missing provider profile"));
    assert_eq!(snapshot.status.active_profile, "other");
}

#[tokio::test]
async fn repl_resume_task_missing_checkpoint_does_not_start_run_or_project_trace() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig::default_with_mock();
    let input = "/resume-task task_missing_checkpoint\n/quit\n";
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "mock".to_string(),
        None,
        Cursor::new(input.as_bytes()),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("error: pause checkpoint not found for task: task_missing_checkpoint"));
    assert!(!stdout.contains("resuming task "));
    assert!(!stdout.contains("assistant>"));
    assert!(snapshot.projection.messages.is_empty());
    assert_eq!(snapshot.status.active_profile, "mock");
}

#[tokio::test]
async fn repl_resume_tasks_lists_resumable_paused_checkpoints_without_runtime_work() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before list");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        temp.path(),
        &config,
        "offline",
        "hello listed paused task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();
    let input = "/resume-tasks\n/quit\n";
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        Cursor::new(input.as_bytes()),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains(&format!("1. {paused_task_id} | trace {}", paused.trace_id)));
    assert!(stdout.contains("provider offline"));
    assert!(stdout.contains("checkpoint "));
    assert!(stdout.contains("reason test pause before list"));
    assert!(!stdout.contains("resuming task "));
    assert!(!stdout.contains("assistant>"));
    assert!(!snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello listed paused task"));
    assert_eq!(snapshot.status.active_profile, "offline");
}

#[tokio::test]
async fn top_level_task_helpers_list_resumable_paused_checkpoints_without_runtime_work() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before top-level helper list");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        temp.path(),
        &config,
        "offline",
        "hello top-level listed paused task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();

    let tasks = list_resumable_tasks(temp.path(), &config).unwrap();
    let lines = format_resumable_task_lines(&tasks);

    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task_id, paused_task_id.to_string());
    assert_eq!(tasks[0].trace_id, paused.trace_id);
    assert_eq!(tasks[0].provider_id, "offline");
    assert_eq!(
        tasks[0].resume_mode,
        tessera_protocol::ResumeMode::FromTraceProjection
    );
    assert_eq!(
        tasks[0].reason.as_deref(),
        Some("test pause before top-level helper list")
    );
    assert!(lines[0].contains(&format!("1. {paused_task_id} | trace {}", paused.trace_id)));
    assert!(lines[0].contains("provider offline"));
    assert!(lines[0].contains("checkpoint "));
}

#[tokio::test]
async fn top_level_tasks_command_lists_resumable_paused_checkpoints_without_runtime_work() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before top-level command list");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        &data_dir,
        &config,
        "offline",
        "hello top-level command paused task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["tasks", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("1. {paused_task_id} | trace {}", paused.trace_id)));
    assert!(stdout.contains("provider offline"));
    assert!(stdout.contains("checkpoint "));
    assert!(stdout.contains("reason test pause before top-level command list"));
    assert!(!stdout.contains("assistant>"));
}

fn write_task_owner_trace(data_dir: &std::path::Path, trace_id: &str) -> (TaskId, TaskOwnershipId) {
    let mut store = TraceStore::open(data_dir).unwrap();
    let task_id = TaskId::from_static("task_cli_owner");
    let lease_id = TaskOwnershipId::from_static("task_owner_cli");
    let lease = TaskOwnerLease {
        lease_id: lease_id.clone(),
        task_id: task_id.clone(),
        trace_id: trace_id.to_string(),
        runtime_id: RuntimeInstanceId::from_static("runtime_cli_owner"),
        client_id: Some(ClientInstanceId::from_static("client_cli_owner")),
        owner_kind: TaskOwnerKind::Execution,
        status: TaskOwnerStatus::Attached,
        acquired_at: Timestamp::now_utc(),
        heartbeat_interval_ms: 5_000,
        expires_at: None,
        last_heartbeat_at: None,
        last_seq: Some(3),
        reason: Some("cli owner test".to_string()),
    };

    store
        .append(
            &EventFrame::new(
                trace_id,
                1,
                RunEvent::TaskOwnerAttached {
                    lease: Box::new(lease.clone()),
                },
            )
            .with_task_id(task_id.clone()),
        )
        .unwrap();
    store
        .append(
            &EventFrame::new(
                trace_id,
                2,
                RunEvent::TaskOwnerLost {
                    lease_id: lease_id.clone(),
                    task_id: task_id.clone(),
                    reason: Some("heartbeat expired".to_string()),
                },
            )
            .with_task_id(task_id.clone()),
        )
        .unwrap();

    (task_id, lease_id)
}

#[test]
fn top_level_task_owner_helpers_project_trace_without_runtime_work() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let trace_id = "trace_cli_owner_helpers";
    let (task_id, lease_id) = write_task_owner_trace(&data_dir, trace_id);

    let owners = list_task_owners(&data_dir, trace_id).unwrap();
    let lines = format_task_owner_lines(&owners);

    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].task_id, task_id.to_string());
    assert_eq!(owners[0].lease_id, lease_id.to_string());
    assert_eq!(owners[0].status, TaskOwnerStatus::Lost);
    assert_eq!(
        owners[0].reattach_mode,
        tessera_protocol::TaskReattachMode::OwnerLost
    );
    assert!(lines[0].contains(&format!("1. {task_id} | lease {lease_id}")));
    assert!(lines[0].contains("runtime runtime_cli_owner"));
    assert!(lines[0].contains("status lost"));
    assert!(lines[0].contains("reattach owner_lost"));
}

#[test]
fn top_level_tasks_command_lists_task_owners_without_runtime_work() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let trace_id = "trace_cli_owner_command";
    let (task_id, lease_id) = write_task_owner_trace(&data_dir, trace_id);

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["tasks", "--owners", "--trace", trace_id, "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("1. {task_id} | lease {lease_id}")));
    assert!(stdout.contains("status lost"));
    assert!(stdout.contains("reason heartbeat expired"));
    assert!(!stdout.contains("assistant>"));
}

#[tokio::test]
async fn top_level_tasks_command_emits_json_for_resumable_paused_checkpoints() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before top-level json list");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        &data_dir,
        &config,
        "offline",
        "hello top-level json paused task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["tasks", "--config"])
        .arg(&config_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let tasks: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(tasks[0]["task_id"], paused_task_id.to_string());
    assert_eq!(tasks[0]["trace_id"], paused.trace_id);
    assert_eq!(tasks[0]["provider_id"], "offline");
    assert_eq!(tasks[0]["resume_mode"], "from_trace_projection");
    assert_eq!(tasks[0]["reason"], "test pause before top-level json list");
    assert!(tasks[0]["checkpoint_id"]
        .as_str()
        .unwrap()
        .starts_with("task_pause_checkpoint_"));
}

#[tokio::test]
async fn top_level_chat_resume_task_runs_chat_from_pause_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before top-level resume");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        &data_dir,
        &config,
        "offline",
        "hello top-level resumable task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--resume-task", paused_task_id.as_str()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!(
        "resuming task {paused_task_id} from trace {}",
        paused.trace_id
    )));
    assert!(stdout.contains("mock response to: Continue the paused task"));
    assert!(stdout.contains("history messages: 2"));

    let event_page = list_events(&data_dir, &paused.trace_id, None, None).unwrap();
    let resumed_count = event_page
        .records
        .iter()
        .filter(|record| record.event_kind == "task_resumed")
        .count();
    assert_eq!(resumed_count, 1);
}

#[tokio::test]
async fn top_level_chat_resume_task_accepts_numbered_selector() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before top-level numbered resume");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        &data_dir,
        &config,
        "offline",
        "hello top-level numbered resumable task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--resume-task", "1"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("resuming task {paused_task_id}")));
    assert!(stdout.contains("mock response to: Continue the paused task"));

    let event_page = list_events(&data_dir, &paused.trace_id, None, None).unwrap();
    let resumed_count = event_page
        .records
        .iter()
        .filter(|record| record.event_kind == "task_resumed")
        .count();
    assert_eq!(resumed_count, 1);
}

#[test]
fn chat_resume_task_rejects_prompt_resume_continue_and_json_options() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        r#"
data_dir = "./data"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
    )
    .unwrap();

    for args in [
        vec![
            "chat",
            "--config",
            config_path.to_str().unwrap(),
            "--resume-task",
            "1",
            "--prompt",
            "hello",
        ],
        vec![
            "chat",
            "--config",
            config_path.to_str().unwrap(),
            "--resume-task",
            "1",
            "--resume",
            "trace_old",
        ],
        vec![
            "chat",
            "--config",
            config_path.to_str().unwrap(),
            "--resume-task",
            "1",
            "--continue",
        ],
        vec![
            "chat",
            "--config",
            config_path.to_str().unwrap(),
            "--resume-task",
            "1",
            "--json",
        ],
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
            .args(args)
            .output()
            .unwrap();

        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("--resume-task cannot be combined"));
    }
}

#[tokio::test]
async fn repl_resume_task_accepts_numbered_resume_task_selector() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before numbered resume");
    let mut paused_task_id = None;
    let paused = run_chat_with_config_and_controls_and_events(
        temp.path(),
        &config,
        "offline",
        "hello numbered paused task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |frame| {
            if let RunEvent::TaskPaused { task_id, .. } = &frame.event {
                paused_task_id = Some(task_id.clone());
            }
            EventSinkAction::Continue
        },
    )
    .await
    .unwrap();
    let paused_task_id = paused_task_id.unwrap();
    let input = "/resume-tasks\n/resume-task 1\n/quit\n";
    let mut output = Vec::new();

    run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        Cursor::new(input.as_bytes()),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains(&format!("1. {paused_task_id} | trace {}", paused.trace_id)));
    assert!(stdout.contains(&format!("resuming task {paused_task_id}")));
    assert!(stdout.contains("assistant> mock response to: Continue the paused task"));

    let event_page = list_events(temp.path(), &paused.trace_id, None, None).unwrap();
    let resumed_count = event_page
        .records
        .iter()
        .filter(|record| record.event_kind == "task_resumed")
        .count();
    assert_eq!(resumed_count, 1);
}

#[tokio::test]
async fn repl_resume_task_rejects_out_of_range_numbered_selector_without_runtime_work() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let pause_token = RunPauseToken::new();
    pause_token.pause("test pause before out of range selector");
    run_chat_with_config_and_controls_and_events(
        temp.path(),
        &config,
        "offline",
        "hello out of range paused task",
        RunControls {
            event_timeout: None,
            cancellation_token: None,
            pause_token: Some(pause_token),
        },
        |_| EventSinkAction::Continue,
    )
    .await
    .unwrap();
    let input = "/resume-task #2\n/quit\n";
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        Cursor::new(input.as_bytes()),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("error: resume task index out of range: 2 (available tasks: 1)"));
    assert!(!stdout.contains("resuming task "));
    assert!(!stdout.contains("assistant>"));
    assert!(!snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello out of range paused task"));
    assert_eq!(snapshot.status.active_profile, "offline");
}

#[test]
fn init_config_template_writes_secret_safe_profiles_and_respects_force() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");

    write_config_template(&config_path, false).unwrap();
    let template = std::fs::read_to_string(&config_path).unwrap();

    assert!(template.contains("[[providers]]"));
    assert!(template.contains("id = \"mock\""));
    assert!(template.contains("id = \"ollama\""));
    assert!(template.contains("id = \"openai-compatible\""));
    assert!(template.contains("api_key_env = \"TESSERA_OPENAI_COMPATIBLE_API_KEY\""));
    assert!(!template.contains("sk-"));
    assert!(!template.contains("Bearer "));

    let error = write_config_template(&config_path, false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("already exists"));

    write_config_template(&config_path, true).unwrap();
}

#[tokio::test]
async fn repl_sessions_and_resume_use_trace_projection_without_provider_call() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let trace_id = {
        let output = run_chat_with_config(temp.path(), &config, "offline", "hello resumable")
            .await
            .unwrap();
        output.trace_id
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();

    let sessions = session
        .handle_command_with_data_dir(temp.path(), &config, CliReplCommand::Sessions)
        .unwrap();
    assert!(sessions.lines.join("\n").contains(&trace_id));
    assert!(sessions.lines[0].starts_with("1. "));

    let resumed = session
        .handle_command_with_data_dir(
            temp.path(),
            &config,
            CliReplCommand::ResumeSession(trace_id.clone()),
        )
        .unwrap();

    assert!(resumed
        .lines
        .join("\n")
        .contains(&format!("resumed trace {trace_id}")));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello resumable"));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content.contains("mock response")));

    let mut follow_up_text = String::new();
    run_repl_prompt_with_writer(
        temp.path(),
        &config,
        &mut session,
        "continue from that",
        |delta| follow_up_text.push_str(delta),
    )
    .await
    .unwrap();

    assert!(follow_up_text.contains("history messages: 3"));
}

#[tokio::test]
async fn repl_resume_accepts_numbered_session_index() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let older_trace_id = run_chat_with_config(temp.path(), &config, "offline", "older indexed")
        .await
        .unwrap()
        .trace_id;
    let latest_trace_id = run_chat_with_config(temp.path(), &config, "offline", "latest indexed")
        .await
        .unwrap()
        .trace_id;
    let sessions = list_sessions(temp.path()).unwrap();
    let first_trace_id = sessions[0].trace_id.clone();
    let first_prompt = if first_trace_id == older_trace_id {
        "older indexed"
    } else {
        assert_eq!(first_trace_id, latest_trace_id);
        "latest indexed"
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();

    let listed = session
        .handle_command_with_data_dir(temp.path(), &config, CliReplCommand::Sessions)
        .unwrap();
    assert!(listed.lines[0].starts_with("1. "));
    assert!(listed.lines[0].contains(&first_trace_id));

    let resumed = session
        .handle_command_with_data_dir(
            temp.path(),
            &config,
            CliReplCommand::ResumeSession("1".to_string()),
        )
        .unwrap();

    assert!(resumed
        .lines
        .join("\n")
        .contains(&format!("resumed trace {first_trace_id}")));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content == first_prompt));

    let error = session
        .handle_command_with_data_dir(
            temp.path(),
            &config,
            CliReplCommand::ResumeSession("3".to_string()),
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("session index out of range: 3"));
    assert!(error.contains("available sessions: 2"));
}

#[tokio::test]
async fn repl_can_start_from_resume_trace_id_and_continue_with_history() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let trace_id = run_chat_with_config(temp.path(), &config, "offline", "hello startup resume")
        .await
        .unwrap()
        .trace_id;
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        Some(trace_id.clone()),
        "continue from startup\n/quit\n".as_bytes(),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains(&format!("resumed trace {trace_id}")));
    assert!(stdout.contains("history messages: 3"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello startup resume"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "continue from startup"));
}

#[tokio::test]
async fn chat_continue_starts_from_latest_trace_and_continues_with_history() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    run_chat_with_config(&data_dir, &config, "offline", "older session")
        .await
        .unwrap();
    let latest_trace_id = run_chat_with_config(&data_dir, &config, "offline", "latest session")
        .await
        .unwrap()
        .trace_id;

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--continue"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"continue latest\n/quit\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("resumed trace {latest_trace_id}")));
    assert!(stdout.contains("history messages: 3"));
    assert!(stdout.contains("continue latest"));
}

#[test]
fn chat_command_uses_default_tessera_toml_from_current_directory() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .current_dir(temp.path())
        .args([
            "chat",
            "--provider",
            "offline",
            "--prompt",
            "hello default config",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock response to: hello default config"));
}

#[test]
fn chat_command_uses_tessera_config_env_when_no_config_flag_is_passed() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("real-test.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .env("TESSERA_CONFIG", &config_path)
        .args([
            "chat",
            "--provider",
            "offline",
            "--prompt",
            "hello env config",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock response to: hello env config"));
}

#[test]
fn chat_continue_rejects_missing_session() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--continue"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("no sessions found to continue"));
}

#[test]
fn chat_continue_rejects_one_shot_prompt_sources() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--continue", "--prompt", "hello"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--continue cannot be combined"));
}

#[tokio::test]
async fn sessions_command_lists_trace_backed_sessions_from_configured_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello sessions")
        .await
        .unwrap()
        .trace_id;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["sessions", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&trace_id));
    assert!(stdout.contains("1. "));
    assert!(stdout.contains("hello sessions"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["sessions", "--config"])
        .arg(&config_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let sessions: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(sessions[0]["trace_id"], trace_id);
    assert_eq!(sessions[0]["user_preview"], "hello sessions");
}

#[tokio::test]
async fn transcript_command_exports_markdown_and_json_from_configured_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello transcript")
        .await
        .unwrap()
        .trace_id;

    let markdown_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["transcript"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(markdown_output.status.success());
    let markdown = String::from_utf8(markdown_output.stdout).unwrap();
    assert!(markdown.contains("# Tessera Export"));
    assert!(markdown.contains("## User"));
    assert!(markdown.contains("hello transcript"));
    assert!(markdown.contains("## Assistant"));
    assert!(markdown.contains("mock response to: hello transcript"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["transcript"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let transcript: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(transcript["trace_id"], trace_id);
    assert_eq!(transcript["messages"][0]["role"], "user");
    assert_eq!(transcript["messages"][0]["content"], "hello transcript");
    assert_eq!(transcript["messages"][1]["role"], "assistant");
    assert_eq!(
        transcript["messages"][1]["content"],
        "mock response to: hello transcript"
    );
}

#[tokio::test]
async fn replay_command_reconstructs_trace_summary_without_provider_call() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello replay cli")
        .await
        .unwrap()
        .trace_id;

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["replay"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains(&trace_id));
    assert!(text.contains("events:"));
    assert!(text.contains("mock response to: hello replay cli"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["replay"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let replay: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(replay["trace_id"], trace_id);
    assert_eq!(
        replay["assistant_text"],
        "mock response to: hello replay cli"
    );
    assert!(replay["event_count"].as_u64().unwrap() > 0);
    assert!(replay["event_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind == "assistant_delta"));
}

#[tokio::test]
async fn events_command_pages_trace_events_from_configured_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello events cli")
        .await
        .unwrap()
        .trace_id;

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["events"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .args(["--limit", "2"])
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains(&trace_id));
    assert!(text.contains("1 |"));
    assert!(text.contains("next_since_seq: 2"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["events"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .args(["--since", "1", "--limit", "2", "--json"])
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let page: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(page["trace_id"], trace_id);
    assert_eq!(page["records"].as_array().unwrap().len(), 2);
    assert!(page["records"][0]["seq"].as_u64().unwrap() > 1);
    assert_eq!(page["next_since_seq"], page["records"][1]["seq"]);
    assert!(!page["records"][0]["event_kind"]
        .as_str()
        .unwrap()
        .is_empty());
}

#[test]
fn profiles_command_lists_configured_profiles_without_secret_values() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        r#"
data_dir = "./data"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"
base_url = "https://example.invalid/v1"
api_key_env = "TESSERA_TEST_PROFILE_SECRET"
"#,
    )
    .unwrap();

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["profiles", "--config"])
        .arg(&config_path)
        .env("TESSERA_TEST_PROFILE_SECRET", "super-secret-profile-key")
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains("offline | mock | model mock-chat"));
    assert!(text.contains("remote | openai-compatible | model test-model"));
    assert!(text.contains("base_url https://example.invalid/v1"));
    assert!(text.contains("api_key_env TESSERA_TEST_PROFILE_SECRET"));
    assert!(!text.contains("super-secret-profile-key"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["profiles", "--config"])
        .arg(&config_path)
        .arg("--json")
        .env("TESSERA_TEST_PROFILE_SECRET", "super-secret-profile-key")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let profiles: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(profiles[0]["id"], "offline");
    assert_eq!(profiles[0]["kind"], "mock");
    assert_eq!(profiles[0]["default_model"], "mock-chat");
    assert_eq!(profiles[0]["base_url"], serde_json::Value::Null);
    assert_eq!(profiles[0]["api_key_env"], serde_json::Value::Null);
    assert_eq!(profiles[1]["id"], "remote");
    assert_eq!(profiles[1]["base_url"], "https://example.invalid/v1");
    assert_eq!(profiles[1]["api_key_env"], "TESSERA_TEST_PROFILE_SECRET");
    assert!(!String::from_utf8(json_output.stdout)
        .unwrap()
        .contains("super-secret-profile-key"));
}

#[test]
fn config_validate_reports_ok_profiles_without_secret_values() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"
base_url = "https://example.invalid/v1"
api_key_env = "TESSERA_TEST_VALIDATE_API_KEY"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .env("TESSERA_TEST_VALIDATE_API_KEY", "super-secret-validate-key")
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains("status: ok"));
    assert!(text.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(text.contains("profile offline: ok (mock, model mock-chat)"));
    assert!(text.contains(
        "profile remote: ok (openai-compatible, model test-model, api_key_env TESSERA_TEST_VALIDATE_API_KEY set)"
    ));
    assert!(!text.contains("super-secret-validate-key"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .arg("--json")
        .env("TESSERA_TEST_VALIDATE_API_KEY", "super-secret-validate-key")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    assert_eq!(report["issues"], serde_json::json!([]));
    assert_eq!(report["profiles"][1]["id"], "remote");
    assert_eq!(
        report["profiles"][1]["api_key_env"],
        "TESSERA_TEST_VALIDATE_API_KEY"
    );
    assert_eq!(report["profiles"][1]["api_key_env_status"], "set");
    assert!(!String::from_utf8(json_output.stdout)
        .unwrap()
        .contains("super-secret-validate-key"));
}

#[test]
fn config_validate_fails_for_missing_secret_env_without_touching_storage() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    let missing_env = format!("TESSERA_TEST_MISSING_VALIDATE_{}", std::process::id());
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"
base_url = "https://example.invalid/v1"
api_key_env = "{}"
"#,
            data_dir.display(),
            missing_env
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .arg("--json")
        .env_remove(&missing_env)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "error");
    assert_eq!(report["profiles"][0]["api_key_env_status"], "missing");
    assert!(report["issues"][0]["message"]
        .as_str()
        .unwrap()
        .contains(&missing_env));
    assert!(!data_dir.exists());
}

#[test]
fn config_validate_fails_when_no_provider_profiles_are_configured() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(&config_path, "providers = []\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("status: error"));
    assert!(text.contains("at least one provider profile is required"));
}

#[test]
fn config_validate_reports_provider_shape_errors() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        r#"
[[providers]]
id = "dup"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "dup"
kind = "ollama"
default_model = "llama3"

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"

[[providers]]
id = "unknown"
kind = "mystery"
default_model = "test-model"
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("status: error"));
    assert!(text.contains("duplicate provider id `dup`"));
    assert!(text.contains("provider `remote` kind openai-compatible requires base_url"));
    assert!(text.contains("unsupported provider kind `mystery`"));
}

#[tokio::test]
async fn chat_command_path_reads_prompt_from_stdin() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello from stdin\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock response to: hello from stdin"));
}

#[tokio::test]
async fn chat_command_path_reads_prompt_from_file() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    let prompt_path = temp.path().join("prompt.md");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    std::fs::write(&prompt_path, "hello from file\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--file"])
        .arg(&prompt_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock response to: hello from file"));
}

#[tokio::test]
async fn chat_command_path_emits_json_for_one_shot_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--prompt", "hello json", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["trace_id"]
        .as_str()
        .unwrap()
        .starts_with("trace_offline_"));
    assert_eq!(json["assistant_text"], "mock response to: hello json");
}

#[test]
fn chat_command_path_rejects_json_without_one_shot_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--json"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--json is only supported with --prompt, --stdin, or --file"));
}

#[test]
fn chat_command_path_rejects_multiple_prompt_sources() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    let prompt_path = temp.path().join("prompt.md");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    std::fs::write(&prompt_path, "hello from file\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--prompt", "hello", "--file"])
        .arg(&prompt_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot be combined"));
}

#[tokio::test]
async fn openai_compatible_profile_requires_declared_api_key_env_before_trace() {
    let temp = tempfile::tempdir().unwrap();
    let missing_env = "TESSERA_TEST_MISSING_API_KEY_FOR_ROUTING";
    std::env::remove_var(missing_env);
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "remote".to_string(),
            kind: "openai-compatible".to_string(),
            default_model: "test-model".to_string(),
            base_url: Some("https://example.invalid/v1".to_string()),
            api_key_env: Some(missing_env.to_string()),
        }],
    };

    let error = match run_chat_with_config(temp.path(), &config, "remote", "hello").await {
        Ok(_) => panic!("expected missing API key env to fail before provider request"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains(missing_env));
    assert!(!temp.path().join("traces/trace_remote.jsonl").exists());
}

#[test]
fn config_resolution_loads_explicit_path_and_data_dir_prefers_config() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
data_dir = "/tmp/tessera-configured"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
    )
    .unwrap();

    let config = resolve_config(Some(config_path)).unwrap();
    let data_dir = resolve_data_dir_with_config(None, &config).unwrap();

    assert_eq!(config.providers[0].id, "offline");
    assert_eq!(
        data_dir,
        std::path::PathBuf::from("/tmp/tessera-configured")
    );
}
