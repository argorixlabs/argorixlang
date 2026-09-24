#!/usr/bin/env python3
"""ESP-016: the compiler built natively, with no C compiler on the path.

    python3 bootstrap/native.py seed --from target/stage1 --cc gcc --out target/native
    python3 bootstrap/native.py bootstrap --out target/native \\
        [--require-no-c-compiler] [--require-rust-free-host] [--require-no-network]
    python3 bootstrap/native.py cases --out target/native CASES.json...

`seed` is the last step that needs a C compiler:

1. **The shim.** The C1 runtime (`bootstrap/c/argorix_core_runtime.c`) and
   the native shim (`bootstrap/native/`) are compiled once, to objects. They
   are the declared dependency native programs link against
   (spec/core/native-x86-64.md). `shim/shim.json` records their sources,
   the flags and the compiler.
2. **Stage1**, compiled from `stage1.c`, which stage0 wrote
   (`bootstrap/stage1.py emit`), as ESP-014 and ESP-015 compile it.
3. **The seed.** Stage1 compiles the compiler's sources to a native object,
   `seed/build/compiler.o`, from the build file of a native build:
   `argorix.build` with `object` in place of `c`, and the limits the C
   profile's flags give the compiler.

`bootstrap` needs the GNU linker, the C library's start files and Python.
With `--require-no-c-compiler` it fails if any C compiler is on the path.

1. **Generations.** The seed object is linked into native1. native1 compiles
   the compiler into native2's object, and native2 into native3's. The three
   objects and manifests must be the seed's, byte for byte, and the three
   executables must be byte-identical. native3 also writes the compiler's C,
   which must be `stage1.c`.
2. **Paths and order.** native3 builds the sources copied under another
   directory, and with the `module` lines reversed: the object must not
   change, and the manifest may differ only in the order of its sources.
3. **The suite.** Every case of the fixture suites (`tests/selfhost/*` and
   the backend regression corpus) is compiled by native2 and by native3,
   which must write the same object. It is linked with the shim, run with the
   case's host roots and budgets, and checked against the case.
4. **No prebuilt seed.** Two edits to a copy of the compiler's sources. A
   diagnostic reworded: native3 compiles a compiler that says it the new
   way. The native backend changed (small immediates written as 64-bit
   ones): generation 1 still writes the old code, generation 2 the new one,
   and generation 3 reaches a fixed point.

`cases` compiles the cases of any `cases.json` natively with a compiler of
`--out` (stage1 by default), links and runs them, and checks each against
its expected exit status and output: the fixture format, which the
generated programs of `core-c-harness generate` share with their oracle's
expectations.

It writes `native-report.json`. The linker, the C library and the shim are
trusted: recorded, not verified. As in ESP-015, equal generations show that
the compiler reproduces itself, not that the seed carries no defect that
reproduces itself too; that is ESP-023 and MAT-023.
"""

from __future__ import annotations

import argparse
import json
import os
import pathlib
import platform
import shutil
import sys
from typing import Any

import selfhost
from selfhost import (
    BIG_BUDGET,
    BUILD_FILE,
    ROOT,
    SUITES,
    BootstrapError,
    artifact,
    build_entries,
    copy_package,
    environment,
    execute_case,
    first_line,
    locked_set,
    network_interfaces,
    os_release,
    relative,
    run,
    run_compiler,
    sha256,
    sha256_bytes,
)

SHIM_SOURCES = [
    "bootstrap/c/argorix_core_runtime.c",
    "bootstrap/native/argorix_native_shim.c",
    "bootstrap/native/argorix_native_host.c",
]
SHIM_FLAGS = ["-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-c"]
# The limits the compiler's own build needs, as the C profile's flags give
# them (bootstrap/stage1.py).
COMPILER_LIMITS = [("steps", "400000000000"), ("buffer-bytes", "268435456")]
OUTPUTS = ["compiler.o", "compiler.json", "diagnostics.txt"]
DYNAMIC_LINKER = "/lib64/ld-linux-x86-64.so.2"
START_FILES = ["crt1.o", "crti.o"]
END_FILES = ["crtn.o"]
C_COMPILER_NAMES = ["cc", "c89", "c99", "gcc", "clang", "tcc", "icx", "icc"]
RUST_TOOLS = ["rustc", "cargo", "rustup"]


