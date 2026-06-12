# Architecture — MUD Game Engine

## Overview

A modern DIKU-style MUD engine written in Rust. Event-driven, ECS-based, terminal-first
with extensible protocol support.

**Stack:** Rust + Tokio + hecs (ECS) + rusqlite + Rhai (scripting)

**Design Philosophy:** Driver/mudlib separation (inspired by LPMud, Evennia).
The engine provides networking, ECS, persistence, and scripting — game content
(combat, spells, quests) lives in data files and scripts, not in engine code.

---

## Cargo Workspace

```
mud/
├── Cargo.toml              # workspace root
├── core/                   # ECS components, systems, events, resources
│   ├── Cargo.toml
│   └── src/
│       ├── components/     # hecs Component types
│       ├── systems/        # Game systems (movement, combat, regen)
│       ├── resources/      # Singleton resources (world state, ticker)
│       ├── events.rs       # Event types
│       └── lib.rs
├── server/                 # Network layer
│   ├── Cargo.toml
│   └── src/
│       ├── telnet/         # Telnet protocol negotiation (NAWS, MCCP, GMCP, MXP)
│       ├── cmd/            # Command dispatch (trie-based parser)
│       ├── connection.rs   # Per-client state
│       └── lib.rs
├── data/                   # Persistence layer
│   ├── Cargo.toml
│   └── src/
│       ├── schema.rs       # SQLite schema & migrations
│       ├── queries.rs      # Type-safe query wrappers
│       └── lib.rs
├── scripting/              # Embedded scripting
│   ├── Cargo.toml
│   └── src/
│       ├── engine.rs       # Rhai engine setup
│       ├── bindings.rs     # Rust ↔ Rhai bindings
│       └── lib.rs
├── content/                # Game data files (TOML)
│   ├── areas/              # Room graphs
│   ├── mobs/               # NPC templates
│   ├── items/              # Item templates
│   └── scripts/            # Rhai script files
└── bin/                    # Binary entrypoint
    ├── Cargo.toml
    └── src/main.rs
```

---

## Game Loop

Not a fixed tick loop. Event-driven with subscription-based timing.

```rust
// Main Tokio loop (pseudocode)
loop {
    select! {
        // Player input
        line = socket.read_line() => parse_and_execute_command(line),

        // Scheduled system pulse
        pulse = scheduler.next_tick() => run_system_phase(pulse.kind),

        // Internal events (combat, triggers)
        event = event_bus.recv() => dispatch_event(event),

        // Graceful shutdown
        _ = shutdown_signal() => save_and_exit(),
    }
}
```

**Pulse system:** Systems register for intervals — combat (2s), regen (6s), weather (5m).
No global heartbeat. Most of the time the server is idle, waiting for input.

---

## ECS Component Design

### Spatial

```rust
struct Position { room: Entity }
struct Room { name: String, description: String }
struct Exit { direction: Direction, dest: Entity, flags: ExitFlags }
enum Direction { North, South, East, West, Up, Down, Northeast, Northwest, Southeast, Southwest }
```

### Character

```rust
struct Player { account_id: i64 }
struct Npc { template_id: String }
struct Attributes { str: u8, dex: u8, int: u8, wis: u8, con: u8, cha: u8 }
struct Health { current: i32, max: i32 }
struct Level(u8);
struct Experience(u64);
```

### Combat

```rust
struct CombatTarget(Entity);
struct Damage(i32);
struct Armor { base: i32, bonus: i32 }
```

### Items

```rust
struct Item { template_id: String, flags: ItemFlags }
struct Inventory(Vec<Entity>);
enum EquipmentSlot { Head, Neck, Torso, Arms, Hands, Finger, Legs, Feet, Weapon, Shield }
```

### Flexible / OLC

```rust
struct Attributes(HashMap<String, String>);  // KV store for builder-defined data
```

### Persistence

```rust
struct Dirty;    // Marker: entity needs DB write
struct DbId(i64); // Maps entity to SQLite row
```

---

## Command System

Commands are prefix-matched via a trie. Each command:

```rust
struct Command {
    name: &'static str,
    aliases: &[&'static str],
    access: AccessLevel,
    handler: fn(&mut World, &mut Connection, args: &str) -> CommandResult,
}
```

**Built-in commands:**

| Command | Description |
|---|---|
| `look` / `l` | Examine room or target |
| `n` / `s` / `e` / `w` / `u` / `d` / `ne` / ... | Movement |
| `say` | Speak in room |
| `tell` / `whisper` | Private message |
| `shout` | Broadcast to zone |
| `kill` | Initiate combat |
| `get` / `drop` | Item manipulation |
| `inventory` / `i` | List carried items |
| `equipment` / `eq` | List worn items |
| `wear` / `wield` / `remove` | Equipment management |
| `help` | Online help |
| `who` | List players |
| `@dig` | Builder: create room |
| `@link` | Builder: connect rooms |
| `@set` | Builder: modify attributes |
| `@load` | Builder: spawn mobs/items |
| `@teleport` | Admin: move players |
| `@force` | Admin: force command |

---

## Event System

Internal events decouple systems from each other:

