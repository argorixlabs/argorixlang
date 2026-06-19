use crate::{manifest::parse_manifest, ModuleError};
use argorix_parser::{ast::Program, is_valid_module_name, parse_source};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

/// A deterministic graph of the modules reachable from a package entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleGraph {
    pub entry: String,
    pub modules: Vec<ResolvedModule>,
    pub imports: Vec<ModuleImportEdge>,
}

/// A single resolved module: its dotted name, its package-relative path, and source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModule {
    pub name: String,
    pub path: String,
    pub source: String,
}

/// A directed import edge from one module to another.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleImportEdge {
    pub from: String,
    pub to: String,
}

/// A fully resolved package: the module graph plus the parsed program of each module.
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub graph: ModuleGraph,
    pub programs: BTreeMap<String, Program>,
}

/// Resolve a package starting from its `argorix.toml` manifest.
///
/// Resolution is import-driven, deterministic, and independent of the current
/// working directory. Diagnostics never contain absolute paths.
pub fn resolve_package(manifest_path: &Path) -> Result<ResolvedPackage, ModuleError> {
    let source = fs::read_to_string(manifest_path)
        .map_err(|error| ModuleError::ManifestRead(error.to_string()))?;
    let manifest = parse_manifest(&source)?;
    let root = manifest_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let entry_components = normalize_relative(&manifest.entry_main)
        .ok_or_else(|| ModuleError::ImportOutsideRoot(manifest.entry_main.clone()))?;
    let entry_rel = entry_components.join("/");
    let entry_file = root.join(&entry_rel);
    if !entry_file.is_file() {
        return Err(ModuleError::EntryNotFound(entry_rel));
    }
    let entry_source =
        fs::read_to_string(&entry_file).map_err(|error| ModuleError::ReadModule {
            path: entry_rel.clone(),
            reason: error.to_string(),
        })?;
    let entry_program = parse_module(&entry_rel, &entry_source)?;
    let entry_name = entry_program.module.value.clone();
    let derived = derive_entry_name(&entry_rel);
    if entry_name != derived && entry_name != format!("app.{derived}") {
        return Err(ModuleError::ModulePathMismatch {
            path: entry_rel,
            declared: entry_name,
            expected: derived,
        });
    }

    let mut resolver = Resolver {
        root: &root,
        paths: BTreeMap::new(),
        sources: BTreeMap::new(),
        programs: BTreeMap::new(),
        edges: Vec::new(),
        stack: Vec::new(),
    };
    resolver.visit(entry_name.clone(), entry_rel, entry_program, entry_source)?;

    let modules = resolver
        .paths
        .iter()
        .map(|(name, path)| ResolvedModule {
            name: name.clone(),
            path: path.clone(),
            source: resolver.sources.get(name).cloned().unwrap_or_default(),
        })
        .collect();
    let mut imports = resolver.edges;
    imports.sort();
    imports.dedup();

    Ok(ResolvedPackage {
        graph: ModuleGraph {
            entry: entry_name,
            modules,
            imports,
        },
        programs: resolver.programs,
    })
}

struct Resolver<'a> {
    root: &'a Path,
    paths: BTreeMap<String, String>,
    sources: BTreeMap<String, String>,
    programs: BTreeMap<String, Program>,
    edges: Vec<ModuleImportEdge>,
    stack: Vec<String>,
}

impl Resolver<'_> {
    fn visit(
        &mut self,
        name: String,
        path: String,
        program: Program,
        source: String,
    ) -> Result<(), ModuleError> {
        self.paths.insert(name.clone(), path);
        self.sources.insert(name.clone(), source);
        let imports = program.imports.clone();
        self.programs.insert(name.clone(), program);
        self.stack.push(name.clone());

        for import in &imports {
            let to = import.path.value.clone();
            if !is_valid_module_name(&to) {
                return Err(ModuleError::InvalidModuleName(to));
            }
            let expected = module_to_relpath(&to);
            self.edges.push(ModuleImportEdge {
                from: name.clone(),
                to: to.clone(),
            });

            if let Some(existing) = self.paths.get(&to) {
                if existing != &expected {
                    return Err(ModuleError::DuplicateModule(to));
                }
                if self.stack.contains(&to) {
                    return Err(ModuleError::CyclicImport(cycle_from(&self.stack, &to)));
                }
                continue;
            }

            if normalize_relative(&expected).is_none() {
                return Err(ModuleError::ImportOutsideRoot(expected));
            }
            let child_file = self.root.join(&expected);
            if !child_file.is_file() {
                return Err(ModuleError::UnknownImport {
                    from: name.clone(),
                    import: to.clone(),
                });
            }
            let child_source =
                fs::read_to_string(&child_file).map_err(|error| ModuleError::ReadModule {
                    path: expected.clone(),
                    reason: error.to_string(),
                })?;
            let child_program = parse_module(&expected, &child_source)?;
            if child_program.module.value != to {
                return Err(ModuleError::ModulePathMismatch {
                    path: expected,
                    declared: child_program.module.value.clone(),
                    expected: to,
                });
            }
            self.visit(to, expected, child_program, child_source)?;
        }

        self.stack.pop();
        Ok(())
    }
}

fn parse_module(path: &str, source: &str) -> Result<Program, ModuleError> {
    parse_source(source).map_err(|diagnostics| ModuleError::ParseModule {
        path: path.to_owned(),
        messages: diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect(),
    })
}

fn module_to_relpath(name: &str) -> String {
    format!("src/{}.argx", name.replace('.', "/"))
}

fn derive_entry_name(entry_rel: &str) -> String {
    let trimmed = entry_rel.strip_prefix("src/").unwrap_or(entry_rel);
    let trimmed = trimmed.strip_suffix(".argx").unwrap_or(trimmed);
    trimmed.replace('/', ".")
}

fn cycle_from(stack: &[String], target: &str) -> Vec<String> {
    let start = stack.iter().position(|name| name == target).unwrap_or(0);
    let mut cycle = stack[start..].to_vec();
    cycle.push(target.to_owned());
    cycle
}

/// Normalize a relative path into components, returning `None` if it is absolute
/// or escapes the package root via `..`.
fn normalize_relative(path: &str) -> Option<Vec<String>> {
    if path.starts_with('/') || path.starts_with('\\') {
        return None;
    }
    let mut components = Vec::new();
    for raw in path.split(['/', '\\']) {
        match raw {
            "" | "." => {}
            ".." => return None,
            // Reject Windows drive prefixes such as `C:`.
            segment if segment.ends_with(':') => return None,
            segment => components.push(segment.to_owned()),
        }
    }
    if components.is_empty() {
        None
    } else {
        Some(components)
    }
}

#[cfg(test)]
mod tests {
    use super::{derive_entry_name, module_to_relpath, normalize_relative};

    #[test]
    fn maps_module_names_to_paths() {
        assert_eq!(module_to_relpath("main"), "src/main.argx");
        assert_eq!(
            module_to_relpath("agents.research"),
            "src/agents/research.argx"
        );
    }

    #[test]
    fn derives_entry_names_from_paths() {
        assert_eq!(derive_entry_name("src/main.argx"), "main");
        assert_eq!(
            derive_entry_name("src/agents/research.argx"),
            "agents.research"
        );
    }

    #[test]
    fn rejects_escaping_paths() {
        assert!(normalize_relative("../escape.argx").is_none());
        assert!(normalize_relative("/abs/main.argx").is_none());
        assert!(normalize_relative("src/main.argx").is_some());
    }
}
