# Caesar III Walker System — Rust Port Plan

## Source Analysis Summary

The Julius project (`bvschaik/julius`) is a faithful C reimplementation of Caesar III. The walker/figure system is the core simulation engine — every moving entity in the game (citizens, soldiers, traders, animals, projectiles) is a "figure."

### What We're Porting

| Component | C Source | Lines | Complexity |
|-----------|----------|-------|------------|
| Figure struct & lifecycle | `src/figure/figure.c/.h` | ~230 | Medium |
| Figure types enum | `src/figure/type.h` | ~90 | Low |
| Action dispatch | `src/figure/action.c/.h` | ~200 | Medium |
| Movement & tick | `src/figure/movement.c` | ~354 | High |
| Routing & pathfinding | `src/figure/route.c`, `src/map/routing_path.c` | ~500 | High |
| Combat | `src/figure/combat.c` | ~432 | Medium |
| Formations | `src/figure/formation.c` | ~750 | High |
| Service delivery | `src/figure/service.c` | ~200 | Low |
| Type behaviors (15+ files) | `src/figuretype/*.c` | ~5000 | High (volume) |
| Building integration | `src/building/figure.c` | ~500 | Medium |

**Total: ~8,200 lines of C → estimated ~6,000-7,000 lines of Rust** (enums compress the type dispatching significantly).

---

## Architecture: C vs Rust

### C Architecture (Julius)

```
Global static array: figure figures[1000]
                          │
    ┌─────────────────────┼─────────────────────┐
    │                     │                     │
figure_action_handle()    │              map grid spatial index
  iterates 1-999          │              (linked list per tile)
  dispatches via           │
  callback[f->type](f)    │
                          │
              figure_movement_*()
              figure_route_*()
              figure_combat_*()
```

**Problems this design has:**
- Monolithic 117-field struct for ALL entity types (wasteful, error-prone)
- Global mutable state everywhere
- Linear scan for free slots on creation
- No lifetime safety on figure ID references (stale IDs cause bugs)
- Type-specific fields pollute the shared struct

### Proposed Rust Architecture

```
SlotMap<FigureId, Figure>           // Generational arena, O(1) access
         │
    ┌────┴────┐
    │         │
  Figure {   FigureKind enum {
    common     Immigrant { ... },
    fields     CartPusher { resource, dest, ... },
    ...        Soldier { formation, index, ... },
               MarketTrader { ... },
               Enemy { variant, ... },
               ...
  }          }
         │
    ActionState enum per kind
         │
    Tick system: fn tick_figures(&mut World)
      → collect commands (reads)
      → apply commands (writes)
```

---

## Phase 1: Core Data Model

### 1.1 Figure Identity & Storage

```rust
use slotmap::{SlotMap, new_key_type};

new_key_type! { pub struct FigureId; }

pub struct FigurePool {
    figures: SlotMap<FigureId, Figure>,
    /// Spatial index: grid_offset → SmallVec of FigureIds
    grid: HashMap<u32, SmallVec<[FigureId; 4]>>,
}
```

