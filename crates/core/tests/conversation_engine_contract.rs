use async_trait::async_trait;
use futures::stream;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tessera_core::{
    AgentRegistry, ContextWorkbench, ConversationEngine, ConversationRequest, CoreError,
    DiagnosticsReporter, EventSinkAction, McpToolAdapter, McpToolAnnotations, McpToolSpec,
    ModelRouteRequest, ModelRouter, NoProgressDetector, NoProgressObservation,
    OrderedToolResultBuffer, OsSandboxPlanner, PolicyGate, ReplayRunner, RunCancellationToken,
    RunControls, RuntimeEventQuery, RuntimeHttpApi, RuntimeHttpEventRequest, RuntimeReader,
    SkillRegistry, ToolRegistry, ToolRepairTelemetry, WorkspaceCheckpointPlanner,
    WorkspaceGuardrailChecker,
};
use tessera_protocol::{
    AgentProfile, AgentProfileId, ArtifactId, ArtifactKind, ContextBudget, ContextId,
    ContextPlacement, ContextReference, ContextSource, ContextSourceKind, Diagnostic,
    DiagnosticRange, DiagnosticSeverity, ErrorSource, EventFrame, ItemId, ModelProfileId,
    NoProgressAction, NoProgressSignalKind, NormalizedError, OsSandboxFilesystem, OsSandboxMode,
    OsSandboxNetwork, OsSandboxShell, PolicyOutcome, ProviderCapability, ProviderId, RouteStrategy,
    RunEvent, SandboxDecisionKind, SkillEntrypoint, SkillEntrypointFormat, SkillId, SkillManifest,
    SkillPolicy, SkillRequirements, SkillSource, SkillSourceKind, SnapshotId, SnapshotKind, TaskId,
    ToolCallId, ToolCallRequest, ToolDescriptor, ToolDispatch, ToolDispatchId, ToolId,
    ToolPermission, ToolRepairKind, ToolResult, ToolResultId, ToolResultStatus, ToolSideEffect,
    WorkspaceCheckpoint, WorkspaceScope,
};
use tessera_providers::{
    mock::MockProvider, ChatProvider, ProviderError, ProviderEventStream, ProviderMessage,
    ProviderMessageRole, ProviderRequest,
};
use tessera_storage::TraceStore;

#[derive(Clone, Debug)]
struct HangingProvider;

#[async_trait]
impl ChatProvider for HangingProvider {
    async fn capability(&self) -> tessera_providers::Result<ProviderCapability> {
        Ok(ProviderCapability {
            provider_id: ProviderId::from_static("hanging"),
            supports_streaming: true,
            supports_reasoning_delta: false,
            supports_cache_telemetry: false,
            supports_cost_estimate: false,
            supports_tool_calling: false,
            max_context_tokens: Some(1024),
            extension: None,
        })
    }

    async fn stream_chat(
        &self,
        _request: ProviderRequest,
    ) -> tessera_providers::Result<ProviderEventStream> {
        Ok(Box::pin(stream::pending()))
    }
}

#[derive(Clone, Debug)]
struct FailingProvider;

#[async_trait]
impl ChatProvider for FailingProvider {
    async fn capability(&self) -> tessera_providers::Result<ProviderCapability> {
        Ok(ProviderCapability {
            provider_id: ProviderId::from_static("failing"),
            supports_streaming: true,
            supports_reasoning_delta: false,
            supports_cache_telemetry: false,
            supports_cost_estimate: false,
            supports_tool_calling: false,
            max_context_tokens: Some(1024),
            extension: None,
        })
    }

    async fn stream_chat(
        &self,
        _request: ProviderRequest,
    ) -> tessera_providers::Result<ProviderEventStream> {
        Err(ProviderError::Normalized(NormalizedError {
            code: "provider_rate_limited".to_string(),
            message: "provider rate limit reached".to_string(),
            retryable: true,
            source: ErrorSource::Provider,
            details: None,
        }))
    }
}

#[derive(Clone, Debug)]
struct EmptyAssistantProvider;

#[async_trait]
impl ChatProvider for EmptyAssistantProvider {
    async fn capability(&self) -> tessera_providers::Result<ProviderCapability> {
        Ok(ProviderCapability {
            provider_id: ProviderId::from_static("empty"),
            supports_streaming: true,
            supports_reasoning_delta: false,
            supports_cache_telemetry: false,
            supports_cost_estimate: false,
            supports_tool_calling: false,
            max_context_tokens: Some(1024),
            extension: None,
        })
    }

    async fn stream_chat(
        &self,
        request: ProviderRequest,
    ) -> tessera_providers::Result<ProviderEventStream> {
        let item_id = request.assistant_item_id;
        Ok(Box::pin(stream::iter(vec![
            Ok(RunEvent::AssistantMessageStarted {
                item_id: item_id.clone(),
            }),
            Ok(RunEvent::AssistantMessageCompleted { item_id }),
        ])))
    }
}

#[derive(Clone, Debug)]
struct CapturingProvider {
    captured_request: Arc<Mutex<Option<ProviderRequest>>>,
}

#[async_trait]
impl ChatProvider for CapturingProvider {
    async fn capability(&self) -> tessera_providers::Result<ProviderCapability> {
        Ok(mock_capability())
    }

    async fn stream_chat(
        &self,
        request: ProviderRequest,
    ) -> tessera_providers::Result<ProviderEventStream> {
        let item_id = request.assistant_item_id.clone();
        *self.captured_request.lock().unwrap() = Some(request);
        Ok(Box::pin(stream::iter(vec![
            Ok(RunEvent::AssistantMessageStarted {
                item_id: item_id.clone(),
            }),
            Ok(RunEvent::AssistantDelta {
                item_id: item_id.clone(),
                text: "captured".to_string(),
            }),
            Ok(RunEvent::AssistantMessageCompleted { item_id }),
        ])))
    }
}

fn mock_capability() -> ProviderCapability {
    ProviderCapability {
        provider_id: ProviderId::from_static("mock"),
        supports_streaming: true,
        supports_reasoning_delta: true,
        supports_cache_telemetry: true,
        supports_cost_estimate: true,
        supports_tool_calling: false,
        max_context_tokens: Some(128_000),
        extension: None,
    }
}

