use futures::TryStreamExt;
use std::time::Duration;
use tessera_protocol::{
    EventFrame, ItemId, ModelProfileId, ProviderId, RouteDecision, RouteDecisionId, RouteStrategy,
    RunEvent, TaskId, TaskKind, ThreadId, TurnId,
};
use tessera_providers::{ChatProvider, ProviderError, ProviderRequest};
use tessera_storage::TraceStore;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("provider failed: {0}")]
    Provider(#[from] tessera_providers::ProviderError),
    #[error("storage failed: {0}")]
    Storage(#[from] tessera_storage::StorageError),
}

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventSinkAction {
    Continue,
    Cancel(String),
}

impl EventSinkAction {
    fn cancel_reason(self) -> Option<String> {
        match self {
            Self::Continue => None,
            Self::Cancel(reason) => Some(reason),
        }
    }
}

impl From<()> for EventSinkAction {
    fn from(_: ()) -> Self {
        Self::Continue
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RunControls {
    pub event_timeout: Option<Duration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationRequest {
    pub trace_id: String,
    pub provider_id: ProviderId,
    pub profile_id: ModelProfileId,
    pub model: String,
    pub prompt: String,
}

impl ConversationRequest {
    pub fn mock(prompt: impl Into<String>) -> Self {
        Self {
            trace_id: "trace_mock".to_string(),
            provider_id: ProviderId::from_static("mock"),
            profile_id: ModelProfileId::from_static("mock-default"),
            model: "mock-chat".to_string(),
            prompt: prompt.into(),
        }
    }
}

pub struct ConversationOutcome {
    pub trace_id: String,
    pub assistant_text: String,
    pub store: TraceStore,
}

pub struct ConversationEngine<P> {
    provider: P,
    store: TraceStore,
}

struct RunContext {
    trace_id: String,
    thread_id: ThreadId,
    turn_id: TurnId,
    task_id: TaskId,
    seq: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplaySummary {
    pub trace_id: String,
    pub assistant_text: String,
    pub event_kinds: Vec<String>,
}

pub struct ReplayRunner<'a> {
    store: &'a TraceStore,
}

impl<'a> ReplayRunner<'a> {
    pub fn new(store: &'a TraceStore) -> Self {
        Self { store }
    }

    pub fn replay(&self, trace_id: &str) -> Result<ReplaySummary> {
        let records = self.store.read_trace_records(trace_id)?;
        let mut assistant_text = String::new();
        let mut event_kinds = Vec::new();

        for record in records {
            if record.event_kind == "assistant_delta" {
                if let Some(text) = record.payload.get("text").and_then(|value| value.as_str()) {
                    assistant_text.push_str(text);
                }
            }
            event_kinds.push(record.event_kind);
        }

        Ok(ReplaySummary {
            trace_id: trace_id.to_string(),
            assistant_text,
            event_kinds,
        })
    }
}

impl<P> ConversationEngine<P>
where
    P: ChatProvider,
{
    pub fn new(provider: P, store: TraceStore) -> Self {
        Self { provider, store }
    }

    pub async fn run_chat(self, request: ConversationRequest) -> Result<ConversationOutcome> {
        self.run_chat_with_event_sink(request, |_| {}).await
    }

    pub async fn run_chat_with_event_sink<F, R>(
        self,
        request: ConversationRequest,
        event_sink: F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        self.run_chat_with_controls_and_event_sink(request, RunControls::default(), event_sink)
            .await
    }

    pub async fn run_chat_with_controls_and_event_sink<F, R>(
        mut self,
        request: ConversationRequest,
        controls: RunControls,
        mut event_sink: F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let trace_id = request.trace_id.clone();
        let mut context = RunContext {
            trace_id: trace_id.clone(),
            thread_id: ThreadId::new(),
            turn_id: TurnId::new(),
            task_id: TaskId::new(),
            seq: 1,
        };
        let user_item_id = ItemId::new();
        let assistant_item_id = ItemId::new();
        let mut assistant_text = String::new();
        let prompt = request.prompt.clone();

        macro_rules! append_event {
            ($event:expr) => {{
                let action = self.append_contextual(&mut context, $event, &mut event_sink)?;
                if let Some(reason) = action.cancel_reason() {
                    return self.finish_cancelled(
                        trace_id,
                        assistant_text,
                        &mut context,
                        reason,
                        &mut event_sink,
                    );
                }
            }};
        }

        let task_id = context.task_id.clone();
        append_event!(RunEvent::TaskCreated {
            task_id,
            kind: TaskKind::Chat,
        });
        let task_id = context.task_id.clone();
        append_event!(RunEvent::TaskStarted { task_id });
        let thread_id = context.thread_id.clone();
        append_event!(RunEvent::ThreadCreated { thread_id });
        let turn_id = context.turn_id.clone();
        append_event!(RunEvent::TurnStarted { turn_id });
        append_event!(RunEvent::UserMessageRecorded {
            item_id: user_item_id,
            text: prompt,
        });
        append_event!(RunEvent::ProviderRequestStarted {
            provider_id: request.provider_id.clone(),
            profile_id: request.profile_id.clone(),
            model: request.model.clone(),
        });

        let capability = match self.provider.capability().await {
            Ok(capability) => capability,
            Err(error) => {
                return self.finish_failed(&mut context, error, &mut event_sink);
            }
        };
        append_event!(RunEvent::ProviderCapabilityReported {
            provider_id: request.provider_id.clone(),
            capability,
        });
        append_event!(RunEvent::RouteDecisionRecorded {
            decision_id: RouteDecisionId::new(),
            decision: RouteDecision {
                requested_profile: Some(request.profile_id.clone()),
                selected_profile: request.profile_id.clone(),
                selected_model: request.model.clone(),
                reasoning_level: None,
                strategy: RouteStrategy::Manual,
                fallback_reason: None,
            },
        });

        let mut stream = match self
            .provider
            .stream_chat(ProviderRequest {
                provider_id: request.provider_id.clone(),
                profile_id: request.profile_id.clone(),
                model: request.model.clone(),
                prompt: request.prompt,
                assistant_item_id,
            })
            .await
        {
            Ok(stream) => stream,
            Err(error) => {
                return self.finish_failed(&mut context, error, &mut event_sink);
            }
        };

        loop {
            let next_event = if let Some(timeout) = controls.event_timeout {
                match tokio::time::timeout(timeout, stream.try_next()).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(error)) => {
                        return self.finish_failed(&mut context, error, &mut event_sink);
                    }
                    Err(_) => {
                        return self.finish_cancelled(
                            trace_id,
                            assistant_text,
                            &mut context,
                            format!("provider event timeout after {}ms", timeout.as_millis()),
                            &mut event_sink,
                        );
                    }
                }
            } else {
                match stream.try_next().await {
                    Ok(result) => result,
                    Err(error) => {
                        return self.finish_failed(&mut context, error, &mut event_sink);
                    }
                }
            };

            let Some(event) = next_event else {
                break;
            };

            if let RunEvent::AssistantDelta { text, .. } = &event {
                assistant_text.push_str(text);
            }
            append_event!(event);
        }

        append_event!(RunEvent::ProviderRequestCompleted {
            provider_id: request.provider_id,
        });
        let turn_id = context.turn_id.clone();
        append_event!(RunEvent::TurnCompleted { turn_id });
        let task_id = context.task_id.clone();
        append_event!(RunEvent::TaskCompleted { task_id });
        append_event!(RunEvent::Done);

        Ok(ConversationOutcome {
            trace_id,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_cancelled<F, R>(
        mut self,
        trace_id: String,
        assistant_text: String,
        context: &mut RunContext,
        reason: String,
        event_sink: &mut F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let task_id = context.task_id.clone();
        let _ = self.append_contextual(
            context,
            RunEvent::TaskCancelled {
                task_id,
                reason: Some(reason),
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;

        Ok(ConversationOutcome {
            trace_id,
            assistant_text,
            store: self.store,
        })
    }

    fn finish_failed<F, R>(
        mut self,
        context: &mut RunContext,
        error: ProviderError,
        event_sink: &mut F,
    ) -> Result<ConversationOutcome>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let normalized = error.normalized();
        let _ = self.append_contextual(
            context,
            RunEvent::Error {
                error: normalized.clone(),
            },
            event_sink,
        )?;
        let task_id = context.task_id.clone();
        let _ = self.append_contextual(
            context,
            RunEvent::TaskFailed {
                task_id,
                error: normalized,
            },
            event_sink,
        )?;
        let _ = self.append_contextual(context, RunEvent::Done, event_sink)?;

        Err(CoreError::Provider(error))
    }

    fn append_contextual<F, R>(
        &mut self,
        context: &mut RunContext,
        event: RunEvent,
        event_sink: &mut F,
    ) -> Result<EventSinkAction>
    where
        F: FnMut(&EventFrame) -> R,
        R: Into<EventSinkAction>,
    {
        let item_id = event.item_id();
        let event_turn_id = event.turn_id();
        let event_task_id = event.task_id();
        let mut frame = EventFrame::new(&context.trace_id, context.seq, event)
            .with_thread_id(context.thread_id.clone())
            .with_turn_id(event_turn_id.unwrap_or_else(|| context.turn_id.clone()))
            .with_task_id(event_task_id.unwrap_or_else(|| context.task_id.clone()));

        if let Some(item_id) = item_id {
            frame = frame.with_item_id(item_id);
        }

        self.store.append(&frame)?;
        let action = event_sink(&frame).into();
        context.seq += 1;
        Ok(action)
    }
}
