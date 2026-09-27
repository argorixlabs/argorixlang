# Argorix Core minimal standard library S1

Status: ESP-009 implementation contract. S1 is the bootstrap library needed by
the lexer, parser, resolver, and emitter; it is not the future general-purpose
application standard library.

## Design boundary

Collection, path, and serialization algorithms are Argorix Core `.argx`
sources under `stdlib/`. The C1 runtime may provide memory allocation, checked
copying, handle validation, and typed host dispatch, but it must not hide a
Rust collection, serializer, filesystem client, or compiler implementation.
Every runtime primitive has an explicit byte/step limit and deterministic trap
or result. Source text never becomes a shell command, host pointer, ambient
path, environment lookup, or capability.

Core 0.1 has built-in container generics but no user-defined generics. S1
therefore uses built-in `Buffer<T>`, `Slice<T>`, `Arena<T>`, and `Handle<T>` and
publishes specialized result/map/serialization types where necessary. This is
an intentional bootstrap constraint, not implicit dynamic typing.

## Primitive containers

### `Buffer<T>`

- `Buffer::new()` creates an empty buffer; `T` is inferred from its annotated
  destination.
- `buffer.length() -> u64` returns initialized element count.
- `buffer.push(value: T) -> unit` appends atomically. Capacity grows by powers
  of two from four elements. Checked size overflow traps `INTEGER_OVERFLOW`;
  exceeding the configured byte ceiling traps `RESOURCE_LIMIT`; allocator
  failure traps `OUT_OF_MEMORY`. On failure, length and existing values remain
  unchanged.
- `buffer[index: u64] -> T` validates initialized length and traps
  `INDEX_OUT_OF_BOUNDS` before access.
- Storage preserves insertion order. No public raw capacity or pointer
  exists.
- `buffer[index] = value` replaces an element in place, after the same
  bounds check.

### Ownership of resource values

A *resource* is a `Buffer<T>`, or a struct, enum or fixed array that holds one
at any depth. Every other type is plain data and is copied. A resource has
exactly one owner at a time, and is released exactly once, when its owner's
scope ends:

- **Moves.** A resource moves when it is bound by `let`, passed as an
  argument (parameters are by value), returned, used as a `break` value,
  stored in a field or an element of a new aggregate, pushed into a buffer,
  assigned, matched on, or evaluated as a statement and discarded. After a
  move the source binding cannot be used; assigning it a new value makes it
  an owner again.
- **Places.** A local, a field of a place or an element of a place can be
  inspected without moving: `h.values.length()`, `rows[i][j]`,
  `h.values.push(x)`. Moving a resource *out of* a field or an element is an
  error (`MoveOutOfPlace`): the value that holds it still owns it.
- **Flow.** A binding moved on any path that reaches a use is an error
  (`UseAfterMove`); both branches of an `if` or `match`, and the right-hand
  side of `&&`/`||`, count as paths. A binding declared before a loop cannot
  be moved on a path back to the loop head (`MoveInLoop`); moving it on a path
  that leaves the loop is fine.
- **Temporaries.** A resource that is only inspected must be a place; a
  temporary such as `make().length()` is an error (`ResourceTemporary`),
  because nothing would own it afterwards.
- **Matching.** Matching consumes the scrutinee. The arm that runs owns what
  its pattern binds; parts it does not bind are released on entry.
- **Arenas.** A resource cannot be stored in an `Arena<T>` slot
  (`ResourceInArena`): a slot is freed without looking inside it.
- **Release.** Each owner is released at the end of its scope, and on every
  `return`, `break` or `continue` that leaves that scope first. Releasing a
  struct releases its resource fields, an enum those of its live variant, a
  buffer or array each resource element before its own storage.

### Views of owned storage

`x.as_slice()` makes a `Slice<T>` view of a `Buffer<T>` or `Array<T, N>`
without moving `x`, and a `Slice<u8>` of the UTF-8 bytes of a `string`, such
as `"name".as_slice()`. The view does not own anything, so it must not outlive
`x`:

- `as_slice()` may only appear directly as an argument of a call,
  `f(x.as_slice())` (`SliceEscapes`), and that call may not also move `x`
  (`SliceAliasesMove`). Inside the callee the view is an ordinary parameter.
- `view.slice(start, end)` narrows a view; `start > end` or `end` past the
  view's length is the trap `INDEX_OUT_OF_BOUNDS`. `stdlib.bytes.range` is
  the checked form that returns a result instead.
- A slice cannot be the element of a container (`SliceEscapes`).
- Core allows a slice as a struct field, an enum payload or a return type,
  such as a parser that keeps its token slice. The transitional C backend
  refuses those programs with `CBackendUnsupported`: its views of buffer and
  array storage carry no generation to check, and a stored view could dangle.

### `Arena<T>` and `Handle<T>`

- `Arena::new()` creates a bounded typed arena under the caller's ownership.
- `arena.alloc(value: T) -> Handle<T>` publishes a handle only after a complete
  allocation and copy.
- `arena.release() -> unit` invalidates every handle by advancing the arena
  epoch. Slot reuse advances generation; stale handles never revive.
- Field/index access through a handle validates arena, epoch, slot, generation,
  type, range, permission, and borrow state according to M1/ABI-1.
- Exhaustion is atomic and deterministic. No handle contains a host pointer.

## Library modules