#[test]
fn model_router_draft_records_manual_reason_without_auto_routing() {
    let requested_profile = ModelProfileId::from_static("manual-profile");
    let decision = ModelRouter::draft().route(ModelRouteRequest {
        requested_profile: Some(requested_profile.clone()),
        default_profile: ModelProfileId::from_static("default-profile"),
        requested_model: "manual-model".to_string(),
        reasoning_level: Some("standard".to_string()),
        provider_capability: Some(mock_capability()),
    });

    assert_eq!(decision.requested_profile, Some(requested_profile.clone()));
    assert_eq!(decision.selected_profile, requested_profile);
    assert_eq!(decision.selected_model, "manual-model");
    assert_eq!(decision.reasoning_level.as_deref(), Some("standard"));
    assert_eq!(decision.strategy, RouteStrategy::Manual);
    assert_eq!(
        decision.decision_reason.as_deref(),
        Some("manual_profile_selected_auto_routing_disabled")
    );
    assert!(decision.fallback_reason.is_none());
}

#[test]
fn no_progress_detector_prefers_stop_over_route_escalation_for_no_output() {
    let item_id = ItemId::from_static("item_empty_assistant");
    let mut detector = NoProgressDetector::default();

    assert!(detector
        .observe_event(&RunEvent::AssistantMessageStarted {
            item_id: item_id.clone()
        })
        .is_none());
    let signal = detector
        .observe_event(&RunEvent::AssistantMessageCompleted { item_id })
        .unwrap();

    assert_eq!(signal.kind, NoProgressSignalKind::NoOutput);
    assert_eq!(signal.action, NoProgressAction::Stop);
    assert_eq!(signal.reason, "assistant_completed_without_output");
    assert!(!signal.route_escalation_allowed);
}

#[test]
fn no_progress_detector_reports_read_only_and_repair_thresholds_without_route_escalation() {
    let mut detector = NoProgressDetector::default();

    assert!(detector
        .record_observation(NoProgressObservation::ReadOnlyStep)
        .is_none());
    assert!(detector
        .record_observation(NoProgressObservation::ReadOnlyStep)
        .is_none());
    let read_only = detector
        .record_observation(NoProgressObservation::ReadOnlyStep)
        .unwrap();

    assert_eq!(read_only.kind, NoProgressSignalKind::RepeatedReadOnly);
    assert_eq!(read_only.consecutive_count, 3);
    assert_eq!(read_only.threshold, 3);
    assert_eq!(read_only.action, NoProgressAction::AskUser);
    assert!(!read_only.route_escalation_allowed);

    assert!(detector
        .record_observation(NoProgressObservation::AssistantOutput)
        .is_none());
    assert!(detector
        .record_observation(NoProgressObservation::RepairStep)
        .is_none());
    assert!(detector
        .record_observation(NoProgressObservation::RepairStep)
        .is_none());
    let repair = detector
        .record_observation(NoProgressObservation::RepairStep)
        .unwrap();

    assert_eq!(repair.kind, NoProgressSignalKind::RepeatedRepair);
    assert_eq!(repair.consecutive_count, 3);
    assert_eq!(repair.threshold, 3);
    assert_eq!(repair.action, NoProgressAction::Summarize);
    assert!(!repair.route_escalation_allowed);
}

#[test]
fn skill_registry_lists_and_finds_manifests_without_runtime_activation() {
    let manifest = SkillManifest {
        id: SkillId::from_static("skill_code_review"),
        name: "code-review".to_string(),
        version: Some("0.1.0".to_string()),
        description: "Review code changes and produce prioritized findings.".to_string(),
        source: SkillSource {
            kind: SkillSourceKind::BuiltIn,
            uri: Some("builtin://code-review/SKILL.md".to_string()),
        },
        entrypoint: SkillEntrypoint {
            format: SkillEntrypointFormat::SkillMd,
            path: "SKILL.md".to_string(),
        },
        requirements: SkillRequirements {
            tools: vec!["git.diff".to_string()],
            context: vec!["workspace".to_string()],
        },
        policy: SkillPolicy {
            default_permission: "ask".to_string(),
            network: "deny".to_string(),
            write_files: "deny".to_string(),
        },
        metadata: None,
    };
    let registry = SkillRegistry::from_manifests([manifest.clone()]);

    assert_eq!(registry.list_skills(), vec![manifest.clone()]);
    assert_eq!(registry.find_skill(&manifest.id), Some(&manifest));
}

#[test]
fn agent_registry_lists_and_finds_profiles_without_agent_runtime() {
    let profile = AgentProfile {
        id: AgentProfileId::from_static("agent_profile_reviewer"),
        name: "Reviewer".to_string(),
        role: "reviewer".to_string(),
        model_profile: ModelProfileId::from_static("profile_fast"),
        skills: vec![SkillId::from_static("skill_code_review")],
        memory_scopes: vec!["workspace".to_string()],
        context_scopes: vec!["thread".to_string()],
        tool_permissions: vec![ToolPermission::FilesystemRead, ToolPermission::Git],
        max_steps: 8,
        metadata: None,
    };
    let registry = AgentRegistry::from_profiles([profile.clone()]);

    assert_eq!(registry.list_agents(), vec![profile.clone()]);
    assert_eq!(registry.find_agent(&profile.id), Some(&profile));
    assert_eq!(registry.list_agents()[0].max_steps, 8);
    assert_eq!(
        registry.list_agents()[0].tool_permissions,
        vec![ToolPermission::FilesystemRead, ToolPermission::Git]
    );
}

#[test]
fn tool_registry_lists_and_finds_descriptors_without_runtime_execution() {
    let descriptor = ToolDescriptor {
        id: ToolId::from_static("tool_workspace_read"),
        display_name: "Read workspace file".to_string(),
        description: "Read a file from the active workspace.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" }
            }
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "content": { "type": "string" }
            }
        }),
        required_permissions: vec![ToolPermission::FilesystemRead],
        side_effects: vec![ToolSideEffect::ReadOnly],
        parallel_safe: false,
        metadata: None,
    };
    let registry = ToolRegistry::from_descriptors([descriptor.clone()]);

    assert_eq!(registry.list_tools(), vec![descriptor.clone()]);
    assert_eq!(registry.find_tool(&descriptor.id), Some(&descriptor));
    assert!(!registry.list_tools()[0].parallel_safe);
    assert_eq!(
        registry.list_tools()[0].side_effects,
        vec![ToolSideEffect::ReadOnly]
    );
}

