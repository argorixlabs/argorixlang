pub mod manifest;
pub mod merge;
pub mod resolver;

pub use manifest::{parse_manifest, Manifest};
pub use merge::{check_package, merge_package, package_ir};
pub use resolver::{
    resolve_package, ModuleGraph, ModuleImportEdge, ResolvedModule, ResolvedPackage,
};

use thiserror::Error;

/// Errors produced while resolving a local Argorix package.
///
/// Every message is portable: it never embeds absolute paths and never depends on
/// the current working directory.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ModuleError {
    #[error("failed to read manifest: {0}")]
    ManifestRead(String),
    #[error("invalid manifest: {0}")]
    ManifestParse(String),
    #[error("entry file `{0}` does not exist")]
    EntryNotFound(String),
    #[error("failed to read module `{path}`: {reason}")]
    ReadModule { path: String, reason: String },
    #[error("failed to parse module `{path}`: {}", messages.join("; "))]
    ParseModule { path: String, messages: Vec<String> },
    #[error("module `{from}` imports unknown module `{import}`")]
    UnknownImport { from: String, import: String },
    #[error("cyclic import detected: {}", .0.join(" -> "))]
    CyclicImport(Vec<String>),
    #[error("duplicate module `{0}`")]
    DuplicateModule(String),
    #[error("module at `{path}` declares `{declared}` but its path requires `{expected}`")]
    ModulePathMismatch {
        path: String,
        declared: String,
        expected: String,
    },
    #[error("module file `{0}` is outside the project root")]
    ImportOutsideRoot(String),
    #[error("invalid module name `{0}`")]
    InvalidModuleName(String),
}
