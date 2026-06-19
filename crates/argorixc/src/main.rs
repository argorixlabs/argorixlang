use anyhow::{bail, Context, Result};
use argorix_bytecode::{lower_ir, verify_bytecode, Instruction};
use argorix_ir::IrProgram;
use argorix_module::{check_package, package_ir, resolve_package, ModuleGraph, ResolvedPackage};
use argorix_parser::{parse_source, Diagnostic, Program};
use argorix_semantics::{check_program_with_options, CheckOptions};
use clap::{Parser, Subcommand};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Parser)]
#[command(name = "argorixc", version, about = "The Argorix Lang compiler")]
struct Cli {
    /// Allow undeclared capabilities in registry-free v0.1 source files.
    #[arg(long, global = true)]
    legacy_capabilities: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate syntax and semantics.
    Check { file: PathBuf },
    /// Compile source into Argorix IR JSON.
    EmitIr { file: PathBuf },
    /// Print protocol communication graphs.
    Graph { file: PathBuf },
    /// List the module capability registry.
    Capabilities { file: PathBuf },
    /// Compile source into Argorix Bytecode JSON.
    EmitBytecode { file: PathBuf },
    /// Compile and verify Argorix Bytecode.
    VerifyBytecode { file: PathBuf },
    /// Validate a multi-file package from its `argorix.toml` manifest (or directory).
    CheckPackage { manifest: PathBuf },
    /// Compile a package into Argorix IR JSON with module metadata.
    EmitIrPackage { manifest: PathBuf },
    /// Compile a package into Argorix Bytecode JSON with module metadata.
    EmitBytecodePackage { manifest: PathBuf },
    /// Print the deterministic module graph of a package.
    GraphPackage { manifest: PathBuf },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let options = CheckOptions {
        allow_legacy_capabilities: cli.legacy_capabilities,
    };
    match cli.command {
        Command::Check { file } => {
            let compiled = compile(&file, options)?;
            println!("Argorix Lang compiler v0.16\n");
            println!("File: {}", file.display());
            println!("Status: OK\n");
            println!("Module: {}", compiled.program.module.value);
            println!("Capabilities: {}", compiled.program.capabilities.len());
            println!("Types: {}", compiled.program.types.len());
            println!("Enums: {}", compiled.program.enums.len());
            println!("Agents: {}", compiled.program.agents.len());
            println!("Protocols: {}", compiled.program.protocols.len());
            println!("Semantic checks: passed");
        }
        Command::EmitIr { file } => {
            let compiled = compile(&file, options)?;
            let ir = IrProgram::from(&compiled.program);
            println!("{}", serde_json::to_string_pretty(&ir)?);
        }
        Command::Graph { file } => {
            let compiled = compile(&file, options)?;
            for (index, protocol) in compiled.program.protocols.iter().enumerate() {
                if index > 0 {
                    println!();
                }
                println!("Protocol: {}\n", protocol.name.value);
                for step in &protocol.steps {
                    println!(
                        "{} --{} {}--> {}",
                        step.from.value, step.act.value, step.message_type.value, step.to.value
                    );
                }
            }
        }
        Command::Capabilities { file } => {
            let compiled = compile(&file, options)?;
            println!("Capabilities\n");
            for capability in &compiled.program.capabilities {
                let suffix = if capability.requires_approval {
                    "  requires approval"
                } else {
                    ""
                };
                println!(
                    "{:<20} {:<12}{}",
                    capability.name.value,
                    capability.level.value.as_str(),
                    suffix
                );
            }
        }
        Command::EmitBytecode { file } => {
            let compiled = compile(&file, options)?;
            let ir = IrProgram::from(&compiled.program);
            let bytecode = lower_ir(&ir);
            verify_bytecode(&bytecode).map_err(bytecode_errors)?;
            println!("{}", serde_json::to_string_pretty(&bytecode)?);
        }
        Command::VerifyBytecode { file } => {
            let compiled = compile(&file, options)?;
            let ir = IrProgram::from(&compiled.program);
            let bytecode = lower_ir(&ir);
            verify_bytecode(&bytecode).map_err(bytecode_errors)?;
            let protocols = bytecode
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, Instruction::DeclareProtocol { .. }))
                .count();

