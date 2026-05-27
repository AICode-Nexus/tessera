use clap::{Args, Parser, Subcommand};
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
        owners: bool,
        #[arg(long)]
        trace: Option<String>,
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
    ApplyPatch(Box<ApplyPatchCommandOptions>),
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

#[derive(Args)]
struct ApplyPatchCommandOptions {
    #[arg(long)]
    trace_id: Option<String>,
    #[arg(long)]
    from_trace: Option<String>,
    #[arg(long)]
    workflow_id: Option<String>,
    #[arg(long)]
    task_id: Option<String>,
    #[arg(long)]
    request_id: Option<String>,
    #[arg(long)]
    patch_id: Option<String>,
    #[arg(long)]
    patch_artifact_id: Option<String>,
    #[arg(long)]
    preflight_id: Option<String>,
    #[arg(long)]
    execution_id: Option<String>,
    #[arg(long)]
    checkpoint_id: Option<String>,
    #[arg(long)]
    reviewer_gate_id: Option<String>,
    #[arg(long)]
    policy_decision_id: Option<String>,
    #[arg(long)]
    sandbox_profile: Option<String>,
    #[arg(long)]
    isolated_root: Option<PathBuf>,
    #[arg(long)]
    root_label: Option<String>,
    #[arg(long = "allowed-path")]
    allowed_paths: Vec<String>,
    #[arg(long)]
    auto_worktree: bool,
    #[arg(long)]
    worktree_base: Option<PathBuf>,
    #[arg(long)]
    worktree_retention: Option<String>,
    #[arg(long)]
    source_root: Option<PathBuf>,
    #[arg(long)]
    patch_file: Option<PathBuf>,
    #[arg(long)]
    stdin: bool,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    json: bool,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    data_dir: Option<PathBuf>,
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
            owners,
            trace,
            config,
            data_dir,
        }) => {
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            if owners {
                let trace_id =
                    trace.ok_or_else(|| anyhow::anyhow!("--trace is required with --owners"))?;
                let owners = tessera_cli::list_task_owners(data_dir, &trace_id)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&owners)?);
                } else {
                    for line in tessera_cli::format_task_owner_lines(&owners) {
                        println!("{line}");
                    }
                }
            } else {
                let tasks = tessera_cli::list_resumable_tasks(data_dir, &config)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&tasks)?);
                } else {
                    for line in tessera_cli::format_resumable_task_lines(&tasks) {
                        println!("{line}");
                    }
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
        Some(Commands::ApplyPatch(options)) => {
            let ApplyPatchCommandOptions {
                trace_id,
                from_trace,
                workflow_id,
                task_id,
                request_id,
                patch_id,
                patch_artifact_id,
                preflight_id,
                execution_id,
                checkpoint_id,
                reviewer_gate_id,
                policy_decision_id,
                sandbox_profile,
                isolated_root,
                root_label,
                allowed_paths,
                auto_worktree,
                worktree_base,
                worktree_retention,
                source_root,
                patch_file,
                stdin,
                dry_run,
                json,
                config,
                data_dir,
            } = *options;
            let config = tessera_cli::resolve_config(config)?;
            let data_dir = tessera_cli::resolve_data_dir_with_config(data_dir, &config)?;
            let patch_source_count = usize::from(patch_file.is_some())
                + usize::from(stdin)
                + usize::from(patch_artifact_id.is_some());

            if auto_worktree {
                if from_trace.is_none() {
                    anyhow::bail!("--auto-worktree requires --from-trace");
                }
                if isolated_root.is_some() || root_label.is_some() {
                    anyhow::bail!(
                        "--auto-worktree cannot be combined with --isolated-root or --root-label"
                    );
                }
            } else if worktree_base.is_some()
                || worktree_retention.is_some()
                || source_root.is_some()
            {
                anyhow::bail!(
                    "--worktree-base, --worktree-retention, and --source-root require --auto-worktree"
                );
            }

            if let Some(_from_trace) = &from_trace {
                if trace_id.is_some() {
                    anyhow::bail!("--trace-id cannot be combined with --from-trace");
                }
                if task_id.is_some()
                    || checkpoint_id.is_some()
                    || reviewer_gate_id.is_some()
                    || policy_decision_id.is_some()
                    || sandbox_profile.is_some()
                {
                    anyhow::bail!(
                        "--task-id, --checkpoint-id, --reviewer-gate-id, --policy-decision-id, and --sandbox-profile are explicit mode only"
                    );
                }
                if patch_source_count > 1 {
                    anyhow::bail!(
                        "at most one of --patch-artifact-id, --patch-file, or --stdin is allowed with --from-trace"
                    );
                }
            } else {
                if patch_artifact_id.is_some() {
                    anyhow::bail!("--patch-artifact-id requires --from-trace");
                }
                if patch_source_count != 1 {
                    anyhow::bail!("exactly one of --patch-file or --stdin is required");
                }
            }

            let patch_body_override = if stdin {
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input)?;
                Some(input)
            } else if let Some(patch_file) = patch_file {
                Some(std::fs::read_to_string(patch_file)?)
            } else {
                None
            };

            let output = if let Some(from_trace) = from_trace {
                if auto_worktree {
                    tessera_cli::run_apply_patch_auto_worktree_options(
                        data_dir,
                        tessera_cli::CliAutoWorktreeApplyPatchOptions {
                            trace_id: from_trace,
                            workflow_id,
                            request_id,
                            patch_id,
                            patch_artifact_id,
                            preflight_id,
                            execution_id,
                            allowed_paths,
                            patch_body_override,
                            dry_run,
                            operator_label: "cli".to_string(),
                            source_root,
                            worktree_base,
                            retention_policy: tessera_cli::parse_worktree_retention_policy(
                                worktree_retention.as_deref(),
                            )?,
                        },
                    )?
                } else {
                    tessera_cli::run_apply_patch_from_trace_options(
                        data_dir,
                        tessera_cli::CliTraceApplyPatchOptions {
                            trace_id: from_trace,
                            workflow_id,
                            request_id,
                            patch_id,
                            patch_artifact_id,
                            preflight_id,
                            execution_id,
                            isolated_root: required_apply_patch_path_arg(
                                isolated_root,
                                "--isolated-root",
                            )?,
                            root_label: required_apply_patch_arg(root_label, "--root-label")?,
                            allowed_paths,
                            patch_body_override,
                            dry_run,
                            operator_label: "cli".to_string(),
                        },
                    )?
                }
            } else {
                let trace_id = trace_id.unwrap_or_else(|| {
                    format!(
                        "trace_apply_patch_{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|duration| duration.as_millis())
                            .unwrap_or(0)
                    )
                });
                tessera_cli::run_apply_patch_with_options(
                    data_dir,
                    tessera_cli::CliApplyPatchOptions {
                        trace_id,
                        workflow_id: required_apply_patch_arg(workflow_id, "--workflow-id")?,
                        task_id: required_apply_patch_arg(task_id, "--task-id")?,
                        request_id: required_apply_patch_arg(request_id, "--request-id")?,
                        patch_id: required_apply_patch_arg(patch_id, "--patch-id")?,
                        preflight_id: required_apply_patch_arg(preflight_id, "--preflight-id")?,
                        execution_id: required_apply_patch_arg(execution_id, "--execution-id")?,
                        checkpoint_id: required_apply_patch_arg(checkpoint_id, "--checkpoint-id")?,
                        reviewer_gate_id: required_apply_patch_arg(
                            reviewer_gate_id,
                            "--reviewer-gate-id",
                        )?,
                        policy_decision_id: required_apply_patch_arg(
                            policy_decision_id,
                            "--policy-decision-id",
                        )?,
                        sandbox_profile_label: required_apply_patch_arg(
                            sandbox_profile,
                            "--sandbox-profile",
                        )?,
                        isolated_root: required_apply_patch_path_arg(
                            isolated_root,
                            "--isolated-root",
                        )?,
                        root_label: required_apply_patch_arg(root_label, "--root-label")?,
                        allowed_paths,
                        patch_body: patch_body_override
                            .expect("patch body exists when explicit patch source count is one"),
                        dry_run,
                        operator_label: "cli".to_string(),
                    },
                )?
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else if let Some(status) = &output.execution_status {
                let worktree = output
                    .worktree_path
                    .as_ref()
                    .map(|path| format!(" worktree={path}"))
                    .unwrap_or_default();
                println!(
                    "apply-patch execution {status} affected={} trace={}{}",
                    output.affected_paths.join(","),
                    output.trace_id,
                    worktree
                );
            } else {
                let worktree = output
                    .worktree_path
                    .as_ref()
                    .map(|path| format!(" worktree={path}"))
                    .unwrap_or_default();
                println!(
                    "apply-patch preflight {} affected={} trace={}{}",
                    output.preflight_status,
                    output.affected_paths.join(","),
                    output.trace_id,
                    worktree
                );
            }
        }
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

fn required_apply_patch_arg(value: Option<String>, name: &str) -> anyhow::Result<String> {
    value.ok_or_else(|| anyhow::anyhow!("{name} is required unless --from-trace is used"))
}

fn required_apply_patch_path_arg(value: Option<PathBuf>, name: &str) -> anyhow::Result<PathBuf> {
    value.ok_or_else(|| anyhow::anyhow!("{name} is required unless --auto-worktree is used"))
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
