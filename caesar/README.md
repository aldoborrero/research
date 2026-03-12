# Caesar 3 Reverse Engineering Project

Systematic decompilation, annotation, and reconstruction of the Caesar 3 (1998, Impressions Games) city-builder binary.

## Project Structure

```
caesar/
├── README.md                          ← This file
├── binary/                            ← Place caesar3.exe here
├── headers/                           ← Reconstructed C headers
│   ├── walker.h                       ← Walker/figure struct
│   └── building.h                     ← Building struct
├── output/                            ← Analysis outputs by phase
│   ├── phase1/recon_report.md         ← Binary reconnaissance
│   ├── phase2/main_loop.md            ← Entry point & main loop
│   ├── phase3/walker_system.md        ← Walker/entity system
│   ├── phase4/building_system.md      ← Building system
│   ├── phase5/house_evolution.md      ← House evolution engine
│   ├── phase6/map_system.md           ← Map & grid system
│   ├── phase7/economy_system.md       ← Economy & monthly tick
│   └── phase8/validation_report.md   ← Cross-reference validation
├── references/                        ← Open-source reimplementations
│   ├── julius/                        ← github.com/bvschaik/julius
│   └── augustus/                       ← github.com/Keriew/augustus
├── scripts/                           ← Analysis automation
│   ├── pe_recon.py                    ← PE header analysis
│   └── extract_strings.py            ← String extraction & classification
└── tools/                             ← Additional tooling
```

## Analysis Phases

| Phase | Topic | Output | Status |
|-------|-------|--------|--------|
| 1 | Binary Reconnaissance | [recon_report.md](output/phase1/recon_report.md) | PENDING |
| 2 | Entry Point & Main Loop | [main_loop.md](output/phase2/main_loop.md) | PENDING |
| 3 | Walker System | [walker_system.md](output/phase3/walker_system.md), [walker.h](headers/walker.h) | PENDING |
| 4 | Building System | [building_system.md](output/phase4/building_system.md), [building.h](headers/building.h) | PENDING |
| 5 | House Evolution | [house_evolution.md](output/phase5/house_evolution.md) | PENDING |
| 6 | Map & Grid | [map_system.md](output/phase6/map_system.md) | PENDING |
| 7 | Economy & Monthly Tick | [economy_system.md](output/phase7/economy_system.md) | PENDING |
| 8 | Cross-Reference Validation | [validation_report.md](output/phase8/validation_report.md) | PENDING |

## System Dependency Graph

```
                    ┌─────────────┐
                    │  Main Loop  │
                    │  (Phase 2)  │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ Walkers  │ │Buildings │ │ Economy  │
        │(Phase 3) │ │(Phase 4) │ │(Phase 7) │
        └────┬─────┘ └────┬─────┘ └────┬─────┘
             │            │            │
             │     ┌──────┴──────┐     │
             │     │             │     │
             ▼     ▼             ▼     ▼
        ┌──────────┐       ┌──────────┐
        │   Map    │       │  Houses  │
        │(Phase 6) │       │(Phase 5) │
        └──────────┘       └──────────┘
```

## Reference Implementations

- **julius** — Faithful open-source re-implementation of Caesar 3. Primary cross-reference source.
- **augustus** — Fork of julius with gameplay enhancements. Most accurate reconstruction available; use for validation.

## Tooling

| Tool | Status | Purpose |
|------|--------|---------|
| Python 3.11 | Available | Script execution |
| pefile | Installing | PE header parsing |
| capstone | Installing | Disassembly engine |
| radare2 | Not found | Interactive disassembler |
| Ghidra | Not found | Decompiler |

## Quick Start

```bash
# 1. Place the binary
cp /path/to/caesar3.exe caesar/binary/

# 2. Run PE reconnaissance
python3 scripts/pe_recon.py binary/caesar3.exe > output/phase1/recon_report.md

# 3. Extract and classify strings
python3 scripts/extract_strings.py binary/caesar3.exe > output/phase1/strings.txt
```

## Notes

- The original binary is a 32-bit Windows PE executable, likely compiled with MSVC
- Almost certainly plain C (not C++)
- Expected max walker array: ~1000-2000 fixed slots
- Map grid: likely 162x162 tiles
- Tick rate: ~25-50ms intervals
