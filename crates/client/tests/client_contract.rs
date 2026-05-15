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

#[test]
fn client_snapshot_maps_slash_commands_to_ui_neutral_intents() {
    let mut snapshot = ClientSnapshot::new("mock-default");

    snapshot.draft_input = " /new ".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::NewThread));

    snapshot.draft_input = "/save".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::SaveThread));

    snapshot.draft_input = "/export".to_string();
    assert_eq!(snapshot.submit_input(), Some(ClientIntent::ExportThread));

    snapshot.draft_input = "/explain this command".to_string();
    assert_eq!(
        snapshot.submit_input(),
        Some(ClientIntent::SubmitPrompt {
            profile_id: "mock-default".to_string(),
            prompt: "/explain this command".to_string(),
        })
    );
}

#[test]
fn client_snapshot_resets_thread_and_exports_markdown_projection() {
    let mut snapshot = ClientSnapshot::new("mock-default");
    snapshot.apply_event(&EventFrame::new(
        "trace_export",
        1,
        RunEvent::UserMessageRecorded {
            item_id: ItemId::from_static("item_user_export"),
            text: "hello export".to_string(),
        },
    ));
    snapshot.apply_event(&EventFrame::new(
        "trace_export",
        2,
        RunEvent::AssistantDelta {
            item_id: ItemId::from_static("item_assistant_export"),
            text: "exported answer".to_string(),
        },
    ));

    let markdown = snapshot.export_markdown();
    assert!(markdown.contains("# Tessera Export"));
    assert!(markdown.contains("## User"));
    assert!(markdown.contains("hello export"));
    assert!(markdown.contains("## Assistant"));
    assert!(markdown.contains("exported answer"));

    snapshot.start_new_thread();

    assert!(snapshot.projection.messages.is_empty());
    assert_eq!(snapshot.draft_input, "");
    assert_eq!(snapshot.status.active_profile, "mock-default");
}