#[test]
fn mcp_tool_adapter_maps_metadata_to_tessera_descriptor_and_call_without_runtime() {
    let adapter = McpToolAdapter;
    let read_spec = McpToolSpec {
        server_id: "filesystem".to_string(),
        name: "Read File".to_string(),
        description: Some("Read a workspace file through MCP metadata.".to_string()),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            }
        }),
        output_schema: None,
        annotations: McpToolAnnotations {
            title: Some("Read file".to_string()),
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(false),
        },
    };
    let network_spec = McpToolSpec {
        server_id: "search".to_string(),
        name: "web.search".to_string(),
        description: Some("Search the web through MCP metadata.".to_string()),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: None,
        annotations: McpToolAnnotations {
            title: None,
            read_only_hint: Some(true),
            destructive_hint: Some(false),
            idempotent_hint: Some(true),
            open_world_hint: Some(true),
        },
    };

    let read_descriptor = adapter.descriptor_from_spec(&read_spec);
    let network_descriptor = adapter.descriptor_from_spec(&network_spec);
    let request = adapter
        .request_from_arguments(&read_descriptor, serde_json::json!({ "path": "README.md" }));

    assert_eq!(read_descriptor.id.as_str(), "tool_mcp_filesystem_read_file");
    assert_eq!(read_descriptor.display_name, "Read file");
    assert_eq!(
        read_descriptor.required_permissions,
        Vec::<ToolPermission>::new()
    );
    assert_eq!(read_descriptor.side_effects, vec![ToolSideEffect::ReadOnly]);
    assert!(!read_descriptor.parallel_safe);
    assert_eq!(
        read_descriptor.output_schema,
        serde_json::json!({ "type": "object" })
    );
    assert_eq!(
        read_descriptor.metadata.as_ref().unwrap()["mcp_server_id"],
        "filesystem"
    );
    assert_eq!(
        read_descriptor.metadata.as_ref().unwrap()["mcp_tool_name"],
        "Read File"
    );
    assert!(read_descriptor
        .metadata
        .as_ref()
        .unwrap()
        .get("command")
        .is_none());
    assert!(read_descriptor
        .metadata
        .as_ref()
        .unwrap()
        .get("server_url")
        .is_none());

    assert_eq!(
        network_descriptor.required_permissions,
        vec![ToolPermission::Network]
    );
    assert_eq!(
        network_descriptor.side_effects,
        vec![ToolSideEffect::Network]
    );
    assert_eq!(network_descriptor.id.as_str(), "tool_mcp_search_web_search");

    assert_eq!(request.tool_id, read_descriptor.id);
    assert_eq!(request.input["path"], "README.md");
    assert_eq!(
        request.metadata.as_ref().unwrap()["mcp_tool_name"],
        "Read File"
    );
    assert!(request
        .metadata
        .as_ref()
        .unwrap()
        .get("executable")
        .is_none());
}

#[test]
fn policy_gate_allows_read_only_asks_for_workspace_write_and_denies_shell() {
    let gate = PolicyGate;
    let read_descriptor = tool_descriptor(
        "tool_workspace_read",
        vec![ToolPermission::FilesystemRead],
        vec![ToolSideEffect::ReadOnly],
    );
    let write_descriptor = tool_descriptor(
        "tool_workspace_write",
        vec![ToolPermission::FilesystemWrite],
        vec![ToolSideEffect::WritesWorkspace],
    );
    let shell_descriptor = tool_descriptor(
        "tool_shell",
        vec![ToolPermission::Shell],
        vec![ToolSideEffect::Shell],
    );

    let read = gate.evaluate(
        &read_descriptor,
        &tool_request("call_read", &read_descriptor.id),
    );
    let write = gate.evaluate(
        &write_descriptor,
        &tool_request("call_write", &write_descriptor.id),
    );
    let shell = gate.evaluate(
        &shell_descriptor,
        &tool_request("call_shell", &shell_descriptor.id),
    );

    assert_eq!(read.outcome, PolicyOutcome::Allow);
    assert!(read.approval_id.is_none());
    assert_eq!(read.reason, "read_only_tool_allowed");

    assert_eq!(write.outcome, PolicyOutcome::AskUser);
    assert!(write.approval_id.is_some());
    assert_eq!(write.reason, "side_effect_requires_user_approval");

    assert_eq!(shell.outcome, PolicyOutcome::Deny);
    assert!(shell.approval_id.is_none());
    assert_eq!(shell.reason, "dangerous_tool_denied_until_sandbox_exists");
}

#[test]
fn workspace_guardrail_checker_allows_workspace_read_asks_write_and_denies_outside_or_shell() {
    let checker = WorkspaceGuardrailChecker::new(WorkspaceScope {
        workspace_root: "/workspace/project".to_string(),
        allowed_roots: vec![],
        denied_roots: vec![],
    });
    let read_descriptor = tool_descriptor(
        "tool_workspace_read",
        vec![ToolPermission::FilesystemRead],
        vec![ToolSideEffect::ReadOnly],
    );
    let write_descriptor = tool_descriptor(
        "tool_workspace_write",
        vec![ToolPermission::FilesystemWrite],
        vec![ToolSideEffect::WritesWorkspace],
    );
    let shell_descriptor = tool_descriptor(
        "tool_shell",
        vec![ToolPermission::Shell],
        vec![ToolSideEffect::Shell],
    );

    let read = checker.evaluate_tool_path(
        &read_descriptor,
        &tool_request("call_read", &read_descriptor.id),
        "src/lib.rs",
    );
    let write = checker.evaluate_tool_path(
        &write_descriptor,
        &tool_request("call_write", &write_descriptor.id),
        "README.md",
    );
    let outside = checker.evaluate_tool_path(
        &read_descriptor,
        &tool_request("call_outside", &read_descriptor.id),
        "../secrets.env",
    );
    let shell = checker.evaluate_tool_path(
        &shell_descriptor,
        &tool_request("call_shell", &shell_descriptor.id),
        "scripts/build.sh",
    );

    assert_eq!(read.kind, SandboxDecisionKind::Allow);
    assert_eq!(read.reason, "workspace_read_allowed");
    assert!(read.guardrail.within_workspace);
    assert_eq!(
        read.guardrail.resolved_path.as_deref(),
        Some("/workspace/project/src/lib.rs")
    );

    assert_eq!(write.kind, SandboxDecisionKind::AskUser);
    assert_eq!(write.reason, "workspace_write_requires_approval");
    assert!(write.guardrail.within_workspace);

    assert_eq!(outside.kind, SandboxDecisionKind::Deny);
    assert_eq!(outside.reason, "path_outside_workspace");
    assert!(!outside.guardrail.within_workspace);
    assert_eq!(
        outside.guardrail.resolved_path.as_deref(),
        Some("/workspace/secrets.env")
    );

    assert_eq!(shell.kind, SandboxDecisionKind::Deny);
    assert_eq!(shell.reason, "dangerous_tool_denied_until_sandbox_exists");
}

