fn main() {
    println!("cargo:rerun-if-changed=permissions");
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "list_profiles",
                "load_client_snapshot",
                "submit_client_intent",
                "cancel_task",
                "load_trace_projection",
                "export_thread",
            ]),
        ),
    )
    .expect("failed to build Tauri ACL and config");
}
