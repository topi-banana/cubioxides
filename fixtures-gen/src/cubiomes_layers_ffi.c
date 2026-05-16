/*
 * cubiomes_layers_ffi.c — wrappers around cubiomes' layer-map functions
 * so they can be exercised from Rust without exposing the full Layer
 * struct layout.
 *
 * Each wrapper builds a freshly zero-initialised `Layer`, sets only the
 * fields the corresponding map function reads, and forwards to cubiomes.
 */

#include "biomenoise.h"
#include "generator.h"
#include "layers.h"
#include "noise.h"
#include <stdio.h>
#include <string.h>

double cubiomes_call_sample_surface_noise(int dim, uint64_t seed, int x, int y,
                                          int z) {
    SurfaceNoise sn;
    initSurfaceNoise(&sn, dim, seed);
    return sampleSurfaceNoise(&sn, x, y, z);
}

double cubiomes_call_sample_surface_noise_between(int dim, uint64_t seed, int x,
                                                  int y, int z, double nmin,
                                                  double nmax) {
    SurfaceNoise sn;
    initSurfaceNoise(&sn, dim, seed);
    return sampleSurfaceNoiseBetween(&sn, x, y, z, nmin, nmax);
}

void cubiomes_call_map_nether_2d(uint64_t seed, int *out, int x, int z, int w,
                                 int h) {
    NetherNoise nn;
    setNetherSeed(&nn, seed);
    mapNether2D(&nn, out, x, z, w, h);
}

int cubiomes_call_get_nether_biome(uint64_t seed, int x, int y, int z,
                                   float *ndel) {
    NetherNoise nn;
    setNetherSeed(&nn, seed);
    return getNetherBiome(&nn, x, y, z, ndel);
}

void cubiomes_call_map_end_biome(int mc, uint64_t seed, int *out, int x, int z,
                                 int w, int h) {
    EndNoise en;
    setEndSeed(&en, mc, seed);
    mapEndBiome(&en, out, x, z, w, h);
}

void cubiomes_call_map_end(int mc, uint64_t seed, int *out, int x, int z, int w,
                           int h) {
    EndNoise en;
    setEndSeed(&en, mc, seed);
    mapEnd(&en, out, x, z, w, h);
}

int cubiomes_call_climate_to_biome(int mc, const uint64_t np[6]) {
    return climateToBiome(mc, np, NULL);
}

/* Initialise + seed a BiomeNoise in one call, sample at (x, y, z),
 * and write the (biome_id, np[6]) tuple back via the output
 * pointers. */
#include "biomes.h"
#include "finders.h"
#include "quadbase.h"

extern int isStrongholdBiome(int mc, int id);
int cubiomes_call_is_stronghold_biome(int mc, int id) {
    return isStrongholdBiome(mc, id);
}
int cubiomes_call_biome_exists(int mc, int id) {
    return biomeExists(mc, id);
}
int cubiomes_call_is_overworld(int mc, int id) {
    return isOverworld(mc, id);
}

int cubiomes_call_is_slime_chunk(uint64_t seed, int cx, int cz) {
    return isSlimeChunk(seed, cx, cz);
}

float cubiomes_call_is_quad_base_feature_24_classic(int structure_type, int mc,
                                                    uint64_t seed) {
    StructureConfig sconf;
    if (!getStructureConfig(structure_type, mc, &sconf)) return 0.0f;
    return isQuadBaseFeature24Classic(sconf, seed);
}

float cubiomes_call_is_quad_base_feature_24(int structure_type, int mc,
                                            uint64_t seed, int ax, int ay,
                                            int az) {
    StructureConfig sconf;
    if (!getStructureConfig(structure_type, mc, &sconf)) return 0.0f;
    return isQuadBaseFeature24(sconf, seed, ax, ay, az);
}

int cubiomes_call_get_quad_hut_cst(uint64_t low20) {
    return getQuadHutCst(low20);
}

void cubiomes_call_estimate_spawn(int mc, uint64_t seed, int *px, int *pz) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, 0, seed);
    Pos p = estimateSpawn(&g, NULL);
    *px = p.x;
    *pz = p.z;
}

#include "finders.h"

/* getPopulationSeed is defined in finders.c but not exported via the
 * cubiomes public header; forward-declare it locally. */
extern uint64_t getPopulationSeed(int mc, uint64_t ws, int x, int z);

uint64_t cubiomes_call_get_population_seed(int mc, uint64_t ws, int x, int z) {
    return getPopulationSeed(mc, ws, x, z);
}

int cubiomes_call_get_end_islands(int mc, uint64_t seed, int chunk_x,
                                  int chunk_z, int *out_xyzr) {
    EndIsland is[2];
    int n = getEndIslands(is, mc, seed, chunk_x, chunk_z);
    for (int i = 0; i < n; i++) {
        out_xyzr[i * 4 + 0] = is[i].x;
        out_xyzr[i * 4 + 1] = is[i].y;
        out_xyzr[i * 4 + 2] = is[i].z;
        out_xyzr[i * 4 + 3] = is[i].r;
    }
    return n;
}

int cubiomes_call_map_end_island_height(int mc, uint64_t seed, int x, int z,
                                        int w, int h, int scale, float *y) {
    EndNoise en;
    setEndSeed(&en, mc, seed);
    for (int i = 0; i < w * h; i++) {
        y[i] = 0.0f;
    }
    return mapEndIslandHeight(y, &en, seed, x, z, w, h, scale);
}

extern float getEndHeightNoise(const EndNoise *en, int x, int z, int range);

float cubiomes_call_end_height_noise(int mc, uint64_t seed, int x, int z,
                                     int range) {
    EndNoise en;
    setEndSeed(&en, mc, seed);
    return getEndHeightNoise(&en, x, z, range);
}

