//! ESP-016: the native x86-64 backend.
//!
//! - The encoder (`compiler/x86.argx`) is compared with GNU `as` on every
//!   instruction form it has, over every register and base.
//! - The object writer (`compiler/elf.argx`) writes an ELF64 relocatable
//!   object that GNU `ld` links, with the runtime shim
//!   (`bootstrap/native/`), into a program that runs.
//!
//! These need a Linux x86-64 host with binutils and a C compiler for the
//! harnesses and the shim. Elsewhere they return early.

#![cfg_attr(
    not(all(target_os = "linux", target_arch = "x86_64")),
    allow(dead_code, unused_imports)
)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

/// A fresh directory for one test.
fn work_dir(name: &str) -> PathBuf {
    let work = env::temp_dir().join(format!("argorix-native-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&work);
    fs::create_dir_all(&work).unwrap();
    work
}

/// A harness built by stage0 and the C backend.
fn build_harness(harness: &str, work: &Path) -> PathBuf {
    let c_file = work.join("harness.c");
    let emit = Command::new(env!("CARGO_BIN_EXE_argorixc"))
        .arg("--stdlib")
        .arg(root().join("stdlib"))
        .arg("--modules")
        .arg(root().join("compiler"))
        .arg("core-emit-c")
        .arg(root().join(harness))
        .arg("--output")
        .arg(&c_file)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "{}",
        String::from_utf8_lossy(&emit.stderr)
    );
    let executable = work.join("harness");
    let compile = Command::new("cc")
        .args(["-std=c11", "-O1", "-Wall", "-Wextra", "-Werror"])
        .arg("-DARGORIX_BUFFER_LIMIT_BYTES=268435456U")
        .arg("-I")
        .arg(root().join("bootstrap/c"))
        .arg(root().join("bootstrap/c/argorix_core_runtime.c"))
        .arg(&c_file)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    executable
}

/// Runs a harness that takes a build root, and returns its result.
fn run_with_build(executable: &Path, build: &Path) -> String {
    fs::create_dir_all(build).unwrap();
    let run = Command::new(executable)
        .arg("--build-root")
        .arg(build)
        .args(["--write-budget", "100000000"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&run.stdout).trim().to_string()
}

const R64: [&str; 16] = [
    "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15",
];
const R32: [&str; 16] = [
    "eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi", "r8d", "r9d", "r10d", "r11d", "r12d",
    "r13d", "r14d", "r15d",
];
const R16: [&str; 16] = [
    "ax", "cx", "dx", "bx", "sp", "bp", "si", "di", "r8w", "r9w", "r10w", "r11w", "r12w", "r13w",
    "r14w", "r15w",
];
const R8: [&str; 16] = [
    "al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil", "r8b", "r9b", "r10b", "r11b", "r12b",
    "r13b", "r14b", "r15b",
];

/// The instructions `tests/selfhost/native/encode_all.argx` encodes, in its
/// order, as Intel-syntax assembly: one entry per instruction it marks.
fn expected_assembly() -> Vec<String> {
    let mut lines = Vec::new();
    for a in R64 {
        for b in R64 {
            lines.push(format!("mov {a}, {b}"));
        }
    }
    for register in R64 {
        lines.push(format!("movabs {register}, 0x1122334455667788"));
    }
    let memory = |base: usize, below: bool| {
        format!("[{} {} 0x12345]", R64[base], if below { '-' } else { '+' })
    };
    let pointer = |size: usize| match size {
        1 => "BYTE",
        2 => "WORD",
        4 => "DWORD",
        _ => "QWORD",
    };
    for size in [1, 2, 4, 8] {
        for signed in [false, true] {
            if size == 8 && signed {
                continue;
            }
            for a in 0..16 {
                for b in 0..16 {
                    for below in [true, false] {
                        let operand = format!("{} PTR {}", pointer(size), memory(b, below));
                        lines.push(match (size, signed) {
                            (8, _) => format!("mov {}, {operand}", R64[a]),
                            (4, false) => format!("mov {}, {operand}", R32[a]),
                            (4, true) => format!("movsxd {}, {operand}", R64[a]),
                            (_, false) => format!("movzx {}, {operand}", R32[a]),
                            (_, true) => format!("movsx {}, {operand}", R64[a]),
                        });
                    }
                }
            }
        }
    }
    for size in [1, 2, 4, 8] {
        let registers = match size {
            1 => R8,
            2 => R16,
            4 => R32,
            _ => R64,
        };
        for register in registers {
            for b in 0..16 {
                for below in [true, false] {
                    lines.push(format!(
                        "mov {} PTR {}, {register}",
                        pointer(size),
                        memory(b, below)
                    ));
                }
            }
        }
    }
    for register in R64 {
        for b in 0..16 {
            for below in [true, false] {
                lines.push(format!("lea {register}, {}", memory(b, below)));
            }
        }
    }
    for operation in ["add", "sub", "and", "or", "xor", "cmp", "test", "imul"] {
        for a in R64 {
            for b in R64 {
                lines.push(format!("{operation} {a}, {b}"));
            }
        }
    }
    for operation in ["not", "neg", "div", "idiv", "shl", "shr", "sar"] {
        for register in R64 {
            lines.push(if matches!(operation, "shl" | "shr" | "sar") {
                format!("{operation} {register}, cl")
            } else {
                format!("{operation} {register}")
            });
        }
    }
    for condition in ["o", "b", "ae", "e", "ne", "be", "a", "l", "ge", "le", "g"] {
        for index in 0..16 {
            lines.push(format!(
                "set{condition} {}\nmovzx {}, {}",
                R8[index], R32[index], R8[index]
            ));
        }
    }
    for register in R64 {
        lines.push(format!("push {register}"));
        lines.push(format!("pop {register}"));
        lines.push(format!("lea {register}, [rip + 0]"));
    }
    lines.extend(
        [
            "sub rsp, 0x12345",
            "add rsp, 0x12345",
            "ret",
            "rep movsb",
            "cqo",
            "rep stosb",
            "ud2",
        ]
        .map(String::from),
    );
    for register in R32 {
        lines.push(format!("mov {register}, 0x87654321"));
    }
    lines
}

/// The `.text` bytes of an ELF64 relocatable object.
fn text_section(object: &[u8]) -> Vec<u8> {
    let read = |at: usize, size: usize| -> u64 {
        object[at..at + size]
            .iter()
            .rev()
            .fold(0_u64, |value, byte| (value << 8) | u64::from(*byte))
    };
    let sections = read(0x28, 8) as usize;
    let count = read(0x3c, 2) as usize;
    let names = read(0x3e, 2) as usize;
    let header = |index: usize| sections + index * 64;
    let names_offset = read(header(names) + 24, 8) as usize;
    for index in 0..count {
        let name = read(header(index), 4) as usize + names_offset;
        let end = object[name..].iter().position(|byte| *byte == 0).unwrap();
        if &object[name..name + end] == b".text" {
            let offset = read(header(index) + 24, 8) as usize;
            let size = read(header(index) + 32, 8) as usize;
            return object[offset..offset + size].to_vec();
        }
    }
    panic!("no .text section");
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn the_encoder_matches_gnu_as_when_available() {
    if !available("cc") || !available("as") {
        eprintln!("cc or as unavailable; the encoder is compared in CI");
        return;
    }
    let work = work_dir("encoder");
    let harness = build_harness("tests/selfhost/native/encode_all.argx", &work);
    let build = work.join("build");
    assert_eq!(run_with_build(&harness, &build), "ARGORIX_RESULT:0");
    let encoded = fs::read(build.join("encoded.bin")).unwrap();
    let ends: Vec<usize> = fs::read_to_string(build.join("ends.txt"))
        .unwrap()
        .lines()
        .map(|line| line.parse().unwrap())
        .collect();
    let lines = expected_assembly();
    assert_eq!(
        lines.len(),
        ends.len(),
        "the harness and the test disagree on the list"
    );
    let source = work.join("expected.s");
    fs::write(
        &source,
        format!(".intel_syntax noprefix\n.text\n{}\n", lines.join("\n")),
    )
    .unwrap();
    let object = work.join("expected.o");
    let assemble = Command::new("as")
        .arg("--64")
        .arg("-o")
        .arg(&object)
        .arg(&source)
        .output()
        .unwrap();
    assert!(
        assemble.status.success(),
        "{}",
        String::from_utf8_lossy(&assemble.stderr)
    );
    let expected = text_section(&fs::read(&object).unwrap());
    let mut start = 0;
    for (line, end) in lines.iter().zip(&ends) {
        assert!(
            expected.get(start..*end) == encoded.get(start..*end),
            "`{line}`: as gives {:02x?}, the encoder {:02x?}",
            expected.get(start..(*end).min(expected.len())),
            &encoded[start..*end]
        );
        start = *end;
    }
    assert_eq!(expected.len(), encoded.len());
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}

/// The C runtime and the native shim, built once with the C compiler: the
/// declared dependency the native backend links against.
fn build_shim(work: &Path) -> Vec<PathBuf> {
    let mut objects = Vec::new();
    for source in [
        "bootstrap/c/argorix_core_runtime.c",
        "bootstrap/native/argorix_native_shim.c",
        "bootstrap/native/argorix_native_host.c",
    ] {
        let object = work.join(
            Path::new(source)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string()
                + ".o",
        );
        let compile = Command::new("cc")
            .args(["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-c"])
            .arg("-I")
            .arg(root().join("bootstrap/c"))
            .arg(root().join(source))
            .arg("-o")
            .arg(&object)
            .output()
            .unwrap();
        assert!(
            compile.status.success(),
            "{source}: {}",
            String::from_utf8_lossy(&compile.stderr)
        );
        objects.push(object);
    }
    objects
}

/// Links a native object with GNU ld: the C start files, the object, the
/// shim objects and the C library.
fn link(object: &Path, shim: &[PathBuf], executable: &Path) {
    let lib = Path::new("/usr/lib/x86_64-linux-gnu");
    let output = Command::new("ld")
        .arg("-o")
        .arg(executable)
        .args(["-dynamic-linker", "/lib64/ld-linux-x86-64.so.2"])
        .arg(lib.join("crt1.o"))
        .arg(lib.join("crti.o"))
        .arg(object)
        .args(shim)
        .arg("-L")
        .arg(lib)
        .arg("-lc")
        .arg(lib.join("crtn.o"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ld: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn a_native_object_links_and_runs_when_binutils_are_available() {
    if !available("cc")
        || !available("ld")
        || !Path::new("/usr/lib/x86_64-linux-gnu/crt1.o").exists()
    {
        eprintln!("cc, ld or the C start files unavailable; the native link runs in CI");
        return;
    }
    let work = work_dir("hello");
    let harness = build_harness("tests/selfhost/native/hello_object.argx", &work);
    let build = work.join("build");
    assert_eq!(run_with_build(&harness, &build), "ARGORIX_RESULT:0");
    let shim = build_shim(&work);
    let executable = work.join("hello");
    link(&build.join("hello.o"), &shim[..2], &executable);
    let run = Command::new(&executable).output().unwrap();
    assert!(run.status.success());
    assert_eq!(String::from_utf8_lossy(&run.stdout), "ARGORIX_RESULT:42\n");
    if env::var_os("ARGORIX_KEEP_WORK").is_none() {
        fs::remove_dir_all(work).unwrap();
    }
}
