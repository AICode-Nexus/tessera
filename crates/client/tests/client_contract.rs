use tessera_client::{
    ClientApprovalStatus, ClientContextBudgetSummary, ClientContextPlacement,
    ClientContextSourceKind, ClientIntent, ClientMemoryProposalStatus, ClientMessageRole,
    ClientProjection, ClientSnapshot, ClientStatus,
};
use tessera_protocol::{
    ApprovalId, ApprovalStatus, ArtifactId, ArtifactKind, ContextId, ContextPlacement,
    ContextReference, ContextSource, ContextSourceKind, CostEstimate, ErrorSource, EventFrame,
    ItemId, MemoryProposal, MemoryProposalId, MemoryProposalStatus, NormalizedError,
    PolicyDecisionId, PolicyOutcome, ProviderCapability, ProviderId, RunEvent, TaskId, TaskKind,
    TaskStatus, ToolApproval, ToolCallId, ToolId, ToolPermission, ToolPolicyDecision,
    ToolSideEffect,
};

#[test]
fn client_projection_turns_core_events_into_ui_neutral_messages() {
    let mut projection = ClientProjection::new("mock-default");
    let user_item_id = ItemId::from_static("item_user");
    let assistant_item_id = ItemId::from_static("item_assistant");

    projection.apply_event(&EventFrame::new(
        "trace_client",
        1,
        RunEvent::UserMessageRecorded {
            item_id: user_item_id.clone(),
            text: "hello client".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        2,
        RunEvent::AssistantMessageStarted {
            item_id: assistant_item_id.clone(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        3,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "shared ".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        4,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "projection".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        5,
        RunEvent::AssistantMessageCompleted {
            item_id: assistant_item_id,
        },
    ));

    assert_eq!(projection.messages[0].role, ClientMessageRole::User);
    assert_eq!(projection.messages[0].content, "hello client");
    assert_eq!(projection.messages[1].role, ClientMessageRole::Assistant);
    assert_eq!(projection.messages[1].content, "shared projection");
    assert!(!projection.messages[1].streaming);
}

#[test]
fn client_snapshot_keeps_status_intents_and_projection_toolkit_neutral() {
    let mut snapshot = ClientSnapshot::with_profiles("mock-default", ["mock-default", "offline"]);

    assert_eq!(
        snapshot.cycle_profile(1),
        Some(ClientIntent::SwitchProfile {
            profile_id: "offline".to_string(),
        })
    );
    assert_eq!(snapshot.status.active_profile, "offline");
    assert_eq!(snapshot.status.active_profile_position(), (2, 2));

    snapshot.draft_input = "hello gui".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::SubmitPrompt {
            profile_id: "offline".to_string(),
            prompt: "hello gui".to_string(),
        })
    );
    assert_eq!(snapshot.draft_input, "");

    let status = ClientStatus::with_profiles("mock-default", ["mock-default"]);
    assert_eq!(status.active_profile_position(), (1, 1));
}

#[test]
fn client_snapshot_projects_context_handles_and_summary() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.set_context_handles(
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
        ClientContextBudgetSummary {
            max_tokens: 200,
            reserved_output_tokens: 40,
            available_tokens: 160,
            used_tokens: 150,
            remaining_tokens: 10,
            stable_prefix_tokens: 100,
            append_only_transcript_tokens: 50,
            volatile_scratch_tokens: 0,
            over_budget: false,
        },
    );

    assert_eq!(snapshot.context_handles.len(), 2);
    assert_eq!(
        snapshot.context_handles[0].context_id,
        ContextId::from_static("context_architecture")
    );
    assert_eq!(
        snapshot.context_handles[0].source_kind,
        ClientContextSourceKind::File
    );
    assert_eq!(
        snapshot.context_handles[0].source_uri.as_deref(),
        Some("docs/technical-architecture.md")
    );
    assert_eq!(
        snapshot.context_handles[0].label.as_deref(),
        Some("architecture")
    );
    assert_eq!(
        snapshot.context_handles[0].placement,
        ClientContextPlacement::StablePrefix
    );
    assert_eq!(snapshot.context_handles[0].estimated_tokens, 100);
    assert!(snapshot.context_handles[0].pinned);
    assert_eq!(
        snapshot.context_handles[0].summary.as_deref(),
        Some("architecture contract")
    );
    assert_eq!(
        snapshot.status.context_handles_summary,
        "context 2 handles / 150/160 tokens"
    );
    assert!(!snapshot
        .status
        .context_handles_summary
        .contains("over budget"));

    snapshot.start_new_thread();

    assert!(snapshot.context_handles.is_empty());
    assert_eq!(
        snapshot.status.context_handles_summary,
        "context 0 handles / 0/0 tokens"
    );
}

