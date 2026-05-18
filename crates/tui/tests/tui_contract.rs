use tessera_client::ClientContextBudgetSummary;
use tessera_protocol::{
    ApprovalId, ArtifactId, ArtifactKind, ContextId, ContextPlacement, ContextReference,
    ContextSource, ContextSourceKind, CostEstimate, EventFrame, ItemId, MemoryProposal,
    MemoryProposalId, MemoryProposalStatus, PolicyDecisionId, PolicyOutcome, ProviderCapability,
    ProviderId, RunEvent, TaskId, TaskKind, ToolCallId, ToolId, ToolPermission, ToolPolicyDecision,
    ToolSideEffect,
};
use tessera_tui::{
    apply_client_intent_locally, apply_live_event, chat_window_lines, draw_terminal_frame,
    handle_terminal_input, live_client_event_channel, map_key_event, status_line, ChatMessageRole,
    ChatViewState, ClientIntent, LiveClientEvent, TerminalAction, TerminalInput,
};

fn buffer_text(buffer: &ratatui::buffer::Buffer) -> String {
    let area = *buffer.area();
    let mut output = String::new();
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            output.push_str(buffer.cell((x, y)).unwrap().symbol());
        }
        output.push('\n');
    }
    output
}

#[test]
fn tui_status_line_contains_profile_reasoning_and_cost_placeholders() {
    let mut state = ChatViewState::new("mock-default");
    state.status.reasoning_visible = true;
    state.status.usage_summary = "usage in 0 / out 0 / total 0".to_string();
    state.status.cache_summary = "cache 0/0".to_string();
    state.status.cost_summary = "CNY 0.0000".to_string();
    state.status.context_summary = "ctx 0 tokens".to_string();

    let line = status_line(&state);
    let spans: Vec<_> = line
        .spans
        .iter()
        .map(|span| span.content.as_ref().to_string())
        .collect();

    assert!(spans.join("").contains("mock-default"));
    assert!(spans.join("").contains("reasoning"));
    assert!(spans.join("").contains("task idle"));
    assert!(spans.join("").contains("artifacts 0"));
    assert!(spans.join("").contains("usage in 0 / out 0 / total 0"));
    assert!(spans.join("").contains("cache 0/0"));
    assert!(spans.join("").contains("CNY 0.0000"));
    assert!(spans.join("").contains("ctx 0 tokens"));
}

#[test]
fn tui_status_line_renders_live_usage_cache_cost_and_context_summary() {
    let mut state = ChatViewState::new("mock-default");

    apply_live_event(
        &mut state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_usage",
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
        ))),
    );
    apply_live_event(
        &mut state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_usage",
            2,
            RunEvent::UsageReported {
                input_tokens: Some(1_000),
                output_tokens: Some(250),
                total_tokens: Some(1_250),
                cache_read_tokens: Some(750),
                cache_write_tokens: None,
                cache_miss_tokens: Some(250),
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
        ))),
    );

    let rendered = status_line(&state)
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(rendered.contains("usage in 1000 / out 250 / total 1250"));
    assert!(rendered.contains("cache 750/1000 (75%)"));
    assert!(rendered.contains("USD 0.0123"));
    assert!(rendered.contains("ctx 1000/4000 (25%)"));
}

#[test]
fn tui_status_line_renders_live_task_summary() {
    let mut state = ChatViewState::new("mock-default");
    let task_id = TaskId::from_static("task_tui_live");

    apply_live_event(
        &mut state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_task",
            1,
            RunEvent::TaskCreated {
                task_id: task_id.clone(),
                kind: TaskKind::Chat,
            },
        ))),
    );
    apply_live_event(
        &mut state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_task",
            2,
            RunEvent::TaskStarted { task_id },
        ))),
    );

    let rendered = status_line(&state)
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(rendered.contains("task running"));
}

#[test]
fn tui_status_line_renders_live_artifact_summary() {
    let mut state = ChatViewState::new("mock-default");

    apply_live_event(
        &mut state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_artifact",
            1,
            RunEvent::ArtifactCreated {
                artifact_id: ArtifactId::from_static("artifact_tui_live"),
                kind: ArtifactKind::Export,
            },
        ))),
    );

    let rendered = status_line(&state)
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(rendered.contains("artifacts 1"));
}

