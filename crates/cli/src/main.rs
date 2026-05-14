use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tessera")]
#[command(about = "AI-friendly local LLM workbench")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Chat {
        #[arg(long, default_value = "mock")]
        provider: String,
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Doctor { json, data_dir } => {
            let data_dir = tessera_cli::resolve_data_dir(data_dir)?;
            let report = tessera_cli::run_doctor(data_dir)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("status: {}", report.status);
            }
        }
        Commands::Chat {
            provider,
            prompt,
            data_dir,
        } => {
            if provider != "mock" {
                anyhow::bail!("v0.1 scaffold only enables the mock provider path");
            }
            let data_dir = tessera_cli::resolve_data_dir(data_dir)?;
            let outcome = tessera_cli::run_chat_mock(data_dir, prompt).await?;
            println!("{}", outcome.assistant_text);
        }
    }

    Ok(())
}
