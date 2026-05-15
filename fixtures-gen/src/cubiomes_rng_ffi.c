/*
 * cubiomes_rng_ffi.c — non-inline wrappers around cubiomes' static
 * inline RNG helpers so they can be linked from Rust via plain FFI
 * symbols. Mirrors the API exposed from `cubiomes/rng.h`.
 */

#include "rng.h"

uint64_t cubiomes_set_seed(uint64_t value) {
    uint64_t s;
    setSeed(&s, value);
    return s;
}

int cubiomes_next(uint64_t *seed, int bits) { return next(seed, bits); }
int cubiomes_next_int(uint64_t *seed, int n) { return nextInt(seed, n); }
uint64_t cubiomes_next_long(uint64_t *seed) { return nextLong(seed); }
float cubiomes_next_float(uint64_t *seed) { return nextFloat(seed); }
double cubiomes_next_double(uint64_t *seed) { return nextDouble(seed); }
void cubiomes_skip_next_n(uint64_t *seed, uint64_t n) { skipNextN(seed, n); }

int cubiomes_next_int_24(uint64_t *seed) {
    int x;
    uint64_t s = *seed;
    JAVA_NEXT_INT24(s, x);
    *seed = s;
    return x;
}

void cubiomes_x_set_seed(Xoroshiro *xr, uint64_t value) { xSetSeed(xr, value); }
uint64_t cubiomes_x_next_long(Xoroshiro *xr) { return xNextLong(xr); }
int cubiomes_x_next_int(Xoroshiro *xr, uint32_t n) { return xNextInt(xr, n); }
double cubiomes_x_next_double(Xoroshiro *xr) { return xNextDouble(xr); }
float cubiomes_x_next_float(Xoroshiro *xr) { return xNextFloat(xr); }
void cubiomes_x_skip_n(Xoroshiro *xr, int count) { xSkipN(xr, count); }
uint64_t cubiomes_x_next_long_j(Xoroshiro *xr) { return xNextLongJ(xr); }
int cubiomes_x_next_int_j(Xoroshiro *xr, uint32_t n) { return xNextIntJ(xr, n); }

uint64_t cubiomes_mc_step_seed(uint64_t s, uint64_t salt) {
    return mcStepSeed(s, salt);
}
int cubiomes_mc_first_int(uint64_t s, int m) { return mcFirstInt(s, m); }
int cubiomes_mc_first_is_zero(uint64_t s, int m) { return mcFirstIsZero(s, m); }
uint64_t cubiomes_get_chunk_seed(uint64_t ss, int x, int z) {
    return getChunkSeed(ss, x, z);
}
uint64_t cubiomes_get_layer_salt(uint64_t salt) { return getLayerSalt(salt); }
uint64_t cubiomes_get_start_salt(uint64_t ws, uint64_t ls) {
    return getStartSalt(ws, ls);
}
uint64_t cubiomes_get_start_seed(uint64_t ws, uint64_t ls) {
    return getStartSeed(ws, ls);
}
uint64_t cubiomes_mul_inv(uint64_t x, uint64_t m) { return mulInv(x, m); }