int cubiomes_call_map_end_surface_height(int mc, uint64_t seed, int x, int z,
                                         int w, int h, int scale, int ymin,
                                         float *y) {
    EndNoise en;
    setEndSeed(&en, mc, seed);
    SurfaceNoise sn;
    initSurfaceNoise(&sn, DIM_END, seed);
    return mapEndSurfaceHeight(y, &en, &sn, x, z, w, h, scale, ymin);
}

int cubiomes_call_is_end_chunk_empty(int mc, uint64_t seed, int chunk_x,
                                     int chunk_z) {
    EndNoise en;
    setEndSeed(&en, mc, seed);
    SurfaceNoise sn;
    initSurfaceNoise(&sn, DIM_END, seed);
    return isEndChunkEmpty(&en, &sn, seed, chunk_x, chunk_z);
}

int cubiomes_call_get_biome_depth_and_scale(int id, double *depth,
                                            double *scale, int *grass) {
    return getBiomeDepthAndScale(id, depth, scale, grass);
}

int cubiomes_call_map_approx_height(int mc, int dim, uint64_t seed, int x,
                                    int z, int w, int h, float *y, int *ids) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, dim, seed);
    SurfaceNoise sn;
    initSurfaceNoise(&sn, dim, seed);
    return mapApproxHeight(y, ids, &g, &sn, x, z, w, h);
}

void cubiomes_call_get_spawn(int mc, uint64_t seed, int *px, int *pz) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, 0, seed);
    Pos p = getSpawn(&g);
    *px = p.x;
    *pz = p.z;
}

int cubiomes_call_is_viable_feature_biome(int mc, int structure_type,
                                          int biome_id) {
    return isViableFeatureBiome(mc, structure_type, biome_id);
}

int cubiomes_call_is_viable_structure_pos(int mc, int dim, int structure_type,
                                          uint64_t seed, int x, int z,
                                          uint32_t flags) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, dim, seed);
    return isViableStructurePos(structure_type, &g, x, z, flags);
}

/* searchAll48 callback context — emulates cubiomes' user-data
 * pattern. The "check" callback returns 1 if a seed passes the
 * Swamp_Hut + radius=128 filter. */
typedef struct {
    StructureConfig sconf;
    int radius;
} search_all_ctx_t;

static int search_all_check_quad_hut(uint64_t s48, void *data) {
    search_all_ctx_t *ctx = (search_all_ctx_t *)data;
    return isQuadBase(ctx->sconf, s48, ctx->radius) ? 1 : 0;
}

double cubiomes_call_approx_surface_beta(uint64_t seed, int x, int z) {
    BiomeNoiseBeta bnb;
    memset(&bnb, 0, sizeof(bnb));
    setBetaBiomeSeed(&bnb, seed);
    SurfaceNoiseBeta snb;
    initSurfaceNoiseBeta(&snb, seed);
    return approxSurfaceBeta(&bnb, &snb, x, z);
}

int cubiomes_call_is_quad_base(int mc, int sty, uint64_t seed, int radius,
                               float *out_sqrad) {
    StructureConfig sc;
    if (!getStructureConfig(sty, mc, &sc)) {
        return 0;
    }
    float r = isQuadBase(sc, seed, radius);
    *out_sqrad = r;
    return r != 0.0f ? 1 : 0;
}

int cubiomes_call_search_all48_quad_hut(int mc, uint64_t start, uint64_t end,
                                        const uint64_t *low_bits,
                                        int low_bit_count, uint64_t *out_seeds,
                                        int n_max) {
    StructureConfig sc;
    if (!getStructureConfig(Swamp_Hut, mc, &sc)) {
        return 0;
    }
    /* Cubiomes' searchAll48 takes a NULL-terminated low_bits array. */
    uint64_t *lb = (uint64_t *)malloc(sizeof(uint64_t) * (size_t)(low_bit_count + 1));
    for (int i = 0; i < low_bit_count; i++) lb[i] = low_bits[i];
    lb[low_bit_count] = 0;
    search_all_ctx_t ctx = {sc, 128};
    /* Inline the relevant portion of searchAll48Thread directly to
     * avoid the file I/O + threading wrapper. */
    const int lbitn = 20;
    const uint64_t hstep = 1ULL << lbitn;
    const uint64_t hmask = ~(hstep - 1);
    int cnt = low_bit_count;
    uint64_t mid = start & hmask;
    int idx = 0;
    uint64_t seed = mid | lb[idx];
    while (seed < start) {
        idx++;
        if (idx >= cnt) {
            idx = 0;
            mid += hstep;
        }
        seed = mid | lb[idx];
    }
    int written = 0;
    while (seed <= end) {
        if (search_all_check_quad_hut(seed, &ctx)) {
            if (written < n_max) {
                out_seeds[written++] = seed;
            } else {
                break;
            }
        }
        idx++;
        if (idx >= cnt) {
            idx = 0;
            mid += hstep;
        }
        seed = mid | lb[idx];
    }
    free(lb);
    return written;
}

int cubiomes_call_scan_for_quads(int mc, int sty, int radius, uint64_t s48,
                                 const uint64_t *low_bits, int low_bit_count,
                                 uint64_t salt, int x, int z, int w, int h,
                                 int *out_xz, int n) {
    StructureConfig sc;
    if (!getStructureConfig(sty, mc, &sc)) {
        return 0;
    }
    Pos *buf = (Pos *)malloc((size_t)n * sizeof(Pos));
    /* Cubiomes expects the low_bits array to be 0-terminated; copy
     * the caller's slice into a fresh buffer and append a sentinel. */
    uint64_t *lb = (uint64_t *)malloc(sizeof(uint64_t) * (size_t)(low_bit_count + 1));
    for (int i = 0; i < low_bit_count; i++) lb[i] = low_bits[i];
    lb[low_bit_count] = 0;
    int cnt = scanForQuads(sc, radius, s48, lb, /*lbitn=*/20, salt, x, z, w, h, buf, n);
    for (int i = 0; i < cnt && i < n; i++) {
        out_xz[i * 2 + 0] = buf[i].x;
        out_xz[i * 2 + 1] = buf[i].z;
    }
    free(lb);
    free(buf);
    return cnt;
}

