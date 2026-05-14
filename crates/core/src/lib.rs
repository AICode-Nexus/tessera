use futures::TryStreamExt;
use tessera_protocol::{
    EventFrame, ItemId, ModelProfileId, ProviderId, RouteDecision, RouteDecisionId, RouteStrategy,
    RunEvent, TaskId, TaskKind, ThreadId, TurnId,
};
use tessera_providers::{ChatProvider, ProviderRequest};
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

    pub async fn run_chat(mut self, request: ConversationRequest) -> Result<ConversationOutcome> {
        let thread_id = ThreadId::new();
        let turn_id = TurnId::new();
        let task_id = TaskId::new();
        let user_item_id = ItemId::new();
        let assistant_item_id = ItemId::new();
        let mut seq = 1;
        let mut assistant_text = String::new();

        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::TaskCreated {
                task_id: task_id.clone(),
                kind: TaskKind::Chat,
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::TaskStarted {
                task_id: task_id.clone(),
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::ThreadCreated {
                thread_id: thread_id.clone(),
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::TurnStarted {
                turn_id: turn_id.clone(),
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::UserMessageRecorded {
                item_id: user_item_id,
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::ProviderRequestStarted {
                provider_id: request.provider_id.clone(),
                profile_id: request.profile_id.clone(),
                model: request.model.clone(),
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;

        let capability = self.provider.capability().await?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::ProviderCapabilityReported {
                provider_id: request.provider_id.clone(),
                capability,
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::RouteDecisionRecorded {
                decision_id: RouteDecisionId::new(),
                decision: RouteDecision {
                    requested_profile: Some(request.profile_id.clone()),
                    selected_profile: request.profile_id.clone(),
                    selected_model: request.model.clone(),
                    reasoning_level: None,
                    strategy: RouteStrategy::Manual,
                    fallback_reason: None,
                },
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;

        let mut stream = self
            .provider
            .stream_chat(ProviderRequest {
                provider_id: request.provider_id.clone(),
                profile_id: request.profile_id.clone(),
                model: request.model.clone(),
                prompt: request.prompt,
                assistant_item_id,
            })
            .await?;

        while let Some(event) = stream.try_next().await? {
            if let RunEvent::AssistantDelta { text, .. } = &event {
                assistant_text.push_str(text);
            }
            self.append_contextual(
                &request.trace_id,
                &mut seq,
                event,
                &thread_id,
                &turn_id,
                &task_id,
            )?;
        }

        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::ProviderRequestCompleted {
                provider_id: request.provider_id,
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::TurnCompleted {
                turn_id: turn_id.clone(),
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::TaskCompleted {
                task_id: task_id.clone(),
            },
            &thread_id,
            &turn_id,
            &task_id,
        )?;
        self.append_contextual(
            &request.trace_id,
            &mut seq,
            RunEvent::Done,
            &thread_id,
            &turn_id,
            &task_id,
        )?;

        Ok(ConversationOutcome {
            trace_id: request.trace_id,
            assistant_text,
            store: self.store,
        })
    }

    fn append_contextual(
        &mut self,
        trace_id: &str,
        seq: &mut u64,
        event: RunEvent,
        thread_id: &ThreadId,
        turn_id: &TurnId,
        task_id: &TaskId,
    ) -> Result<()> {
        let item_id = event.item_id();
        let event_turn_id = event.turn_id();
        let event_task_id = event.task_id();
        let mut frame = EventFrame::new(trace_id, *seq, event)
            .with_thread_id(thread_id.clone())
            .with_turn_id(event_turn_id.unwrap_or_else(|| turn_id.clone()))
            .with_task_id(event_task_id.unwrap_or_else(|| task_id.clone()));

        if let Some(item_id) = item_id {
            frame = frame.with_item_id(item_id);
        }

        self.store.append(&frame)?;
        *seq += 1;
        Ok(())
    }
}