#[test]
fn os_sandbox_planner_selects_profiles_without_runtime_execution() {
    let planner = OsSandboxPlanner::new("/workspace/project");
    let read_descriptor = tool_descriptor(
        "tool_workspace_read",
        vec![ToolPermission::FilesystemRead],
        vec![ToolSideEffect::ReadOnly],
    );
    let write_descriptor = tool_descriptor(
        "tool_workspace_write",
        vec![ToolPermission::FilesystemWrite],
        vec![ToolSideEffect::WritesWorkspace],
    );
    let network_descriptor = tool_descriptor(
        "tool_network",
        vec![ToolPermission::Network],
        vec![ToolSideEffect::Network],
    );
    let shell_descriptor = tool_descriptor(
        "tool_shell",
        vec![ToolPermission::Shell],
        vec![ToolSideEffect::Shell],
    );

    let read = planner.plan_tool(&read_descriptor);
    let write = planner.plan_tool(&write_descriptor);
    let network = planner.plan_tool(&network_descriptor);
    let shell = planner.plan_tool(&shell_descriptor);

    assert_eq!(read.mode, OsSandboxMode::ReadOnly);
    assert_eq!(read.filesystem, OsSandboxFilesystem::ReadOnly);
    assert_eq!(read.network, OsSandboxNetwork::Disabled);
    assert_eq!(read.shell, OsSandboxShell::Denied);
    assert!(!read.requires_checkpoint);

    assert_eq!(write.mode, OsSandboxMode::WorkspaceWrite);
    assert_eq!(write.filesystem, OsSandboxFilesystem::WorkspaceWrite);
    assert_eq!(write.network, OsSandboxNetwork::Disabled);
    assert_eq!(write.shell, OsSandboxShell::Denied);
    assert!(write.requires_checkpoint);

    assert_eq!(network.mode, OsSandboxMode::NetworkRequired);
    assert_eq!(network.network, OsSandboxNetwork::Requested);
    assert_eq!(network.filesystem, OsSandboxFilesystem::ReadOnly);

    assert_eq!(shell.mode, OsSandboxMode::Denied);
    assert_eq!(shell.shell, OsSandboxShell::Denied);
    assert_eq!(shell.reason, "dangerous_tool_requires_real_os_sandbox");
}

#[test]
fn workspace_checkpoint_planner_builds_checkpoint_metadata_without_file_operations() {
    let sandbox_planner = OsSandboxPlanner::new("/workspace/project");
    let checkpoint_planner =
        WorkspaceCheckpointPlanner::new(SnapshotKind::SideGit, "tessera://snapshots/");
    let read_descriptor = tool_descriptor(
        "tool_workspace_read",
        vec![ToolPermission::FilesystemRead],
        vec![ToolSideEffect::ReadOnly],
    );
    let write_descriptor = tool_descriptor(
        "tool_workspace_write",
        vec![ToolPermission::FilesystemWrite],
        vec![ToolSideEffect::WritesWorkspace],
    );

    let read_profile = sandbox_planner.plan_tool(&read_descriptor);
    let write_profile = sandbox_planner.plan_tool(&write_descriptor);
    let parent_snapshot_id = SnapshotId::from_static("snapshot_parent");

    let checkpoint = checkpoint_planner
        .plan_required_checkpoint(
            &write_profile,
            Some(parent_snapshot_id.clone()),
            "before workspace write",
        )
        .unwrap();

    assert_eq!(checkpoint.kind, SnapshotKind::SideGit);
    assert!(checkpoint
        .storage_uri
        .starts_with("tessera://snapshots/snapshot_"));
    assert_eq!(
        checkpoint.workspace_root.as_deref(),
        Some("/workspace/project")
    );
    assert_eq!(checkpoint.parent_snapshot_id, Some(parent_snapshot_id));
    assert_eq!(
        checkpoint.summary.as_deref(),
        Some("before workspace write")
    );
    assert!(checkpoint.metadata.is_none());
    assert!(checkpoint
        .storage_uri
        .chars()
        .all(|character| !character.is_whitespace()));
    assert!(checkpoint_planner
        .plan_required_checkpoint(&read_profile, None, "before read")
        .is_none());
}

#[test]
fn diagnostics_reporter_builds_event_without_running_lsp_process() {
    let reporter = DiagnosticsReporter;
    let diagnostic = Diagnostic {
        severity: DiagnosticSeverity::Error,
        code: Some("E0308".to_string()),
        message: "mismatched types".to_string(),
        uri: Some("file:///workspace/project/src/lib.rs".to_string()),
        range: Some(DiagnosticRange {
            start_line: 4,
            start_character: 12,
            end_line: 4,
            end_character: 18,
        }),
        metadata: None,
    };

    let report = reporter.report("rustc", [diagnostic.clone()]);
    let event = reporter.report_event(report.clone());

    assert!(report.report_id.as_str().starts_with("diagnostics_"));
    assert_eq!(report.source, "rustc");
    assert_eq!(report.diagnostics, vec![diagnostic]);
    assert!(report.metadata.is_none());
    assert!(matches!(
        event,
        RunEvent::DiagnosticsReported { report: event_report }
            if event_report.report_id == report.report_id
    ));
}