#[test]
fn tui_status_line_renders_context_handle_summary() {
    let mut state = ChatViewState::new("mock-default");

    state.set_context_handles(
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

    let rendered = status_line(&state)
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(rendered.contains("context 2 handles / 150/160 tokens"));
}

#[test]
fn tui_status_line_renders_pending_approval_summary() {
    let mut state = ChatViewState::new("mock-default");

    apply_live_event(
        &mut state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_approval",
            1,
            RunEvent::ToolPolicyDecisionRecorded {
                decision: ToolPolicyDecision {
                    decision_id: PolicyDecisionId::from_static("policy_write_readme"),
                    call_id: ToolCallId::from_static("tool_call_write_readme"),
                    tool_id: ToolId::from_static("tool_workspace_write"),
                    outcome: PolicyOutcome::AskUser,
                    reason: "workspace_write_requires_approval".to_string(),
                    required_permissions: vec![ToolPermission::FilesystemWrite],
                    side_effects: vec![ToolSideEffect::WritesWorkspace],
                    approval_id: Some(ApprovalId::from_static("approval_write_readme")),
                },
            },
        ))),
    );

    let rendered = status_line(&state)
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(rendered.contains("approvals 1 pending"));
}

#[test]
fn tui_chat_loop_submits_input_and_renders_core_events() {
    let mut state = ChatViewState::new("mock-default");
    state.set_input("hello tui");

    assert_eq!(
        state.submit_input(),
        Some(ClientIntent::SubmitPrompt {
            profile_id: "mock-default".to_string(),
            prompt: "hello tui".to_string()
        })
    );
    assert_eq!(state.draft_input, "");

    let user_item_id = ItemId::from_static("item_user");
    let assistant_item_id = ItemId::from_static("item_assistant");

    state.apply_event(&EventFrame::new(
        "trace_tui",
        1,
        RunEvent::UserMessageRecorded {
            item_id: user_item_id,
            text: "hello tui".to_string(),
        },
    ));
    state.apply_event(&EventFrame::new(
        "trace_tui",
        2,
        RunEvent::AssistantMessageStarted {
            item_id: assistant_item_id.clone(),
        },
    ));
    state.apply_event(&EventFrame::new(
        "trace_tui",
        3,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "mock ".to_string(),
        },
    ));
    state.apply_event(&EventFrame::new(
        "trace_tui",
        4,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "response".to_string(),
        },
    ));
    state.apply_event(&EventFrame::new(
        "trace_tui",
        5,
        RunEvent::AssistantMessageCompleted {
            item_id: assistant_item_id,
        },
    ));

    let rendered = chat_window_lines(&state)
        .into_iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("You: hello tui"));
    assert!(rendered.contains("Assistant: mock response"));
    assert!(rendered.contains("> "));
    assert_eq!(state.projection.messages[0].role, ChatMessageRole::User);
    assert_eq!(
        state.projection.messages[1].role,
        ChatMessageRole::Assistant
    );
    assert!(!state.projection.messages[1].streaming);
}

#[test]
fn tui_keeps_reasoning_and_answer_deltas_separate_for_same_item() {
    let mut state = ChatViewState::new("mock-default");
    state.status.reasoning_visible = true;
    let assistant_item_id = ItemId::from_static("item_assistant");

    state.apply_event(&EventFrame::new(
        "trace_tui_reasoning",
        1,
        RunEvent::AssistantReasoningDelta {
            item_id: assistant_item_id.clone(),
            text: "thinking".to_string(),
        },
    ));
    state.apply_event(&EventFrame::new(
        "trace_tui_reasoning",
        2,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id,
            text: "answer".to_string(),
        },
    ));

    assert_eq!(
        state.projection.messages[0].role,
        ChatMessageRole::Reasoning
    );
    assert_eq!(state.projection.messages[0].content, "thinking");
    assert_eq!(
        state.projection.messages[1].role,
        ChatMessageRole::Assistant
    );
    assert_eq!(state.projection.messages[1].content, "answer");
}