# ------------------------------------------------------------------ build files


def native_build_text(text: str) -> str:
    """A build file with `object` in place of `c`, and the compiler's limits."""
    lines = []
    for line in text.splitlines():
        if line.startswith("c "):
            line = "object " + line[2:].removesuffix(".c") + ".o"
        lines.append(line)
    lines += [f"{key} {value}" for key, value in COMPILER_LIMITS]
    return "\n".join(lines) + "\n"


def compiler_package(target: pathlib.Path, source: pathlib.Path = ROOT, build_text: str | None = None, native: bool = True) -> pathlib.Path:
    """The compiler's sources, as `argorix.build` lists them, under
    `target`, with a native build file unless `native` is false."""
    if target.exists():
        shutil.rmtree(target)
    text = (source / "argorix.build").read_text(encoding="utf-8") if build_text is None else build_text
    root, modules, _ = build_entries(text)
    copy_package([root, *modules], source, target)
    (target / "argorix.build").write_text(native_build_text(text) if native else text, encoding="utf-8")
    return target


def native_package(source: pathlib.Path, target: pathlib.Path) -> pathlib.Path:
    """A copy of a package whose build file asks for C, asking for an object."""
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(source, target)
    text = (target / "argorix.build").read_text(encoding="utf-8")
    lines = ["object " + line[2:].removesuffix(".c") + ".o" if line.startswith("c ") else line for line in text.splitlines()]
    (target / "argorix.build").write_text("\n".join(lines) + "\n", encoding="utf-8")
    return target


def build(compiler: pathlib.Path, package: pathlib.Path, out: pathlib.Path) -> dict[str, bytes]:
    result = run_compiler(compiler, package, out)
    if result != "0":
        diagnostics = (out / "diagnostics.txt").read_text(encoding="utf-8", errors="replace")
        raise BootstrapError(f"{relative(compiler)} returned {result}:\n{diagnostics}")
    return {name: (out / name).read_bytes() for name in OUTPUTS}


# ------------------------------------------------------------------ linking


def libdir(requested: pathlib.Path | None) -> pathlib.Path:
    if requested is not None:
        return requested
    for candidate in ["/usr/lib/x86_64-linux-gnu", "/usr/lib64", "/usr/lib"]:
        path = pathlib.Path(candidate)
        if (path / "crt1.o").is_file():
            return path
    raise BootstrapError("no directory with the C library's start files (crt1.o)")


def link(objects: list[pathlib.Path], executable: pathlib.Path, shim: list[pathlib.Path], lib: pathlib.Path) -> list[str]:
    """Links native objects with the shim and the C library: the link line of
    spec/core/native-x86-64.md. Paths are made relative to the executable's
    directory, so the same inputs give the same command."""
    directory = executable.parent
    def here(path: pathlib.Path) -> str:
        return os.path.relpath(path, directory)
    command = [
        "ld",
        "-o",
        executable.name,
        "-dynamic-linker",
        DYNAMIC_LINKER,
        *[str(lib / name) for name in START_FILES],
        *[here(path) for path in objects],
        *[here(path) for path in shim],
        f"-L{lib}",
        "-lc",
        *[str(lib / name) for name in END_FILES],
    ]
    result = run(command, f"ld for {relative(executable)}", cwd=directory)
    if result.returncode != 0:
        raise BootstrapError(f"ld failed for {relative(executable)}: {result.stderr.decode(errors='replace')}")
    return command


def shim_objects(out: pathlib.Path) -> list[pathlib.Path]:
    return [out / "shim" / (pathlib.PurePosixPath(source).stem + ".o") for source in SHIM_SOURCES]