#[test]
fn ordered_tool_result_buffer_releases_results_in_declared_order_after_parallel_completion() {
    let tool_id = ToolId::from_static("tool_workspace_read");
    let dispatch_zero = tool_dispatch(0, "call_zero", &tool_id);
    let dispatch_one = tool_dispatch(1, "call_one", &tool_id);
    let mut buffer =
        OrderedToolResultBuffer::from_dispatches([dispatch_zero.clone(), dispatch_one.clone()]);

    let start_events = buffer.start_events();
    assert_eq!(start_events.len(), 2);
    assert!(matches!(
        &start_events[0],
        RunEvent::ToolDispatchStarted { dispatch } if dispatch.call_id == dispatch_zero.call_id
    ));
    assert!(matches!(
        &start_events[1],
        RunEvent::ToolDispatchStarted { dispatch } if dispatch.call_id == dispatch_one.call_id
    ));

    let held = buffer.record_completion(tool_result(1, "call_one", &tool_id, "second"));
    assert!(held.is_empty());

    let released = buffer.record_completion(tool_result(0, "call_zero", &tool_id, "first"));
    assert_eq!(released.len(), 4);
    assert!(matches!(
        &released[0],
        RunEvent::ToolDispatchCompleted { result } if result.declared_index == 0
    ));
    assert!(matches!(
        &released[1],
        RunEvent::ToolResultRecorded { result } if result.declared_index == 0
    ));
    assert!(matches!(
        &released[2],
        RunEvent::ToolDispatchCompleted { result } if result.declared_index == 1
    ));
    assert!(matches!(
        &released[3],
        RunEvent::ToolResultRecorded { result } if result.declared_index == 1
    ));

    let model_visible_outputs: Vec<&str> = released
        .iter()
        .filter_map(|event| match event {
            RunEvent::ToolResultRecorded { result } => result.output["text"].as_str(),
            _ => None,
        })
        .collect();
    assert_eq!(model_visible_outputs, vec!["first", "second"]);
}

#[test]
fn tool_repair_telemetry_records_summaries_without_provider_raw_text() {
    let telemetry = ToolRepairTelemetry;
    let tool_id = ToolId::from_static("tool_workspace_read");
    let call_id = ToolCallId::from_static("tool_call_scavenged");

    let scavenged = telemetry.scavenged_json(
        Some(call_id.clone()),
        Some(tool_id.clone()),
        2,
        1,
        "scavenged_tool_call_json_from_provider_text",
    );
    let storm = telemetry.call_storm_detected(42, 16, "tool_call_storm_threshold_exceeded");

    assert_eq!(scavenged.call_id, Some(call_id));
    assert_eq!(scavenged.tool_id, Some(tool_id));
    assert_eq!(scavenged.kind, ToolRepairKind::ScavengedJson);
    assert_eq!(scavenged.original_call_count, Some(2));
    assert_eq!(scavenged.repaired_call_count, Some(1));
    assert!(scavenged.metadata.is_none());

    assert_eq!(storm.kind, ToolRepairKind::CallStormDetected);
    assert_eq!(storm.original_call_count, Some(42));
    assert_eq!(storm.repaired_call_count, Some(16));
    assert_eq!(storm.reason, "tool_call_storm_threshold_exceeded");
}

fn tool_descriptor(
    id: &'static str,
    required_permissions: Vec<ToolPermission>,
    side_effects: Vec<ToolSideEffect>,
) -> ToolDescriptor {
    ToolDescriptor {
        id: ToolId::from_static(id),
        display_name: id.to_string(),
        description: "test descriptor".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        required_permissions,
        side_effects,
        parallel_safe: false,
        metadata: None,
    }
}

fn tool_request(id: &'static str, tool_id: &ToolId) -> ToolCallRequest {
    ToolCallRequest {
        call_id: ToolCallId::from_static(id),
        tool_id: tool_id.clone(),
        input: serde_json::json!({}),
        metadata: None,
    }
}

fn tool_dispatch(index: u32, call_id: &'static str, tool_id: &ToolId) -> ToolDispatch {
    ToolDispatch {
        dispatch_id: ToolDispatchId::from_static(match index {
            0 => "tool_dispatch_zero",
            1 => "tool_dispatch_one",
            _ => "tool_dispatch_extra",
        }),
        call_id: ToolCallId::from_static(call_id),
        tool_id: tool_id.clone(),
        declared_index: index,
        parallel_safe: true,
        metadata: None,
    }
}

fn tool_result(
    index: u32,
    call_id: &'static str,
    tool_id: &ToolId,
    text: &'static str,
) -> ToolResult {
    ToolResult {
        result_id: ToolResultId::from_static(match index {
            0 => "tool_result_zero",
            1 => "tool_result_one",
            _ => "tool_result_extra",
        }),
        call_id: ToolCallId::from_static(call_id),
        tool_id: tool_id.clone(),
        declared_index: index,
        status: ToolResultStatus::Succeeded,
        output: serde_json::json!({ "text": text }),
        error: None,
        artifact_refs: vec![],
        metadata: None,
    }
}

#[test]
fn runtime_reader_lists_snapshot_checkpoints_from_trace_without_restoring() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();
    let snapshot_id = SnapshotId::from_static("snapshot_runtime");
    let task_id = TaskId::from_static("task_snapshot_runtime");

    store
        .append(
            &EventFrame::new(
                "trace_snapshot_runtime",
                1,
                RunEvent::SnapshotCreated {
                    checkpoint: WorkspaceCheckpoint {
                        id: snapshot_id.clone(),
                        kind: SnapshotKind::SideGit,
                        storage_uri: "tessera://snapshots/snapshot_runtime".to_string(),
                        workspace_root: Some("/workspace/project".to_string()),
                        parent_snapshot_id: None,
                        summary: Some("before patch".to_string()),
                        metadata: None,
                    },
                },
            )
            .with_task_id(task_id.clone()),
        )
        .unwrap();

    let reader = RuntimeReader::new(store);
    let snapshots = reader.list_snapshots("trace_snapshot_runtime").unwrap();

    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].snapshot_id, snapshot_id);
    assert_eq!(snapshots[0].kind, Some(SnapshotKind::SideGit));
    assert_eq!(snapshots[0].task_id, Some(task_id));
    assert_eq!(
        snapshots[0].storage_uri.as_deref(),
        Some("tessera://snapshots/snapshot_runtime")
    );
    assert_eq!(snapshots[0].summary.as_deref(), Some("before patch"));
}

