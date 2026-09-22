#include "argorix_core_runtime.h"

#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ARGORIX_ARENA_REGISTRY_LIMIT 64U

typedef struct argorix_runtime_slot {
    void *data;
    uint32_t generation;
    bool active;
} argorix_runtime_slot;

struct argorix_arena_state {
    uint64_t arena_id;
    uint64_t epoch;
    uint64_t element_size;
    uint64_t byte_limit;
    uint64_t used_bytes;
    uint32_t type_id;
    uint32_t slot_limit;
    uint32_t slot_count;
    bool active;
    argorix_runtime_slot *slots;
};

static argorix_arena_state *argorix_arena_registry[ARGORIX_ARENA_REGISTRY_LIMIT];
static uint64_t argorix_next_arena_id = 1U;

_Noreturn void argorix_trap(const char *code) {
    (void)fprintf(stderr, "ARGORIX_TRAP:%s\n", code);
    exit(ARGORIX_TRAP_EXIT);
}

void argorix_enter(argorix_budget *budget) {
    if (budget == NULL || budget->remaining_depth == 0U) {
        argorix_trap("CALL_DEPTH_LIMIT");
    }
    budget->remaining_depth -= 1U;
}

void argorix_leave(argorix_budget *budget) {
    if (budget != NULL) {
        budget->remaining_depth += 1U;
    }
}

void argorix_step(argorix_budget *budget) {
    if (budget == NULL || budget->remaining_steps == 0U) {
        argorix_trap("STEP_LIMIT");
    }
    budget->remaining_steps -= 1U;
}

size_t argorix_bounds(uint64_t index, uint64_t length) {
    if (index >= length || index > (uint64_t)SIZE_MAX) {
        argorix_trap("INDEX_OUT_OF_BOUNDS");
    }
    return (size_t)index;
}

static bool argorix_continuation(uint8_t value) {
    return value >= 0x80U && value <= 0xBFU;
}

argorix_string argorix_decode_utf8(argorix_bytes value) {
    if (value.length > 0U && value.data == NULL) {
        argorix_trap("INVALID_MEMORY");
    }
    uint64_t index = 0U;
    while (index < value.length) {
        uint8_t first = value.data[argorix_bounds(index, value.length)];
        if (first <= 0x7FU) {
            index += 1U;
            continue;
        }
        if (first >= 0xC2U && first <= 0xDFU && value.length - index >= 2U &&
            argorix_continuation(value.data[index + 1U])) {
            index += 2U;
            continue;
        }
        if (value.length - index >= 3U) {
            uint8_t second = value.data[index + 1U];
            uint8_t third = value.data[index + 2U];
            bool valid_three =
                ((first == 0xE0U && second >= 0xA0U && second <= 0xBFU) ||
                 ((first >= 0xE1U && first <= 0xECU) && argorix_continuation(second)) ||
                 (first == 0xEDU && second >= 0x80U && second <= 0x9FU) ||
                 ((first >= 0xEEU && first <= 0xEFU) && argorix_continuation(second))) &&
                argorix_continuation(third);
            if (valid_three) {
                index += 3U;
                continue;
            }
        }
        if (value.length - index >= 4U) {
            uint8_t second = value.data[index + 1U];
            uint8_t third = value.data[index + 2U];
            uint8_t fourth = value.data[index + 3U];
            bool valid_four =
                ((first == 0xF0U && second >= 0x90U && second <= 0xBFU) ||
                 ((first >= 0xF1U && first <= 0xF3U) && argorix_continuation(second)) ||
                 (first == 0xF4U && second >= 0x80U && second <= 0x8FU)) &&
                argorix_continuation(third) && argorix_continuation(fourth);
            if (valid_four) {
                index += 4U;
                continue;
            }
        }
        argorix_trap("UTF8_INVALID");
    }
    argorix_string result = {value.data, value.length};
    return result;
}

