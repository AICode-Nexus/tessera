use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{future::Future, io, time::Duration};
use tessera_protocol::{EventFrame, ItemId, RunEvent, TraceRecord};

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
pub enum TerminalInput {
    Char(char),
    Backspace,
    Submit,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalAction {
    Render,
    Submit(TuiUserIntent),
    Quit,
    Ignore,
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

    pub fn handle_terminal_input(&mut self, input: TerminalInput) -> TerminalAction {
        match input {
            TerminalInput::Char(character) => {
                self.input.push(character);
                TerminalAction::Render
            }
            TerminalInput::Backspace => {
                self.input.pop();
                TerminalAction::Render
            }
            TerminalInput::Submit => self
                .submit_input()
                .map(TerminalAction::Submit)
                .unwrap_or(TerminalAction::Ignore),
            TerminalInput::Quit => TerminalAction::Quit,
        }
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

    pub fn apply_trace_record(&mut self, record: &TraceRecord) {
        let item_id = trace_record_item_id(record);
        match record.event_kind.as_str() {
            "user_message_recorded" => {
                let Some(text) = record.payload.get("text").and_then(|value| value.as_str()) else {
                    return;
                };
                self.messages.push(ChatMessage {
                    role: ChatMessageRole::User,
                    content: text.to_string(),
                    item_id,
                    streaming: false,
                });
            }
            "assistant_message_started" => {
                let Some(item_id) = item_id else {
                    return;
                };
                self.push_empty_streaming_message(ChatMessageRole::Assistant, item_id);
            }
            "assistant_delta" => {
                let (Some(item_id), Some(text)) = (
                    item_id.as_ref(),
                    record.payload.get("text").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                self.append_to_streaming_message(ChatMessageRole::Assistant, item_id, text);
            }
            "assistant_reasoning_delta" => {
                let (Some(item_id), Some(text)) = (
                    item_id.as_ref(),
                    record.payload.get("text").and_then(|value| value.as_str()),
                ) else {
                    return;
                };
                if self.reasoning_visible {
                    self.append_to_streaming_message(ChatMessageRole::Reasoning, item_id, text);
                }
            }
            "assistant_message_completed" => {
                let Some(item_id) = item_id else {
                    return;
                };
                self.complete_assistant_item(&item_id);
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

    fn complete_assistant_item(&mut self, item_id: &ItemId) {
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

pub fn map_key_event(event: KeyEvent) -> Option<TerminalInput> {
    match event.code {
        KeyCode::Enter => Some(TerminalInput::Submit),
        KeyCode::Backspace => Some(TerminalInput::Backspace),
        KeyCode::Esc => Some(TerminalInput::Quit),
        KeyCode::Char('c') if event.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TerminalInput::Quit)
        }
        KeyCode::Char(character) if !event.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(TerminalInput::Char(character))
        }
        _ => None,
    }
}

pub fn draw_terminal_frame(frame: &mut Frame<'_>, state: &ChatViewState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(frame.area());

    frame.render_widget(Paragraph::new(status_line(state)), chunks[0]);

    let messages = Paragraph::new(message_lines(state))
        .block(Block::default().title("Chat").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(messages, chunks[1]);

    let input = Paragraph::new(Line::from(vec![
        Span::raw("> "),
        Span::raw(state.input.clone()),
    ]))
    .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, chunks[2]);
}

pub async fn run_terminal_chat<F, Fut>(
    initial_state: ChatViewState,
    mut submit_prompt: F,
) -> io::Result<ChatViewState>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Vec<TraceRecord>, String>>,
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_terminal_chat_loop(&mut terminal, initial_state, &mut submit_prompt).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_terminal_chat_loop<B, F, Fut>(
    terminal: &mut Terminal<B>,
    mut state: ChatViewState,
    submit_prompt: &mut F,
) -> io::Result<ChatViewState>
where
    B: ratatui::backend::Backend,
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Vec<TraceRecord>, String>>,
{
    loop {
        terminal.draw(|frame| draw_terminal_frame(frame, &state))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        let Event::Key(key_event) = event::read()? else {
            continue;
        };
        let Some(input) = map_key_event(key_event) else {
            continue;
        };

        match state.handle_terminal_input(input) {
            TerminalAction::Render | TerminalAction::Ignore => {}
            TerminalAction::Quit => return Ok(state),
            TerminalAction::Submit(TuiUserIntent::SubmitPrompt { prompt }) => {
                match submit_prompt(prompt).await {
                    Ok(records) => {
                        for record in records {
                            state.apply_trace_record(&record);
                        }
                    }
                    Err(error) => state.messages.push(ChatMessage {
                        role: ChatMessageRole::Assistant,
                        content: format!("Error: {error}"),
                        item_id: None,
                        streaming: false,
                    }),
                }
            }
        }
    }
}

fn message_lines(state: &ChatViewState) -> Vec<Line<'static>> {
    if state.messages.is_empty() {
        return vec![Line::from(Span::raw("No messages yet"))];
    }

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
}

fn trace_record_item_id(record: &TraceRecord) -> Option<ItemId> {
    record.item_id.clone().or_else(|| {
        record
            .payload
            .get("item_id")
            .and_then(|value| value.as_str())
            .map(ItemId::from)
    })
}