#[test]
fn context_workbench_tracks_references_and_token_budget_without_loading_sources() {
    let mut workbench = ContextWorkbench::new(ContextBudget {
        max_tokens: 200,
        reserved_output_tokens: 40,
    });
    let scratch_id = ContextId::from_static("context_scratch");

    workbench.add_reference(ContextReference {
        id: ContextId::from_static("context_architecture"),
        source: ContextSource {
            kind: ContextSourceKind::File,
            uri: Some("docs/technical-architecture.md".to_string()),
            label: Some("architecture".to_string()),
        },
        placement: ContextPlacement::StablePrefix,
        estimated_tokens: 100,
        pinned: true,
        summary: Some("architecture contract".to_string()),
        metadata: None,
    });
    workbench.add_reference(ContextReference {
        id: ContextId::from_static("context_transcript"),
        source: ContextSource {
            kind: ContextSourceKind::Trace,
            uri: Some("trace://trace_mock".to_string()),
            label: Some("transcript".to_string()),
        },
        placement: ContextPlacement::AppendOnlyTranscript,
        estimated_tokens: 50,
        pinned: false,
        summary: None,
        metadata: None,
    });
    workbench.add_reference(ContextReference {
        id: scratch_id.clone(),
        source: ContextSource {
            kind: ContextSourceKind::Inline,
            uri: None,
            label: Some("scratch".to_string()),
        },
        placement: ContextPlacement::VolatileScratch,
        estimated_tokens: 25,
        pinned: false,
        summary: None,
        metadata: None,
    });

    let summary = workbench.summary();
    assert_eq!(summary.available_tokens, 160);
    assert_eq!(summary.used_tokens, 175);
    assert_eq!(summary.stable_prefix_tokens, 100);
    assert_eq!(summary.append_only_transcript_tokens, 50);
    assert_eq!(summary.volatile_scratch_tokens, 25);
    assert!(summary.over_budget);

    let removed = workbench.remove_reference(&scratch_id).unwrap();
    assert_eq!(removed.id, scratch_id);
    let summary = workbench.summary();
    assert_eq!(workbench.list_references().len(), 2);
    assert_eq!(summary.used_tokens, 150);
    assert_eq!(summary.remaining_tokens, 10);
    assert!(!summary.over_budget);
}

#[test]
fn context_workbench_projects_handles_without_loading_sources() {
    let workbench = ContextWorkbench::from_references(
        ContextBudget {
            max_tokens: 200,
            reserved_output_tokens: 40,
        },
        [
            ContextReference {
                id: ContextId::from_static("context_architecture"),
                source: ContextSource {
                    kind: ContextSourceKind::File,
                    uri: Some("docs/technical-architecture.md".to_string()),
                    label: Some("architecture".to_string()),
                },
                placement: ContextPlacement::StablePrefix,
                estimated_tokens: 100,
                pinned: true,
                summary: Some("architecture contract".to_string()),
                metadata: None,
            },
            ContextReference {
                id: ContextId::from_static("context_trace"),
                source: ContextSource {
                    kind: ContextSourceKind::Trace,
                    uri: Some("trace://trace_mock".to_string()),
                    label: Some("transcript".to_string()),
                },
                placement: ContextPlacement::AppendOnlyTranscript,
                estimated_tokens: 50,
                pinned: false,
                summary: None,
                metadata: None,
            },
        ],
    );

    let projection = workbench.projection();

    assert_eq!(projection.references.len(), 2);
    assert_eq!(projection.summary.used_tokens, 150);
    assert_eq!(projection.summary.available_tokens, 160);
    assert_eq!(
        projection.references[0].source.label.as_deref(),
        Some("architecture")
    );
}

#[tokio::test]
async fn conversation_engine_drives_mock_provider_and_persists_trace() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello from core"))
        .await
        .unwrap();

    assert!(outcome.assistant_text.contains("mock response"));
    assert_eq!(outcome.trace_id, "trace_mock");

    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert_eq!(events.first().map(String::as_str), Some("task_created"));
    assert!(events.contains(&"provider_capability_reported".to_string()));
    assert!(events.contains(&"route_decision_recorded".to_string()));
    assert!(events.contains(&"assistant_reasoning_delta".to_string()));
    assert!(events.contains(&"usage_reported".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));

    let records = outcome.store.read_trace_records(&outcome.trace_id).unwrap();
    let user_message = records
        .iter()
        .find(|record| record.event_kind == "user_message_recorded")
        .unwrap();
    assert_eq!(user_message.payload["text"], "hello from core");

    let route_record = records
        .iter()
        .find(|record| record.event_kind == "route_decision_recorded")
        .unwrap();
    assert_eq!(route_record.payload["decision"]["strategy"], "manual");
    assert_eq!(
        route_record.payload["decision"]["decision_reason"],
        "manual_profile_selected_auto_routing_disabled"
    );
    assert!(route_record.payload["decision"]["fallback_reason"].is_null());
}

#[tokio::test]
async fn conversation_engine_passes_history_to_provider_without_retracing_it() {
    let temp = tempfile::tempdir().unwrap();
    let captured_request = Arc::new(Mutex::new(None));
    let provider = CapturingProvider {
        captured_request: captured_request.clone(),
    };
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(provider, store);
    let mut request = ConversationRequest::mock("current question");
    request.history = vec![
        ProviderMessage::user("prior question"),
        ProviderMessage::assistant("prior answer"),
    ];

    let outcome = engine.run_chat(request).await.unwrap();
    let captured = captured_request.lock().unwrap().clone().unwrap();

    assert_eq!(captured.messages.len(), 3);
    assert_eq!(captured.messages[0].role, ProviderMessageRole::User);
    assert_eq!(captured.messages[0].content, "prior question");
    assert_eq!(captured.messages[1].role, ProviderMessageRole::Assistant);
    assert_eq!(captured.messages[1].content, "prior answer");
    assert_eq!(captured.messages[2].role, ProviderMessageRole::User);
    assert_eq!(captured.messages[2].content, "current question");

    let user_records = outcome
        .store
        .read_trace_records(&outcome.trace_id)
        .unwrap()
        .into_iter()
        .filter(|record| record.event_kind == "user_message_recorded")
        .collect::<Vec<_>>();

    assert_eq!(user_records.len(), 1);
    assert_eq!(user_records[0].payload["text"], "current question");
}