void cubiomes_call_get_linked_gateway_pos(int mc, uint64_t seed, int src_x,
                                          int src_z, int *out_x, int *out_z) {
    EndNoise en;
    setEndSeed(&en, mc, seed);
    SurfaceNoise sn;
    initSurfaceNoise(&sn, DIM_END, seed);
    Pos src = {src_x, src_z};
    Pos dst = getLinkedGatewayPos(&en, &sn, seed, src);
    *out_x = dst.x;
    *out_z = dst.z;
}

void cubiomes_call_get_fixed_end_gateways(int mc, uint64_t seed, int *out_xz) {
    Pos src[20];
    getFixedEndGateways(mc, seed, src);
    for (int i = 0; i < 20; i++) {
        out_xz[i * 2 + 0] = src[i].x;
        out_xz[i * 2 + 1] = src[i].z;
    }
}

int cubiomes_call_get_variant(int structure_type, int mc, uint64_t seed, int x,
                              int z, int biome_id, int *out) {
    StructureVariant sv;
    int rc = getVariant(&sv, structure_type, mc, seed, x, z, biome_id);
    out[0] = sv.abandoned;
    out[1] = sv.giant;
    out[2] = sv.underground;
    out[3] = sv.airpocket;
    out[4] = sv.basement;
    out[5] = sv.cracked;
    out[6] = sv.size;
    out[7] = sv.start;
    out[8] = sv.biome;
    out[9] = sv.rotation;
    out[10] = sv.mirror;
    out[11] = sv.x;
    out[12] = sv.y;
    out[13] = sv.z;
    out[14] = sv.sx;
    out[15] = sv.sy;
    out[16] = sv.sz;
    return rc;
}

#include "quadbase.h"

void cubiomes_call_get_optimal_afk(int *px, int *pz, int *spcnt, int p0x,
                                   int p0z, int p1x, int p1z, int p2x, int p2z,
                                   int p3x, int p3z, int ax, int ay, int az) {
    Pos p[4] = {
        {p0x, p0z}, {p1x, p1z}, {p2x, p2z}, {p3x, p3z},
    };
    int count = 0;
    Pos afk = getOptimalAfk(p, ax, ay, az, &count);
    *px = afk.x;
    *pz = afk.z;
    if (spcnt) *spcnt = count;
}


void cubiomes_call_nth_strongholds(int mc, uint64_t seed, int n_steps,
                                   int *out_xz) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, 0, seed);
    StrongholdIter sh;
    initFirstStronghold(&sh, mc, seed);
    for (int i = 0; i < n_steps; i++) {
        int rem = nextStronghold(&sh, &g);
        out_xz[i * 2 + 0] = sh.pos.x;
        out_xz[i * 2 + 1] = sh.pos.z;
        if (rem <= 0) {
            for (int k = i + 1; k < n_steps; k++) {
                out_xz[k * 2 + 0] = 0;
                out_xz[k * 2 + 1] = 0;
            }
            break;
        }
    }
}

void cubiomes_call_init_first_stronghold(int mc, uint64_t seed, int *px,
                                         int *pz) {
    Pos p = initFirstStronghold(NULL, mc, seed);
    *px = p.x;
    *pz = p.z;
}

#include <stdlib.h>
int cubiomes_call_get_mineshafts(int mc, uint64_t seed, int cx0, int cz0,
                                 int cx1, int cz1, int *out_xz, int n_max,
                                 int *total) {
    Pos *buf = (Pos *)malloc((size_t)n_max * sizeof(Pos));
    int n = getMineshafts(mc, seed, cx0, cz0, cx1, cz1, buf, n_max);
    int written = n < n_max ? n : n_max;
    for (int i = 0; i < written; i++) {
        out_xz[i * 2 + 0] = buf[i].x;
        out_xz[i * 2 + 1] = buf[i].z;
    }
    *total = n;
    free(buf);
    return written;
}

/* getStructurePos(type, mc, seed, regX, regZ, pos) — write the
 * attempt position into pos_x / pos_z. Returns the cubiomes valid
 * flag (1 = structure placed, 0 = no structure in this region). */
int cubiomes_call_get_structure_pos(int structure_type, int mc, uint64_t seed,
                                    int reg_x, int reg_z, int *pos_x,
                                    int *pos_z) {
    Pos pos = {0, 0};
    int valid = getStructurePos(structure_type, mc, seed, reg_x, reg_z, &pos);
    *pos_x = pos.x;
    *pos_z = pos.z;
    return valid;
}

int cubiomes_call_get_biome_at(int mc, uint32_t flags, int dim, uint64_t seed,
                               int scale, int x, int y, int z) {
    Generator g;
    setupGenerator(&g, mc, flags);
    applySeed(&g, dim, seed);
    return getBiomeAt(&g, scale, x, y, z);
}

#include <stdlib.h>
/* Run cubiomes' setupGenerator + applySeed + allocCache + genBiomes
 * and copy the first sx*sy*sz biome ids into `out`. Uses
 * `allocCache` because cubiomes' `genBiomes` reads scratch beyond
 * sx*sy*sz for some layer / Voronoi paths. Returns cubiomes' error
 * code (0 on success). */
int cubiomes_call_gen_biomes(int mc, uint32_t flags, int dim, uint64_t seed,
                             int scale, int x, int z, int sx, int sz, int y,
                             int sy, int *out) {
    Generator g;
    setupGenerator(&g, mc, flags);
    applySeed(&g, dim, seed);
    Range r = {scale, x, z, sx, sz, y, sy};
    int *cache = allocCache(&g, r);
    if (!cache) return -1;
    int err = genBiomes(&g, cache, r);
    if (err == 0) {
        int sy_norm = sy == 0 ? 1 : sy;
        memcpy(out, cache, sizeof(int) * (size_t)sx * sz * sy_norm);
    }
    free(cache);
    return err;
}

