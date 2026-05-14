use tessera_protocol::{EventFrame, ItemId, RunEvent};
use tessera_tui::{chat_window_lines, status_line, ChatMessageRole, ChatViewState, TuiUserIntent};

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
        Some(TuiUserIntent::SubmitPrompt {
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
