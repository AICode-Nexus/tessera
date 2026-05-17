use std::fs;

use tessera_client::{ClientIntent, ClientMessageRole};
use tessera_gui_bridge::{
    BoundedGuiEventBuffer, GuiBridge, GuiBridgeError, GuiEvent, GuiRuntimeMode,
};
use tessera_protocol::TaskId;

#[test]
fn gui_bridge_loads_mock_replay_snapshot_without_provider_or_storage_dependencies() {
    let manifest =
        fs::read_to_string(format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"))).unwrap();

    assert!(!manifest.contains("tessera-providers"));
    assert!(!manifest.contains("tessera-storage"));
    assert!(!manifest.contains("rusqlite"));
    assert!(!manifest.contains("reqwest"));

    let bridge = GuiBridge::new(8);
    let state = bridge.shell_state();

    assert_eq!(state.mode, GuiRuntimeMode::MockReplay);
    assert_eq!(state.event_buffer_capacity, 8);
    assert!(state
        .profiles
        .iter()
        .any(|profile| profile.id == "mock-replay"));
    assert!(state
        .snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.role == ClientMessageRole::Assistant
            && message.content.contains("mock/replay")));
}

#[test]
fn gui_bridge_submit_prompt_projects_mock_events_into_client_snapshot() {
    let mut bridge = GuiBridge::new(8);

    let outcome = bridge
        .submit_client_intent(ClientIntent::SubmitPrompt {
            profile_id: "mock-replay".to_string(),
            prompt: "hello gui".to_string(),
        })
        .unwrap();

    assert!(outcome.accepted);
    assert!(outcome.notice.is_some());
    assert!(outcome
        .snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.role == ClientMessageRole::User && message.content == "hello gui"));
    assert!(outcome
        .snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.role == ClientMessageRole::Assistant
            && message.content.contains("mock/replay")));
}

#[test]
fn gui_bridge_cancel_task_is_typed_but_does_not_execute_runtime_work() {
    let mut bridge = GuiBridge::new(8);

    let outcome = bridge
        .cancel_task(Some(TaskId::from_static("task_gui_mock")))
        .unwrap();

    assert!(outcome.accepted);
    assert!(outcome.notice.as_deref().unwrap().contains("mock/replay"));
    assert_eq!(bridge.drain_events().len(), 1);
}

#[test]
fn bounded_gui_event_buffer_returns_backpressure_instead_of_growing_unbounded() {
    let mut buffer = BoundedGuiEventBuffer::new(1);

    buffer
        .push(GuiEvent::Notice {
            message: "first".to_string(),
        })
        .unwrap();
    let error = buffer
        .push(GuiEvent::Notice {
            message: "second".to_string(),
        })
        .unwrap_err();

    assert_eq!(error, GuiBridgeError::Backpressure { capacity: 1 });
    assert_eq!(buffer.drain().len(), 1);
    assert!(buffer.is_empty());
}