#[test]
fn client_snapshot_maps_slash_commands_to_ui_neutral_intents() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.draft_input = " /new ".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::NewThread));

    snapshot.draft_input = "/save".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::SaveThread));

    snapshot.draft_input = "/export".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::ExportThread));

    snapshot.draft_input = "/cancel".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::CancelTask { task_id: None })
    );

    snapshot.draft_input = "/approve approval_write_readme".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::ApproveToolCall {
            approval_id: ApprovalId::from_static("approval_write_readme")
        })
    );

    snapshot.draft_input = "/deny approval_write_readme".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::DenyToolCall {
            approval_id: ApprovalId::from_static("approval_write_readme")
        })
    );

    snapshot.draft_input = "/remember memory_proposal_prefers_rust".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::AcceptMemoryProposal {
            proposal_id: MemoryProposalId::from_static("memory_proposal_prefers_rust")
        })
    );

    snapshot.draft_input = "/forget memory_proposal_prefers_rust".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::RejectMemoryProposal {
            proposal_id: MemoryProposalId::from_static("memory_proposal_prefers_rust")
        })
    );

    snapshot.draft_input = "/explain this command".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::SubmitPrompt {
            profile_id: "mock-default".to_string(),
            prompt: "/explain this command".to_string(),
        })
    );
}

#[test]
fn client_snapshot_cancel_command_targets_latest_running_task() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let first_task_id = TaskId::from_static("task_old_completed");
    let running_task_id = TaskId::from_static("task_running_cancel");

    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        1,
        RunEvent::TaskCreated {
            task_id: first_task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        2,
        RunEvent::TaskStarted {
            task_id: first_task_id.clone(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        3,
        RunEvent::TaskCompleted {
            task_id: first_task_id,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        4,
        RunEvent::TaskCreated {
            task_id: running_task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_cancel_intent",
        5,
        RunEvent::TaskStarted {
            task_id: running_task_id.clone(),
        },
    ));

    assert_eq!(
        snapshot.active_cancellable_task_id(),
        Some(running_task_id.clone())
    );

    snapshot.draft_input = "/cancel".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::CancelTask {
            task_id: Some(running_task_id)
        })
    );
}

