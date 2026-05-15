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
