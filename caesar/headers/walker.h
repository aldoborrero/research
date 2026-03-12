#ifndef CAESAR3_WALKER_H
#define CAESAR3_WALKER_H

/**
 * Caesar 3 — Reconstructed Walker (Figure) System Header
 *
 * Status: SKELETON — awaiting binary analysis
 *
 * Reference: julius/src/figure/figure.h
 * Expected max walker slots: ~1000-2000 (fixed array)
 */

#include <stdint.h>

/* Walker type enum — values TBD from binary analysis */
typedef enum {
    WALKER_NONE = 0,
    /* ... enumerate from binary + julius cross-reference ... */
    WALKER_TYPE_MAX
} walker_type_t;

/* Walker state enum */
typedef enum {
    WALKER_STATE_NONE = 0,
    WALKER_STATE_ALIVE,
    WALKER_STATE_DEAD,
    /* TBD */
} walker_state_t;

/* Walker action enum */
typedef enum {
    WALKER_ACTION_IDLE = 0,
    /* TBD */
} walker_action_t;

/**
 * Walker struct — reconstructed layout
 * Size: TBD bytes
 * Array base address: TBD
 */
typedef struct {
    uint8_t  in_use;            /* offset 0x00 — slot occupied flag */
    uint8_t  type;              /* walker_type_t */
    uint8_t  state;             /* walker_state_t */
    uint8_t  action;            /* walker_action_t */

    int16_t  x;                 /* tile x position */
    int16_t  y;                 /* tile y position */
    int16_t  destination_x;
    int16_t  destination_y;

    int16_t  source_building_id;
    int16_t  destination_building_id;

    uint16_t graphic_id;        /* sprite/animation index */
    uint8_t  direction;         /* 0-7 compass direction */
    uint8_t  speed;

    int16_t  action_timer;      /* countdown for current action */
    int16_t  wait_ticks;

    /* Path buffer — fixed size array for pre-computed route */
    uint8_t  path_length;
    uint8_t  path_current;
    uint8_t  path[/* TBD — likely 32 or 64 */];

    /* TBD: additional fields from binary analysis */
} walker_t;

/* Expected globals */
// walker_t walkers[MAX_WALKERS];   /* base addr TBD */
// int      walker_count;

#endif /* CAESAR3_WALKER_H */
