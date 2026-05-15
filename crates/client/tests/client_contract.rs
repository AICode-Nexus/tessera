use tessera_client::{
    ClientIntent, ClientMessageRole, ClientProjection, ClientSnapshot, ClientStatus,
};
use tessera_protocol::{EventFrame, ItemId, RunEvent};

#[test]
fn client_projection_turns_core_events_into_ui_neutral_messages() {
    let mut projection = ClientProjection::new("mock-default");
    let user_item_id = ItemId::from_static("item_user");
    let assistant_item_id = ItemId::from_static("item_assistant");

    projection.apply_event(&EventFrame::new(
        "trace_client",
        1,
        RunEvent::UserMessageRecorded {
            item_id: user_item_id.clone(),
            text: "hello client".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        2,
        RunEvent::AssistantMessageStarted {
            item_id: assistant_item_id.clone(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        3,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "shared ".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        4,
        RunEvent::AssistantDelta {
            item_id: assistant_item_id.clone(),
            text: "projection".to_string(),
        },
    ));
    projection.apply_event(&EventFrame::new(
        "trace_client",
        5,
        RunEvent::AssistantMessageCompleted {
            item_id: assistant_item_id,
        },
    ));

    assert_eq!(projection.messages[0].role, ClientMessageRole::User);
    assert_eq!(projection.messages[0].content, "hello client");
    assert_eq!(projection.messages[1].role, ClientMessageRole::Assistant);
    assert_eq!(projection.messages[1].content, "shared projection");
    assert!(!projection.messages[1].streaming);
}

#[test]
fn client_snapshot_keeps_status_intents_and_projection_toolkit_neutral() {
    let mut snapshot = ClientSnapshot::with_profiles("mock-default", ["mock-default", "offline"]);

    assert_eq!(
        snapshot.cycle_profile(1),
        Some(ClientIntent::SwitchProfile {
            profile_id: "offline".to_string(),
        })
    );
    assert_eq!(snapshot.status.active_profile, "offline");
    assert_eq!(snapshot.status.active_profile_position(), (2, 2));

    snapshot.draft_input = "hello gui".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::SubmitPrompt {
            profile_id: "offline".to_string(),
            prompt: "hello gui".to_string(),
        })
    );
    assert_eq!(snapshot.draft_input, "");

    let status = ClientStatus::with_profiles("mock-default", ["mock-default"]);
    assert_eq!(status.active_profile_position(), (1, 1));
}
