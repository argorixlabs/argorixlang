#ifndef ARGORIX_CORE_RUNTIME_H
#define ARGORIX_CORE_RUNTIME_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define ARGORIX_C_ABI_VERSION "C1"
#define ARGORIX_TRAP_EXIT 70

typedef struct argorix_budget {
    uint64_t remaining_steps;
} argorix_budget;

typedef struct argorix_bytes {
    const uint8_t *data;
    uint64_t length;
} argorix_bytes;

typedef struct argorix_string {
    const uint8_t *data;
    uint64_t length;
} argorix_string;

typedef struct argorix_buffer {
    uint8_t *data;
    uint64_t length;
    uint64_t capacity;
    uint64_t element_size;
    uint64_t byte_limit;
} argorix_buffer;

enum {
    ARGORIX_PERMISSION_READ = 1,
    ARGORIX_PERMISSION_READ_WRITE = 2
};

typedef struct argorix_handle {
    uint64_t arena_id;
    uint64_t arena_epoch;
    uint32_t slot;
    uint32_t allocation_generation;
    uint64_t offset;
    uint64_t length;
    uint32_t type_id;
    uint8_t permission;
    uint8_t reserved[3];
} argorix_handle;

_Static_assert(sizeof(argorix_handle) == 48U, "CoreHandleV1 must remain 48 bytes");

typedef struct argorix_slot {
    uint32_t generation;
    uint32_t type_id;
    uint64_t length;
    bool active;
} argorix_slot;

typedef struct argorix_arena_view {
    uint64_t arena_id;
    uint64_t epoch;
    const argorix_slot *slots;
    uint64_t slot_count;
    bool active;
} argorix_arena_view;

typedef struct argorix_arena_state argorix_arena_state;

typedef struct argorix_arena {
    argorix_arena_state *state;
} argorix_arena;

_Noreturn void argorix_trap(const char *code);
void argorix_step(argorix_budget *budget);
size_t argorix_bounds(uint64_t index, uint64_t length);
argorix_string argorix_decode_utf8(argorix_bytes value);
argorix_buffer argorix_buffer_new(uint64_t element_size, uint64_t byte_limit);
void argorix_buffer_push(argorix_buffer *buffer, const void *value);
void argorix_buffer_drop(argorix_buffer *buffer);
argorix_arena argorix_arena_new(
    uint64_t element_size,
    uint32_t type_id,
    uint64_t byte_limit,
    uint32_t slot_limit
);
argorix_handle argorix_arena_alloc(argorix_arena *arena, const void *value);
void *argorix_handle_get(
    argorix_handle handle,
    uint32_t expected_type_id,
    uint64_t expected_element_size,
    bool require_write
);
void argorix_arena_release(argorix_arena *arena);
void argorix_validate_handle(
    const argorix_arena_view *arena,
    argorix_handle handle,
    uint32_t expected_type_id,
    bool require_write
);

uint8_t argorix_u8_add(uint8_t left, uint8_t right);
uint8_t argorix_u8_sub(uint8_t left, uint8_t right);
uint8_t argorix_u8_mul(uint8_t left, uint8_t right);
uint8_t argorix_u8_div(uint8_t left, uint8_t right);
uint8_t argorix_u8_rem(uint8_t left, uint8_t right);

uint16_t argorix_u16_add(uint16_t left, uint16_t right);
uint16_t argorix_u16_sub(uint16_t left, uint16_t right);
uint16_t argorix_u16_mul(uint16_t left, uint16_t right);
uint16_t argorix_u16_div(uint16_t left, uint16_t right);
uint16_t argorix_u16_rem(uint16_t left, uint16_t right);

uint32_t argorix_u32_add(uint32_t left, uint32_t right);
uint32_t argorix_u32_sub(uint32_t left, uint32_t right);
uint32_t argorix_u32_mul(uint32_t left, uint32_t right);
uint32_t argorix_u32_div(uint32_t left, uint32_t right);
uint32_t argorix_u32_rem(uint32_t left, uint32_t right);

uint64_t argorix_u64_add(uint64_t left, uint64_t right);
uint64_t argorix_u64_sub(uint64_t left, uint64_t right);
uint64_t argorix_u64_mul(uint64_t left, uint64_t right);
uint64_t argorix_u64_div(uint64_t left, uint64_t right);
uint64_t argorix_u64_rem(uint64_t left, uint64_t right);

int32_t argorix_i32_add(int32_t left, int32_t right);
int32_t argorix_i32_sub(int32_t left, int32_t right);
int32_t argorix_i32_mul(int32_t left, int32_t right);
int32_t argorix_i32_div(int32_t left, int32_t right);
int32_t argorix_i32_rem(int32_t left, int32_t right);

int8_t argorix_i8_add(int8_t left, int8_t right);
int8_t argorix_i8_sub(int8_t left, int8_t right);
int8_t argorix_i8_mul(int8_t left, int8_t right);
int8_t argorix_i8_div(int8_t left, int8_t right);
int8_t argorix_i8_rem(int8_t left, int8_t right);

int16_t argorix_i16_add(int16_t left, int16_t right);
int16_t argorix_i16_sub(int16_t left, int16_t right);
int16_t argorix_i16_mul(int16_t left, int16_t right);
int16_t argorix_i16_div(int16_t left, int16_t right);
int16_t argorix_i16_rem(int16_t left, int16_t right);

int64_t argorix_i64_add(int64_t left, int64_t right);
int64_t argorix_i64_sub(int64_t left, int64_t right);
int64_t argorix_i64_mul(int64_t left, int64_t right);
int64_t argorix_i64_div(int64_t left, int64_t right);
int64_t argorix_i64_rem(int64_t left, int64_t right);

#endif
