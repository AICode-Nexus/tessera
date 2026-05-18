use tessera_protocol::{
    AgentProfile, AgentProfileId, ApprovalId, ApprovalStatus, ArtifactId, ContextBudget, ContextId,
    ContextPlacement, ContextReference, ContextSource, ContextSourceKind, CostEstimate, Diagnostic,
    DiagnosticRange, DiagnosticReport, DiagnosticReportId, DiagnosticSeverity, EventFrame, ItemId,
    MemoryProposal, MemoryProposalId, MemoryProposalStatus, ModelProfileId, NoProgressAction,
    NoProgressLoop, NoProgressSignalKind, OsSandboxFilesystem, OsSandboxMode, OsSandboxNetwork,
    OsSandboxProfile, OsSandboxProfileId, OsSandboxShell, PolicyDecisionId, PolicyOutcome,
    ProviderCapability, ProviderId, RouteDecision, RouteDecisionId, RouteStrategy, RunEvent,
    SandboxDecision, SandboxDecisionId, SandboxDecisionKind, SkillEntrypoint,
    SkillEntrypointFormat, SkillId, SkillManifest, SkillPolicy, SkillRequirements, SkillSource,
    SkillSourceKind, SnapshotId, SnapshotKind, TaskId, ToolApproval, ToolCallId, ToolCallRequest,
    ToolDescriptor, ToolDispatch, ToolDispatchId, ToolId, ToolPermission, ToolPolicyDecision,
    ToolRepairId, ToolRepairKind, ToolRepairReport, ToolResult, ToolResultId, ToolResultStatus,
    ToolSideEffect, WorkspaceAccess, WorkspaceCheckpoint, WorkspaceGuardrail, WorkspaceScope,
};

#[test]
fn event_frame_serializes_reasoning_delta_with_artifact_refs() {
    let item_id = ItemId::new();
    let frame = EventFrame::new(
        "trace_test",
        1,
        RunEvent::AssistantReasoningDelta {
            item_id: item_id.clone(),
            text: "planning".to_string(),
        },
    )
    .with_item_id(item_id)
    .with_artifact_ref(tessera_protocol::ArtifactId::new());

    let record = frame.to_trace_record();
    assert_eq!(record.schema_version, 1);
    assert_eq!(record.event_kind, "assistant_reasoning_delta");
    assert_eq!(record.payload["text"], "planning");
    assert_eq!(record.artifact_refs.len(), 1);
}

#[test]
fn user_message_recorded_keeps_text_for_tui_replay() {
    let item_id = ItemId::new();
    let record = EventFrame::new(
        "trace_user_message",
        1,
        RunEvent::UserMessageRecorded {
            item_id: item_id.clone(),
            text: "hello from tui".to_string(),
        },
    )
    .with_item_id(item_id)
    .to_trace_record();

    assert_eq!(record.event_kind, "user_message_recorded");
    assert_eq!(record.payload["text"], "hello from tui");
}

#[test]
fn usage_and_route_decision_keep_ai_ready_telemetry() {
    let usage = RunEvent::UsageReported {
        input_tokens: Some(10),
        output_tokens: Some(4),
        total_tokens: Some(14),
        cache_read_tokens: Some(8),
        cache_write_tokens: Some(2),
        cache_miss_tokens: Some(0),
        estimated_cost: Some(CostEstimate {
            amount: 0.001,
            currency: "CNY".to_string(),
            input_cost: Some(0.0004),
            output_cost: Some(0.0006),
            cache_read_cost: Some(0.0),
            cache_write_cost: Some(0.0),
        }),
        latency_ms: Some(32),
    };

    let usage_record = EventFrame::new("trace_usage", 1, usage).to_trace_record();
    assert_eq!(usage_record.event_kind, "usage_reported");
    assert_eq!(usage_record.payload["cache_read_tokens"], 8);
    assert_eq!(usage_record.payload["estimated_cost"]["currency"], "CNY");

    let capability = ProviderCapability {
        provider_id: ProviderId::from_static("mock"),
        supports_streaming: true,
        supports_reasoning_delta: true,
        supports_cache_telemetry: true,
        supports_cost_estimate: true,
        supports_tool_calling: false,
        max_context_tokens: Some(1_000_000),
        extension: None,
    };
    let route = RunEvent::RouteDecisionRecorded {
        decision_id: RouteDecisionId::new(),
        decision: RouteDecision {
            requested_profile: Some(ModelProfileId::from_static("mock-default")),
            selected_profile: ModelProfileId::from_static("mock-default"),
            selected_model: "mock-chat".to_string(),
            reasoning_level: Some("standard".to_string()),
            strategy: RouteStrategy::Manual,
            decision_reason: Some("manual_profile_selected_auto_routing_disabled".to_string()),
            fallback_reason: None,
        },
    };

    let capability_record = EventFrame::new(
        "trace_capability",
        1,
        RunEvent::ProviderCapabilityReported {
            provider_id: ProviderId::from_static("mock"),
            capability,
        },
    )
    .to_trace_record();
    let route_record = EventFrame::new("trace_route", 1, route).to_trace_record();

    assert_eq!(capability_record.event_kind, "provider_capability_reported");
    assert_eq!(
        capability_record.payload["capability"]["supports_reasoning_delta"],
        true
    );
    assert_eq!(route_record.event_kind, "route_decision_recorded");
    assert_eq!(route_record.payload["decision"]["strategy"], "manual");
    assert_eq!(
        route_record.payload["decision"]["decision_reason"],
        "manual_profile_selected_auto_routing_disabled"
    );
}

