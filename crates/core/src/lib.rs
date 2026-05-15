use futures::TryStreamExt;
use std::time::Duration;
use tessera_protocol::{
    ArtifactId, EventFrame, ItemId, ModelProfileId, ProviderId, RouteDecision, RouteDecisionId,
    RouteStrategy, RunEvent, TaskId, TaskKind, TaskStatus, ThreadId, Timestamp, TraceRecord,
    TurnId,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEventQuery {
    pub trace_id: String,
    pub since_seq: Option<u64>,
    pub limit: Option<usize>,
}

impl RuntimeEventQuery {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            since_seq: None,
            limit: None,
        }
    }

    pub fn since_seq(mut self, seq: u64) -> Self {
        self.since_seq = Some(seq);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeEventPage {
    pub trace_id: String,
    pub records: Vec<TraceRecord>,
    pub next_since_seq: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeObjectIndex {
    pub threads: Vec<ThreadId>,
    pub turns: Vec<TurnId>,
    pub items: Vec<ItemId>,
    pub tasks: Vec<TaskId>,
    pub artifacts: Vec<ArtifactId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeTaskSummary {
    pub task_id: TaskId,
    pub kind: Option<TaskKind>,
    pub status: TaskStatus,
    pub thread_id: Option<ThreadId>,
    pub turn_id: Option<TurnId>,
    pub created_at: Option<Timestamp>,
    pub started_at: Option<Timestamp>,
    pub finished_at: Option<Timestamp>,
    pub cancel_reason: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl RuntimeTaskSummary {
    fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            kind: None,
            status: TaskStatus::Pending,
            thread_id: None,
            turn_id: None,
            created_at: None,
            started_at: None,
            finished_at: None,
            cancel_reason: None,
            error_code: None,
            error_message: None,
        }
    }

    fn update_scope(&mut self, thread_id: Option<ThreadId>, turn_id: Option<TurnId>) {
        if thread_id.is_some() {
            self.thread_id = thread_id;
        }
        if turn_id.is_some() {
            self.turn_id = turn_id;
        }
    }
}

pub struct RuntimeReader {
    store: TraceStore,
}

impl RuntimeReader {
    pub fn new(store: TraceStore) -> Self {
        Self { store }
    }

    pub fn list_events(&self, query: RuntimeEventQuery) -> Result<RuntimeEventPage> {
        let since_seq = query.since_seq.unwrap_or(0);
        let mut records = self
            .store
            .read_trace_records(&query.trace_id)?
            .into_iter()
            .filter(|record| record.seq > since_seq)
            .collect::<Vec<_>>();

        if let Some(limit) = query.limit {
            records.truncate(limit);
        }

        let next_since_seq = records.last().map(|record| record.seq);
        Ok(RuntimeEventPage {
            trace_id: query.trace_id,
            records,
            next_since_seq,
        })
    }

    pub fn list_objects(&self, trace_id: &str) -> Result<RuntimeObjectIndex> {
        let objects = self.store.list_indexed_objects(trace_id)?;
        Ok(RuntimeObjectIndex {
            threads: objects.threads,
            turns: objects.turns,
            items: objects.items,
            tasks: objects.tasks,
            artifacts: objects.artifacts,
        })
    }

    pub fn list_tasks(&self, trace_id: &str) -> Result<Vec<RuntimeTaskSummary>> {
        let records = self.store.read_trace_records(trace_id)?;
        let mut tasks = Vec::new();
        for record in records {
            apply_task_record(&mut tasks, &record);
        }
        Ok(tasks)
    }
}

fn apply_task_record(tasks: &mut Vec<RuntimeTaskSummary>, record: &TraceRecord) {
    match record.event_kind.as_str() {
        "task_created" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let kind = record
                .payload
                .get("kind")
                .and_then(|value| value.as_str())
                .and_then(TaskKind::from_snake_case);
            let task = task_mut_or_insert(tasks, &task_id);
            task.kind = kind;
            task.status = TaskStatus::Pending;
            task.created_at = Some(record.timestamp.clone());
            task.finished_at = None;
            task.cancel_reason = None;
            task.error_code = None;
            task.error_message = None;
            task.update_scope(record.thread_id.clone(), record.turn_id.clone());
        }
        "task_started" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Running;
            task.started_at = Some(record.timestamp.clone());
            task.update_scope(record.thread_id.clone(), record.turn_id.clone());
        }
        "task_completed" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Completed;
            task.finished_at = Some(record.timestamp.clone());
        }
        "task_failed" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Failed;
            task.finished_at = Some(record.timestamp.clone());
            task.error_code = record
                .payload
                .get("error")
                .and_then(|error| error.get("code"))
                .and_then(|value| value.as_str())
                .map(str::to_string);
            task.error_message = record
                .payload
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
        "task_cancelled" => {
            let Some(task_id) = trace_record_task_id(record) else {
                return;
            };
            let task = task_mut_or_insert(tasks, &task_id);
            task.status = TaskStatus::Cancelled;
            task.finished_at = Some(record.timestamp.clone());
            task.cancel_reason = record
                .payload
                .get("reason")
                .and_then(|value| value.as_str())
                .map(str::to_string);
        }
        _ => {}
    }
}

fn task_mut_or_insert<'a>(
    tasks: &'a mut Vec<RuntimeTaskSummary>,
    task_id: &TaskId,
) -> &'a mut RuntimeTaskSummary {
    if let Some(index) = tasks.iter().position(|task| &task.task_id == task_id) {
        return &mut tasks[index];
    }

    tasks.push(RuntimeTaskSummary::new(task_id.clone()));
    tasks
        .last_mut()
        .expect("task was just inserted into non-empty registry")
}

fn trace_record_task_id(record: &TraceRecord) -> Option<TaskId> {
    record.task_id.clone().or_else(|| {
        record
            .payload
            .get("task_id")
            .and_then(|value| value.as_str())
            .map(TaskId::from)
    })
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
