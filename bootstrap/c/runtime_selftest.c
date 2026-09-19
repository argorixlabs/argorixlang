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
    return 2;
}
