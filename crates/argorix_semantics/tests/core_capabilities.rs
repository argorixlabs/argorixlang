//! Compiler-host capabilities in Argorix Core (ESP-009, `stdlib.compiler_host`).
//!
//! `PackageRead` and `BuildWrite` are lent by the driver to `argorix_main`.
//! Source can pass them on as parameters and hold them in locals, but cannot
//! make one, store one or return one.

use argorix_parser::core::parse_core_source;
use argorix_semantics::{check_core_program, CoreCheckOptions};

fn codes(items: &str) -> Vec<String> {
    let source = format!("core 0.1;\nmodule host.case;\n\n{items}\n");
    let program = parse_core_source(&source).expect("case parses");
    match check_core_program(&program, &CoreCheckOptions::default()) {
        Ok(()) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|item| item.code).collect(),
    }
}

#[test]
fn capabilities_pass_down_as_parameters_and_locals() {
    let items = "fn size(package: PackageRead, file: Slice<u8>) -> u64 {
    let lent: PackageRead = package;
    let status: u64 = lent.status(file);
    let bytes: Buffer<u8> = lent.read(file);
    status + bytes.length()
}

pub fn argorix_main(package: PackageRead, build: BuildWrite) -> u64 {
    let name: Array<u8, 1> = [97u8];
    let written: u64 = build.write(name.as_slice(), name.as_slice());
    size(package, name.as_slice()) + written
}";
    assert_eq!(codes(items), Vec::<String>::new());
}

#[test]
fn a_capability_cannot_be_a_field() {
    let items = "struct Keeper { package: PackageRead, }
pub fn argorix_main() -> u64 { 0u64 }";
    assert_eq!(codes(items), vec!["CapabilityEscapes"]);
}

#[test]
fn a_capability_cannot_be_returned() {
    let items = "fn keep(package: PackageRead) -> PackageRead { package }
pub fn argorix_main() -> u64 { 0u64 }";
    assert_eq!(codes(items), vec!["CapabilityEscapes"]);
}

#[test]
fn a_capability_cannot_be_stored_in_a_container() {
    let items = "fn keep(package: PackageRead) -> u64 {
    let mut kept: Buffer<PackageRead> = Buffer::new();
    kept.push(package);
    kept.length()
}
pub fn argorix_main() -> u64 { 0u64 }";
    assert!(codes(items).contains(&"CapabilityEscapes".to_string()));
}

#[test]
fn the_capability_types_cannot_be_redeclared() {
    let items = "struct PackageRead { root: u64, }
pub fn argorix_main() -> u64 { 0u64 }";
    assert!(codes(items).contains(&"DuplicateDeclaration".to_string()));
}

#[test]
fn each_capability_has_only_its_own_operations() {
    let items = "pub fn argorix_main(package: PackageRead) -> u64 {
    let name: Array<u8, 1> = [97u8];
    package.write(name.as_slice(), name.as_slice())
}";
    assert!(codes(items).contains(&"TypeMismatch".to_string()));
}
