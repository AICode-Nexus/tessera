use clap::{Parser, Subcommand};
use std::io::Read;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tessera")]
#[command(version = tessera_cli::VERSION_TEXT)]
#[command(about = "AI-friendly local LLM workbench")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(long, default_value = "tessera.toml")]
        config: PathBuf,
        #[arg(long)]
        force: bool,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
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
    Tasks {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
    Profiles {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        config: Option<PathBuf>,
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
    Agent {
        #[command(subcommand)]
        command: AgentCommands,
    },
    Instructions {
        #[command(subcommand)]
        command: InstructionsCommands,
    },
    Skills {
        #[command(subcommand)]
        command: SkillsCommands,
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
        list_commands: bool,
        #[arg(long)]
        resume: Option<String>,
        #[arg(long)]
        resume_task: Option<String>,
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

#[derive(Subcommand)]
enum ConfigCommands {
    Validate {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum AgentCommands {
    Run {
        #[arg(long, default_value = "mock")]
        provider: String,
        #[arg(long)]
        goal: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        instructions: bool,
        #[arg(long = "skill")]
        skills: Vec<String>,
        #[arg(long = "skill-reference")]
        skill_references: Vec<String>,
        #[arg(long)]
        workspace: Option<PathBuf>,
        #[arg(long)]
        target_dir: Option<PathBuf>,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        data_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum InstructionsCommands {
    Inspect {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        target_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum SkillsCommands {
    Inspect {
        #[arg(long)]
        workspace: PathBuf,
        #[arg(long)]
        target_dir: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Init { config, force }) => {
            let path = tessera_cli::write_config_template(config, force)?;
            println!("wrote {}", path.display());
        }
        Some(Commands::Config { command }) => match command {
            ConfigCommands::Validate {
                json,
                config,
                data_dir,
            } => {
                let config = tessera_cli::resolve_config(config)?;
                let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
                let report = tessera_cli::validate_config(&config, data_dir);
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    for line in tessera_cli::format_config_validation_lines(&report) {
                        println!("{line}");
                    }
                }
                if report.has_errors() {
                    anyhow::bail!("config validation failed");
                }
            }
        },
        Some(Commands::Doctor {
            json,
            config,
            data_dir,
        }) => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let report = tessera_cli::run_doctor_with_config(data_dir, &config)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for line in tessera_cli::format_doctor_lines(&report) {
                    println!("{line}");
                }
            }
        }
        Some(Commands::Sessions {
            json,
            config,
            data_dir,
        }) => {
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
        Some(Commands::Tasks {
            json,
            config,
            data_dir,
        }) => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let tasks = tessera_cli::list_resumable_tasks(data_dir, &config)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tasks)?);
            } else {
                for line in tessera_cli::format_resumable_task_lines(&tasks) {
                    println!("{line}");
                }
            }
        }
        Some(Commands::Profiles { json, config }) => {
            let config = tessera_cli::resolve_config(config)?;
            let profiles = tessera_cli::list_profiles(&config);
            if json {
                println!("{}", serde_json::to_string_pretty(&profiles)?);
            } else {
                for line in tessera_cli::format_profile_lines(&profiles) {
                    println!("{line}");
                }
            }
        }
        Some(Commands::Transcript {
            trace_id,
            json,
            config,
            data_dir,
        }) => {
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
        Some(Commands::Replay {
            trace_id,
            json,
            config,
            data_dir,
        }) => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let replay = tessera_cli::replay_trace(data_dir, &trace_id)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&replay)?);
            } else {
                print!("{}", tessera_cli::format_replay_summary(&replay));
            }
        }
        Some(Commands::Events {
            trace_id,
            json,
            since,
            limit,
            config,
            data_dir,
        }) => {
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
        Some(Commands::Agent { command }) => match command {
            AgentCommands::Run {
                provider,
                goal,
                json,
                instructions,
                skills,
                skill_references,
                workspace,
                target_dir,
                config,
                data_dir,
            } => {
                if !skill_references.is_empty() && skills.is_empty() {
                    anyhow::bail!("--skill-reference requires --skill");
                }
                if !instructions
                    && skills.is_empty()
                    && (workspace.is_some() || target_dir.is_some())
                {
                    anyhow::bail!("--workspace and --target-dir require --instructions or --skill");
                }
                let config = tessera_cli::resolve_config(config)?;
                let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
                let context_workspace = if instructions || !skills.is_empty() {
                    Some(workspace.unwrap_or(std::env::current_dir()?))
                } else {
                    None
                };
                let parsed_skill_references = skill_references
                    .into_iter()
                    .map(|reference| {
                        let (skill, path) = reference.split_once(':').ok_or_else(|| {
                            anyhow::anyhow!(
                                "--skill-reference must use <skill_id_or_name>:<relative/path>"
                            )
                        })?;
                        Ok((skill.to_string(), path.to_string()))
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                let instruction_options = if instructions {
                    Some(tessera_cli::CliInstructionContextOptions {
                        workspace: context_workspace
                            .clone()
                            .expect("context workspace is set when instructions are enabled"),
                        target_dir: target_dir.clone(),
                    })
                } else {
                    None
                };
                let skill_options = if !skills.is_empty() {
                    Some(tessera_cli::CliSkillContextOptions {
                        workspace: context_workspace
                            .expect("context workspace is set when skills are enabled"),
                        target_dir,
                        skills,
                        references: parsed_skill_references,
                    })
                } else {
                    None
                };
                let outcome = tessera_cli::run_agent_with_config_and_instruction_options(
                    data_dir,
                    &config,
                    &provider,
                    goal,
                    instruction_options,
                    skill_options,
                )
                .await?;
                if json {
                    let output = tessera_cli::CliAgentRunOutput::from(outcome);
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    for line in tessera_cli::format_agent_run_lines(&outcome) {
                        println!("{line}");
                    }
                }
            }
        },
        Some(Commands::Instructions { command }) => match command {
            InstructionsCommands::Inspect {
                workspace,
                target_dir,
                json,
            } => {
                let set = tessera_cli::inspect_instructions(workspace, target_dir)?;
                if json {
                    let output = tessera_cli::CliInstructionDiscoveryOutput::from(&set);
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    for line in tessera_cli::format_instruction_discovery_lines(&set) {
                        println!("{line}");
                    }
                }
            }
        },
        Some(Commands::Skills { command }) => match command {
            SkillsCommands::Inspect {
                workspace,
                target_dir,
                json,
            } => {
                let report = tessera_cli::inspect_skills(workspace, target_dir)?;
                if json {
                    let output = tessera_cli::CliSkillDiscoveryOutput::from(&report);
                    println!("{}", serde_json::to_string_pretty(&output)?);
                } else {
                    for line in tessera_cli::format_skill_discovery_lines(&report) {
                        println!("{line}");
                    }
                }
            }
        },
        Some(Commands::Chat {
            provider,
            prompt,
            stdin,
            file,
            json,
            continue_last,
            list_commands,
            resume,
            resume_task,
            config,
            data_dir,
        }) => {
            run_chat_command(ChatCommandOptions {
                provider,
                prompt,
                stdin,
                file,
                json,
                continue_last,
                list_commands,
                resume,
                resume_task,
                config,
                data_dir,
            })
            .await?;
        }
        Some(Commands::Tui {
            provider,
            config,
            data_dir,
        }) => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            tessera_cli::run_tui_with_config(data_dir, config, provider).await?;
        }
        None => {
            run_chat_command(ChatCommandOptions::default_interactive()).await?;
        }
    }

    Ok(())
}

struct ChatCommandOptions {
    provider: String,
    prompt: Option<String>,
    stdin: bool,
    file: Option<PathBuf>,
    json: bool,
    continue_last: bool,
    list_commands: bool,
    resume: Option<String>,
    resume_task: Option<String>,
    config: Option<PathBuf>,
    data_dir: Option<PathBuf>,
}

impl ChatCommandOptions {
    fn default_interactive() -> Self {
        Self {
            provider: "mock".to_string(),
            prompt: None,
            stdin: false,
            file: None,
            json: false,
            continue_last: false,
            list_commands: false,
            resume: None,
            resume_task: None,
            config: None,
            data_dir: None,
        }
    }
}

async fn run_chat_command(options: ChatCommandOptions) -> anyhow::Result<()> {
    if options.list_commands {
        for line in tessera_cli::chat_command_lines() {
            println!("{line}");
        }
        return Ok(());
    }

    let config = tessera_cli::resolve_config(options.config)?;
    let data_dir = tessera_cli::resolve_data_dir_with_config(options.data_dir, &config)?;
    let prompt_source_count = usize::from(options.prompt.is_some())
        + usize::from(options.stdin)
        + usize::from(options.file.is_some());
    if prompt_source_count > 1 {
        anyhow::bail!("--prompt, --stdin, and --file cannot be combined");
    }
    if options.resume_task.is_some()
        && (prompt_source_count > 0
            || options.resume.is_some()
            || options.continue_last
            || options.json)
    {
        anyhow::bail!(
            "--resume-task cannot be combined with --prompt, --stdin, --file, --resume, --continue, or --json"
        );
    }
    if options.continue_last
        && (prompt_source_count > 0 || options.resume.is_some() || options.resume_task.is_some())
    {
        anyhow::bail!(
            "--continue cannot be combined with --prompt, --stdin, --file, --resume, or --resume-task"
        );
    }
    if options.resume.is_some() && (prompt_source_count > 0 || options.resume_task.is_some()) {
        anyhow::bail!("--resume is only supported in interactive chat mode");
    }

    let prompt = if options.stdin {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        Some(input.trim_end_matches(['\r', '\n']).to_string())
    } else if let Some(path) = options.file {
        let input = std::fs::read_to_string(path)?;
        Some(input.trim_end_matches(['\r', '\n']).to_string())
    } else {
        options.prompt
    };

    if let Some(resume_task) = options.resume_task {
        let mut output = std::io::stdout();
        tessera_cli::resume_task_with_config(data_dir, &config, &resume_task, &mut output).await?;
    } else if let Some(prompt) = prompt {
        let outcome =
            tessera_cli::run_chat_with_config(data_dir, &config, &options.provider, prompt).await?;
        if options.json {
            let output = tessera_cli::CliChatOutput::from(outcome);
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{}", outcome.assistant_text);
        }
    } else {
        if options.json {
            anyhow::bail!("--json is only supported with --prompt, --stdin, or --file");
        }
        let resume = if options.continue_last {
            Some(tessera_cli::latest_session_trace_id(&data_dir)?)
        } else {
            options.resume
        };
        tessera_cli::run_chat_repl_with_config_and_resume(
            data_dir,
            config,
            options.provider,
            resume,
        )
        .await?;
    }

    Ok(())
}