int cubiomes_call_sample_biome_noise_beta(uint64_t seed, int x, int z,
                                          double *t_out, double *h_out) {
    BiomeNoiseBeta bnb;
    memset(&bnb, 0, sizeof(bnb));
    setBetaBiomeSeed(&bnb, seed);
    double nv[2];
    int id = sampleBiomeNoiseBeta(&bnb, NULL, nv, x, z);
    *t_out = nv[0];
    *h_out = nv[1];
    return id;
}

int cubiomes_call_sample_biome_noise(int mc, uint64_t seed, int large_biomes,
                                     int x, int y, int z, int sample_flags,
                                     int64_t *np_out) {
    BiomeNoise bn;
    memset(&bn, 0, sizeof(bn));
    initBiomeNoise(&bn, mc);
    setBiomeSeed(&bn, seed, large_biomes);
    int64_t np[6];
    int id = sampleBiomeNoise(&bn, np, x, y, z, NULL, (uint32_t)sample_flags);
    for (int i = 0; i < 6; i++) np_out[i] = np[i];
    return id;
}

void cubiomes_call_map_continent(uint64_t start_seed, int *out, int x, int z,
                                 int w, int h) {
    Layer l;
    memset(&l, 0, sizeof(l));
    l.startSeed = start_seed;
    mapContinent(&l, out, x, z, w, h);
}

/* Helper: build a (parent = mapContinent) -> (child = mapZoom*) pair,
 * setLayerSeed the chain, and invoke the child. */
static int call_zoom_chain(mapfunc_t *child_fn, uint64_t world_seed,
                           uint64_t parent_layer_salt,
                           uint64_t child_layer_salt, int *out, int x, int z,
                           int w, int h) {
    Layer parent;
    memset(&parent, 0, sizeof(parent));
    parent.getMap = mapContinent;
    parent.layerSalt = parent_layer_salt;

    Layer child;
    memset(&child, 0, sizeof(child));
    child.getMap = child_fn;
    child.layerSalt = child_layer_salt;
    child.p = &parent;

    setLayerSeed(&child, world_seed);
    return child_fn(&child, out, x, z, w, h);
}

void cubiomes_call_map_zoom_fuzzy(uint64_t world_seed,
                                  uint64_t parent_layer_salt,
                                  uint64_t zoom_layer_salt, int *out, int x,
                                  int z, int w, int h) {
    call_zoom_chain(mapZoomFuzzy, world_seed, parent_layer_salt,
                    zoom_layer_salt, out, x, z, w, h);
}

void cubiomes_call_map_zoom(uint64_t world_seed, uint64_t parent_layer_salt,
                            uint64_t zoom_layer_salt, int *out, int x, int z,
                            int w, int h) {
    call_zoom_chain(mapZoom, world_seed, parent_layer_salt, zoom_layer_salt,
                    out, x, z, w, h);
}

static int call_land_chain(mapfunc_t *land_fn, uint64_t world_seed,
                           uint64_t parent_layer_salt, uint64_t land_layer_salt,
                           int *out, int x, int z, int w, int h) {
    Layer parent;
    memset(&parent, 0, sizeof(parent));
    parent.getMap = mapContinent;
    parent.layerSalt = parent_layer_salt;

    Layer land;
    memset(&land, 0, sizeof(land));
    land.getMap = land_fn;
    land.layerSalt = land_layer_salt;
    land.p = &parent;

    setLayerSeed(&land, world_seed);
    return land_fn(&land, out, x, z, w, h);
}

void cubiomes_call_map_land(uint64_t world_seed, uint64_t parent_layer_salt,
                            uint64_t land_layer_salt, int *out, int x, int z,
                            int w, int h) {
    call_land_chain(mapLand, world_seed, parent_layer_salt, land_layer_salt,
                    out, x, z, w, h);
}

void cubiomes_call_map_land16(uint64_t world_seed, uint64_t parent_layer_salt,
                              uint64_t land_layer_salt, int *out, int x, int z,
                              int w, int h) {
    call_land_chain(mapLand16, world_seed, parent_layer_salt, land_layer_salt,
                    out, x, z, w, h);
}

void cubiomes_call_map_land_b18(uint64_t world_seed,
                                uint64_t parent_layer_salt,
                                uint64_t land_layer_salt, int *out, int x,
                                int z, int w, int h) {
    call_land_chain(mapLandB18, world_seed, parent_layer_salt, land_layer_salt,
                    out, x, z, w, h);
}

/* Generic 2-layer chain: parent = mapContinent, child = the requested
 * mapfunc_t. Used for layers whose parity tests only need a binary
 * (ocean / plains) parent grid (island, snow*, special, mushroom,
 * deep_ocean). */
static int call_simple_chain(mapfunc_t *child_fn, uint64_t world_seed,
                             uint64_t parent_layer_salt,
                             uint64_t child_layer_salt, int *out, int x, int z,
                             int w, int h) {
    Layer parent;
    memset(&parent, 0, sizeof(parent));
    parent.getMap = mapContinent;
    parent.layerSalt = parent_layer_salt;

    Layer child;
    memset(&child, 0, sizeof(child));
    child.getMap = child_fn;
    child.layerSalt = child_layer_salt;
    child.p = &parent;

    setLayerSeed(&child, world_seed);
    return child_fn(&child, out, x, z, w, h);
}

void cubiomes_call_map_island(uint64_t world_seed, uint64_t parent_salt,
                              uint64_t child_salt, int *out, int x, int z,
                              int w, int h) {
    call_simple_chain(mapIsland, world_seed, parent_salt, child_salt, out, x, z,
                      w, h);
}

void cubiomes_call_map_snow16(uint64_t world_seed, uint64_t parent_salt,
                              uint64_t child_salt, int *out, int x, int z,
                              int w, int h) {
    call_simple_chain(mapSnow16, world_seed, parent_salt, child_salt, out, x, z,
                      w, h);
}

