use anyhow::{bail, Context, Result};
use argorix_bytecode::BytecodeProgram;
use argorix_vm::{
    evidence::{verify_evidence, EvidenceBundle},
    parse_injection, ReactiveExecutionTrace, SecurityReport, Vm,
};
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
        #[arg(long)]
        providers: bool,
        #[arg(long)]
        provider_contracts: bool,
        #[arg(long, value_name = "PATH")]
        security_report: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        trace_out: Option<PathBuf>,
        #[arg(long, value_name = "PATH")]
        evidence_bundle: Option<PathBuf>,
    },
    VerifyEvidence {
        bundle: PathBuf,
        #[arg(long)]
        json: bool,
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
            providers,
            provider_contracts,
            security_report,
            trace_out,
            evidence_bundle,
        } => {
            if !dry_run {
                bail!("v0.17 only supports execution with `--dry-run`");
            }
            let source = fs::read_to_string(&file)
                .with_context(|| format!("failed to read `{}`", file.display()))?;
            let bytecode: BytecodeProgram = serde_json::from_str(&source)
                .with_context(|| format!("invalid bytecode JSON in `{}`", file.display()))?;
            if reactive {
                let injection = inject
                    .as_deref()
                    .ok_or_else(|| anyhow::anyhow!("`--reactive` requires `--inject`"))
                    .and_then(|value| parse_injection(value).map_err(Into::into))?;
                let outcome = Vm::new().run_reactive_outcome(&bytecode, injection);
                let report = SecurityReport::from_outcome(&bytecode, &outcome);
                if let Some(path) = security_report.as_deref() {
                    write_security_report(path, &report)?;
                }
                if let (Some(path), Ok(trace)) = (trace_out.as_deref(), outcome.result.as_ref()) {
                    write_trace(path, trace)?;
                }
                if let Some(path) = evidence_bundle.as_deref() {
                    let bundle = EvidenceBundle::from_outcome(
                        &bytecode,
                        &outcome,
                        &report,
                        path,
                        Some(&file),
                        trace_out.as_deref(),
                        security_report.as_deref(),
                    )?;
                    write_evidence_bundle(path, &bundle)?;
                }
                let trace = outcome.result?;
                let blocked_policy = trace
                    .policy_report
                    .actions
                    .iter()
                    .find(|action| action.action == "block")
                    .map(|action| action.policy.clone());
                if json {
                    println!("{}", serde_json::to_string_pretty(&trace)?);
                } else {
                    println!("Argorix VM v0.17\n");
                    println!("Execution mode: reactive dry-run");
                    println!("Scheduler: {}", trace.scheduler);
                    if providers {
                        println!("Provider boundary: enabled\n");
                        println!("Provider registry:");
                        for provider in &trace.providers {
                            println!("- {}: {}, executable", provider.name, provider.kind);
                        }
                        println!();
                    } else {
                        println!();
                    }
                    if provider_contracts {
                        println!("Provider contracts:");
                        for contract in &trace.provider_contracts {
                            println!(
                                "- {}: {}, disabled, dry-run-only, requires feature_flag, requires approval",
                                contract.name, contract.kind
                            );
                            println!(
                                "  allowed_targets: {}",
                                format_allowlist(&contract.allowed_targets)
                            );
                            println!(
                                "  allowed_capabilities: {}",
                                format_allowlist(&contract.allowed_capabilities)
                            );
                        }
                        println!("- simulated: executable\n");
                        println!("External provider execution: blocked by design\n");
                    }
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
                                if providers {
                                    if let Some(provider_call) =
                                        trace.provider_calls.iter().find(|provider_call| {
                                            provider_call.kind == "tool"
                                                && provider_call.target == *tool
                                        })
                                    {
                                        println!(
                                            "Step {}: Provider {} selected for tool {}",
                                            step.index, provider_call.provider, tool
                                        );
                                        println!("Step {}: Provider request created", step.index);
                                        println!("Step {}: Provider response received", step.index);
                                    }
                                }
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
                                if providers {
                                    if let Some(provider_call) =
                                        trace.provider_calls.iter().find(|provider_call| {
                                            provider_call.kind == "model"
                                                && provider_call.target == *model
                                        })
                                    {
                                        println!(
                                            "Step {}: Provider {} selected for model {}",
                                            step.index, provider_call.provider, model
                                        );
                                        println!("Step {}: Provider request created", step.index);
                                        println!("Step {}: Provider response received", step.index);
                                    }
                                }
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
                        println!("Legacy policy assertions:");
                        for assertion in &trace.policy_report.assertions {
                            let argument = assertion
                                .argument
                                .as_deref()
                                .map(|value| format!(" {value}"))
                                .unwrap_or_default();
                            println!("- {}{}: {}", assertion.name, argument, assertion.status);
                        }
                        println!();
                        println!("Policy blocks:");
                        for block in &trace.policy_report.policy_blocks {
                            println!("- {}: {}", block.name, block.status);
                            for rule in block
                                .require_rules
                                .iter()
                                .chain(block.deny_rules.iter())
                            {
                                println!(
                                    "  {} {}: {}",
                                    rule.effect,
                                    rule.rule,
                                    if rule.passed { "passed" } else { "failed" }
                                );
                            }
                            for violation in &block.violations {
                                println!(
                                    "  violation: {} {} - {}",
                                    violation.effect, violation.rule, violation.reason
                                );
                            }
                            if let Some(action) = &block.action {
                                println!(
                                    "  action: {}, trace_required={}",
                                    action, block.trace_required
                                );
                            }
                        }
                        println!("\nPolicy report: {}", trace.policy_report.status);
                    }
                    if providers {
                        println!("Provider calls:");
                        for call in &trace.provider_calls {
                            println!(
                                "- {} {} {}: {}, simulated={}",
                                call.provider, call.kind, call.target, call.status, call.simulated
                            );
                        }
                        println!();
                    }
                    println!("Status: {}", trace.status);
                    println!("Trace ledger: generated");
                    if let Some(path) = security_report.as_deref() {
                        println!("Security report written: {}", path.display());
                    }
                    if let Some(path) = trace_out.as_deref() {
                        println!("Trace written: {}", path.display());
                    }
                    if let Some(path) = evidence_bundle.as_deref() {
                        println!("Evidence bundle written: {}", path.display());
                    }
                }
                if let Some(policy) = blocked_policy {
                    bail!("policy `{policy}` activated block action");
                }
                return Ok(());
            }
            let trace = Vm::new().run_dry(&bytecode)?;

            if json {
                println!("{}", serde_json::to_string_pretty(&trace)?);
            } else if mailboxes {
                println!("Argorix VM v0.17\n");
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
                println!("Argorix VM v0.17\n");
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
        Command::VerifyEvidence { bundle, json } => {
            let result = verify_evidence(&bundle)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else if result.passed {
                println!("Evidence verification: passed");
            } else {
                println!("Evidence verification: failed");
                for failure in &result.failures {
                    println!("- {failure}");
                }
            }
            if !result.passed {
                bail!("evidence verification failed");
            }
        }
    }
    Ok(())
}