#[tokio::test]
async fn conversation_engine_records_normalized_provider_errors_before_returning_failure() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(FailingProvider, store);
    let mut request = ConversationRequest::mock("hello failure");
    request.trace_id = "trace_provider_failure".to_string();
    request.provider_id = ProviderId::from_static("failing");
    request.profile_id = ModelProfileId::from_static("failing-default");
    request.model = "failing-chat".to_string();

    let result = engine.run_chat(request).await;

    assert!(matches!(result, Err(CoreError::Provider(_))));
    let store = TraceStore::open(temp.path()).unwrap();
    let records = store.read_trace_records("trace_provider_failure").unwrap();
    let event_kinds = records
        .iter()
        .map(|record| record.event_kind.as_str())
        .collect::<Vec<_>>();

    assert!(event_kinds.contains(&"error"));
    assert!(event_kinds.contains(&"task_failed"));
    assert_eq!(event_kinds.last(), Some(&"done"));

    let error_record = records
        .iter()
        .find(|record| record.event_kind == "error")
        .unwrap();
    assert_eq!(
        error_record.payload["error"]["code"],
        "provider_rate_limited"
    );
    assert_eq!(error_record.payload["error"]["retryable"], true);
}

#[tokio::test]
async fn conversation_engine_streams_event_frames_to_live_sink_while_persisting_trace() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);
    let mut live_events = Vec::new();

    let outcome = engine
        .run_chat_with_event_sink(ConversationRequest::mock("hello live"), |frame| {
            live_events.push(frame.clone());
        })
        .await
        .unwrap();

    assert!(live_events
        .iter()
        .any(|frame| matches!(frame.event, RunEvent::AssistantDelta { .. })));
    assert_eq!(live_events.last().unwrap().event.kind(), "done");

    let persisted_events = outcome.store.list_events(&outcome.trace_id).unwrap();
    let live_event_kinds = live_events
        .iter()
        .map(|frame| frame.event.kind().to_string())
        .collect::<Vec<_>>();
    assert_eq!(live_event_kinds, persisted_events);
}

#[tokio::test]
async fn conversation_engine_records_cancellation_when_live_sink_requests_stop() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);
    let mut live_events = Vec::new();

    let outcome = engine
        .run_chat_with_event_sink(ConversationRequest::mock("hello cancel"), |frame| {
            live_events.push(frame.clone());
            match frame.event {
                RunEvent::AssistantMessageStarted { .. } => {
                    EventSinkAction::Cancel("live client stopped".to_string())
                }
                _ => EventSinkAction::Continue,
            }
        })
        .await
        .unwrap();

    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));
}

#[tokio::test]
async fn conversation_engine_records_timeout_when_provider_stalls() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(HangingProvider, store);
    let request = ConversationRequest {
        trace_id: "trace_timeout".to_string(),
        provider_id: ProviderId::from_static("hanging"),
        profile_id: ModelProfileId::from_static("hanging"),
        model: "hanging-model".to_string(),
        prompt: "hello timeout".to_string(),
        history: Vec::new(),
    };

    let outcome = engine
        .run_chat_with_controls_and_event_sink(
            request,
            RunControls {
                event_timeout: Some(Duration::from_millis(5)),
                cancellation_token: None,
            },
            |_| {},
        )
        .await
        .unwrap();

    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));
}

#[tokio::test]
async fn conversation_engine_cancellation_token_interrupts_stalled_provider_stream() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(HangingProvider, store);
    let cancellation_token = RunCancellationToken::new();
    let cancel_from_task = cancellation_token.clone();
    let request = ConversationRequest {
        trace_id: "trace_external_cancel".to_string(),
        provider_id: ProviderId::from_static("hanging"),
        profile_id: ModelProfileId::from_static("hanging"),
        model: "hanging-model".to_string(),
        prompt: "hello external cancel".to_string(),
        history: Vec::new(),
    };

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(5)).await;
        cancel_from_task.cancel("external cancel requested");
    });

    let outcome = engine
        .run_chat_with_controls_and_event_sink(
            request,
            RunControls {
                event_timeout: None,
                cancellation_token: Some(cancellation_token),
            },
            |_| {},
        )
        .await
        .unwrap();

    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert!(events.contains(&"provider_request_started".to_string()));
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));

    let records = outcome.store.read_trace_records(&outcome.trace_id).unwrap();
    let cancel_reason = records
        .iter()
        .find(|record| record.event_kind == "task_cancelled")
        .and_then(|record| record.payload.get("reason"))
        .and_then(|reason| reason.as_str());
    assert_eq!(cancel_reason, Some("external cancel requested"));
}

#[tokio::test]
async fn conversation_engine_records_no_progress_and_stops_when_provider_finishes_without_output() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(EmptyAssistantProvider, store);
    let request = ConversationRequest {
        trace_id: "trace_no_progress".to_string(),
        provider_id: ProviderId::from_static("empty"),
        profile_id: ModelProfileId::from_static("empty-default"),
        model: "empty-chat".to_string(),
        prompt: "hello empty".to_string(),
        history: Vec::new(),
    };

    let outcome = engine.run_chat(request).await.unwrap();

    assert_eq!(outcome.assistant_text, "");
    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert!(events.contains(&"no_progress_loop_detected".to_string()));
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));

    let records = outcome.store.read_trace_records(&outcome.trace_id).unwrap();
    let no_progress = records
        .iter()
        .find(|record| record.event_kind == "no_progress_loop_detected")
        .unwrap();
    assert_eq!(no_progress.payload["signal"]["kind"], "no_output");
    assert_eq!(no_progress.payload["signal"]["action"], "stop");
    assert_eq!(
        no_progress.payload["signal"]["route_escalation_allowed"],
        false
    );
}

#[tokio::test]
async fn replay_runner_reconstructs_mock_assistant_text_from_trace() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello replay"))
        .await
        .unwrap();

    let replay = ReplayRunner::new(&outcome.store)
        .replay(&outcome.trace_id)
        .unwrap();

    assert!(replay.assistant_text.contains("mock response"));
    assert!(replay.event_kinds.contains(&"assistant_delta".to_string()));
    assert!(replay.event_kinds.contains(&"usage_reported".to_string()));
}

#[test]
fn replay_runner_accepts_golden_trace_fixture() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("traces")).unwrap();
    std::fs::write(
        temp.path().join("traces/trace_golden.jsonl"),
        include_str!("fixtures/mock_trace.jsonl"),
    )
    .unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();

    store.rebuild_index("trace_golden").unwrap();
    let replay = ReplayRunner::new(&store).replay("trace_golden").unwrap();
    let events = store.list_events("trace_golden").unwrap();

    assert_eq!(replay.assistant_text, "golden hello");
    assert_eq!(events, vec!["assistant_delta", "usage_reported", "done"]);
}