argorix_buffer argorix_buffer_new(uint64_t element_size, uint64_t byte_limit) {
    if (element_size == 0U || element_size > (uint64_t)SIZE_MAX) {
        argorix_trap("INVALID_MEMORY");
    }
    argorix_buffer result = {NULL, 0U, 0U, element_size, byte_limit};
    return result;
}

void argorix_buffer_push(argorix_buffer *buffer, const void *value) {
    if (buffer == NULL || value == NULL || buffer->element_size == 0U ||
        buffer->element_size > (uint64_t)SIZE_MAX) {
        argorix_trap("INVALID_MEMORY");
    }
    if (buffer->length == UINT64_MAX) {
        argorix_trap("INTEGER_OVERFLOW");
    }
    if (buffer->length == buffer->capacity) {
        uint64_t next_capacity = buffer->capacity == 0U ? 4U : buffer->capacity * 2U;
        if (next_capacity < buffer->capacity ||
            next_capacity > UINT64_MAX / buffer->element_size) {
            argorix_trap("INTEGER_OVERFLOW");
        }
        uint64_t next_bytes = next_capacity * buffer->element_size;
        if (next_bytes > buffer->byte_limit || next_bytes > (uint64_t)SIZE_MAX) {
            argorix_trap("RESOURCE_LIMIT");
        }
        void *next = realloc(buffer->data, (size_t)next_bytes);
        if (next == NULL) {
            argorix_trap("OUT_OF_MEMORY");
        }
        buffer->data = next;
        buffer->capacity = next_capacity;
    }
    size_t offset = (size_t)(buffer->length * buffer->element_size);
    (void)memcpy(&buffer->data[offset], value, (size_t)buffer->element_size);
    buffer->length += 1U;
}

void argorix_buffer_drop(argorix_buffer *buffer) {
    if (buffer == NULL) {
        return;
    }
    free(buffer->data);
    buffer->data = NULL;
    buffer->length = 0U;
    buffer->capacity = 0U;
}

static argorix_arena_state *argorix_find_arena(uint64_t arena_id) {
    for (size_t index = 0U; index < ARGORIX_ARENA_REGISTRY_LIMIT; index += 1U) {
        argorix_arena_state *state = argorix_arena_registry[index];
        if (state != NULL && state->arena_id == arena_id) {
            return state;
        }
    }
    return NULL;
}

argorix_arena argorix_arena_new(
    uint64_t element_size,
    uint32_t type_id,
    uint64_t byte_limit,
    uint32_t slot_limit
) {
    uint64_t slot_bytes = (uint64_t)slot_limit * (uint64_t)sizeof(argorix_runtime_slot);
    if (element_size == 0U || element_size > (uint64_t)SIZE_MAX || type_id == 0U ||
        slot_limit == 0U || slot_bytes > (uint64_t)SIZE_MAX) {
        argorix_trap("INVALID_MEMORY");
    }
    size_t registry_index = ARGORIX_ARENA_REGISTRY_LIMIT;
    for (size_t index = 0U; index < ARGORIX_ARENA_REGISTRY_LIMIT; index += 1U) {
        if (argorix_arena_registry[index] == NULL) {
            registry_index = index;
            break;
        }
    }
    if (registry_index == ARGORIX_ARENA_REGISTRY_LIMIT || argorix_next_arena_id == UINT64_MAX) {
        argorix_trap("RESOURCE_LIMIT");
    }
    argorix_arena_state *state = calloc(1U, sizeof(*state));
    argorix_runtime_slot *slots = calloc((size_t)slot_limit, sizeof(*slots));
    if (state == NULL || slots == NULL) {
        free(state);
        free(slots);
        argorix_trap("OUT_OF_MEMORY");
    }
    state->arena_id = argorix_next_arena_id;
    argorix_next_arena_id += 1U;
    state->epoch = 1U;
    state->element_size = element_size;
    state->byte_limit = byte_limit;
    state->type_id = type_id;
    state->slot_limit = slot_limit;
    state->active = true;
    state->slots = slots;
    argorix_arena_registry[registry_index] = state;
    argorix_arena result = {state};
    return result;
}

