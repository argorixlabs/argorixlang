/* The runtime interface of the native backend (ESP-016,
 * spec/core/native-x86-64.md).
 *
 * Native code calls the C1 runtime (bootstrap/c/argorix_core_runtime.c)
 * directly wherever a function takes and returns only scalars and pointers.
 * The functions below cover the rest: each passes a structure by pointer
 * instead of by value, so native code only ever needs integer registers.
 * They add no behaviour of their own.
 *
 * This file, the runtime and argorix_native_host.c are the bounded native
 * shim of the independence policy (bootstrap/independence-policy.json): a
 * C compiler builds them once, as a declared dependency, and no Argorix
 * program is compiled through C.
 */
#define _DEFAULT_SOURCE 1
#include "argorix_core_runtime.h"

#include <inttypes.h>
#include <stdio.h>
#include <sys/mman.h>

/* The stack a native program runs on: `size` bytes, sized by the backend for
 * the deepest call the depth budget allows, so running out of depth is the
 * typed trap and never a stack overflow. Pages are committed as they are
 * touched; a guard page below the stack faults instead of running into
 * another mapping. Returns the stack's top, 16-byte aligned. */
void *argorix_rt_stack(uint64_t size) {
    const uint64_t guard = 65536U;
    uint64_t rounded = (size + 65535U) / 65536U * 65536U;
    uint8_t *base = mmap(
        NULL, (size_t)(rounded + guard), PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0
    );
    if (base == MAP_FAILED) {
        argorix_trap("OUT_OF_MEMORY");
    }
    if (mprotect(base, (size_t)guard, PROT_NONE) != 0) {
        argorix_trap("OUT_OF_MEMORY");
    }
    return base + guard + rounded;
}

/* The entry's result, as the C backend's main prints it. */
void argorix_rt_print_u64(uint64_t value) {
    (void)printf("ARGORIX_RESULT:%" PRIu64 "\n", value);
}

void argorix_rt_print_i64(int64_t value) {
    (void)printf("ARGORIX_RESULT:%" PRId64 "\n", value);
}

void argorix_rt_print_bool(uint64_t value) {
    (void)printf("ARGORIX_RESULT:%s\n", value != 0U ? "true" : "false");
}

void argorix_rt_buffer_new(argorix_buffer *out, uint64_t element_size, uint64_t byte_limit) {
    *out = argorix_buffer_new(element_size, byte_limit);
}

void argorix_rt_arena_new(
    argorix_arena *out, uint64_t element_size, uint32_t type_id, uint64_t byte_limit, uint32_t slot_limit
) {
    *out = argorix_arena_new(element_size, type_id, byte_limit, slot_limit);
}

void argorix_rt_arena_alloc(argorix_handle *out, argorix_arena *arena, const void *value) {
    *out = argorix_arena_alloc(arena, value);
}

void *argorix_rt_handle_get(
    const argorix_handle *handle, uint32_t type_id, uint64_t element_size, uint32_t require_write
) {
    return argorix_handle_get(*handle, type_id, element_size, require_write != 0U);
}

void argorix_rt_decode_utf8(argorix_string *out, const uint8_t *data, uint64_t length) {
    argorix_bytes value = {data, length};
    *out = argorix_decode_utf8(value);
}
