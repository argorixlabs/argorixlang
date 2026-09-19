#include "argorix_core_runtime.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

int main(int argc, char **argv) {
    if (argc != 2) {
        return 2;
    }
    if (strcmp(argv[1], "ok") == 0) {
        argorix_budget budget = {2U};
        argorix_step(&budget);
        argorix_step(&budget);
        uint32_t value = argorix_u32_mul(argorix_u32_add(20U, 1U), 2U);
        (void)printf("ARGORIX_RESULT:%" PRIu32 "\n", value);
        return 0;
    }
    if (strcmp(argv[1], "overflow") == 0) {
        (void)argorix_u8_add(UINT8_MAX, 1U);
        return 3;
    }
    if (strcmp(argv[1], "divide-zero") == 0) {
        (void)argorix_i32_div(7, 0);
        return 3;
    }
    if (strcmp(argv[1], "budget") == 0) {
        argorix_budget budget = {0U};
        argorix_step(&budget);
        return 3;
    }
    if (strcmp(argv[1], "handle-ok") == 0 ||
        strcmp(argv[1], "handle-generation") == 0 ||
        strcmp(argv[1], "arena-released") == 0) {
        argorix_slot slots[1] = {{7U, 42U, 8U, true}};
        argorix_arena_view arena = {11U, 3U, slots, 1U, true};
        argorix_handle handle = {
            11U, 3U, 0U, 7U, 2U, 4U, 42U,
            ARGORIX_PERMISSION_READ_WRITE, {0U, 0U, 0U}
        };
        if (strcmp(argv[1], "handle-generation") == 0) {
            handle.allocation_generation = 6U;
        } else if (strcmp(argv[1], "arena-released") == 0) {
            arena.active = false;
        }
        argorix_validate_handle(&arena, handle, 42U, true);
        (void)printf("ARGORIX_RESULT:HANDLE_OK\n");
        return 0;
    }
    return 2;
}
