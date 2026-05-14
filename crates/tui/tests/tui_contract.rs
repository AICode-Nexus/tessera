use tessera_tui::{status_line, ChatViewState};

#[test]
fn tui_status_line_contains_profile_reasoning_and_cost_placeholders() {
    let state = ChatViewState {
        active_profile: "mock-default".to_string(),
        reasoning_visible: true,
        cache_summary: "cache 0/0".to_string(),
        cost_summary: "CNY 0.0000".to_string(),
    };

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