# ------------------------------------------------------------------ seed


def seed(cc: str, source: pathlib.Path, out: pathlib.Path) -> None:
    stage0_record = source / "stage0.json"
    stage1_c = source / "stage1.c"
    if not stage0_record.is_file() or not stage1_c.is_file():
        raise BootstrapError(f"{relative(source)} has no stage1.c and stage0.json: run `bootstrap/stage1.py emit` first")
    stage0 = json.loads(stage0_record.read_text(encoding="utf-8"))
    if sha256(stage1_c) != stage0["stage1_c"]["sha256"]:
        raise BootstrapError("stage1.c is not the file stage0 recorded")
    cc_path = shutil.which(cc)
    if cc_path is None:
        raise BootstrapError(f"no C compiler named {cc}")
    out.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(stage1_c, out / "stage1.c")
    shutil.copyfile(stage0_record, out / "stage0.json")

    # The shim, compiled from the repository root with relative paths.
    shim_dir = out / "shim"
    shim_dir.mkdir(parents=True, exist_ok=True)
    commands = []
    for source_file, object_file in zip(SHIM_SOURCES, shim_objects(out)):
        command = [cc, *SHIM_FLAGS, "-I", "bootstrap/c", source_file, "-o", str(object_file)]
        result = run(command, f"{cc} on {source_file}")
        if result.returncode != 0:
            raise BootstrapError(f"{cc} failed on {source_file}: {result.stderr.decode(errors='replace')}")
        commands.append(command)
    shim_record = {
        "c_compiler": {"name": cc, "path": cc_path, "version": first_line([cc, "--version"])},
        "flags": SHIM_FLAGS,
        "sources": [artifact(ROOT / file) for file in [*SHIM_SOURCES, "bootstrap/c/argorix_core_runtime.h", "bootstrap/c/argorix_core_host.h"]],
        "objects": [artifact(path) for path in shim_objects(out)],
    }
    (shim_dir / "shim.json").write_text(json.dumps(shim_record, indent=2) + "\n", encoding="utf-8")

    # Stage1, then the seed object it writes.
    stage1, stage1_command = selfhost.compile_compiler(cc, stage1_c, out / "stage1")
    package = compiler_package(out / "seed" / "package")
    outputs = build(stage1, package, out / "seed" / "build")
    record = {
        "schema_version": 1,
        "task": "ESP-016",
        "source": stage0["source"],
        "stage0": stage0["stage0"],
        "stage1_c": stage0["stage1_c"],
        "stage1": {"command": stage1_command, "executable": artifact(stage1)},
        "shim": shim_record,
        "build_file": (package / "argorix.build").read_text(encoding="utf-8"),
        "seed": {name: {"bytes": len(data), "sha256": sha256_bytes(data)} for name, data in outputs.items()},
    }
    (out / "seed.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(
        f"seed object {record['seed']['compiler.o']['sha256'][:16]} ({record['seed']['compiler.o']['bytes']} bytes), "
        f"written by stage1; shim built with {cc}. Record: {relative(out / 'seed.json')}"
    )


# ------------------------------------------------------------------ bootstrap


