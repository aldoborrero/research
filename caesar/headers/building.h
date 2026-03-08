#ifndef CAESAR3_BUILDING_H
#define CAESAR3_BUILDING_H

/**
 * Caesar 3 — Reconstructed Building System Header
 *
 * Status: SKELETON — awaiting binary analysis
 *
 * Reference: julius/src/building/building.h
 */

#include <stdint.h>

/* Building type enum — values TBD */
typedef enum {
    BUILDING_NONE = 0,
    /* ... enumerate from binary + julius cross-reference ... */
    BUILDING_TYPE_MAX
} building_type_t;

/* Building state */
typedef enum {
    BUILDING_STATE_UNUSED = 0,
    BUILDING_STATE_IN_USE,
    BUILDING_STATE_UNDO,
    BUILDING_STATE_CREATED,
    BUILDING_STATE_RUBBLE,
    BUILDING_STATE_DELETED_BY_GAME,
    BUILDING_STATE_DELETED_BY_PLAYER,
    /* TBD */
} building_state_t;

/**
 * Building struct — reconstructed layout
 * Size: TBD bytes
 */
typedef struct {
    uint8_t  state;             /* building_state_t */
    uint8_t  type;              /* building_type_t (may be uint16) */
    uint8_t  size;              /* footprint size (1-5 tiles) */
    uint8_t  _pad0;

    int16_t  x;                 /* grid x of upper-left corner */
    int16_t  y;                 /* grid y of upper-left corner */
    int16_t  grid_offset;       /* linear index into map grid */

    int16_t  num_workers;       /* current workers assigned */
    int16_t  max_workers;       /* worker capacity */

    int16_t  walker_id;         /* ID of spawned walker */
    int16_t  walker_spawn_timer;

    int16_t  production_timer;
    int16_t  production_resource;

    /* Inventory — up to N good slots */
    int16_t  inventory[/* TBD — likely 8 or 16 */];

    uint16_t graphic_id;
    uint8_t  fire_risk;
    uint8_t  damage_risk;

    /* TBD: many more fields from binary analysis */
} building_t;

#endif /* CAESAR3_BUILDING_H */
