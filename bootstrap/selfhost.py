#!/usr/bin/env python3
"""ESP-015: stage1 builds stage2, stage2 builds stage3, without Rust.

    python3 bootstrap/selfhost.py --from target/stage1 --cc gcc --out target/selfhost \\
        [--require-rust-free-host] [--require-no-network]

`--from` is the output of `bootstrap/stage1.py emit`: `stage1.c`, which the
Rust stage0 wrote, and `stage0.json`. Everything after that runs with a C
compiler and this script only.

1. **Generations.** Each stage's C is compiled as `compiler.c` in a directory
   of its own, all at the same depth. The flags, the runtime path and the
   environment are the same for every stage, so equal C gives equal
   executables. Each stage then compiles the sources `argorix.build` lists,
   and its C is the next stage's. Stage1 and stage2 must write the same C,
   manifest and diagnostics as stage3, and all three executables must be
   byte-identical.
2. **Paths and order.** Stage3 builds the same sources copied under another
   directory, and again with the `module` lines reversed. The C must not
   change. The manifest may differ only in the order of its sources.
3. **The suite.** Every case of the C fixture suites
   (`tests/selfhost/*/cases.json` and the backend regression corpus) is
   compiled by stage2 and by stage3. Their C must be the same. It is built
   with the C profile and run, and its exit status, output and files are
   checked against the case.
4. **No prebuilt seed.** Two edits to the compiler's sources, a diagnostic's
   wording and the header the C backend writes, are compiled by stage3. The
   new compiler shows the edit, and a second generation reaches a fixed point
   with it.

Nothing here reads a clock. The compiler has no host operation for time, and
the C runtime uses no `__DATE__` or `__TIME__`. Tools run with a fixed
environment: `LC_ALL=C`, `TZ=UTC` and no ccache. `--require-rust-free-host`
fails if `rustc`, `cargo` or `rustup` exists. `--require-no-network` fails if
the host has a network interface besides loopback; CI runs this step in a
container started with `--network none`.

It writes `selfhost-report.json`. Equal stages show that the compiler
reproduces itself; they do not show that stage0, which produced `stage1.c`,
is free of a planted defect that reproduces itself too. That limit is
recorded in the report. ESP-023 and MAT-023 are where it is addressed, with
diverse double compilation.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import platform
import shutil
import subprocess
import sys
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parents[1]
BUILD_FILE = ROOT / "argorix.build"
RUNTIME_C = ROOT / "bootstrap" / "c" / "argorix_core_runtime.c"
RUNTIME_DIR = ROOT / "bootstrap" / "c"
# The compiler's own trees need more steps and larger buffers than the
# defaults; see bootstrap/stage1.py.
COMPILER_FLAGS = [
    "-std=c11",
    "-O2",
    "-Wall",
    "-Wextra",
    "-Werror",
    "-DARGORIX_STEP_LIMIT=400000000000ULL",
    "-DARGORIX_BUFFER_LIMIT_BYTES=268435456U",
]
OUTPUTS = ["compiler.c", "compiler.json", "diagnostics.txt"]
RUST_TOOLS = ["rustc", "cargo", "rustup"]
SUITES = [
    "tests/selfhost/runtime/cases.json",
    "tests/selfhost/stdlib/cases.json",
    "tests/selfhost/lexer/cases.json",
    "tests/selfhost/parser/cases.json",
    "tests/selfhost/check/cases.json",
    "tests/selfhost/ir/cases.json",
    "tests/selfhost/c/cases.json",
    "tests/selfhost/pipeline/cases.json",
    "tests/selfhost/stage1/cases.json",
    "conformance/core_c/regression/cases.json",
]
TIMEOUT_SECONDS = 10
BIG_BUDGET = 100_000_000


class BootstrapError(Exception):
    pass


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())


def relative(path: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


def artifact(path: pathlib.Path) -> dict[str, Any]:
    return {"path": relative(path), "bytes": path.stat().st_size, "sha256": sha256(path)}


def environment() -> dict[str, str]:
    """The fixed environment every tool runs in."""
    return {"PATH": os.environ.get("PATH", "/usr/bin:/bin"), "LC_ALL": "C", "TZ": "UTC", "CCACHE_DISABLE": "1"}


def run(command: list[str], what: str, cwd: pathlib.Path = ROOT, timeout: int | None = None) -> subprocess.CompletedProcess[bytes]:
    try:
        result = subprocess.run(command, cwd=cwd, capture_output=True, env=environment(), timeout=timeout, check=False)
    except subprocess.TimeoutExpired as error:
        raise BootstrapError(f"{what} timed out") from error
    return result


def first_line(command: list[str]) -> str | None:
    try:
        result = subprocess.run(command, capture_output=True, text=True, env=environment(), check=False)
    except OSError:
        return None
    text = (result.stdout or result.stderr).strip()
    return text.splitlines()[0] if text else None


# ------------------------------------------------------------------ build files


def build_entries(text: str) -> tuple[str, list[str], dict[str, str]]:
    """The root, the modules and the outputs of a build file."""
    lines = text.splitlines()
    if not lines or lines[0] != "argorix-build 1":
        raise BootstrapError("a build file must start with `argorix-build 1`")
    root = ""
    modules: list[str] = []
    outputs: dict[str, str] = {}
    for line in lines[1:]:
        if not line or line.startswith("#"):
            continue
        key, value = line.split(" ", 1)
        if key == "root":
            root = value
        elif key == "module":
            modules.append(value)
        else:
            outputs[key] = value
    return root, modules, outputs


def write_build_file(path: pathlib.Path, root: str, modules: list[str], outputs: dict[str, str]) -> None:
    lines = ["argorix-build 1", f"root {root}"]
    lines += [f"module {module}" for module in modules]
    lines += [f"{key} {outputs[key]}" for key in ("c", "manifest", "diagnostics")]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def copy_package(files: list[str], source: pathlib.Path, target: pathlib.Path) -> None:
    for file in files:
        destination = target / file
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source / file, destination)


# ------------------------------------------------------------------ compiling and running


def compile_compiler(cc: str, c_file: pathlib.Path, directory: pathlib.Path) -> tuple[pathlib.Path, list[str]]:
    """`directory/argorixc` from `c_file`, compiled as `directory/compiler.c`.
    Every stage directory sits at the same depth, so the paths the compiler
    records are the same strings for every stage."""
    directory.mkdir(parents=True, exist_ok=True)
    if c_file.resolve() != (directory / "compiler.c").resolve():
        shutil.copyfile(c_file, directory / "compiler.c")
    runtime_dir = os.path.relpath(RUNTIME_DIR, directory)
    runtime_c = os.path.relpath(RUNTIME_C, directory)
    command = [cc, *COMPILER_FLAGS, "-I", runtime_dir, runtime_c, "compiler.c", "-o", "argorixc"]
    result = run(command, f"{cc} on {relative(c_file)}", cwd=directory)
    if result.returncode != 0:
        raise BootstrapError(f"{cc} failed on {relative(c_file)}: {result.stderr.decode(errors='replace')}")
    return directory / "argorixc", command


def run_compiler(compiler: pathlib.Path, package: pathlib.Path, build: pathlib.Path, directories: tuple[str, ...] = ()) -> str:
    """Runs a compiler over `package` into a fresh `build`, with
    `directories` made in it, and returns its result."""
    if build.exists():
        shutil.rmtree(build)
    build.mkdir(parents=True)
    for directory in directories:
        (build / directory).mkdir(parents=True)
    command = [
        str(compiler.resolve()),
        "--package-root",
        str(package.resolve()),
        "--read-budget",
        str(BIG_BUDGET),
        "--build-root",
        str(build.resolve()),
        "--write-budget",
        str(BIG_BUDGET),
    ]
    result = run(command, relative(compiler), timeout=600)
    stdout = result.stdout.decode(errors="replace").strip()
    if result.returncode != 0 or not stdout.startswith("ARGORIX_RESULT:"):
        raise BootstrapError(
            f"{relative(compiler)} failed ({result.returncode}): {stdout} {result.stderr.decode(errors='replace')}"
        )
    return stdout.removeprefix("ARGORIX_RESULT:")


def build_self(compiler: pathlib.Path, package: pathlib.Path, build: pathlib.Path) -> dict[str, bytes]:
    result = run_compiler(compiler, package, build)
    if result != "0":
        diagnostics = (build / "diagnostics.txt").read_text(encoding="utf-8", errors="replace")
        raise BootstrapError(f"{relative(compiler)} returned {result}:\n{diagnostics}")
    return {name: (build / name).read_bytes() for name in OUTPUTS}


# ------------------------------------------------------------------ steps


def generations(cc: str, stage1_c: pathlib.Path, out: pathlib.Path) -> dict[str, Any]:
    stages = []
    c_file = stage1_c
    outputs_by_stage: list[dict[str, bytes]] = []
    for number in (1, 2, 3):
        directory = out / f"stage{number}"
        compiler, command = compile_compiler(cc, c_file, directory)
        outputs = build_self(compiler, ROOT, directory / "build")
        outputs_by_stage.append(outputs)
        stages.append(
            {
                "stage": number,
                "c": artifact(directory / "compiler.c"),
                "compile_command": command,
                "executable": artifact(compiler),
                "outputs": {name: sha256_bytes(data) for name, data in outputs.items()},
            }
        )
        c_file = directory / "build" / "compiler.c"
    last = outputs_by_stage[-1]
    for number, outputs in enumerate(outputs_by_stage, start=1):
        for name in OUTPUTS:
            if outputs[name] != last[name]:
                raise BootstrapError(f"stage{number} writes a different {name} from stage3")
    if stage1_c.read_bytes() != last["compiler.c"]:
        raise BootstrapError("stage0's C for stage1 differs from the C the stages write")
    executables = {stage["executable"]["sha256"] for stage in stages}
    if len(executables) != 1:
        raise BootstrapError("the three stage executables are not byte-identical")
    if last["diagnostics.txt"]:
        raise BootstrapError("a stage wrote diagnostics for a build that succeeded")
    return {
        "stages": stages,
        "c_identical": True,
        "c_equals_stage0_c": True,
        "manifests_identical": True,
        "executables_identical": True,
    }


def paths_and_order(out: pathlib.Path, stage3: pathlib.Path, expected: dict[str, bytes]) -> dict[str, Any]:
    root, modules, outputs = build_entries(BUILD_FILE.read_text(encoding="utf-8"))
    files = [root, *modules]
    # The same sources under another directory.
    relocated = out / "relocated" / "sources"
    if relocated.parent.exists():
        shutil.rmtree(relocated.parent)
    copy_package(files, ROOT, relocated)
    write_build_file(relocated / "argorix.build", root, modules, outputs)
    moved = build_self(stage3, relocated, out / "relocated" / "build")
    for name in OUTPUTS:
        if moved[name] != expected[name]:
            raise BootstrapError(f"building from another directory changes {name}")
    # The modules listed in reverse.
    reordered = out / "reordered" / "sources"
    if reordered.parent.exists():
        shutil.rmtree(reordered.parent)
    copy_package(files, ROOT, reordered)
    write_build_file(reordered / "argorix.build", root, list(reversed(modules)), outputs)
    shuffled = build_self(stage3, reordered, out / "reordered" / "build")
    if shuffled["compiler.c"] != expected["compiler.c"]:
        raise BootstrapError("listing the modules in another order changes the C")
    before = json.loads(expected["compiler.json"])
    after = json.loads(shuffled["compiler.json"])
    by_path = lambda sources: sorted(sources, key=lambda source: source["path"])  # noqa: E731
    if by_path(before.pop("sources")) != by_path(after.pop("sources")) or before != after:
        raise BootstrapError("listing the modules in another order changes the manifest beyond the order of its sources")
    return {
        "relocated_outputs_identical": True,
        "reordered_c_identical": True,
        "reordered_manifest_differs_only_in_source_order": True,
    }


def locked_set(root: str) -> list[str]:
    """What the fixture harness passes stage0: the root's directory, then
    `compiler/` and `stdlib/`, every `.argx` file but the root."""
    directories = [str(pathlib.PurePosixPath(root).parent), "compiler", "stdlib"]
    seen: set[str] = set()
    files: list[str] = []
    for directory in directories:
        if directory in seen:
            continue
        seen.add(directory)
        for path in sorted((ROOT / directory).glob("*.argx")):
            file = path.relative_to(ROOT).as_posix()
            if file != root:
                files.append(file)
    return files


def compile_case(compiler: pathlib.Path, root: str, work: pathlib.Path) -> bytes:
    package = work / "package"
    if package.exists():
        shutil.rmtree(package)
    modules = locked_set(root)
    copy_package([root, *modules], ROOT, package)
    write_build_file(
        package / "argorix.build",
        root,
        modules,
        {"c": "case.c", "manifest": "case.json", "diagnostics": "case.txt"},
    )
    build = work / f"build-{compiler.parent.name}"
    result = run_compiler(compiler, package, build)
    if result != "0":
        raise BootstrapError(f"{root}: {relative(compiler)} returned {result}: {(build / 'case.txt').read_text(errors='replace')}")
    return (build / "case.c").read_bytes()


def strip_one_newline(text: str) -> str:
    return text[:-1] if text.endswith("\n") else text


def run_case(cc: str, suite: pathlib.Path, case: dict[str, Any], c_source: bytes, work: pathlib.Path) -> list[str]:
    c_file = work / "case.c"
    c_file.write_bytes(c_source)
    executable = work / "case"
    command = [cc, "-std=c11", "-Wall", "-Wextra", "-Werror", "-I", str(RUNTIME_DIR), str(c_file), str(RUNTIME_C), "-o", str(executable)]
    compiled = run(command, "case compile")
    if compiled.returncode != 0:
        return [f"C compilation failed: {compiled.stderr.decode(errors='replace')[:500]}"]
    return execute_case(suite, case, executable, work)


def execute_case(suite: pathlib.Path, case: dict[str, Any], executable: pathlib.Path, work: pathlib.Path) -> list[str]:
    """Runs a case's executable with its host roots and budgets, and checks
    its exit status, output and build files against the case."""
    arguments: list[str] = []
    host = case.get("host")
    build = work / "build"
    if host:
        if "package_root" in host:
            arguments += ["--package-root", str((suite.parent / host["package_root"]).resolve())]
        if "read_budget" in host:
            arguments += ["--read-budget", str(host["read_budget"])]
        if "write_budget" in host:
            if build.exists():
                shutil.rmtree(build)
            build.mkdir(parents=True)
            for directory in host.get("build_dirs", []):
                (build / directory).mkdir(parents=True, exist_ok=True)
            arguments += ["--build-root", str(build), "--write-budget", str(host["write_budget"])]
    try:
        execution = subprocess.run(
            [str(executable), *arguments], capture_output=True, env=environment(), timeout=TIMEOUT_SECONDS, check=False
        )
    except subprocess.TimeoutExpired:
        return [f"timed out after {TIMEOUT_SECONDS} s"]
    failures = []
    if execution.returncode != case["expected_exit"]:
        failures.append(f"exit {execution.returncode} != {case['expected_exit']}")
    stdout = strip_one_newline(execution.stdout.decode(errors="replace"))
    if stdout != case["expected_stdout"]:
        failures.append(f"stdout {stdout[:200]!r} != {case['expected_stdout'][:200]!r}")
    stderr = strip_one_newline(execution.stderr.decode(errors="replace"))
    if stderr != case["expected_stderr"]:
        failures.append(f"stderr {stderr[:200]!r} != {case['expected_stderr'][:200]!r}")
    if host and "write_budget" in host:
        found = {
            path.relative_to(build).as_posix(): path.read_bytes()
            for path in build.rglob("*")
            if path.is_file()
        }
        expected = {name: text.encode() for name, text in host.get("expected_build", {}).items()}
        if found != expected:
            failures.append(
                f"build files differ: missing or different {sorted(name for name in expected if found.get(name) != expected[name])}, "
                f"unexpected {sorted(set(found) - set(expected))}"
            )
    return failures


def suite(cc: str, stage2: pathlib.Path, stage3: pathlib.Path, out: pathlib.Path) -> dict[str, Any]:
    results = []
    failed = []
    for manifest in SUITES:
        path = ROOT / manifest
        cases = json.loads(path.read_text(encoding="utf-8"))["cases"]
        for case in cases:
            root = (path.parent / case["file"]).resolve().relative_to(ROOT).as_posix()
            work = out / "suite" / f"{path.parent.name}-{case['id']}"
            work.mkdir(parents=True, exist_ok=True)
            from_stage2 = compile_case(stage2, root, work)
            from_stage3 = compile_case(stage3, root, work)
            failures = []
            if from_stage2 != from_stage3:
                failures.append("stage2 and stage3 write different C")
            failures += run_case(cc, path, case, from_stage3, work)
            results.append({"suite": manifest, "id": case["id"], "c_sha256": sha256_bytes(from_stage3), "passed": not failures})
            if failures:
                failed.append(f"{manifest} {case['id']}: {'; '.join(failures)}")
    if failed:
        raise BootstrapError("cases failed with the stage3 compiler:\n" + "\n".join(failed))
    return {"cases": len(results), "passed": len(results), "results": results}


def mutated(out: pathlib.Path, name: str, edits: list[tuple[str, str, str]]) -> pathlib.Path:
    """A copy of the compiler's sources with `edits` made: (file, old, new),
    each `old` found exactly as many times as it is replaced."""
    root, modules, outputs = build_entries(BUILD_FILE.read_text(encoding="utf-8"))
    sources = out / "mutations" / name / "sources"
    if sources.parent.exists():
        shutil.rmtree(sources.parent)
    copy_package([root, *modules], ROOT, sources)
    shutil.copyfile(BUILD_FILE, sources / "argorix.build")
    for file, old, new in edits:
        text = (sources / file).read_text(encoding="utf-8")
        if old not in text:
            raise BootstrapError(f"the edit of {file} finds nothing to change")
        (sources / file).write_text(text.replace(old, new), encoding="utf-8")
    return sources


def no_seed(cc: str, stage3: pathlib.Path, out: pathlib.Path, original_c: bytes) -> dict[str, Any]:
    failing = ROOT / "tests" / "selfhost" / "stage1" / "check_package"
    program = ROOT / "tests" / "selfhost" / "stage1" / "emit_package"
    report: dict[str, Any] = {}

    # A diagnostic reworded: the new compiler says it the new way.
    old_wording, new_wording = "unknown name `", "no local or constant is named `"
    sources = mutated(out, "diagnostic", [("compiler/check.argx", old_wording, new_wording)])
    base = out / "mutations" / "diagnostic"
    first = build_self(stage3, sources, base / "first-build")
    if first["compiler.c"] == original_c:
        raise BootstrapError("stage3 wrote the same C for edited sources")
    edited, _ = compile_compiler(cc, base / "first-build" / "compiler.c", base / "gen1")
    again = build_self(edited, sources, base / "gen1" / "build")
    if again["compiler.c"] != first["compiler.c"]:
        raise BootstrapError("the reworded compiler does not reproduce itself")
    if run_compiler(stage3, failing, base / "stage3-diagnostics", ("out",)) != "1":
        raise BootstrapError("stage3 compiled a package that does not check")
    if run_compiler(edited, failing, base / "edited-diagnostics", ("out",)) != "1":
        raise BootstrapError("the reworded compiler compiled a package that does not check")
    before = (base / "stage3-diagnostics" / "out" / "diagnostics.txt").read_text(encoding="utf-8")
    after = (base / "edited-diagnostics" / "out" / "diagnostics.txt").read_text(encoding="utf-8")
    if old_wording not in before or new_wording not in after or old_wording in after:
        raise BootstrapError("the reworded diagnostic does not show in the new compiler's output")
    report["diagnostic"] = {
        "edit": {"file": "compiler/check.argx", "from": old_wording, "to": new_wording},
        "c_changed": True,
        "fixed_point_in_one_generation": True,
        "stage3_says": next(line for line in before.splitlines() if old_wording in line),
        "edited_compiler_says": next(line for line in after.splitlines() if new_wording in line),
    }

    # A function of the C backend changed: the header it writes.
    old_header = "/* generated by Argorix Core C backend 0.1 */"
    new_header = "/* generated by Argorix Core C backend 0.1, edited */"
    sources = mutated(out, "backend", [("compiler/c_emit.argx", old_header, new_header)])
    base = out / "mutations" / "backend"
    first = build_self(stage3, sources, base / "first-build")
    if not first["compiler.c"].startswith(old_header.encode()):
        raise BootstrapError("stage3 did not write its own header")
    gen1, _ = compile_compiler(cc, base / "first-build" / "compiler.c", base / "gen1")
    second = build_self(gen1, sources, base / "gen1" / "build")
    if not second["compiler.c"].startswith(new_header.encode()):
        raise BootstrapError("the edited backend does not write the new header")
    first_rest = first["compiler.c"].split(b"\n", 1)[1]
    second_rest = second["compiler.c"].split(b"\n", 1)[1]
    if first_rest != second_rest:
        raise BootstrapError("the edited backend changes more than the header")
    gen2, _ = compile_compiler(cc, base / "gen1" / "build" / "compiler.c", base / "gen2")
    third = build_self(gen2, sources, base / "gen2" / "build")
    if third["compiler.c"] != second["compiler.c"]:
        raise BootstrapError("the edited backend does not reach a fixed point")
    # Its output for another program shows the edit too.
    if run_compiler(gen1, program, base / "program", ("out",)) != "0":
        raise BootstrapError("the edited backend does not compile another program")
    emitted = (base / "program" / "out" / "program.c").read_bytes()
    if not emitted.startswith(new_header.encode()):
        raise BootstrapError("the edited backend does not change another program's C")
    report["backend"] = {
        "edit": {"file": "compiler/c_emit.argx", "from": old_header, "to": new_header},
        "generation1_header_unchanged": True,
        "generation2_header_changed_only": True,
        "fixed_point_at_generation": 2,
        "other_program_changed": True,
    }
    return report


# ------------------------------------------------------------------ host checks


def network_interfaces() -> list[str]:
    """The network interfaces of this process's network namespace, from
    `/proc/net/dev` (`/sys/class/net` can show the host's)."""
    table = pathlib.Path("/proc/net/dev")
    if not table.is_file():
        return []
    lines = table.read_text(encoding="utf-8").splitlines()[2:]
    return sorted(line.split(":", 1)[0].strip() for line in lines if ":" in line)


def os_release() -> str | None:
    path = pathlib.Path("/etc/os-release")
    if not path.is_file():
        return None
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith("PRETTY_NAME="):
            return line.split("=", 1)[1].strip('"')
    return None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--from", dest="source", type=pathlib.Path, default=ROOT / "target" / "stage1")
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--out", type=pathlib.Path, default=ROOT / "target" / "selfhost")
    parser.add_argument("--require-rust-free-host", action="store_true")
    parser.add_argument("--require-no-network", action="store_true")
    parser.add_argument("--skip-suite", action="store_true", help="skip the fixture suites (for a quick run)")
    arguments = parser.parse_args()
    out = arguments.out if arguments.out.is_absolute() else pathlib.Path.cwd() / arguments.out
    source = arguments.source if arguments.source.is_absolute() else pathlib.Path.cwd() / arguments.source
    try:
        stage0_record = source / "stage0.json"
        stage1_c = source / "stage1.c"
        if not stage0_record.is_file() or not stage1_c.is_file():
            raise BootstrapError(f"{relative(source)} has no stage1.c and stage0.json: run `bootstrap/stage1.py emit` first")
        stage0 = json.loads(stage0_record.read_text(encoding="utf-8"))
        if sha256(stage1_c) != stage0["stage1_c"]["sha256"]:
            raise BootstrapError("stage1.c is not the file stage0 recorded")
        rust_tools = {tool: shutil.which(tool) for tool in RUST_TOOLS}
        if arguments.require_rust_free_host and any(rust_tools.values()):
            raise BootstrapError(f"this host is meant to have no Rust tools, but has {rust_tools}")
        interfaces = network_interfaces()
        if arguments.require_no_network and [name for name in interfaces if name != "lo"]:
            raise BootstrapError(f"this host is meant to have no network, but has {interfaces}")
        cc_path = shutil.which(arguments.cc)
        if cc_path is None:
            raise BootstrapError(f"no C compiler named {arguments.cc}")
        if "ccache" in pathlib.Path(cc_path).resolve().name:
            raise BootstrapError(f"{arguments.cc} is ccache, a cache this build must not use")
        out.mkdir(parents=True, exist_ok=True)

        chain = generations(arguments.cc, stage1_c, out)
        stage2 = out / "stage2" / "argorixc"
        stage3 = out / "stage3" / "argorixc"
        expected = {name: (out / "stage3" / "build" / name).read_bytes() for name in OUTPUTS}
        placement = paths_and_order(out, stage3, expected)
        tested = {"skipped": True} if arguments.skip_suite else suite(arguments.cc, stage2, stage3, out)
        seed = no_seed(arguments.cc, stage3, out, expected["compiler.c"])
        report = {
            "schema_version": 1,
            "task": "ESP-015",
            "stage": "B2",
            "bootstrap_dependency": "rust",
            "source": stage0["source"],
            "stage0": stage0["stage0"],
            "stage1_c": stage0["stage1_c"],
            "host": {
                "system": platform.system(),
                "machine": platform.machine(),
                "os": os_release(),
                "python": platform.python_version(),
                "rust_tools": rust_tools,
                "rust_free_required": arguments.require_rust_free_host,
                "network_interfaces": interfaces,
                "no_network_required": arguments.require_no_network,
            },
            "c_compiler": {
                "name": arguments.cc,
                "path": cc_path,
                "version": first_line([arguments.cc, "--version"]),
                "flags": COMPILER_FLAGS,
                "runtime": artifact(RUNTIME_C),
            },
            "environment": environment() | {"PATH": "(inherited)"},
            "build_file": artifact(BUILD_FILE),
            "generations": chain,
            "paths_and_order": placement,
            "suite": tested,
            "no_prebuilt_seed": seed,
            "limits": [
                "Equal stages show that the compiler reproduces itself from its sources. They do not "
                "show that stage0, which wrote stage1.c, carries no defect that reproduces itself as "
                "well (a trusting-trust attack); diverse double compilation is ESP-023 and MAT-023.",
                "The C compiler and its C library are trusted and recorded, not verified.",
            ],
        }
        path = out / "selfhost-report.json"
        path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    except BootstrapError as error:
        print(f"self-hosting bootstrap failed: {error}", file=sys.stderr)
        return 1
    stage = chain["stages"][-1]
    suite_text = "suite skipped" if arguments.skip_suite else f"{tested['passed']} suite cases passed with stage3"
    print(
        f"stage1, stage2 and stage3 are identical: C {stage['c']['sha256'][:16]}, "
        f"executable {stage['executable']['sha256'][:16]}; {suite_text}; "
        f"both source edits propagate. Report: {relative(path)}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