fn write_security_report(path: &std::path::Path, report: &SecurityReport) -> Result<()> {
    write_pretty_json(path, report, "security report")
}

fn write_trace(path: &std::path::Path, trace: &ReactiveExecutionTrace) -> Result<()> {
    write_pretty_json(path, trace, "trace")
}

fn write_evidence_bundle(path: &std::path::Path, bundle: &EvidenceBundle) -> Result<()> {
    write_pretty_json(path, bundle, "evidence bundle")
}

fn write_pretty_json<T: serde::Serialize>(
    path: &std::path::Path,
    value: &T,
    label: &str,
) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).with_context(|| {
            format!("failed to create {label} directory `{}`", parent.display())
        })?;
    }
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write {label} `{}`", path.display()))
}
fn format_allowlist(values: &[String]) -> String {
    if values.is_empty() {
        "none".into()
    } else {
        values.join(", ")
    }
}
#[cfg(test)]
mod tests {
    use super::{format_allowlist, parse_injection, Cli, Command};
    use clap::Parser;

    #[test]
    fn formats_populated_and_empty_allowlists() {
        assert_eq!(format_allowlist(&[]), "none");
        assert_eq!(
            format_allowlist(&["GuardModel".into(), "WebSearch".into()]),
            "GuardModel, WebSearch"
        );
    }
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

    #[test]
    fn cli_accepts_providers_flag() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--providers",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::Run {
                providers: true,
                ..
            }
        ));
    }

    #[test]
    fn cli_accepts_security_report_path() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--security-report",
            "reports/run.security.json",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::Run {
                security_report: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn cli_accepts_evidence_and_trace_paths() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--trace-out",
            "reports/run.trace.json",
            "--evidence-bundle",
            "reports/run.bundle.json",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::Run {
                trace_out: Some(_),
                evidence_bundle: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn cli_accepts_verify_evidence_json() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "verify-evidence",
            "reports/run.bundle.json",
            "--json",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::VerifyEvidence { json: true, .. }
        ));
    }

    #[test]
    fn cli_accepts_provider_contracts_flag() {
        let cli = Cli::try_parse_from([
            "argorix-vm",
            "run",
            "program.json",
            "--dry-run",
            "--reactive",
            "--inject",
            "User:Worker:tell:Ping",
            "--provider-contracts",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Command::Run {
                provider_contracts: true,
                ..
            }
        ));
    }
}