#[test]
fn no_progress_loop_event_records_stop_without_route_escalation() {
    let task_id = TaskId::from_static("task_no_progress");
    let record = EventFrame::new(
        "trace_no_progress",
        1,
        RunEvent::NoProgressLoopDetected {
            task_id: task_id.clone(),
            signal: NoProgressLoop {
                kind: NoProgressSignalKind::NoOutput,
                consecutive_count: 1,
                threshold: 1,
                action: NoProgressAction::Stop,
                reason: "assistant_completed_without_output".to_string(),
                route_escalation_allowed: false,
            },
        },
    )
    .with_task_id(task_id)
    .to_trace_record();

    assert_eq!(record.event_kind, "no_progress_loop_detected");
    assert_eq!(record.payload["signal"]["kind"], "no_output");
    assert_eq!(record.payload["signal"]["action"], "stop");
    assert_eq!(
        record.payload["signal"]["reason"],
        "assistant_completed_without_output"
    );
    assert_eq!(record.payload["signal"]["route_escalation_allowed"], false);
}

#[test]
fn skill_manifest_schema_is_read_only_and_skill_md_compatible() {
    let manifest = SkillManifest {
        id: SkillId::from_static("skill_code_review"),
        name: "code-review".to_string(),
        version: Some("0.1.0".to_string()),
        description: "Review code changes and produce prioritized findings.".to_string(),
        source: SkillSource {
            kind: SkillSourceKind::Workspace,
            uri: Some(".tessera/skills/code-review/SKILL.md".to_string()),
        },
        entrypoint: SkillEntrypoint {
            format: SkillEntrypointFormat::SkillMd,
            path: "SKILL.md".to_string(),
        },
        requirements: SkillRequirements {
            tools: vec!["git.diff".to_string(), "filesystem.read".to_string()],
            context: vec!["workspace".to_string()],
        },
        policy: SkillPolicy {
            default_permission: "ask".to_string(),
            network: "deny".to_string(),
            write_files: "deny".to_string(),
        },
        metadata: None,
    };

    let value = serde_json::to_value(&manifest).unwrap();

    assert_eq!(value["entrypoint"]["format"], "skill_md");
    assert_eq!(value["entrypoint"]["path"], "SKILL.md");
    assert_eq!(value["policy"]["network"], "deny");
    assert_eq!(value["policy"]["write_files"], "deny");
    assert!(value.get("command").is_none());
    assert!(value.get("executable").is_none());
}

#[test]
fn agent_profile_schema_declares_role_scope_and_limits_without_runtime_execution() {
    let profile = AgentProfile {
        id: AgentProfileId::from_static("agent_profile_reviewer"),
        name: "Reviewer".to_string(),
        role: "reviewer".to_string(),
        model_profile: ModelProfileId::from_static("profile_fast"),
        skills: vec![SkillId::from_static("skill_code_review")],
        memory_scopes: vec!["workspace".to_string(), "project".to_string()],
        context_scopes: vec!["thread".to_string(), "workspace_summary".to_string()],
        tool_permissions: vec![ToolPermission::FilesystemRead, ToolPermission::Git],
        max_steps: 8,
        metadata: None,
    };

    let value = serde_json::to_value(&profile).unwrap();

    assert_eq!(value["id"], "agent_profile_reviewer");
    assert_eq!(value["role"], "reviewer");
    assert_eq!(value["model_profile"], "profile_fast");
    assert_eq!(value["skills"][0], "skill_code_review");
    assert_eq!(value["tool_permissions"][0], "filesystem_read");
    assert_eq!(value["max_steps"], 8);
    assert!(value.get("command").is_none());
    assert!(value.get("executable").is_none());
    assert!(value.get("shell").is_none());
}

