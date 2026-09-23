use anyhow::{bail, Context, Result};
use argorix_bytecode::{lower_ir, source_digest, verify_bytecode, BytecodeProgram, Instruction};
use argorix_ir::{
    lower_core_program, verify_core_ir, CoreCBackend, CoreIrBackend, CoreIrProgram, IrProgram,
};
use argorix_module::{check_package, package_ir, resolve_package, ModuleGraph, ResolvedPackage};
use argorix_parser::{
    core::{core_token_dump, parse_core_source, CoreDiagnostic, CoreItemKind, CoreProgram},
    parse_source, Diagnostic, Program,
};
use argorix_semantics::{
    check_core_program, check_program_with_options, core_link_order, link_core_program,
    verify_core_program, CheckOptions, CoreCheckOptions, CoreLinkError,
};
use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, BTreeSet};
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

    /// Directory of the Argorix Core standard library. Its modules join the
    /// locked compilation set of a Core program; nothing is found by searching.
    #[arg(long, global = true)]
    stdlib: Option<PathBuf>,

    /// Another directory whose Core modules join the locked compilation set,
    /// such as `compiler/` for a program that imports the Argorix compiler.
    #[arg(long = "modules", global = true)]
    modules: Vec<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate syntax and semantics.
    Check { file: PathBuf },
    /// Validate an explicitly versioned Argorix Core 0.1 source file (stage0).
    CoreCheck { file: PathBuf },
    /// Lower checked Argorix Core 0.1 source into versioned Core IR JSON.
    CoreEmitIr { file: PathBuf },
    /// Verify serialized Argorix Core IR JSON.
    CoreVerifyIr { file: PathBuf },
    /// Print the canonical token dump of a Core source file (spec/core/tokens.md).
    CoreTokens { file: PathBuf },
    /// Lower checked Argorix Core 0.1 source into transitional C11.
    CoreEmitC {
        file: PathBuf,
        /// Write generated C to a file instead of standard output.
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
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
            println!("Argorix Lang compiler v1.0\n");
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
        Command::CoreTokens { file } => {
            let source =
                fs::read(&file).with_context(|| format!("failed to read `{}`", file.display()))?;
            print!("{}", core_token_dump(&source));
        }
        Command::CoreCheck { file } => {
            let compiled = compile_core(&file, cli.stdlib.as_deref(), &cli.modules)?;
            let functions = compiled
                .root
                .items
                .iter()
                .filter(|item| matches!(item.kind, CoreItemKind::Function(_)))
                .count();
            let types = compiled.root.items.len() - functions;
            println!(
                "Argorix Core stage0 frontend v{}\n",
                compiled.root.version.value
            );
            println!("File: {}", file.display());
            println!("Status: OK\n");
            println!("Module: {}", compiled.root.module.value);
            println!("Imports: {}", compiled.root.imports.len());
            println!("Linked modules: {}", compiled.linked_modules);
            println!("Types/constants: {types}");
            println!("Functions: {functions}");
            println!("Semantic checks: passed");
            println!("Core IR: available through core-emit-ir");
            println!("Execution: available through core-emit-c");
        }
        Command::CoreEmitIr { file } => {
            let compiled = compile_core(&file, cli.stdlib.as_deref(), &cli.modules)?;
            let verified = verify_core_program(
                &compiled.program,
                &CoreCheckOptions {
                    available_modules: compiled.available_modules,
                },
            )
            .map_err(|diagnostics| {
                core_diagnostics_error(&diagnostics, &file.display().to_string(), &compiled.source)
            })?;
            let ir = lower_core_program(verified);
            verify_core_ir(&ir).map_err(core_ir_errors)?;
            println!("{}", serde_json::to_string_pretty(&ir)?);
        }
        Command::CoreVerifyIr { file } => {
            let source = fs::read_to_string(&file)
                .with_context(|| format!("failed to read `{}`", file.display()))?;
            let ir: CoreIrProgram = serde_json::from_str(&source)
                .with_context(|| format!("invalid Argorix Core IR JSON in `{}`", file.display()))?;
            let verified = verify_core_ir(&ir).map_err(core_ir_errors)?;
            println!("Argorix Core IR verifier v{}\n", ir.ir_version);
            println!("File: {}", file.display());
            println!("Status: OK\n");
            println!("Module: {}", ir.module);
            println!("Items: {}", ir.items.len());
            println!("Effects: {}", ir.effect_policy.len());
            println!("Semantic fingerprint: {}", verified.semantic_fingerprint());
        }
        Command::CoreEmitC { file, output } => {
            let compiled = compile_core(&file, cli.stdlib.as_deref(), &cli.modules)?;
            let checked = verify_core_program(
                &compiled.program,
                &CoreCheckOptions {
                    available_modules: compiled.available_modules,
                },
            )
            .map_err(|diagnostics| {
                core_diagnostics_error(&diagnostics, &file.display().to_string(), &compiled.source)
            })?;
            let ir = lower_core_program(checked);
            let verified = verify_core_ir(&ir).map_err(core_ir_errors)?;
            let generated = CoreCBackend
                .emit(verified)
                .map_err(|error| anyhow::anyhow!(error))?;
            if let Some(path) = output {
                fs::write(&path, generated.source)
                    .with_context(|| format!("failed to write `{}`", path.display()))?;
            } else {
                print!("{}", generated.source);
            }
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
            let mut bytecode = lower_ir(&ir);
            // Bind the emitted program to the exact source bytes it came from.
            // Only the compiler can assert this; the VM never sees the source.
            bytecode.source_digest = Some(source_digest(compiled.source.as_bytes()));
            verify_bytecode(&bytecode).map_err(bytecode_errors)?;
            println!("{}", serde_json::to_string_pretty(&bytecode)?);
        }
        Command::VerifyBytecode { file } => {
            let bytecode = load_bytecode_for_verification(&file, options)?;
            verify_bytecode(&bytecode).map_err(bytecode_errors)?;
            let protocols = bytecode
                .instructions
                .iter()
                .filter(|instruction| matches!(instruction, Instruction::DeclareProtocol { .. }))
                .count();

            println!("Argorix Bytecode verification v1.0\n");
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
            println!("Argorix Lang compiler v1.0\n");
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

fn load_bytecode_for_verification(path: &Path, options: CheckOptions) -> Result<BytecodeProgram> {
    let is_serialized_bytecode = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".argbc.json"));
    if is_serialized_bytecode {
        let source = fs::read_to_string(path)
            .with_context(|| format!("failed to read `{}`", path.display()))?;
        serde_json::from_str(&source)
            .with_context(|| format!("invalid Argorix Bytecode JSON in `{}`", path.display()))
    } else {
        let compiled = compile(path, options)?;
        Ok(lower_ir(&IrProgram::from(&compiled.program)))
    }
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
    source: String,
}