#[test]
fn client_snapshot_projects_pending_and_resolved_tool_approvals() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let approval_id = ApprovalId::from_static("approval_write_readme");
    let call_id = ToolCallId::from_static("tool_call_write_readme");
    let tool_id = ToolId::from_static("tool_workspace_write");

    snapshot.apply_event(&EventFrame::new(
        "trace_approval",
        1,
        RunEvent::ToolPolicyDecisionRecorded {
            decision: ToolPolicyDecision {
                decision_id: PolicyDecisionId::from_static("policy_write_readme"),
                call_id: call_id.clone(),
                tool_id: tool_id.clone(),
                outcome: PolicyOutcome::AskUser,
                reason: "workspace_write_requires_approval".to_string(),
                required_permissions: vec![ToolPermission::FilesystemWrite],
                side_effects: vec![ToolSideEffect::WritesWorkspace],
                approval_id: Some(approval_id.clone()),
            },
        },
    ));

    assert_eq!(snapshot.approvals.len(), 1);
    assert_eq!(snapshot.approvals[0].approval_id, approval_id);
    assert_eq!(snapshot.approvals[0].call_id, call_id);
    assert_eq!(snapshot.approvals[0].tool_id, tool_id);
    assert_eq!(snapshot.approvals[0].status, ClientApprovalStatus::Pending);
    assert_eq!(
        snapshot.approvals[0].required_permissions,
        vec!["filesystem_write"]
    );
    assert_eq!(snapshot.status.approval_summary, "approvals 1 pending");

    snapshot.apply_event(&EventFrame::new(
        "trace_approval",
        2,
        RunEvent::ToolCallApproved {
            approval: ToolApproval {
                approval_id: ApprovalId::from_static("approval_write_readme"),
                call_id: ToolCallId::from_static("tool_call_write_readme"),
                tool_id: ToolId::from_static("tool_workspace_write"),
                status: ApprovalStatus::Approved,
                reason: Some("user approved workspace write".to_string()),
            },
        },
    ));

    assert_eq!(snapshot.approvals[0].status, ClientApprovalStatus::Approved);
    assert_eq!(
        snapshot.approvals[0].reason.as_deref(),
        Some("user approved workspace write")
    );
    assert_eq!(snapshot.status.approval_summary, "approvals 0 pending");
}

#[test]
fn client_snapshot_projects_memory_proposals_for_ui_review() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let proposal_id = MemoryProposalId::from_static("memory_proposal_prefers_rust");
    let pending = MemoryProposal {
        proposal_id: proposal_id.clone(),
        status: MemoryProposalStatus::Pending,
        title: "Preferred language".to_string(),
        summary: "User prefers Rust-first implementations.".to_string(),
        source_item_id: Some(ItemId::from_static("item_memory_source")),
        reason: Some("explicit preference".to_string()),
        metadata: None,
    };

    snapshot.apply_event(&EventFrame::new(
        "trace_memory_client",
        1,
        RunEvent::MemoryWriteProposed {
            proposal: pending.clone(),
        },
    ));

    assert_eq!(snapshot.memory_proposals.len(), 1);
    assert_eq!(snapshot.memory_proposals[0].proposal_id, proposal_id);
    assert_eq!(
        snapshot.memory_proposals[0].status,
        ClientMemoryProposalStatus::Pending
    );
    assert_eq!(
        snapshot.memory_proposals[0].summary,
        "User prefers Rust-first implementations."
    );
    assert_eq!(snapshot.status.memory_summary, "memory 1 pending");

    snapshot.apply_event(&EventFrame::new(
        "trace_memory_client",
        2,
        RunEvent::MemoryWriteApplied {
            proposal: MemoryProposal {
                status: MemoryProposalStatus::Applied,
                ..pending.clone()
            },
        },
    ));

    assert_eq!(
        snapshot.memory_proposals[0].status,
        ClientMemoryProposalStatus::Applied
    );
    assert_eq!(snapshot.status.memory_summary, "memory 0 pending");

    let mut replayed = ClientSnapshot::new("mock-default");
    replayed.apply_trace_record(
        &EventFrame::new(
            "trace_memory_client",
            3,
            RunEvent::MemoryWriteRejected {
                proposal: MemoryProposal {
                    status: MemoryProposalStatus::Rejected,
                    reason: Some("user rejected".to_string()),
                    ..pending
                },
            },
        )
        .to_trace_record(),
    );

    assert_eq!(
        replayed.memory_proposals[0].status,
        ClientMemoryProposalStatus::Rejected
    );
    assert_eq!(
        replayed.memory_proposals[0].reason.as_deref(),
        Some("user rejected")
    );
}