#[test]
fn tool_descriptor_schema_defaults_to_not_parallel_safe_without_runtime_execution() {
    let descriptor: ToolDescriptor = serde_json::from_value(serde_json::json!({
        "id": "tool_workspace_read",
        "display_name": "Read workspace file",
        "description": "Read a file from the active workspace.",
        "input_schema": {
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" }
            }
        },
        "output_schema": {
            "type": "object",
            "properties": {
                "content": { "type": "string" }
            }
        },
        "required_permissions": ["filesystem_read"],
        "side_effects": ["read_only"]
    }))
    .unwrap();

    assert_eq!(descriptor.id, ToolId::from_static("tool_workspace_read"));
    assert!(!descriptor.parallel_safe);
    assert_eq!(
        descriptor.required_permissions,
        vec![ToolPermission::FilesystemRead]
    );
    assert_eq!(descriptor.side_effects, vec![ToolSideEffect::ReadOnly]);
    assert_eq!(descriptor.input_schema["required"][0], "path");

    let value = serde_json::to_value(&descriptor).unwrap();
    assert_eq!(value["parallel_safe"], false);
    assert_eq!(value["side_effects"][0], "read_only");
    assert!(value.get("command").is_none());
    assert!(value.get("executable").is_none());
    assert!(value.get("shell").is_none());
}

#[test]
fn tool_policy_and_approval_events_are_traceable_without_tool_execution() {
    let tool_id = ToolId::from_static("tool_workspace_write");
    let call_id = ToolCallId::from_static("tool_call_write_readme");
    let approval_id = ApprovalId::from_static("approval_write_readme");
    let request = ToolCallRequest {
        call_id: call_id.clone(),
        tool_id: tool_id.clone(),
        input: serde_json::json!({ "path": "README.md", "content": "updated" }),
        metadata: None,
    };
    let decision = ToolPolicyDecision {
        decision_id: PolicyDecisionId::from_static("policy_write_readme"),
        call_id: call_id.clone(),
        tool_id: tool_id.clone(),
        outcome: PolicyOutcome::AskUser,
        reason: "workspace write requires approval".to_string(),
        required_permissions: vec![ToolPermission::FilesystemWrite],
        side_effects: vec![ToolSideEffect::WritesWorkspace],
        approval_id: Some(approval_id.clone()),
    };
    let approval = ToolApproval {
        approval_id: approval_id.clone(),
        call_id: call_id.clone(),
        tool_id,
        status: ApprovalStatus::Approved,
        reason: Some("user approved workspace write".to_string()),
    };
    let task_id = TaskId::from_static("task_tool_policy");

    let requested = EventFrame::new(
        "trace_tool_policy",
        1,
        RunEvent::ToolCallRequested {
            request: request.clone(),
        },
    )
    .with_task_id(task_id.clone())
    .to_trace_record();
    let policy = EventFrame::new(
        "trace_tool_policy",
        2,
        RunEvent::ToolPolicyDecisionRecorded {
            decision: decision.clone(),
        },
    )
    .with_task_id(task_id.clone())
    .to_trace_record();
    let approved = EventFrame::new(
        "trace_tool_policy",
        3,
        RunEvent::ToolCallApproved {
            approval: approval.clone(),
        },
    )
    .with_task_id(task_id)
    .to_trace_record();

    assert_eq!(requested.event_kind, "tool_call_requested");
    assert_eq!(requested.payload["request"]["call_id"], call_id.as_str());
    assert_eq!(requested.payload["request"]["input"]["path"], "README.md");
    assert!(requested.payload.get("command").is_none());
    assert!(requested.payload.get("executable").is_none());
    assert!(requested.payload.get("shell").is_none());

    assert_eq!(policy.event_kind, "tool_policy_decision_recorded");
    assert_eq!(policy.payload["decision"]["outcome"], "ask_user");
    assert_eq!(
        policy.payload["decision"]["approval_id"],
        approval_id.as_str()
    );
    assert_eq!(
        policy.payload["decision"]["side_effects"][0],
        "writes_workspace"
    );

    assert_eq!(approved.event_kind, "tool_call_approved");
    assert_eq!(approved.payload["approval"]["status"], "approved");
    assert_eq!(
        approved.payload["approval"]["approval_id"],
        approval_id.as_str()
    );
}

