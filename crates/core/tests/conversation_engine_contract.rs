use tessera_core::{ConversationEngine, ConversationRequest, ReplayRunner};
use tessera_providers::mock::MockProvider;
use tessera_storage::TraceStore;

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
