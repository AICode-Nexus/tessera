use std::io::Write;

use tessera_cli::{
    build_tui_state_with_config, parse_repl_command, resolve_config, resolve_data_dir_with_config,
    run_chat_mock, run_chat_repl_with_io_and_resume, run_chat_with_config,
    run_chat_with_config_and_events, run_doctor, run_repl_prompt_with_writer,
    write_config_template, CliReplCommand, CliReplSession, DoctorReport,
};
use tessera_config::{ProviderProfile, TesseraConfig};
use tessera_core::EventSinkAction;
use tessera_protocol::RunEvent;

#[test]
fn version_output_reports_crate_version_and_git_sha() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .arg("--version")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let git_sha = option_env!("TESSERA_GIT_SHA").unwrap_or("unknown");

    assert_ne!(git_sha, "unknown");
    assert_eq!(git_sha.len(), 40);
    assert!(git_sha
        .chars()
        .all(|character| character.is_ascii_hexdigit()));
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    assert!(stdout.contains(git_sha));
}

#[test]
fn chat_help_lists_resume_option() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--resume <RESUME>"));
    assert!(stdout.contains("--stdin"));
}

#[test]
fn sessions_help_lists_json_and_data_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["sessions", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--data-dir"));
}

#[test]
fn transcript_help_lists_trace_id_and_json_option() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["transcript", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<TRACE_ID>"));
    assert!(stdout.contains("--json"));
}

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

#[test]
fn repl_parser_recognizes_local_slash_commands() {
    assert_eq!(parse_repl_command("hello repl"), None);
    assert_eq!(parse_repl_command("/help"), Some(CliReplCommand::Help));
    assert_eq!(parse_repl_command("/new"), Some(CliReplCommand::NewThread));
    assert_eq!(
        parse_repl_command("/profiles"),
        Some(CliReplCommand::Profiles)
    );
    assert_eq!(parse_repl_command("/status"), Some(CliReplCommand::Status));
    assert_eq!(parse_repl_command("/export"), Some(CliReplCommand::Export));
    assert_eq!(parse_repl_command("/quit"), Some(CliReplCommand::Quit));
    assert_eq!(parse_repl_command("/exit"), Some(CliReplCommand::Quit));
    assert_eq!(
        parse_repl_command("/profile offline"),
        Some(CliReplCommand::SwitchProfile("offline".to_string()))
    );
    assert_eq!(
        parse_repl_command("/does-not-exist"),
        Some(CliReplCommand::Unknown("/does-not-exist".to_string()))
    );
    assert_eq!(
        parse_repl_command("/sessions"),
        Some(CliReplCommand::Sessions)
    );
    assert_eq!(
        parse_repl_command("/resume trace_123"),
        Some(CliReplCommand::ResumeSession("trace_123".to_string()))
    );
}

#[test]
fn repl_session_switches_profiles_and_rejects_unknown_profiles() {
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
    let mut session = CliReplSession::new(&config, "offline").unwrap();

    let outcome = session
        .handle_command(&config, CliReplCommand::SwitchProfile("local".to_string()))
        .unwrap();

    assert!(!outcome.should_quit);
    assert_eq!(session.snapshot().status.active_profile, "local");
    assert!(outcome
        .lines
        .join("\n")
        .contains("profile switched to local"));

    let error = session
        .handle_command(
            &config,
            CliReplCommand::SwitchProfile("missing".to_string()),
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("provider profile not found"));
    assert_eq!(session.snapshot().status.active_profile, "local");
}

#[test]
fn repl_session_handles_local_commands_without_runtime_work() {
    let config = TesseraConfig::default_with_mock();
    let mut session = CliReplSession::new(&config, "mock").unwrap();
    session.snapshot_mut().push_notice("temporary note");

    let status = session
        .handle_command(&config, CliReplCommand::Status)
        .unwrap();
    assert!(status.lines.join("\n").contains("profile mock"));

    let export = session
        .handle_command(&config, CliReplCommand::Export)
        .unwrap();
    assert!(export.lines.join("\n").contains("temporary note"));

    let new_thread = session
        .handle_command(&config, CliReplCommand::NewThread)
        .unwrap();
    assert!(new_thread.lines.join("\n").contains("new thread"));
    assert!(session.snapshot().projection.messages.is_empty());

    let unknown = session
        .handle_command(&config, CliReplCommand::Unknown("/danger".to_string()))
        .unwrap();
    assert!(unknown.lines.join("\n").contains("unknown command"));

    let quit = session
        .handle_command(&config, CliReplCommand::Quit)
        .unwrap();
    assert!(quit.should_quit);
}

