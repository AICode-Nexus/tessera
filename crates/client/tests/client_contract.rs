use tessera_client::{
    ClientIntent, ClientMessageRole, ClientProjection, ClientSnapshot, ClientStatus,
};
use tessera_protocol::{
    ArtifactId, ArtifactKind, CostEstimate, ErrorSource, EventFrame, ItemId, NormalizedError,
    ProviderCapability, ProviderId, RunEvent, TaskId, TaskKind, TaskStatus,
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
fn client_snapshot_maps_slash_commands_to_ui_neutral_intents() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.draft_input = " /new ".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::NewThread));

    snapshot.draft_input = "/save".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::SaveThread));

    snapshot.draft_input = "/export".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::ExportThread));

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
