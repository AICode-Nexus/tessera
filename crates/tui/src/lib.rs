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
pub use tessera_client::{
    ClientIntent, ClientMessage, ClientMessageRole as ChatMessageRole,
    ClientSnapshot as ChatViewState,
};
use tessera_protocol::EventFrame;
use tokio::sync::mpsc;

pub type TuiUserIntent = ClientIntent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalInput {
    Char(char),
    Backspace,
    NextProfile,
    PreviousProfile,
    Submit,
    Quit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalAction {
    Render,
    Dispatch(ClientIntent),
    Quit,
    Ignore,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LiveClientEvent {
    Frame(Box<EventFrame>),
    Error(String),
}

pub const LIVE_EVENT_BUFFER_CAPACITY: usize = 128;

pub type LiveClientEventSender = mpsc::Sender<LiveClientEvent>;
pub type LiveClientEventReceiver = mpsc::Receiver<LiveClientEvent>;

pub fn live_client_event_channel(
    capacity: usize,
) -> (LiveClientEventSender, LiveClientEventReceiver) {
    mpsc::channel(capacity)
}

pub fn status_line(state: &ChatViewState) -> Line<'static> {
    let reasoning = if state.status.reasoning_visible {
        "reasoning:on"
    } else {
        "reasoning:off"
    };
    let (profile_index, profile_total) = state.status.active_profile_position();
    Line::from(vec![
        Span::raw("profile "),
        Span::raw(state.status.active_profile.clone()),
        Span::raw(format!(" [{profile_index}/{profile_total}]")),
        Span::raw(" | "),
        Span::raw(reasoning),
        Span::raw(" | "),
        Span::raw(state.status.task_summary.clone()),
        Span::raw(" | "),
        Span::raw(state.status.context_summary.clone()),
        Span::raw(" | "),
        Span::raw(state.status.usage_summary.clone()),
        Span::raw(" | "),
        Span::raw(state.status.cache_summary.clone()),
        Span::raw(" | "),
        Span::raw(state.status.cost_summary.clone()),
    ])
}

pub fn chat_window_lines(state: &ChatViewState) -> Vec<Line<'static>> {
    let mut lines = if state.projection.messages.is_empty() {
        vec![Line::from(Span::raw("No messages yet"))]
    } else {
        state
            .projection
            .messages
            .iter()
            .map(|message| {
                let role = match message.role {
                    ChatMessageRole::System => "System",
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
        Span::raw(state.draft_input.clone()),
    ]));
    lines
}

pub fn handle_terminal_input(state: &mut ChatViewState, input: TerminalInput) -> TerminalAction {
    match input {
        TerminalInput::Char(character) => {
            state.draft_input.push(character);
            TerminalAction::Render
        }
        TerminalInput::Backspace => {
            state.draft_input.pop();
            TerminalAction::Render
        }
        TerminalInput::NextProfile => state
            .cycle_profile(1)
            .map(TerminalAction::Dispatch)
            .unwrap_or(TerminalAction::Ignore),
        TerminalInput::PreviousProfile => state
            .cycle_profile(-1)
            .map(TerminalAction::Dispatch)
            .unwrap_or(TerminalAction::Ignore),
        TerminalInput::Submit => state
            .submit_input()
            .map(TerminalAction::Dispatch)
            .unwrap_or(TerminalAction::Ignore),
        TerminalInput::Quit => TerminalAction::Quit,
    }
}

pub fn apply_live_event(state: &mut ChatViewState, event: LiveClientEvent) {
    match event {
        LiveClientEvent::Frame(frame) => state.apply_event(&frame),
        LiveClientEvent::Error(error) => state.projection.messages.push(ClientMessage {
            role: ChatMessageRole::Assistant,
            content: format!("Error: {error}"),
            item_id: None,
            streaming: false,
        }),
    }
}

pub fn apply_client_intent_locally(state: &mut ChatViewState, intent: &ClientIntent) -> bool {
    match intent {
        ClientIntent::NewThread => {
            state.start_new_thread();
            true
        }
        ClientIntent::SaveThread => {
            state.push_notice("Saved locally. Runtime traces are persisted automatically.");
            true
        }
        ClientIntent::ExportThread => {
            let export = state.export_markdown();
            state.push_notice(format!(
                "Export prepared ({} bytes markdown).",
                export.len()
            ));
            true
        }
        ClientIntent::SwitchProfile { .. } => true,
        ClientIntent::SubmitPrompt { .. } | ClientIntent::CancelTask { .. } => false,
    }
}

pub fn map_key_event(event: KeyEvent) -> Option<TerminalInput> {
    match event.code {
        KeyCode::Enter => Some(TerminalInput::Submit),
        KeyCode::Backspace => Some(TerminalInput::Backspace),
        KeyCode::Tab => Some(TerminalInput::NextProfile),
        KeyCode::BackTab => Some(TerminalInput::PreviousProfile),
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
        Span::raw(state.draft_input.clone()),
    ]))
    .block(Block::default().title("Input").borders(Borders::ALL));
    frame.render_widget(input, chunks[2]);
}

pub async fn run_terminal_chat<F, Fut>(
    initial_state: ChatViewState,
    mut submit_prompt: F,
) -> io::Result<ChatViewState>
where
    F: FnMut(String, String, LiveClientEventSender) -> Fut,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
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
    F: FnMut(String, String, LiveClientEventSender) -> Fut,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    let (live_event_tx, mut live_event_rx) = live_client_event_channel(LIVE_EVENT_BUFFER_CAPACITY);

    loop {
        while let Ok(event) = live_event_rx.try_recv() {
            apply_live_event(&mut state, event);
        }

        terminal.draw(|frame| draw_terminal_frame(frame, &state))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        let Event::Key(key_event) = event::read()? else {
            continue;
        };
        let Some(input) = map_key_event(key_event) else {
            continue;
        };

        match handle_terminal_input(&mut state, input) {
            TerminalAction::Render | TerminalAction::Ignore => {}
            TerminalAction::Quit => return Ok(state),
            TerminalAction::Dispatch(intent)
                if apply_client_intent_locally(&mut state, &intent) => {}
            TerminalAction::Dispatch(intent) => match intent {
                ClientIntent::SubmitPrompt { profile_id, prompt } => {
                    let submit_result_tx = live_event_tx.clone();
                    let submit_events_tx = live_event_tx.clone();
                    let submit = submit_prompt(profile_id, prompt, submit_events_tx);
                    tokio::spawn(async move {
                        if let Err(error) = submit.await {
                            let _ = submit_result_tx.try_send(LiveClientEvent::Error(error));
                        }
                    });
                }
                ClientIntent::SwitchProfile { .. }
                | ClientIntent::NewThread
                | ClientIntent::SaveThread
                | ClientIntent::ExportThread
                | ClientIntent::CancelTask { .. } => {}
            },
        }
    }
}

fn message_lines(state: &ChatViewState) -> Vec<Line<'static>> {
    if state.projection.messages.is_empty() {
        return vec![Line::from(Span::raw("No messages yet"))];
    }

    state
        .projection
        .messages
        .iter()
        .map(|message| {
            let role = match message.role {
                ChatMessageRole::System => "System",
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