void cubiomes_call_map_snow(uint64_t world_seed, uint64_t parent_salt,
                            uint64_t child_salt, int *out, int x, int z, int w,
                            int h) {
    call_simple_chain(mapSnow, world_seed, parent_salt, child_salt, out, x, z,
                      w, h);
}

void cubiomes_call_map_special(uint64_t world_seed, uint64_t parent_salt,
                               uint64_t child_salt, int *out, int x, int z,
                               int w, int h) {
    call_simple_chain(mapSpecial, world_seed, parent_salt, child_salt, out, x,
                      z, w, h);
}

void cubiomes_call_map_mushroom(uint64_t world_seed, uint64_t parent_salt,
                                uint64_t child_salt, int *out, int x, int z,
                                int w, int h) {
    call_simple_chain(mapMushroom, world_seed, parent_salt, child_salt, out, x,
                      z, w, h);
}

void cubiomes_call_map_deep_ocean(uint64_t world_seed, uint64_t parent_salt,
                                  uint64_t child_salt, int *out, int x, int z,
                                  int w, int h) {
    call_simple_chain(mapDeepOcean, world_seed, parent_salt, child_salt, out, x,
                      z, w, h);
}

/* 3-layer chain: mapContinent -> mapSnow -> child. The cool / heat
 * layers expect a temperature-category parent, which is what mapSnow
 * produces. */
static int call_chain3(mapfunc_t *child_fn, uint64_t world_seed,
                       uint64_t continent_salt, uint64_t snow_salt,
                       uint64_t child_salt, int *out, int x, int z, int w,
                       int h) {
    Layer continent;
    memset(&continent, 0, sizeof(continent));
    continent.getMap = mapContinent;
    continent.layerSalt = continent_salt;

    Layer snow;
    memset(&snow, 0, sizeof(snow));
    snow.getMap = mapSnow;
    snow.layerSalt = snow_salt;
    snow.p = &continent;

    Layer child;
    memset(&child, 0, sizeof(child));
    child.getMap = child_fn;
    child.layerSalt = child_salt;
    child.p = &snow;

    setLayerSeed(&child, world_seed);
    return child_fn(&child, out, x, z, w, h);
}

void cubiomes_call_map_cool(uint64_t world_seed, uint64_t continent_salt,
                            uint64_t snow_salt, uint64_t cool_salt, int *out,
                            int x, int z, int w, int h) {
    call_chain3(mapCool, world_seed, continent_salt, snow_salt, cool_salt, out,
                x, z, w, h);
}

void cubiomes_call_map_heat(uint64_t world_seed, uint64_t continent_salt,
                            uint64_t snow_salt, uint64_t heat_salt, int *out,
                            int x, int z, int w, int h) {
    call_chain3(mapHeat, world_seed, continent_salt, snow_salt, heat_salt, out,
                x, z, w, h);
}

/* 4-layer chain: mapContinent -> mapSnow -> mapBiome -> child. Each
 * layer is given its own salt. `child_fn` operates on biome-id input. */
static int call_chain4(mapfunc_t *child_fn, uint64_t world_seed, int mc,
                       uint64_t continent_salt, uint64_t snow_salt,
                       uint64_t biome_salt, uint64_t child_salt, int *out,
                       int x, int z, int w, int h) {
    Layer continent;
    memset(&continent, 0, sizeof(continent));
    continent.getMap = mapContinent;
    continent.layerSalt = continent_salt;
    continent.mc = mc;

    Layer snow;
    memset(&snow, 0, sizeof(snow));
    snow.getMap = mapSnow;
    snow.layerSalt = snow_salt;
    snow.mc = mc;
    snow.p = &continent;

    Layer biome;
    memset(&biome, 0, sizeof(biome));
    biome.getMap = mapBiome;
    biome.layerSalt = biome_salt;
    biome.mc = mc;
    biome.p = &snow;

    Layer child;
    memset(&child, 0, sizeof(child));
    child.getMap = child_fn;
    child.layerSalt = child_salt;
    child.mc = mc;
    child.p = &biome;

    setLayerSeed(&child, world_seed);
    return child_fn(&child, out, x, z, w, h);
}

void cubiomes_call_map_noise(uint64_t world_seed, int mc,
                             uint64_t continent_salt, uint64_t snow_salt,
                             uint64_t biome_salt, uint64_t child_salt, int *out,
                             int x, int z, int w, int h) {
    call_chain4(mapNoise, world_seed, mc, continent_salt, snow_salt, biome_salt,
                child_salt, out, x, z, w, h);
}

void cubiomes_call_map_bamboo(uint64_t world_seed, int mc,
                              uint64_t continent_salt, uint64_t snow_salt,
                              uint64_t biome_salt, uint64_t child_salt,
                              int *out, int x, int z, int w, int h) {
    call_chain4(mapBamboo, world_seed, mc, continent_salt, snow_salt,
                biome_salt, child_salt, out, x, z, w, h);
}

void cubiomes_call_map_swamp_river(uint64_t world_seed, int mc,
                                   uint64_t continent_salt, uint64_t snow_salt,
                                   uint64_t biome_salt, uint64_t child_salt,
                                   int *out, int x, int z, int w, int h) {
    call_chain4(mapSwampRiver, world_seed, mc, continent_salt, snow_salt,
                biome_salt, child_salt, out, x, z, w, h);
}

void cubiomes_call_map_sunflower(uint64_t world_seed, int mc,
                                 uint64_t continent_salt, uint64_t snow_salt,
                                 uint64_t biome_salt, uint64_t child_salt,
                                 int *out, int x, int z, int w, int h) {
    call_chain4(mapSunflower, world_seed, mc, continent_salt, snow_salt,
                biome_salt, child_salt, out, x, z, w, h);
}

#include "generator.h"

/* Run cubiomes' genArea at a specific layer slot and store the
 * output. setupLayerStack + setLayerSeed first, then forward to
 * the requested layer's getMap. Returns the cubiomes error code. */