#[test]
fn terminal_input_edits_and_submits_prompt() {
    let mut state = ChatViewState::new("mock-default");

    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Char('h')),
        TerminalAction::Render
    );
    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Char('i')),
        TerminalAction::Render
    );
    assert_eq!(state.draft_input, "hi");

    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Backspace),
        TerminalAction::Render
    );
    assert_eq!(state.draft_input, "h");

    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Submit),
        TerminalAction::Dispatch(ClientIntent::SubmitPrompt {
            profile_id: "mock-default".to_string(),
            prompt: "h".to_string()
        })
    );
    assert_eq!(state.draft_input, "");
}

#[test]
fn terminal_key_mapping_handles_submit_backspace_and_quit() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    assert_eq!(
        map_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(TerminalInput::Submit)
    );
    assert_eq!(
        map_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        Some(TerminalInput::Backspace)
    );
    assert_eq!(
        map_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        Some(TerminalInput::Interrupt)
    );
    assert_eq!(
        map_key_event(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        Some(TerminalInput::Char('x'))
    );
    assert_eq!(
        map_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        Some(TerminalInput::NextProfile)
    );
    assert_eq!(
        map_key_event(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
        Some(TerminalInput::PreviousProfile)
    );
}

#[test]
fn terminal_interrupt_cancels_running_task_and_quits_when_idle() {
    let mut idle_state = ChatViewState::new("mock-default");
    assert_eq!(
        handle_terminal_input(&mut idle_state, TerminalInput::Interrupt),
        TerminalAction::Quit
    );

    let mut running_state = ChatViewState::new("mock-default");
    let task_id = TaskId::from_static("task_tui_cancel");
    apply_live_event(
        &mut running_state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_cancel",
            1,
            RunEvent::TaskCreated {
                task_id: task_id.clone(),
                kind: TaskKind::Chat,
            },
        ))),
    );
    apply_live_event(
        &mut running_state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_cancel",
            2,
            RunEvent::TaskStarted {
                task_id: task_id.clone(),
            },
        ))),
    );

    assert_eq!(
        handle_terminal_input(&mut running_state, TerminalInput::Interrupt),
        TerminalAction::Dispatch(ClientIntent::CancelTask {
            task_id: Some(task_id)
        })
    );
}

#[test]
fn profile_switch_cycles_available_profiles_as_client_intents() {
    let mut state =
        ChatViewState::with_profiles("mock-default", ["mock-default", "offline", "local-llm"]);

    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::NextProfile),
        TerminalAction::Dispatch(ClientIntent::SwitchProfile {
            profile_id: "offline".to_string()
        })
    );
    assert_eq!(state.status.active_profile, "offline");

    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::PreviousProfile),
        TerminalAction::Dispatch(ClientIntent::SwitchProfile {
            profile_id: "mock-default".to_string()
        })
    );
    assert_eq!(state.status.active_profile, "mock-default");
}

#[test]
fn tui_status_line_renders_pending_memory_proposal_summary() {
    let mut state = ChatViewState::new("mock-default");

    apply_live_event(
        &mut state,
        LiveClientEvent::Frame(Box::new(EventFrame::new(
            "trace_tui_memory",
            1,
            RunEvent::MemoryWriteProposed {
                proposal: MemoryProposal {
                    proposal_id: MemoryProposalId::from_static("memory_proposal_tui"),
                    status: MemoryProposalStatus::Pending,
                    title: "Preferred language".to_string(),
                    summary: "User prefers Rust-first work.".to_string(),
                    source_item_id: None,
                    reason: None,
                    metadata: None,
                },
            },
        ))),
    );

    let rendered = status_line(&state)
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<String>();

    assert!(rendered.contains("memory 1 pending"));
}

#[test]
fn prompt_submit_uses_current_profile_after_switch() {
    let mut state = ChatViewState::with_profiles("mock-default", ["mock-default", "offline"]);
    handle_terminal_input(&mut state, TerminalInput::NextProfile);
    state.set_input("hello selected profile");

    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Submit),
        TerminalAction::Dispatch(ClientIntent::SubmitPrompt {
            profile_id: "offline".to_string(),
            prompt: "hello selected profile".to_string()
        })
    );
}