def generations(out: pathlib.Path, shim: list[pathlib.Path], lib: pathlib.Path, seed_outputs: dict[str, bytes]) -> dict[str, Any]:
    """native1 from the seed object; native2 and native3 from their
    predecessors' builds. Every stage directory is at the same depth."""
    package = compiler_package(out / "package")
    stages = []
    link_command: list[str] = []
    previous = out / "seed" / "build" / "compiler.o"
    executables = []
    for number in (1, 2, 3):
        directory = out / f"native{number}"
        directory.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(previous, directory / "compiler.o")
        link_command = link([directory / "compiler.o"], directory / "argorixc", shim, lib)
        executable = directory / "argorixc"
        outputs = build(executable, package, directory / "build")
        if outputs != seed_outputs:
            changed = [name for name in OUTPUTS if outputs[name] != seed_outputs[name]]
            raise BootstrapError(f"native{number} does not reproduce the seed's {changed}")
        executables.append(executable.read_bytes())
        stages.append(
            {
                "stage": f"native{number}",
                "executable": artifact(executable),
                "writes": {name: {"bytes": len(data), "sha256": sha256_bytes(data)} for name, data in outputs.items()},
            }
        )
        previous = directory / "build" / "compiler.o"
    if len(set(executables)) != 1:
        raise BootstrapError("native1, native2 and native3 are not byte-identical executables")
    # The transitional C, from the native compiler: stage0's own.
    c_package = compiler_package(out / "c-package", native=False)
    c_build = out / "native3" / "c-build"
    if run_compiler(out / "native3" / "argorixc", c_package, c_build) != "0":
        raise BootstrapError("native3 did not write the compiler's C")
    if (c_build / "compiler.c").read_bytes() != (out / "stage1.c").read_bytes():
        raise BootstrapError("native3's C for the compiler is not stage1.c")
    return {
        "stages": stages,
        "link_command": link_command,
        "objects_identical": True,
        "executables_identical": True,
        "native3_writes_stage1_c": True,
    }


def paths_and_order(out: pathlib.Path, native3: pathlib.Path, expected: dict[str, bytes]) -> dict[str, Any]:
    relocated = compiler_package(out / "relocated" / "deeper" / "package")
    moved = build(native3, relocated, out / "relocated" / "build")
    if moved != expected:
        raise BootstrapError("the relocated sources give different outputs")
    lines = BUILD_FILE.read_text(encoding="utf-8").splitlines()
    first = next(index for index, line in enumerate(lines) if line.startswith("module "))
    last = max(index for index, line in enumerate(lines) if line.startswith("module "))
    reordered_text = "\n".join([*lines[:first], *reversed(lines[first : last + 1]), *lines[last + 1 :]]) + "\n"
    reordered = compiler_package(out / "reordered" / "package", build_text=reordered_text)
    shuffled = build(native3, reordered, out / "reordered" / "build")
    if shuffled["compiler.o"] != expected["compiler.o"]:
        raise BootstrapError("reordering the modules changes the object")
    before = json.loads(expected["compiler.json"])
    after = json.loads(shuffled["compiler.json"])
    if {key: value for key, value in before.items() if key != "sources"} != {key: value for key, value in after.items() if key != "sources"}:
        raise BootstrapError("reordering the modules changes the manifest beyond its sources")
    if sorted(json.dumps(source, sort_keys=True) for source in before["sources"]) != sorted(json.dumps(source, sort_keys=True) for source in after["sources"]):
        raise BootstrapError("reordering the modules changes the sources the manifest names")
    return {"relocated_outputs_identical": True, "reordered_object_identical": True, "reordered_manifest_differs_only_in_source_order": True}


def compile_case(compiler: pathlib.Path, root: str, work: pathlib.Path) -> pathlib.Path:
    package = work / "package"
    if package.exists():
        shutil.rmtree(package)
    modules = locked_set(root)
    copy_package([root, *modules], ROOT, package)
    lines = ["argorix-build 1", f"root {root}", *[f"module {module}" for module in modules], "object case.o", "manifest case.json", "diagnostics case.txt"]
    (package / "argorix.build").write_text("\n".join(lines) + "\n", encoding="utf-8")
    build_dir = work / f"build-{compiler.parent.name}"
    result = run_compiler(compiler, package, build_dir)
    if result != "0":
        raise BootstrapError(f"{root}: {relative(compiler)} returned {result}: {(build_dir / 'case.txt').read_text(errors='replace')}")
    return build_dir / "case.o"


