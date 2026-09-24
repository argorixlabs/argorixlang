# Native backend: Linux x86-64 (ESP-016)

Status: ESP-016. `compiler/native.argx` compiles a linked and checked Core
package to x86-64 machine code in an ELF64 relocatable object. The system
linker combines the object with the runtime shim and the C library into an
executable. No C is written and no C compiler runs: the C compiler's one job
is to build the shim once, as a declared dependency (below).

The backend is written in Argorix, in three modules:

| Module | Role |
| --- | --- |
| `compiler/x86.argx` | The encoder: the instructions the backend uses, as the Intel SDM encodes them for 64-bit mode |
| `compiler/elf.argx` | The object writer: an ELF64 relocatable object for x86-64 Linux |
| `compiler/native.argx` | The code generator, and the object's assembly |

## One program, two backends

The native backend runs the program the C backend (`spec/core/c-backend.md`)
writes as C, step for step.

- **Shared checks.** It starts from `compiler.c_emit.prepare`: the same
  tables of types, functions, records and constants, and the same checks on
  the whole program. It runs the same checks on `let` annotations and on
  types that hold themselves by value, and it walks each body as the C
  backend does. So it refuses exactly the programs the C backend refuses,
  with the same reason. The driver writes the reason as
  `NativeBackendUnsupported: <reason>`.
- **Same order of effects.** What the C backend writes as a C statement,
  this backend writes as machine code at the same point: temporaries,
  checked-arithmetic helpers, bounds and handle checks, ownership flags and
  drops, step and depth budgets, and the entry point.
- **Values.** The C backend binds almost every value it computes to a
  temporary. It leaves three kinds of value unevaluated:
  - a literal;
  - a local or temporary, read by name;
  - a place: a field, an element behind its bounds check, or an arena slot
    behind its handle.

  The native backend keeps each of these as a node of a small tree. It
  evaluates the node where the C statement that uses it runs, so every read,
  check and trap happens where it happens in the C. Where C leaves an
  order unspecified (the operands of one helper call), this backend
  evaluates left to right, in source order.

`crates/argorixc/tests/native.rs` checks this over the package corpus of
`link_differential.rs`, with stage1 running both backends:

- every package gets the same status;
- a refused package gets the same reason;
- a package that does not check gets the same diagnostics;
- every program both backends compile has the same exit status, standard
  output and standard error.

## Object format

One object per program, with these sections in order:

- the null section;
- `.text`: the code, then the read-only data at a 16-byte boundary;
- `.rela.text`;
- `.symtab`;
- `.strtab`;
- `.shstrtab`;
- an empty `.note.GNU-stack`, which asks for a stack that is not executable.

Every reference inside the object is resolved before it is written:

- calls between functions and to drop functions;
- RIP-relative addresses of string data and trap codes.

What is left for the linker is a call to each runtime function: an
`R_X86_64_PLT32` relocation against an undefined global symbol, with
addend -4.

The symbol table holds, in order:

- the null symbol;
- `.text`'s section symbol;
- `main`, a global function, the object's one definition;
- the runtime functions it calls, undefined, in the order of first use.

The object depends on nothing but its input: the same package and limits
give the same bytes, on any host.

## Link line

```text
ld -o <program> -dynamic-linker /lib64/ld-linux-x86-64.so.2 \
   <libdir>/crt1.o <libdir>/crti.o <program>.o \
   argorix_core_runtime.o argorix_native_shim.o argorix_native_host.o \
   -L<libdir> -lc <libdir>/crtn.o
```

`<libdir>` is the C library's directory, `/usr/lib/x86_64-linux-gnu` on
Debian and Ubuntu. The executable is not position-independent, and it
loads the C library dynamically. `bootstrap/native/toolchain.json` lists
every input of the link. `bootstrap/native.py` records the versions and
digests of the host's copies.

## Calling convention and frames

Functions of the program use their own convention.

- **At a call:**
  - RDI holds the budget's address;
  - RDX holds the address the result is written to (unused for `unit`);
  - the arguments are in a block at the caller's stack pointer: each at its
    alignment, in parameter order, the block rounded up to 8 bytes.
- **In the callee:**
  - `push rbp; mov rbp, rsp; sub rsp, <frame>`;
  - the budget's address is stored at `[rbp - 8]`, the result's at
    `[rbp - 16]`;
  - parameter *i* is read and written in place, at
    `[rbp + 16 + offset(i)]`.
- **Frame:**
  - locals and temporaries take slots below `[rbp - 16]`, at their
    alignment, as the C declares them;
  - a slot is reused once the C block that declares it closes;
  - under the slots is the largest argument block the function passes;
  - the frame is a multiple of 16 bytes, so the stack is aligned at every
    call.