#[test]
fn slash_commands_dispatch_new_save_and_export_intents() {
    let mut state = ChatViewState::new("mock-default");

    state.set_input("/new");
    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Submit),
        TerminalAction::Dispatch(ClientIntent::NewThread)
    );

    state.set_input("/save");
    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Submit),
        TerminalAction::Dispatch(ClientIntent::SaveThread)
    );

    state.set_input("/export");
    assert_eq!(
        handle_terminal_input(&mut state, TerminalInput::Submit),
        TerminalAction::Dispatch(ClientIntent::ExportThread)
    );
}

#[test]
fn local_thread_commands_update_tui_snapshot_without_runtime_access() {
    let mut state = ChatViewState::new("mock-default");
    state.apply_event(&EventFrame::new(
        "trace_local_commands",
        1,
        RunEvent::UserMessageRecorded {
            item_id: ItemId::from_static("item_local_command"),
            text: "keep then clear".to_string(),
        },
    ));

    assert!(apply_client_intent_locally(
        &mut state,
        &ClientIntent::SaveThread
    ));
    assert_eq!(
        state.projection.messages.last().unwrap().role,
        ChatMessageRole::System
    );

    assert!(apply_client_intent_locally(
        &mut state,
        &ClientIntent::ExportThread
    ));
    assert!(state
        .projection
        .messages
        .last()
        .unwrap()
        .content
        .contains("Export prepared"));

    assert!(apply_client_intent_locally(
        &mut state,
        &ClientIntent::NewThread
    ));
    assert!(state.projection.messages.is_empty());

    assert!(!apply_client_intent_locally(
        &mut state,
        &ClientIntent::SubmitPrompt {
            profile_id: "mock-default".to_string(),
            prompt: "hello".to_string(),
        }
    ));
}

#[test]
fn terminal_frame_renders_status_messages_and_input() {
    let backend = ratatui::backend::TestBackend::new(80, 12);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let mut state = ChatViewState::new("mock-default");
    state.set_input("draft");
    state.apply_event(&EventFrame::new(
        "trace_tui_frame",
        1,
        RunEvent::UserMessageRecorded {
            item_id: ItemId::from_static("item_user"),
            text: "hello frame".to_string(),
        },
    ));

    terminal
        .draw(|frame| draw_terminal_frame(frame, &state))
        .unwrap();

    let rendered = buffer_text(terminal.backend().buffer());
    assert!(rendered.contains("profile mock-default [1/1]"));
    assert!(rendered.contains("You: hello frame"));
    assert!(rendered.contains("> draft"));
}

#[test]
fn tui_applies_trace_records_from_core_storage() {
    let mut state = ChatViewState::new("mock-default");
    let record = EventFrame::new(
        "trace_tui_record",
        1,
        RunEvent::AssistantDelta {
            item_id: ItemId::from_static("item_assistant"),
            text: "from trace".to_string(),
        },
    )
    .to_trace_record();

    state.apply_trace_record(&record);

    assert_eq!(
        state.projection.messages[0].role,
        ChatMessageRole::Assistant
    );
    assert_eq!(state.projection.messages[0].content, "from trace");
}

#[test]
fn tui_applies_live_client_events_without_waiting_for_trace_replay() {
    let mut state = ChatViewState::new("mock-default");
    let frame = EventFrame::new(
        "trace_live_tui",
        1,
        RunEvent::AssistantDelta {
            item_id: ItemId::from_static("item_live"),
            text: "live delta".to_string(),
        },
    );

    apply_live_event(&mut state, LiveClientEvent::Frame(Box::new(frame)));
    apply_live_event(
        &mut state,
        LiveClientEvent::Error("network down".to_string()),
    );

    assert_eq!(
        state.projection.messages[0].role,
        ChatMessageRole::Assistant
    );
    assert_eq!(state.projection.messages[0].content, "live delta");
    assert_eq!(
        state.projection.messages[1].role,
        ChatMessageRole::Assistant
    );
    assert_eq!(state.projection.messages[1].content, "Error: network down");
}

#[test]
fn live_client_event_channel_is_bounded_for_backpressure() {
    let (sender, _receiver) = live_client_event_channel(1);

    sender
        .try_send(LiveClientEvent::Error("first".to_string()))
        .unwrap();
    let error = sender
        .try_send(LiveClientEvent::Error("second".to_string()))
        .unwrap_err();

    assert!(matches!(
        error,
        tokio::sync::mpsc::error::TrySendError::Full(_)
    ));
}