def suite(out: pathlib.Path, native2: pathlib.Path, native3: pathlib.Path, shim: list[pathlib.Path], lib: pathlib.Path) -> dict[str, Any]:
    results = []
    failed = []
    for manifest in SUITES:
        path = ROOT / manifest
        cases = json.loads(path.read_text(encoding="utf-8"))["cases"]
        for case in cases:
            root = (path.parent / case["file"]).resolve().relative_to(ROOT).as_posix()
            work = out / "suite" / f"{path.parent.name}-{case['id']}"
            work.mkdir(parents=True, exist_ok=True)
            from_native2 = compile_case(native2, root, work)
            from_native3 = compile_case(native3, root, work)
            failures = []
            if from_native2.read_bytes() != from_native3.read_bytes():
                failures.append("native2 and native3 write different objects")
            link([from_native3], work / "case", shim, lib)
            failures += execute_case(path, case, work / "case", work)
            results.append({"suite": manifest, "id": case["id"], "object_sha256": sha256(from_native3), "passed": not failures})
            if failures:
                failed.append(f"{manifest} {case['id']}: {'; '.join(failures)}")
    if failed:
        raise BootstrapError("cases failed with the native3 compiler:\n" + "\n".join(failed))
    return {"cases": len(results), "passed": len(results), "results": results}


def mutated(out: pathlib.Path, name: str, edits: list[tuple[str, str, str]]) -> pathlib.Path:
    """A copy of the compiler's sources with `edits` made: (file, old, new)."""
    base = out / "mutations" / name
    if base.exists():
        shutil.rmtree(base)
    sources = compiler_package(base / "sources")
    for file, old, new in edits:
        text = (sources / file).read_text(encoding="utf-8")
        if old not in text:
            raise BootstrapError(f"the edit of {file} finds nothing to change")
        (sources / file).write_text(text.replace(old, new), encoding="utf-8")
    return sources


def text_size(object_bytes: bytes) -> int:
    """The size of an ELF64 object's `.text`, its first section after the
    null one."""
    sections = int.from_bytes(object_bytes[0x28:0x30], "little")
    return int.from_bytes(object_bytes[sections + 64 + 32 : sections + 64 + 40], "little")