fn compile(path: &Path, options: CheckOptions) -> Result<CompiledSource> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("argx") {
        bail!("Argorix source files must use the `.argx` extension");
    }

    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    if source.trim_start().starts_with("core ") {
        bail!(
            "Core sources are isolated from the historical compiler; use `argorixc core-check {}` (IR and execution arrive in ESP-007/008)",
            path.display()
        );
    }
    let file = path.display().to_string();
    let program = parse_source(&source)
        .map_err(|diagnostics| diagnostics_error(&diagnostics, &file, &source))?;
    check_program_with_options(&program, options)
        .map_err(|diagnostics| diagnostics_error(&diagnostics, &file, &source))?;

    Ok(CompiledSource { program, source })
}

struct CheckedCoreSource {
    /// The root module linked with every module it imports, ready to lower.
    program: CoreProgram,
    /// The root module as written, for the summary `core-check` prints.
    root: CoreProgram,
    source: String,
    available_modules: BTreeSet<String>,
    /// How many modules went into `program`, the root included.
    linked_modules: usize,
}

fn compile_core(
    path: &Path,
    stdlib: Option<&Path>,
    module_dirs: &[PathBuf],
) -> Result<CheckedCoreSource> {
    if path.extension().and_then(|extension| extension.to_str()) != Some("argx") {
        bail!("Argorix source files must use the `.argx` extension");
    }
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read `{}`", path.display()))?;
    let file = path.display().to_string();
    let program = parse_core_source(&source)
        .map_err(|diagnostics| core_diagnostics_error(&diagnostics, &file, &source))?;
    let root_name = program.module.value.clone();

    // The locked compilation set is the directory the root lives in. A bare
    // file name has an empty parent, which means the current directory.
    let directory = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => PathBuf::from("."),
    };
    let root_identity = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut modules: BTreeMap<String, CoreProgram> = BTreeMap::new();
    let mut files: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    files.insert(root_name.clone(), (file.clone(), source.clone()));
    // The standard library joins the set only when the driver names it.
    let mut directories = vec![directory.clone()];
    for extra in module_dirs.iter().map(PathBuf::as_path).chain(stdlib) {
        let identity = fs::canonicalize(extra).ok();
        if !directories
            .iter()
            .any(|known| fs::canonicalize(known).ok() == identity)
        {
            directories.push(extra.to_path_buf());
        }
    }
    let mut candidates = Vec::new();
    for scanned in &directories {
        for entry in fs::read_dir(scanned)
            .with_context(|| format!("failed to enumerate `{}`", scanned.display()))?
        {
            candidates.push(entry?.path());
        }
    }
    for candidate in candidates {
        if candidate
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("argx")
        {
            continue;
        }
        if fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone()) == root_identity {
            continue;
        }
        let Ok(candidate_source) = fs::read_to_string(&candidate) else {
            continue;
        };
        // A sibling that does not parse cannot be imported; it is only an
        // error if something imports the module it meant to declare.
        let Ok(candidate_program) = parse_core_source(&candidate_source) else {
            continue;
        };
        let name = candidate_program.module.value.clone();
        if name == root_name || modules.contains_key(&name) {
            duplicates.insert(name);
            continue;
        }
        files.insert(
            name.clone(),
            (candidate.display().to_string(), candidate_source),
        );
        modules.insert(name, candidate_program);
    }
    if duplicates.contains(&root_name) {
        bail!(
            "{file}: module `{root_name}` is declared by more than one file in `{}`",
            directory.display()
        );
    }
    let mut available_modules: BTreeSet<String> = modules.keys().cloned().collect();
    available_modules.insert(root_name.clone());
    let options = CoreCheckOptions {
        available_modules: available_modules.clone(),
    };
    let link_error = |error: CoreLinkError| -> anyhow::Error {
        let (file, source) = &files[&error.module];
        core_diagnostics_error(&error.diagnostics, file, source)
    };

    // Dependencies are checked first, each linked as a root of its own, so a
    // diagnostic is always rendered against the file that contains it.
    let order = core_link_order(&program, &modules, &duplicates).map_err(link_error)?;
    for name in order.iter().filter(|name| **name != root_name) {
        let linked =
            link_core_program(&modules[name], &modules, &duplicates).map_err(link_error)?;
        check_core_program(&linked, &options).map_err(|diagnostics| {
            let (file, source) = &files[name];
            core_diagnostics_error(&diagnostics, file, source)
        })?;
    }
    let linked = link_core_program(&program, &modules, &duplicates).map_err(link_error)?;
    check_core_program(&linked, &options)
        .map_err(|diagnostics| core_diagnostics_error(&diagnostics, &file, &source))?;
    Ok(CheckedCoreSource {
        program: linked,
        root: program,
        source,
        available_modules,
        linked_modules: order.len(),
    })
}

