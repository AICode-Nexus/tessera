use clap::{Parser, Subcommand};
use std::io::Read;
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
    Sessions {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Transcript {
        trace_id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Replay {
        trace_id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Events {
        trace_id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        since: Option<u64>,
        #[arg(long)]
        limit: Option<usize>,
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
        stdin: bool,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long = "continue")]
        continue_last: bool,
        #[arg(long)]
        resume: Option<String>,
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
        Commands::Sessions {
            json,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let sessions = tessera_cli::list_sessions(data_dir)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&sessions)?);
            } else {
                for line in tessera_cli::format_session_lines(&sessions) {
                    println!("{line}");
                }
            }
        }
        Commands::Transcript {
            trace_id,
            json,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            if json {
                let transcript = tessera_cli::load_transcript(data_dir, &trace_id)?;
                println!("{}", serde_json::to_string_pretty(&transcript)?);
            } else {
                let markdown = tessera_cli::export_transcript_markdown(data_dir, &trace_id)?;
                print!("{markdown}");
            }
        }
        Commands::Replay {
            trace_id,
            json,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let replay = tessera_cli::replay_trace(data_dir, &trace_id)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&replay)?);
            } else {
                print!("{}", tessera_cli::format_replay_summary(&replay));
            }
        }
        Commands::Events {
            trace_id,
            json,
            since,
            limit,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let page = tessera_cli::list_events(data_dir, &trace_id, since, limit)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&page)?);
            } else {
                for line in tessera_cli::format_event_lines(&page) {
                    println!("{line}");
                }
            }
        }
        Commands::Chat {
            provider,
            prompt,
            stdin,
            file,
            json,
            continue_last,
            resume,
            config,
            data_dir,
        } => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let prompt_source_count =
                usize::from(prompt.is_some()) + usize::from(stdin) + usize::from(file.is_some());
            if prompt_source_count > 1 {
                anyhow::bail!("--prompt, --stdin, and --file cannot be combined");
            }
            if continue_last && (prompt_source_count > 0 || resume.is_some()) {
                anyhow::bail!(
                    "--continue cannot be combined with --prompt, --stdin, --file, or --resume"
                );
            }
            if resume.is_some() && prompt_source_count > 0 {
                anyhow::bail!("--resume is only supported in interactive chat mode");
            }

            let prompt = if stdin {
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input)?;
                Some(input.trim_end_matches(['\r', '\n']).to_string())
            } else if let Some(path) = file {
                let input = std::fs::read_to_string(path)?;
                Some(input.trim_end_matches(['\r', '\n']).to_string())
            } else {
                prompt
            };

            if let Some(prompt) = prompt {
                let outcome =
                    tessera_cli::run_chat_with_config(data_dir, &config, &provider, prompt).await?;
                if json {
                    let output = tessera_cli::CliChatOutput::from(outcome);
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    println!("{}", outcome.assistant_text);
                }
            } else {
                if json {
                    anyhow::bail!("--json is only supported with --prompt, --stdin, or --file");
                }
                let resume = if continue_last {
                    Some(tessera_cli::latest_session_trace_id(&data_dir)?)
                } else {
                    resume
                };
                tessera_cli::run_chat_repl_with_config_and_resume(
                    data_dir, config, provider, resume,
                )
                .await?;
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