def no_seed(out: pathlib.Path, native3: pathlib.Path, shim: list[pathlib.Path], lib: pathlib.Path, original: bytes) -> dict[str, Any]:
    failing = native_package(ROOT / "tests" / "selfhost" / "stage1" / "check_package", out / "mutations" / "check_package")
    program = native_package(ROOT / "tests" / "selfhost" / "stage1" / "emit_package", out / "mutations" / "emit_package")
    report: dict[str, Any] = {}

    # A diagnostic reworded: the new compiler says it the new way.
    old_wording, new_wording = "unknown name `", "no local or constant is named `"
    sources = mutated(out, "diagnostic", [("compiler/check.argx", old_wording, new_wording)])
    base = out / "mutations" / "diagnostic"
    first = build(native3, sources, base / "first-build")
    if first["compiler.o"] == original:
        raise BootstrapError("native3 wrote the same object for edited sources")
    (base / "gen1").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(base / "first-build" / "compiler.o", base / "gen1" / "compiler.o")
    link([base / "gen1" / "compiler.o"], base / "gen1" / "argorixc", shim, lib)
    edited = base / "gen1" / "argorixc"
    again = build(edited, sources, base / "gen1" / "build")
    if again["compiler.o"] != first["compiler.o"]:
        raise BootstrapError("the reworded compiler does not reproduce itself")
    if run_compiler(native3, failing, base / "native3-diagnostics", ("out",)) != "1":
        raise BootstrapError("native3 compiled a package that does not check")
    if run_compiler(edited, failing, base / "edited-diagnostics", ("out",)) != "1":
        raise BootstrapError("the reworded compiler compiled a package that does not check")
    before = (base / "native3-diagnostics" / "out" / "diagnostics.txt").read_text(encoding="utf-8")
    after = (base / "edited-diagnostics" / "out" / "diagnostics.txt").read_text(encoding="utf-8")
    if old_wording not in before or new_wording not in after or old_wording in after:
        raise BootstrapError("the reworded diagnostic does not show in the new compiler's output")
    report["diagnostic"] = {
        "edit": {"file": "compiler/check.argx", "from": old_wording, "to": new_wording},
        "object_changed": True,
        "fixed_point_in_one_generation": True,
        "native3_says": next(line for line in before.splitlines() if old_wording in line),
        "edited_compiler_says": next(line for line in after.splitlines() if new_wording in line),
    }

    # A function of the native backend changed: small immediates are written
    # in the 64-bit form too.
    old_move = "        return asm(e, x86.mov_imm32(Buffer::new(), reg, value));"
    new_move = "        return asm(e, x86.mov_imm(Buffer::new(), reg, value));"
    sources = mutated(out, "backend", [("compiler/native.argx", old_move, new_move)])
    base = out / "mutations" / "backend"
    first = build(native3, sources, base / "first-build")
    gens: list[dict[str, bytes]] = [first]
    for number in (1, 2):
        directory = base / f"gen{number}"
        directory.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(base / ("first-build" if number == 1 else "gen1/build") / "compiler.o", directory / "compiler.o")
        link([directory / "compiler.o"], directory / "argorixc", shim, lib)
        gens.append(build(directory / "argorixc", sources, directory / "build"))
    first_text, second_text, third_text = (text_size(outputs["compiler.o"]) for outputs in gens)
    if gens[1]["compiler.o"] == first["compiler.o"] or second_text <= first_text:
        raise BootstrapError("the edited backend does not write the longer moves")
    if gens[2]["compiler.o"] != gens[1]["compiler.o"]:
        raise BootstrapError("the edited backend does not reach a fixed point")
    # Its output for another program shows the edit too, and still runs.
    if run_compiler(native3, program, base / "program-before", ("out",)) != "0":
        raise BootstrapError("native3 does not compile another program")
    if run_compiler(base / "gen1" / "argorixc", program, base / "program-after", ("out",)) != "0":
        raise BootstrapError("the edited backend does not compile another program")
    object_before = base / "program-before" / "out" / "program.o"
    object_after = base / "program-after" / "out" / "program.o"
    if text_size(object_after.read_bytes()) <= text_size(object_before.read_bytes()):
        raise BootstrapError("the edited backend does not change another program's code")
    runs = []
    for label, object_file in (("before", object_before), ("after", object_after)):
        link([object_file], base / f"program-{label}" / "program", shim, lib)
        result = run([str(base / f"program-{label}" / "program")], f"program {label}")
        runs.append((result.returncode, result.stdout, result.stderr))
    if runs[0] != runs[1]:
        raise BootstrapError("the edited backend changes what another program does")
    report["backend"] = {
        "edit": {"file": "compiler/native.argx", "from": old_move.strip(), "to": new_move.strip()},
        "text_bytes": {"generation1": first_text, "generation2": second_text, "generation3": third_text},
        "fixed_point_at_generation": 2,
        "other_program_changed": True,
        "other_program_behaves_the_same": True,
    }
    return report


def cases(out: pathlib.Path, compiler: pathlib.Path, manifests: list[pathlib.Path], lib: pathlib.Path) -> dict[str, Any]:
    """Each case of `manifests` compiled natively by `compiler`, linked, run
    and checked. A case under the repository gets the fixture harness's
    locked set; any other is compiled on its own."""
    shim = shim_objects(out)
    results = []
    failed = []
    for manifest in manifests:
        path = manifest.resolve()
        for case in json.loads(path.read_text(encoding="utf-8"))["cases"]:
            file = (path.parent / case["file"]).resolve()
            work = out / "cases" / f"{path.parent.name}-{case['id']}"
            work.mkdir(parents=True, exist_ok=True)
            try:
                root = file.relative_to(ROOT).as_posix()
                object_file = compile_case(compiler, root, work)
            except ValueError:
                package = work / "package"
                if package.exists():
                    shutil.rmtree(package)
                package.mkdir(parents=True)
                shutil.copyfile(file, package / "main.argx")
                (package / "argorix.build").write_text(
                    "argorix-build 1\nroot main.argx\nobject case.o\nmanifest case.json\ndiagnostics case.txt\n",
                    encoding="utf-8",
                )
                build_dir = work / "build"
                result = run_compiler(compiler, package, build_dir)
                if result != "0":
                    failed.append(f"{case['id']}: the compiler returned {result}: {(build_dir / 'case.txt').read_text(errors='replace')}")
                    continue
                object_file = build_dir / "case.o"
            link([object_file], work / "case", shim, lib)
            failures = execute_case(path, case, work / "case", work)
            results.append({"cases": relative(path), "id": case["id"], "object_sha256": sha256(object_file), "passed": not failures})
            if failures:
                failed.append(f"{relative(path)} {case['id']}: {'; '.join(failures)}")
    if failed:
        raise BootstrapError("cases failed natively:\n" + "\n".join(failed))
    return {"cases": len(results), "passed": len(results), "results": results}