- **Returns:** a function writes its result through the saved address, then
  `mov rsp, rbp; pop rbp; ret`.
- **Registers:** only RAX, RCX, RDX, RSI, RDI and R8, which the System V ABI
  lets a callee change, and RBP. Runtime functions are called as that ABI
  says. Arguments are passed zero- or sign-extended to 64 bits, and a
  result narrower than 64 bits is stored at its own width.

Drop functions take the value's address in RDI.

`main(argc, argv)` does the following:

1. It switches to a stack sized by the backend:
   - the largest frame of the program, plus the return address and saved
     frame pointer, times the depth limit, plus a mebibyte for the runtime;
   - `argorix_rt_stack` maps it lazily, with a guard page below it.

   So running out of call depth is the typed `CALL_DEPTH_LIMIT` trap, never
   a stack overflow. The compiler's own largest frame is 44 KiB, so at the
   default depth it reserves 2 GiB of address space, committed only as it
   is used.
2. It sets the budget.
3. When the entry takes capabilities, it starts the host and passes the
   capabilities to the entry.
4. It calls the entry.
5. It prints the result as the C backend's `main` does.

## Layout

Types are laid out as a C compiler lays out the C backend's definitions on
x86-64 (System V). The runtime sees buffer elements and arena values by
their size, and the byte and slot limits count them, so the sizes must be C's.

| Type | Size | Alignment |
| --- | --- | --- |
| `unit` | 0 | 1 |
| `bool`, `u8`, `i8` | 1 | 1 |
| `u16`, `i16` | 2 | 2 |
| `u32`, `i32`, capabilities | 4 | 4 |
| `u64`, `i64` | 8 | 8 |
| `bytes`, `string`, `Slice<T>` | 16: data pointer, length | 8 |
| `Buffer<T>` | 40 (`argorix_buffer`) | 8 |
| `Arena<T>` | 8 (`argorix_arena`) | 8 |
| `Handle<T>` | 48 (`argorix_handle`) | 8 |
| `Array<T, N>` | N (at least 1) elements | T's |

- **A struct:** its fields in name order, each at its alignment. The size is
  rounded up to the largest alignment.
- **An enum:** a 4-byte tag, the variant's index in name order, then a union
  at the largest alignment of its variants. Each variant is a struct of its
  fields in name order, or a single byte for a variant without fields.

## Operations

| Operation | How it is done |
| --- | --- |
| `+ - * / %`, shifts, negation | The C1 runtime's checked helpers `argorix_<type>_<op>`: the same checks and the same traps as the C |
| Comparisons | Inline: `cmp` and `setcc`, signed or unsigned by the operands' type |
| `& \| ^`, `!` | Inline |
| `wrapping_add` | Inline, in 64 bits, stored at the type's width |
| Elements | `argorix_bounds` |
| `slice` | `argorix_range` |
| Handles | `argorix_rt_handle_get` |
| Step, depth and traps | `argorix_step`, `argorix_enter`, `argorix_leave`, `argorix_trap` |

No optimization is attempted. Every value lives in memory, and every
checked operation is a call.

## Limits

The C backend takes its limits as C preprocessor flags. A native object has
them built in, from the build file (`spec/core/stage1.md`):

| Key | Limit | Default |
| --- | --- | --- |
| `steps` | Step budget | 1000000 |
| `depth` | Call depth | 50000 |
| `buffer-bytes` | Bytes per buffer, and per host read | 1048576 |
| `arena-bytes` | Bytes per arena | 1048576 |
| `arena-slots` | Slots per arena | 1024 |

The build manifest records them. The compiler's own native build uses
`steps 400000000000` and `buffer-bytes 268435456`, the values the C
profile's flags give it.

## The runtime shim

The shim is three C files, compiled once to objects with
`-std=c11 -O2 -Wall -Wextra -Werror -c`:

- `bootstrap/c/argorix_core_runtime.c`: the C1 runtime, unchanged;
- `bootstrap/native/argorix_native_shim.c`;
- `bootstrap/native/argorix_native_host.c`.

The C compiler that builds them is a transitional dependency (stage N1 of
`bootstrap/independence-policy.json`). Nothing after that step needs one.
`bootstrap/native.py seed` builds the shim and records its sources, flags,
compiler and object digests in `shim.json`.

**Symbols native code calls:**

