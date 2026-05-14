use tessera_protocol::{EventFrame, ItemId, RunEvent};
use tessera_tui::{
    chat_window_lines, draw_terminal_frame, map_key_event, status_line, ChatMessageRole,
    ChatViewState, ClientIntent, TerminalAction, TerminalInput,
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
    state.reasoning_visible = true;
    state.cache_summary = "cache 0/0".to_string();
    state.cost_summary = "CNY 0.0000".to_string();

    let line = status_line(&state);
    let spans: Vec<_> = line
        .spans
        .iter()
        .map(|span| span.content.as_ref().to_string())
        .collect();

    assert!(spans.join("").contains("mock-default"));
    assert!(spans.join("").contains("reasoning"));
    assert!(spans.join("").contains("cache 0/0"));
    assert!(spans.join("").contains("CNY 0.0000"));
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
    assert_eq!(state.input, "");

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
    assert_eq!(state.messages[0].role, ChatMessageRole::User);
    assert_eq!(state.messages[1].role, ChatMessageRole::Assistant);
    assert!(!state.messages[1].streaming);
}

#[test]
fn tui_keeps_reasoning_and_answer_deltas_separate_for_same_item() {
    let mut state = ChatViewState::new("mock-default");
    state.reasoning_visible = true;
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

    assert_eq!(state.messages[0].role, ChatMessageRole::Reasoning);
    assert_eq!(state.messages[0].content, "thinking");
    assert_eq!(state.messages[1].role, ChatMessageRole::Assistant);
    assert_eq!(state.messages[1].content, "answer");
}

#[test]
fn terminal_input_edits_and_submits_prompt() {
    let mut state = ChatViewState::new("mock-default");

    assert_eq!(
        state.handle_terminal_input(TerminalInput::Char('h')),
        TerminalAction::Render
    );
    assert_eq!(
        state.handle_terminal_input(TerminalInput::Char('i')),
        TerminalAction::Render
    );
    assert_eq!(state.input, "hi");

    assert_eq!(
        state.handle_terminal_input(TerminalInput::Backspace),
        TerminalAction::Render
    );
    assert_eq!(state.input, "h");

    assert_eq!(
        state.handle_terminal_input(TerminalInput::Submit),
        TerminalAction::Dispatch(ClientIntent::SubmitPrompt {
            profile_id: "mock-default".to_string(),
            prompt: "h".to_string()
        })
    );
    assert_eq!(state.input, "");
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
        Some(TerminalInput::Quit)
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
fn profile_switch_cycles_available_profiles_as_client_intents() {
    let mut state =
        ChatViewState::with_profiles("mock-default", ["mock-default", "offline", "local-llm"]);

    assert_eq!(
        state.handle_terminal_input(TerminalInput::NextProfile),
        TerminalAction::Dispatch(ClientIntent::SwitchProfile {
            profile_id: "offline".to_string()
        })
    );
    assert_eq!(state.active_profile, "offline");

    assert_eq!(
        state.handle_terminal_input(TerminalInput::PreviousProfile),
        TerminalAction::Dispatch(ClientIntent::SwitchProfile {
            profile_id: "mock-default".to_string()
        })
    );
    assert_eq!(state.active_profile, "mock-default");
}

#[test]
fn prompt_submit_uses_current_profile_after_switch() {
    let mut state = ChatViewState::with_profiles("mock-default", ["mock-default", "offline"]);
    state.handle_terminal_input(TerminalInput::NextProfile);
    state.set_input("hello selected profile");

    assert_eq!(
        state.handle_terminal_input(TerminalInput::Submit),
        TerminalAction::Dispatch(ClientIntent::SubmitPrompt {
            profile_id: "offline".to_string(),
            prompt: "hello selected profile".to_string()
        })
    );
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

    assert_eq!(state.messages[0].role, ChatMessageRole::Assistant);
    assert_eq!(state.messages[0].content, "from trace");
}
