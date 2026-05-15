/*
 * cubiomes_layers_ffi.c — wrappers around cubiomes' layer-map functions
 * so they can be exercised from Rust without exposing the full Layer
 * struct layout.
 *
 * Each wrapper builds a freshly zero-initialised `Layer`, sets only the
 * fields the corresponding map function reads, and forwards to cubiomes.
 */

#include "layers.h"
#include "noise.h"
#include <string.h>

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