int cubiomes_call_gen_area_at(int mc, int large_biomes, uint64_t world_seed,
                              int layer_id_ord, int *out, int x, int z,
                              int w, int h) {
    LayerStack g;
    setupLayerStack(&g, mc, large_biomes);
    setLayerSeed(g.entry_1, world_seed);
    return genArea(g.layers + layer_id_ord, out, x, z, w, h);
}

/* Run cubiomes' genArea at an arbitrary layer with a properly
 * sized cache (matches cubiomes' allocCache(g, r) sizing). The
 * `out` buffer must hold w*h cells (caller's responsibility). */
int cubiomes_call_gen_area_at_with_cache(int mc, int large_biomes,
                                         uint64_t world_seed,
                                         int layer_id_ord, int *out,
                                         int x, int z, int w, int h) {
    Generator g;
    setupGenerator(&g, mc, large_biomes ? LARGE_BIOMES : 0);
    applySeed(&g, 0, world_seed);
    Range r = {1, x, z, w, h, 0, 1};
    int *cache = allocCache(&g, r);
    if (!cache) return -1;
    int err = genArea(g.ls.layers + layer_id_ord, cache, x, z, w, h);
    if (err == 0) {
        memcpy(out, cache, sizeof(int) * (size_t)w * h);
    }
    free(cache);
    return err;
}

/* Run cubiomes' genArea at the per-version entry_1 (Voronoi1). */
int cubiomes_call_gen_area_at_entry1(int mc, int large_biomes,
                                     uint64_t world_seed, int *out, int x,
                                     int z, int w, int h) {
    LayerStack g;
    setupLayerStack(&g, mc, large_biomes);
    setLayerSeed(g.entry_1, world_seed);
    return genArea(g.entry_1, out, x, z, w, h);
}

/* Dump (layerSalt, startSalt, startSeed) of every node in a freshly
 * setup LayerStack into `out` after a setLayerSeed(entry_1, seed).
 * Layout: 3 * L_NUM uint64s in cubiomes index order. */
void cubiomes_call_dump_layer_stack(int mc, int large_biomes,
                                    uint64_t world_seed, uint64_t *out) {
    LayerStack g;
    setupLayerStack(&g, mc, large_biomes);
    setLayerSeed(g.entry_1, world_seed);
    for (int i = 0; i < L_NUM; i++) {
        out[3 * i + 0] = g.layers[i].layerSalt;
        out[3 * i + 1] = g.layers[i].startSalt;
        out[3 * i + 2] = g.layers[i].startSeed;
    }
}

void cubiomes_call_map_voronoi(uint64_t world_seed, uint64_t biome_salt,
                               int *out, int x, int z, int w, int h) {
    Layer biome;
    memset(&biome, 0, sizeof(biome));
    biome.getMap = mapContinent;
    biome.layerSalt = biome_salt;

    Layer voronoi;
    memset(&voronoi, 0, sizeof(voronoi));
    voronoi.getMap = mapVoronoi;
    voronoi.p = &biome;
    voronoi.layerSalt = (uint64_t)~0; /* LAYER_INIT_SHA marker */

    setLayerSeed(&voronoi, world_seed);
    mapVoronoi(&voronoi, out, x, z, w, h);
}

void cubiomes_call_voronoi_access_3d(uint64_t world_seed, int x, int y, int z,
                                     int *x4, int *y4, int *z4) {
    uint64_t sha = getVoronoiSHA(world_seed);
    voronoiAccess3D(sha, x, y, z, x4, y4, z4);
}

void cubiomes_call_map_ocean_mix(uint64_t world_seed, uint64_t biome_salt,
                                 int *out, int x, int z, int w, int h) {
    Layer biome;
    memset(&biome, 0, sizeof(biome));
    biome.getMap = mapContinent;
    biome.layerSalt = biome_salt;

    PerlinNoise noise;
    uint64_t s;
    setSeed(&s, world_seed);
    perlinInit(&noise, &s);

    Layer ocean_t;
    memset(&ocean_t, 0, sizeof(ocean_t));
    ocean_t.getMap = mapOceanTemp;
    ocean_t.noise = &noise;

    Layer mix;
    memset(&mix, 0, sizeof(mix));
    mix.getMap = mapOceanMix;
    mix.p = &biome;
    mix.p2 = &ocean_t;

    setLayerSeed(&mix, world_seed);
    mapOceanMix(&mix, out, x, z, w, h);
}

void cubiomes_call_map_river(uint64_t world_seed, int mc, uint64_t parent_salt,
                             uint64_t river_salt, int *out, int x, int z, int w,
                             int h) {
    Layer parent;
    memset(&parent, 0, sizeof(parent));
    parent.getMap = mapContinent;
    parent.layerSalt = parent_salt;
    parent.mc = mc;

    Layer river;
    memset(&river, 0, sizeof(river));
    river.getMap = mapRiver;
    river.layerSalt = river_salt;
    river.mc = mc;
    river.p = &parent;

    setLayerSeed(&river, world_seed);
    mapRiver(&river, out, x, z, w, h);
}

void cubiomes_call_map_smooth(uint64_t world_seed, int mc,
                              uint64_t parent_salt, uint64_t smooth_salt,
                              int *out, int x, int z, int w, int h) {
    Layer parent;
    memset(&parent, 0, sizeof(parent));
    parent.getMap = mapContinent;
    parent.layerSalt = parent_salt;
    parent.mc = mc;

    Layer smooth;
    memset(&smooth, 0, sizeof(smooth));
    smooth.getMap = mapSmooth;
    smooth.layerSalt = smooth_salt;
    smooth.mc = mc;
    smooth.p = &parent;

    setLayerSeed(&smooth, world_seed);
    mapSmooth(&smooth, out, x, z, w, h);
}