def c_compilers() -> dict[str, str]:
    """Every C compiler on the path: the usual names, and anything named like
    gcc or clang."""
    found = {name: path for name in C_COMPILER_NAMES if (path := shutil.which(name))}
    for directory in os.environ.get("PATH", "").split(os.pathsep):
        if not directory or not pathlib.Path(directory).is_dir():
            continue
        for entry in pathlib.Path(directory).iterdir():
            if ("gcc" in entry.name or "clang" in entry.name) and os.access(entry, os.X_OK) and entry.is_file():
                found.setdefault(entry.name, str(entry))
    return found


def bootstrap(out: pathlib.Path, requested_lib: pathlib.Path | None, arguments: argparse.Namespace) -> dict[str, Any]:
    record_path = out / "seed.json"
    if not record_path.is_file():
        raise BootstrapError(f"{relative(out)} has no seed.json: run `bootstrap/native.py seed` first")
    record = json.loads(record_path.read_text(encoding="utf-8"))
    seed_outputs = {name: (out / "seed" / "build" / name).read_bytes() for name in OUTPUTS}
    for name, data in seed_outputs.items():
        if sha256_bytes(data) != record["seed"][name]["sha256"]:
            raise BootstrapError(f"the seed's {name} is not the file the seed step recorded")
    shim = shim_objects(out)
    for path, expected in zip(shim, record["shim"]["objects"]):
        if sha256(path) != expected["sha256"]:
            raise BootstrapError(f"{relative(path)} is not the shim object the seed step recorded")
    if sha256(out / "stage1.c") != record["stage1_c"]["sha256"]:
        raise BootstrapError("stage1.c is not the file stage0 recorded")
    compilers = c_compilers()
    if arguments.require_no_c_compiler and compilers:
        raise BootstrapError(f"this host is meant to have no C compiler, but has {compilers}")
    rust_tools = {tool: shutil.which(tool) for tool in RUST_TOOLS}
    if arguments.require_rust_free_host and any(rust_tools.values()):
        raise BootstrapError(f"this host is meant to have no Rust tools, but has {rust_tools}")
    interfaces = network_interfaces()
    if arguments.require_no_network and [name for name in interfaces if name != "lo"]:
        raise BootstrapError(f"this host is meant to have no network, but has {interfaces}")
    lib = libdir(requested_lib)

    chain = generations(out, shim, lib, seed_outputs)
    native2 = out / "native2" / "argorixc"
    native3 = out / "native3" / "argorixc"
    placement = paths_and_order(out, native3, seed_outputs)
    tested = {"skipped": True} if arguments.skip_suite else suite(out, native2, native3, shim, lib)
    edits = no_seed(out, native3, shim, lib, seed_outputs["compiler.o"])
    return {
        "schema_version": 1,
        "task": "ESP-016",
        "stage": "native",
        "source": record["source"],
        "stage0": record["stage0"],
        "stage1_c": record["stage1_c"],
        "stage1": record["stage1"],
        "seed": record["seed"],
        "host": {
            "system": platform.system(),
            "machine": platform.machine(),
            "os": os_release(),
            "python": platform.python_version(),
            "c_compilers": compilers,
            "no_c_compiler_required": arguments.require_no_c_compiler,
            "rust_tools": rust_tools,
            "rust_free_required": arguments.require_rust_free_host,
            "network_interfaces": interfaces,
            "no_network_required": arguments.require_no_network,
        },
        "linker": {
            "version": first_line(["ld", "--version"]),
            "dynamic_linker": DYNAMIC_LINKER,
            "c_library": first_line(["ldd", "--version"]),
            "libdir": str(lib),
            "files": [artifact(lib / name) for name in [*START_FILES, *END_FILES, "libc.so"] if (lib / name).is_file()],
        },
        "shim": record["shim"],
        "environment": environment() | {"PATH": "(inherited)"},
        "build_file": record["build_file"],
        "generations": chain,
        "paths_and_order": placement,
        "suite": tested,
        "no_prebuilt_seed": edits,
        "limits": [
            "Equal generations show that the compiler reproduces itself from its sources. They do not "
            "show that the seed, which stage1 wrote and stage0 before it, carries no defect that "
            "reproduces itself as well; diverse double compilation is ESP-023 and MAT-023.",
            "The linker, the C library and the shim objects are trusted and recorded, not verified. "
            "The shim is the runtime of the C backend, compiled once by the C compiler shim.json names.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    seed_parser = commands.add_parser("seed", help="build the shim, stage1 and the seed object (needs a C compiler)")
    seed_parser.add_argument("--from", dest="source", type=pathlib.Path, default=ROOT / "target" / "stage1")
    seed_parser.add_argument("--cc", default="cc")
    seed_parser.add_argument("--out", type=pathlib.Path, default=ROOT / "target" / "native")
    boot = commands.add_parser("bootstrap", help="native generations, the suite and the edits (needs ld)")
    boot.add_argument("--out", type=pathlib.Path, default=ROOT / "target" / "native")
    boot.add_argument("--libdir", type=pathlib.Path)
    boot.add_argument("--require-no-c-compiler", action="store_true")
    boot.add_argument("--require-rust-free-host", action="store_true")
    boot.add_argument("--require-no-network", action="store_true")
    boot.add_argument("--skip-suite", action="store_true", help="skip the fixture suites (for a quick run)")
    run_cases = commands.add_parser("cases", help="compile, link, run and check the cases of cases.json files natively")
    run_cases.add_argument("--out", type=pathlib.Path, default=ROOT / "target" / "native")
    run_cases.add_argument("--compiler", type=pathlib.Path, help="the compiler to use (default: stage1 of --out)")
    run_cases.add_argument("--libdir", type=pathlib.Path)
    run_cases.add_argument("manifests", type=pathlib.Path, nargs="+")
    arguments = parser.parse_args()
    out = arguments.out if arguments.out.is_absolute() else pathlib.Path.cwd() / arguments.out
    try:
        if arguments.command == "seed":
            source = arguments.source if arguments.source.is_absolute() else pathlib.Path.cwd() / arguments.source
            seed(arguments.cc, source, out)
            return 0
        if arguments.command == "cases":
            compiler = arguments.compiler or out / "stage1" / "argorixc"
            summary = cases(out, compiler.resolve(), arguments.manifests, libdir(arguments.libdir))
            print(f"{summary['passed']} cases passed natively")
            return 0
        report = bootstrap(out, arguments.libdir, arguments)
        path = out / "native-report.json"
        path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    except BootstrapError as error:
        print(f"native bootstrap failed: {error}", file=sys.stderr)
        return 1
    stage = report["generations"]["stages"][-1]
    suite_text = "suite skipped" if arguments.skip_suite else f"{report['suite']['passed']} suite cases passed natively"
    print(
        f"native1, native2 and native3 are identical: object {stage['writes']['compiler.o']['sha256'][:16]}, "
        f"executable {stage['executable']['sha256'][:16]}; {suite_text}; both source edits propagate. "
        f"Report: {relative(path)}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