            println!("Argorix Bytecode verification v0.16\n");
            println!("File: {}", file.display());
            println!("Status: OK\n");
            println!("Bytecode version: {}", bytecode.bytecode_version);
            println!("Instructions: {}", bytecode.instructions.len());
            println!("Agents: {}", bytecode.agents.len());
            println!("Protocols: {protocols}");
        }
        Command::CheckPackage { manifest } => {
            let package = resolve_package_arg(&manifest)?;
            let merged = check_package_program(&package)?;
            println!("Argorix Lang compiler v0.16\n");
            println!("Package entry: {}", package.graph.entry);
            println!("Modules: {}", package.graph.modules.len());
            println!("Imports: {}", package.graph.imports.len());
            println!("Agents: {}", merged.agents.len());
            println!("Protocols: {}", merged.protocols.len());
            println!("Semantic checks: passed");
        }
        Command::EmitIrPackage { manifest } => {
            let package = resolve_package_arg(&manifest)?;
            let merged = check_package_program(&package)?;
            let ir = package_ir(&merged, &package.graph);
            println!("{}", serde_json::to_string_pretty(&ir)?);
        }
        Command::EmitBytecodePackage { manifest } => {
            let package = resolve_package_arg(&manifest)?;
            let merged = check_package_program(&package)?;
            let ir = package_ir(&merged, &package.graph);
            let bytecode = lower_ir(&ir);
            verify_bytecode(&bytecode).map_err(bytecode_errors)?;
            println!("{}", serde_json::to_string_pretty(&bytecode)?);
        }
        Command::GraphPackage { manifest } => {
            let package = resolve_package_arg(&manifest)?;
            print_module_graph(&package.graph);
        }
    }
    Ok(())
}

/// Resolve a package from a manifest path or a directory containing `argorix.toml`.
fn resolve_package_arg(manifest: &Path) -> Result<ResolvedPackage> {
    let manifest_path = if manifest.is_dir() {
        manifest.join("argorix.toml")
    } else {
        manifest.to_path_buf()
    };
    resolve_package(&manifest_path).map_err(|error| anyhow::anyhow!("{error}"))
}

fn check_package_program(package: &ResolvedPackage) -> Result<Program> {
    check_package(package).map_err(|messages| anyhow::anyhow!("{}", messages.join("\n")))
}

fn print_module_graph(graph: &ModuleGraph) {
    println!("{}", graph.entry);
    let children: Vec<&str> = graph
        .imports
        .iter()
        .filter(|edge| edge.from == graph.entry)
        .map(|edge| edge.to.as_str())
        .collect();
    for (index, child) in children.iter().enumerate() {
        let connector = if index + 1 == children.len() {
            "└──"
        } else {
            "├──"
        };
        println!("{connector} {child}");
    }
}

fn bytecode_errors(errors: Vec<argorix_bytecode::BytecodeError>) -> anyhow::Error {
    anyhow::anyhow!(
        "{}",
        errors
            .into_iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    )
}

struct CompiledSource {
    program: Program,
}

fn compile(path: &Path, options: CheckOptions) -> Result<CompiledSource> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("argx") {
        bail!("Argorix source files must use the `.argx` extension");
    }

    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let file = path.display().to_string();
    let program = parse_source(&source)
        .map_err(|diagnostics| diagnostics_error(&diagnostics, &file, &source))?;
    check_program_with_options(&program, options)
        .map_err(|diagnostics| diagnostics_error(&diagnostics, &file, &source))?;

    Ok(CompiledSource { program })
}

fn diagnostics_error(diagnostics: &[Diagnostic], file: &str, source: &str) -> anyhow::Error {
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.render(file, source))
        .collect::<Vec<_>>()
        .join("\n\n");
    anyhow::anyhow!("{rendered}")
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use argorix_bytecode::{lower_ir, verify_bytecode};
    use argorix_ir::IrProgram;
    use argorix_parser::parse_source;
    use argorix_semantics::check_program;
    use clap::Parser;

    const SOURCE: &str = include_str!("../../../examples/prompt_defense_v02.argx");

    #[test]
    fn emit_bytecode_command_produces_valid_json() {
        let cli = Cli::try_parse_from([
            "argorixc",
            "emit-bytecode",
            "examples/prompt_defense_v02.argx",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::EmitBytecode { .. }));

        let program = parse_source(SOURCE).unwrap();
        check_program(&program).unwrap();
        let json = serde_json::to_string(&lower_ir(&IrProgram::from(&program))).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["bytecode_version"], "0.16");
    }

    #[test]
    fn verify_bytecode_command_accepts_valid_source() {
        let cli = Cli::try_parse_from([
            "argorixc",
            "verify-bytecode",
            "examples/prompt_defense_v02.argx",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::VerifyBytecode { .. }));

        let program = parse_source(SOURCE).unwrap();
        check_program(&program).unwrap();
        verify_bytecode(&lower_ir(&IrProgram::from(&program))).unwrap();
    }
}