void cubiomes_call_map_river_mix(uint64_t world_seed, int mc,
                                 uint64_t biome_salt, uint64_t river_salt,
                                 uint64_t mix_salt, int *out, int x, int z,
                                 int w, int h) {
    Layer biome;
    memset(&biome, 0, sizeof(biome));
    biome.getMap = mapContinent;
    biome.layerSalt = biome_salt;
    biome.mc = mc;

    Layer river;
    memset(&river, 0, sizeof(river));
    river.getMap = mapContinent;
    river.layerSalt = river_salt;
    river.mc = mc;

    Layer mix;
    memset(&mix, 0, sizeof(mix));
    mix.getMap = mapRiverMix;
    mix.layerSalt = mix_salt;
    mix.mc = mc;
    mix.p = &biome;
    mix.p2 = &river;

    setLayerSeed(&mix, world_seed);
    mapRiverMix(&mix, out, x, z, w, h);
}

void cubiomes_call_map_hills(uint64_t world_seed, int mc,
                             uint64_t biome_parent_salt,
                             uint64_t river_parent_salt, uint64_t hills_salt,
                             int *out, int x, int z, int w, int h) {
    Layer biome_p;
    memset(&biome_p, 0, sizeof(biome_p));
    biome_p.getMap = mapContinent;
    biome_p.layerSalt = biome_parent_salt;
    biome_p.mc = mc;

    Layer river_p;
    memset(&river_p, 0, sizeof(river_p));
    river_p.getMap = mapContinent;
    river_p.layerSalt = river_parent_salt;
    river_p.mc = mc;

    Layer hills;
    memset(&hills, 0, sizeof(hills));
    hills.getMap = mapHills;
    hills.layerSalt = hills_salt;
    hills.mc = mc;
    hills.p = &biome_p;
    hills.p2 = &river_p;

    setLayerSeed(&hills, world_seed);
    mapHills(&hills, out, x, z, w, h);
}

void cubiomes_call_map_shore(uint64_t world_seed, int mc, uint64_t parent_salt,
                             uint64_t shore_salt, int *out, int x, int z, int w,
                             int h) {
    Layer parent;
    memset(&parent, 0, sizeof(parent));
    parent.getMap = mapContinent;
    parent.layerSalt = parent_salt;
    parent.mc = mc;

    Layer shore;
    memset(&shore, 0, sizeof(shore));
    shore.getMap = mapShore;
    shore.layerSalt = shore_salt;
    shore.mc = mc;
    shore.p = &parent;

    setLayerSeed(&shore, world_seed);
    mapShore(&shore, out, x, z, w, h);
}

void cubiomes_call_map_voronoi114(uint64_t world_seed, uint64_t parent_salt,
                                  uint64_t voronoi_salt, int *out, int x, int z,
                                  int w, int h) {
    Layer parent;
    memset(&parent, 0, sizeof(parent));
    parent.getMap = mapContinent;
    parent.layerSalt = parent_salt;

    Layer voronoi;
    memset(&voronoi, 0, sizeof(voronoi));
    voronoi.getMap = mapVoronoi114;
    voronoi.layerSalt = voronoi_salt;
    voronoi.p = &parent;

    setLayerSeed(&voronoi, world_seed);
    mapVoronoi114(&voronoi, out, x, z, w, h);
}

void cubiomes_call_map_ocean_temp(uint64_t world_seed, int *out, int x, int z,
                                  int w, int h) {
    Layer ot;
    memset(&ot, 0, sizeof(ot));
    ot.getMap = mapOceanTemp;
    PerlinNoise noise;
    uint64_t s;
    setSeed(&s, world_seed);
    perlinInit(&noise, &s);
    ot.noise = &noise;
    mapOceanTemp(&ot, out, x, z, w, h);
}

void cubiomes_call_map_biome(uint64_t world_seed, int mc,
                             uint64_t continent_salt, uint64_t snow_salt,
                             uint64_t biome_salt, int *out, int x, int z, int w,
                             int h) {
    Layer continent;
    memset(&continent, 0, sizeof(continent));
    continent.getMap = mapContinent;
    continent.layerSalt = continent_salt;
    continent.mc = mc;

    Layer snow;
    memset(&snow, 0, sizeof(snow));
    snow.getMap = mapSnow;
    snow.layerSalt = snow_salt;
    snow.mc = mc;
    snow.p = &continent;

    Layer biome;
    memset(&biome, 0, sizeof(biome));
    biome.getMap = mapBiome;
    biome.layerSalt = biome_salt;
    biome.mc = mc;
    biome.p = &snow;

    setLayerSeed(&biome, world_seed);
    mapBiome(&biome, out, x, z, w, h);
}

#include "finders.h"

/* Generate up to END_CITY_PIECES_MAX End City pieces for the given
 * (seed, chunkX, chunkZ). Writes the piece count via *out_count and
 * serialises each piece's bounding box into out_records (one
 * BBRecord per piece, in cubiomes' emission order). */
typedef struct {
    int32_t bb0_x, bb0_y, bb0_z;
    int32_t bb1_x, bb1_y, bb1_z;
    int32_t pos_x, pos_y, pos_z;
    int32_t rot;
    int32_t type;
} EndCityBBRecord;

void cubiomes_call_get_end_city_pieces(uint64_t seed, int chunk_x, int chunk_z,
                                       int *out_count,
                                       EndCityBBRecord *out_records) {
    Piece pieces[END_CITY_PIECES_MAX];
    int n = getEndCityPieces(pieces, seed, chunk_x, chunk_z);
    *out_count = n;
    for (int i = 0; i < n; i++) {
        out_records[i].bb0_x = pieces[i].bb0.x;
        out_records[i].bb0_y = pieces[i].bb0.y;
        out_records[i].bb0_z = pieces[i].bb0.z;
        out_records[i].bb1_x = pieces[i].bb1.x;
        out_records[i].bb1_y = pieces[i].bb1.y;
        out_records[i].bb1_z = pieces[i].bb1.z;
        out_records[i].pos_x = pieces[i].pos.x;
        out_records[i].pos_y = pieces[i].pos.y;
        out_records[i].pos_z = pieces[i].pos.z;
        out_records[i].rot = pieces[i].rot;
        out_records[i].type = pieces[i].type;
    }
}