**Why SlotMap over the C array:**
- Generational IDs prevent stale-reference bugs (C's `created_sequence` hack becomes free)
- O(1) insert/remove without linear scan
- Dense iteration (no skipping dead slots)
- IDs are `Copy` — safe to pass around without borrowing issues

### 1.2 Figure Struct — Split by Kind

Instead of one 117-field struct, split into common fields + per-type data:

```rust
pub struct Figure {
    // Identity
    pub id: FigureId,
    pub faction: Faction,

    // Position & movement (shared by all)
    pub pos: TilePos,
    pub previous_pos: TilePos,
    pub cross_country: CrossCountryPos,
    pub direction: Direction,
    pub progress_on_tile: u8,  // 0-15
    pub speed_multiplier: u8,
    pub terrain_usage: TerrainUsage,

    // Routing
    pub route: Option<Route>,
    pub roam: RoamState,

    // Visual
    pub image_id: u16,
    pub image_offset: u8,
    pub cart_image_id: u16,
    pub is_ghost: bool,

    // Combat (optional, but kept inline for cache)
    pub combat: CombatState,

    // The discriminated union — type-specific data + action state
    pub kind: FigureKind,
}
```

### 1.3 FigureKind Enum

```rust
pub enum FigureKind {
    Immigrant {
        action: ImmigrantAction,
        building_id: Option<BuildingId>,
        num_people: u8,
    },
    Emigrant {
        action: EmigrantAction,
    },
    CartPusher {
        action: CartPusherAction,
        building_id: BuildingId,
        destination_building_id: Option<BuildingId>,
        resource: ResourceType,
        loads: u8,
    },
    Soldier {
        action: SoldierAction,
        formation_id: FormationId,
        index_in_formation: u8,
        formation_pos: (i8, i8),
        soldier_type: SoldierType,
    },
    MarketTrader {
        action: MarketTraderAction,
        building_id: BuildingId,
    },
    MarketBuyer {
        action: MarketBuyerAction,
        building_id: BuildingId,
        collecting_item: Option<ResourceType>,
    },
    Prefect {
        action: PrefectAction,
        building_id: BuildingId,
    },
    TradeCaravan {
        action: TradeCaravanAction,
        trader_id: TraderId,
        empire_city_id: u8,
    },
    TradeShip {
        action: TradeShipAction,
        trader_id: TraderId,
    },
    // ... ~60 more variants for all figure types
    // Many service walkers share the same shape:
    ServiceWalker {
        action: ServiceAction,
        service_type: ServiceType,
        building_id: BuildingId,
    },
    Enemy {
        action: EnemyAction,
        variant: EnemyType,
        formation_id: FormationId,
    },
    Animal {
        action: AnimalAction,
        animal_type: AnimalType,
    },
    Missile {
        action: MissileAction,
        shooter_id: Option<FigureId>,
        damage: u8,
    },
}
```

**Key insight:** Many of the 73 C figure types share identical structure (all service walkers are roam+return). We collapse them into `ServiceWalker { service_type }` with a `ServiceType` enum. This reduces ~30 C types to one Rust variant.

### 1.4 Action State Enums (per kind)

Each figure kind gets its own action enum instead of magic numbers:

```rust
pub enum ImmigrantAction {
    Created { wait_ticks: u8 },
    Arriving,
    EnteringHouse,
}

pub enum CartPusherAction {
    Initial,
    DeliveringToWarehouse,
    DeliveringToGranary,
    DeliveringToWorkshop,
    AtWarehouse,
    AtGranary,
    AtWorkshop,
    Returning,
}

pub enum SoldierAction {
    AtRest,
    GoingToStandard,
    AtStandard,
    MoppingUp,
    GoingToDistantBattle,
    AtDistantBattle,
    ReturningFromDistantBattle,
}

pub enum ServiceAction {
    Roaming { roam_ticks: u16 },
    Returning,
}
```

---

## Phase 2: Movement & Routing

### 2.1 Movement System

```rust
pub struct TilePos {
    pub x: u8,
    pub y: u8,
    pub grid_offset: u16,  // precomputed y * MAP_WIDTH + x
}

pub struct CrossCountryPos {
    pub x: i16,  // 15 * tile_x
    pub y: i16,
    pub dest_x: i16,
    pub dest_y: i16,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_xy: i16,
    pub direction: CrossCountryDirection,
}

/// Movement result tells the caller what happened
pub enum MoveResult {
    InProgress,
    ArrivedAtTile(TilePos),
    ArrivedAtDestination,
    Blocked,
    Reroute,
    AttackTriggered(FigureId),
}

pub fn advance_figure_movement(
    figure: &mut Figure,
    grid: &MapGrid,
    speed: u8,
) -> MoveResult { ... }
```

### 2.2 Routing

```rust
pub struct Route {
    pub directions: SmallVec<[Direction; 64]>,  // path as direction sequence
    pub current_index: u16,
}

pub fn calculate_route(
    from: TilePos,
    to: TilePos,
    terrain_usage: TerrainUsage,
    grid: &MapGrid,
) -> Option<Route> { ... }
```

Port the distance-field pathfinding from `map/routing_path.c`:
- Pre-compute distance field from destination
- Trace back from source following decreasing distances
- Max path length: 500 tiles
- Support terrain modes: roads-only, prefer-roads, any, walls, enemy, animal

### 2.3 Roaming

```rust
pub struct RoamState {
    pub length: u16,
    pub max_length: u16,
    pub choose_destination: bool,
    pub random_counter: u8,
    pub turn_direction: i8,
    pub ticks_until_next_turn: i8,
}

pub fn advance_roaming(
    figure: &mut Figure,
    grid: &MapGrid,
) -> RoamResult { ... }
```

---

## Phase 3: Action/Behavior System

### 3.1 Two-Phase Tick Architecture

The C code mutates global state during iteration (unsafe pattern). Rust port uses command buffering:

```rust
pub enum FigureCommand {
    Move { id: FigureId, direction: Direction },
    SetAction { id: FigureId, action: ActionTransition },
    SpawnFigure { kind: FigureKind, pos: TilePos, dir: Direction },
    KillFigure { id: FigureId },
    DamageBuilding { building_id: BuildingId, amount: u8 },
    ProvideService { building_id: BuildingId, service: ServiceType, amount: u8 },
    TransferResource { from: BuildingId, to: BuildingId, resource: ResourceType, amount: u8 },
    StartCombat { attacker: FigureId, defender: FigureId },
    UpdateFormation { formation_id: FormationId, update: FormationUpdate },
}

pub fn tick_figures(world: &World) -> Vec<FigureCommand> {
    let mut commands = Vec::new();
    for (id, figure) in world.figures.iter() {
        match &figure.kind {
            FigureKind::Immigrant { action, .. } => {
                tick_immigrant(id, figure, world, &mut commands);
            }
            FigureKind::CartPusher { action, .. } => {
                tick_cart_pusher(id, figure, world, &mut commands);
            }
            FigureKind::Soldier { action, .. } => {
                tick_soldier(id, figure, world, &mut commands);
            }
            // ...
        }
    }
    commands
}

pub fn apply_commands(world: &mut World, commands: Vec<FigureCommand>) {
    for cmd in commands {
        match cmd {
            FigureCommand::KillFigure { id } => { world.figures.remove(id); }
            FigureCommand::SpawnFigure { kind, pos, dir } => { ... }
            // ...
        }
    }
}
```

### 3.2 Per-Type Tick Functions

Each figure type gets a pure-ish function:

```rust
fn tick_immigrant(
    id: FigureId,
    figure: &Figure,
    world: &World,
    commands: &mut Vec<FigureCommand>,
) {
    let FigureKind::Immigrant { action, building_id, num_people } = &figure.kind else {
        unreachable!()
    };
    match action {
        ImmigrantAction::Created { wait_ticks } => {
            if *wait_ticks == 0 {
                commands.push(FigureCommand::SetAction {
                    id,
                    action: ActionTransition::ImmigrantStartArriving,
                });
            }
        }
        ImmigrantAction::Arriving => {
            // Check if building still valid, move toward it
            // ...
        }
        ImmigrantAction::EnteringHouse => {
            // Cross-country movement into building
            // On arrival: add population, kill figure
        }
    }
}
```

---

## Phase 4: Combat & Formations

### 4.1 Combat

```rust
pub struct CombatState {
    pub damage: u8,
    pub num_attackers: u8,
    pub attacker_ids: [Option<FigureId>; 2],
    pub opponent_id: Option<FigureId>,
    pub target_id: Option<FigureId>,
    pub targeted_by: Option<FigureId>,
    pub action_before_attack: Option<Box<FigureKind>>,  // or a saved action tag
}

pub fn resolve_combat(
    attacker: &Figure,
    defender: &Figure,
    formations: &FormationPool,
) -> CombatResult {
    let attack = figure_properties(attacker).attack;
    let defense = figure_properties(defender).defense;
    let mut net = attack as i16 - defense as i16;

    // Back-attack bonus
    if !defender.combat.targeted_by.is_some() { net += 4; }

    // Formation bonuses
    if let Some(fid) = defender_formation_id(defender) {
        net -= formation_defense_bonus(formations, fid);
    }

    // ... resolve hit/miss/kill
}
```

### 4.2 Formations

```rust
new_key_type! { pub struct FormationId; }

pub struct Formation {
    pub id: FormationId,
    pub figure_type: FigureType,
    pub layout: FormationLayout,
    pub morale: u8,       // 0-100
    pub position: TilePos,
    pub members: SmallVec<[FigureId; 16]>,
    pub is_at_fort: bool,
    pub months_without_combat: u8,
    pub cursed_by_mars: bool,
}

pub enum FormationLayout {
    DoubleLine,
    Column,
    FishFormation,
    MopUp,
    AtRest,
    // enemy-specific layouts
    Tortoise,
    Enemy12,
}
```

---

## Phase 5: Service Delivery & Building Integration

### 5.1 Service Coverage

```rust
pub fn provide_service_coverage(
    figure: &Figure,
    buildings: &BuildingPool,
    grid: &MapGrid,
) -> Vec<FigureCommand> {
    // For each building in ~2-4 tile radius of figure's current position
    // Provide service points (capped at 96)
    // Service type determined by figure's ServiceType
}
```

Service types map directly from C:
- Entertainment (theater, amphitheater, colosseum, hippodrome)
- Education (school, academy, library)
- Health (doctor, barber, bathhouse, hospital)
- Religion (5 god temples)
- Maintenance (prefect, engineer)
- Market access, labor access, tax collection

### 5.2 Building Figure Spawning

```rust
/// Called once per 50-tick cycle (at tick 31)
pub fn building_generate_figures(
    buildings: &BuildingPool,
    figures: &FigurePool,
) -> Vec<FigureCommand> {
    let mut commands = Vec::new();
    for (bid, building) in buildings.iter() {
        if !building.has_road_access || building.num_workers == 0 {
            continue;
        }
        if building.figure_id.is_some() {
            continue; // already has an active figure
        }
        match building.kind {
            BuildingKind::Market => {
                commands.push(FigureCommand::SpawnFigure {
                    kind: FigureKind::MarketTrader { ... },
                    pos: building.road_access_pos,
                    dir: building.figure_spawn_direction(),
                });
            }
            // ... other building types
        }
    }
    commands
}
```

---

## Phase 6: Serialization (Save/Load)

Port the figure serialization for save-game compatibility:

```rust
pub fn serialize_figure(f: &Figure, buf: &mut Vec<u8>) {
    // Write 128 bytes per figure matching C layout for save compatibility
    // Map FigureKind variants back to C type IDs
    // Map action enums back to C action_state numbers
}

pub fn deserialize_figure(buf: &[u8]) -> Figure {
    // Read C-format figure data
    // Map type ID → FigureKind variant
    // Map action_state number → per-type action enum
}
```

**Decision point:** Do we need save-game compatibility with original Caesar III / Julius? If yes, maintain the C struct layout for serialization. If no, use `serde` with a versioned format.

---

## Implementation Order

### Sprint 1: Foundation (est. ~2 weeks)
1. `FigureId` + `SlotMap` pool
2. `Figure` struct with `FigureKind` enum (start with 5 types: Immigrant, CartPusher, ServiceWalker, Prefect, MarketTrader)
3. `TilePos`, `Direction`, `TerrainUsage` types
4. Basic movement: `progress_on_tile` advancement, tile transitions
5. Spatial grid index (grid_offset → FigureId list)
6. Unit tests for creation, deletion, movement

### Sprint 2: Routing & Roaming (~2 weeks)
1. Distance-field pathfinding (`calculate_route`)
2. Route storage and following
3. Roaming system (random walk on roads)
4. Cross-country movement (Bresenham-style)
5. Integration test: spawn a ServiceWalker, watch it roam and return

### Sprint 3: Action System (~2 weeks)
1. Two-phase tick loop (collect commands → apply)
2. Port 5 initial figure type behaviors:
   - `tick_immigrant` (3 states)
   - `tick_cart_pusher` (8 states)
   - `tick_service_walker` (2 states — roam/return)
   - `tick_prefect` (roam + fire/enemy response)
   - `tick_market_trader` (roam variant)
3. Building-figure spawning
4. Service delivery coverage

### Sprint 4: Combat & Military (~2 weeks)
1. `Formation` struct and `FormationPool`
2. `CombatState` and combat resolution
3. `tick_soldier` (9 states)
4. Enemy figure types and AI
5. Missile/projectile figures

### Sprint 5: Remaining Figure Types (~2 weeks)
1. Trade caravan + trade ship (complex multi-state)
2. All remaining service walkers (~20 types, mostly identical)
3. Animals, flotsam, special figures
4. Docker, fishing boat, ferry

### Sprint 6: Serialization & Integration (~1 week)
1. Save/load serialization
2. Integration with game tick loop
3. Integration with building system
4. Full simulation test: run 1000 ticks, verify figure counts and positions

---

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Storage | `SlotMap` | Generational IDs prevent stale refs, dense iteration, O(1) ops |
| Type modeling | `FigureKind` enum | Compiler-enforced exhaustive matching, no wasted fields |
| Mutation model | Command buffer | Avoids borrow checker fights, enables parallelism later |
| Action states | Per-type enums | Type-safe, self-documenting, impossible invalid states |
| Spatial index | `HashMap<u32, SmallVec>` | Simple, good enough for 162×162 grid |
| Service walkers | Collapsed to one variant | 30+ C types share identical structure |
| Path storage | `SmallVec<[Direction; 64]>` inline | Most paths are short, avoids allocation |

---

## Risk Areas

1. **Behavior fidelity**: Each of the ~60 figure type handlers has subtle logic. Must port exactly or simulation diverges. Mitigation: snapshot-based testing against Julius.
2. **Command buffer ordering**: Some C behaviors depend on mutation order within a tick. May need ordered command application or priority system.
3. **Formation system complexity**: 750 lines of formation logic with many edge cases. Port carefully with extensive tests.
4. **Save compatibility**: If targeting Caesar III save-file compatibility, the serialization layer must perfectly match the C struct layout byte-for-byte.

---

## File Structure

```
src/
├── figure/
│   ├── mod.rs           // FigureId, Figure, FigureKind, FigurePool
│   ├── movement.rs      // advance_figure_movement, cross_country
│   ├── route.rs         // calculate_route, Route
│   ├── roam.rs          // RoamState, advance_roaming
│   ├── combat.rs        // CombatState, resolve_combat
│   ├── service.rs       // provide_service_coverage
│   ├── formation.rs     // Formation, FormationPool, FormationLayout
│   ├── tick.rs          // tick_figures, apply_commands, FigureCommand
│   └── serialize.rs     // save/load
├── figuretype/
│   ├── mod.rs
│   ├── migrant.rs       // tick_immigrant, tick_emigrant, tick_homeless
│   ├── cartpusher.rs    // tick_cart_pusher
│   ├── soldier.rs       // tick_soldier
│   ├── trader.rs        // tick_trade_caravan, tick_trade_ship
│   ├── market.rs        // tick_market_trader, tick_market_buyer
│   ├── maintenance.rs   // tick_prefect, tick_engineer
│   ├── service.rs       // tick_service_walker (generic for ~20 types)
│   ├── enemy.rs         // tick_enemy (by variant)
│   ├── animal.rs        // tick_animal
│   └── missile.rs       // tick_missile
└── map/
    ├── grid.rs          // MapGrid, spatial queries
    └── routing.rs       // distance-field pathfinding
```
