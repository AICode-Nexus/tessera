use std::{fs, path::PathBuf};

use tessera_gui_bindings::{generate_bindings, write_bindings};

#[test]
fn generated_bindings_include_gui_dtos_without_forbidden_runtime_commands() {
    let bindings = generate_bindings();

    assert!(bindings.contains("export type ClientIntent"));
    assert!(bindings.contains("export type ClientApproval"));
    assert!(bindings.contains("export type ClientContextBudgetSummary"));
    assert!(bindings.contains("export type ClientContextHandle"));
    assert!(bindings.contains("export type ClientContextPlacement"));
    assert!(bindings.contains("export type ClientContextSourceKind"));
    assert!(bindings.contains("export type ClientMemoryProposal"));
    assert!(bindings.contains("export type ClientAgentHandoff"));
    assert!(bindings.contains("export type ClientReviewerGate"));
    assert!(bindings.contains("export type ClientSubagentSession"));
    assert!(bindings.contains("export type ClientSubagentRuntimeDecision"));
    assert!(bindings.contains("export type ClientSubagentTranscriptArtifact"));
    assert!(bindings.contains("export type ClientSubagentTranscriptArtifactLifecycle"));
    assert!(bindings.contains("export type ClientSubagentTranscriptArtifactStatus"));
    assert!(bindings.contains("export type ClientSubagentApprovalForwarding"));
    assert!(bindings.contains("export type ClientSubagentInactivePolicy"));
    assert!(bindings.contains("export type ClientSubagentCancellation"));
    assert!(bindings.contains("export type SubagentSessionDescriptor"));
    assert!(bindings.contains("export type SubagentSessionCaps"));
    assert!(bindings.contains("export type SubagentRuntimeDecision"));
    assert!(bindings.contains("export type SubagentTranscriptArtifactRecord"));
    assert!(bindings.contains("export type SubagentTranscriptArtifactLifecycleRecord"));
    assert!(bindings.contains("export type SubagentTranscriptArtifactStatus"));
    assert!(bindings.contains("export type SubagentApprovalForwardingRecord"));
    assert!(bindings.contains("export type SubagentInactivePolicyRecord"));
    assert!(bindings.contains("export type SubagentCancellationRecord"));
    assert!(bindings.contains("export type AgentHandoffSummary"));
    assert!(bindings.contains("export type ReviewerGateRequest"));
    assert!(bindings.contains("export type ReviewerGateDecision"));
    assert!(bindings.contains("export type ClientSnapshot"));
    assert!(bindings.contains("export type ContextId"));
    assert!(bindings.contains("export type GuiCommandOutcome"));
    assert!(bindings.contains("export type GuiShellState"));
    assert!(bindings.contains("export type RuntimeApiServerConfig"));
    assert!(bindings.contains("export type RuntimeApiCommandEnvelope"));
    assert!(bindings.contains("export type RuntimeApiEventStreamRequest"));
    assert!(bindings.contains("export type RuntimeApiCommandAck"));
    assert!(bindings.contains("context_handles"));
    assert!(bindings.contains("loopback_dev_token"));
    assert!(bindings.contains("localhost_tcp"));
    assert!(bindings.contains("submit_prompt"));
    assert!(bindings.contains("cancel_task"));
    assert!(bindings.contains("pause_task"));
    assert!(bindings.contains("resume_task"));
    assert!(bindings.contains("task_paused"));
    assert!(bindings.contains("task_resumed"));
    assert!(bindings.contains("agent_handoff_recorded"));
    assert!(bindings.contains("reviewer_gate_requested"));
    assert!(bindings.contains("reviewer_gate_resolved"));
    assert!(bindings.contains("subagent_session_planned"));
    assert!(bindings.contains("subagent_session_waiting_for_approval"));
    assert!(bindings.contains("subagent_runtime_decision_recorded"));
    assert!(bindings.contains("subagent_transcript_artifact_recorded"));
    assert!(bindings.contains("subagent_transcript_artifact_lifecycle_recorded"));
    assert!(bindings.contains("subagent_approval_forwarding_recorded"));
    assert!(bindings.contains("subagent_inactive_policy_recorded"));
    assert!(bindings.contains("subagent_cancellation_recorded"));
    assert!(bindings.contains("approve_tool_call"));
    assert!(bindings.contains("deny_tool_call"));
    assert!(bindings.contains("accept_memory_proposal"));
    assert!(bindings.contains("reject_memory_proposal"));
    assert!(!bindings.contains("start_server"));
    assert!(!bindings.contains("bind_remote"));
    assert!(!bindings.contains("call_provider"));
    assert!(!bindings.contains("read_sql"));
    assert!(!bindings.contains("execute_shell"));
    assert!(!bindings.contains("start_child_agent"));
    assert!(!bindings.contains("spawn_subagent"));
    assert!(!bindings.contains("start_scheduler"));
    assert!(!bindings.contains("store_transcript_body"));
    assert!(!bindings.contains("summarize_child_transcript"));
    assert!(!bindings.contains("execute_tool"));
}

#[test]
fn checked_in_gui_bindings_match_rust_generation() {
    let generated_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/gui-tauri/src/generated/bindings.ts");
    let checked_in = fs::read_to_string(&generated_path).unwrap();

    assert_eq!(checked_in, generate_bindings());
}

#[test]
fn write_bindings_creates_parent_directory_and_exact_generated_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let output_path = temp_dir.path().join("generated/bindings.ts");

    write_bindings(&output_path).unwrap();

    assert_eq!(
        fs::read_to_string(output_path).unwrap(),
        generate_bindings()
    );
}