#[test]
fn sandbox_decision_event_records_workspace_guardrail_without_execution() {
    let call_id = ToolCallId::from_static("tool_call_write_readme");
    let tool_id = ToolId::from_static("tool_workspace_write");
    let decision = SandboxDecision {
        decision_id: SandboxDecisionId::from_static("sandbox_write_readme"),
        call_id: Some(call_id.clone()),
        tool_id: Some(tool_id.clone()),
        kind: SandboxDecisionKind::AskUser,
        reason: "workspace_write_requires_approval".to_string(),
        guardrail: WorkspaceGuardrail {
            scope: WorkspaceScope {
                workspace_root: "/workspace/project".to_string(),
                allowed_roots: vec![],
                denied_roots: vec![],
            },
            requested_path: Some("README.md".to_string()),
            resolved_path: Some("/workspace/project/README.md".to_string()),
            access: WorkspaceAccess::Write,
            within_workspace: true,
            required_permissions: vec![ToolPermission::FilesystemWrite],
            side_effects: vec![ToolSideEffect::WritesWorkspace],
        },
        metadata: None,
    };
    let record = EventFrame::new(
        "trace_sandbox",
        1,
        RunEvent::SandboxDecisionRecorded {
            decision: decision.clone(),
        },
    )
    .with_task_id(TaskId::from_static("task_sandbox"))
    .to_trace_record();

    assert_eq!(record.event_kind, "sandbox_decision_recorded");
    assert_eq!(record.payload["decision"]["kind"], "ask_user");
    assert_eq!(
        record.payload["decision"]["guardrail"]["scope"]["workspace_root"],
        "/workspace/project"
    );
    assert_eq!(
        record.payload["decision"]["guardrail"]["requested_path"],
        "README.md"
    );
    assert_eq!(
        record.payload["decision"]["guardrail"]["resolved_path"],
        "/workspace/project/README.md"
    );
    assert_eq!(
        record.payload["decision"]["guardrail"]["side_effects"][0],
        "writes_workspace"
    );
    assert!(record.payload.get("command").is_none());
    assert!(record.payload.get("executable").is_none());
    assert!(record.payload.get("shell").is_none());
}

#[test]
fn os_sandbox_profile_event_records_isolation_metadata_without_runtime_execution() {
    let profile = OsSandboxProfile {
        profile_id: OsSandboxProfileId::from_static("os_sandbox_workspace_write"),
        mode: OsSandboxMode::WorkspaceWrite,
        workspace_root: Some("/workspace/project".to_string()),
        filesystem: OsSandboxFilesystem::WorkspaceWrite,
        network: OsSandboxNetwork::Disabled,
        shell: OsSandboxShell::Denied,
        requires_checkpoint: true,
        reason: "workspace_write_requires_checkpointed_sandbox".to_string(),
        metadata: None,
    };
    let record = EventFrame::new(
        "trace_os_sandbox",
        1,
        RunEvent::OsSandboxProfileSelected {
            profile: profile.clone(),
        },
    )
    .to_trace_record();

    assert_eq!(record.event_kind, "os_sandbox_profile_selected");
    assert_eq!(record.payload["profile"]["mode"], "workspace_write");
    assert_eq!(
        record.payload["profile"]["workspace_root"],
        "/workspace/project"
    );
    assert_eq!(record.payload["profile"]["filesystem"], "workspace_write");
    assert_eq!(record.payload["profile"]["network"], "disabled");
    assert_eq!(record.payload["profile"]["shell"], "denied");
    assert_eq!(record.payload["profile"]["requires_checkpoint"], true);
    assert!(record.payload.get("command").is_none());
    assert!(record.payload.get("executable").is_none());
    assert!(record.payload.get("shell_command").is_none());
}