fn core_ir_errors(errors: Vec<argorix_ir::CoreIrDiagnostic>) -> anyhow::Error {
    anyhow::anyhow!(
        "{}",
        errors
            .into_iter()
            .map(|error| format!("{}: {}", error.code, error.message))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn diagnostics_error(diagnostics: &[Diagnostic], file: &str, source: &str) -> anyhow::Error {
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.render(file, source))
        .collect::<Vec<_>>()
        .join("\n\n");
    anyhow::anyhow!("{rendered}")
}

fn core_diagnostics_error(
    diagnostics: &[CoreDiagnostic],
    file: &str,
    source: &str,
) -> anyhow::Error {
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.render(file, source))
        .collect::<Vec<_>>()
        .join("\n\n");
    anyhow::anyhow!("{rendered}")
}

#[cfg(test)]
mod tests {
    use super::{compile, load_bytecode_for_verification, Cli, Command};
    use argorix_bytecode::{lower_ir, verify_bytecode};
    use argorix_ir::IrProgram;
    use argorix_parser::parse_source;
    use argorix_semantics::check_program;
    use clap::Parser;
    use std::path::Path;

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
        assert_eq!(parsed["bytecode_version"], "1.0");
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

    #[test]
    fn verify_bytecode_command_accepts_serialized_bytecode() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/provider_harness_v020.argbc.json");
        let bytecode = load_bytecode_for_verification(&fixture, Default::default()).unwrap();
        assert_eq!(bytecode.bytecode_version, "0.20");
        assert_eq!(bytecode.provider_harnesses[0].name, "OpenAIHarness");
        verify_bytecode(&bytecode).unwrap();
    }

    #[test]
    fn core_check_command_is_explicitly_isolated() {
        let cli = Cli::try_parse_from([
            "argorixc",
            "core-check",
            "tests/selfhost/spec/valid/lexer.argx",
        ])
        .unwrap();
        assert!(matches!(cli.command, Command::CoreCheck { .. }));
    }

    #[test]
    fn core_ir_commands_are_explicitly_isolated() {
        let emit = Cli::try_parse_from([
            "argorixc",
            "core-emit-ir",
            "tests/selfhost/spec/valid/lexer.argx",
        ])
        .unwrap();
        assert!(matches!(emit.command, Command::CoreEmitIr { .. }));

        let verify =
            Cli::try_parse_from(["argorixc", "core-verify-ir", "lexer.coreir.json"]).unwrap();
        assert!(matches!(verify.command, Command::CoreVerifyIr { .. }));

        let emit_c = Cli::try_parse_from([
            "argorixc",
            "core-emit-c",
            "tests/selfhost/runtime/scalar_success.argx",
        ])
        .unwrap();
        assert!(matches!(emit_c.command, Command::CoreEmitC { .. }));
    }

    #[test]
    fn historical_compiler_refuses_core_source() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/selfhost/spec/valid/lexer.argx");
        let error = match compile(&fixture, Default::default()) {
            Ok(_) => panic!("historical compiler accepted Core source"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("core-check"));
    }
}
