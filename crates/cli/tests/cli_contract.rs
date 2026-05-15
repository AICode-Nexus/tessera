use tessera_cli::{
    build_tui_state_with_config, resolve_config, resolve_data_dir_with_config, run_chat_mock,
    run_chat_with_config, run_chat_with_config_and_events, run_doctor, DoctorReport,
};
use tessera_config::{ProviderProfile, TesseraConfig};
use tessera_core::EventSinkAction;
use tessera_protocol::RunEvent;

#[tokio::test]
async fn doctor_json_reports_trace_and_sqlite_health() {
    let temp = tempfile::tempdir().unwrap();
    let report: DoctorReport = run_doctor(temp.path()).unwrap();

    assert_eq!(report.status, "ok");
    assert!(report.trace_writable);
    assert!(report.sqlite_index_healthy);
    assert!(report
        .provider_profiles
        .iter()
        .any(|profile| profile == "mock"));
}

#[tokio::test]
async fn chat_command_path_runs_mock_provider() {
    let temp = tempfile::tempdir().unwrap();
    let output = run_chat_mock(temp.path(), "hello").await.unwrap();

    assert!(output.assistant_text.contains("mock response"));
    assert_eq!(output.trace_id, "trace_mock");
}

#[tokio::test]
async fn chat_command_path_routes_to_configured_mock_profile() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };

    let output = run_chat_with_config(temp.path(), &config, "offline", "hello")
        .await
        .unwrap();

    assert!(output.assistant_text.contains("mock response"));
    let events = output.store.list_events(&output.trace_id).unwrap();
    assert!(events.contains(&"route_decision_recorded".to_string()));
}

#[tokio::test]
async fn config_routed_chat_can_stream_live_event_frames_without_reading_trace_back() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let mut live_events = Vec::new();

    let output =
        run_chat_with_config_and_events(temp.path(), &config, "offline", "hello", |frame| {
            live_events.push(frame.clone());
        })
        .await
        .unwrap();

    assert!(live_events
        .iter()
        .any(|frame| matches!(frame.event, RunEvent::AssistantDelta { .. })));
    assert_eq!(live_events.last().unwrap().event.kind(), "done");
    assert_eq!(
        live_events
            .iter()
            .map(|frame| frame.event.kind().to_string())
            .collect::<Vec<_>>(),
        output.store.list_events(&output.trace_id).unwrap()
    );
}

#[tokio::test]
async fn config_routed_chat_records_cancellation_when_event_sink_stops() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };

    let output =
        run_chat_with_config_and_events(temp.path(), &config, "offline", "hello", |frame| {
            match frame.event {
                RunEvent::AssistantMessageStarted { .. } => {
                    EventSinkAction::Cancel("cli sink closed".to_string())
                }
                _ => EventSinkAction::Continue,
            }
        })
        .await
        .unwrap();

    let events = output.store.list_events(&output.trace_id).unwrap();
    assert!(events.contains(&"task_cancelled".to_string()));
    assert!(!events.contains(&"task_completed".to_string()));
}

#[tokio::test]
async fn config_routed_chat_uses_unique_trace_ids_across_runs() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-routed".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };

    let first = run_chat_with_config(temp.path(), &config, "offline", "hello")
        .await
        .unwrap();
    let second = run_chat_with_config(temp.path(), &config, "offline", "hello again")
        .await
        .unwrap();

    assert_ne!(first.trace_id, second.trace_id);
    assert!(first.trace_id.starts_with("trace_offline_"));
    assert!(second.trace_id.starts_with("trace_offline_"));
}

#[tokio::test]
async fn chat_command_path_rejects_unknown_provider_profile() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig::default_with_mock();

    let error = match run_chat_with_config(temp.path(), &config, "missing", "hello").await {
        Ok(_) => panic!("expected missing provider profile to fail"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains("provider profile not found"));
}

#[test]
fn tui_state_uses_configured_profiles_for_switching() {
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![
            ProviderProfile {
                id: "offline".to_string(),
                kind: "mock".to_string(),
                default_model: "mock-chat".to_string(),
                base_url: None,
                api_key_env: None,
            },
            ProviderProfile {
                id: "local".to_string(),
                kind: "ollama".to_string(),
                default_model: "llama3".to_string(),
                base_url: None,
                api_key_env: None,
            },
        ],
    };

    let state = build_tui_state_with_config(&config, "local").unwrap();

    assert_eq!(state.status.active_profile, "local");
    assert_eq!(state.status.available_profiles, vec!["offline", "local"]);
}

#[test]
fn tui_state_rejects_unknown_initial_profile() {
    let config = TesseraConfig::default_with_mock();

    let error = build_tui_state_with_config(&config, "missing")
        .unwrap_err()
        .to_string();

    assert!(error.contains("provider profile not found"));
}

#[tokio::test]
async fn openai_compatible_profile_requires_declared_api_key_env_before_trace() {
    let temp = tempfile::tempdir().unwrap();
    let missing_env = "TESSERA_TEST_MISSING_API_KEY_FOR_ROUTING";
    std::env::remove_var(missing_env);
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "remote".to_string(),
            kind: "openai-compatible".to_string(),
            default_model: "test-model".to_string(),
            base_url: Some("https://example.invalid/v1".to_string()),
            api_key_env: Some(missing_env.to_string()),
        }],
    };

    let error = match run_chat_with_config(temp.path(), &config, "remote", "hello").await {
        Ok(_) => panic!("expected missing API key env to fail before provider request"),
        Err(error) => error.to_string(),
    };

    assert!(error.contains(missing_env));
    assert!(!temp.path().join("traces/trace_remote.jsonl").exists());
}

#[test]
fn config_resolution_loads_explicit_path_and_data_dir_prefers_config() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
data_dir = "/tmp/tessera-configured"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
    )
    .unwrap();

    let config = resolve_config(Some(config_path)).unwrap();
    let data_dir = resolve_data_dir_with_config(None, &config).unwrap();

    assert_eq!(config.providers[0].id, "offline");
    assert_eq!(
        data_dir,
        std::path::PathBuf::from("/tmp/tessera-configured")
    );
}