#[test]
fn diagnostics_report_event_records_lsp_style_metadata_without_running_tools() {
    let report = DiagnosticReport {
        report_id: DiagnosticReportId::from_static("diagnostics_rust_analyzer"),
        source: "rust-analyzer".to_string(),
        diagnostics: vec![Diagnostic {
            severity: DiagnosticSeverity::Warning,
            code: Some("unused_variables".to_string()),
            message: "unused variable: value".to_string(),
            uri: Some("file:///workspace/project/src/lib.rs".to_string()),
            range: Some(DiagnosticRange {
                start_line: 12,
                start_character: 8,
                end_line: 12,
                end_character: 13,
            }),
            metadata: None,
        }],
        metadata: None,
    };
    let record = EventFrame::new(
        "trace_diagnostics",
        1,
        RunEvent::DiagnosticsReported {
            report: report.clone(),
        },
    )
    .to_trace_record();

    assert_eq!(record.event_kind, "diagnostics_reported");
    assert_eq!(record.payload["report"]["source"], "rust-analyzer");
    assert_eq!(
        record.payload["report"]["diagnostics"][0]["severity"],
        "warning"
    );
    assert_eq!(
        record.payload["report"]["diagnostics"][0]["range"]["start_line"],
        12
    );
    assert!(record.payload.get("command").is_none());
    assert!(record.payload.get("executable").is_none());
    assert!(record.payload.get("lsp_process_id").is_none());
}

#[test]
fn memory_proposal_events_record_ui_review_state_without_memory_runtime() {
    let proposal = MemoryProposal {
        proposal_id: MemoryProposalId::from_static("memory_proposal_prefers_rust"),
        status: MemoryProposalStatus::Pending,
        title: "Preferred language".to_string(),
        summary: "User prefers Rust-first implementations.".to_string(),
        source_item_id: Some(ItemId::from_static("item_user_memory_source")),
        reason: Some("explicit user preference".to_string()),
        metadata: None,
    };
    let proposed = EventFrame::new(
        "trace_memory",
        1,
        RunEvent::MemoryWriteProposed {
            proposal: proposal.clone(),
        },
    )
    .to_trace_record();
    let applied = EventFrame::new(
        "trace_memory",
        2,
        RunEvent::MemoryWriteApplied {
            proposal: MemoryProposal {
                status: MemoryProposalStatus::Applied,
                ..proposal.clone()
            },
        },
    )
    .to_trace_record();

    assert_eq!(proposed.event_kind, "memory_write_proposed");
    assert_eq!(proposed.payload["proposal"]["status"], "pending");
    assert_eq!(
        proposed.payload["proposal"]["source_item_id"],
        "item_user_memory_source"
    );
    assert_eq!(applied.event_kind, "memory_write_applied");
    assert_eq!(applied.payload["proposal"]["status"], "applied");
    assert!(proposed.payload.get("memory_store_path").is_none());
    assert!(proposed.payload.get("database_uri").is_none());
    assert!(proposed.payload.get("command").is_none());
}

#[test]
fn tool_dispatch_and_result_events_are_traceable_without_execution() {
    let call_id = ToolCallId::from_static("tool_call_read_src");
    let tool_id = ToolId::from_static("tool_workspace_read");
    let dispatch = ToolDispatch {
        dispatch_id: ToolDispatchId::from_static("tool_dispatch_read_src"),
        call_id: call_id.clone(),
        tool_id: tool_id.clone(),
        declared_index: 0,
        parallel_safe: true,
        metadata: None,
    };
    let artifact_id = ArtifactId::from_static("artifact_tool_output");
    let result = ToolResult {
        result_id: ToolResultId::from_static("tool_result_read_src"),
        call_id,
        tool_id,
        declared_index: 0,
        status: ToolResultStatus::Succeeded,
        output: serde_json::json!({ "content": "fn main() {}" }),
        error: None,
        artifact_refs: vec![artifact_id.clone()],
        metadata: None,
    };

    let started = EventFrame::new(
        "trace_tool_dispatch",
        1,
        RunEvent::ToolDispatchStarted {
            dispatch: dispatch.clone(),
        },
    )
    .to_trace_record();
    let completed = EventFrame::new(
        "trace_tool_dispatch",
        2,
        RunEvent::ToolDispatchCompleted {
            result: result.clone(),
        },
    )
    .to_trace_record();
    let visible = EventFrame::new(
        "trace_tool_dispatch",
        3,
        RunEvent::ToolResultRecorded {
            result: result.clone(),
        },
    )
    .with_artifact_ref(artifact_id.clone())
    .to_trace_record();

    assert_eq!(started.event_kind, "tool_dispatch_started");
    assert_eq!(started.payload["dispatch"]["declared_index"], 0);
    assert_eq!(started.payload["dispatch"]["parallel_safe"], true);
    assert!(started.payload.get("command").is_none());
    assert!(started.payload.get("executable").is_none());
    assert!(started.payload.get("shell").is_none());

    assert_eq!(completed.event_kind, "tool_dispatch_completed");
    assert_eq!(completed.payload["result"]["declared_index"], 0);
    assert_eq!(completed.payload["result"]["status"], "succeeded");
    assert_eq!(visible.event_kind, "tool_result");
    assert_eq!(
        visible.payload["result"]["output"]["content"],
        "fn main() {}"
    );
    assert_eq!(
        visible.payload["result"]["artifact_refs"][0],
        artifact_id.as_str()
    );
    assert_eq!(visible.artifact_refs, vec![artifact_id]);
}