| Module | Bootstrap API and invariant |
| --- | --- |
| `stdlib.bytes` | bounded copy/append/equality and validated byte slicing |
| `stdlib.text` | UTF-8 validation, byte length, equality, and deterministic escaping; no locale-dependent behavior |
| `stdlib.vector` | Argorix algorithms over `Buffer<T>` used by compiler-specific specialized vector types |
| `stdlib.ordered_map` | specialized sorted-vector maps; unique keys, bytewise key order, duplicate-key result, deterministic iteration |
| `stdlib.result` | specialized tagged `Ok`/`Err` results; no sentinel success values |
| `stdlib.arena` | safe wrappers and traversal patterns over `Arena<T>`/`Handle<T>` |
| `stdlib.path` | normalized relative UTF-8 package paths; rejects empty segments, `.`, `..`, absolute roots, drive/UNC prefixes, NUL, and separator ambiguity |
| `stdlib.json` | bounded canonical JSON subset required by bootstrap manifests/IR; stable field order, escaping, integer limits, duplicate-key rejection |
| `stdlib.compiler_host` | typed `package.read` and `build.write` requests using capabilities supplied by the driver |

Modules may temporarily contain a bootstrap `argorix_main` test entry until
multi-module linking lands. Test entrypoints are not part of the public S1 API.

## Determinism and limits

All ordering is defined over unsigned UTF-8 bytes; host locale and hash seeds
are irrelevant. Maps emit keys in canonical byte order. JSON objects use the
declared schema order or ordered-map order. Repeated execution with gcc and
clang must produce byte-identical output.

The execution profile supplies ceilings for steps, buffer bytes, arena bytes
and slots, input/output bytes, nesting depth, path length, and file count.
Checked arithmetic precedes allocation or mutation. A limit failure cannot
publish a partial element, handle, map entry, output file, or success result.

## Compiler-host file boundary

Only `package.read` and `build.write` are exposed to S1. The driver supplies a
canonical package root, build root, operation-specific capability, byte budget,
and request ID. Paths are normalized in Argorix and revalidated by the host
immediately before dispatch. Reads cannot escape the package root; writes
cannot escape the build root. Symlink/reparse resolution is host-checked.

The agent-runtime profile cannot inherit these capabilities. S1 exposes no
ambient current directory, general filesystem API, network, environment,
arbitrary process spawn, linker selection, FFI, or shell evaluation.

#### Core surface (ESP-009)

- `PackageRead` and `BuildWrite` are built-in capability types. Source cannot
  construct one or redeclare the names (`DuplicateDeclaration`). A capability
  may be a parameter or a local; it cannot be a struct field, an enum payload,
  a container element or a return type (`CapabilityEscapes`), so no data
  structure outlives the lending.
- Only the entry lends them: `argorix_main` takes no parameters, or at most
  one `PackageRead` and one `BuildWrite`. A function that takes a capability
  has the IR effect `package.read` or `build.write`; no other host effect
  exists.
- Operations, whose paths and data are byte views:
  - `package.status(path) -> u64` returns a status code;
  - `package.read(path) -> Buffer<u8>` returns the file, and is meant only
    after a status of 0. A file that changed in between is the trap
    `HOST_UNAVAILABLE`.
  - `build.write(path, data) -> u64` creates or replaces the file and returns
    a status code. Its directory must already exist inside the build root.
- `stdlib.compiler_host` wraps them as `read(package, path) -> BytesResult`
  and `write(build, path, data) -> Checked`. It validates the path with
  `stdlib.path` first, and reports a host refusal as
  `Failure::Host { at: code }`.
- Status codes: 0 ok, 1 invalid path, 2 not found, 3 outside the root, 4 not a
  regular file, 5 over the byte budget, 6 I/O error, 7 unsupported host.

#### Host shim (C1)

The shim is `bootstrap/c/argorix_core_host.h`. It is compiled only into a
program that can hold a capability; no other binary may import a
file-system function, and the harness checks this (`host_only_imports` in
`conformance/core_c/policy.json`).

- The driver passes `--package-root`, `--read-budget`, `--build-root` and
  `--write-budget`. An unknown flag, a flag for a capability the entry does not
  take, a missing root or budget, or a root that is not a directory traps
  `PERMISSION_DENIED` before any Core code runs.
- The roots are canonicalized with `realpath`. Every operation:
  1. checks the path again against the `stdlib.path` rules;
  2. resolves it;
  3. refuses a result that lies outside its root, including through a
     symlink;
  4. writes with `O_NOFOLLOW`;
  5. charges the byte budget.
- On Windows (ESP-017) the same rules hold over the wide Win32 API:
  - arguments are read from the UTF-16 command line and paths are converted
    from UTF-8, so no code page stands in between;
  - roots and files are resolved through an open handle
    (`GetFinalPathNameByHandleW`), which follows symbolic links and
    junctions as `realpath` does, and every name is used in its `\\?\` form,
    so a DOS device name or a trailing dot is not reinterpreted;
  - a file is read through the handle that was resolved and checked;
  - a file written is opened without following a reparse point, and one
    found there is refused as outside the root, as `O_NOFOLLOW` refuses a
    symlink.
- Status 7 is left for a host with no file access.
- On POSIX, the window between resolving a path and opening it is not closed
  against concurrent writers to the roots; on Windows a read has no such
  window, but a write between checking its directory and creating its file
  does. OS isolation of the compiler process (MAT-011) covers those races.

## Required evidence

Closure requires source-level positive fixtures plus adversarial cases for
invalid UTF-8, numeric/size overflow, duplicate keys, stale/released handles,
path traversal and root escape, oversized data, OOM/resource ceilings, and
malformed serialization. Fixtures execute through the verified IR and C1
backend with exact output/trap oracles under gcc and clang. A clean execution
host may contain the declared C library but no Rust runtime dependency.

Passing S1 proves only the minimum executable library substrate. It does not
prove the lexer/parser are written in Argorix, multi-module self-hosting,
stage2/stage3 reproducibility, a native backend, or full Rust independence.