argorix_handle argorix_arena_alloc(argorix_arena *arena, const void *value) {
    if (arena == NULL || arena->state == NULL || value == NULL) {
        argorix_trap("INVALID_HANDLE");
    }
    argorix_arena_state *state = arena->state;
    if (!state->active) {
        argorix_trap("ARENA_RELEASED");
    }
    if (state->slot_count >= state->slot_limit) {
        argorix_trap("RESOURCE_LIMIT");
    }
    if (state->used_bytes > state->byte_limit ||
        state->element_size > state->byte_limit - state->used_bytes) {
        argorix_trap("RESOURCE_LIMIT");
    }
    void *data = malloc((size_t)state->element_size);
    if (data == NULL) {
        argorix_trap("OUT_OF_MEMORY");
    }
    (void)memcpy(data, value, (size_t)state->element_size);
    uint32_t slot_index = state->slot_count;
    argorix_runtime_slot *slot = &state->slots[slot_index];
    slot->data = data;
    slot->generation = 1U;
    slot->active = true;
    state->slot_count += 1U;
    state->used_bytes += state->element_size;
    argorix_handle handle = {
        state->arena_id, state->epoch, slot_index, slot->generation,
        0U, 1U, state->type_id, ARGORIX_PERMISSION_READ_WRITE, {0U, 0U, 0U}
    };
    return handle;
}

void *argorix_handle_get(
    argorix_handle handle,
    uint32_t expected_type_id,
    uint64_t expected_element_size,
    bool require_write
) {
    argorix_arena_state *state = argorix_find_arena(handle.arena_id);
    if (state == NULL) {
        if (handle.arena_id > 0U && handle.arena_id < argorix_next_arena_id) {
            argorix_trap("ARENA_RELEASED");
        }
        argorix_trap("INVALID_HANDLE");
    }
    if (expected_element_size == 0U || expected_element_size > (uint64_t)SIZE_MAX) {
        argorix_trap("INVALID_HANDLE");
    }
    if (!state->active || handle.arena_epoch != state->epoch) {
        argorix_trap("ARENA_RELEASED");
    }
    if (handle.slot >= state->slot_count) {
        argorix_trap("INVALID_HANDLE");
    }
    argorix_runtime_slot *slot = &state->slots[handle.slot];
    argorix_slot view_slot = {
        slot->generation, state->type_id, 1U, slot->active
    };
    argorix_arena_view view = {
        state->arena_id, state->epoch, &view_slot, 1U, state->active
    };
    argorix_handle local = handle;
    local.slot = 0U;
    argorix_validate_handle(&view, local, expected_type_id, require_write);
    if (state->element_size != expected_element_size) {
        argorix_trap("TYPE_MISMATCH");
    }
    return slot->data;
}

void argorix_arena_release(argorix_arena *arena) {
    if (arena == NULL || arena->state == NULL) {
        argorix_trap("INVALID_HANDLE");
    }
    argorix_arena_state *state = arena->state;
    if (!state->active) {
        argorix_trap("ARENA_RELEASED");
    }
    if (state->epoch == UINT64_MAX) {
        argorix_trap("INTEGER_OVERFLOW");
    }
    for (uint32_t index = 0U; index < state->slot_count; index += 1U) {
        free(state->slots[index].data);
        state->slots[index].data = NULL;
        state->slots[index].active = false;
    }
    state->used_bytes = 0U;
    state->active = false;
    state->epoch += 1U;
    for (size_t index = 0U; index < ARGORIX_ARENA_REGISTRY_LIMIT; index += 1U) {
        if (argorix_arena_registry[index] == state) {
            argorix_arena_registry[index] = NULL;
            break;
        }
    }
    free(state->slots);
    state->slots = NULL;
    free(state);
    arena->state = NULL;
}