#[test]
fn client_snapshot_resets_thread_and_exports_markdown_projection() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    snapshot.apply_event(&EventFrame::new(
        "trace_export",
        1,
        RunEvent::UserMessageRecorded {
            item_id: ItemId::from_static("item_user_export"),
            text: "hello export".to_string(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_export",
        2,
        RunEvent::AssistantDelta {
            item_id: ItemId::from_static("item_assistant_export"),
            text: "exported answer".to_string(),
        },
    ));

    let markdown = snapshot.export_markdown();
    assert!(markdown.contains("# Tessera Export"));
    assert!(markdown.contains("## User"));
    assert!(markdown.contains("hello export"));
    assert!(markdown.contains("## Assistant"));
    assert!(markdown.contains("exported answer"));

    snapshot.start_new_thread();

    assert!(snapshot.projection.messages.is_empty());
    assert_eq!(snapshot.draft_input, "");
    assert_eq!(snapshot.status.active_profile, "mock-default");
}

#[test]
fn client_snapshot_updates_status_from_live_usage_reported_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.apply_event(&EventFrame::new(
        "trace_usage",
        1,
        RunEvent::ProviderCapabilityReported {
            provider_id: ProviderId::from_static("mock"),
            capability: ProviderCapability {
                provider_id: ProviderId::from_static("mock"),
                supports_streaming: true,
                supports_reasoning_delta: true,
                supports_cache_telemetry: true,
                supports_cost_estimate: true,
                supports_tool_calling: false,
                max_context_tokens: Some(4_000),
                extension: None,
            },
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_usage",
        2,
        RunEvent::UsageReported {
            input_tokens: Some(1_000),
            output_tokens: Some(200),
            total_tokens: Some(1_200),
            cache_read_tokens: Some(800),
            cache_write_tokens: None,
            cache_miss_tokens: Some(200),
            estimated_cost: Some(CostEstimate {
                amount: 0.0123,
                currency: "USD".to_string(),
                input_cost: Some(0.0100),
                output_cost: Some(0.0023),
                cache_read_cost: Some(0.0010),
                cache_write_cost: None,
            }),
            latency_ms: Some(42),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_usage",
        3,
        RunEvent::UsageReported {
            input_tokens: Some(500),
            output_tokens: Some(50),
            total_tokens: Some(550),
            cache_read_tokens: Some(300),
            cache_write_tokens: None,
            cache_miss_tokens: Some(200),
            estimated_cost: Some(CostEstimate {
                amount: 0.0057,
                currency: "USD".to_string(),
                input_cost: Some(0.0040),
                output_cost: Some(0.0017),
                cache_read_cost: Some(0.0003),
                cache_write_cost: None,
            }),
            latency_ms: Some(24),
        },
    ));

    assert_eq!(
        snapshot.status.usage_summary,
        "usage in 1500 / out 250 / total 1750"
    );
    assert_eq!(snapshot.status.cache_summary, "cache 1100/1500 (73%)");
    assert_eq!(snapshot.status.cost_summary, "USD 0.0180");
    assert_eq!(snapshot.status.context_summary, "ctx 500/4000 (12%)");
}

#[test]
fn client_snapshot_updates_status_from_replayed_usage_reported_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let record = EventFrame::new(
        "trace_usage_replay",
        1,
        RunEvent::UsageReported {
            input_tokens: Some(2_000),
            output_tokens: Some(300),
            total_tokens: Some(2_300),
            cache_read_tokens: Some(1_500),
            cache_write_tokens: None,
            cache_miss_tokens: Some(500),
            estimated_cost: Some(CostEstimate {
                amount: 0.0456,
                currency: "CNY".to_string(),
                input_cost: Some(0.0300),
                output_cost: Some(0.0156),
                cache_read_cost: Some(0.0030),
                cache_write_cost: None,
            }),
            latency_ms: Some(84),
        },
    )
    .to_trace_record();

    snapshot.apply_trace_record(&record);

    assert_eq!(
        snapshot.status.usage_summary,
        "usage in 2000 / out 300 / total 2300"
    );
    assert_eq!(snapshot.status.cache_summary, "cache 1500/2000 (75%)");
    assert_eq!(snapshot.status.cost_summary, "CNY 0.0456");
    assert_eq!(snapshot.status.context_summary, "ctx 2000 tokens");
}

#[test]
fn client_snapshot_uses_input_tokens_as_cache_denominator_when_miss_tokens_are_absent() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.apply_event(&EventFrame::new(
        "trace_cache_denominator",
        1,
        RunEvent::UsageReported {
            input_tokens: Some(1_000),
            output_tokens: Some(200),
            total_tokens: Some(1_200),
            cache_read_tokens: Some(800),
            cache_write_tokens: None,
            cache_miss_tokens: None,
            estimated_cost: None,
            latency_ms: None,
        },
    ));

    assert_eq!(snapshot.status.cache_summary, "cache 800/1000 (80%)");
}

