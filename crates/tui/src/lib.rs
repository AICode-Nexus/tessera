use ratatui::text::{Line, Span};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatViewState {
    pub active_profile: String,
    pub reasoning_visible: bool,
    pub cache_summary: String,
    pub cost_summary: String,
}

pub fn status_line(state: &ChatViewState) -> Line<'static> {
    let reasoning = if state.reasoning_visible {
        "reasoning:on"
    } else {
        "reasoning:off"
    };
    Line::from(vec![
        Span::raw("profile "),
        Span::raw(state.active_profile.clone()),
        Span::raw(" | "),
        Span::raw(reasoning),
        Span::raw(" | "),
        Span::raw(state.cache_summary.clone()),
        Span::raw(" | "),
        Span::raw(state.cost_summary.clone()),
    ])
}