#[tokio::test]
async fn runtime_reader_pages_trace_events_without_mutating_runtime_state() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello runtime api"))
        .await
        .unwrap();
    let original_events = outcome.store.list_events(&outcome.trace_id).unwrap();

    let reader = RuntimeReader::new(outcome.store);
    let page = reader
        .list_events(
            RuntimeEventQuery::new(&outcome.trace_id)
                .since_seq(5)
                .limit(3),
        )
        .unwrap();

    assert_eq!(page.trace_id, outcome.trace_id);
    assert_eq!(page.records.len(), 3);
    assert!(page.records.iter().all(|record| record.seq > 5));
    assert_eq!(
        page.next_since_seq,
        page.records.last().map(|record| record.seq)
    );

    let reopened = TraceStore::open(temp.path()).unwrap();
    assert_eq!(
        reopened.list_events(&outcome.trace_id).unwrap(),
        original_events
    );
}

#[tokio::test]
async fn runtime_http_api_pages_events_and_encodes_sse_without_owning_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello runtime http api"))
        .await
        .unwrap();
    let api = RuntimeHttpApi::new(RuntimeReader::new(outcome.store));
    let request = RuntimeHttpEventRequest::new(&outcome.trace_id).limit(2);

    let page = api.list_events(request.clone()).unwrap();
    let page_json = api.list_events_json(request.clone()).unwrap();
    let frames = api.sse_event_frames(request).unwrap();

    assert_eq!(page.trace_id, outcome.trace_id);
    assert_eq!(page.records.len(), 2);
    assert_eq!(page_json["trace_id"], outcome.trace_id);
    assert_eq!(page_json["records"].as_array().unwrap().len(), 2);
    assert_eq!(frames.len(), 2);

    let first_encoded = frames[0].encode();
    assert!(first_encoded.starts_with("id: "));
    assert!(first_encoded.contains("\nevent: "));
    assert!(first_encoded.contains("\ndata: {"));
    assert!(first_encoded.ends_with("\n\n"));
    assert!(!first_encoded.contains("command"));
    assert!(!first_encoded.contains("authorization"));
}

#[tokio::test]
async fn runtime_reader_exposes_indexed_thread_and_task_ids_through_core() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello runtime objects"))
        .await
        .unwrap();

    let reader = RuntimeReader::new(outcome.store);
    let objects = reader.list_objects(&outcome.trace_id).unwrap();

    assert_eq!(objects.threads.len(), 1);
    assert_eq!(objects.turns.len(), 1);
    assert!(!objects.items.is_empty());
    assert_eq!(objects.tasks.len(), 1);
    assert!(objects.artifacts.is_empty());
}

#[tokio::test]
async fn runtime_reader_lists_session_summaries_from_trace_files() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);
    let mut first_request = ConversationRequest::mock("hello first session");
    first_request.trace_id = "trace_session_first".to_string();

    let first = engine.run_chat(first_request).await.unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), first.store);
    let mut second_request = ConversationRequest::mock("hello second session");
    second_request.trace_id = "trace_session_second".to_string();

    let second = engine.run_chat(second_request).await.unwrap();
    let reader = RuntimeReader::new(second.store);
    let sessions = reader.list_sessions().unwrap();

    let first_summary = sessions
        .iter()
        .find(|session| session.trace_id == "trace_session_first")
        .unwrap();
    let second_summary = sessions
        .iter()
        .find(|session| session.trace_id == "trace_session_second")
        .unwrap();

    assert_eq!(sessions.len(), 2);
    assert!(first_summary.event_count > 0);
    assert!(first_summary.last_seq >= first_summary.event_count as u64);
    assert_eq!(first_summary.last_event_kind.as_deref(), Some("done"));
    assert!(first_summary.user_preview.contains("hello first session"));
    assert!(second_summary
        .assistant_preview
        .contains("mock response to: hello second session"));
}

#[tokio::test]
async fn runtime_reader_lists_task_registry_from_trace() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello task registry"))
        .await
        .unwrap();

    let reader = RuntimeReader::new(outcome.store);
    let tasks = reader.list_tasks(&outcome.trace_id).unwrap();

    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].kind, Some(tessera_protocol::TaskKind::Chat));
    assert_eq!(tasks[0].status, tessera_protocol::TaskStatus::Completed);
    assert!(tasks[0].created_at.is_some());
    assert!(tasks[0].started_at.is_some());
    assert!(tasks[0].finished_at.is_some());
    assert!(tasks[0].error_code.is_none());
    assert!(tasks[0].cancel_reason.is_none());
}

#[test]
fn runtime_reader_lists_artifact_handles_from_trace() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();
    let artifact_id = ArtifactId::from_static("artifact_runtime");
    let task_id = TaskId::from_static("task_artifact_runtime");
    let item_id = ItemId::from_static("item_artifact_runtime");

    store
        .append(
            &EventFrame::new(
                "trace_artifact_runtime",
                1,
                RunEvent::AssistantDelta {
                    item_id: item_id.clone(),
                    text: "runtime artifact ref".to_string(),
                },
            )
            .with_task_id(task_id.clone())
            .with_item_id(item_id.clone())
            .with_artifact_ref(artifact_id.clone()),
        )
        .unwrap();
    store
        .append(
            &EventFrame::new(
                "trace_artifact_runtime",
                2,
                RunEvent::ArtifactCreated {
                    artifact_id: artifact_id.clone(),
                    kind: ArtifactKind::Patch,
                },
            )
            .with_task_id(task_id.clone()),
        )
        .unwrap();

    let reader = RuntimeReader::new(store);
    let artifacts = reader.list_artifacts("trace_artifact_runtime").unwrap();

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].artifact_id, artifact_id);
    assert_eq!(artifacts[0].kind, Some(ArtifactKind::Patch));
    assert_eq!(artifacts[0].task_id, Some(task_id));
    assert_eq!(artifacts[0].item_id, Some(item_id));
    assert!(artifacts[0].created_at.is_some());
    assert_eq!(
        artifacts[0].referenced_by_event_kinds,
        vec!["assistant_delta"]
    );
}