#[test]
fn tool_repair_report_event_records_provider_neutral_summary_without_raw_reasoning() {
    let report = ToolRepairReport {
        repair_id: ToolRepairId::from_static("tool_repair_scavenge"),
        call_id: Some(ToolCallId::from_static("tool_call_scavenged")),
        tool_id: Some(ToolId::from_static("tool_workspace_read")),
        kind: ToolRepairKind::ScavengedJson,
        reason: "scavenged_tool_call_json_from_provider_text".to_string(),
        original_call_count: Some(2),
        repaired_call_count: Some(1),
        truncated_bytes: None,
        metadata: None,
    };
    let record = EventFrame::new(
        "trace_tool_repair",
        1,
        RunEvent::ToolRepairReported {
            report: report.clone(),
        },
    )
    .to_trace_record();

    assert_eq!(record.event_kind, "tool_repair_reported");
    assert_eq!(record.payload["report"]["kind"], "scavenged_json");
    assert_eq!(record.payload["report"]["original_call_count"], 2);
    assert_eq!(record.payload["report"]["repaired_call_count"], 1);
    assert_eq!(
        record.payload["report"]["reason"],
        "scavenged_tool_call_json_from_provider_text"
    );
    assert!(record.payload.get("raw_reasoning").is_none());
    assert!(record.payload.get("provider_reasoning").is_none());
    assert!(record.payload.get("provider_raw").is_none());
    assert!(record.payload.get("raw_text").is_none());
}

#[test]
fn snapshot_created_event_records_checkpoint_without_restore_action() {
    let snapshot_id = SnapshotId::from_static("snapshot_workspace_before_edit");
    let task_id = TaskId::from_static("task_checkpoint");
    let record = EventFrame::new(
        "trace_snapshot",
        1,
        RunEvent::SnapshotCreated {
            checkpoint: WorkspaceCheckpoint {
                id: snapshot_id.clone(),
                kind: SnapshotKind::SideGit,
                storage_uri: "tessera://snapshots/snapshot_workspace_before_edit".to_string(),
                workspace_root: Some("/workspace/project".to_string()),
                parent_snapshot_id: None,
                summary: Some("before file edit".to_string()),
                metadata: None,
            },
        },
    )
    .with_task_id(task_id)
    .to_trace_record();

    assert_eq!(record.event_kind, "snapshot_created");
    assert_eq!(record.payload["checkpoint"]["id"], snapshot_id.as_str());
    assert_eq!(record.payload["checkpoint"]["kind"], "side_git");
    assert_eq!(
        record.payload["checkpoint"]["storage_uri"],
        "tessera://snapshots/snapshot_workspace_before_edit"
    );
    assert!(record.payload.get("restore_command").is_none());
    assert!(record.payload.get("revert_command").is_none());
}

#[test]
fn context_reference_schema_preserves_source_placement_and_budget_without_content() {
    let reference = ContextReference {
        id: ContextId::from_static("context_architecture_doc"),
        source: ContextSource {
            kind: ContextSourceKind::File,
            uri: Some("docs/technical-architecture.md".to_string()),
            label: Some("technical architecture".to_string()),
        },
        placement: ContextPlacement::StablePrefix,
        estimated_tokens: 1_200,
        pinned: true,
        summary: Some("runtime architecture contract".to_string()),
        metadata: None,
    };
    let budget = ContextBudget {
        max_tokens: 8_000,
        reserved_output_tokens: 1_000,
    };

    let reference_value = serde_json::to_value(&reference).unwrap();
    let budget_value = serde_json::to_value(budget).unwrap();

    assert_eq!(reference_value["source"]["kind"], "file");
    assert_eq!(
        reference_value["source"]["uri"],
        "docs/technical-architecture.md"
    );
    assert_eq!(reference_value["placement"], "stable_prefix");
    assert_eq!(reference_value["estimated_tokens"], 1_200);
    assert_eq!(budget_value["reserved_output_tokens"], 1_000);
    assert!(reference_value.get("content").is_none());
    assert!(reference_value.get("bytes").is_none());
}