void argorix_validate_handle(
    const argorix_arena_view *arena,
    argorix_handle handle,
    uint32_t expected_type_id,
    bool require_write
) {
    if (arena == NULL || handle.arena_id == 0U || handle.type_id == 0U ||
        handle.reserved[0] != 0U || handle.reserved[1] != 0U ||
        handle.reserved[2] != 0U ||
        (handle.permission != ARGORIX_PERMISSION_READ &&
         handle.permission != ARGORIX_PERMISSION_READ_WRITE)) {
        argorix_trap("INVALID_HANDLE");
    }
    if (!arena->active || handle.arena_id != arena->arena_id ||
        handle.arena_epoch != arena->epoch) {
        argorix_trap("ARENA_RELEASED");
    }
    if (handle.slot >= arena->slot_count || arena->slots == NULL) {
        argorix_trap("INVALID_HANDLE");
    }
    const argorix_slot *slot = &arena->slots[handle.slot];
    if (!slot->active || handle.allocation_generation != slot->generation) {
        argorix_trap("USE_AFTER_FREE");
    }
    if (handle.type_id != expected_type_id || handle.type_id != slot->type_id) {
        argorix_trap("TYPE_MISMATCH");
    }
    if (handle.offset > slot->length || handle.length > slot->length - handle.offset) {
        argorix_trap("INDEX_OUT_OF_BOUNDS");
    }
    if (require_write && handle.permission != ARGORIX_PERMISSION_READ_WRITE) {
        argorix_trap("PERMISSION_DENIED");
    }
}

#define DEFINE_UNSIGNED_CHECKED(width, type, maximum)                         \
    type argorix_u##width##_add(type left, type right) {                      \
        /* The headroom goes through a local of the same width: written     \
           inline, `(type)(maximum - left)` is the bitwise complement of    \
           `left` for the narrow widths, and GCC 12 rejects comparing that  \
           promoted value with an unsigned one under -Werror=sign-compare. */ \
        const type headroom = (type)((maximum) - left);                        \
        if (right > headroom) {                                                \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left + right);                                           \
    }                                                                          \
    type argorix_u##width##_sub(type left, type right) {                       \
        if (left < right) {                                                    \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left - right);                                           \
    }                                                                          \
    type argorix_u##width##_mul(type left, type right) {                       \
        if (right != 0U && left > (type)((maximum) / right)) {                 \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left * right);                                           \
    }                                                                          \
    type argorix_u##width##_div(type left, type right) {                       \
        if (right == 0U) {                                                     \
            argorix_trap("DIVISION_BY_ZERO");                                \
        }                                                                      \
        return (type)(left / right);                                           \
    }                                                                          \
    type argorix_u##width##_rem(type left, type right) {                       \
        if (right == 0U) {                                                     \
            argorix_trap("DIVISION_BY_ZERO");                                \
        }                                                                      \
        return (type)(left % right);                                           \
    }

DEFINE_UNSIGNED_CHECKED(8, uint8_t, UINT8_MAX)
DEFINE_UNSIGNED_CHECKED(16, uint16_t, UINT16_MAX)
DEFINE_UNSIGNED_CHECKED(32, uint32_t, UINT32_MAX)
DEFINE_UNSIGNED_CHECKED(64, uint64_t, UINT64_MAX)

