use std::io::Write;

use tessera_cli::{
    build_tui_state_with_config, list_sessions, parse_repl_command, resolve_config,
    resolve_data_dir_with_config, run_chat_mock, run_chat_repl_with_io_and_resume,
    run_chat_with_config, run_chat_with_config_and_events, run_doctor, run_repl_prompt_with_writer,
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
    assert!(stdout.contains("--file <FILE>"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--continue"));
    assert!(stdout.contains("--list-commands"));
}

#[test]
fn chat_list_commands_prints_repl_commands_without_runtime_config() {
    let temp = tempfile::tempdir().unwrap();
    let missing_config = temp.path().join("missing.toml");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--list-commands", "--config"])
        .arg(&missing_config)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("commands:"));
    assert!(stdout.contains("/help"));
    assert!(stdout.contains("/cancel"));
    assert!(stdout.contains("/clear"));
    assert!(stdout.contains("/paste"));
    assert!(stdout.contains("/profiles"));
    assert!(stdout.contains("/history"));
    assert!(stdout.contains("/resume <trace_id|#>"));
    assert!(stdout.contains("/doctor"));
    assert!(stdout.contains("/quit"));
    assert!(!stdout.contains("Tessera CLI interactive chat"));
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
fn profiles_help_lists_json_and_config_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["profiles", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--config"));
}

#[test]
fn config_validate_help_lists_json_config_and_data_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--config"));
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

#[test]
fn replay_help_lists_trace_id_and_json_option() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["replay", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<TRACE_ID>"));
    assert!(stdout.contains("--json"));
}

#[test]
fn events_help_lists_pagination_and_json_options() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["events", "--help"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("<TRACE_ID>"));
    assert!(stdout.contains("--json"));
    assert!(stdout.contains("--since <SINCE>"));
    assert!(stdout.contains("--limit <LIMIT>"));
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

#[test]
fn doctor_text_reports_runtime_health_details() {
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

[[providers]]
id = "local"
kind = "ollama"
default_model = "llama3"
base_url = "http://localhost:11434"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["doctor", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("status: ok"));
    assert!(stdout.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(stdout.contains("trace_writable: true"));
    assert!(stdout.contains("sqlite_index_healthy: true"));
    assert!(stdout.contains("provider_profiles: offline, local"));
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
    assert_eq!(parse_repl_command("/commands"), Some(CliReplCommand::Help));
    assert_eq!(parse_repl_command("/new"), Some(CliReplCommand::NewThread));
    assert_eq!(parse_repl_command("/clear"), Some(CliReplCommand::Clear));
    assert_eq!(parse_repl_command("/cancel"), Some(CliReplCommand::Cancel));
    assert_eq!(parse_repl_command("/paste"), Some(CliReplCommand::Paste));
    assert_eq!(
        parse_repl_command("/profiles"),
        Some(CliReplCommand::Profiles)
    );
    assert_eq!(
        parse_repl_command("/history"),
        Some(CliReplCommand::History)
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
    assert_eq!(parse_repl_command("/doctor"), Some(CliReplCommand::Doctor));
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

    let cancel = session
        .handle_command(&config, CliReplCommand::Cancel)
        .unwrap();
    assert!(cancel.lines.join("\n").contains("no active run to cancel"));

    let quit = session
        .handle_command(&config, CliReplCommand::Quit)
        .unwrap();
    assert!(quit.should_quit);
}

#[test]
fn repl_session_lists_and_clears_visible_history_without_runtime_work() {
    let config = TesseraConfig::default_with_mock();
    let mut session = CliReplSession::new(&config, "mock").unwrap();

    let empty_history = session
        .handle_command(&config, CliReplCommand::History)
        .unwrap();
    assert!(empty_history
        .lines
        .join("\n")
        .contains("no messages in current thread"));

    session.snapshot_mut().push_notice("temporary note");
    let history = session
        .handle_command(&config, CliReplCommand::History)
        .unwrap();
    assert!(history
        .lines
        .join("\n")
        .contains("1. system: temporary note"));

    let clear = session
        .handle_command(&config, CliReplCommand::Clear)
        .unwrap();
    assert!(clear.lines.join("\n").contains("current thread cleared"));
    assert!(session.snapshot().projection.messages.is_empty());
}

#[test]
fn repl_doctor_reports_runtime_health_for_active_data_dir() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
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
        .handle_command_with_data_dir(&data_dir, &config, CliReplCommand::Doctor)
        .unwrap();

    let lines = outcome.lines.join("\n");
    assert!(!outcome.should_quit);
    assert!(lines.contains("status: ok"));
    assert!(lines.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(lines.contains("trace_writable: true"));
    assert!(lines.contains("sqlite_index_healthy: true"));
    assert!(lines.contains("provider_profiles: offline, local"));
}

#[tokio::test]
async fn repl_startup_prints_runtime_context_before_first_prompt() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
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
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        data_dir.clone(),
        config,
        "local".to_string(),
        None,
        "/quit\n".as_bytes(),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert_eq!(snapshot.status.active_profile, "local");
    assert!(stdout.contains("Tessera CLI interactive chat"));
    assert!(stdout.contains("active_profile: local"));
    assert!(stdout.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(stdout.contains("available_profiles: offline, local"));
    assert!(stdout.contains("type /help or run `tessera chat --list-commands`"));
    assert!(stdout.contains("/doctor"));
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

#[tokio::test]
async fn repl_paste_mode_submits_multiline_prompt_and_can_cancel() {
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
    let mut output = Vec::new();

    let snapshot = run_chat_repl_with_io_and_resume(
        temp.path().to_path_buf(),
        config,
        "offline".to_string(),
        None,
        "/paste\nfirst pasted line\nsecond pasted line\n/send\n/paste\nignored pasted line\n/cancel\n/history\n/quit\n".as_bytes(),
        &mut output,
    )
    .await
    .unwrap();

    let stdout = String::from_utf8(output).unwrap();
    assert!(stdout.contains("paste mode; end with /send or /cancel"));
    assert!(stdout.contains("paste cancelled"));
    assert!(stdout.contains("assistant> mock response to: first pasted line"));
    assert!(stdout.contains("1. user: first pasted line second pasted line"));
    assert!(snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content == "first pasted line\nsecond pasted line"));
    assert!(!snapshot
        .projection
        .messages
        .iter()
        .any(|message| message.content.contains("ignored pasted line")));
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
    assert!(sessions.lines[0].starts_with("1. "));

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
async fn repl_resume_accepts_numbered_session_index() {
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
    let older_trace_id = run_chat_with_config(temp.path(), &config, "offline", "older indexed")
        .await
        .unwrap()
        .trace_id;
    let latest_trace_id = run_chat_with_config(temp.path(), &config, "offline", "latest indexed")
        .await
        .unwrap()
        .trace_id;
    let sessions = list_sessions(temp.path()).unwrap();
    let first_trace_id = sessions[0].trace_id.clone();
    let first_prompt = if first_trace_id == older_trace_id {
        "older indexed"
    } else {
        assert_eq!(first_trace_id, latest_trace_id);
        "latest indexed"
    };
    let mut session = CliReplSession::new(&config, "offline").unwrap();

    let listed = session
        .handle_command_with_data_dir(temp.path(), &config, CliReplCommand::Sessions)
        .unwrap();
    assert!(listed.lines[0].starts_with("1. "));
    assert!(listed.lines[0].contains(&first_trace_id));

    let resumed = session
        .handle_command_with_data_dir(
            temp.path(),
            &config,
            CliReplCommand::ResumeSession("1".to_string()),
        )
        .unwrap();

    assert!(resumed
        .lines
        .join("\n")
        .contains(&format!("resumed trace {first_trace_id}")));
    assert!(session
        .snapshot()
        .projection
        .messages
        .iter()
        .any(|message| message.content == first_prompt));

    let error = session
        .handle_command_with_data_dir(
            temp.path(),
            &config,
            CliReplCommand::ResumeSession("3".to_string()),
        )
        .unwrap_err()
        .to_string();
    assert!(error.contains("session index out of range: 3"));
    assert!(error.contains("available sessions: 2"));
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
async fn chat_continue_starts_from_latest_trace_and_continues_with_history() {
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
    run_chat_with_config(&data_dir, &config, "offline", "older session")
        .await
        .unwrap();
    let latest_trace_id = run_chat_with_config(&data_dir, &config, "offline", "latest session")
        .await
        .unwrap()
        .trace_id;

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--continue"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"continue latest\n/quit\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(&format!("resumed trace {latest_trace_id}")));
    assert!(stdout.contains("history messages: 3"));
    assert!(stdout.contains("continue latest"));
}

#[test]
fn chat_continue_rejects_missing_session() {
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

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--continue"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("no sessions found to continue"));
}

#[test]
fn chat_continue_rejects_one_shot_prompt_sources() {
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

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--continue", "--prompt", "hello"])
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--continue cannot be combined"));
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
    assert!(stdout.contains("1. "));
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
async fn replay_command_reconstructs_trace_summary_without_provider_call() {
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
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello replay cli")
        .await
        .unwrap()
        .trace_id;

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["replay"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains(&trace_id));
    assert!(text.contains("events:"));
    assert!(text.contains("mock response to: hello replay cli"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["replay"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .arg("--json")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let replay: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(replay["trace_id"], trace_id);
    assert_eq!(
        replay["assistant_text"],
        "mock response to: hello replay cli"
    );
    assert!(replay["event_count"].as_u64().unwrap() > 0);
    assert!(replay["event_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind == "assistant_delta"));
}

#[tokio::test]
async fn events_command_pages_trace_events_from_configured_data_dir() {
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
    let trace_id = run_chat_with_config(&data_dir, &config, "offline", "hello events cli")
        .await
        .unwrap()
        .trace_id;

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["events"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .args(["--limit", "2"])
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains(&trace_id));
    assert!(text.contains("1 |"));
    assert!(text.contains("next_since_seq: 2"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["events"])
        .arg(&trace_id)
        .args(["--config"])
        .arg(&config_path)
        .args(["--since", "1", "--limit", "2", "--json"])
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let page: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(page["trace_id"], trace_id);
    assert_eq!(page["records"].as_array().unwrap().len(), 2);
    assert!(page["records"][0]["seq"].as_u64().unwrap() > 1);
    assert_eq!(page["next_since_seq"], page["records"][1]["seq"]);
    assert!(!page["records"][0]["event_kind"]
        .as_str()
        .unwrap()
        .is_empty());
}

#[test]
fn profiles_command_lists_configured_profiles_without_secret_values() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        r#"
data_dir = "./data"

[[providers]]
id = "offline"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"
base_url = "https://example.invalid/v1"
api_key_env = "TESSERA_TEST_PROFILE_SECRET"
"#,
    )
    .unwrap();

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["profiles", "--config"])
        .arg(&config_path)
        .env("TESSERA_TEST_PROFILE_SECRET", "super-secret-profile-key")
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains("offline | mock | model mock-chat"));
    assert!(text.contains("remote | openai-compatible | model test-model"));
    assert!(text.contains("base_url https://example.invalid/v1"));
    assert!(text.contains("api_key_env TESSERA_TEST_PROFILE_SECRET"));
    assert!(!text.contains("super-secret-profile-key"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["profiles", "--config"])
        .arg(&config_path)
        .arg("--json")
        .env("TESSERA_TEST_PROFILE_SECRET", "super-secret-profile-key")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let profiles: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(profiles[0]["id"], "offline");
    assert_eq!(profiles[0]["kind"], "mock");
    assert_eq!(profiles[0]["default_model"], "mock-chat");
    assert_eq!(profiles[0]["base_url"], serde_json::Value::Null);
    assert_eq!(profiles[0]["api_key_env"], serde_json::Value::Null);
    assert_eq!(profiles[1]["id"], "remote");
    assert_eq!(profiles[1]["base_url"], "https://example.invalid/v1");
    assert_eq!(profiles[1]["api_key_env"], "TESSERA_TEST_PROFILE_SECRET");
    assert!(!String::from_utf8(json_output.stdout)
        .unwrap()
        .contains("super-secret-profile-key"));
}

#[test]
fn config_validate_reports_ok_profiles_without_secret_values() {
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

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"
base_url = "https://example.invalid/v1"
api_key_env = "TESSERA_TEST_VALIDATE_API_KEY"
"#,
            data_dir.display()
        ),
    )
    .unwrap();

    let text_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .env("TESSERA_TEST_VALIDATE_API_KEY", "super-secret-validate-key")
        .output()
        .unwrap();

    assert!(text_output.status.success());
    let text = String::from_utf8(text_output.stdout).unwrap();
    assert!(text.contains("status: ok"));
    assert!(text.contains(&format!("data_dir: {}", data_dir.display())));
    assert!(text.contains("profile offline: ok (mock, model mock-chat)"));
    assert!(text.contains(
        "profile remote: ok (openai-compatible, model test-model, api_key_env TESSERA_TEST_VALIDATE_API_KEY set)"
    ));
    assert!(!text.contains("super-secret-validate-key"));

    let json_output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .arg("--json")
        .env("TESSERA_TEST_VALIDATE_API_KEY", "super-secret-validate-key")
        .output()
        .unwrap();

    assert!(json_output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&json_output.stdout).unwrap();
    assert_eq!(report["status"], "ok");
    assert_eq!(report["issues"], serde_json::json!([]));
    assert_eq!(report["profiles"][1]["id"], "remote");
    assert_eq!(
        report["profiles"][1]["api_key_env"],
        "TESSERA_TEST_VALIDATE_API_KEY"
    );
    assert_eq!(report["profiles"][1]["api_key_env_status"], "set");
    assert!(!String::from_utf8(json_output.stdout)
        .unwrap()
        .contains("super-secret-validate-key"));
}

#[test]
fn config_validate_fails_for_missing_secret_env_without_touching_storage() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    let missing_env = format!("TESSERA_TEST_MISSING_VALIDATE_{}", std::process::id());
    std::fs::write(
        &config_path,
        format!(
            r#"
data_dir = "{}"

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"
base_url = "https://example.invalid/v1"
api_key_env = "{}"
"#,
            data_dir.display(),
            missing_env
        ),
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .arg("--json")
        .env_remove(&missing_env)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "error");
    assert_eq!(report["profiles"][0]["api_key_env_status"], "missing");
    assert!(report["issues"][0]["message"]
        .as_str()
        .unwrap()
        .contains(&missing_env));
    assert!(!data_dir.exists());
}

#[test]
fn config_validate_fails_when_no_provider_profiles_are_configured() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(&config_path, "providers = []\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("status: error"));
    assert!(text.contains("at least one provider profile is required"));
}

#[test]
fn config_validate_reports_provider_shape_errors() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("tessera.toml");
    std::fs::write(
        &config_path,
        r#"
[[providers]]
id = "dup"
kind = "mock"
default_model = "mock-chat"

[[providers]]
id = "dup"
kind = "ollama"
default_model = "llama3"

[[providers]]
id = "remote"
kind = "openai-compatible"
default_model = "test-model"

[[providers]]
id = "unknown"
kind = "mystery"
default_model = "test-model"
"#,
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["config", "validate", "--config"])
        .arg(&config_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("status: error"));
    assert!(text.contains("duplicate provider id `dup`"));
    assert!(text.contains("provider `remote` kind openai-compatible requires base_url"));
    assert!(text.contains("unsupported provider kind `mystery`"));
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
async fn chat_command_path_reads_prompt_from_file() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    let prompt_path = temp.path().join("prompt.md");
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
    std::fs::write(&prompt_path, "hello from file\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--file"])
        .arg(&prompt_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("mock response to: hello from file"));
}

#[tokio::test]
async fn chat_command_path_emits_json_for_one_shot_prompt() {
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

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--prompt", "hello json", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json["trace_id"]
        .as_str()
        .unwrap()
        .starts_with("trace_offline_"));
    assert_eq!(json["assistant_text"], "mock response to: hello json");
}

#[test]
fn chat_command_path_rejects_json_without_one_shot_prompt() {
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

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--json"])
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("--json is only supported with --prompt, --stdin, or --file"));
}

#[test]
fn chat_command_path_rejects_multiple_prompt_sources() {
    let temp = tempfile::tempdir().unwrap();
    let data_dir = temp.path().join("data");
    let config_path = temp.path().join("tessera.toml");
    let prompt_path = temp.path().join("prompt.md");
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
    std::fs::write(&prompt_path, "hello from file\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tessera"))
        .args(["chat", "--config"])
        .arg(&config_path)
        .args(["--provider", "offline", "--prompt", "hello", "--file"])
        .arg(&prompt_path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot be combined"));
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