```rust
enum GameEvent {
    PlayerSaid { speaker: Entity, message: String },
    PlayerMoved { player: Entity, from: Entity, to: Entity },
    PlayerAttacked { attacker: Entity, target: Entity },
    PlayerDied { victim: Entity, killer: Option<Entity> },
    MobDied { mob: Entity, killer: Entity },
    ItemPickedUp { player: Entity, item: Entity },
    ItemDropped { player: Entity, item: Entity },
    RoomEntered { actor: Entity, room: Entity },
    ScriptTrigger { entity: Entity, trigger: TriggerType },
}
```

Events are published to a broadcast channel. Systems and scripts subscribe
to relevant event types.

---

## Telnet Protocol

**Phase 0 (MVP):** Line mode, local echo, ANSI color codes (`\x1b[31m`).

**Later phases:**

| Feature | Description |
|---|---|
| **NAWS** | Client window size negotiation |
| **MCCP** | ZLIB compression (5-10x bandwidth reduction) |
| **GMCP** | Structured data (maps, gauges, room info) |
| **MXP** | Rich text, clickable links |
| **MSSP** | Server listing protocol |

Commands are transport-agnostic. A `Connection` trait abstracts telnet,
WebSocket, and future transports:

```rust
trait Connection: Send {
    fn send(&mut self, text: &str);
    fn supports(&self, feature: Feature) -> bool;
    fn id(&self) -> u64;
}
```

---

## Persistence

**Two-tier:** In-memory ECS world ↔ SQLite on disk.

- **Load:** Read all entities from SQLite into ECS on startup
- **Dirty tracking:** Mutated entities get a `Dirty` marker component
- **Flush:** Background writer persists dirty entities every 5s
- **Shutdown:** Full flush + WAL checkpoint
- **Crash safety:** WAL mode + critical-state snapshots

**Schema pattern:** SQLite tables mirror component types:

```sql
CREATE TABLE entities (
    id INTEGER PRIMARY KEY,
    type TEXT NOT NULL
);

CREATE TABLE components_position (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    room_id INTEGER NOT NULL REFERENCES entities(id)
);

CREATE TABLE components_health (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    current INTEGER NOT NULL,
    max INTEGER NOT NULL
);

CREATE TABLE attributes (
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (entity_id, key)
);
```

---

## Scripting & OLC

**Rhai** is embedded for dynamic content:

```rust
// goblin_guard.rhai — attached to a goblin NPC
on("death", |ctx| {
    let room = ctx.entity().room();
    room.echo("The goblin guard crumples to the ground.");
    room.spawn_item("rusty_key", 1);
});

on("enter", |ctx| {
    if ctx.actor().has_flag("sneaking") {
        ctx.entity().say("Did I hear something?");
    }
});
```

**OLC commands** allow builders to modify the world in-game without restart:

```
@dig forest/clearing
@link forest/clearing e forest/path_1
@set forest/clearing.flags = "peaceful"
@load forest/clearing deer = 3
```

**Content files** (TOML) define templates loaded at startup:

```toml
[templates.room.forest_clearing]
name = "A Sunlit Clearing"
description = "Golden light filters through the canopy..."
exits = [
    { direction = "east", target = "forest/path_1" }
]

[templates.mob.deer]
name = "a deer"
description = "A graceful deer grazing peacefully."
faction = "neutral"
attributes = { str = 5, dex = 16, int = 4 }
health = { max = 15 }
script = "animals/deer.rhai"
```

---

## Protocol Expansion Path

```
Phase 0:  Telnet (TCP/23) — line mode, ANSI colors
Phase 6:  WebSocket bridge (Axum/Warp) — web clients
Phase 6:  MXP + GMCP — rich desktop clients (Mudlet, MUSHclient)
Phase 6:  REST API — companion apps, mobile
```

---

## Development Phases

### Phase 0 — Foundation
- [ ] Cargo workspace & crate skeleton
- [ ] Core types (`Room`, `Exit`, `Direction`, entity management)
- [ ] Tokio TCP listener with telnet negotiation
- [ ] Basic ECS world with `hecs`
- [ ] Raw line-in/line-out to connected players

### Phase 1 — World & Movement
- [ ] Room graph with exits
- [ ] `Position` component
- [ ] `look`, `north/s/e/w`, `say` commands
- [ ] ANSI color support

### Phase 2 — Character System
- [ ] Account creation & login
- [ ] SQLite schema & persistence
- [ ] Character creation (name, race, class)
- [ ] `Attributes`, `Level`, `Experience` components
- [ ] Skills/spells scaffold

### Phase 3 — Combat & Equipment
- [ ] `Health`, `Damage` components
- [ ] Combat system (attack/damage rolls)
- [ ] `Equipment`, `Inventory` components
- [ ] Weapon/armor items
- [ ] NPC mobiles with basic AI (wander, aggro)

### Phase 4 — Advanced Gameplay
- [ ] Crafting system (recipes, materials)
- [ ] Quest system (objective tracking)
- [ ] Spell system (effects, targeting)
- [ ] Factions & reputation
- [ ] Optional PvP flagging

### Phase 5 — OLC & Tooling
- [ ] Online creation commands
- [ ] Zone/area management
- [ ] Rhai scripting engine integration
- [ ] Scriptable triggers & events
- [ ] Hot-reload content files

### Phase 6 — Polish & Expansion
- [ ] WebSocket bridge for web clients
- [ ] MCCP, GMCP, MXP, MSSP support
- [ ] Performance profiling & optimization
- [ ] Area builder toolkit
- [ ] Admin/GM tools