#[test]
fn client_snapshot_updates_context_summary_from_replayed_provider_capability_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let capability_record = EventFrame::new(
        "trace_context_replay",
        1,
        RunEvent::ProviderCapabilityReported {
            provider_id: ProviderId::from_static("mock"),
            capability: ProviderCapability {
                provider_id: ProviderId::from_static("mock"),
                supports_streaming: true,
                supports_reasoning_delta: true,
                supports_cache_telemetry: true,
                supports_cost_estimate: true,
                supports_tool_calling: false,
                max_context_tokens: Some(8_000),
                extension: None,
            },
        },
    )
    .to_trace_record();
    let usage_record = EventFrame::new(
        "trace_context_replay",
        2,
        RunEvent::UsageReported {
            input_tokens: Some(2_000),
            output_tokens: Some(300),
            total_tokens: Some(2_300),
            cache_read_tokens: None,
            cache_write_tokens: None,
            cache_miss_tokens: None,
            estimated_cost: None,
            latency_ms: None,
        },
    )
    .to_trace_record();

    snapshot.apply_trace_record(&capability_record);
    snapshot.apply_trace_record(&usage_record);

    assert_eq!(snapshot.status.context_summary, "ctx 2000/8000 (25%)");
}

#[test]
fn client_snapshot_updates_task_registry_from_live_task_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let task_id = TaskId::from_static("task_live");

    snapshot.apply_event(&EventFrame::new(
        "trace_task_live",
        1,
        RunEvent::TaskCreated {
            task_id: task_id.clone(),
            kind: TaskKind::Chat,
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_task_live",
        2,
        RunEvent::TaskStarted {
            task_id: task_id.clone(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_task_live",
        3,
        RunEvent::TaskCompleted {
            task_id: task_id.clone(),
        },
    ));

    assert_eq!(snapshot.tasks.len(), 1);
    assert_eq!(snapshot.tasks[0].task_id, task_id);
    assert_eq!(snapshot.tasks[0].kind, Some(TaskKind::Chat));
    assert_eq!(snapshot.tasks[0].status, TaskStatus::Completed);
    assert!(snapshot.tasks[0].created_at.is_some());
    assert!(snapshot.tasks[0].started_at.is_some());
    assert!(snapshot.tasks[0].finished_at.is_some());
    assert_eq!(snapshot.status.task_summary, "task completed");
}

#[test]
fn client_snapshot_updates_task_registry_from_replayed_failed_and_cancelled_tasks() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let failed_task_id = TaskId::from_static("task_failed_replay");
    let cancelled_task_id = TaskId::from_static("task_cancelled_replay");

    let records = [
        EventFrame::new(
            "trace_task_replay",
            1,
            RunEvent::TaskCreated {
                task_id: failed_task_id.clone(),
                kind: TaskKind::Chat,
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_task_replay",
            2,
            RunEvent::TaskFailed {
                task_id: failed_task_id.clone(),
                error: NormalizedError {
                    code: "provider_rate_limited".to_string(),
                    message: "provider rate limit reached".to_string(),
                    retryable: true,
                    source: ErrorSource::Provider,
                    details: None,
                },
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_task_replay",
            3,
            RunEvent::TaskCreated {
                task_id: cancelled_task_id.clone(),
                kind: TaskKind::Replay,
            },
        )
        .to_trace_record(),
        EventFrame::new(
            "trace_task_replay",
            4,
            RunEvent::TaskCancelled {
                task_id: cancelled_task_id.clone(),
                reason: Some("client stopped".to_string()),
            },
        )
        .to_trace_record(),
    ];

    for record in records {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.tasks.len(), 2);
    assert_eq!(snapshot.tasks[0].task_id, failed_task_id);
    assert_eq!(snapshot.tasks[0].status, TaskStatus::Failed);
    assert_eq!(
        snapshot.tasks[0].error_code.as_deref(),
        Some("provider_rate_limited")
    );
    assert_eq!(
        snapshot.tasks[0].error_message.as_deref(),
        Some("provider rate limit reached")
    );
    assert_eq!(snapshot.tasks[1].task_id, cancelled_task_id);
    assert_eq!(snapshot.tasks[1].status, TaskStatus::Cancelled);
    assert_eq!(
        snapshot.tasks[1].cancel_reason.as_deref(),
        Some("client stopped")
    );
    assert_eq!(snapshot.status.task_summary, "task cancelled");
}

#[test]
fn client_snapshot_projects_artifact_handles_from_live_events() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let artifact_id = ArtifactId::from_static("artifact_live");
    let task_id = TaskId::from_static("task_artifact_live");
    let item_id = ItemId::from_static("item_artifact_live");

    snapshot.apply_event(
        &EventFrame::new(
            "trace_artifact_live",
            1,
            RunEvent::ArtifactCreated {
                artifact_id: artifact_id.clone(),
                kind: ArtifactKind::Export,
            },
        )
        .with_task_id(task_id.clone()),
    );
    snapshot.apply_event(
        &EventFrame::new(
            "trace_artifact_live",
            2,
            RunEvent::AssistantDelta {
                item_id: item_id.clone(),
                text: "see artifact".to_string(),
            },
        )
        .with_task_id(task_id.clone())
        .with_item_id(item_id.clone())
        .with_artifact_ref(artifact_id.clone()),
    );

    assert_eq!(snapshot.artifacts.len(), 1);
    assert_eq!(snapshot.artifacts[0].artifact_id, artifact_id);
    assert_eq!(snapshot.artifacts[0].kind, Some(ArtifactKind::Export));
    assert_eq!(snapshot.artifacts[0].task_id, Some(task_id));
    assert_eq!(snapshot.artifacts[0].item_id, Some(item_id));
    assert!(snapshot.artifacts[0].created_at.is_some());
    assert_eq!(
        snapshot.artifacts[0].referenced_by_event_kinds,
        vec!["assistant_delta"]
    );
    assert_eq!(snapshot.status.artifact_summary, "artifacts 1");
}

#[test]
fn client_snapshot_projects_artifact_handles_from_replayed_trace_records() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    let artifact_id = ArtifactId::from_static("artifact_replay");
    let task_id = TaskId::from_static("task_artifact_replay");

    let records = [
        EventFrame::new(
            "trace_artifact_replay",
            1,
            RunEvent::AssistantDelta {
                item_id: ItemId::from_static("item_artifact_replay"),
                text: "artifact ref first".to_string(),
            },
        )
        .with_task_id(task_id.clone())
        .with_artifact_ref(artifact_id.clone())
        .to_trace_record(),
        EventFrame::new(
            "trace_artifact_replay",
            2,
            RunEvent::ArtifactCreated {
                artifact_id: artifact_id.clone(),
                kind: ArtifactKind::TestReport,
            },
        )
        .with_task_id(task_id.clone())
        .to_trace_record(),
    ];

    for record in records {
        snapshot.apply_trace_record(&record);
    }

    assert_eq!(snapshot.artifacts.len(), 1);
    assert_eq!(snapshot.artifacts[0].artifact_id, artifact_id);
    assert_eq!(snapshot.artifacts[0].kind, Some(ArtifactKind::TestReport));
    assert_eq!(snapshot.artifacts[0].task_id, Some(task_id));
    assert_eq!(
        snapshot.artifacts[0].referenced_by_event_kinds,
        vec!["assistant_delta"]
    );
    assert_eq!(snapshot.status.artifact_summary, "artifacts 1");
}
