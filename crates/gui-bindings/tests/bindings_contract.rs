use std::{fs, path::PathBuf};

use tessera_gui_bindings::{generate_bindings, write_bindings};

#[test]
fn generated_bindings_include_gui_dtos_without_forbidden_runtime_commands() {
    let bindings = generate_bindings();

    assert!(bindings.contains("export type ClientIntent"));
    assert!(bindings.contains("export type ClientApproval"));
    assert!(bindings.contains("export type ClientMemoryProposal"));
    assert!(bindings.contains("export type ClientSnapshot"));
    assert!(bindings.contains("export type GuiCommandOutcome"));
    assert!(bindings.contains("export type GuiShellState"));
    assert!(bindings.contains("submit_prompt"));
    assert!(bindings.contains("cancel_task"));
    assert!(bindings.contains("approve_tool_call"));
    assert!(bindings.contains("deny_tool_call"));
    assert!(bindings.contains("accept_memory_proposal"));
    assert!(bindings.contains("reject_memory_proposal"));
    assert!(!bindings.contains("call_provider"));
    assert!(!bindings.contains("read_sql"));
    assert!(!bindings.contains("execute_shell"));
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