/* Run cubiomes' getHouseList and copy its 9-entry output. */
uint64_t cubiomes_call_get_house_list(uint64_t seed, int chunk_x, int chunk_z,
                                      int *out_houses) {
    return getHouseList(out_houses, seed, chunk_x, chunk_z);
}

/* Generate the Nether-Fortress piece tree for the given seed/chunk.
 * Writes the piece count to *out_count and serialises each piece's
 * bounding box into out_records. Mirrors cubiomes' getFortressPieces. */
typedef struct {
    int32_t bb0_x, bb0_y, bb0_z;
    int32_t bb1_x, bb1_y, bb1_z;
    int32_t pos_x, pos_y, pos_z;
    int32_t rot;
    int32_t type;
} FortressBBRecord;

void cubiomes_call_get_fortress_pieces(int mc, uint64_t seed, int chunk_x,
                                       int chunk_z, int max_pieces,
                                       int *out_count,
                                       FortressBBRecord *out_records) {
    Piece *pieces = (Piece *)malloc(sizeof(Piece) * (size_t)max_pieces);
    int n = getFortressPieces(pieces, max_pieces, mc, seed, chunk_x, chunk_z);
    *out_count = n;
    for (int i = 0; i < n && i < max_pieces; i++) {
        out_records[i].bb0_x = pieces[i].bb0.x;
        out_records[i].bb0_y = pieces[i].bb0.y;
        out_records[i].bb0_z = pieces[i].bb0.z;
        out_records[i].bb1_x = pieces[i].bb1.x;
        out_records[i].bb1_y = pieces[i].bb1.y;
        out_records[i].bb1_z = pieces[i].bb1.z;
        out_records[i].pos_x = pieces[i].pos.x;
        out_records[i].pos_y = pieces[i].pos.y;
        out_records[i].pos_z = pieces[i].pos.z;
        out_records[i].rot = pieces[i].rot;
        out_records[i].type = pieces[i].type;
    }
    free(pieces);
}

/* Run cubiomes' isViableStructureTerrain and return its int result. */
int cubiomes_call_is_viable_structure_terrain(int struct_type, int mc,
                                              uint64_t seed, int x, int z) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, 0, seed); // overworld
    return isViableStructureTerrain(struct_type, &g, x, z);
}

/* Run cubiomes' isViableEndCityTerrain. Initialises a fresh generator
 * + SurfaceNoise per call. Returns the height (>= 60) or 0 if not viable. */
int cubiomes_call_is_viable_end_city_terrain(int mc, uint64_t seed, int x,
                                             int z) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, DIM_END, seed);
    SurfaceNoise sn;
    initSurfaceNoise(&sn, DIM_END, seed);
    return isViableEndCityTerrain(&g, &sn, x, z);
}

/* Debug helper: dump cubiomes' h00 + rotation for a given seed. */
extern int getSurfaceHeight(
        const double ncol00[], const double ncol01[],
        const double ncol10[], const double ncol11[],
        int colymin, int colymax, int blockspercell, double dx, double dz);
extern void sampleNoiseColumnEnd(double column[], const SurfaceNoise *sn,
        const EndNoise *en, int x, int z, int colymin, int colymax);

void cubiomes_call_debug_end_city_terrain(int mc, uint64_t seed, int x, int z,
                                          int *out_h00, int *out_h01,
                                          int *out_h10, int *out_h11,
                                          int *out_rot) {
    Generator g;
    setupGenerator(&g, mc, 0);
    applySeed(&g, DIM_END, seed);
    SurfaceNoise sn;
    initSurfaceNoise(&sn, DIM_END, seed);
    const EndNoise *en = &g.en;
    int chunkX = x >> 4;
    int chunkZ = z >> 4;
    int blockX = chunkX * 16 + 7;
    int blockZ = chunkZ * 16 + 7;
    int cellx = (blockX >> 3);
    int cellz = (blockZ >> 3);

    enum { y0 = 15, y1 = 18, yn = y1-y0+1 };
    double ncol[3][3][yn];

    sampleNoiseColumnEnd(ncol[0][0], &sn, en, cellx, cellz, y0, y1);
    sampleNoiseColumnEnd(ncol[0][1], &sn, en, cellx, cellz+1, y0, y1);
    sampleNoiseColumnEnd(ncol[1][0], &sn, en, cellx+1, cellz, y0, y1);
    sampleNoiseColumnEnd(ncol[1][1], &sn, en, cellx+1, cellz+1, y0, y1);

    *out_h00 = getSurfaceHeight(ncol[0][0], ncol[0][1], ncol[1][0], ncol[1][1],
            y0, y1, 4, (blockX & 7) / 8.0, (blockZ & 7) / 8.0);

    uint64_t cs;
    if (en->mc <= MC_1_18)
        setSeed(&cs, chunkX + chunkZ * 10387313ULL);
    else
        cs = chunkGenerateRnd(seed, chunkX, chunkZ);
    *out_rot = nextInt(&cs, 4);
    *out_h01 = 0;
    *out_h10 = 0;
    *out_h11 = 0;
}

/* Debug: verify cubiomes' nextInt(0, 4). */
int cubiomes_call_seed_zero_nextint4(void) {
    uint64_t cs;
    setSeed(&cs, 0);
    return nextInt(&cs, 4);
}

uint64_t cubiomes_call_get_shadow(uint64_t seed) { return getShadow(seed); }

int cubiomes_call_get_largest_rec(int target, const int *ids, int sx, int sz,
                                  int *p0x, int *p0z, int *p1x, int *p1z) {
    Pos p0 = {0, 0};
    Pos p1 = {0, 0};
    int area = getLargestRec(target, ids, sx, sz, &p0, &p1);
    *p0x = p0.x;
    *p0z = p0.z;
    *p1x = p1.x;
    *p1z = p1.z;
    return area;
}

int cubiomes_call_can_biome_generate(int layer_id, int mc, uint32_t flags, int id) {
    return canBiomeGenerate(layer_id, mc, flags, id);
}