#[tokio::test]
async fn repl_prompt_streams_live_events_into_client_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();
    let mut streamed_text = String::new();

    let outcome =
        run_repl_prompt_with_writer(temp.path(), &config, &mut session, "hello repl", |delta| {
            streamed_text.push_str(delta)
        })
        .await
        .unwrap();

    assert!(streamed_text.contains("mock response"));
    assert_eq!(outcome.assistant_text, streamed_text);
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello repl"));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content.contains("mock response")));

    let mut follow_up_text = String::new();
    run_repl_prompt_with_writer(
        temp.path(),
        &config,
        &mut session,
        "continue from that",
        |delta| follow_up_text.push_str(delta),
    )
    .await
    .unwrap();

    assert!(follow_up_text.contains("history messages: 3"));
}

#[test]
fn init_config_template_writes_secret_safe_profiles_and_respects_force() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");

    write_config_template(&config_path, false).unwrap();
    let template = std::fs::read_to_string(&config_path).unwrap();

    assert!(template.contains("[[providers]]"));
    assert!(template.contains("id = \"mock\""));
    assert!(template.contains("id = \"ollama\""));
    assert!(template.contains("id = \"openai-compatible\""));
    assert!(template.contains("api_key_env = \"TESSERA_OPENAI_COMPATIBLE_API_KEY\""));
    assert!(!template.contains("sk-"));
    assert!(!template.contains("Bearer "));

    let error = write_config_template(&config_path, false)
        .unwrap_err()
        .to_string();
    assert!(error.contains("already exists"));

    write_config_template(&config_path, true).unwrap();
}

#[tokio::test]
async fn repl_sessions_and_resume_use_trace_projection_without_provider_call() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let trace_id = {
        let output = run_chat_with_config(temp.path(), &config, "offline", "hello resumable")
            .await
            .unwrap();
        output.trace_id
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();

    let sessions = session
        .handle_command_with_data_dir(temp.path(), &config, CliReplCommand::Sessions)
        .unwrap();
    assert!(sessions.lines.join("\n").contains(&trace_id));

    let resumed = session
        .handle_command_with_data_dir(
            temp.path(),
            &config,
            CliReplCommand::ResumeSession(trace_id.clone()),
        )
        .unwrap();

    assert!(resumed
        .lines
        .join("\n")
        .contains(&format!("resumed trace {trace_id}")));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello resumable"));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content.contains("mock response")));

    let mut follow_up_text = String::new();
    run_repl_prompt_with_writer(
        temp.path(),
        &config,
        &mut session,
        "continue from that",
        |delta| follow_up_text.push_str(delta),
    )
    .await
    .unwrap();

    assert!(follow_up_text.contains("history messages: 3"));
}

#[tokio::test]
async fn repl_can_start_from_resume_trace_id_and_continue_with_history() {
    let temp = tempfile::tempdir().unwrap();
    let config = TesseraConfig {
        data_dir: None,
        providers: vec![ProviderProfile {
            id: "offline".to_string(),
            kind: "mock".to_string(),
            default_model: "mock-chat".to_string(),
            base_url: None,
            api_key_env: None,
        }],
    };
    let trace_id = run_chat_with_config(temp.path(), &config, "offline", "hello startup resume")
        .await
        .unwrap()
        .trace_id;
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        Some(trace_id.clone()),
        "continue from startup\n/quit\n".as_bytes(),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains(&format!("resumed trace {trace_id}")));
    assert!(stdout.contains("history messages: 3"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "hello startup resume"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "continue from startup"));
}

#[tokio::test]
async fn sessions_command_lists_trace_backed_sessions_from_configured_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello sessions")
        .await
        .unwrap()
        .trace_id;

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["sessions", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&trace_id));
    assert!(stdout.contains("hello sessions"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["sessions", "--config"])
        .arg(&config_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let sessions: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(sessions[0]["trace_id"], trace_id);
    assert_eq!(sessions[0]["user_preview"], "hello sessions");
}

#[tokio::test]
async fn transcript_command_exports_markdown_and_json_from_configured_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();
    let config = resolve_config(Some(config_path.clone())).unwrap();
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello transcript")
        .await
        .unwrap()
        .trace_id;

    let markdown_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["transcript"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(markdown_output.status.success());
    let markdown = String::from_utf8(markdown_output.stdout).unwrap();
    assert!(markdown.contains("# Tessera Export"));
    assert!(markdown.contains("## User"));
    assert!(markdown.contains("hello transcript"));
    assert!(markdown.contains("## Assistant"));
    assert!(markdown.contains("mock response to: hello transcript"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["transcript"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let transcript: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(transcript["trace_id"], trace_id);
    assert_eq!(transcript["messages"][0]["role"], "user");
    assert_eq!(transcript["messages"][0]["content"], "hello transcript");
    assert_eq!(transcript["messages"][1]["role"], "assistant");
    assert_eq!(
        transcript["messages"][1]["content"],
        "mock response to: hello transcript"
    );
}

#[tokio::test]
async fn chat_command_path_reads_prompt_from_stdin() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"hello from stdin\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock response to: hello from stdin"));
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
