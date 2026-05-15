use async_trait::async_trait;
use futures::stream;
use std::time::Duration;
use tessera_core::{
    ConversationEngine, ConversationRequest, CoreError, EventSinkAction, ReplayRunner, RunControls,
};
use tessera_protocol::{
    ErrorSource, ModelProfileId, NormalizedError, ProviderCapability, ProviderId, RunEvent,
};
use tessera_providers::{
    mock::MockProvider, ChatProvider, ProviderError, ProviderEventStream, ProviderRequest,
};
use tessera_storage::TraceStore;

#[derive(Clone, Debug)]
struct HangingProvider;

#[async_trait]
impl ChatProvider for HangingProvider {
    async fn capability(&self) -> tessera_providers::Result<ProviderCapability> {
        Ok(ProviderCapability {
            provider_id: ProviderId::from_static("hanging"),
            supports_streaming: true,
            supports_reasoning_delta: false,
            supports_cache_telemetry: false,
            supports_cost_estimate: false,
            supports_tool_calling: false,
            max_context_tokens: Some(1024),
            extension: None,
        })
    }

    async fn stream_chat(
        &self,
        _request: ProviderRequest,
    ) -> tessera_providers::Result<ProviderEventStream> {
        Ok(Box::pin(stream::pending()))
    }
}

#[derive(Clone, Debug)]
struct FailingProvider;

#[async_trait]
impl ChatProvider for FailingProvider {
    async fn capability(&self) -> tessera_providers::Result<ProviderCapability> {
        Ok(ProviderCapability {
            provider_id: ProviderId::from_static("failing"),
            supports_streaming: true,
            supports_reasoning_delta: false,
            supports_cache_telemetry: false,
            supports_cost_estimate: false,
            supports_tool_calling: false,
            max_context_tokens: Some(1024),
            extension: None,
        })
    }

    async fn stream_chat(
        &self,
        _request: ProviderRequest,
    ) -> tessera_providers::Result<ProviderEventStream> {
        Err(ProviderError::Normalized(NormalizedError {
            code: "provider_rate_limited".to_string(),
            message: "provider rate limit reached".to_string(),
            retryable: true,
            source: ErrorSource::Provider,
            details: None,
        }))
    }
}

#[tokio::test]
async fn conversation_engine_drives_mock_provider_and_persists_trace() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello from core"))
        .await
        .unwrap();

    assert!(outcome.assistant_text.contains("mock response"));
    assert_eq!(outcome.trace_id, "trace_mock");

    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert_eq!(events.first().map(String::as_str), Some("task_created"));
    assert!(events.contains(&"provider_capability_reported".to_string()));
    assert!(events.contains(&"route_decision_recorded".to_string()));
    assert!(events.contains(&"assistant_reasoning_delta".to_string()));
    assert!(events.contains(&"usage_reported".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));

    let records = outcome.store.read_trace_records(&outcome.trace_id).unwrap();
    let user_message = records
        .iter()
        .find(|record| record.event_kind == "user_message_recorded")
        .unwrap();
    assert_eq!(user_message.payload["text"], "hello from core");
}

#[tokio::test]
async fn conversation_engine_records_normalized_provider_errors_before_returning_failure() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(FailingProvider, store);
    let mut request = ConversationRequest::mock("hello failure");
    request.trace_id = "trace_provider_failure".to_string();
    request.provider_id = ProviderId::from_static("failing");
    request.profile_id = ModelProfileId::from_static("failing-default");
    request.model = "failing-chat".to_string();

    let result = engine.run_chat(request).await;

    assert!(matches!(result, Err(CoreError::Provider(_))));
    let store = TraceStore::open(temp.path()).unwrap();
    let records = store.read_trace_records("trace_provider_failure").unwrap();
    let event_kinds = records
        .iter()
        .map(|record| record.event_kind.as_str())
        .collect::<Vec<_>>();

    assert!(event_kinds.contains(&"error"));
    assert!(event_kinds.contains(&"task_failed"));
    assert_eq!(event_kinds.last(), Some(&"done"));

    let error_record = records
        .iter()
        .find(|record| record.event_kind == "error")
        .unwrap();
    assert_eq!(
        error_record.payload["error"]["code"],
        "provider_rate_limited"
    );
    assert_eq!(error_record.payload["error"]["retryable"], true);
}