| Symbol | ABI | Memory | Traps |
| --- | --- | --- | --- |
| `argorix_step`, `argorix_enter`, `argorix_leave` (`argorix_budget *`) | runtime | the caller's budget | `STEP_LIMIT`, `CALL_DEPTH_LIMIT` |
| `argorix_trap(const char *)` | runtime | a static string | exits with 70 |
| `argorix_<t>_<op>` for `u8`…`i64` and `add sub mul div rem shl shr`; `argorix_i<n>_neg` | runtime | none | `INTEGER_OVERFLOW`, `DIVISION_BY_ZERO`, `SHIFT_OUT_OF_RANGE` |
| `argorix_bounds(index, length)`, `argorix_range(start, end, length)` | runtime | none | `INDEX_OUT_OF_BOUNDS` |
| `argorix_buffer_push(argorix_buffer *, const void *)`, `argorix_buffer_drop(argorix_buffer *)` | runtime | the buffer owns its storage | `RESOURCE_LIMIT`, `INTEGER_OVERFLOW`, `OUT_OF_MEMORY`, `INVALID_MEMORY` |
| `argorix_arena_release(argorix_arena *)` | runtime | frees the arena's slots | `ARENA_RELEASED` |
| `argorix_rt_buffer_new(out, element size, byte limit)` | shim | writes a buffer to `out` | as `argorix_buffer_new` |
| `argorix_rt_arena_new(out, element size, type id, byte limit, slot limit)` | shim | writes an arena to `out` | as `argorix_arena_new` |
| `argorix_rt_arena_alloc(out, arena, value)` | shim | copies the value into the arena and writes a handle to `out` | `RESOURCE_LIMIT`, `ARENA_RELEASED`, `INVALID_HANDLE`, `OUT_OF_MEMORY` |
| `argorix_rt_handle_get(handle, type id, size, write)` | shim | returns a pointer into the arena | `INVALID_HANDLE`, `ARENA_RELEASED`, `USE_AFTER_FREE`, `TYPE_MISMATCH`, `PERMISSION_DENIED`, `INDEX_OUT_OF_BOUNDS` |
| `argorix_rt_decode_utf8(out, data, length)` | shim | writes a view of the same bytes to `out` | `UTF8_INVALID` |
| `argorix_rt_print_u64`, `argorix_rt_print_i64`, `argorix_rt_print_bool` | shim | none | none |
| `argorix_rt_stack(size)` | shim | maps the program's stack | `OUT_OF_MEMORY` |
| `argorix_rt_host_start`, `argorix_rt_host_capability`, `argorix_rt_package_status`, `argorix_rt_package_read`, `argorix_rt_build_write` | host | a read buffer is owned by the caller | as the C1 host shim: `PERMISSION_DENIED`, `HOST_UNAVAILABLE`, statuses |

The shim functions add no behavior of their own. A C1 function that takes
or returns a structure by value gets a wrapper that passes it by address,
so native code only needs integer registers. The consumers are every
native program, and the compiler itself. The tests are the fixture suites
run natively, the differential, and the encoder and link tests in
`crates/argorixc/tests/native.rs`.

**Retirement.** The shim is the C backend's runtime, kept so both backends
behave the same while C stays in the tree. It holds no phase of the
toolchain: no lexer, parser, checker, lowering or resolver. Its planned
retirement:

- checked arithmetic, bounds and budgets can be emitted inline;
- buffers, arenas and handles can be written in Argorix over a narrower
  interface to the C library (allocation, output, exit, files).

That is follow-up work after ESP-016 (ESP-017 or later). Until then the
shim, the linker and the C library are recorded, not verified.

## Driver and bootstrap

- **`object <path>`** in the build file, in place of `c`, asks stage1 for a
  native object (`spec/core/stage1.md`).
- **The manifest** names the backend `argorix-core-native 0.1`, with the
  target, the object format, the runtime ABI, the limits, and the object's
  size and SHA-256.
- **`bootstrap/native.py`:**
  - `seed` needs a C compiler. It builds the shim and stage1, and stage1
    writes the seed object.
  - `bootstrap` needs the linker, the C library and Python.
    - Three native generations of the compiler must write the seed's
      object, byte for byte, and be byte-identical executables.
    - Relocating the sources or reordering the modules changes nothing.
    - The fixture suites pass natively.
    - An edited diagnostic and an edited backend function propagate
      through new generations to a fixed point.
  - In CI (`core-c.yml`, `native`), `bootstrap` runs in a container with
    no C compiler, no Rust and no network.

## Limits of this backend

- **One target:** Linux x86-64. A second target, Windows, is ESP-017.
- **No debug information and no local symbols:** a debugger sees one
  function, `main`, and addresses.
- **The trusted base:** the linker, the C library and the shim objects are
  trusted and recorded. As in ESP-015, equal generations show that the
  compiler reproduces itself, not that no defect reproduces with it; that
  is ESP-023 and MAT-023.
