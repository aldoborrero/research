# Phase 6 — Map & Grid System

> Status: **PENDING**

## Checklist

- [ ] Map dimensions and tile struct layout
- [ ] Road connectivity encoding (bitmask?)
- [ ] Desirability grid (separate overlay or embedded?)
- [ ] Water/entertainment/religion access overlays
- [ ] Building footprint stamping

## Map Dimensions

Expected: 162x162 tiles (with border padding) — verify from binary constants.

## Tile Struct

```c
// TBD
```

## Grid Overlays

| Overlay | Storage | Notes |
|---------|---------|-------|
| Terrain | _TBD_ | Base terrain type |
| Road connectivity | _TBD_ | Bitmask per direction? |
| Desirability | _TBD_ | Separate grid? |
| Water access | _TBD_ | Timer-based |
| Fire risk | _TBD_ | — |
| Damage risk | _TBD_ | — |

## Building Footprint Stamping

```c
// TBD
```
