#!/usr/bin/env python3
"""ESP-014: build stage1, the first self-hosted Argorix compiler, and record
where it came from.

Two steps, which may run on different hosts:

    python3 bootstrap/stage1.py emit  --argorixc target/debug/argorixc --out target/stage1
    python3 bootstrap/stage1.py build --cc gcc --out target/stage1 [--require-rust-free-host]

`emit` is stage B0 (bootstrap/architecture.md): the Rust stage0 writes the C
of `compiler/main.argx` and the rest of `compiler/` and `stdlib/` to
`stage1.c`, and records its own identity in `stage0.json`.

`build` is stage B1 and needs only a C compiler: it compiles `stage1.c` with
the C1 runtime into `stage1`, runs stage1 on its own sources as
`argorix.build` lists them, and checks that:

- stage1 writes, byte for byte, the C stage0 wrote for it;
- its manifest names each source with the right size and SHA-256, and the
  output;
- that C, compiled, is a working compiler: it does the same build again and
  writes the same three files;
- neither executable depends on a Rust library, and, with
  `--require-rust-free-host`, no Rust tool exists on the host at all.

It writes `stage1-provenance.json`: the source revision, stage0 and the Rust
toolchain that built it, the C compiler and flags, the runtime, the sources,
and the digest of every artifact. Rust stays in the bootstrap's trusted base
until ESP-017 (spec/provenance.md, P1-02), so the record says so.

Only the Python standard library is used.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import platform
import re
import shutil
import subprocess
import sys
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
BUILD_FILE = ROOT / "argorix.build"
RUNTIME = [
    "bootstrap/c/argorix_core_runtime.c",
    "bootstrap/c/argorix_core_runtime.h",
    "bootstrap/c/argorix_core_host.h",
]
# The C profile (spec/core/c-backend.md), and the ceilings the compiler's own
# trees need: linking the whole compiler takes more steps than the default
# budget, and its merged trees pass the default 1 MiB per buffer.
FLAGS = [
    "-std=c11",
    "-O2",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-DARGORIX_STEP_LIMIT=400000000000ULL",
    "-DARGORIX_BUFFER_LIMIT_BYTES=268435456U",
]
WRITE_BUDGET = 100_000_000
OUTPUTS = ["compiler.c", "compiler.json", "diagnostics.txt"]
RUST_TOOLS = ["rustc", "cargo", "rustup"]


class BootstrapError(Exception):
    pass


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def relative(path: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def artifact(path: pathlib.Path) -> dict[str, Any]:
    return {"path": relative(path), "bytes": path.stat().st_size, "sha256": sha256(path)}


def first_line(command: list[str]) -> str | None:
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=False)
    except OSError:
        return None
    text = (result.stdout or result.stderr).strip()
    return text.splitlines()[0] if text else None


def run(command: list[str], what: str) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, check=False)
    if result.returncode != 0:
        raise BootstrapError(f"{what} failed ({result.returncode}): {result.stderr.strip()}")
    return result


def source_revision() -> dict[str, Any]:
    commit = first_line(["git", "-C", str(ROOT), "rev-parse", "HEAD"])
    status = subprocess.run(
        ["git", "-C", str(ROOT), "status", "--porcelain", "--untracked-files=no"],
        capture_output=True,
        text=True,
        check=False,
    )
    return {
        "commit": commit,
        "dirty": bool(status.stdout.strip()) if status.returncode == 0 else None,
    }


def build_files() -> list[str]:
    """The sources `argorix.build` lists, the root first."""
    lines = BUILD_FILE.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != "argorix-build 1":
        raise BootstrapError("argorix.build does not start with `argorix-build 1`")
    files = []
    for line in lines[1:]:
        if not line or line.startswith("#"):
            continue
        key, value = line.split(" ", 1)
        if key == "root":
            files.insert(0, value)
        elif key == "module":
            files.append(value)
    return files


def emit(argorixc: pathlib.Path, out: pathlib.Path) -> None:
    out.mkdir(parents=True, exist_ok=True)
    stage1_c = out / "stage1.c"
    command = [
        str(argorixc.resolve()),
        "--stdlib",
        "stdlib",
        "core-emit-c",
        "compiler/main.argx",
        "--output",
        str(stage1_c.resolve()),
    ]
    run(command, "stage0")
    record = {
        "schema_version": 1,
        "task": "ESP-014",
        "stage": "B0",
        "source": source_revision(),
        "stage0": {
            "executable": artifact(argorixc),
            "version": first_line([str(argorixc), "--version"]),
            "rustc": first_line(["rustc", "--version"]),
            "cargo": first_line(["cargo", "--version"]),
            "command": ["argorixc", *command[1:5], "--output", "stage1.c"],
        },
        "stage1_c": artifact(stage1_c),
    }
    (out / "stage0.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(f"stage0 wrote {relative(stage1_c)} ({record['stage1_c']['sha256']})")


def compile_c(cc: str, source: pathlib.Path, executable: pathlib.Path) -> list[str]:
    command = [
        cc,
        *FLAGS,
        "-I",
        "bootstrap/c",
        RUNTIME[0],
        relative(source),
        "-o",
        relative(executable),
    ]
    run(command, f"{cc} on {relative(source)}")
    return command


def dynamic_dependencies(executable: pathlib.Path) -> list[str]:
    """The shared libraries the executable loads, as the platform tool lists
    them. A Rust library among them is a failure."""
    for tool in (["ldd"], ["otool", "-L"]):
        if shutil.which(tool[0]) is None:
            continue
        result = subprocess.run([*tool, str(executable)], capture_output=True, text=True, check=False)
        # Without load addresses, which change from run to run.
        lines = [
            re.sub(r"\s*\(0x[0-9a-f]+\)$", "", line.strip())
            for line in result.stdout.splitlines()
            if line.strip()
        ]
        if tool[0] == "otool":
            lines = lines[1:]
        for line in lines:
            lowered = line.lower()
            if "rust" in lowered or "libstd-" in lowered:
                raise BootstrapError(f"{relative(executable)} loads a Rust library: {line}")
        return lines
    return []


def compile_run(executable: pathlib.Path, build: pathlib.Path, budget: int) -> tuple[list[str], str]:
    if build.exists():
        shutil.rmtree(build)
    build.mkdir(parents=True)
    command = [
        relative(executable),
        "--package-root",
        ".",
        "--read-budget",
        str(budget),
        "--build-root",
        relative(build),
        "--write-budget",
        str(WRITE_BUDGET),
    ]
    result = run([str(executable.resolve()), *command[1:]], relative(executable))
    stdout = result.stdout.strip()
    if not stdout.startswith("ARGORIX_RESULT:"):
        raise BootstrapError(f"{relative(executable)} printed no result: {stdout!r}")
    return command, stdout.removeprefix("ARGORIX_RESULT:")


def build(cc: str, out: pathlib.Path, require_rust_free_host: bool) -> None:
    record = out / "stage0.json"
    if not record.is_file():
        raise BootstrapError(f"{relative(record)} is missing: run the `emit` step first")
    stage0 = json.loads(record.read_text(encoding="utf-8"))
    stage1_c = out / "stage1.c"
    if sha256(stage1_c) != stage0["stage1_c"]["sha256"]:
        raise BootstrapError("stage1.c is not the file stage0 recorded")
    rust_tools = {tool: shutil.which(tool) for tool in RUST_TOOLS}
    if require_rust_free_host and any(rust_tools.values()):
        found = ", ".join(f"{tool} at {path}" for tool, path in rust_tools.items() if path)
        raise BootstrapError(f"this host is meant to have no Rust tools, but has {found}")
    cc_path = shutil.which(cc)
    if cc_path is None:
        raise BootstrapError(f"no C compiler named {cc}")

    stage1 = out / "stage1"
    stage1_command = compile_c(cc, stage1_c, stage1)
    files = build_files()
    budget = sum((ROOT / file).stat().st_size for file in files) + BUILD_FILE.stat().st_size

    first = out / "self"
    run_command, result = compile_run(stage1, first, budget)
    if result != "0":
        diagnostics = (first / "diagnostics.txt").read_text(encoding="utf-8", errors="replace")
        raise BootstrapError(f"stage1 returned {result} on its own sources:\n{diagnostics}")
    if (first / "compiler.c").read_bytes() != stage1_c.read_bytes():
        raise BootstrapError("stage1's C for its own sources differs from stage0's")
    if (first / "diagnostics.txt").stat().st_size != 0:
        raise BootstrapError("stage1 wrote diagnostics for a build that succeeded")

    # The manifest, checked against the files themselves.
    manifest = json.loads((first / "compiler.json").read_text(encoding="utf-8"))
    if manifest.get("status") != "emitted" or manifest.get("root") != "compiler.main":
        raise BootstrapError("the manifest does not describe an emitted compiler.main")
    listed = manifest.get("sources", [])
    if [source.get("path") for source in listed] != files:
        raise BootstrapError("the manifest's sources are not the build file's")
    for source in listed:
        data = (ROOT / source["path"]).read_bytes()
        if source.get("bytes") != len(data) or source.get("sha256") != hashlib.sha256(data).hexdigest():
            raise BootstrapError(f"the manifest's digest of {source['path']} is wrong")
    if manifest["output"]["sha256"] != sha256(first / "compiler.c"):
        raise BootstrapError("the manifest's digest of the output is wrong")

    # The C stage1 wrote is a compiler: built, it does the same build again.
    rebuilt = out / "stage1-rebuilt"
    rebuilt_command = compile_c(cc, first / "compiler.c", rebuilt)
    second = out / "rebuilt"
    _, result = compile_run(rebuilt, second, budget)
    if result != "0":
        raise BootstrapError(f"the rebuilt compiler returned {result}")
    for name in OUTPUTS:
        if (first / name).read_bytes() != (second / name).read_bytes():
            raise BootstrapError(f"the rebuilt compiler writes a different {name}")

    provenance = {
        "schema_version": 1,
        "task": "ESP-014",
        "stage": "B1",
        "bootstrap_dependency": "rust",
        "note": (
            "stage0, written in Rust, produced stage1.c; everything after that ran without "
            "Rust. The transitional C backend and a C compiler remain in the path (ESP-016)."
        ),
        "source": stage0["source"],
        "stage0": stage0["stage0"],
        "host": {
            "system": platform.system(),
            "machine": platform.machine(),
            "rust_tools": rust_tools,
            "rust_free_required": require_rust_free_host,
        },
        "c_compiler": {
            "name": cc,
            "path": cc_path,
            "version": first_line([cc, "--version"]),
            "flags": FLAGS,
            "runtime": [artifact(ROOT / path) for path in RUNTIME],
        },
        "build_file": artifact(BUILD_FILE),
        "sources": listed,
        "stage1": {
            "c": artifact(stage1_c),
            "command": stage1_command,
            "executable": artifact(stage1),
            "dynamic_dependencies": dynamic_dependencies(stage1),
        },
        "self_build": {
            "command": run_command,
            "result": 0,
            "outputs": {name: artifact(first / name) for name in OUTPUTS},
            "modules": manifest["modules"],
            "c_equals_stage0_c": True,
        },
        "rebuilt": {
            "command": rebuilt_command,
            "executable": artifact(rebuilt),
            "dynamic_dependencies": dynamic_dependencies(rebuilt),
            "outputs_equal_self_build": True,
        },
    }
    path = out / "stage1-provenance.json"
    path.write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")
    print(
        f"stage1 built itself: {len(files)} sources, C {provenance['stage1']['c']['sha256']}, "
        f"identical to stage0's and to the rebuilt compiler's. Provenance: {relative(path)}"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("step", choices=["emit", "build", "all"])
    parser.add_argument("--argorixc", type=pathlib.Path, help="stage0 (for emit)")
    parser.add_argument("--cc", default="cc", help="the C compiler (for build)")
    parser.add_argument("--out", type=pathlib.Path, default=ROOT / "target" / "stage1")
    parser.add_argument("--require-rust-free-host", action="store_true")
    arguments = parser.parse_args()
    out = arguments.out if arguments.out.is_absolute() else pathlib.Path.cwd() / arguments.out
    try:
        if arguments.step in ("emit", "all"):
            if arguments.argorixc is None:
                parser.error("emit needs --argorixc")
            emit(arguments.argorixc, out)
        if arguments.step in ("build", "all"):
            build(arguments.cc, out, arguments.require_rust_free_host)
    except BootstrapError as error:
        print(f"stage1 bootstrap failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
