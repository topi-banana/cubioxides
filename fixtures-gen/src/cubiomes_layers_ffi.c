/*
 * cubiomes_layers_ffi.c — wrappers around cubiomes' layer-map functions
 * so they can be exercised from Rust without exposing the full Layer
 * struct layout.
 *
 * Each wrapper builds a freshly zero-initialised `Layer`, sets only the
 * fields the corresponding map function reads, and forwards to cubiomes.
 */

#include "layers.h"
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
