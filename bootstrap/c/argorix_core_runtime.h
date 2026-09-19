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

_Noreturn void argorix_trap(const char *code);
void argorix_step(argorix_budget *budget);
size_t argorix_bounds(uint64_t index, uint64_t length);
argorix_string argorix_decode_utf8(argorix_bytes value);

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
