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

_Noreturn void argorix_trap(const char *code);
void argorix_step(argorix_budget *budget);

uint8_t argorix_u8_add(uint8_t left, uint8_t right);
uint8_t argorix_u8_sub(uint8_t left, uint8_t right);
uint8_t argorix_u8_mul(uint8_t left, uint8_t right);
uint8_t argorix_u8_div(uint8_t left, uint8_t right);
uint8_t argorix_u8_rem(uint8_t left, uint8_t right);

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

int64_t argorix_i64_add(int64_t left, int64_t right);
int64_t argorix_i64_sub(int64_t left, int64_t right);
int64_t argorix_i64_mul(int64_t left, int64_t right);
int64_t argorix_i64_div(int64_t left, int64_t right);
int64_t argorix_i64_rem(int64_t left, int64_t right);

#endif
