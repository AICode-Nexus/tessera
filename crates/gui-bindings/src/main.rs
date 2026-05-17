use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let output_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("apps/gui-tauri/src/generated/bindings.ts"));

    tessera_gui_bindings::write_bindings(output_path)?;
    Ok(())
}
