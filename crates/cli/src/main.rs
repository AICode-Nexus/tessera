use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tessera")]
#[command(version = tessera_cli::VERSION_TEXT)]
#[command(about = "AI-friendly local LLM workbench")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(long, default_value = "tessera.toml")]
        config: PathBuf,
        #[arg(long)]
        force: bool,
    },
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Chat {
        #[arg(long, default_value = "mock")]
        provider: String,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Tui {
        #[arg(long, default_value = "mock")]
        provider: String,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { config, force } => {
            let path = tessera_cli::write_config_template(config, force)?;
            println!("wrote {}", path.display());
        }
        Commands::Doctor {
            json,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let report = tessera_cli::run_doctor_with_config(data_dir, &config)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("status: {}", report.status);
            }
        }
        Commands::Chat {
            provider,
            prompt,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            if let Some(prompt) = prompt {
                let outcome =
                    tessera_cli::run_chat_with_config(data_dir, &config, &provider, prompt).await?;
                println!("{}", outcome.assistant_text);
            } else {
                tessera_cli::run_chat_repl_with_config(data_dir, config, provider).await?;
            }
        }
        Commands::Tui {
            provider,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            tessera_cli::run_tui_with_config(data_dir, config, provider).await?;
        }
    }

    Ok(())
}
