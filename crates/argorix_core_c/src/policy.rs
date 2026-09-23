//! What a transitional Core C executable may depend on.
//!
//! The rules live in `conformance/core_c/policy.json` so they can be audited
//! without reading code. Pattern matching is deliberately simple: a literal
//! substring, or an anchored prefix/suffix with `^` and `$`. Regular
//! expressions would add a dependency and hide what each rule accepts.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::elf::Inspection;

#[derive(Debug, Clone, Deserialize)]
pub struct Policy {
    pub schema_version: u32,
    pub allowed_needed_libraries: Vec<String>,
    pub forbidden_library_patterns: Vec<String>,
    pub forbidden_symbol_patterns: Vec<String>,
    pub forbidden_imports: Vec<String>,
    /// File-system functions only the compiler-host shim may import: a
    /// program that cannot hold a capability must not link any of them.
    #[serde(default)]
    pub host_only_imports: Vec<String>,
    pub forbidden_byte_markers: Vec<String>,
    pub execution_timeout_seconds: u64,
}

impl Policy {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let policy: Policy = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        anyhow::ensure!(
            policy.schema_version == 1,
            "unsupported policy schema_version"
        );
        Ok(policy)
    }

    /// Every violation found; an empty list means the binary is clean.
    pub fn violations(&self, inspection: &Inspection, content: &[u8]) -> Vec<String> {
        let mut violations = Vec::new();
        let allowed: BTreeSet<&str> = self
            .allowed_needed_libraries
            .iter()
            .map(String::as_str)
            .collect();
        for library in &inspection.needed {
            if !allowed.contains(library.as_str()) {
                violations.push(format!("NEEDED library not allow-listed: {library}"));
            }
            for pattern in &self.forbidden_library_patterns {
                if matches(pattern, library) {
                    violations.push(format!("NEEDED library matches `{pattern}`: {library}"));
                }
            }
        }
        let mut names: Vec<&str> = inspection
            .symbols
            .iter()
            .map(|symbol| symbol.name.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        for name in names {
            for pattern in &self.forbidden_symbol_patterns {
                if matches(pattern, name) {
                    violations.push(format!("symbol matches `{pattern}`: {name}"));
                }
            }
        }
        let forbidden: BTreeSet<&str> = self.forbidden_imports.iter().map(String::as_str).collect();
        for name in inspection.imported() {
            if forbidden.contains(name) {
                violations.push(format!("imports process/loader function: {name}"));
            }
        }
        for marker in &self.forbidden_byte_markers {
            if contains_bytes(content, marker.as_bytes()) {
                violations.push(format!("contains byte marker: {marker}"));
            }
        }
        violations
    }
}

/// `^prefix`, `suffix$`, `^exact$`, or a plain substring.
pub fn matches(pattern: &str, value: &str) -> bool {
    match (pattern.strip_prefix('^'), pattern.strip_suffix('$')) {
        (Some(rest), None) => value.starts_with(rest),
        (None, Some(rest)) => value.ends_with(rest),
        (Some(_), Some(_)) => {
            let inner = &pattern[1..pattern.len() - 1];
            value == inner
        }
        (None, None) => value.contains(pattern),
    }
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elf::Symbol;

    fn policy() -> Policy {
        Policy {
            schema_version: 1,
            allowed_needed_libraries: vec!["libc.so.6".into()],
            forbidden_library_patterns: vec!["rust".into(), "libstd-".into()],
            forbidden_symbol_patterns: vec!["^rust_".into(), "^__rust".into()],
            forbidden_imports: vec!["system".into(), "execve".into()],
            host_only_imports: vec!["open".into()],
            forbidden_byte_markers: vec!["/rustc/".into()],
            execution_timeout_seconds: 10,
        }
    }

    fn inspection(needed: &[&str], symbols: &[(&str, bool)]) -> Inspection {
        Inspection {
            needed: needed.iter().map(|item| item.to_string()).collect(),
            symbols: symbols
                .iter()
                .map(|(name, imported)| Symbol {
                    name: name.to_string(),
                    imported: *imported,
                })
                .collect(),
        }
    }

    #[test]
    fn a_clean_binary_has_no_violations() {
        let clean = inspection(&["libc.so.6"], &[("printf", true), ("argorix_trap", false)]);
        assert!(policy().violations(&clean, b"\x7fELF").is_empty());
    }

    #[test]
    fn trust_named_symbols_are_not_false_positives() {
        let named = inspection(&[], &[("argorix_trust_check", false), ("entrust", false)]);
        assert!(policy().violations(&named, b"").is_empty());
    }

    #[test]
    fn rust_libraries_and_symbols_are_flagged() {
        let dirty = inspection(
            &["librust_sensor.so", "libstd-8e1f.so"],
            &[("__rust_alloc", false)],
        );
        let violations = policy().violations(&dirty, b"");
        assert!(violations
            .iter()
            .any(|item| item.contains("librust_sensor.so")));
        assert!(violations
            .iter()
            .any(|item| item.contains("libstd-8e1f.so")));
        assert!(violations.iter().any(|item| item.contains("__rust_alloc")));
    }

    #[test]
    fn unknown_libraries_are_flagged() {
        let dirty = inspection(&["libm.so.6"], &[]);
        assert_eq!(
            policy().violations(&dirty, b""),
            vec!["NEEDED library not allow-listed: libm.so.6".to_string()]
        );
    }

    #[test]
    fn only_imported_process_functions_are_flagged() {
        let imported = inspection(&[], &[("system", true)]);
        assert!(!policy().violations(&imported, b"").is_empty());
        let defined = inspection(&[], &[("system", false)]);
        assert!(policy().violations(&defined, b"").is_empty());
        let similar = inspection(&[], &[("system_status", true)]);
        assert!(policy().violations(&similar, b"").is_empty());
    }

    #[test]
    fn toolchain_paths_in_the_binary_are_flagged() {
        let violations = policy().violations(&inspection(&[], &[]), b"x\x00/rustc/abc/library");
        assert_eq!(
            violations,
            vec!["contains byte marker: /rustc/".to_string()]
        );
    }

    #[test]
    fn pattern_anchors() {
        assert!(matches("^rust_", "rust_begin_unwind"));
        assert!(!matches("^rust_", "trust_me"));
        assert!(matches("libstd-", "x/libstd-1.so"));
        assert!(matches("^exact$", "exact"));
        assert!(!matches("^exact$", "exactly"));
        assert!(matches(".so$", "libc.so"));
    }
}
