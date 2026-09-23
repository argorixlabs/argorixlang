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
