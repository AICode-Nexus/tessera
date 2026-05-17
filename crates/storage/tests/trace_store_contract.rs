use tessera_protocol::{
    ArtifactId, ArtifactKind, EventFrame, ItemId, RunEvent, TaskId, TaskKind, ThreadId, TurnId,
};
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

#[test]
fn trace_store_lists_indexed_runtime_object_ids() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();
    let thread_id = ThreadId::new();
    let turn_id = TurnId::new();
    let item_id = ItemId::new();
    let task_id = TaskId::new();
    let artifact_id = ArtifactId::new();

    store
        .append(
            &EventFrame::new(
                "trace_objects",
                1,
                RunEvent::TaskCreated {
                    task_id: task_id.clone(),
                    kind: TaskKind::Chat,
                },
            )
            .with_thread_id(thread_id.clone())
            .with_turn_id(turn_id.clone())
            .with_item_id(item_id.clone())
            .with_task_id(task_id.clone())
            .with_artifact_ref(artifact_id.clone()),
        )
        .unwrap();
    store
        .append(
            &EventFrame::new(
                "trace_objects",
                2,
                RunEvent::ArtifactCreated {
                    artifact_id: artifact_id.clone(),
                    kind: ArtifactKind::Trace,
                },
            )
            .with_thread_id(thread_id.clone())
            .with_turn_id(turn_id.clone())
            .with_task_id(task_id.clone()),
        )
        .unwrap();

    let objects = store.list_indexed_objects("trace_objects").unwrap();

    assert_eq!(objects.threads, vec![thread_id]);
    assert_eq!(objects.turns, vec![turn_id]);
    assert_eq!(objects.items, vec![item_id]);
    assert_eq!(objects.tasks, vec![task_id]);
    assert_eq!(objects.artifacts, vec![artifact_id]);
}

#[test]
fn trace_store_rebuilds_sqlite_index_from_jsonl_trace() {
    let temp = tempfile::tempdir().unwrap();
    let task_id = TaskId::new();
    {
        let mut store = TraceStore::open(temp.path()).unwrap();
        store
            .append(&EventFrame::new(
                "trace_rebuild",
                1,
                RunEvent::TaskCreated {
                    task_id: task_id.clone(),
                    kind: TaskKind::Chat,
                },
            ))
            .unwrap();
        store
            .append(&EventFrame::new(
                "trace_rebuild",
                2,
                RunEvent::TaskCompleted { task_id },
            ))
            .unwrap();
    }

    std::fs::remove_file(temp.path().join("tessera.sqlite3")).unwrap();
    let mut rebuilt = TraceStore::open(temp.path()).unwrap();
    rebuilt.rebuild_index("trace_rebuild").unwrap();

    assert_eq!(
        rebuilt.list_events("trace_rebuild").unwrap(),
        vec!["task_created", "task_completed"]
    );
}

#[test]
fn trace_store_lists_trace_ids_from_jsonl_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = TraceStore::open(temp.path()).unwrap();

    store
        .append(&EventFrame::new(
            "trace_beta",
            1,
            RunEvent::AssistantDelta {
                item_id: ItemId::from_static("item_beta"),
                text: "beta".to_string(),
            },
        ))
        .unwrap();
    store
        .append(&EventFrame::new(
            "trace_alpha",
            1,
            RunEvent::AssistantDelta {
                item_id: ItemId::from_static("item_alpha"),
                text: "alpha".to_string(),
            },
        ))
        .unwrap();
    std::fs::write(temp.path().join("traces/not-a-trace.txt"), "ignore me").unwrap();

    assert_eq!(
        store.list_trace_ids().unwrap(),
        vec!["trace_alpha".to_string(), "trace_beta".to_string()]
    );
}