#define DEFINE_SIGNED_CHECKED(width, type, minimum, maximum)                  \
    type argorix_i##width##_add(type left, type right) {                       \
        if ((right > 0 && left > (type)((maximum) - right)) ||                 \
            (right < 0 && left < (type)((minimum) - right))) {                 \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left + right);                                           \
    }                                                                          \
    type argorix_i##width##_sub(type left, type right) {                       \
        if ((right < 0 && left > (type)((maximum) + right)) ||                 \
            (right > 0 && left < (type)((minimum) + right))) {                 \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left - right);                                           \
    }                                                                          \
    type argorix_i##width##_mul(type left, type right) {                       \
        if ((left == (minimum) && right == -1) ||                              \
            (right == (minimum) && left == -1) ||                              \
            (left > 0 && right > 0 && left > (type)((maximum) / right)) ||     \
            (left > 0 && right < 0 && right < (type)((minimum) / left)) ||     \
            (left < 0 && right > 0 && left < (type)((minimum) / right)) ||     \
            (left < 0 && right < 0 && left < (type)((maximum) / right))) {     \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left * right);                                           \
    }                                                                          \
    type argorix_i##width##_div(type left, type right) {                       \
        if (right == 0) {                                                      \
            argorix_trap("DIVISION_BY_ZERO");                                \
        }                                                                      \
        if (left == (minimum) && right == -1) {                                \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left / right);                                           \
    }                                                                          \
    type argorix_i##width##_rem(type left, type right) {                       \
        if (right == 0) {                                                      \
            argorix_trap("DIVISION_BY_ZERO");                                \
        }                                                                      \
        if (left == (minimum) && right == -1) {                                \
            argorix_trap("INTEGER_OVERFLOW");                                \
        }                                                                      \
        return (type)(left % right);                                           \
    }

DEFINE_SIGNED_CHECKED(8, int8_t, INT8_MIN, INT8_MAX)
DEFINE_SIGNED_CHECKED(16, int16_t, INT16_MIN, INT16_MAX)
DEFINE_SIGNED_CHECKED(32, int32_t, INT32_MIN, INT32_MAX)
DEFINE_SIGNED_CHECKED(64, int64_t, INT64_MIN, INT64_MAX)

#define DEFINE_UNSIGNED_SHIFTS(width, type)                                    \
    type argorix_u##width##_shl(type left, type right) {                       \
        if (right >= (type)(width)) {                                          \
            argorix_trap("SHIFT_OUT_OF_RANGE");                                \
        }                                                                      \
        return (type)((uint64_t)left << right);                                \
    }                                                                          \
    type argorix_u##width##_shr(type left, type right) {                       \
        if (right >= (type)(width)) {                                          \
            argorix_trap("SHIFT_OUT_OF_RANGE");                                \
        }                                                                      \
        return (type)(left >> right);                                          \
    }

#define DEFINE_SIGNED_SHIFTS(width, type, utype, minimum)                      \
    type argorix_i##width##_shl(type left, type right) {                       \
        if (right < 0 || right >= (type)(width)) {                             \
            argorix_trap("SHIFT_OUT_OF_RANGE");                                \
        }                                                                      \
        /* Shifting through the unsigned representation is defined for every   \
           input, including a negative left operand. */                        \
        return (type)(utype)((utype)left << (utype)right);                     \
    }                                                                          \
    type argorix_i##width##_shr(type left, type right) {                       \
        if (right < 0 || right >= (type)(width)) {                             \
            argorix_trap("SHIFT_OUT_OF_RANGE");                                \
        }                                                                      \
        if (left >= 0) {                                                       \
            return (type)((utype)left >> (utype)right);                        \
        }                                                                      \
        /* Arithmetic shift written out, so the sign is kept whatever the      \
           implementation does with `>>` on a negative value. */               \
        return (type)~(utype)((utype)~(utype)left >> (utype)right);            \
    }                                                                          \
    type argorix_i##width##_neg(type value) {                                  \
        if (value == (minimum)) {                                              \
            argorix_trap("INTEGER_OVERFLOW");                                  \
        }                                                                      \
        return (type)(-value);                                                 \
    }

DEFINE_UNSIGNED_SHIFTS(8, uint8_t)
DEFINE_UNSIGNED_SHIFTS(16, uint16_t)
DEFINE_UNSIGNED_SHIFTS(32, uint32_t)
DEFINE_UNSIGNED_SHIFTS(64, uint64_t)

DEFINE_SIGNED_SHIFTS(8, int8_t, uint8_t, INT8_MIN)
DEFINE_SIGNED_SHIFTS(16, int16_t, uint16_t, INT16_MIN)
DEFINE_SIGNED_SHIFTS(32, int32_t, uint32_t, INT32_MIN)
DEFINE_SIGNED_SHIFTS(64, int64_t, uint64_t, INT64_MIN)
