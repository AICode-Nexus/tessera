use tessera_protocol::{EventFrame, RunEvent, TaskId, TaskKind};
use tessera_storage::TraceStore;

#[test]
fn trace_store_appends_jsonl_and_indexes_events() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();
    let task_id = TaskId::new();

    store
        .append(&EventFrame::new(
            "trace_store",
            1,
            RunEvent::TaskCreated {
                task_id: task_id.clone(),
                kind: TaskKind::Chat,
            },
        ))
        .unwrap();
    store
        .append(&EventFrame::new(
            "trace_store",
            2,
            RunEvent::TaskCompleted { task_id },
        ))
        .unwrap();

    let events = store.list_events("trace_store").unwrap();
    assert_eq!(events, vec!["task_created", "task_completed"]);

    let jsonl = std::fs::read_to_string(temp.path().join("traces/trace_store.jsonl")).unwrap();
    assert!(jsonl.contains("\"event_kind\":\"task_created\""));
    assert!(jsonl.contains("\"event_kind\":\"task_completed\""));
}
