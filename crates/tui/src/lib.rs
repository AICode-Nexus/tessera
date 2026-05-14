use ratatui::text::{Line, Span};
use tessera_protocol::{EventFrame, ItemId, RunEvent};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChatMessageRole {
    User,
    Assistant,
    Reasoning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatMessage {
    pub role: ChatMessageRole,
    pub content: String,
    pub item_id: Option<ItemId>,
    pub streaming: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TuiUserIntent {
    SubmitPrompt { prompt: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatViewState {
    pub active_profile: String,
    pub reasoning_visible: bool,
    pub cache_summary: String,
    pub cost_summary: String,
    pub input: String,
    pub messages: Vec<ChatMessage>,
}

impl ChatViewState {
    pub fn new(active_profile: impl Into<String>) -> Self {
        Self {
            active_profile: active_profile.into(),
            reasoning_visible: false,
            cache_summary: "cache 0/0".to_string(),
            cost_summary: "CNY 0.0000".to_string(),
            input: String::new(),
            messages: Vec::new(),
        }
    }

    pub fn set_input(&mut self, input: impl Into<String>) {
        self.input = input.into();
    }

    pub fn submit_input(&mut self) -> Option<TuiUserIntent> {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() {
            return None;
        }
        self.input.clear();
        Some(TuiUserIntent::SubmitPrompt { prompt })
    }

    pub fn apply_event(&mut self, frame: &EventFrame) {
        match &frame.event {
            RunEvent::UserMessageRecorded { item_id, text } => {
                self.messages.push(ChatMessage {
                    role: ChatMessageRole::User,
                    content: text.clone(),
                    item_id: Some(item_id.clone()),
                    streaming: false,
                });
            }
            RunEvent::AssistantMessageStarted { item_id } => {
                self.push_empty_streaming_message(ChatMessageRole::Assistant, item_id.clone());
            }
            RunEvent::AssistantDelta { item_id, text } => {
                self.append_to_streaming_message(ChatMessageRole::Assistant, item_id, text);
            }
            RunEvent::AssistantReasoningDelta { item_id, text } => {
                if self.reasoning_visible {
                    self.append_to_streaming_message(ChatMessageRole::Reasoning, item_id, text);
                }
            }
            RunEvent::AssistantMessageCompleted { item_id } => {
                for message in self.messages.iter_mut().filter(|message| {
                    message.item_id.as_ref() == Some(item_id)
                        && matches!(
                            message.role,
                            ChatMessageRole::Assistant | ChatMessageRole::Reasoning
                        )
                }) {
                    message.streaming = false;
                }
            }
            _ => {}
        }
    }

    fn push_empty_streaming_message(&mut self, role: ChatMessageRole, item_id: ItemId) {
        self.messages.push(ChatMessage {
            role,
            content: String::new(),
            item_id: Some(item_id),
            streaming: true,
        });
    }

    fn append_to_streaming_message(&mut self, role: ChatMessageRole, item_id: &ItemId, text: &str) {
        if let Some(message) = self.message_by_item_id_and_role_mut(item_id, &role) {
            message.content.push_str(text);
            message.streaming = true;
            return;
        }

        self.messages.push(ChatMessage {
            role,
            content: text.to_string(),
            item_id: Some(item_id.clone()),
            streaming: true,
        });
    }

    fn message_by_item_id_and_role_mut(
        &mut self,
        item_id: &ItemId,
        role: &ChatMessageRole,
    ) -> Option<&mut ChatMessage> {
        self.messages
            .iter_mut()
            .rev()
            .find(|message| message.item_id.as_ref() == Some(item_id) && message.role == *role)
    }
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

pub fn chat_window_lines(state: &ChatViewState) -> Vec<Line<'static>> {
    let mut lines = if state.messages.is_empty() {
        vec![Line::from(Span::raw("No messages yet"))]
    } else {
        state
            .messages
            .iter()
            .map(|message| {
                let role = match message.role {
                    ChatMessageRole::User => "You",
                    ChatMessageRole::Assistant => "Assistant",
                    ChatMessageRole::Reasoning => "Reasoning",
                };
                Line::from(vec![
                    Span::raw(role),
                    Span::raw(": "),
                    Span::raw(message.content.clone()),
                ])
            })
            .collect()
    };

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(vec![
        Span::raw("> "),
        Span::raw(state.input.clone()),
    ]));
    lines
}
