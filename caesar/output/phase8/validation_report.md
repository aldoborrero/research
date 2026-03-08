# Phase 8 — Cross-Reference Validation Report

> Status: **PENDING**

## Methodology

For every major struct and function discovered in the binary:
1. Search julius/augustus codebase for equivalent symbols
2. Compare field offsets, enum values, function logic
3. Flag discrepancies
4. Note systems reimplemented differently

## Validation Matrix

| System | Binary Addr | Julius Equivalent | Match | Notes |
|--------|-------------|-------------------|-------|-------|
| Walker struct | _TBD_ | `src/figure/figure.h` | — | |
| Building struct | _TBD_ | `src/building/building.h` | — | |
| House evolution | _TBD_ | `src/building/house.c` | — | |
| Map grid | _TBD_ | `src/map/` | — | |
| Economy | _TBD_ | `src/city/` | — | |
| Pathfinding | _TBD_ | `src/figure/route.c` | — | |

## Discrepancies

_TBD_

## Systems Present in Binary but Reimplemented Differently

_TBD_
