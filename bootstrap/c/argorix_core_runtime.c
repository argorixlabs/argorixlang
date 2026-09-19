#include "argorix_core_runtime.h"

#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

_Noreturn void argorix_trap(const char *code) {
    (void)fprintf(stderr, "ARGORIX_TRAP:%s\n", code);
    exit(ARGORIX_TRAP_EXIT);
}

void argorix_step(argorix_budget *budget) {
    if (budget == NULL || budget->remaining_steps == 0U) {
        argorix_trap("STEP_LIMIT");
    }
    budget->remaining_steps -= 1U;
}

#define DEFINE_UNSIGNED_CHECKED(width, type, maximum)                         \
    type argorix_u##width##_add(type left, type right) {                      \
        if (right > (type)((maximum) - left)) {                               \
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