#[tokio::test]
async fn conversation_engine_streams_event_frames_to_live_sink_while_persisting_trace() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);
    let mut live_events = Vec::new();

    let outcome = engine
        .run_chat_with_event_sink(ConversationRequest::mock("hello live"), |frame| {
            live_events.push(frame.clone());
        })
        .await
        .unwrap();

    assert!(live_events
        .iter()
        .any(|frame| matches!(frame.event, RunEvent::AssistantDelta { .. })));
    assert_eq!(live_events.last().unwrap().event.kind(), "done");

    let persisted_events = outcome.store.list_events(&outcome.trace_id).unwrap();
    let live_event_kinds = live_events
        .iter()
        .map(|frame| frame.event.kind().to_string())
        .collect::<Vec<_>>();
    assert_eq!(live_event_kinds, persisted_events);
}

#[tokio::test]
async fn conversation_engine_records_cancellation_when_live_sink_requests_stop() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);
    let mut live_events = Vec::new();

    let outcome = engine
        .run_chat_with_event_sink(ConversationRequest::mock("hello cancel"), |frame| {
            live_events.push(frame.clone());
            match frame.event {
                RunEvent::AssistantMessageStarted { .. } => {
                    EventSinkAction::Cancel("live client stopped".to_string())
                }
                _ => EventSinkAction::Continue,
            }
        })
        .await
        .unwrap();

    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));
}

#[tokio::test]
async fn conversation_engine_records_timeout_when_provider_stalls() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(HangingProvider, store);
    let request = ConversationRequest {
        trace_id: "trace_timeout".to_string(),
        provider_id: ProviderId::from_static("hanging"),
        profile_id: ModelProfileId::from_static("hanging"),
        model: "hanging-model".to_string(),
        prompt: "hello timeout".to_string(),
    };

    let outcome = engine
        .run_chat_with_controls_and_event_sink(
            request,
            RunControls {
                event_timeout: Some(Duration::from_millis(5)),
            },
            |_| {},
        )
        .await
        .unwrap();

    let events = outcome.store.list_events(&outcome.trace_id).unwrap();
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
    assert_eq!(events.last().map(String::as_str), Some("done"));
}

#[tokio::test]
async fn replay_runner_reconstructs_mock_assistant_text_from_trace() {
    let temp = tempfile::tempdir().unwrap();
    let store = TraceStore::open(temp.path()).unwrap();
    let engine = ConversationEngine::new(MockProvider::default(), store);

    let outcome = engine
        .run_chat(ConversationRequest::mock("hello replay"))
        .await
        .unwrap();

    let replay = ReplayRunner::new(&outcome.store)
        .replay(&outcome.trace_id)
        .unwrap();

    assert!(replay.assistant_text.contains("mock response"));
    assert!(replay.event_kinds.contains(&"assistant_delta".to_string()));
    assert!(replay.event_kinds.contains(&"usage_reported".to_string()));
}

#[test]
fn replay_runner_accepts_golden_trace_fixture() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("traces")).unwrap();
    std::fs::write(
        temp.path().join("traces/trace_golden.jsonl"),
        include_str!("fixtures/mock_trace.jsonl"),
    )
    .unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();

    store.rebuild_index("trace_golden").unwrap();
    let replay = ReplayRunner::new(&store).replay("trace_golden").unwrap();
    let events = store.list_events("trace_golden").unwrap();

    assert_eq!(replay.assistant_text, "golden hello");
    assert_eq!(events, vec!["assistant_delta", "usage_reported", "done"]);
}
