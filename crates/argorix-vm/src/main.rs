use anyhow::{bail, Context, Result};
use argorix_bytecode::BytecodeProgram;
use argorix_vm::{InjectedMessage, Vm, VmError};
use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[command(name = "argorix-vm", version, about = "Argorix Bytecode dry-run VM")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Run {
        file: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        mailboxes: bool,
        #[arg(long)]
        reactive: bool,
        #[arg(long)]
        inject: Option<String>,
        #[arg(long)]
        state: bool,
        #[arg(long)]
        tools: bool,
        #[arg(long)]
        models: bool,
        #[arg(long)]
        policy: bool,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Run {
            file,
            dry_run,
            json,
            mailboxes,
            reactive,
            inject,
            state,
            tools,
            models,
            policy,
        } => {
            if !dry_run {
                bail!("v0.9 only supports execution with `--dry-run`");
            }
            let source = fs::read_to_string(&file)
                .with_context(|| format!("failed to read `{}`", file.display()))?;
            let bytecode: BytecodeProgram = serde_json::from_str(&source)
                .with_context(|| format!("invalid bytecode JSON in `{}`", file.display()))?;
            if reactive {
                let injection = inject
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("`--reactive` requires `--inject`"))
                    .and_then(parse_injection)?;
                let trace = Vm::new().run_reactive(&bytecode, injection)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&trace)?);
                } else {
                    println!("Argorix VM v0.9\n");
                    println!("Execution mode: reactive dry-run");
                    println!("Scheduler: {}\n", trace.scheduler);
                    println!(
                        "Injected: {} --{} {}--> {}\n",
                        trace.injected.from,
                        trace.injected.act,
                        trace.injected.message_type,
                        trace.injected.to
                    );
                    for step in &trace.steps {
                        println!(
                            "Step {}: delivered {} to {}.mailbox",
                            step.index, step.handled, step.agent
                        );
                        println!(
                            "Step {}: {} handled {}",
                            step.index, step.agent, step.handled
                        );
                        for intrinsic in &step.intrinsics {
                            println!(
                                "Step {}: {} invoked {}({})",
                                step.index, step.agent, intrinsic.name, intrinsic.argument
                            );
                        }
                        for tool in &step.tool_calls {
                            println!(
                                "Step {}: {} requested tool {}",
                                step.index, step.agent, tool
                            );
                            if let Some(call) = trace
                                .tool_calls
                                .iter()
                                .find(|call| call.agent == step.agent && call.tool == *tool)
                            {
                                println!(
                                    "Step {}: Tool {} allowed by capability {}",
                                    step.index, tool, call.capability
                                );
                                println!(
                                    "Step {}: Tool {} dry-run result generated",
                                    step.index, tool
                                );
                            }
                        }
                        for model in &step.model_calls {
                            println!("Step {}: {} asked model {}", step.index, step.agent, model);
                            if let Some(call) = trace
                                .model_calls
                                .iter()
                                .find(|call| call.agent == step.agent && call.model == *model)
                            {
                                println!(
                                    "Step {}: Model {} allowed by capability {}",
                                    step.index, model, call.capability
                                );
                                println!(
                                    "Step {}: Model {} dry-run result generated",
                                    step.index, model
                                );
                            }
                        }
                        for emitted in &step.emitted {
                            println!(
                                "Step {}: {} emitted {} to {}",
                                step.index, step.agent, emitted.message_type, emitted.to
                            );
                        }
                        for binding in &step.traced {
                            println!("Step {}: {} traced {}", step.index, step.agent, binding);
                        }
                        if step.halted {
                            println!("Step {}: {} halted execution", step.index, step.agent);
                        }
                        println!();
                    }
                    if state {
                        println!("Agent state:");
                        for agent in &trace.agent_state {
                            println!(
                                "- {}: handled={}, checkpoints={}",
                                agent.agent, agent.handled_count, agent.checkpoints
                            );
                        }
                        println!();
                    }
                    if tools {
                        println!("Tool calls:");
                        for call in &trace.tool_calls {
                            println!(
                                "- {} -> {}: {}, {}",
                                call.agent, call.tool, call.status, call.mode
                            );
                        }
                        println!();
                    }
                    if models {
                        println!("Model calls:");
                        for call in &trace.model_calls {
                            println!(
                                "- {} -> {}: {}, {} {}",
                                call.agent, call.model, call.status, call.provider, call.mode
                            );
                        }
                        println!();
                    }
                    if policy {
                        println!("Policy assertions:");
                        for assertion in &trace.policy_report.assertions {
                            let argument = assertion
                                .argument
                                .as_deref()
                                .map(|value| format!(" {value}"))
                                .unwrap_or_default();
                            println!("- {}{}: {}", assertion.name, argument, assertion.status);
                        }
                        println!("\nPolicy report: {}", trace.policy_report.status);
                    }
                    println!("Status: {}", trace.status);
                    println!("Trace ledger: generated");
                }
                return Ok(());
            }
            let trace = Vm::new().run_dry(&bytecode)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&trace)?);
            } else if mailboxes {
                println!("Argorix VM v0.9\n");
                println!("Execution mode: dry-run");
                println!("Scheduler: {}", trace.scheduler);
                println!("Agents: {}\n", trace.mailboxes.len());
                println!("Mailboxes initialized:");
                for mailbox in &trace.mailboxes {
                    println!("- {}", mailbox.agent);
                }
                println!();
                for step in &trace.steps {
                    println!(
                        "Step {}: scheduled {} --{} {}--> {}",
                        step.index, step.from, step.act, step.message_type, step.to
                    );
                    println!("Step {}: delivered to {}.mailbox", step.index, step.to);
                    println!("Step {}: processed by {}\n", step.index, step.to);
                }
                println!("Status: {}", trace.status);
                println!("Trace ledger: generated");
            } else {
                println!("Argorix VM v0.9\n");
                println!("Loaded bytecode: {}", file.display());
                println!("Execution mode: dry-run\n");
                for step in &trace.steps {
                    println!(
                        "Step {}: {} --{} {}--> {}",
                        step.index, step.from, step.act, step.message_type, step.to
                    );
                }
                println!("\nSecurity checks: {}", trace.security_checks);
                println!("Trace: generated");
                println!("Status: {}", trace.status);
            }
        }
    }
    Ok(())
}

fn parse_injection(value: &str) -> Result<InjectedMessage> {
    let parts = value.split(':').collect::<Vec<_>>();
    if parts.len() != 4 || parts.iter().any(|part| part.trim().is_empty()) {
        return Err(VmError::InvalidInjection(value.to_owned()).into());
    }
    Ok(InjectedMessage {
        from: parts[0].to_owned(),
        to: parts[1].to_owned(),
        act: parts[2].to_owned(),
        message_type: parts[3].to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_injection, Cli, Command};
    use clap::Parser;

    #[test]
    fn cli_accepts_reactive_injection() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::Run {
                reactive: true,
                inject: Some(_),
                ..
            }
        ));
        assert_eq!(
            parse_injection("User:Worker:tell:Ping").unwrap().to,
            "Worker"
        );
    }

    #[test]
    fn cli_accepts_state_flag() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--state",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::Run { state: true, .. }));
    }

    #[test]
    fn cli_accepts_tools_flag() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--tools",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::Run { tools: true, .. }));
    }

    #[test]
    fn cli_accepts_models_flag() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--models",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::Run { models: true, .. }));
    }

    #[test]
    fn cli_accepts_policy_flag() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--policy",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::Run { policy: true, .. }));
    }
}
