# Phase 5 — House Evolution Engine

> Status: **PENDING**

## Checklist

- [ ] House struct fields (population, level, service_access[] with timers)
- [ ] House tick function (service need decay / evaluation)
- [ ] Evolution thresholds (service combinations for upgrade/downgrade)
- [ ] Desirability calculation (nearby building contribution scores)

## House Struct Extensions

```c
// TBD — extends building_t with house-specific fields
```

## House Levels

| Level | Name | Requirements |
|-------|------|-------------|
| 0 | Small Tent | — |
| 1 | Large Tent | Water |
| ... | ... | ... |

_TBD — extract from binary + julius `house.c`_

## Evolution Tick Pseudocode

```c
// TBD
```

## Desirability Calculation

```c
// TBD
```

## Service Decay

_TBD — how walker visits reset service timers_
