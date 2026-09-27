# Agent-language package dump

Status: ESP-018.C. Two implementations resolve a package of agent-language
modules, merge it and check it as one program:

- stage0's `crates/argorix_module` (the manifest, `resolve_package`,
  `merge_package`), with `crates/argorix_semantics/src/checker.rs`;
- the Argorix `compiler/agent_package.argx`, with
  `compiler/agent_check.argx`. It reads the package through the
  compiler-host boundary.

`crates/argorixc/tests/agent_package_differential.rs` compares them over
every directory of the repository with an `argorix.toml` and the adversarial
packages in `tests/selfhost/agent/package_samples/`.

## Dump

`argorixc agent-package <directory or manifest>` prints the dump, and so
does `compiler.agent_package.dump`.

- A package that does not resolve gives one line: `error: ` and the
  resolver's message, which never holds an absolute path.
- Otherwise:
  - `entry <name>`;
  - `module <name> <path>` for each module, by name;
  - `import <from> <to>` for each import edge, sorted, without repeats;
  - then the merged program's check: `ok`, or one `line:column: message`
    line per diagnostic, as in `check.md`. The line and column are in the
    file that declares the construct.

## Resolution

In stage0's order, stopping at the first error:

1. **The manifest** `argorix.toml`: `[section]` headers, `key = "value"`
   pairs, `#` comments and blank lines, each line trimmed of Unicode white
   space. The keys are `package.name`, `package.version` and `entry.main`;
   any other key, a line without `=` or an unquoted value is an error that
   names its line. Then a missing `entry.main`, an empty one, a missing name
   and a missing version, in that order.
2. **The entry path** is split on `/` and `\`, with `.` and empty components
   dropped. An absolute path, `..` or a component ending in `:` (a drive) is
   outside the project root.
3. **The entry module** must parse, and declare the name its path gives
   (`src/agents/x.argx` is `agents.x`), or that name after `app.`.
4. **Each import**, depth first in source order:
   - an invalid module name is an error;
   - module `a.b` lives at `src/a/b.argx`;
   - a module already resolved at another path is a duplicate;
   - one still being visited closes a cycle, printed from its first visit;
   - a missing file is an unknown import;
   - the file must parse and declare the imported name.

A parse error prints the module's messages without positions, joined by
`; `.

## Merge

The merged program holds the declarations of the entry module first, then
of each other module by name, list by list; imports are dropped. The
checker runs on it with default options.
