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
├── content/                # Game data files (TOML + Rhai)
│   ├── areas/              # Room graphs
│   ├── mobs/               # NPC templates
│   ├── items/              # Item templates
│   ├── races/              # Race definitions
│   ├── classes/            # Class definitions
│   ├── skills/             # Unified skill/spell/ability definitions
│   │   ├── combat/         # Combat skills (shield_bash, power_attack)
│   │   ├── magic/          # Magic skills / spells (fireball, bless)
│   │   ├── craft/          # Crafting skills (smithing, alchemy)
│   │   ├── racial/         # Racial abilities (taunt, stone_form)
│   │   └── general/        # General skills (sneak, swim)
│   ├── scripts/            # Rhai script files
│   ├── help/               # Help topic files
│   ├── affixes.toml        # Item enchantment affix definitions
│   ├── sets.toml           # Item set bonus definitions
│   └── socials.toml        # Social/emote definitions
├── tui/                    # spade — builder TUI & MUD client
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Entry: --mode, --content-path, crossterm init, start App
│       ├── app.rs          # App: state machine, event loop, screen router
│       ├── ui/
│       │   ├── mod.rs      # render() dispatch by active screen
│       │   ├── layout.rs   # Panel split, tab bar, status bar, breadcrumbs
│       │   ├── help.rs     # Help overlay (modal, data-driven sections)
│       │   ├── toast.rs    # Toast notification system
│       │   ├── sidebar.rs  # Immortal command palette (MUD mode)
│       │   ├── world_tree.rs      # Collapsible area/room tree
│       │   ├── editor.rs   # TOML field form editor
│       │   ├── room_graph.rs      # ASCII room map with mouse navigation
│       │   ├── inspector.rs       # Entity detail table with scrolling
│       │   ├── validator.rs       # Error list with jump-to-source
│       │   ├── dashboard.rs       # Server status gauges
│       │   ├── palette.rs  # Ctrl+P command palette (fuzzy search)
│       │   ├── file_browser.rs    # Content directory tree
│       │   ├── mud_client.rs      # Full MUD client output + input + sidebar
│       │   ├── output_window.rs   # Scrollable ANSI output with clickable names
│       │   ├── input_bar.rs       # Command input with autocomplete popup
│       │   ├── name_context.rs    # Right-click context menu for entity names
│       │   ├── connect_dialog.rs  # Modal: host/port/profile picker
│       │   ├── profile_manager.rs # Connection profile list editor
│       │   └── script_console.rs  # Inline Rhai REPL
│       ├── components/
│       │   ├── mod.rs      # Reusable widget exports
│       │   ├── scroll.rs   # ScrollState, Scrollbar renderer
│       │   ├── tree.rs     # Generic collapsible tree widget
│       │   ├── tabs.rs     # Tab bar widget
│       │   ├── table.rs    # Scrollable table widget
│       │   ├── form.rs     # Labeled field form widget
│       │   ├── modal.rs    # Modal overlay widget
│       │   ├── context_menu.rs    # Generic popup context menu
│       │   ├── autocomplete.rs    # Popup autocomplete list
│       │   └── text_input.rs      # Text input field with cursor/selection
│       ├── state/
│       │   ├── mod.rs      # AppState (screen, mode, focus, selection)
│       │   ├── scroll.rs   # Per-pane ScrollState map
│       │   ├── history.rs  # Undo stack for editor
│       │   └── keybinds.rs # Keybinding registry + help text lookup
│       ├── offline/
│       │   ├── mod.rs      # OfflineClient — owns TemplateRegistry
│       │   ├── loader.rs   # TOML deserialize content/ → registry
│       │   ├── saver.rs    # Atomic TOML write (temp + rename)
│       │   └── validator.rs # Runs content validator → diagnostics
│       ├── online/
│       │   ├── mod.rs      # OnlineClient — owns MudSession
│       │   ├── client.rs   # WebSocket/telnet transport, MMCC frame parsing
│       │   ├── commands.rs # Command builder + autocomplete + response dispatch
│       │   ├── session.rs  # Session state machine, reconnect logic
│       │   └── profiles.rs # Connection profile load/save
│       └── preview/
│           ├── mod.rs      # Render TOML tags → ratatui styled spans
│           ├── ansi.rs     # ANSI escape codes → ratatui Style
│           └── tags.rs     # {red}/{brightblue}/{/} tag parser
├── mcp/                    # MCP server — AI agent world-building
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs         # Entry: --mode, --port, --content-path, start McpServer
│       ├── server.rs       # McpServer: stdio transport, request dispatch
│       ├── tools/
│       │   ├── mod.rs      # Tool registry (list/handle/discover)
│       │   ├── area.rs     # @dig/@area commands as MCP tools
│       │   ├── room.rs     # Room CRUD + link/unlink exits
│       │   ├── mob.rs      # Mob template CRUD
│       │   ├── item.rs     # Item template CRUD
│       │   ├── quest.rs    # Quest template CRUD
│       │   ├── recipe.rs   # Recipe template CRUD
│       │   ├── faction.rs  # Faction template CRUD
│       │   ├── shop.rs     # Shop template CRUD
│       │   ├── race.rs     # Race template read/edit
│       │   ├── class.rs    # Class template read/edit
│       │   ├── skill.rs    # Skill/spell template read/edit
│       │   ├── validate.rs # Run validator, return diagnostics
│       │   └── search.rs   # Fuzzy search across all content
│       ├── resources/
│       │   ├── mod.rs      # Resource provider registry
│       │   ├── templates.rs # Template contents as structured resources
│       │   ├── validation.rs # Current validation state
│       │   └── stats.rs    # Content summary statistics
│       └── prompts/
│           ├── mod.rs      # Prompt template registry
│           ├── builder.rs  # "Create a new area" guided flow
│           └── reviewer.rs # "Review this content" checklist
└── bin/                    # Game server binary
    ├── Cargo.toml
    └── src/
        ├── main.rs         # tokio::main → run → shutdown
        ├── init.rs         # initialize() — init phases, returns MainLoop
        ├── main_loop.rs    # MainLoop struct, run(), shutdown handler
        ├── signals.rs      # Signal handling (SIGTERM, SIGINT)
        ├── commands.rs     # register_all_commands() — all built-in commands
        └── config.rs       # CliArgs + mud.toml merge → Config
                             #   (--content-path, MUD_CONTENT, game.content_dir)
```

---

## Game Loop & Scheduler

Not a fixed tick loop. Event-driven with subscription-based timing.

### Main Loop

The server task owns `World` behind an `Arc<RwLock<World>>`. Pulse systems
acquire a write lock; command handlers acquire a write lock in response to
player input; event dispatch acquires a write lock (mutations are common
enough that a read-write split is not worth the complexity until profiling
says otherwise).

```
┌─────────────────────────────────────────────────────────────────┐
│  tokio::select! {                                               │
│                                                                 │
│    ◄── shutdown_signal ─── flush + WAL checkpoint + exit       │
│                                                                 │
│    ◄── scheduler.next ─── run_system_phase(phase)              │
│                           (write lock World, iterate systems)   │
│                                                                 │
│    ◄── event_bus.recv ─── dispatch_event(event)                │
│                           (write lock World, fan-out to subs)   │
│                                                                 │
│    ◄── player_input ───── commands.execute(world, conn, line)  │
│         (per-connection   (write lock World)                    │
│          mpsc channel)                                          │
│  }                                                              │
└─────────────────────────────────────────────────────────────────┘
```

```rust
loop {
    select! {
        biased;

        _ = shutdown_signal() => {
            flush_dirty(&mut world.write());
            world.write().clear();
            break;
        }

        pulse = scheduler.next() => {
            let mut w = world.write();
            let phase = pulse.phase;
            for system in systems.by_phase(phase) {
                system.run(&mut w);
            }
        }

        event = event_rx.recv() => {
            let mut w = world.write();
            for system in systems.subscribed_to(event.tag()) {
                if system.handle_event(&mut w, &event) {
                    break; // consumed
                }
            }
        }

        (id, line) = input_rx.recv() => {
            let mut w = world.write();
            if let Some(conn) = connections.get_mut(id) {
                commands.execute(&mut w, conn.as_mut(), &line);
            }
        }
    }
}
```

### Scheduler

The `Scheduler` resource maintains a set of named intervals. Each interval
produces a `Pulse` on a shared mpsc channel when it fires.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Phase {
    Movement,
    Combat,
    Regeneration,
    Weather,
    DirtyFlush,
}
```

```rust
struct Scheduler {
    intervals: Vec<(Phase, Interval)>,
    tx: mpsc::Sender<Pulse>,
}

impl Scheduler {
    fn register(&mut self, phase: Phase, duration: Duration);
    async fn next(&mut self) -> Pulse;
}
```

Default intervals:

| Phase | Interval | Description |
|---|---|---|
| `Movement` | 100ms | Process queued movement commands |
| `Combat` | 2s | Combat round tick |
| `Regeneration` | 6s | HP/mana regen tick |
| `Weather` | 5m | Zone weather updates |
| `DirtyFlush` | 5s | Persist dirty entities to SQLite |

The scheduler wraps `tokio::time::interval` per phase. There is no global
heartbeat — most of the time the server is idle, waiting for input.

### Run Phase

`run_system_phase()` iterates systems registered for the pulse's phase,
sorted by priority (lower runs first). Each system receives `&mut World`
and performs its logic.

Phases are independent — `Combat` and `Regeneration` can fire concurrently
in different select iterations.

---

## Systems Architecture

Game logic is organized into systems. Each system implements the `System`
trait and is registered with the server at startup.

### System Trait

```rust
/// A unit of game logic. Can be pulse-driven, event-driven, or both.
trait System: Send + Sync {
    /// Which phase(s) this system participates in. Empty for event-only.
    fn phases(&self) -> Vec<Phase>;

    /// Called on every matching pulse. World is write-locked.
    fn run(&mut self, world: &mut World);

    /// Called when a subscribed event is dispatched.
    /// Return true to consume the event (prevent further dispatch).
    fn handle_event(&mut self, world: &mut World, event: &GameEvent) -> bool {
        false
    }

    /// Event types this system wants to receive.
    fn subscribed_events(&self) -> Vec<EventTag> {
        vec![]
    }

    /// Priority within a phase. Lower values run first.
    fn priority(&self) -> u8 {
        100
    }
}
```

### Built-in Systems

| System | Phase(s) | Events | Priority | Responsibility |
|---|---|---|---|---|---|
| `MovementSystem` | Movement | — | 10 | Process queued direction commands, update `Position`, emit `PlayerMoved` |
| `FollowSystem` | Movement | `PlayerMoved` | 20 | Move followers behind leader, pause in combat |
| `EchoSystem` | — | `PlayerSaid`, `PlayerMoved`, `PlayerDied`, `ItemDropped` | 10 | Broadcast messages to room occupants |
| `CombatSystem` | Combat | `PlayerAttacked` | 20 | Combat round: hit, damage, death |
| `StanceSystem` | Combat | — | 15 | Apply stance modifiers to combat calculations |
| `AISystem` | Combat | — | 30 | NPC behavior state machine (idle, wander, patrol, aggro, flee) |
| `FormationSystem` | Combat | — | 25 | Apply formation bonuses to group members in same room |
| `RegenSystem` | Regeneration | — | 10 | Regen HP/mana/resource pools for all entities |
| `EffectExpirySystem` | Regeneration | — | 20 | Tick down active effect durations |
| `PassiveApplicationSystem` | Regeneration | `PlayerLeveled` | 30 | Apply/remove class passives on login and level-up |
| `WeatherSystem` | Weather | — | 10 | Update zone weather states |
| `DirtyFlushSystem` | DirtyFlush | — | 50 | Flush dirty entities to SQLite |
| `SkillRequirementSystem` | DirtyFlush | — | 40 | Check skill gates on equipped items, auto-remove on failure |
| `GroupCleanupSystem` | DirtyFlush | — | 45 | Sweep stale followers and disconnected members |
| `CorpseSystem` | DirtyFlush | — | 60 | Decay expired corpses, transfer contents to room |
| `AreaResetSystem` | DirtyFlush | — | 70 | Trigger area resets for zones past their interval |
| `SetBonusSystem` | — | `ItemWorn`, `ItemRemoved` | 10 | Evaluate item set bonuses on equip/unequip |
| `QuestProgressSystem` | — | `MobDied`, `ItemPickedUp`, `SkillUsed` | 20 | Update quest objective counters on relevant events |
| `CraftingSystem` | — | `SkillUsed` (craft type) | 20 | Execute crafting: check materials, roll success, consume/spawn |
| `ScriptTriggerSystem` | — | `ScriptTrigger` | 100 | Evaluate attached Rhai scripts |

### Registration

```rust
server.add_system(Box::new(MovementSystem::new()));
server.add_system(Box::new(CombatSystem::new()));
```

Systems are stored in a `PhaseMap<Vec<Box<dyn System>>>` sorted by priority
at registration time.

### World Access

All pulse and event dispatch acquires a **write lock** on `World`. This is
deliberate — most mutations happen during ticks, and contention is low
(hundreds of short-lived ticks per minute, not thousands). A read-write
split can be introduced later if profiling shows contention.

---

## Event Bus

### Topology

Events are dispatched over a `tokio::sync::broadcast` channel. The channel
has a single sender (owned by the server task) and one receiver per
subscribed system.

```
                  ┌─────────────────┐
                  │   Event Bus tx  │
                  │  (broadcast)    │
                  └────────┬────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
   ┌──────────┐    ┌──────────┐    ┌──────────────┐
   │EchoSystem│    │CombatSys │    │ScriptTrigger │
   └──────────┘    └──────────┘    └──────────────┘
```

### Event Envelope

All events carry a metadata header for routing and debugging:

```rust
#[derive(Debug, Clone)]
struct EventEnvelope {
    id: u64,            // monotonic counter
    tag: EventTag,      // discriminator for fast filtering
    timestamp: Instant,
    payload: GameEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EventTag {
    PlayerSaid,
    PlayerMoved,
    PlayerAttacked,
    PlayerDied,
    PlayerLeveled,
    MobDied,
    MobKilled,
    ItemPickedUp,
    ItemDropped,
    ItemWorn,
    ItemRemoved,
    SkillUsed,
    SkillTrained,
    RoomEntered,
    QuestUpdated,
    QuestCompleted,
    FactionChanged,
    SetBonusChanged,
    CorpseDecayed,
    ContentReloaded,
    ScriptTrigger,
    Pulse(Phase),
}
```

### Subscription & Dispatch

Systems declare interest via `subscribed_events()`. On dispatch, the
event bus iterates systems whose tags match and calls `handle_event()`
in priority order. If a system returns `true`, the event is consumed
and no further systems receive it. Scripts always run last (priority 100).

**In-band** (default): dispatched synchronously inside the main `select!`
branch. World lock is held for the full dispatch.

**Out-of-band** (opt-in for expensive handlers): spawned as a separate
tokio task. Used for logging, analytics, or database writes.

```rust
trait System {
    fn dispatch_mode(&self) -> DispatchMode { DispatchMode::InBand }
}

enum DispatchMode {
    InBand,
    OutOfBand,
}
```

### Script Events

The `ScriptTriggerSystem` receives `ScriptTrigger` events and evaluates
attached Rhai scripts via the `on()` handler system (see Scripting & OLC).

---

## ECS Component Design

### Spatial

```rust
struct Position { room: Entity }
struct Room { name: String, description: String }
struct Exit { direction: Direction, dest: Entity, flags: ExitFlags }
struct RoomExits(Vec<Exit>);        // one per room entity
struct PortalExit { keyword: String, dest: Entity, description: String, flags: PortalFlags }
struct RoomPortals(Vec<PortalExit>); // one per room entity
type PortalFlags = u8;
const PORTAL_HIDDEN: PortalFlags = 0x01;
type RoomFlagBits = u16;
struct RoomFlags(RoomFlagBits);      // portal/teleport permissions
const ROOM_PORTAL_IN: RoomFlagBits = 0x0001;
const ROOM_PORTAL_OUT: RoomFlagBits = 0x0002;
const ROOM_NO_TELEPORT_IN: RoomFlagBits = 0x0004;
const ROOM_NO_TELEPORT_OUT: RoomFlagBits = 0x0008;
struct VoidRoom;                      // marker: inescapable room, blocks all movement/recall/teleport
struct Teleportable(pub bool);       // targetable by player teleport spells
enum Direction { North, South, East, West, Up, Down, Northeast, Northwest, Southeast, Southwest }
```

The **void room** is a singleton inescapable room — the default spawn point before
character creation (Phase 2). It has the `VoidRoom` marker component and zero
exits. All movement commands, recall spells, teleport effects, and similar
relocation mechanics must check for `VoidRoom` on the origin or destination
and reject the action unless the actor has immortal bypass or it's an approved
codepath (e.g. finalizing character creation).

### Character

```rust
struct Player { account_id: i64 }
struct Npc { template_id: String }
struct Attributes { str: u8, dex: u8, int: u8, wis: u8, con: u8, cha: u8 }
struct Health { current: i32, max: i32 }
struct Level(u8);
struct Experience(u64);

/// Resource pools (not all used for every character — depends on class/skill access)
struct Stamina { current: u16, max: u16 }
struct Mana { current: u16, max: u16 }
struct Energy { current: u16, max: u16 }
struct Psi { current: u16, max: u16 }
struct Immortal { incognito: bool, holylight: bool, build_mode: bool };
struct Teleportable(pub bool); // can this entity be teleported by other players?
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

### Character & Progression

```rust
struct Name(String);
struct Description(String);
struct Alignment(String);
struct Wallet(u64);

struct CombatStats {
    base_attack_bonus: i8,
    fort_save: i8,
    ref_save: i8,
    will_save: i8,
}

struct ActiveStance(Option<String>);

struct PassiveEffect {
    id: String,
    effect: EffectTemplate,
}

struct LearnedSkills {
    skills: HashMap<String, SkillRank>,
    cooldowns: HashMap<String, Instant>,
}

struct SkillRank(u16);

struct MultiClassInfo {
    classes: Vec<ClassEntry>,
}

struct ClassEntry {
    class_id: String,
    level: u8,
    is_favored: bool,
}

struct FactionStanding {
    standings: HashMap<String, i32>,
}

struct QuestLog {
    active: HashMap<String, QuestProgress>,
    completed: Vec<String>,
}

struct QuestProgress {
    quest_id: String,
    objectives: Vec<ObjectiveState>,
    started_at: Instant,
}

struct ObjectiveState {
    objective_index: usize,
    current: u32,
    completed: bool,
}

struct LearnedRecipes {
    recipes: Vec<String>,
}
```

### Item Progression

```rust
struct SetTracker {
    active_sets: HashMap<String, ActiveSet>,
}

struct ActiveSet {
    template_id: String,
    counts: HashMap<String, u16>,   // piece_type → count
    equipped: Vec<String>,           // item template IDs
    active_tiers: Vec<usize>,        // indices into set.bonuses
}

struct ItemTriggers {
    on_hit: Vec<TriggerEffect>,
    on_wear: Vec<TriggerEffect>,
    on_remove: Vec<TriggerEffect>,
    on_use: Vec<TriggerEffect>,
}

struct TriggerEffect {
    chance: u8,          // 0–100
    skill_id: String,    // skill to execute
    target: TriggerTarget, // self | attacker | room | random
}

enum TriggerTarget { Self, Attacker, Room, Random }
```

---

## Combat System

### Components

```rust
struct CombatTarget(Entity);           // set by `kill` command
struct Damage(i32);                     // raw damage for current round
struct Armor { base: i32, bonus: i32 } // damage reduction
enum DamageType { Slash, Pierce, Bludgeon, Fire, Cold, Lightning, Acid, Poison, Magic, True }
```

`Health`, `Attributes`, `Level` components are defined in ECS Component Design.

### Attack Flow

```
kill <target>
    │
    ▼
Set CombatTarget on attacker
    │
    ▼
[Combat pulse every 2s] ─── for each entity with CombatTarget:
    │
    ├── Check same room (melee) or line-of-sight (ranged)
    ├── Hit roll: d20 + attacker.level + attacker.str_mod ≥ target AC
    │   ├── Natural 1 → automatic miss
    │   └── Natural 20 → automatic crit (damage × 2)
    ├── Damage roll: weapon.dice + attacker.str_mod
    ├── Apply damage: target.health.current -= damage
    ├── Emit PlayerAttacked / MobAttacked event
    │
    └── If target dead:
        ├── Emit PlayerDied / MobDied event
        ├── Grant XP to killer (XP = target.level² × 50)
        ├── Create corpse entity at room (inventory transfer)
        ├── Remove target from combat (clear CombatTarget)
        └── Respawn NPC after delay (configurable per template)
```

### Hit & Damage Formulas

```
AC = 10 + level + dex_mod + armor.total()

Hit:    d20 + level + str_mod ≥ AC     (melee)
        d20 + level + dex_mod ≥ AC     (ranged)
Crit:   natural 20 → double all dice
Miss:   natural 1  → no damage + lose next round

Damage: weapon_damage + str_mod
        + level / 5 (bonus per 5 levels)
```

### NPC AI

Each NPC has a state machine updated each combat pulse:

```
                   kill command / aggro
    ┌──────┐       ──────────────────►  ┌────────┐
    │ Idle │                             │ Combat │
    └──────┘◄──────────────────────────  └────────┘
              target dead / fled             │
                                             │ flee condition
                                             ▼
                                        ┌────────┐
                                        │ Flee   │
                                        └────────┘
                                              │
                                     flee to adjacent room,
                                     then → Idle
```

```rust
enum NpcState {
    Idle,
    Wander { dest: Option<Entity> },
    Combat { target: Entity, threat_table: Vec<(Entity, i32)> },
    Flee,
}
```

- **Wander:** NPC picks a random exit every 3–5 pulses and moves there.
- **Aggro:** NPC detects player within aggro range (configurable per template).
  Sets `CombatTarget` and moves to engage.
- **Threat table:** Updated by damage dealt, healing done, taunt effects.
  NPC attacks highest-threat target.

### Death Processing

1. Emit `PlayerDied` or `MobDied` event
2. Calculate XP grant: `killer.experience += victim.level² × 50`
3. Spawn corpse entity with victim's inventory (all `Inventory` items transferred)
4. Remove `Inventory` from victim, set `Health.current = 0`
5. Player: prompt "Return to bind point? (y/n)" → teleport or stay ghost
6. NPC: despawn immediately, respawn after `respawn_delay` (from template)

### Combat Log

Each round outputs a formatted combat message to the attacker and target
(and optionally to the room):

```
You hit goblin for 12 damage!
goblin dodges your attack!
Critical hit! You smash the orc for 34 damage!
```

### Damage Types & Resistances

The `DamageType` enum covers all damage forms:

```rust
enum DamageType { Slash, Pierce, Bludgeon, Fire, Cold, Lightning, Acid, Poison, Magic, True }
```

Each entity can have resistances and vulnerabilities defined across
multiple sources (race template, class template, equipment, active
buffs). Resistances are multipliers applied to incoming damage:

```rust
struct DamageResistances {
    entries: HashMap<DamageType, f32>,
}
```

| Multiplier | Meaning |
|---|---|
| `2.0` | Vulnerable — double damage |
| `1.0` | Normal — no modifier |
| `0.5` | Resistant — half damage |
| `0.0` | Immune — no damage |
| `-1.0` | Absorbed — healed instead of damaged |

**Source stacking:** Resistances from different sources multiply.
A character with `fire = 0.5` from race and `fire = 0.5` from
equipment receives `0.5 × 0.5 = 0.25` (75% reduction).

**TOML definition in templates:**

```toml
# content/mobs/fire_elemental.toml
resistances = { fire = -1.0, cold = 2.0, physical = 0.5 }

# content/items/flame_sword.toml
[weapon]
damage = { count = 1, sides = 8, type = "fire" }

# content/races/dwarf.toml
resistances = { poison = 0.5 }
```

**Damage formula with resistances:**

```
base_damage = roll(weapon.dice) + str_mod + level_bonus
for each damage_type on the attack:
    multiplier = target.resistances.get(damage_type).unwrap_or(1.0)
    final_damage += base_damage × multiplier
```

If the weapon does multiple damage types (e.g. `1d8 slash + 1d6 fire`),
each type is rolled separately and multiplied by its own resistance.

**Resistance from buffs:**

```toml
# content/skills/magic/resist_fire.toml
[effect]
type = "buff"
stat = "resistance"
subtype = "fire"
amount = 0.5           # +50% fire resistance
duration_secs = 300
```

### Weapon Styles

**Two-handed weapons:**

Weapons with `hands = "two"` in their template get:

- **Damage:** 1.5 × STR modifier (rounded down, minimum +1)
- **Speed:** Base weapon speed × 1.2 (slower swings)
- **Shield incompatible:** Cannot equip a shield while wielding
- **Two-handed grip:** Can also wield a one-handed weapon in two hands
  for 1.5× STR but no shield

```toml
# content/items/greatsword.toml
[weapon]
hands = "two"
damage = { count = 2, sides = 6, type = "slash" }
speed = 3.0
```

**Dual-wielding:**

Equipping weapons in both `Weapon` and `OffHand` slots:

| Slot | Penalty (without feat) | Penalty (with `ambidexterity` feat) |
|---|---|---|
| Primary | −4 to hit | −2 to hit |
| Off-hand | −8 to hit, 0.5 × STR mod to damage | −4 to hit, full STR mod to damage |

```rust
enum EquipmentSlot {
    // ...existing slots...
    OffHand,   // used by both shields and second weapons
}
```

**Rules:**
- Dual-wielding requires both weapons to be one-handed (`hands = "one"`)
- Off-hand weapon speed must be ≤ primary weapon speed (off-hand
  follows the primary's swing timer)
- Shields occupy `OffHand` — cannot dual-wield with a shield
- Dual-wielding grants an extra attack roll each round (off-hand)
- Feats and skills can reduce penalties (see `ambidexterity` skill in
  `content/skills/combat/`)

**Template field:**

```toml
[weapon]
hands = "one"        # "one" (default) | "two"
```

---

## Corpse & Loot

### Corpse Creation

On death, a corpse entity is spawned with the following structure:

```rust
struct Corpse {
    owner: Option<Entity>,
    created_at: Instant,
    decay_secs: u32,
    lootable_by: LootRule,
}

enum LootRule {
    Public,
    GroupOnly,
    OwnerOnly,
    Faction,
}
```

- **Name:** "corpse of <name>"
- **Container:** Contains all items from victim's `Inventory` + `Equipment`
- **Player corpses:** `GroupOnly` by default, 10-minute decay
- **NPC corpses:** `Public` by default, 5-minute decay

### Looting

```
loot <corpse>                  — show contents
loot <corpse> <item>           — take specific item
loot <corpse> all              — take all items
loot <corpse> all.coin         — take all currency
get <item> corpse              — alias shortcut
```

Checking contents (`loot` without `all`/item) is always allowed. Taking
items respects `LootRule`.

### Corpse Decay

A `CorpseSystem` runs on each `DirtyFlush` phase:

1. Query all entities with `Corpse` component
2. Check `Instant::now() - created_at >= Duration::from_secs(decay_secs)`
3. Expired corpses: transfer remaining items to room floor, despawn corpse
4. Emit `CorpseDecayed { corpse }` event

Corpses are purely in-memory — no SQL persistence (transient).

---

## Group & Party

### Group Structure

```rust
struct Group {
    leader: Entity,
    members: Vec<Entity>,
    loot_mode: LootMode,
    formation: Formation,
}

enum LootMode {
    FreeForAll,
    RoundRobin { next_index: usize },
    MasterLooter(Entity),
}

enum Formation {
    Default,
    Line,
    Scattered,
}
```

Groups are managed by a singleton resource:

```rust
struct GroupManager {
    groups: Vec<Group>,
    invites: Vec<GroupInvite>,
}

struct GroupInvite {
    inviter: Entity,
    target: Entity,
    expires_at: Instant,
}
```

### Components

```rust
struct GroupMember {
    group_id: u64,
    role: GroupRole,
}

enum GroupRole { Leader, Member }

struct Following {
    target: Entity,          // entity being followed
    autofollow: bool,        // auto-follow on room entry
}
```

### Commands

```
group invite <player>       — invite player
group accept                — accept pending invite
group decline               — decline pending invite
group leave                 — leave group
group kick <player>         — remove member (leader only)
group disband               — disband group (leader only)
group loot <mode>           — change loot mode (leader only)
group status                — show group members + health
group chat <message>        — send message to all group members
follow <player>             — start following a player
follow stop                 — stop following
```

### Invite Flow

```
> group invite bob
You invite bob to join your group.
[Bob receives:] Alice invites you to join a group. (group accept / group decline)
```

Invites expire after 30 seconds. A player can only have one pending invite.

### Follow System

Players can follow other players, automatically moving through rooms
behind them:

```
> follow alice
You start following Alice.
[Alice:] Bob is now following you.

> follow stop
You stop following Alice.
[Alice:] Bob stops following you.
```

**Follow movement:** When an entity with `Following` moves to a new room,
the follower's `Position` is also updated (1-tick delay to prevent
synchronization issues on fast movement). If the follower is in combat,
the follow is paused until combat ends.

**Auto-follow:** If `autofollow = true` (default), the follower
automatically targets the leader's new room on any movement event.
If `autofollow = false`, only explicit `follow` commands queue movement.

**Chained follow:** If A follows B and B follows C, only B moves on
C's movement. A does not move until B moves (next tick). This prevents
teleport chaining.

### Formation Bonuses

Formations modify combat stats for all group members in the same room:

| Formation | Effect | Activates at |
|---|---|---|
| `Default` | None | Always available |
| `Line` | +1 AC per member (front rank), −1 AC per member (back rank) | Group size ≥ 2 |
| `Scattered` | −2 AC, +10% dodge chance per member | Group size ≥ 2 |
| `Column` | +1 damage per member on first hit | Group size ≥ 3 |
| `Wedge` | +2 attack, −4 AC for leader | Group size ≥ 3 |
| `Shield Wall` | +2 AC for all, −2 attack for all | Requires shield + group ≥ 2 |

```
> group formation line
You form a line formation.
Members gain +1 AC (front) / −1 AC (back).
```

Formation bonuses are applied as `ActiveEffect` components on each
member by a `FormationSystem` (Combat phase). Bonuses update when
formation changes or members enter/leave the room.

### Group Skills & Buffs

Certain skills can target the entire group:

```toml
# content/skills/magic/group_heal.toml
[effect]
type = "heal"
dice_count = 2
dice_sides = 6
targeting = "group"          # affects all group members in room
```

Group-targeting skills pulse to all `GroupMember` entities sharing the
same `group_id` within the caster's room. Out-of-room members do not
receive the effect.

**Persistent group buffs** (e.g. "Bless Group") apply a `PassiveEffect`
with `scope = "group"`. The `PassiveApplicationSystem` checks group
membership at application time and re-applies on member join.

### Shared Quest Credit

Group kills and gathers increment quest objectives for all members in
the same room (configurable per quest):

```toml
# content/quests/goblin_problem.toml
share_kills = true      # group kills count for all members
share_gather = false    # only the looter gets credit
```

The `QuestProgressSystem` listens to `MobDied` and `ItemPickedUp`
events. If the source is in a group, it iterates members in the same
room and updates each qualifying `QuestLog`.

### Loot Distribution

| Mode | Behavior |
|---|---|
| `FreeForAll` | Anyone can loot any corpse (default) |
| `RoundRobin` | Each kill cycles to the next player in order. Only that player may loot the corpse for the first 30s; after that, it opens to all. |
| `MasterLooter` | Only the designated looter may loot. They distribute via `give`. |

```rust
enum LootMode {
    FreeForAll,
    RoundRobin { next_index: usize },
    MasterLooter(Entity),
}
```

`RoundRobin` tracking: each kill increments `next_index` (wrapping).
The selected player sees: "You are next to loot goblin's corpse."
Others see: "Bob is next to loot goblin's corpse."

### Disconnect Handling

When a group member disconnects:

1. **Leader disconnect:** Leader role transfers to the longest-standing
   member. Disconnected leader reconnects as a regular member.
2. **Member disconnect:** The member is removed from the group after a
   60-second grace period (configurable). If they reconnect within the
   grace period, auto-rejoin.
3. **Solo player disconnect:** No action (group already empty).
4. **Follower disconnect:** Following entity is removed; follower
   remains in their last known room until timeout.

A `GroupCleanupSystem` (DirtyFlush phase) sweeps stale followers and
disconnected members past their grace period.

### Group Chat

```
> group chat Anyone seen the blacksmith?
[Group] You say, "Anyone seen the blacksmith?"
[Group | Bob]: "Over by the forge."
```

Group messages are prefixed with `[Group]` and only reach group members
(currently online). Offline members do not receive backlog (channels
are real-time only).

+10% XP per member, capped at +50% (full group of 5).

---

## Skill System

### Unified Skill Model

All character abilities — combat maneuvers, spells, racial powers, crafting
recipes, psionics, tech — are a single `SkillDef` type discriminated by
`skill_type`. Spells are not a separate system; they are skills with
`type = "magic"` and a `MagicConfig` attached.

This means:
- Everything lives in one `content/skills/` directory (subdirectories for organization)
- One registry map: `TemplateRegistry.skills: HashMap<String, SkillDef>`
- One command (`use`) invokes any skill; `cast` is syntactic sugar that gates on `skill_type == Magic`
- New skill-like systems (tech, psionics, martial arts) add a `SkillType` variant + config struct — zero new infrastructure

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
enum SkillTypeConfig {
    Combat,
    Magic(MagicConfig),
    Tech(TechConfig),
    Psionics(PsionicsConfig),
    Craft(CraftConfig),
    Social,
    General,
}

struct MagicConfig {
    school: MagicSchool,          // Abjuration, Conjuration, Divination, Enchantment, Evocation, Illusion, Necromancy, Transmutation
    sub_school: Option<String>,
    casting_time_secs: u8,
    concentration: bool,
    components: Vec<SpellComponent>,  // Verbal, Somatic, Material
    material_component: Option<MaterialComponent>,
}

struct TechConfig {
    skill_prereqs: HashMap<String, u8>,  // skill_id → minimum level
    hardware_required: Option<String>,    // item template required as focus
}

struct PsionicsConfig {
    discipline: String,
    power_source: String,
    risk: PsionicRisk,
}

struct CraftConfig {
    station: String,            // e.g. "anvil", "alchemy_table"
    materials: Vec<CraftMaterial>,
    difficulty: u8,
}

struct SkillDef {
    id: String,
    name: String,
    skill_type: SkillTypeConfig,
    level_requirement: u8,
    cooldown_secs: u32,
    targeting: Targeting,
    cost: ResourceCost,
    effect: EffectTemplate,
    script: Option<String>,

    // Constraints (data-driven; never hardcoded)
    allowed_classes: Vec<AllowedClassEntry>,
    allowed_races: Vec<String>,
    requires_skill: Vec<SkillPrereq>,
    must_train: bool,
    trainer_types: Vec<String>,
    use_while_fighting: bool,
    use_while_sitting: bool,
}

enum ResourceCost {
    None,
    Stamina(u16),
    Mana(u16),
    Energy(u16),
    Psi(u16),
    Gold(u64),
    Xp(u64),
}

struct AllowedClassEntry {
    class: String,          // class key
    spell_level: Option<u8>, // for magic skills: which circle/level this class gets it at
}

struct SkillPrereq {
    id: String,
    level: u8,              // minimum skill rank
}

enum Targeting {
    Self,
    Single { range: u8 },
    Room,
    Area { radius: u8 },
}
```

### Mana & Resource Components

Characters have resource pools tracked in ECS components. Which pool(s) a
character has depends on class/race — a warrior has `Stamina`, a mage has
`Mana`, a psion has `Psi`. Resources are depleted on skill use and
regenerated each Regeneration pulse.

```rust
struct Stamina { current: u16, max: u16 }
struct Mana { current: u16, max: u16 }
struct Energy { current: u16, max: u16 }
struct PsiPool { current: u16, max: u16 }
```

**Regen:** `current += max / 20` per Regeneration pulse (5% per 6s = ~100% in 2min), applied to all resource pools present on the entity.

### Learned Skills

Characters track which skills they know and when they're on cooldown:

```rust
struct LearnedSkills {
    skills: HashMap<String, SkillRank>,   // skill_id → current rank
    cooldowns: HashMap<String, Instant>,
}

struct SkillRank(u16);  // proficiency level in the skill (0 = untrained)
```

**Auto-learn:** On level-up, scan class definition for `auto_skills` where
`level_requirement <= new_level` and the character does not already
know them. Racial abilities are auto-granted on character creation.

**Trainer NPCs:** `train` command opens a training menu showing
available skills with costs. Pay → increase rank.

### Flow

```
use <skill> [target]
  → skill known? (LearnedSkills)
  → cooldown ready?
  → resource sufficient?
  → type-specific checks (concentration? components? hardware?)
  → consume resource, apply effect, start cooldown
```

`cast` is an alias: `cast fireball` ≡ `use fireball` with a type gate
that rejects non-Magic skills.

### Type-Specific Behavior

| `type` | Resource | Extra checks | Display label |
|---|---|---|---|
| `combat` | Stamina | Must be fighting or `use_while_fighting` | "skill" |
| `magic` | Mana | Concentration check if damaged during cast, component check | "spell" |
| `tech` | Energy | Hardware focus must be in inventory/equipped | "tech" |
| `psionics` | Psi | Risk roll on use (backlash chance) | "power" |
| `craft` | Stamina | Station must be in room, materials consumed | "craft" |
| `social` | None | — | "social" |
| `general` | None/Stamina | — | "skill" |

### Effect System

Effects are the runtime representation of a skill's action:

```rust
enum EffectTemplate {
    Damage { dice_count: u8, dice_sides: u8, damage_type: DamageType },
    Heal { dice_count: u8, dice_sides: u8 },
    Buff { stat: Stat, amount: i8, duration_secs: u32 },
    Debuff { stat: Stat, amount: i8, duration_secs: u32 },
    Teleport { target_room: String },
    Script { script_id: String },
    Spawn { mob_id: String, count: u8 },
    Aura { aura_id: String, radius: u8 },   // persistent area effect
}

enum Stat {
    Strength,
    Dexterity,
    Intelligence,
    Wisdom,
    Constitution,
    Charisma,
}
```

**Duration tracking:**

```rust
struct ActiveEffect {
    effect: EffectTemplate,
    remaining_secs: u32,
}
```

`EffectExpirySystem` (Regeneration phase) decrements `remaining_secs` and
removes expired effects.

### TOML Examples

A magic skill (formerly a spell):

```toml
# content/skills/magic/fireball.toml
name = "Fireball"
type = "magic"
level_requirement = 5
cooldown_secs = 6
targeting = { area = { radius = 2 } }

[cost]
mana = 35

[magic]
school = "Evocation"
casting_time_secs = 3
concentration = true
components = ["verbal", "somatic", "material"]
material_component = { item = "bat_gui", consumed = true }

[effect]
type = "damage"
dice_count = 6
dice_sides = 6
damage_type = "Fire"

allowed_classes = [
    { class = "mage", spell_level = 5 },
    { class = "sorcerer", spell_level = 5 },
    { class = "warlock", spell_level = 6 },
]

must_train = false
trainer_types = ["mage_guild", "archmage"]
use_while_fighting = false
use_while_sitting = false
```

A combat skill:

```toml
# content/skills/combat/shield_bash.toml
name = "Shield Bash"
type = "combat"
level_requirement = 3
cooldown_secs = 8
targeting = { single = {} }

[cost]
stamina = 15

[effect]
type = "damage"
dice_count = 1
dice_sides = 6
damage_type = "Bludgeon"

allowed_classes = [
    { class = "warrior" },
    { class = "paladin" },
    { class = "fighter" },
]

requires_skill = [{ id = "bash", level = 3 }]
must_train = true
trainer_types = ["weapon_master", "trainer"]
use_while_fighting = true
use_while_sitting = false
```

A racial ability (auto-granted by race, not class):

```toml
# content/skills/racial/taunt.toml
name = "Taunt"
type = "combat"
level_requirement = 1
cooldown_secs = 60
targeting = { single = { range = 20 } }

[cost]
stamina = 5

[effect]
type = "script"
script_id = "taunt.rhai"

allowed_races = ["dwarf"]
allowed_classes = []  # no class restriction, but race-gated
must_train = false
use_while_fighting = true
```

### Commands

```
use <skill> [target]    — use any known skill
cast <spell> [target]   — alias for use, gated to skill_type == Magic
use                     — show known skills grouped by type with cooldown
cast                    — show known magic skills with cooldown
```

---

## Races

### Design Principle

Races are **template definitions, not enums**. The engine has zero baked-in
knowledge of specific races. All behavior flows from data in
`content/races/*.toml`. Removing a race file removes that race from the game
without recompiling.

### Race Template (Expanded)

Every race file under `content/races/` uses this schema:

```toml
# content/races/dwarf.toml
name = "Dwarf"
description = "Stout and resilient."
attributes = { str = 12, dex = 8, int = 10, wis = 12, con = 14, cha = 8 }
size = "medium"        # small | medium | large
speed = 25

# Which classes this race can choose at creation
allowed_classes = ["warrior", "paladin", "cleric", "fighter"]

# Starting languages (auto-granted)
languages = ["common", "dwarvish"]

# Hometown — room key for starting location override
hometown = "midgaard/dwarf_hall"

# Passive racial traits
[traits]
infravision = 60        # range in feet; 0 means none
stonecunning = true     # +2 bonus on stonework-related checks
poison_resist = 2       # save bonus vs poison

# Active racial abilities — references to SkillDefs in content/skills/racial/
# Auto-granted on character creation, just like class auto_skills
racial_abilities = ["taunt", "stone_form"]

# Cultural knowledge — racial familiarity bonuses
[familiarity]
orc = 2
giant = 2
goblin = 1

# Recommended alignments — for creation wizard display
[alignment_tendencies]
lawful_good = 30
lawful_neutral = 25
neutral = 20
chaotic_good = 15
```

**Size effects** (applied by the engine based on `size`):

| Size | AC mod | Damage mod | Carry capacity | Weapon sizing |
|---|---|---|---|---|
| `small` | +1 | -1 | ×0.75 | Small weapons only |
| `medium` | 0 | 0 | ×1.0 | Standard |
| `large` | -1 | +1 | ×1.5 | Large weapons; can't use small |

### Traits

Traits are boolean-or-numeric always-on passives defined per race. They are
not skills — they are checked at query time (e.g., `has_trait(entity, "infravision")`).
The engine provides trait-checking helper functions but never matches on
trait names as strings.

### Racial Abilities

Racial abilities are `SkillDef` entries in `content/skills/racial/` with
`allowed_races` restricting them to a specific race. On character creation,
the engine scans the chosen race's `racial_abilities` list and auto-grants
each matching skill to `LearnedSkills`.

This means racial abilities are **still skills** — they share cooldown tracking,
resource costs, targeting, and effect system with every other skill type.

### Languages

Languages are defined in `content/languages.toml`:

```toml
# content/languages.toml
[language.common]
name = "Common"
speakers = ["human", "elf", "dwarf", "half_elf", "halfling"]

[language.dwarvish]
name = "Dwarvish"
script = "dwarvish_runes"
speakers = ["dwarf"]

[language.elvish]
name = "Elvish"
script = "elvish_script"
speakers = ["elf", "half_elf"]
```

Characters auto-learn languages from their race's `languages` list at
creation. Additional languages can be learned via the `learn` command
(spending skill points or finding a teacher).

### Familiarity

`[familiarity]` maps race keys to bonus values. These bonuses apply to
social skills, lore checks, and identification attempts against the
target race. For example, a dwarf gets +2 on checks involving orcs.

### Rust Struct

```rust
struct RaceTemplate {
    id: String,
    name: String,
    description: String,
    attributes: Attributes,
    size: Size,
    speed: u8,
    allowed_classes: Vec<String>,
    languages: Vec<String>,
    hometown: Option<String>,
    traits: HashMap<String, RaceTraitValue>,
    racial_abilities: Vec<String>,
    familiarity: HashMap<String, i8>,
    alignment_tendencies: HashMap<String, u8>,
}

enum RaceTraitValue {
    Bool(bool),
    Int(i32),
}

struct LanguageDef {
    id: String,
    name: String,
    script: Option<String>,
    speakers: Vec<String>,
}
```

The `LanguageDef` struct is loaded from `content/languages.toml` and stored
in `TemplateRegistry.languages`.

---

## Classes

### Design Principle

Classes are **template definitions, not enums**. A class defines attribute
mods, hit die, base attack bonus, save progression, skill access categories,
auto-learned skills, stances, passives, prestige gates, and multi-classing
rules. Everything is data-driven.

### Base Class Template

```toml
# content/classes/warrior.toml
name = "Warrior"
description = "A master of martial combat."
attribute_mods = { str = 3, con = 2, dex = 1 }
hit_die = 10
base_attack_bonus = 1.0          # 1.0 = full BAB (warrior), 0.75 = cleric, 0.5 = mage
skill_ranks_per_level = 2
fort_save = "good"               # good: +2 + 0.5×level | poor: ~0.33×level
ref_save = "poor"
will_save = "poor"
prestige = false

# Character creation gates
allowed_races = ["human", "dwarf", "elf", "half_elf"]
allowed_alignments = ["lawful_good", "neutral_good", "chaotic_good", "lawful_neutral", "neutral"]

# Skill access — three categories determine max rank formula
[[class_skills]]
id = "swords"
max_rank = "level+3"

[[class_skills]]
id = "shield_bash"
max_rank = "level+3"

[[class_skills]]
id = "power_attack"
max_rank = "level+3"

[[class_skills]]
id = "toughness"
max_rank = "level+3"

[[cross_class_skills]]             # max_rank = (level+3) / 2
id = "sneak"

[[cross_class_skills]]
id = "pick_lock"

[[exclusive_skills]]                # only warriors can learn these at all
id = "whirlwind"
max_rank = "level+3"

# Auto-learned on level-up
auto_skills = [
    { id = "power_attack", level = 1 },
    { id = "shield_bash",  level = 5 },
    { id = "whirlwind",    level = 10 },
    { id = "toughness",    level = 15 },
]

# Auto-learned magic skills (if any)
auto_spells = []

# Toggleable combat modes
[[stances]]
id = "defensive"
name = "Defensive Stance"
ac_bonus = 2
attack_penalty = -2
min_level = 1

[[stances]]
id = "offensive"
name = "Offensive Stance"
damage_bonus = 2
ac_penalty = -2
min_level = 3

# Always-on class bonuses
[[passives]]
id = "warrior_strength"
name = "Warrior's Strength"
description = "Adds +1 damage per 4 levels."
effect = { type = "script", script_id = "warrior_strength.rhai" }
min_level = 1

[multi_classing]
favored = true              # no XP penalty for this class as secondary
```

### Skill Access Categories

Classes categorize skills into three access levels, which determine the
maximum rank a character can achieve:

| Category | Max rank formula | Description |
|---|---|---|
| `class_skills` | `level + 3` | Full proficiency — core skills |
| `cross_class_skills` | `(level + 3) / 2` | Half proficiency — peripheral skills |
| `exclusive_skills` | Per-class max (usually `level + 3`) | Only this class can ever train this skill |

A skill not listed in any category for the character's class is **unavailable**
(max rank = 0). The `train` command refuses to teach it.

### BAB & Save Progressions

```rust
struct ClassProgression {
    base_attack_bonus: f32,      // multiplied by level, then floored
    fort_save: SaveProgression,
    ref_save: SaveProgression,
    will_save: SaveProgression,
}

enum SaveProgression {
    Good,   // +2 + level * 0.5
    Poor,   // level * 0.33
}
```

Computed at level-up and stored on the character:

```rust
struct CombatStats {
    base_attack_bonus: i8,
    fort_save: i8,
    ref_save: i8,
    will_save: i8,
}
```

### Alignment Restrictions

`allowed_alignments` is an array of alignment keys. At character creation,
the class selection is filtered to show only classes where the chosen race
intersects with `allowed_races` AND the player's chosen alignment is in
`allowed_alignments`. Alignment changes during gameplay that violate class
restrictions trigger a warning; further violations may cause class feature
loss at the admin's discretion (defined in per-class Rhai script).

### Stances

Stances are toggleable combat modes defined in the class template. A
character can have exactly one active stance at a time (activating a new
stance deactivates the previous one). Stances are tracked via:

```rust
struct ActiveStance(Option<String>);
```

Activation: `stance <name>` command. Deactivation: `stance default`.
Stances cannot be changed while in combat unless the stance explicitly
allows it.

### Passives

Passives are always-on class bonuses. They are applied as components
on level-up (or on class grant for multi-classed characters) and
removed if the class is lost. Passives can reference scripts for
complex effects.

```rust
struct PassiveEffect {
    id: String,
    effect: EffectTemplate,
}
```

A `PassiveApplicationSystem` runs at login and level-up: it queries all
character class components, resolves the class template's `[passives]` list,
and ensures the matching `PassiveEffect` components exist on the entity.

### Rust Struct

```rust
struct ClassTemplate {
    id: String,
    name: String,
    description: String,
    attribute_mods: Attributes,
    hit_die: u8,
    base_attack_bonus: f32,
    skill_ranks_per_level: u8,
    fort_save: SaveProgression,
    ref_save: SaveProgression,
    will_save: SaveProgression,
    prestige: bool,
    prestige_gate: Option<PrestigeGate>,
    allowed_races: Vec<String>,
    allowed_alignments: Vec<String>,
    class_skills: Vec<SkillAccessEntry>,
    cross_class_skills: Vec<SkillAccessEntry>,
    exclusive_skills: Vec<SkillAccessEntry>,
    auto_skills: Vec<AutoSkillEntry>,
    auto_spells: Vec<AutoSkillEntry>,
    stances: Vec<StanceDef>,
    passives: Vec<PassiveDef>,
    multi_classing: MultiClassingConfig,
}

struct SkillAccessEntry {
    id: String,
    max_rank: String,                  // formula like "level+3"
}

struct AutoSkillEntry {
    id: String,
    level: u8,
}

struct StanceDef {
    id: String,
    name: String,
    ac_bonus: i8,
    attack_penalty: i8,
    damage_bonus: i8,
    ac_penalty: i8,
    min_level: u8,
}

struct PassiveDef {
    id: String,
    name: String,
    description: String,
    effect: EffectTemplate,
    min_level: u8,
}

struct MultiClassingConfig {
    favored: bool,
}
```

---

## Skill Caps & Training

### Skill Category System

Every class template defines three skill access categories — `class_skills`,
`cross_class_skills`, and `exclusive_skills`. These determine the maximum
rank a character can achieve in each skill at a given level.

```rust
fn max_rank(skill_id: &str, class: &ClassTemplate, level: u8) -> u16 {
    if class.exclusive_skills.iter().any(|s| s.id == skill_id) {
        level + 3
    } else if class.class_skills.iter().any(|s| s.id == skill_id) {
        level + 3
    } else if class.cross_class_skills.iter().any(|s| s.id == skill_id) {
        (level + 3) / 2
    } else {
        0  // unavailable
    }
}
```

The formula string (e.g. `"level+3"`, `"(level+3)/2"`) is stored in the
template and evaluated at runtime. Custom formulas are possible through
Rhai scripts (e.g. `"min(level+5, 50)"`).

### Training Cost

Skills are trained via the `train` command at a trainer NPC:

```
train <skill>
```

Cost formula (defined in `mud.toml`):

```toml
[training]
base_cost = 100                          # gp at level 1, skill rank 0
cost_per_level = 50                      # additional gp per character level
cost_per_rank = 25                       # additional gp per skill rank
cost_multiplier = 1.0                    # global multiplier (e.g., 2.0 for prestige skills)
```

```
total_gold = (base_cost + level * cost_per_level + current_rank * cost_per_rank) * cost_multiplier
```

### Skill Tree Prerequisites

Skills can require other skills at minimum ranks before they can be trained:

```toml
# content/skills/combat/shield_bash.toml
requires_skill = [{ id = "bash", level = 3 }]
```

The engine resolves the skill prerequisite DAG at training time:

```
train shield_bash
  → check: bash rank >= 3?
  → if no: "You need Bash (rank 3) to train Shield Bash."
  → if yes: show training cost, proceed
```

The validator checks for circular prerequisites at content load time:

- Walk each skill's `requires_skill` chain
- If any skill appears twice in the same chain, log error and skip

### Trainer NPCs

Trainer NPCs have a `trainer_types` field in their template that lists
which skill categories or specific skills they can teach:

```toml
# content/mobs/weapon_master.toml
trainer_types = ["weapon_master", "trainer"]
```

A skill's `trainer_types` field lists which trainer types can teach it:

```toml
# content/skills/combat/shield_bash.toml
trainer_types = ["weapon_master", "trainer"]
```

The intersection of NPC trainer_types and skill trainer_types determines
what a given NPC can teach. The `train` command without arguments shows
a filtered menu of trainable skills for that NPC.

---

## Portal & Teleport Skills

Skills with `skill_type = "magic"` (or other types) can create temporary portals
or teleport entities at runtime. These are not defined as special skill types —
they use the generic `MagicConfig` / effect system and check room flags for permission.

### Portal Creation Flow

1. Player activates a portal skill targeting a destination room
2. System checks source room `RoomFlags(ROOM_PORTAL_OUT)` — fail if absent
3. System checks destination room `RoomFlags(ROOM_PORTAL_IN)` — fail if absent
4. Creates a `TempPortal` on the source room pointing to the destination
5. Optionally creates a return `TempPortal` on the destination

Temporary portals are added to the source room's `TempPortals` component and
expire after a skill-defined duration. They are usable via the `enter` command,
same as permanent template-defined portals.

### Teleport Flow

1. Player activates a teleport skill
2. System checks source room `RoomFlags(ROOM_NO_TELEPORT_OUT)` — blocked if set
3. Gathers all one-hop adjacent rooms (reachable via exits from the source room)
4. Filters candidates by `RoomFlags(ROOM_NO_TELEPORT_IN)` — blocked rooms excluded
5. If targeting another entity, checks target's `Teleportable` component — fail if false
6. Picks a valid candidate at random, moves the target entity
7. If no valid candidates, the spell fails ("You concentrate but find no suitable destination.")

Range is limited to rooms directly adjacent via exit. Marked rooms use
`Teleportable(false)` to opt out of being teleported by others (players toggle
via `config teleport`, NPC templates set `teleportable = false`).

---

## Stances & Passives

### Stances

Stances are toggleable combat modes defined in class templates. They
provide trade-offs: a bonus to one stat at the cost of a penalty to another.

**Template definition:**

```toml
[[stances]]
id = "defensive"
name = "Defensive Stance"
ac_bonus = 2
attack_penalty = -2
min_level = 1

[[stances]]
id = "offensive"
name = "Offensive Stance"
damage_bonus = 2
ac_penalty = -2
min_level = 3
```

**Runtime:**

```rust
struct ActiveStance(Option<String>);
```

| Command | Effect |
|---|---|
| `stance` | Show current stance and available stances |
| `stance <name>` | Activate stance (deactivates previous) |
| `stance default` | Return to default (no stance mods) |

**Rules:**
- One active stance at a time
- Cannot change stance while in combat unless the stance's `combat_switch = true`
- Stance effects are applied immediately on activation, removed on deactivation
- A `StanceSystem` (runs on Combat phase) applies modifiers to combat calculations

### Passives

Passives are always-on class bonuses applied as ECS components at login,
level-up, or class grant.

**Template definition:**

```toml
[[passives]]
id = "warrior_strength"
name = "Warrior's Strength"
description = "Adds +1 damage per 4 levels."
effect = { type = "script", script_id = "warrior_strength.rhai" }
min_level = 1
```

**Runtime:**

```rust
struct PassiveEffect {
    id: String,
    effect: EffectTemplate,
}
```

A `PassiveApplicationSystem` (runs on login and level-up):

1. Query entities with `MultiClassInfo` (or `Level` + single class)
2. For each class entry, load class template's `[passives]`
3. Check `min_level` against current level
4. Add missing `PassiveEffect` components, remove expired ones
5. Skip passives already present (idempotent)

Passives stack across classes for multi-classed characters unless the
passive has `stackable = false`.

---

## Prestige Classes

### Definition

Prestige classes are specialized class templates with a `prestige = true`
flag and a `[prestige_gate]` block that defines the requirements to enter
them. They are defined in `content/classes/` alongside base classes.

```toml
# content/classes/assassin.toml
name = "Assassin"
description = "A shadowy killer."
attribute_mods = { str = 1, dex = 3, int = 1, wis = -1, con = 0, cha = 0 }
hit_die = 6
prestige = true

[prestige_gate]
requires_class = "thief"
requires_level = 5
requires_skills = [
    { id = "sneak", rank = 50 },
    { id = "backstab", rank = 30 },
]
requires_race = "human"
requires_alignment = "evil"
requires_quest = "assassins_guild_initiation"
requires_faction = { id = "assassins_guild", standing = 100 }
```

### Prestige Gate Fields

| Field | Type | Description |
|---|---|---|
| `requires_class` | String | Base class the character must have levels in |
| `requires_level` | u8 | Minimum character level |
| `requires_skills` | `[{id, rank}]` | Minimum skill ranks in specific skills |
| `requires_race` | String (optional) | Racial requirement |
| `requires_alignment` | String (optional) | Alignment requirement |
| `requires_quest` | String (optional) | Quest that must be completed |
| `requires_faction` | `{id, standing}` (optional) | Faction standing requirement |

### Prestige Flow

```
@prestige <class>
  → validate prestige_gate:
      • character has requires_class in MultiClassInfo
      • character.level >= requires_level
      • all requires_skills met (LearnedSkills rank >= required)
      • race matches (if specified)
      • alignment matches (if specified)
      • quest completed (if specified, checked against QuestLog)
      • faction standing >= required (if specified, checked against FactionStanding)
  → if all pass:
      • add class to MultiClassInfo.classes
      • apply prestige class's auto_skills/auto_spells
      • apply prestige class's passives
      • add prestige class's stances to available list
      → "You have taken the path of the Assassin!"
  → if any fail:
      → "You do not meet the requirements: [list all failures]"
```

Prestige classes can also be gained through roleplay triggers (quest
completion, faction achievement) via Rhai scripts:

```rust
// grant prestige class via script
world.call_fn::<_, ()>("grant_prestige", (entity_id, "assassin"))?;
```

### Rules

- A character can have multiple prestige classes (subject to DM discretion)
- Prestige class levels count toward total character level
- Prestige class `auto_skills` and `auto_spells` stack with base class grants
- If a prestige class's requirements are lost (alignment change, faction
  drop), the character keeps existing levels but cannot gain more until
  requirements are restored

### Rust Struct

```rust
struct PrestigeGate {
    requires_class: Vec<String>,
    requires_level: u8,
    requires_skills: Vec<PrerequisiteSkill>,
    requires_race: Vec<String>,
    requires_alignment: Vec<String>,
    requires_quest: Vec<String>,
    requires_faction: Vec<PrerequisiteFaction>,
}

struct PrerequisiteSkill {
    id: String,
    rank: u8,
}

struct PrerequisiteFaction {
    id: String,
    standing: i32,
}
```

---

## Multi-Classing

### Overview

Multi-classing allows a character to gain levels in multiple classes.
Each class is tracked independently in `MultiClassInfo`:

```rust
struct MultiClassInfo {
    classes: Vec<ClassEntry>,
}

struct ClassEntry {
    class_id: String,
    level: u8,
    is_favored: bool,
}
```

### XP Penalty Formula

XP penalty discourages unfocused class combinations. Formula (defined in
`mud.toml`):

```toml
[multi_classing]
xp_penalty_pct_per_class = 20      # 20% penalty per non-favored class
xp_penalty_max = 80                 # cap at 80% penalty
favored_class_empty_slot = true     # empty favored class slot waives penalty
```

```
penalty = (non_favored_class_count - 1) * xp_penalty_pct_per_class
if penalty > xp_penalty_max, penalty = xp_penalty_max
effective_xp = base_xp * (100 - penalty) / 100
```

- A character with 1 non-favored class (2 total, 1 non-favored): 0% penalty
  (only the second class counts)
- A character with 2 non-favored classes: 20% penalty
- A character with 4 non-favored classes: 60% penalty
- A character with 5 non-favored classes: 80% penalty (capped)

### Favored Class

Each race-class combination can mark a class as `favored`:

```toml
# In class template:
[multi_classing]
favored = true
```

If the race also defines a favored class through `allowed_classes`
ordering, the first listed class is considered "culturally favored."
The `is_favored` flag on `ClassEntry` is set at creation time based on
the class template's `favored` field.

A character can have exactly one favored class at a time. If they have
levels in their favored class, it does not count toward the penalty.

### Alignment Restrictions

Changing alignment to one not in a class's `allowed_alignments` list:

1. Immediate warning: `"Your alignment no longer suits the warrior path."`
2. No new levels can be gained in that class until alignment is restored
3. Class features (stances, passives) remain active (no feature loss —
   at admin discretion, a Rhai script can strip features on violation)

### Gaining a New Class

```
@multi_class <class>
  → check: not already in MultiClassInfo (no duplicates)
  → check: class.allowed_races contains character's race
  → check: character's alignment is in class.allowed_alignments
  → check: if prestige, run prestige_gate validation
  → if all pass:
      • add ClassEntry { class_id, level: 1, is_favored }
      • apply class's auto_skills for level 1
      • apply class's passives for level 1
      → "You have begun your training as a Mage."
```

### Multi-Class Leveling

On level-up, the player chooses which class to advance:

```
You have enough XP to level!
Pick a class to advance:
  [1] Warrior (level 5)
  [2] Mage (level 3)
> 2

You gain a level as a Mage!
  +4 HP (d4 + con_mod)
  +1 mana (d4 + int_mod)
  New skill: magic_missile
```

The chosen class's hit die, skill points, and auto-learns are applied.
The class entry's `level` increments. Total character level is the sum
of all class entry levels.

---

## Experience & Leveling

### XP Curve

```
Formula:  XP(level) = level³ × 100

Level  1:      100 XP       (cumulative:     100)
Level  2:      800 XP       (cumulative:     900)
Level  5:   12,500 XP       (cumulative:  18,700)
Level 10:  100,000 XP       (cumulative: 384,000)
Level 25: 1,562,500 XP      (cumulative: ~5.7M)
Level 50: 12,500,000 XP      (cumulative: ~212M)
```

Level cap is configurable (default 100).

### XP Sources

| Source | Formula | Notes |
|---|---|---|
| Kill mob | `victim.level² × 50 × xp_multiplier` | Split among party |
| Quest reward | Per-quest definition | One-time or repeatable |
| Explore room | `5 × room.level` | First discovery only |
| Craft item | `item.level² × 10` | Per successful craft |
| Group bonus | +10% per member (max +50%) | Same room |
| Danger bonus | +25% | Mob is aggressive to player level |

### Level-Up

When `current_xp >= xp_for_next_level()`:

1. **HP gain:** Roll hit die + CON modifier (e.g. warrior: `d10 + con_mod`)
2. **Skill unlock:** Check class definition for `level_requirement`
3. **Attribute point:** +1 to any stat, every 5 levels (player choice)
4. **Mana gain (casters):** `d4 + int_mod` additional max mana
5. Display level-up message with summary of gains
6. Full heal + restore mana

### Death Penalty

```toml
[game]
xp_loss = 0.10        # lose 10% of current XP on death (default)
xp_loss_level_cap = 5  # never lose more than 5 levels worth of XP
```

Death penalty can never de-level below `current_level`. XP floor is
`xp_for_level(current_level)`. At level cap, XP loss is waived.

### Components

```rust
struct Experience(u64);

impl Experience {
    fn for_level(level: u8) -> u64 { (level as u64).pow(3) * 100 }
    fn to_next_level(&self, level: u8) -> u64 {
        Self::for_level(level + 1).saturating_sub(self.0)
    }
}

struct LevelUpReward {
    new_level: u8,
    hp_gain: i32,
    mana_gain: i32,
    unlocked_skills: Vec<String>,
    attribute_points: u8,
}
```

### SQL

```sql
ALTER TABLE characters ADD COLUMN xp_loss_pct REAL NOT NULL DEFAULT 0.10;
```

---

## Item System

### Item Template

Items are defined in `content/items/*.toml`:

```toml
# content/items/rusty_sword.toml
id = "rusty_sword"
name = "rusty sword"
description = "A battered blade, nicked and stained."
item_type = "weapon"
subtype = "longsword"
quality = "common"
level_requirement = 1
weight = 3.0
value = 25
flags = ["no_sell"]

[attributes]
strength_requirement = 10

# Restrictions (optional — class/race/alignment/skill gates)
allowed_classes = ["warrior", "paladin", "fighter"]
allowed_races = []
allowed_alignments = []
requires_skill = { id = "longsword", level = 5 }

# Item set membership (optional)
[set]
id = "templar_armor"
piece_type = "weapon"    # armor | weapon | shield | jewelry

# Triggers (optional)
[[triggers]]
event = "on_hit"
chance = 10
cast = "fireball"
target = "attacker"

[[triggers]]
event = "on_wear"
cast = "bless"
target = "self"

[weapon]
damage = { count = 1, sides = 8, type = "slash" }
speed = 2.5
range = "melee"

[equipment]
slot = "Weapon"
```

### Item Quality

| Quality | Stat multiplier | Prefix examples |
|---|---|---|
| `poor` | 0.75× | "crude", "worn", "chipped" |
| `common` | 1.0× | (none) |
| `magic` | 1.5× | "glowing", "enchanted" |
| `rare` | 2.0× | "runed", "mithril" |
| `legendary` | 3.0× | "ancient", "god-forged" |

### Item Types

| Type | Behavior |
|---|---|
| `weapon` | Damage dice, speed, range, weapon skill |
| `armor` | AC bonus, slot, material, skill penalty |
| `container` | Capacity (weight/items), lock ID |
| `potion` | Effect on drink, charges, sip timer |
| `scroll` | Contains a spell, one-use, scribeable |
| `wand` | Spell charges, rechargeable |
| `food` | Satiation value, eat timer, effects |
| `drink` | Liquid type, capacity, drink timer |
| `key` | Opens specific lock ID(s) |
| `quest` | No-drop, no-sell, quest interaction |
| `treasure` | High value, no use other than selling |
| `light` | Light radius, fuel/duration |
| `furniture` | Sitting/sleeping capacity, comfort |

### Item Affixes (Enchantments)

For magic+ items, affixes are randomly rolled or hand-placed:

```toml
# content/items/flame_sword.toml
affixes = [
    { type = "damage", element = "fire", amount = "1d6" },
    { type = "stat",   stat = "strength", amount = 2 },
]
```

Affix definitions in `content/affixes.toml`:

```toml
[affix.damage_fire]
name = "of Flame"
description = "Wreathed in flickering flames"
type = "damage"
element = "fire"
amount = "1d6"
quality_min = "magic"
slot = ["Weapon"]
weight = 50
```

### Durability & Repair

```rust
struct Durability {
    current: u16,
    max: u16,
    decay_rate: f32,
}
```

Weapons lose durability on hit. Armor loses durability on being hit.
At `current == 0`, the item is **broken** (no stats) until repaired.
Repair via NPC blacksmith (`repair <item>`) or the `repair` skill.

### Extra Components

```rust
struct Weapon {
    damage: DamageDice,
    speed: f32,
    range: WeaponRange,
}

enum WeaponRange { Melee, Ranged, Reach, Thrown }

struct Armor {
    ac_bonus: i32,
    slot: EquipmentSlot,
    material: Material,
    skill_penalty: i32,
}

struct Container {
    capacity_weight: f32,
    capacity_items: u16,
    lock_id: Option<String>,
    is_locked: bool,
}

enum Material { Cloth, Leather, Metal, Mithril, Adamantium, Dragonhide, Wood }
```

### Item Restrictions

Every restriction gate is optional. If a field is empty or absent, the gate
is not enforced.

| Gate | Checked | Failure behavior |
|---|---|---|
| `allowed_classes` | On equip/wield | `You are not proficient with this item.` |
| `allowed_races` | On equip/wield | `This item does not fit your anatomy.` |
| `allowed_alignments` | On equip/wield | `The item rejects your touch.` |
| `requires_skill` | On equip/wield + continuous | `You lack the skill to use this.` |

Runtime flow for `wear` / `wield`:

```
1. Check allowed_classes → character's class(es) intersect
2. Check allowed_races → character's race in list
3. Check allowed_alignments → character's alignment in list
4. Check requires_skill → character's skill rank >= required level
5. If all pass: equip. If any fail: reject with specific message.
```

`requires_skill` is also checked continuously by a
`SkillRequirementSystem` (Runs on DirtyFlush phase). If a player's
skill rank drops below the requirement (e.g. due to drain or curse),
the item is auto-removed with a message.

### Item Triggers

Items can trigger skill executions on specific events. Triggers are
defined in the item template:

```toml
[[triggers]]
event = "on_hit"
chance = 10
cast = "fireball"
target = "attacker"
```

| Event | When it fires | Target options |
|---|---|---|
| `on_hit` | When the wielder successfully hits a target | `attacker`, `target`, `room` |
| `on_wear` | When the item is equipped | `self` |
| `on_remove` | When the item is unequipped | `self` |
| `on_use` | When the item is activated (e.g. wand, potion) | `self`, `target` |
| `on_kill` | When the wielder kills a target | `self`, `room` |
| `on_damage_taken` | When the wearer takes damage | `self`, `attacker` |

Chance is rolled per-trigger, per-event occurrence. All triggers for
a given event are evaluated in order; each trigger that passes its
chance roll fires independently.

Triggered skills reference the unified `SkillDef` by ID. The skill
is executed as if the character used it, respecting cooldowns and
resource costs (set to 0 for triggered executes unless specified
otherwise).

### Random Loot Parameters

For items intended as random drops, optional loot generation parameters
control quality and affix rolling:

```toml
[loot]
min_quality = "magic"
max_quality = "rare"
min_affixes = 1
max_affixes = 2
weight = 10
```

| Field | Default | Description |
|---|---|---|
| `min_quality` | `common` | Lowest quality tier this drop can roll |
| `max_quality` | `common` | Highest quality tier this drop can roll |
| `min_affixes` | 0 | Minimum number of random affixes |
| `max_affixes` | 0 | Maximum number of random affixes |
| `weight` | 1 | Relative weight within loot tables |

When a mob with a loot table dies, the system:
1. Rolls for each item entry (weighted random)
2. For each won item, rolls quality between min/max
3. Rolls `n` affixes where `n = rand(min, max)`, selecting from
   affixes where `quality_min <= rolled_quality` and `slot` matches
4. Spawns the item with the rolled quality and affixes

### Item Sets

Defined in `content/sets.toml`:

```toml
[set.templar_armor]
name = "Templar Armor Set"

# Each bonus tier activates when conditions are met
[[set.templar_armor.bonuses]]
min_pieces = 2
effects = [{ type = "stat", stat = "constitution", amount = 2 }]

[[set.templar_armor.bonuses]]
min_pieces = 4
effects = [
    { type = "stat", stat = "strength", amount = 2 },
    { type = "damage_reduction", amount = 3 },
]

[[set.templar_armor.bonuses]]
min_pieces = 6
conditions = [
    { piece_type = "armor", min = 4 },
    { piece_type = "weapon", min = 1 },
    { piece_type = "shield", min = 1 },
]
effects = [
    { type = "stat", stat = "all", amount = 3 },
    { type = "aura", aura_id = "holy_radiance", radius = 20 },
]
```

Each tier has:
- `min_pieces` — minimum total equipped pieces from this set
- `conditions` (optional) — array of `{ piece_type, min }` constraints.
  All conditions must be satisfied for the tier to activate.
- `effects` — array of effects applied while the tier is active

Items declare set membership via `[set]` in their template:

```toml
# content/items/templar_helm.toml
id = "templar_helm"
name = "Templar Helm"
[set]
id = "templar_armor"
piece_type = "armor"
```

Runtime tracking uses `SetTracker` component:

```rust
struct SetTracker {
    active_sets: HashMap<String, ActiveSet>,
}

struct ActiveSet {
    template_id: String,
    counts: HashMap<String, u16>,   // piece_type → count
    equipped: HashSet<String>,       // item template IDs (prevent double-count)
    active_tiers: Vec<usize>,        // indices into set.bonuses
}
```

**Flow on wear/remove:**

1. Scan equipped items for `[set]` blocks
2. Group by `set_id`, count by `piece_type`, deduplicate by template_id
3. For each bonus tier: evaluate `min_pieces` + all `conditions`
4. Apply newly-activated effects, remove deactivated ones
5. Emit `SetBonusChanged { set_id, tiers }` event

### Item Commands

```
examine <item>               — detailed info (affixes, durability, damage, set info)
inventory / i                — list carried items
equipment / eq               — list worn items (shows active set bonuses)
wear <item>                  — equip to appropriate slot
wield <item>                 — equip weapon
remove <item>                — unequip
get <item> [from]            — pick up from ground or container
drop <item>                  — drop to ground
put <item> in <container>    — store in container
give <item> to <player>      — transfer to another player
```

### SQL Tables

```sql
CREATE TABLE components_item (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    template_id TEXT NOT NULL,
    flags INTEGER NOT NULL DEFAULT 0,
    quality TEXT NOT NULL DEFAULT 'common'
);

CREATE TABLE components_durability (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    current INTEGER NOT NULL,
    max INTEGER NOT NULL,
    decay_rate REAL NOT NULL DEFAULT 1.0
);

CREATE TABLE components_affixes (
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    affix_index INTEGER NOT NULL,
    affix_id TEXT NOT NULL,
    PRIMARY KEY (entity_id, affix_index)
);
```

### Rust Structs

```rust
struct ItemTemplate {
    id: String,
    name: String,
    description: String,
    item_type: String,
    subtype: String,
    quality: String,
    level_requirement: u8,
    weight: f32,
    value: u64,
    flags: Vec<String>,
    attributes: HashMap<String, u8>,
    allowed_classes: Vec<String>,
    allowed_races: Vec<String>,
    allowed_alignments: Vec<String>,
    requires_skill: Option<SkillRequirement>,
    weapon: Option<WeaponDef>,
    equipment: Option<EquipmentDef>,
    triggers: Vec<TriggerDef>,
    affixes: Vec<AffixRef>,
    loot: Option<LootParams>,
    set: Option<SetMembership>,
}

struct SkillRequirement { id: String, level: u8 }

struct WeaponDef {
    damage: DamageDice,
    speed: f32,
    range: String,
}

struct DamageDice {
    count: u8,
    sides: u8,
    damage_type: String,
}

struct EquipmentDef {
    slot: String,
}

struct TriggerDef {
    event: String,
    chance: u8,
    cast: String,
    target: String,
}

struct AffixRef {
    affix_type: String,
    element: Option<String>,
    stat: Option<String>,
    amount: String,
}

struct LootParams {
    min_quality: String,
    max_quality: String,
    min_affixes: u8,
    max_affixes: u8,
    weight: u8,
}

struct SetMembership {
    id: String,
    piece_type: String,
}

struct AffixDef {
    id: String,
    name: String,
    description: String,
    affix_type: String,
    element: Option<String>,
    amount: String,
    quality_min: String,
    slot: Vec<String>,
    weight: u8,
}

struct SetDef {
    id: String,
    name: String,
    bonuses: Vec<SetBonus>,
}

struct SetBonus {
    min_pieces: u8,
    conditions: Vec<SetCondition>,
    effects: Vec<SetEffect>,
}

struct SetCondition {
    piece_type: String,
    min: u8,
}

struct SetEffect {
    effect_type: String,
    stat: Option<String>,
    amount: Option<i32>,
    aura_id: Option<String>,
    radius: Option<u8>,
}
```

---

## Mob Templates

### Design Principle

NPCs (mobs) are defined as TOML templates in `content/mobs/*.toml`. The engine
has zero baked-in knowledge of specific mobs. Each template defines stats,
behavior, equipment, loot, faction alignment, skills, and script hooks.

### Mob Template

```toml
# content/mobs/goblin.toml
id = "goblin"
name = "goblin"
description = "A {green}green-skinned{/} goblin."
level = 1
attributes = { str = 10, dex = 12, int = 8, wis = 8, con = 10, cha = 6 }
health = { current = 20, max = 20 }
armor = 10

# Natural damage (if unarmed)
damage = { count = 1, sides = 4, type = "pierce" }

race = "goblinoid"
size = "small"

# Starting equipment (spawned with the NPC)
[[equipment]]
template_id = "rusty_dagger"
slot = "Weapon"

[[equipment]]
template_id = "tattered_robe"
slot = "Torso"

# XP granted on kill
xp_value = 100

# Loot table
[[loot.entries]]
item = "copper_coin"
count = { min = 1, max = 10 }
chance = 100

[[loot.entries]]
item = "rusty_dagger"
chance = 25

[[loot.entries]]
item = "goblin_ear"
chance = 50

# AI behavior
ai_mode = "wander"            # idle | wander | patrol | stationary
aggro_range = 10
aggro_players = true
aggro_race = ["elf"]

# Faction alignment
faction = "goblin_tribe"
faction_standing = 0

# Trainer types (empty = not a trainer)
trainer_types = []

# Languages spoken
languages = ["goblin"]

# Known skills (auto-granted at spawn)
[[skills]]
id = "backstab"
level = 3

# Script hooks
[[scripts]]
event = "death"
script = "goblin_guard.rhai"

[[scripts]]
event = "enter"
script = "goblin_alert.rhai"
```

### Rust Struct

```rust
struct MobTemplate {
    id: String,
    name: String,
    description: String,
    level: u8,
    attributes: Attributes,
    health: HealthBounds,
    armor: i32,
    damage: DamageDice,
    race: Option<String>,
    size: Size,
    equipment: Vec<MobEquipment>,
    xp_value: u64,
    loot: Option<LootTable>,
    ai_mode: AiMode,
    aggro_range: u8,
    aggro_players: bool,
    aggro_race: Vec<String>,
    faction: Option<String>,
    faction_standing: i32,
    trainer_types: Vec<String>,
    languages: Vec<String>,
    skills: Vec<MobSkill>,
    scripts: Vec<ScriptHook>,
}

struct HealthBounds {
    current: i32,
    max: i32,
}

struct MobEquipment {
    template_id: String,
    slot: EquipmentSlot,
}

struct LootTable {
    entries: Vec<LootEntry>,
}

struct LootEntry {
    item: String,
    count: Option<CountRange>,
    chance: u8,
    loot_params: Option<LootQualityParams>,
}

struct CountRange {
    min: u32,
    max: u32,
}

enum AiMode {
    Idle,
    Wander,
    Patrol,
    Stationary,
}

struct MobSkill {
    id: String,
    level: u8,
}

struct ScriptHook {
    event: String,
    script: String,
}
```

### Loot System

When a mob dies, the `LootSystem` (runs on MobDied event):

1. Query `MobTemplate.loot` from registry
2. For each `LootEntry`, roll `chance` (0–100)
3. If pass, roll `count` between `min` and `max`
4. If `loot_params` present, roll quality and affixes
5. Spawn item entities, transfer to corpse's `Inventory`
6. Items without count field default to 1

Loot tables can also reference `treasure_classes` (defined in
`content/treasure_classes.toml`) for shared loot groups:

```toml
# content/treasure_classes.toml
[treasure_class.jewelry]
min_quality = "magic"
max_quality = "legendary"
entries = [
    { item = "gold_ring", weight = 10 },
    { item = "silver_necklace", weight = 5 },
    { item = "gemstone", weight = 20 },
]
```

```toml
# Referenced from mob loot:
[[loot.entries]]
treasure_class = "jewelry"
chance = 5
```

### Mob Spawns in Areas

Area templates reference mobs by template ID:

```toml
# Inside area template:
[mobs.midgaard.square]
spawns = [
    { mob = "midgaard_guard", count = 2, respawn_secs = 60 },
    { mob = "wandering_merchant", count = 1, respawn_secs = 300 },
]
```

The `AreaResetSystem` uses these spawn tables to populate rooms on area reset
and to respawn killed mobs after `respawn_secs`.

---

## Shop & Economy

### Currency

Three-tier decimal system tracked in copper pieces (`Wallet` component):

| Coin | Value |
|---|---|
| Copper (cp) | 1 |
| Silver (sp) | 100 cp |
| Gold (gp) | 10,000 cp (100 sp) |

```rust
struct Wallet {
    copper: u64,
    banked_copper: u64,
}
```

### NPC Shops

Shops are entities with a `Shop` component:

```rust
struct Shop {
    name: String,
    buy_rate: f32,
    sell_rate: f32,
    inventory: Vec<ShopItem>,
    currency: u64,
    restock_secs: u32,
}

struct ShopItem {
    template_id: String,
    count: u32,
    price_override: Option<u64>,
    unlimited: bool,
}
```

### Shop Commands

```
list                          — browse shop inventory
list <category>               — filter (weapon, armor, potion)
buy <item> [count]            — purchase
sell <item>                   — sell to shop
buyback <item>                — re-purchase last-sold (session only)
value <item>                  — ask shop's estimated price
deposit <amount>              — bank deposit
withdraw <amount>             — bank withdrawal
balance                       — check bank balance
```

### Price Formula

```
buy_price   = base_value × sell_rate × reputation_multiplier
sell_price  = base_value × buy_rate  × reputation_multiplier

Reputation multiplier:
  adored:    0.80 ×   (standing ≥ 900)
  friendly:  0.90 ×   (standing ≥ 500)
  neutral:   1.00 ×   (standing 0–499)
  unfriendly: 1.25 ×  (standing > -500)
  hostile:   1.50 ×   (standing ≤ -500)
```

### SQL Tables

```sql
CREATE TABLE components_wallet (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    copper INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE components_shop (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    name TEXT NOT NULL,
    buy_rate REAL NOT NULL DEFAULT 0.5,
    sell_rate REAL NOT NULL DEFAULT 1.5,
    currency INTEGER NOT NULL DEFAULT 100000,
    restock_secs INTEGER NOT NULL DEFAULT 300
);

CREATE TABLE shop_inventory (
    shop_id INTEGER NOT NULL REFERENCES entities(id),
    template_id TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 1,
    price_override INTEGER,
    unlimited INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (shop_id, template_id)
);

### Shop Templates (File-Based)

Shops can also be defined as file-based TOML templates under `content/shops/`,
loaded into a `ShopTemplate` registry and instantiated at server start or
area reset:

```toml
# content/shops/blacksmith.toml
id = "midgaard_blacksmith"
name = "The Heavy Hammer"
npc = "blacksmith_npc"            # NPC template ID that runs this shop
buy_rate = 0.5
sell_rate = 1.5
currency = 50000
restock_secs = 300

[[inventory]]
item = "iron_sword"
count = 3
price_override = 200

[[inventory]]
item = "repair_kit"
count = 10
unlimited = true

[[inventory]]
item = "leather_armor"
count = 5

[[inventory]]
item = "healing_potion"
count = 20
unlimited = true
price_override = 50
```

```rust
struct ShopTemplate {
    id: String,
    name: String,
    npc: String,                      // mob template ID
    buy_rate: f32,
    sell_rate: f32,
    currency: u64,
    restock_secs: u32,
    inventory: Vec<ShopInventoryEntry>,
}

struct ShopInventoryEntry {
    item: String,                     // item template ID
    count: u32,
    price_override: Option<u64>,
    unlimited: bool,
}
```

**Instantiation flow:** On server start (or area load), for each `ShopTemplate`
whose `npc` mob is present in the loaded area, a `Shop` ECS component is
created on the NPC entity using the template's values.

---

## Crafting System

### Design Principle

Crafting is entirely data-driven. Recipes are TOML files in `content/recipes/`.
The engine provides the crafting UI and material consumption logic; what can be
crafted and how is defined by recipe files.

### Recipe Template

```toml
# content/recipes/iron_sword.toml
id = "iron_sword"
name = "Forge Iron Sword"
station = "anvil"
skill = { id = "smithing", level = 5 }
difficulty = 15

materials = [
    { item = "iron_ingot", count = 3 },
    { item = "leather_strip", count = 1 },
    { item = "wooden_handle", count = 1 },
]

result = { item = "iron_sword", count = 1, quality = "common" }
success_chance = 80

[quality_scaling]
margin_per_point = 5
max_quality = "rare"

script = "craft_iron_sword.rhai"
```

### Rust Struct

```rust
struct RecipeDef {
    id: String,
    name: String,
    station: String,
    skill: RecipeSkill,
    difficulty: u8,
    materials: Vec<CraftMaterial>,
    result: CraftResult,
    success_chance: u8,
    quality_scaling: Option<QualityScaling>,
    script: Option<String>,
}

struct RecipeSkill { id: String, level: u8 }
struct CraftMaterial { item: String, count: u32 }
struct CraftResult { item: String, count: u32, quality: String }
struct QualityScaling { margin_per_point: u8, max_quality: String }
```

### Crafting Flow

```
craft <recipe>
  → Check: recipe known? (LearnedRecipes component)
  → Check: station present in room?
  → Check: skill rank >= skill.level?
  → Check: materials in inventory?
  → Roll success: random(0..100) < success_chance + skill_margin
      • Success:
          → Roll quality: if skill margin / margin_per_point >= threshold, upgrade
          → Consume materials, spawn result item
          → Grant XP: recipe.result.level² × 10
      • Failure:
          → Consume 50% of materials (configurable)
      • Critical failure (natural 1):
          → All materials consumed
```

Stations are room flags (`room_flags = ["station:anvil"]`) or entities with:

```rust
struct Station { station_type: String, quality_bonus: i8 }
```

### Known Recipes

```rust
struct LearnedRecipes { recipes: Vec<String> }
```

Recipes are learned from auto-grant, trainer NPCs, recipe scroll drops, or
Rhai `grant_recipe()`.

---

## Quest System

### Quest Template

```toml
# content/quests/goblin_problem.toml
id = "goblin_problem"
name = "Goblin Problem"
description = "Clear out the goblins."
level_requirement = 1
repeatable = false
auto_complete = false
giver_npc = "village_elder"
turn_in_npc = "village_elder"

[[prerequisites]]
type = "level"
level = 1

[[objectives]]
type = "kill"
mob = "goblin"
count = 5

[[objectives]]
type = "gather"
item = "goblin_ear"
count = 5

[[rewards]]
type = "xp"
amount = 500

[[rewards]]
type = "gold"
amount = 100

[[rewards]]
type = "item"
item = "rusty_sword"
count = 1

[[rewards]]
type = "faction"
faction = "village"
standing = 50

[scripts]
on_accept = "goblin_accept.rhai"
on_complete = "goblin_complete.rhai"
```

### Rust Struct

```rust
struct QuestDef {
    id: String,
    name: String,
    description: String,
    level_requirement: u8,
    repeatable: bool,
    auto_complete: bool,
    giver_npc: Option<String>,
    turn_in_npc: Option<String>,
    prerequisites: Vec<QuestPrerequisite>,
    objectives: Vec<QuestObjective>,
    rewards: Vec<QuestReward>,
    scripts: HashMap<String, String>,
}

enum QuestPrerequisite {
    Level(u8),
    Quest(String),
    Faction { id: String, standing: i32 },
    Skill { id: String, rank: u8 },
    Item { id: String, count: u32 },
}

enum QuestObjective {
    Kill { mob: String, count: u32, room_area: Option<String> },
    Gather { item: String, count: u32 },
    Deliver { item: String, target_npc: String },
    Explore { room: String },
    Talk { npc: String },
    Escort { npc: String, destination: String },
    Craft { item: String, count: u32, station: Option<String> },
    Use { skill: String, count: u32 },
}

enum QuestReward {
    Xp(u64),
    Gold(u64),
    Item { id: String, count: u32 },
    Faction { id: String, standing: i32 },
    Skill { id: String, rank: u8 },
    Recipe(String),
}
```

### Objective Types

| Type | Auto-update via |
|---|---|
| `kill` | `MobDied` event |
| `gather` | `ItemPickedUp` event |
| `deliver` | `give` command handler |
| `explore` | `PlayerMoved` event |
| `talk` | `NpcSaid` event |
| `escort` | Periodic script check |
| `craft` | `ItemCrafted` event |
| `use` | `SkillUsed` event |

### Quest Runtime

```rust
struct QuestLog {
    active: HashMap<String, QuestProgress>,
    completed: Vec<String>,
}

struct QuestProgress {
    quest_id: String,
    objectives: Vec<ObjectiveState>,
    started_at: Instant,
}

struct ObjectiveState {
    objective_index: usize,
    current: u32,
    completed: bool,
}
```

**Flow:** NPC offers → accept → progress tracked via events → all objectives
done → auto-complete or turn-in → rewards delivered → `on_complete` script.

### Commands

```
quests                    — show active quests
quest <id>                — show quest details + progress
quest abandon <id>        — abandon quest (with confirmation)
```

---

## Faction System

### Design Principle

Factions track numeric standing that gates access, affects prices, triggers
aggro, and acts as prerequisites for quests and prestige classes.

### Faction Template

```toml
# content/factions/village.toml
id = "village"
name = "Village of Midgaard"
description = "The peaceful villagers of Midgaard."
starting_standing = 0
min_standing = -1000
max_standing = 1000

[ranks]
"-1000" = "Hated"
"-500"  = "Hostile"
"-100"  = "Unfriendly"
"0"     = "Neutral"
"100"   = "Friendly"
"500"   = "Honored"
"1000"  = "Exalted"

[relationships]
"goblin_tribe" = -0.5
"merchant_guild" = 0.2

[aggro]
threshold = -500
members = ["guard", "townsfolk"]
```

### Rust Struct

```rust
struct FactionDef {
    id: String,
    name: String,
    description: String,
    starting_standing: i32,
    min_standing: i32,
    max_standing: i32,
    ranks: BTreeMap<i32, String>,
    relationships: HashMap<String, f32>,
    aggro: Option<FactionAggro>,
}

struct FactionAggro {
    threshold: i32,
    members: Vec<String>,
}

struct FactionStanding {
    standings: HashMap<String, i32>,
}
```

### Standing Changes

```
new_value = clamp(current + delta, min, max)
propagate to related factions: other += delta * multiplier
```

| Source | Delta |
|---|---|
| Kill aggro member (enemy) | +5 |
| Kill ally member | −10 |
| Quest completion | Per quest |
| Attack faction member | −50 to −200 |

Rank resolved by highest threshold reached. Aggro checked on room entry: if
`standing < threshold` and NPC template is in `aggro.members`, NPC attacks.

### Commands

```
factions                  — list known factions with standing
faction <name>            — show faction details + current rank
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

**Built-in commands (by access level):**

| Command | Access | Description |
|---|---|---|
| `look` / `l` | Player | Examine room or target |
| `n` / `s` / `e` / `w` / `u` / `d` / `ne` / ... | Player | Movement (direction commands) |
| `enter` | Player | List portals in current room |
| `enter <keyword>` | Player | Use a named portal (e.g. "enter sewer grate") |
| `say` | Player | Speak in room |
| `tell` / `whisper` | Player | Private message |
| `reply` / `r` | Player | Reply to last tell |
| `shout` | Player | Broadcast to zone |
| `emote` / `:` | Player | Third-person action |
| `channels` | Player | List available channels |
| `channel <name> <on\|off>` | Player | Toggle channel subscription |
| `kill` | Player | Initiate combat |
| `get` / `drop` | Player | Item manipulation |
| `put <item> in <container>` | Player | Store item in container |
| `give <item> to <player>` | Player | Transfer item to player |
| `inventory` / `i` | Player | List carried items |
| `equipment` / `eq` | Player | List worn items |
| `wear` / `wield` / `remove` | Player | Equipment management |
| `examine` / `exam` | Player | Detailed item/entity info |
| `use` / `cast` | Player | Use any skill (cast is alias for magic-type skills) |
| `train` | Player | Train skills at trainer NPC |
| `craft` | Player | Craft an item from a known recipe |
| `recipes` | Player | List known crafting recipes |
| `repair` | Player | Repair damaged item at blacksmith NPC |
| `stance` | Player | View/change combat stance |
| `group` | Player | Group management (invite, accept, leave, kick, disband, loot, status, chat, formation) |
| `follow` / `follow stop` | Player | Follow/unfollow a player |
| `quests` | Player | Show active quests |
| `quest <id>` | Player | Show quest details + progress |
| `quest abandon <id>` | Player | Abandon a quest |
| `factions` | Player | List known factions with standing |
| `faction <name>` | Player | Show faction details + current rank |
| `sit` / `rest` / `sleep` | Player | Enter resting state |
| `wake` / `stand` | Player | Exit resting state |
| `loot <corpse>` | Player | Loot corpse |
| `time` | Player | Show current game time and date |
| `weather` | Player | Show current weather in zone |
| `motd` | Player | Toggle MOTD display on login |
| `help` / `?` | Player | Online help |
| `who` | Player | List players |
| `config` | Player | Personal settings (blink, color mode, teleportable) |
| `@prestige` | Player | Apply for prestige class |
| `@multi_class` | Player | Add a new base class |
| `@area` | Builder | Area management (create, list, edit, delete, reset, save) |
| `@dig` | Builder | Create room |
| `@link` / `@unlink` | Builder | Connect/disconnect rooms |
| `@set` | Builder | Modify attributes |
| `@desc` | Builder | Room description editor |
| `@room delete` | Builder | Remove room (must be unlinked first) |
| `@mob` | Builder | Mob spawn management (add, remove, edit) |
| `@item` | Builder | Item template management (create, edit, delete) |
| `@load` | Builder | Spawn mobs/items |
| `@area save` | Builder | Persist OLC edits to disk |
| `@area reset` | Builder | Force area reset |
| `goto` | Immortal | Teleport to room |
| `at` | Immortal | Execute cmd in another room |
| `force` | Immortal | Force action on target |
| `stat` | Immortal | Examine entity stats |
| `owhere` / `olocate` | Immortal | Locate entities |
| `gecho` | Immortal | Global echo |
| `gtell` | Immortal | Immortal chat channel |
| `wizwho` | Immortal | List admins online |
| `wizin` | Immortal | Toggle incognito mode |
| `holylight` | Immortal | Toggle see-hidden mode |
| `@teleport` | Immortal | Force-move players |
| `switch` | God | Possess NPC |
| `return` | God | Return from possession |
| `@purge` | God | Remove entities |
| `@slay` | God | Kill target |
| `@restore` | God | Full heal target |
| `@clone` | God | Duplicate entity |
| `ban` / `unban` | God | Ban accounts/IPs |
| `freeze` / `unfreeze` | God | Lock accounts |
| `load` | God | Spawn mobs/items by template |
| `shutdown` | Admin | Graceful server stop |
| `restart` | Admin | Reboot server |
| `wizlock` | Admin | Lock out players below level |
| `config` | Admin | Runtime server configuration |
| `version` | Admin | Server version info |
| `audit` | Admin | View admin action log |

---

## Communication & Socials

### Channels

Channels are named communication streams. Each channel has:

```rust
struct Channel {
    name: String,
    color: Option<&'static str>,
    min_level: u8,
    min_access: AccessLevel,
    history: Vec<String>,
}
```

Built-in channels:

| Channel | Access | Scope | Notes |
|---|---|---|---|
| `say` | Player | Same room | Automatic on `say <text>` |
| `tell` | Player | Specific player | `tell <name> <message>` |
| `reply` / `r` | Player | Same as incoming tell | Replies to last tell |
| `whisper` | Player | Same room target | `whisper <name> <message>` |
| `shout` | Player | Same zone | Higher aggro chance |
| `yell` | Player | Same area | Between shout and say |
| `emote` / `:` | Player | Same room | Third-person action |
| `gossip` | Player | Global | Newbie help channel |
| `auction` | Player | Global | Buy/sell announcements |
| `ooc` | Player | Global | Out-of-character chat |
| `gtell` | Immortal | All immortals | Staff discussion |
| `admin` | Admin | All admins | Admin-only discussion |

### Channel Commands

```
tell <name> <message>    — private message
reply <message>          — reply to last tell
whisper <name> <msg>     — room-private message
emote smiles warmly      — third person action
; smiles warmly          — shortcut (; prefix)
channels                 — list available channels
channel <name> <on|off>  — toggle channel subscription
```

### Socials / Emotions

Predefined socials defined in `content/socials.toml`:

```toml
# content/socials.toml
[socials.smile]
no_target = ["You smile.", "$n smiles."]
with_target = ["You smile at $N.", "$n smiles at you.", "$n smiles at $N."]

[socials.wave]
no_target = ["You wave.", "$n waves."]
with_target = ["You wave at $N.", "$n waves at you.", "$n waves at $N."]
```

Each social has three message forms: self, target, and room:

```rust
struct SocialDef {
    name: String,
    no_target: (String, String),          // (self_msg, room_msg)
    with_target: (String, String, String), // (self, target, room)
}
```

Built-in socials: `smile`, `wave`, `nod`, `glare`, `poke`, `hug`, `frown`,
`grin`, `wince`, `cough`, `sigh`, `laugh`, `bow`, `curtsey`, `shrug`,
`applaud`, `sniff`, `salute`, `shiver`.

### Resting States

```rust
enum RestState {
    Standing,
    Sitting,
    Resting,
    Sleeping,
    Unconscious,
    Dead,
}
```

| State | Regen Bonus | Dodge Penalty | Input |
|---|---|---|---|
| Standing | 0% | 0% | Full |
| Sitting | 0% | -20% | Full (except combat) |
| Resting | +50% | -40% | Chat only |
| Sleeping | +100% | — | Only tell/say wakes you |
| Unconscious | +50% | — | Forced at 0 HP, auto-wake at HP > 0 |

Commands: `sit`, `rest`, `sleep`, `wake`, `stand`.

---

## Time & Weather

### MUD Time System

The game maintains its own calendar and clock, independent of real time.

```rust
struct GameTimeResource {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    is_daytime: bool,
    season: Season,
    raw_seconds: u64,
}
```

**Time ratios:**

| Real time | Game time | Ratio |
|---|---|---|
| 1 minute | 1 hour | 1:60 (default) |
| 24 minutes | 1 day | 1:60 |
| 720 minutes (12h) | 1 month | 1:60 |
| 144 hours (6 days) | 1 year | 1:60 |

### Season & Day/Night

```rust
enum Season { Spring, Summer, Autumn, Winter }
```

Each season affects:
- **Daylight hours:** Summer has longer days, winter longer nights
- **Weather tables:** Different precipitation probabilities per season
- **Temperature:** Modifies zone base temperature
- **Visual:** Room descriptions can reference the season
- **Gameplay:** Some mobs only spawn in certain seasons

### Weather System

Weather is tracked per `weather_zone` (from area definition). The `WeatherSystem`
updates weather each `Weather` phase pulse.

```rust
struct WeatherState {
    zone: String,
    condition: WeatherCondition,
    temperature: i8,
    wind: WindLevel,
    visibility: u8,
    remaining_secs: u32,
}

enum WeatherCondition {
    Clear, Cloudy, Overcast, Fog,
    LightRain, HeavyRain, Storm,
    Snow, Blizzard,
}

enum WindLevel { Calm, Light, Moderate, Strong, Gale }
```

Weather effects on gameplay:

| Condition | Effect |
|---|---|
| Rain/Storm | -2 fire damage, +2 lightning damage |
| Fog | -25% visibility to ranged combat |
| Snow/Blizzard | -1 DEX, tracks show in snow |
| Strong wind | -2 ranged attacks, extinguishes open flames |

### Commands

```
time       — show current game time and date
weather    — show current weather in your zone
```

### SQL

```sql
CREATE TABLE game_time (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    year INTEGER NOT NULL DEFAULT 0,
    month INTEGER NOT NULL DEFAULT 1,
    day INTEGER NOT NULL DEFAULT 1,
    hour INTEGER NOT NULL DEFAULT 8,
    minute INTEGER NOT NULL DEFAULT 0,
    raw_seconds INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

On first startup, game time starts at `Year 0, Month 1, Day 1, 08:00`
(spring morning). On subsequent startups, `raw_seconds` is read from DB
and the server fast-forwards to catch up.

---

## Telnet Protocol

### IAC Structure

Telnet commands use the Interpret-as-Command (IAC) escape byte `0xFF`:

```rust
const IAC: u8  = 0xFF;  // interpret as command
const WILL: u8 = 0xFB;  // willingness to perform option
const WONT: u8 = 0xFC;  // refusal to perform option
const DO: u8   = 0xFD;  // request peer to perform option
const DONT: u8 = 0xFE;  // demand peer to stop performing option
const SB: u8   = 0xFA;  // subnegotiation begin
const SE: u8   = 0xF0;  // subnegotiation end
const NOP: u8  = 0xF1;  // no operation (keepalive)
```

A literal `0xFF` in the data stream is sent as `IAC IAC` (double 0xFF)
by both sides.

### State Machine

The telnet byte parser transitions between states on each incoming byte:

```
Data ───── IAC (0xFF) ───→ IAC
  │                          │
  │                          ├── WILL ──→ Will
  │                          ├── WONT ──→ Wont
  │                          ├── DO   ──→ Do
  │                          ├── DONT ──→ Dont
  │                          ├── SB   ──→ Subneg ── IAC ──→ SubnegIac
  │                          │                           │
  │                          │                    SE (0xF0) → emit subneg
  │                          │                    IAC (0xFF) → literal 0xFF in buffer
  │                          ├── IAC  ──→ emit literal 0xFF
  │                          └── NOP  ──→ keepalive (no-op)
```

```rust
enum TelnetState {
    Data,
    IAC,        // received 0xFF, waiting for command byte
    Will,       // received WILL, waiting for option byte
    Wont,       // received WONT, waiting for option byte
    Do,         // received DO, waiting for option byte
    Dont,       // received DONT, waiting for option byte
    Subneg,     // inside subnegotiation (collecting until IAC SE)
    SubnegIac,  // received IAC inside subneg — next byte is SE or literal 0xFF
}
```

### TelnetConnection

Each connected client gets a `TelnetConnection` wrapping a `TcpStream`:

```rust
struct TelnetConnection {
    stream: TcpStream,
    read_buf: Vec<u8>,
    write_buf: Vec<u8>,
    state: TelnetState,
    subneg_buf: Vec<u8>,           // bytes accumulated during subnegotiation
    options: HashMap<TelnetOption, OptionState>,
    capabilities: HashSet<Feature>,
    last_activity: Instant,
}

struct OptionState {
    local: bool,    // server is performing this option
    remote: bool,   // client is performing this option
}
```

The read path: raw bytes from `TcpStream` → IAC state machine →
parsed text lines + option events. The write path: text + ANSI →
write buffer → flushed after each `send()`.

### Option Handlers

Options are parsed by a dispatch table registered at startup:

```rust
fn handle_option(opt: TelnetOption, action: OptionAction, data: &[u8]) -> OptionResponse {
    match opt {
        TelnetOption::Echo => handle_echo(action),
        TelnetOption::Naws => handle_naws(action, data),
        TelnetOption::TerminalType => handle_termtype(action, data),
        TelnetOption::Mccp => handle_mccp(action),
        TelnetOption::Gmcp => handle_gmcp(action, data),
        TelnetOption::Mxp => handle_mxp(action),
        _ => OptionResponse::Reject,  // unsupported → WONT/DONT
    }
}
```

**Default negotiation sequence at connect:**

```
Server → IAC WILL ECHO          (server offers to handle echo)
Server → IAC DO NAWS            (server requests window size)
Server → IAC DO TERMINAL-TYPE   (server requests terminal info)
Client → IAC DO ECHO            (client accepts)
Client → IAC WILL NAWS          (client sends window size)
Client → SB NAWS <w><h> SE      (client reports w×h)
Client → SB TERMINAL-TYPE SEND SE → Server → SB TERMINAL-TYPE IS "xterm-256color" SE
```

After the initial handshake, the server builds the client's
capability set and negotiates higher protocols only if the terminal
type indicates support (MTTS detection for 256-color, GMCP, MXP).

### Feature Enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Feature {
    Ansi,             // 16-color ANSI codes supported
    ExtendedColor,    // 256-color mode supported
    Naws,             // client reports window size
    Mccp,             // ZLIB compression active
    Gmcp,             // GMCP messages supported
    Mxp,              // MXP tag rendering supported
    Mssp,             // MUD server listing protocol
    Blink,            // blink formatting allowed (user opt-in)
    Html,             // WebSocket bridge, HTML output
    Utf8,             // UTF-8 character encoding
}
```

Features are stored per-connection in `capabilities` and consulted
by the `Connection` trait's `supports()` method.

### Keepalive & Disconnect

A periodic `IAC NOP` (every 60s) checks connectivity:

1. `TelnetConnection.send_raw(&[IAC, NOP])` on each `Pulse` phase
2. If `last_activity` exceeds 60s without any incoming data, send NOP
3. If `last_activity` exceeds 120s (2 missed NOPs), treat as disconnect
4. Emit `PlayerDisconnected` event, clean up entity

`last_activity` is updated on every received byte (including NOP
responses). Disconnect detection runs in a lightweight `KeepaliveSystem`
(DirtyFlush phase).

### Transport-Agnostic Layer

The `Connection` trait abstracts telnet, WebSocket, and future transports:

```rust
trait Connection: Send {
    fn send(&mut self, text: &str);
    fn send_line(&mut self, text: &str);
    fn send_rich(&mut self, text: &RichText);
    fn supports(&self, feature: Feature) -> bool;
    fn id(&self) -> u64;
    fn entity(&self) -> Option<Entity>;
    fn set_entity(&mut self, entity: Entity);
    fn disconnect(&mut self);
    fn last_activity(&self) -> Instant;
}
```

The `TelnetConnection` implements this trait, as does a future
`WsConnection` for WebSocket clients. Command dispatch never
references telnet specifics — it operates on `Box<dyn Connection>`.

### Connection Registry

Before the event bus is fully wired (Phase 3+), room broadcasts
(say, movement enter/leave) use a shared mapping from entity to
output channel:

```rust
type ConnectionRegistry = Arc<Mutex<HashMap<Entity, mpsc::UnboundedSender<Vec<u8>>>>>;
```

- **Insert** on player spawn (`handle_connection` creates entity → register)
- **Remove** on disconnect (cleanup also despawns entity)
- **Broadcast helpers** query `Position` to find room occupants, then
  look up each entity in the registry to forward messages

This mechanism is temporary — once the event bus exists, broadcasts
will flow through `GameEvent::PlayerSaid` / `GameEvent::PlayerMoved`
instead. The registry stays as a lightweight fallback for commands
that need synchronous room enumeration.

---

## Text Formatting & Color

### Color & Format Types

```rust
enum Color {
    // Terminal default (no explicit color)
    Default,
    // 16 standard ANSI colors
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    // Extended (256-color mode)
    Indexed(u8),
}

/// Bitmask of text modifiers (replaces the earlier `Format` bool-struct
/// design — more compact, supports more attributes).
struct Modifier(u8); // BOLD | DIM | ITALIC | UNDERLINE | BLINK | REVERSE | HIDDEN | STRIKE

impl Color {
    /// Nearest 16-color equivalent for basic-ANSI clients.
    fn fallback_16(self) -> Self;
}
```

`Color::Default` plays the role of "no color set" (instead of
`Option<Color>`), so every `Segment` carries a concrete fg/bg.

### RichText Builder

```rust
struct RichText(Vec<Segment>);

struct Segment {
    text: String,
    fg: Color,
    bg: Color,
    modifiers: Modifier,
}

impl Segment {
    fn new(text: impl Into<String>) -> Self;                     // default fg/bg, no modifiers
    fn colored(text: impl Into<String>, fg: Color) -> Self;
    fn styled(text: impl Into<String>, fg: Color, bg: Color, modifiers: Modifier) -> Self;
}

impl RichText {
    fn new() -> Self;
    fn push(&mut self, segment: Segment);
    fn is_empty(&self) -> bool;
    fn segments(&self) -> &[Segment];
    fn plain(self) -> String;       // strip formatting, consume
    fn as_plain(&self) -> String;   // strip formatting, borrow
    /// Render to ANSI string if `ansi=true`, else plain text.
    /// `allow_blink` gates blink output (client or user preference).
    fn render(&self, ansi: bool, allow_blink: bool) -> String;
}
```

### Color Mode

Two-axis model — client capability × user preference:

| Mode | Client supports | User prefers | Behavior |
|---|---|---|---|
| `Off` | — | No color | Strip all ANSI |
| `Basic` | 16-color | Default | 16 standard colors; `Indexed` falls back to nearest 16 |
| `Extended` | 256-color | Opt-in | Full 256-color palette |

Detection: Phase 0 assumes basic ANSI. Later phases detect 256-color via
terminal-type or MTTS (GMCP). User preference persisted per-account.

Fallback mapping (`Indexed(n)` → nearest 16):

```
0–7     → Black, Red, Green, Yellow, Blue, Magenta, Cyan, White
8–15    → Bright variants of 0–7
16–231  → cube colors: weighted nearest to one of the 16
232–255 → grayscale: nearest to Black or White
```

### Blink Gating

Blink is gated by **both** client support and user preference. Default off.
User toggles via `config blink on` / `config blink off`. Value stored in
`accounts.blink_enabled`.

### Connection Trait Additions

```rust
trait Connection: Send {
    fn send(&mut self, text: &str);
    fn send_line(&mut self, text: &str);
    fn send_rich(&mut self, text: &RichText);
    fn send_rich_line(&mut self, text: &RichText);
    fn supports(&self, feature: Feature) -> bool;
    fn id(&self) -> u64;
}
```

Default `send_rich` impl:

```rust
fn send_rich(&mut self, text: &RichText) {
    let ansi = self.supports(Feature::Ansi);
    let blink = self.supports(Feature::Blink);
    self.send(&text.render(ansi, blink));
}
```

### Inline Tag Syntax (Content Files)

Verbose color names, terse reset:

```
{red}this text is red{/}
{brightblue}magic item name{/}
{yellow bold}critical hit!{/}
{green italic}system message{/}
```

Bright colors also accept hyphenated aliases (`{bright-blue}` ==
`{brightblue}`), plus `{grey}`/`{gray}` for `brightblack`. Additional
forms: `{bg:color}` sets background, `{/modifier}` clears a single
modifier, `{{` emits a literal brace.

Parser in `core/src/format/tag.rs`:

```rust
fn parse_tags(input: &str) -> RichText;
```

Applied at content load time; re-rendered per client at display time
(respecting their color mode).

### Color Conventions

| Context | Format | Example |
|---|---|---|
| Room name | `brightwhite` bold | `Town Square` |
| Room description | default | `A cobblestone square...` |
| Exits header | `cyan` | `[Exits: n e s w]` |
| Portals header | `cyan` | `[Portals: sewer grate]` |
| Player name | `yellow` bold | `Alice is here.` |
| Mob name | `red` bold | `A goblin is here.` |
| Item (common) | default | `a rusty sword` |
| Item (magic) | `brightblue` | `a glowing blade` |
| Item (rare) | `brightyellow` | `a runed mithril axe` |
| Item (legendary) | `brightmagenta` | `The Starforge Hammer` |
| Say | default | `Alice says, "Hello!"` |
| Tell / whisper | `magenta` | `Alice tells you, "Hi"` |
| Shout | `brightred` | `Alice shouts, "FIRE!"` |
| Channel name | `green` | `[Gossip]` |
| Combat hit (you) | `brightgreen` | `You hit goblin for 12!` |
| Combat hit (on you) | `brightred` | `goblin hits you for 5!` |
| Critical hit | `yellow` bold | `Critical hit!` |
| Combat miss | default | `You miss the goblin.` |
| Death | `red` bold | `You have been slain!` |
| XP gain | `brightcyan` | `You gain 250 experience.` |
| Level up | `brightyellow` bold | `You have reached level 5!` |
| Prompt (normal) | default | `<100hp 100m>` |
| Prompt (low HP) | `red` | `<15hp 100m>` |
| Prompt (critical) | `red` bold blink | `<5hp 100m>` |
| Error | `brightred` | `You can't do that.` |
| Immortal title | `brightmagenta` | `[Immortal] Alice` |
| Builder output | `yellow` | `Room created.` |
| Help header | `cyan` bold | `Usage:` |

### Content File Embedding

TOML templates use tag syntax for colored text:

```toml
# content/items/flame_sword.toml
name = "{brightred}Flame Sword{/} of the {yellow}Sun King{/}"

# content/mobs/goblin.toml
description = "A {green}green-skinned{/} goblin."
```

### Module Layout

| Path | Contents |
|---|---|
| `core/src/format/mod.rs` | Re-exports |
| `core/src/format/color.rs` | `Color`, `fallback_16()` |
| `core/src/format/rich_text.rs` | `RichText`, `Segment`, `Modifier` |
| `core/src/format/tag.rs` | `parse_tags()` |
| `core/src/format/conventions.rs` | Color convention helpers (room name, player name, exits, ...) |

### Schema Addition

```sql
ALTER TABLE accounts ADD COLUMN blink_enabled INTEGER NOT NULL DEFAULT 0;
```

---

## Persistence

**Two-tier:** In-memory ECS world ↔ SQLite on disk.

- **Load:** Read all entities from SQLite into ECS on startup
- **Dirty tracking:** Mutated entities get a `Dirty` marker component
- **Flush:** Background writer persists dirty entities every 5s (DirtyFlush phase)
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

### Dirty Tracking

Components that mutate during gameplay (e.g. `Health`, `Position`, `Inventory`)
mark the entity `Dirty` on write. The `DirtyFlushSystem` queries all entities
with `&Dirty`, serializes their components to SQLite, then removes `Dirty`.

```rust
fn system(world: &mut World) {
    let mut flush_queue: Vec<(DbId, WriteBatch)> = vec![];
    for (db_id, dirty, ..) in &mut world.query::<(&DbId, &Dirty, ..)>() {
        flush_queue.push((*db_id, collect_components(world, db_id)));
    }
    // batch write to SQLite
    for (db_id, batch) in &flush_queue {
        db.write_components(*db_id, batch);
        world.remove_one::<Dirty>(db_id.entity());
    }
}
```

### Full Flush on Shutdown

On `shutdown_signal`:
1. Steal the `Scheduler` (prevent new pulses)
2. Disconnect all players with "Server shutting down."
3. Flush all dirty entities (same as DirtyFlushSystem logic)
4. Execute `PRAGMA wal_checkpoint(TRUNCATE)`
5. Close database

### Schema Migrations

The `data` crate manages schema versioning via a `schema_version` pragma
and a `migrations` table:

```rust
const SCHEMA_VERSION: u8 = 1;

// The `migrations` table tracks which versions have been applied:
// CREATE TABLE migrations (version INTEGER PRIMARY KEY, applied_at TEXT);
```

Each migration is a function returning the SQL statement(s) for that
version bump:

```rust
fn migrate_0_to_1() -> &'static str {
    "
    CREATE TABLE entities (id INTEGER PRIMARY KEY, type TEXT NOT NULL);
    CREATE TABLE components_position (...);
    CREATE TABLE components_health (...);
    -- ... all initial tables
    "
}
```

**Startup flow:**

1. `PRAGMA user_version` → get current DB version (0 = fresh)
2. If `current < SCHEMA_VERSION`, iterate `current..SCHEMA_VERSION`:
   - Execute migration SQL in a transaction
   - Insert row into `migrations` table
   - `PRAGMA user_version = new_version`
3. Log applied migrations (`info!("Migration 0→1 applied")`)
4. If any migration fails, log `error!` and abort startup

### Entity Serialization

The `collect_components()` function builds a `WriteBatch` — a set of
table-value pairs for a single entity:

```rust
struct WriteBatch {
    entity_id: i64,
    entity_type: String,
    components: Vec<ComponentRow>,
}

enum ComponentRow {
    Position { room_id: i64 },
    Health { current: i32, max: i32 },
    Attributes { str: u8, dex: u8, int: u8, wis: u8, con: u8, cha: u8 },
    Level(u8),
    Experience(u64),
    Wallet(u64),
    LearnedSkills(String),        // JSON: { skill_id → rank, cooldowns }
    Equipment(String),            // JSON: { slot → item_entity_id }
    Inventory(String),            // JSON: [item_entity_id, ...]
    ActiveEffects(String),        // JSON: [{ effect_id, remaining_secs }]
    MultiClassInfo(String),       // JSON: [{ class_id, level, is_favored }]
    SetTracker(String),           // JSON: { active_sets }
    QuestLog(String),             // JSON: { active, completed }
    FactionStanding(String),      // JSON: { faction_id → standing }
    LearnedRecipes(String),       // JSON: [recipe_id, ...]
    Dirty,                        // transient — never flushed
}
```

**Per-type SQL tables** mirror each variant:

```sql
CREATE TABLE components_skills (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    data TEXT NOT NULL                   -- JSON blob
);

CREATE TABLE components_equipment (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    data TEXT NOT NULL                   -- JSON mapping slot → item_entity_id
);

CREATE TABLE components_inventory (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    data TEXT NOT NULL                   -- JSON array of item_entity_ids
);

CREATE TABLE components_wallet (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    copper INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE components_level (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    level INTEGER NOT NULL DEFAULT 1,
    experience INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE components_attrs (
    entity_id INTEGER PRIMARY KEY REFERENCES entities(id),
    str INTEGER NOT NULL DEFAULT 10,
    dex INTEGER NOT NULL DEFAULT 10,
    int INTEGER NOT NULL DEFAULT 10,
    wis INTEGER NOT NULL DEFAULT 10,
    con INTEGER NOT NULL DEFAULT 10,
    cha INTEGER NOT NULL DEFAULT 10
);

CREATE TABLE components_json (
    entity_id INTEGER NOT NULL REFERENCES entities(id),
    key TEXT NOT NULL,                   -- "skills" | "equipment" | "inventory" | "effects" | etc.
    value TEXT NOT NULL,
    PRIMARY KEY (entity_id, key)
);
```

The `components_json` catch-all table stores any component that doesn't
warrant its own table — `LearnedSkills`, `Equipment`, `Inventory`,
`SetTracker`, `QuestLog`, `FactionStanding`, `LearnedRecipes`. Each
entity has one row per key, stored as a JSON string.

### Entity Deserialization (Startup Load)

On startup, the `data` crate rebuilds the ECS world from SQLite:

```
1. SELECT id, type FROM entities          → create ECS entity for each
2. SELECT * FROM components_position      → insert Position component
3. SELECT * FROM components_health        → insert Health component
4. SELECT * FROM components_json WHERE key = "skills"
5. ... repeat for each component table
6. SELECT entity_id FROM components_json  → find entities with no components
   WHERE entity_id NOT IN (all above)     → these are stale — DELETE FROM entities
```

Stale entities (in DB but with no component rows after all tables are
scanned) are deleted. This happens when a template file is removed and
the entity was never cleaned up — the startup sweep garbage-collects it.

### WAL Configuration

```rust
fn open_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch("
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;
        PRAGMA synchronous = NORMAL;
    ")?;
    Ok(conn)
}
```

The connection is wrapped in `Arc<parking_lot::Mutex<Connection>>` and
shared across all systems that need DB access. Contention is low —
flushes happen every 5s and queries are rare (login, character select,
admin commands).

### Type-Safe Query Wrappers

The `data/src/queries.rs` module exposes typed functions that hide SQL:

```rust
// accounts
fn get_account(username: &str) -> Result<Option<Account>>;
fn create_account(username: &str, password_hash: &str) -> Result<i64>;
fn set_access_level(account_id: i64, level: AccessLevel) -> Result<()>;

// characters
fn get_characters(account_id: i64) -> Result<Vec<CharacterRow>>;
fn create_character(account_id: i64, name: &str, race: &str, class: &str) -> Result<i64>;
fn delete_character(character_id: i64) -> Result<()>;

// persistence
fn save_components(batch: &WriteBatch) -> Result<()>;
fn load_all_entities() -> Result<Vec<EntityRow>>;
fn load_component(entity_id: i64, component: &str) -> Result<Option<String>>;

// admin
fn get_admin_log(limit: u32) -> Result<Vec<AdminLogEntry>>;
fn write_admin_log(entry: &AdminLogEntry) -> Result<()>;
```

### Backup Strategy

Hot backup uses SQLite's online backup API, which works safely with WAL:

```rust
fn backup_db(source: &Connection, dest_path: &Path) -> Result<()> {
    let mut dest = Connection::open(dest_path)?;
    let backup = backup::Backup::new(source, &mut dest)?;
    backup.run_to_completion(100, Duration::from_millis(250), None)?;
    Ok(())
}
```

Scheduled by a `BackupSystem` (DirtyFlush phase, once per hour):

```rust
fn run(world: &mut World) {
    let db = world.get_resource::<DbConnection>();
    let backup_path = format!("data/backups/mud_{}.db", chrono::Local::now().format("%Y%m%d_%H%M%S"));
    info!(target: "data", path = %backup_path, "Starting hourly backup");
    backup_db(&db.lock(), Path::new(&backup_path));
}
```

Backups are stored in `data/backups/` and pruned by retention policy
(default: keep 7 daily + 4 weekly). The backup path is configurable
in `mud.toml`:

```toml
[database]
backup_enabled = true
backup_dir = "data/backups"
backup_retention_days = 7
backup_retention_weekly = 4
```

---

## Content Loading & Hot-Reload

### Template Registry

All game content (areas, mobs, items, races, classes, skills, help,
affixes, sets, languages, socials) is defined in TOML files under
`content/` (configurable via `game.content_dir` in `mud.toml`, `--content-path`
CLI flag, or `MUD_CONTENT` env var). At startup, the `content` crate scans the
directory tree
and loads every `.toml` file into a registry:

```rust
struct TemplateRegistry {
    areas: HashMap<String, AreaTemplate>,
    rooms: HashMap<String, RoomTemplate>,
    mobs: HashMap<String, MobTemplate>,
    items: HashMap<String, ItemTemplate>,
    races: HashMap<String, RaceTemplate>,
    classes: HashMap<String, ClassTemplate>,
    skills: HashMap<String, SkillDef>,          // unified: combat, magic, racial, tech, psi, etc.
    help: HashMap<String, HelpEntry>,
    affixes: HashMap<String, AffixDef>,          // from content/affixes.toml
    sets: HashMap<String, SetDef>,               // from content/sets.toml
    languages: HashMap<String, LanguageDef>,      // from content/languages.toml
    socials: HashMap<String, SocialDef>,          // from content/socials.toml
    recipes: HashMap<String, RecipeDef>,          // from content/recipes/*.toml
    quests: HashMap<String, QuestDef>,            // from content/quests/*.toml
    factions: HashMap<String, FactionDef>,        // from content/factions/*.toml
    shops: HashMap<String, ShopTemplate>,         // from content/shops/*.toml
    treasure_classes: HashMap<String, LootTable>, // from content/treasure_classes.toml

    // Derived indices (built after validation)
    class_skill_index: HashMap<String, Vec<String>>,
    race_class_index: HashMap<String, Vec<String>>,
    class_spell_index: HashMap<String, Vec<(String, u8)>>,
    prestige_index: HashMap<String, PrestigeGate>,
    trainer_skill_index: HashMap<String, Vec<String>>,
    set_item_index: HashMap<String, Vec<String>>,
    recipe_station_index: HashMap<String, Vec<String>>,
    quest_giver_index: HashMap<String, Vec<String>>,
    faction_member_index: HashMap<String, Vec<String>>,
}
```

### Loading Pipeline

```
content/*.toml + content/skills/**/*.toml
      │
      ▼
  ┌──────────┐
  │  Scanner  │ ── Walk directory tree, collect .toml files
  │           │    (recursive scan of content/skills/)
  └────┬─────┘
       │
       ▼
  ┌──────────┐
  │  Parser  │ ── serde deserialize into template structs
  │           │    (single SkillDef for all skill types)
  └────┬─────┘
       │
       ▼
  ┌────────────┐
  │  Validator │ ── Cross-reference checks (existing):
│    │ • Room exits point to existing room keys
   │            │    • Room portal dest points to valid area/room
   │            │    • Room portal keywords are unique within a room
   │            │    • Room flag name is valid (portal_in, portal_out, no_teleport_in, no_teleport_out)
   │            │    • Mob/item references exist in registry
  │            │    • Script paths point to real .rhai files
  │            │    • Race.allowed_classes[i] exists in classes
  │            │    • Class.allowed_races[i] exists in races
  │            │    • Class.auto_skills[i].id exists in skills
  │            │    • Skill.allowed_classes[i].class exists in classes
  │            │    • Skill.requires_skill[i].id exists in skills, no circular deps
  │            │    • Item.allowed_classes[i] exists in classes
  │            │    • Item.allowed_races[i] exists in races
  │            │    • Item.allowed_alignments[i] is valid alignment
  │            │    • Item.requires_skill.id exists in skills
  │            │    • Item.set.id exists in sets (if present)
  │            │    • Item.cast trigger skill_id exists and skill_type == Magic
  │            │    • Prestige gate requires_class exists in classes
  │            │    • Prestige gate requires_skills[i].id exists in skills
  │            │    • Race.racial_abilities[i] exists in skills
  │            │    • Race.hometown room key exists in areas
  │            │    • Affix slots reference valid EquipmentSlot names
  │            │    • Set conditions reference valid piece_type values
  │            │    • MobTemplate.equipment[i].template_id exists in items
  │            │    • MobTemplate.loot.entries[i].item exists in items
  │            │    • MobTemplate.faction exists in factions (if set)
  │            │    • MobTemplate.skills[i].id exists in skills
  │            │    • RecipeDef.skill.id exists in skills
  │            │    • RecipeDef.materials[i].item exists in items
  │            │    • RecipeDef.result.item exists in items
  │            │    • RecipeDef.station is valid station type
  │            │    • QuestDef.prerequisites reference valid quest IDs (Quest type)
  │            │    • QuestDef.prerequisites reference valid faction IDs (Faction type)
  │            │    • QuestDef.objectives reference valid mob/item IDs
  │            │    • QuestDef.giver_npc / turn_in_npc exist in mobs
  │            │    • QuestDef.rewards reference valid items/factions/skills
  │            │    • FactionDef.relationships key exists in factions
  │            │    • FactionDef.aggro.members[i] exists in mobs
  │            │    • ShopTemplate.npc exists in mobs
  │            │    • ShopTemplate.inventory[i].item exists in items
   │            │    • Treasure class item entries exist in items
   │            │    • Validation summary logged
  └────┬─────┘
       │
       ▼
  ┌─────────────┐
  │  Indexer    │ ── Build derived indices from validated registry:
  │             │    • class_skill_index:      class_id → [skill_ids in auto_skills]
  │             │    • race_class_index:       race_id → [class_ids from allowed_classes]
  │             │    • class_spell_index:      class_id → [(spell_id, spell_level) from skills where skill_type == Magic]
  │             │    • prestige_index:         class_id → PrestigeGate (only prestiges)
  │             │    • trainer_skill_index:    trainer_tag → [skill_ids with matching trainer_types]
  │             │    • set_item_index:         set_id → [item_template_ids that reference it]
  │             │    • recipe_station_index:   station_type → [recipe_ids using it]
  │             │    • quest_giver_index:      npc_id → [quest_ids they give]
  │             │    • faction_member_index:   mob_id → faction_id (for aggro lookups)
  └────┬─────┘
       │
       ▼
  ┌──────────┐
  │ Registry │ ── Insert into TemplateRegistry (Arc<RwLock<...>>)
  └──────────┘
```

### Hot-Reload Mechanism

The content directory is watched by the `notify` crate (inotify on Linux,
FSEvents on macOS, ReadDirectoryChanges on Windows). On file change:

1. Re-parse the changed file
2. Validate in isolation (parse errors → log warning, keep old template)
3. Cross-validate with existing registry (removed references → log warning)
4. Rebuild affected derived indices
5. Atomic swap: lock `TemplateRegistry` write handle, swap single entry
6. Emit `ContentReloaded { template_type, id }` event for affected systems

**Startup eager-load** blocks the server from accepting connections until
the registry is fully populated. **Runtime hot-reload** is non-blocking —
players experience zero interruption.

### Template File Organization

All paths below are relative to the content root (`game.content_dir`, default `content/`):

```
content/
├── areas/
│   ├── midgaard.toml
│   └── forest.toml
├── mobs/
│   ├── goblin.toml
│   └── deer.toml
├── items/
│   ├── rusty_sword.toml
│   ├── leather_armor.toml
│   ├── templar_helm.toml
│   └── templar_sword.toml
├── races/
│   ├── human.toml
│   ├── elf.toml
│   └── dwarf.toml
├── classes/
│   ├── warrior.toml
│   ├── mage.toml
│   ├── rogue.toml
│   ├── paladin.toml
│   └── assassin.toml          # prestige class
├── skills/
│   ├── combat/
│   │   ├── power_attack.toml
│   │   └── shield_bash.toml
│   ├── magic/
│   │   ├── fireball.toml
│   │   └── bless.toml
│   ├── craft/
│   │   └── smithing.toml
│   ├── racial/
│   │   ├── taunt.toml
│   │   └── stone_form.toml
│   └── general/
│       ├── sneak.toml
│       └── swim.toml
├── scripts/
│   ├── goblin_guard.rhai
│   ├── deer.rhai
│   ├── divine_grace.rhai
│   └── taunt.rhai
├── recipes/
│   ├── iron_sword.toml
│   ├── healing_potion.toml
│   └── leather_armor.toml
├── quests/
│   ├── goblin_problem.toml
│   └── lost_heirloom.toml
├── factions/
│   ├── village.toml
│   ├── goblin_tribe.toml
│   └── merchant_guild.toml
├── shops/
│   └── blacksmith.toml
├── help/
│   ├── look.toml
│   ├── say.toml
│   └── combat.toml
├── affixes.toml
├── sets.toml
├── languages.toml
├── socials.toml
└── treasure_classes.toml
```

---

## Zone & Area System

### Area Template

Areas group rooms into named, managed zones. Each area is defined in a single
TOML file under `content/areas/`:

```toml
# content/areas/midgaard.toml
id = "midgaard"
name = "Midgaard City"
description = "The bustling capital city of the realm."
level_range = { min = 1, max = 20 }
flags = ["city", "peaceful", "no_summon"]
weather_zone = "temperate"
reset_interval_secs = 300

[credits]
author = "Admin"
based_on = "Original Diku Midgaard"
```

### Area Flags

| Flag | Effect |
|---|---|
| `city` | Players can rest/sleep safely |
| `peaceful` | No combat allowed |
| `no_pk` | Player-vs-player disabled |
| `no_magic` | Spell casting blocked |
| `no_summon` | Summon/teleport spells blocked |
| `no_flee` | Combat flee commands blocked |
| `underground` | No day/night cycle, always dark |
| `water` | Underwater — breathing check needed |
| `air` | Flying creatures only |
| `hell` | No recall, death → special respawn |

### Room Flags

Room-level flags control movement permission at a finer grain than area flags.
Set via `flags` on room templates or `@set` at runtime.

**Portal flags are opt-in** — absent means blocked. Teleport flags are opt-out —
absent means allowed. Immortal commands bypass all flags.

| TOML Flag | Constant | Effect | Default |
|---|---|---|---|
| `portal_in` | `ROOM_PORTAL_IN` | Temp portals can target this room | Blocked |
| `portal_out` | `ROOM_PORTAL_OUT` | Temp portals can originate from this room | Blocked |
| `no_teleport_in` | `ROOM_NO_TELEPORT_IN` | Teleport spells cannot land here | Allowed |
| `no_teleport_out` | `ROOM_NO_TELEPORT_OUT` | Teleport spells cannot leave here | Allowed |

Combined example:

```toml
[templates.room.midgaard.temple_sanctum]
area = "midgaard"
name = "Temple Sanctum"
description = "..."
flags = ["portal_in"]  # portals can target this room (party follow), but random teleports are blocked
```

### Room → Area Linking

Room templates reference their area by ID:

```toml
[templates.room.midgaard.square]
area = "midgaard"
name = "Town Square"
description = "A large cobblestone square at the heart of the city..."
exits = [
    { direction = "north", target = "midgaard/temple" },
    { direction = "east", target = "midgaard/market" },
]
portals = [
    { keyword = "sewer grate", dest = "midgaard/sewer01",
      description = "A rusty iron grate set into the cobblestones leads into darkness below." },
    { keyword = "painting", dest = "midgaard/art_gallery",
      description = "An ornate painting of a pastoral landscape.",
      flags = ["hidden"] },
]

[mobs.midgaard.square]
spawns = [
    { mob = "midgaard_guard", count = 2, respawn_secs = 60 },
]
```

### Area Reset

Each area has a reset interval. On reset (triggered by a dedicated phase or
timer), the system:

1. Respawns all dead mobs from `[mobs.<room>]` spawn tables
2. Re-equips any stripped NPCs
3. Resets room flags, doors, and container contents to template defaults
4. Cleans up expired corpses and dropped items older than threshold

Resets are **staggered** — a `last_reset` timestamp per area prevents
re-reset within `reset_interval_secs`.

### Builder Commands

```
@area create <id>              — create new area template
@area list                     — list all areas
@area edit <id>                — modify area properties
@area reset <id>               — force immediate reset
@area delete <id>              — remove area
@mob add <area> <room> <mob> <count>   — add mob spawn
@mob remove <area> <room> <mob>        — remove mob spawn
```

### SQL Tables

```sql
CREATE TABLE areas (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    level_min INTEGER NOT NULL DEFAULT 1,
    level_max INTEGER NOT NULL DEFAULT 99,
    flags TEXT NOT NULL DEFAULT '[]',
    weather_zone TEXT NOT NULL DEFAULT 'temperate',
    reset_interval_secs INTEGER NOT NULL DEFAULT 300,
    last_reset TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE room_spawns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    room_entity_id INTEGER NOT NULL REFERENCES entities(id),
    mob_template_id TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 1,
    respawn_secs INTEGER NOT NULL DEFAULT 60
);
```

### Rust Structs

```rust
struct AreaTemplate {
    id: String,
    name: String,
    description: String,
    level_range: LevelRange,
    flags: Vec<String>,
    weather_zone: String,
    reset_interval_secs: u32,
    credits: HashMap<String, String>,
}

struct LevelRange { min: u8, max: u8 }

struct RoomTemplate {
    id: String,
    area: String,
    name: String,
    description: String,
    exits: Vec<ExitDef>,
    portals: Vec<PortalDef>,
    flags: Vec<String>,
    flags_to: HashMap<String, Vec<String>>,
    heal_rate: Option<u8>,
    mana_rate: Option<u8>,
    teleport_dest: Option<String>,
    extra_descriptions: Vec<ExtraDesc>,
    content: RoomContent,
}

struct ExitDef {
    direction: String,
    target: String,
    flags: Vec<String>,
    key_id: Option<String>,
    door_name: Option<String>,
}

struct PortalDef {
    keyword: String,
    target: String,
    description: String,
    flags: Vec<String>,
}

struct ExtraDesc {
    keyword: String,
    text: String,
}

struct RoomContent {}

struct MobSpawnDef {
    mob: String,
    count: u8,
    respawn_secs: u32,
}
```

---

## Startup & Shutdown Flow

### Startup Phases

The server initializes in a fixed sequence. Each phase logs `info!` on
entry and either advances to the next or aborts with a fatal error —
no partial startup is possible.

```rust
#[derive(Copy, Clone, Debug, Display)]
enum InitPhase {
    CliParse,          // clap --port, --config, --db
    ConfigLoad,        // mud.toml → Config resource
    LoggingInit,       // tracing_subscriber from config
    ContentLoad,       // scan content_dir/ → TemplateRegistry
    Validation,        // cross-ref checks on templates
    DatabaseOpen,      // SQLite connect, WAL mode, migrations
    WorldCreate,       // World::new(), insert resources
    StateSeed,         // load persistent entities from DB
    SystemRegister,    // scheduler init, all systems registered
    ScriptingInit,     // Rhai engine, load scripts, bind events
    EventBusInit,      // subscriber tables built
    CommandTrie,       // register all built-in + content commands
    ListenerBind,      // TcpListener::bind, accept loop
    BackgroundTasks,   // spawn flush timer, hot-reload watcher
    Ready,             // log "ready", enter main loop
}
```

### Wire Diagram

```
bin::main()
  │
  ├── clap::parse()                        → CliArgs
  ├── mud_core::Config::from_file(args)    → Config resource
  │
  ├── tracing_subscriber::fmt().with_env_filter().init()
  │   (filter level from logging.level in Config)
  │
  ├── mud_content::Loader::load(content_dir)  → TemplateRegistry
  │   (scan content_dir/ subdirs, deserialize every .toml)
  ├── mud_content::Validator::validate(&registry) → Vec<Diagnostic>
  │   (42 cross-reference checks; abort on errors, warn on warnings)
  │
  ├── mud_data::Database::open(config.database.path) → Connection
  ├── mud_data::migrate::run(&conn) → schema version
  │   (migration table + PRAGMA user_version, sequential migration fns)
  │
  ├── mud_core::World::new()               → hecs::World
  ├── World.insert(Config)
  ├── World.insert(TemplateRegistry)
  ├── World.insert(EventBus::new())
  ├── World.insert(ScriptEngine::new())
  ├── World.insert(Scheduler::new())
  ├── World.insert(Systems::new())
  │
  ├── mud_data::loader::load_world(&mut world, &conn)
  │   (re-hydrate persistent entity state from DB into ECS)
  │
  ├── mud_core::systems::register_all(&mut world)
  │   (register all 16 built-in systems with scheduler phases)
  │
  ├── mud_scripting::Engine::init(&mut world)
  │   (compile scripts, build AST cache, wire event→script bindings)
  │
  ├── mud_server::cmd::CommandTrie::register_all(&mut world)
  │   (built-in commands + content-derived meta-commands)
  │
  ├── mud_server::Listener::bind(config.server.host, config.server.port)
  │
  ├── tokio::spawn(flush_daemon)           → every 5s flush
  ├── tokio::spawn(hot_reload_watcher)     → notify events
  ├── tokio::spawn(area_reset_timer)       → configurable interval
  │
  └── MainLoop::run(world, listener).await
```

### Main Loop

Expands the existing `tokio::select!` loop with all concurrent channels:

```rust
use tokio::{select, signal};
use std::time::Duration;

struct MainLoop {
    world: Arc<RwLock<World>>,
    scheduler: Scheduler,
    event_bus: EventBus,
    listener: Listener,
    connections: ConnectionMap,
    flush_timer: tokio::time::Interval,
}

impl MainLoop {
    async fn run(mut self) {
        let mut shutdown_signal = signal::unix::signal(
            signal::unix::SignalKind::terminate(),
        ).unwrap();

        loop {
            select! {
                biased;

                _ = shutdown_signal.recv() => {
                    self.shutdown().await;
                    break;
                },

                _ = tokio::signal::ctrl_c() => {
                    self.shutdown().await;
                    break;
                },

                pulse = self.scheduler.next() => {
                    let mut w = self.world.write().await;
                    let phase = pulse.phase;
                    for system in w.get_resource_mut::<Systems>()
                        .unwrap().by_phase(phase)
                    {
                        system.run(&mut w);
                    }
                },

                event = self.event_bus.recv() => {
                    let mut w = self.world.write().await;
                    EventBus::dispatch(&mut w, event);
                },

                Some((conn_id, line)) = self.listener.next_line() => {
                    let mut w = self.world.write().await;
                    let cmd = CommandTrie::resolve(&line);
                    if let Some(cmd) = cmd {
                        let access = self.connections[conn_id].access_level();
                        if access >= cmd.access {
                            (cmd.handler)(&mut w, &mut self.connections[conn_id], &cmd.args);
                        } else {
                            self.connections[conn_id]
                                .send_line("Permission denied.");
                        }
                    }
                },

                _ = self.flush_timer.tick() => {
                    let mut w = self.world.write().await;
                    flush_dirty(&mut w);
                },

                _ = self.hot_reload_watcher.changed() => {
                    let mut w = self.world.write().await;
                    hot_reload(&mut w);
                },
            }
        }
    }

    async fn shutdown(&mut self) {
        // 1. Stop listener — no new connections
        self.listener.close();

        // 2. Notify players
        {
            let w = self.world.read().await;
            for (_, conn) in &self.connections {
                conn.send_line("\n\x1b[31mServer shutting down...\x1b[0m");
            }
        }

        // 3. Drain in-flight commands (200ms grace period)
        tokio::time::sleep(Duration::from_millis(200)).await;

        // 4. Final flush + WAL checkpoint
        {
            let mut w = self.world.write().await;
            flush_all_dirty(&mut w);
            if let Some(db) = w.get_resource::<Database>() {
                db.conn.checkpoint(CheckpointMode::Full).ok();
            }
        }

        // 5. Disconnect all players
        for (_, conn) in &mut self.connections {
            conn.disconnect();
        }

        info!("Server shutdown complete.");
    }
}
```

### bin/src/ Module Layout

```
bin/
├── Cargo.toml
└── src/
    ├── main.rs          # #[tokio::main] → initialize() → MainLoop::run()
    ├── init.rs          # initialize() — runs all init phases, returns MainLoop
    ├── main_loop.rs     # MainLoop struct, run(), shutdown() handler
    ├── signals.rs       # Signal handling (SIGTERM, SIGINT, ctrl-c)
    ├── commands.rs      # register_all_commands() — all built-in commands
    └── config.rs        # CliArgs + mud.toml merge → Config resource
                         #   CliArgs: --port, --host, --config, --content-path
                         #   Env:     MUD_PORT, MUD_HOST, MUD_CONFIG, MUD_CONTENT
                         #   Config:  Config { server, database, game, combat, ... }
```

### Startup Sequence (init.rs)

```rust
async fn initialize() -> Result<MainLoop> {
    let phase = |p| info!("startup: {p}");

    phase(InitPhase::CliParse);
    let args = CliArgs::parse();

    phase(InitPhase::ConfigLoad);
    let config = Config::from_args(&args);

    phase(InitPhase::LoggingInit);
    init_logging(&config);

    phase(InitPhase::ContentLoad);
    let registry = TemplateRegistry::load(&config.game.content_dir)?;

    phase(InitPhase::Validation);
    let diagnostics = registry.validate();
    for d in &diagnostics {
        match d.severity {
            Severity::Error => error!("{d}"),
            Severity::Warning => warn!("{d}"),
        }
    }
    if diagnostics.iter().any(|d| d.severity == Severity::Error) {
        bail!("content validation failed — aborting");
    }

    phase(InitPhase::DatabaseOpen);
    let mut db = Database::open(&config.database.path)?;
    db.run_migrations()?;

    phase(InitPhase::WorldCreate);
    let world = Arc::new(RwLock::new(World::new()));
    {
        let mut w = world.write().await;
        w.insert(config);
        w.insert(registry);
        w.insert(EventBus::new());
        w.insert(ScriptEngine::new());
        w.insert(Scheduler::new());
        w.insert(Systems::new());
        w.insert(db);

        phase(InitPhase::StateSeed);
        load_world_state(&mut w)?;

        phase(InitPhase::SystemRegister);
        systems::register_all(&mut w)?;

        phase(InitPhase::ScriptingInit);
        scripting::Engine::init(&mut w)?;

        phase(InitPhase::CommandTrie);
        cmd::CommandTrie::register_all(&mut w)?;
    }

    phase(InitPhase::ListenerBind);
    let listener = Listener::bind(
        config.server.host,
        config.server.port,
    ).await?;

    phase(InitPhase::BackgroundTasks);
    let flush_timer = tokio::time::interval(Duration::from_secs(5));
    let hot_reload_watcher = start_hot_reload_watcher()?;

    phase(InitPhase::Ready);
    info!("Server ready — listening on {}:{}",
        config.server.host, config.server.port);

    Ok(MainLoop {
        world,
        listener,
        flush_timer,
        hot_reload_watcher,
        // ... other fields
    })
}
```

### Shutdown Sequence

| Step | Action | Timeout |
|---|---|---|
| 1 | Close listener (stop accepting) | Immediate |
| 2 | Notify players: `Server shutting down...` | Immediate |
| 3 | Drain in-flight commands | 200ms |
| 4 | Flush all dirty entities to SQLite | ∞ |
| 5 | WAL checkpoint (FULL) | 5s |
| 6 | Disconnect all players | Immediate |
| 7 | Exit process | — |

The shutdown is triggered by:
- **SIGTERM** — standard daemon stop
- **SIGINT (Ctrl+C)** — interactive stop
- **`shutdown` command** — admin command in-game
- **Fatal error** — panic or unrecoverable state (logs first, then exits)

### bin/Cargo.toml Dependencies

```toml
[dependencies]
mud-core = { path = "../core" }
mud-server = { path = "../server" }
mud-data = { path = "../data" }
mud-scripting = { path = "../scripting" }
mud-content = { path = "../content" }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "2"
```

---

## Configuration

### Config File

The server reads `mud.toml` from the current working directory (override with
`--config` flag or `MUD_CONFIG` env var):

```toml
[server]
host = "127.0.0.1"
port = 4000
max_players = 256

[database]
path = "data/mud.db"

[game]
name = "Mud"
motd = "Welcome to Mud!"
new_player_start_room = "limbo/starting_room"
max_level = 100
start_race = "human"
start_class = "warrior"
content_dir = "content"               # path to game data (areas/, mobs/, items/, etc.)

[combat]
base_attack_cooldown_secs = 2
xp_multiplier = 1.0
mob_respawn_secs = 30

[training]
base_cost = 100                          # gp at level 1, skill rank 0
cost_per_level = 50                      # additional gp per character level
cost_per_rank = 25                       # additional gp per skill rank
cost_multiplier = 1.0                    # global multiplier (2.0 for prestige skills)

[multi_classing]
xp_penalty_pct_per_class = 20            # penalty per non-favored class beyond first
xp_penalty_max = 80                      # cap
favored_class_empty_slot = true          # empty favored slot waives penalty

[item_sets]
enable_set_bonuses = true                # global toggle for set bonus system

[logging]
format = "compact"       # compact | full | json
level = "info"           # trace | debug | info | warn | error
```

### Precedence (highest to lowest)

1. **CLI flags** (`--port 5000`) — parsed with `clap`
2. **Environment variables** (`MUD_PORT=5000`) — `MUD_` prefix + uppercase key
3. **Config file** (`mud.toml`) — on-disk TOML
4. **Built-in defaults** — hardcoded in source

Key │ Flag │ Env │ Config file │ Default
---|---|---|---|---
Content path │ `--content-path` │ `MUD_CONTENT` │ `game.content_dir` │ `"content"`

### Runtime Configuration

The `[game]` section is writable at runtime via the `config` command:

```
> config motd "New message of the day!"
> config show
```

Runtime overrides are persisted to a `config` SQLite table and restored
on next startup. Server section changes (`host`, `port`) require a restart.

### Config Resource

At startup, the parsed configuration is stored as a `Config` resource in
the ECS world, available to all systems:

```rust
struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub game: GameConfig,
    pub combat: CombatConfig,
    pub training: TrainingConfig,
    pub multi_classing: MultiClassingConfig,
    pub item_sets: ItemSetConfig,
    pub logging: LoggingConfig,
}

impl Resource for Config {}
```

Systems access config via `world.get_resource::<Config>()`.

---

## Error Handling & Logging

### Error Types

Each crate defines its own error enum with `Display + std::error::Error`
(using `thiserror` for convenience) and a crate-level `Result` alias:

```rust
// mud_core::error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("entity {0} not found")]
    EntityNotFound(Entity),
    #[error("component {0} missing on entity")]
    ComponentMissing(&'static str),
    #[error("invalid direction: {0}")]
    InvalidDirection(String),
}

// mud_server::error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("telnet error: {0}")]
    Telnet(String),
    #[error("command not found: {0}")]
    CommandNotFound(String),
    #[error("insufficient access: {0} < {1}")]
    InsufficientAccess(AccessLevel, AccessLevel),
}

// mud_data::error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("entity {0} not found in database")]
    EntityNotFound(i64),
    #[error("migration failed: {0}")]
    MigrationFailed(String),
}

// mud_scripting::error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("rhai error: {0}")]
    Rhai(#[from] rhai::EvalAltResult),
    #[error("binding not found: {0}")]
    BindingNotFound(String),
    #[error("script not found: {0}")]
    ScriptNotFound(String),
}
```

Errors are **composed**, not boxed. Use `From` impls to convert between
crate error types where appropriate (e.g. `data::Error::from(rusqlite::Error)`).

### Logging Conventions

Logging uses `tracing` for structured, async-aware diagnostics:

| Level | When | Example |
|---|---|---|
| `error!` | Unrecoverable faults, DB failures | `error!(target: "data", err = %e, "Query failed")` |
| `warn!` | Admin actions, unexpected states | `warn!(target: "audit", executor = %id, cmd = "purge", target = %t, "Destructive action")` |
| `info!` | Normal lifecycle events | `info!(target: "server", addr = %addr, "Client connected")` |
| `debug!` | Development diagnostics | `debug!(target: "combat", attacker = %a, target = %t, roll = hit, "Attack resolved")` |
| `trace!` | Per-pulse verbosity | `trace!(target: "movement", entity = %e, from = %f, to = %t, "Player moved")` |

### Audit Logging

All destructive admin actions are logged with `tracing::warn!`:

```rust
warn!(
    target: "audit",
    action = "purge",
    executor = %conn.entity().unwrap(),
    target = %args,
    // timestamp added automatically by tracing-subscriber
    "Admin action"
);
```

An optional `admin_log` SQLite table (Phase 2+) persists these for review
via the `audit` command:

```sql
CREATE TABLE admin_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action TEXT NOT NULL,
    executor_entity_id INTEGER,
    executor_account_id INTEGER,
    target TEXT,
    details TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

---

## Help System

### Help Files

Help topics are defined as TOML entries in `content/help/`:

```toml
# content/help/look.toml
id = "look"
aliases = ["l"]
title = "Look"
text = """
Usage:  look [target]

Examine your surroundings or a specific object, mob, or character.

Without arguments, look shows the description of the room you are in,
including visible exits, mobs, items, and other characters.

Examples:
  look               — examine the current room
  look orc           — examine an orc in the room
  look rusty_sword   — examine an item on the floor
"""
```

### Index

At startup, the `content` crate loads all `content/help/*.toml` files
into the `TemplateRegistry` as `HashMap<String, HelpEntry>`. The index
maps both `id` and all `aliases` to the same entry (bidirectional).

### Command Behavior

```
help               → show topic index (all topics with one-line summary)
help <topic>       → show full help text for that topic
help <partial>     → if no exact match, show topics containing the keyword
```

The index is also used by the command parser for a hint on unrecognized input:
`Huh? Type 'help' for a list of commands.`

### Builder-created Help

Admin-level commands can manage help topics in-game:

```
> @help create <topic>
> @help edit <topic>
> @help delete <topic>
```

Builder-created help is stored in a `help` SQLite table and merged with
file-based topics at startup (DB entries override file entries on key
collision).

### SQL Table

```sql
CREATE TABLE help (
    id TEXT PRIMARY KEY,
    aliases TEXT NOT NULL DEFAULT '[]',  -- JSON array of strings
    title TEXT NOT NULL,
    text TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Rust Struct

```rust
struct HelpEntry {
    id: String,
    aliases: Vec<String>,
    title: String,
    text: String,
}
```

---

## Admin & Immortal System

### Access Levels

Five tiers, stored at the account level, consulted per-command:

```
Player < Builder < Immortal < God < Admin
```

Account-level permission means any character on an admin account inherits that tier. Permission is cached on the `Connection` at login.

### Immortal Component

```rust
struct Immortal {
    incognito: bool,   // wizin — hidden from who/room
    holylight: bool,   // see hidden exits/rooms
    build_mode: bool,  // walk unlinked exits, technical info
}
```

Added to a character entity at spawn if the account's `access_level > Player`. Flags are per-session (default false on reconnect).

### Connection Changes

```rust
trait Connection: Send {
    fn send(&mut self, text: &str);
    fn send_line(&mut self, text: &str);
    fn id(&self) -> u64;
    fn entity(&self) -> Option<Entity>;
    fn set_entity(&mut self, entity: Entity);
    fn disconnect(&mut self);
    fn access_level(&self) -> AccessLevel;
    fn set_access_level(&mut self, level: AccessLevel);
    fn has_immortal(&self) -> bool;
    fn immortal_flag(&self, flag: ImmortalFlag) -> bool;
}
```

### Permission Checking

Command dispatch gates on `conn.access_level() < cmd.access`. If insufficient, send "You lack the power for that."

### Incognito Mode (wizin)

- `who` list: skip incognito characters entirely (God+ with holylight see them via `wizwho`)
- `look`: incognito characters shown as "You sense a presence here." to mortals. Non-incognito immortals show with tier title (e.g. `[Immortal] Name`, `[God] Name`). God+ with holylight see through incognito.
- Incognito immortals remain fully interactive (can speak, fight, move).

### Accounts Table

```sql
CREATE TABLE accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    access_level TEXT NOT NULL DEFAULT 'player',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_login TEXT
);
```

Schema version bumped to 2. No data migration needed (no real accounts exist yet).

### Admin Command Inventory

| Tier | Scope | Commands |
|------|-------|----------|
| **Player** | Basic gameplay | look, say, tell, whisper, shout, kill, get, drop, inventory, equipment, wear, wield, remove, use, cast, help, who, quit, save, emote |
| **Builder** | World-building | @dig, @link, @unlink, @set, @load, @room, @exits, @reset, @copy, @move, @whereami |
| **Immortal** | See/move freely | goto, at, force, stat, owhere, wizin, holylight, gecho, gtell, wizwho, return, follow, glance, @teleport |
| **God** | Create/destroy | switch, load, purge, slay, restore, set (on targets), ban, unban, freeze, unfreeze, clone, noquit, deny, @create |
| **Admin** | Server ops | shutdown, restart, wizlock, motd, config, version, @save, memory, audit |

### Safety Invariants

- Combat system skips damage application if target has `Immortal` component
- `switch` refuses player targets (NPC-only possession)
- `force` blocked if target's access level >= executor's
- `purge` refuses to remove entities with `Immortal` component
- All destructive actions logged via `tracing::warn!()` with entity ID, target, and args

### Implementation Order

1. Expand `AccessLevel` enum to 5 tiers
2. Add `Immortal` component to `core`
3. Add `accounts` table to schema, bump VERSION to 2
4. Add `access_level` to `Connection` trait + `TelnetConnection`
5. Gate command dispatch on access level
6. Implement `wizin` command + who/room filtering
7. Remaining admin commands added incrementally per phase

---

## Login & Character System

### Connection State Machine

Between telnet negotiation and gameplay input, each connection traverses a state
machine. Input routes to `handle_login()` instead of the command trie while
`conn.state() != Playing`.

```rust
enum ConnectionState {
    Connected,
    Negotiating,
    Banner,
    Username,
    Password,
    CharacterSelect,
    CharacterCreateName,
    CharacterCreateRace,
    CharacterCreateClass,
    CharacterCreateAttributes,
    CharacterCreateConfirm,
    Playing,
}
```

**Transitions:**

```
Connected → Negotiating → Banner → Username → Password → CharacterSelect → Playing
                                                                    ↘ CharacterCreate* ↗
```

| State | Action |
|---|---|
| `Connected` | TCP accepted, begin telnet negotiation |
| `Negotiating` | Telnet option negotiation (echo, NAWS, etc.) |
| `Banner` | Send `<content_dir>/banner.txt` + MOTD from DB config |
| `Username` | Prompt "Enter your username:" |
| `Password` | Prompt "Password:" (no echo), argon2 verify |
| `CharacterSelect` | Show account's characters + "Create new" |
| `CharacterCreate*` | Multi-step creation wizard (name → race → class → attrs → confirm) |
| `Playing` | Normal command dispatch via trie |

Failed input at any pre-Playing step counts toward a per-connection strike limit
(3 strikes → disconnect with a message).

### Login Flow

1. **Connected** — telnet negotiation completes, then:
   - Read `<content_dir>/banner.txt`, send to client
   - Query MOTD from SQLite `config` table, send to client
   - Send "Enter your username:"
2. **Username lookup** — case-insensitive match in `accounts` table
   - **Found** → transition to `Password`, prompt for password
   - **Not found** → offer "That name isn't registered. Create a new account? (y/n)"
     - Yes → prompt password → confirm password → hash (argon2) → insert `accounts` row → transition to `CharacterSelect`
     - No → return to `Username`
3. **Password verification** — argon2 verify against `accounts.password_hash`
   - Match → cache `access_level` on connection → transition to `CharacterSelect`
   - No match → "Invalid password." → retry (up to 3 total)
4. **Character select** — query `characters WHERE account_id = ?`
   - Show numbered list: name, race, class, level
   - Append "N. Create a new character"
   - Pick existing → spawn entity into ECS world → transition to `Playing`
   - Pick "Create new" → transition to `CharacterCreateName`

### Character Creation Wizard

One-shot wizard that runs before the player enters the game:

| Step | Prompt | Validation | Filtering |
|---|---|---|---|
| **Name** | "Enter your character's name:" | 3–16 alphanumeric + `_-`, capitalized, unique in `characters.name` | — |
| **Race** | Show list from `content/races/*.toml`, pick by number | Valid race key | — |
| **Class** | Show list from `content/classes/*.toml`, filtered by race | Valid class key | Only show classes where `race` ∈ `class.allowed_races` AND `class` ∈ `race.allowed_classes` (intersection) |
| **Attributes** | Display base (race) + mods (class) = starting stats; offer roll or point-buy | Within caps | — |
| **Confirm** | Show full summary (name, race, class, attrs) — "Accept? (y/n)" | y → save; n → restart from name | — |

**Race→Class filtering logic:**

```
picked_race = races["elf"]
available_classes = classes.values().filter(|c|
    c.allowed_races.contains(picked_race.id) &&
    picked_race.allowed_classes.contains(c.id)
)
```

If no classes pass the filter, inform the player and step back to race
selection. This ensures race+class combinations are always valid.

**Class→skill filtering at creation:**

After class is chosen, the creation wizard shows a list of starting skills
that the character auto-receives (from `class.auto_skills` level 1 + race's
`racial_abilities`). The character sheet on the confirm screen includes
starting skills so the player knows what they get.

**Attribute calculation:**
```
final = race_base + class_mod + bonus_points
```

The player chooses one of three methods:

**Method 1 — Point-Buy (recommended):**

Each stat starts at 8. The player has 27 points to spend (distinct from
the "5 bonus points" from race/class). Costs are progressive:

| Score | Cost | Score | Cost |
|---|---|---|---|
| 8 | 0 | 14 | 7 |
| 9 | 1 | 15 | 9 |
| 10 | 2 | 16 | 12 |
| 11 | 3 | 17 | 15 |
| 12 | 4 | 18 | 19 |
| 13 | 5 | (max) | |

**Method 2 — Standard Array:**

Pre-generated set the player assigns freely to the six stats:
```
15, 14, 13, 12, 10, 8
```

**Method 3 — Roll:**

Roll 4d6, drop the lowest die, repeat 6 times. The player then
assigns the resulting six values to stats freely. Re-rolling is
allowed (whole set only, up to 3 attempts).

```
Example roll:
  4d6 [6, 4, 3, 1] → drop 1 → sum = 13
  4d6 [5, 5, 5, 2] → drop 2 → sum = 15
  4d6 [4, 4, 3, 3] → drop 3 → sum = 11
  4d6 [6, 6, 1, 1] → drop 1 → sum = 13
  4d6 [5, 4, 4, 4] → drop 4 → sum = 13
  4d6 [3, 3, 2, 2] → drop 2 → sum = 8
  Assigned: STR 15, DEX 13, CON 13, INT 11, WIS 13, CHA 8
```

After the chosen method, race base modifiers and class modifiers are
applied. Final values are clamped to [3, 25]. The confirm screen shows
both the base scores and the final computed values.

**On confirm:** insert `characters` row → create ECS entity with `Position`, `Player`, `Attributes`, `Health`, `Level`, `Experience`, `LearnedSkills` (auto-granted from race `racial_abilities` + class `auto_skills` for level 1) components → spawn in starting room → `state = Playing`.

### TOML Templates

Races and classes live on disk at `content/races/*.toml` and `content/classes/*.toml`.
See the [Races](#races) and [Classes](#classes) sections for full template
schemas with constraint fields.

### SQL Schema

New `characters` table:

```sql
CREATE TABLE characters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES accounts(id),
    name TEXT NOT NULL UNIQUE,
    race TEXT NOT NULL,
    class TEXT NOT NULL,
    level INTEGER NOT NULL DEFAULT 1,
    experience INTEGER NOT NULL DEFAULT 0,
    room_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen TEXT
);
```

Add `show_motd` flag to accounts (default 1; `motd` command toggles it per-account):

```sql
ALTER TABLE accounts ADD COLUMN show_motd INTEGER NOT NULL DEFAULT 1;
```

### Connection Trait Additions

```rust
trait Connection: Send {
    // ...existing methods...
    fn state(&self) -> &ConnectionState;
    fn set_state(&mut self, state: ConnectionState);
    fn create_buffer(&mut self) -> &mut CharacterCreateBuffer;
}

struct CharacterCreateBuffer {
    name: Option<String>,
    race: Option<String>,
    class: Option<String>,
    attributes: Option<Attributes>,
}
```

### Implementation Order

1. Add `ConnectionState` enum + `state()`/`set_state()` to `Connection` trait
2. Wire `handle_login()` gate between I/O read and command dispatch
3. Implement `Banner` / `Username` / `Password` states
4. Implement account creation flow (new username path)
5. Add `characters` table to schema, bump VERSION
6. Implement `CharacterSelect` state
7. Implement character creation wizard (all substates)
8. Add race/class TOML loading in `content/` crate
9. Add default `content/banner.txt` + example race/class templates
10. Connect final "Confirm → spawn" step to ECS + persistence

---

## Scripting & OLC

### Design Principle

Rhai scripts drive all dynamic behavior not captured by TOML templates.
Scripts handle NPC logic, item procs, quest triggers, stance/passive
effects, OLC automation, and custom skill behavior. The engine exposes a
rich Rust API; nothing is baked into Rhai that can't be changed per-file.

### Rhai Engine Setup

A sandboxed `rhai::Engine` is created per-script-execution to limit damage
from buggy or abusive scripts:

```rust
struct ScriptEngine {
    engine: rhai::Engine,
    module_resolver: FileModuleResolver,
}

impl ScriptEngine {
    fn new(script_dir: &Path) -> Self {
        let mut engine = rhai::Engine::new();

        // Sandbox — no filesystem, no network, no process spawn
        engine.set_max_modules(8);
        engine.set_max_call_levels(32);
        engine.set_max_operations(50_000);
        engine.set_max_string_size(10_000);
        engine.set_max_dynamic_arrays(100);
        engine.set_max_map_size(50);

        // Register all Rust types + methods (see below)
        Self::register_types(&mut engine);

        let resolver = FileModuleResolver::new(script_dir);
        Self { engine, module_resolver: resolver }
    }
}
```

### Script Lifecycle

Each content template can reference Rhai scripts via `[scripts]`:

```toml
# content/mobs/goblin.toml
[[scripts]]
event = "death"
script = "goblin_guard.rhai"

[[scripts]]
event = "enter"
script = "goblin_alert.rhai"
```

```
[Startup]
  ┌────────────────────┐
  │  Scan scripts/     │ ── Walk scripts/ dir, index all .rhai paths
  └────────┬───────────┘
           ▼
  ┌────────────────────┐
  │  Parse & AST-cache │ ── Parse each .rhai into AST, cache by path
  │  (per file)        │    (no execution yet)
  └────────┬───────────┘
           ▼
  ┌────────────────────┐
  │  Register event    │ ── For each template's [[scripts]] entries:
  │  bindings          │    link event → AST so ScriptTriggerSystem
  │                    │    can dispatch without re-reading disk
  └────────┬───────────┘
           ▼
[Runtime]
  ┌────────────────────┐
  │  Event fires       │ ── ScriptTriggerSystem looks up event + entity
  │  (e.g. MobDied)    │
  └────────┬───────────┘
           ▼
  ┌────────────────────┐
  │  Create engine+ctx │ ── Fresh rhai::Engine per call (sandbox reset)
  │  Build ScriptCtx   │    entity, actor, world_ref → ScriptCtx
  └────────┬───────────┘
           ▼
  ┌────────────────────┐
  │  Execute AST       │ ── engine.run_ast(ast) with ctx scope
  │  (max 50k ops)     │    If timeout/abort → log warn, continue
  └────────┬───────────┘
           ▼
  ┌────────────────────┐
  │  Collect effects   │ ── world_ref mutations applied after return
  │  (deferred)        │    (ScriptTriggerSystem holds World lock)
  └────────────────────┘
```

### Script Context

```rust
struct ScriptCtx {
    entity: EntityHandle,       // the entity this script is attached to
    actor: Option<EntityHandle>, // triggering entity (attacker, user, etc.)
    target: Option<EntityHandle>, // secondary target (if applicable)
    world: WorldGuard,           // &mut World in scope for the call
}
```

`EntityHandle` is a thin wrapper around `hecs::Entity` that exposes methods
to Rhai. It is NOT serializable and is invalidated after the script returns.

### Rhai Type Bindings — EntityHandle

```rust
// the entity the script is attached to
ctx.entity.name()           → String
ctx.entity.id()             → i64 (DbId, 0 if transient)
ctx.entity.room()           → RoomHandle
ctx.entity.has_flag(f)      → Bool
ctx.entity.has_comp(name)   → Bool         // component exists?
ctx.entity.health()         → Int (current HP)
ctx.entity.max_health()     → Int
ctx.entity.level()          → Int
ctx.entity.race()           → String
ctx.entity.classes()        → Array        // list of class strings
ctx.entity.is_player()      → Bool
ctx.entity.is_npc()         → Bool
ctx.entity.say(msg)         → void         // speak in room
ctx.entity.emote(msg)       → void         // perform emote
ctx.entity.echo(msg)        → void         // send to player only
ctx.entity.get_attr(name)   → Dynamic      // get KV attribute (OLC)
ctx.entity.set_attr(k, v)   → void         // set KV attribute
```

### Rhai Type Bindings — RoomHandle

```rust
ctx.entity.room().name()          → String
ctx.entity.room().echo(msg)       → void   // broadcast to room
ctx.entity.room().entities()      → Array  // list of EntityHandle
ctx.entity.room().players()       → Array  // player entities only
ctx.entity.room().exits()         → Array  // exit direction strings
ctx.entity.room().has_mob(id)     → Bool   // mob template present?
ctx.entity.room().has_item(id)    → Bool   // item template present?
```

### Rhai Type Bindings — WorldHandle

```rust
ctx.world.spawn_mob(template_id, room)      → EntityHandle
ctx.world.spawn_item(template_id, room, count) → void
ctx.world.remove_entity(entity)             → void
ctx.world.echo_room(room_id, msg)           → void
ctx.world.echo_zone(zone_id, msg)           → void
ctx.world.echo_world(msg)                   → void
ctx.world.grant_xp(entity, amount)          → void
ctx.world.grant_recipe(entity, recipe_id)   → void
ctx.world.grant_quest(entity, quest_id)     → void
ctx.world.advance_quest(entity, quest_id, objective_index, amount) → void
ctx.world.set_faction(entity, faction_id, standing) → void
ctx.world.mod_faction(entity, faction_id, delta)    → void
ctx.world.has_entity(entity_id)             → Bool
```

### Event Registration

Scripts use the `on()` function to register handlers. The runtime
maps event names to `EventTag` values:

```rust
// Built-in events a script can subscribe to:
on("death",      |ctx| { ... })   // entity died
on("enter",      |ctx| { ... })   // someone entered entity's room
on("leave",      |ctx| { ... })   // someone left entity's room
on("hit",        |ctx| { ... })   // entity was hit in combat
on("kill",       |ctx| { ... })   // entity killed someone
on("say",        |ctx| { ... })   // someone said something in room
on("use",        |ctx| { ... })   // entity used a skill
on("damage",     |ctx| { ... })   // entity took damage
on("tick",       |ctx| { ... })   // every regen pulse (combat only)
on("spawn",      |ctx| { ... })   // entity was just spawned
on("reset",      |ctx| { ... })   // area reset
on("quest_done", |ctx| { ... })   // quest completed by entity
on("custom",     |ctx| { ... })   // dispatched by other scripts
```

The `[[scripts]]` TOML entries are loaded once at startup and cached.
At runtime, the `ScriptTriggerSystem` uses a `HashMap<(Entity, EventTag), Ast>` lookup.

### Thread Safety

Scripts run under the `World` write lock. The `ScriptTriggerSystem`:

1. Locks `World`
2. Looks up the entity's script bindings for the firing event
3. Creates a fresh `ScriptCtx` with `WorldGuard` (a reborrow wrapper)
4. Runs the Rhai AST inline (no tokio spawn — scripts are fast by design)
5. If the script exceeds resource limits (max_operations), the engine
   aborts with a `RuntimeError`. The system catches it, logs a warning,
   and continues.

Scripts must NOT spawn tokio tasks or hold cross-entity references across
script calls — all state is ephemeral within the `run_ast()` call.

### Script Module System

Scripts can import shared logic via Rhai modules:

```toml
# content/scripts/lib/combat_helpers.rhai
export function calculate_damage(level, base) {
    return base + level * 2;
}
```

```rust
// content/scripts/goblin_guard.rhai
import "lib/combat_helpers" as combat;

on("hit", |ctx| {
    let extra = combat::calculate_damage(ctx.entity.level(), 5);
    ctx.entity.echo(`Extra damage: ${extra}`);
});
```

Module resolution path: `<content_dir>/scripts/` → relative path from `import`.
Modules are AST-cached at startup (hot-reload watch). Cyclic imports
are detected at parse time by Rhai's module resolver.

### Script Hot-Reload

The `notify` watcher (same as TOML hot-reload) monitors `<content_dir>/scripts/`:

1. File change detected → re-parse into AST
2. Update the `ScriptTriggerSystem`'s binding map for all templates
   referencing that script file
3. Log: `info!("Hot-reloaded script: goblin_guard.rhai")`
4. Parse failure → keep old AST, log error

### Rhai Registerable Functions

Helper functions available to all scripts without import:

```rust
// In-Game utility
rng(min, max)             → Int    // uniform random
clamp(val, min, max)      → Int    // numeric clamp
roll(dice, sides)         → Int    // 3d6 style roll
capitalize(s)             → String

// Lookup (reads from TemplateRegistry)
template(id, type)        → Map    // get template by ID + type
item_template(id)         → Map    // shortcut for item
mob_template(id)          → Map    // shortcut for mob
skill_template(id)        → Map    // shortcut for skill

// Messaging (access via ctx.world)
echo_room(room_id, msg)   → void
echo_entity(entity, msg)  → void
```

### Security Model

| Concern | Protection |
|---|---|
| Infinite loops | `max_operations` = 50k, `max_call_levels` = 32 |
| Memory exhaustion | `max_string_size` = 10k, `max_dynamic_arrays` = 100, `max_map_size` = 50 |
| File access | Sandboxed — resolver only reads `<content_dir>/scripts/` |
| Network access | No socket bindings registered |
| World corruption | Script runs under `World` lock; mutations are synchronous |
| CPU starvation | Script runs inline but is limited to ~1ms at 50k ops |
| Recursive scripts | Event dispatch is synchronous; re-entrant events are queued |

### OLC Commands

OLC commands let builders and immortals edit the world at runtime.
Each command has a minimum access level and a set of parameters:

#### Area OLC

| Command | Access | Parameters | Description |
|---|---|---|---|
| `@area create` | Builder | `<id> [name]` | Create new area template |
| `@area list` | Builder | (none) | List all areas |
| `@area edit` | Builder | `<id>` | Enter area edit mode (interactive) |
| `@area delete` | Builder | `<id>` | Remove area (requires confirmation) |
| `@area reset` | Builder | `<id>` | Force immediate area reset |
| `@area save` | Builder | (none) | Persist current area edits to disk |

#### Room OLC

| Command | Access | Parameters | Description |
|---|---|---|---|
| `@dig` | Builder | `<room_key> [area_id]` | Create new room in area (or area inferred from key) |
| `@link` | Builder | `<room_a> <dir> <room_b>` | Link two rooms with exit in both directions |
| `@unlink` | Builder | `<room> <dir>` | Remove exit from room |
| `@set` | Builder | `<room>.<field> = <value>` | Set room field (name, desc, flags, heal_rate, mana_rate) |
| `@desc` | Builder | `<room>` | Enter room description editor (multi-line) |
| `@room delete` | Builder | `<room>` | Remove room (must be unlinked first) |
| `@portal` | Builder | `<room>` | List portals in room |
| `@portal add` | Builder | `<room> <keyword> <dest> <description>` | Add portal exit to room |
| `@portal remove` | Builder | `<room> <keyword>` | Remove portal from room |
| `@portal hide` | Builder | `<room> <keyword>` | Toggle hidden flag on portal |

#### Mob OLC

| Command | Access | Parameters | Description |
|---|---|---|---|
| `@mob add` | Builder | `<area> <room> <mob_id> <count>` | Add mob spawn to room |
| `@mob remove` | Builder | `<area> <room> <mob_id>` | Remove mob spawn |
| `@mob edit` | Builder | `<mob_id>.<field> = <value>` | Edit mob template property |
| `@load` | Builder | `<room> <mob_id> [count]` | Spawn mobs immediately (runtime entity) |

#### Item OLC

| Command | Access | Parameters | Description |
|---|---|---|---|
| `@item create` | Builder | `<id> [name]` | Create new item template |
| `@item edit` | Builder | `<item_id>.<field> = <value>` | Edit item template property |
| `@item delete` | Builder | `<item_id>` | Remove item template |
| `@load` | Builder | `<room> <item_id> [count]` | Spawn items immediately (runtime entity) |

#### Admin OLC

| Command | Access | Parameters | Description |
|---|---|---|---|
| `@purge` | God | `<room/all>` | Remove all non-player entities from room (or entire world) |
| `@clone` | God | `<entity>` | Duplicate an entity |
| `@restore` | God | `<entity>` | Full heal + restore resources |
| `@slay` | God | `<entity>` | Kill target instantly |
| `@goto` | Immortal | `<room>` | Teleport self to room |
| `@at` | Immortal | `<room> <command>` | Execute command from another room |
| `@force` | Immortal | `<entity> <command>` | Force entity to execute a command |
| `@stat` | Immortal | `<entity>` | Show detailed entity debug info |
| `@locate` | Immortal | `<entity/player>` | Find entity/player location |
| `@teleport` | Immortal | `<player> <room>` | Force-teleport a player |

### Builder Workflow

```
@dig forest/clearing                → creates room in area "forest"
@link forest/clearing e forest/path_1 → connects rooms
@set forest/clearing.name = "Forest Clearing"
@set forest/clearing.flags = "peaceful"
@mob add forest forest/clearing deer 3  → spawns 3 deer on reset
@load forest/clearing deer            → spawns 1 deer right now
@area save                             → writes all edits to disk
```

All OLC edits are transactional — they modify the in-memory
`TemplateRegistry` immediately and are persisted to disk on `@area save`
or during the DirtyFlush phase (auto-save every 30s of OLC inactivity).

### Runtime Editing vs Template Files

Edits made via OLC commands are stored in a `builder_edits` overlay
(HashMap of diffs) applied on top of the file-based templates at load.
On `@area save`, edits are written back to the TOML file, the overlay
is cleared for that template, and a hot-reload is triggered.

This means:
- TOML files remain the **source of truth**
- OLC edits are ephemeral until saved (crash-safe: auto-save on DirtyFlush)
- Conflicts between file edits and OLC edits warn on next load

---

## Protocol Expansion Path

Protocol features are added in phases. The engine's `Connection` trait
abstracts the transport layer — `send()`/`supports()` at the bottom,
GMCP/MXP message builders at the top.

### Phase 0 — Telnet (line mode)

```
Protocol:  TCP/23 (default)
Features:  Local echo, ANSI 16-color codes (\x1b[31m)
Negot:     WILL/WONT ECHO (server turns off local echo)
Pending:   NAWS (window size), CHARSET (UTF-8 detection)
```

The `TelnetConnection` type handles IAC byte parsing, option negotiation,
and encoding. All higher protocols negotiate on top of Telnet.

> **Encryption note:** Plain telnet is unencrypted. For deployment, wrap
> with [stunnel](https://www.stunnel.org/) (TLS proxy, no code changes)
> or use an SSH tunnel. Native TLS support is not planned — stunnel is
> simpler, battle-tested, and works with any MUD client that speaks
> cleartext telnet.

### GMCP (Generic Mud Communication Protocol)

GMCP sends structured JSON messages over the telnet subnegotiation channel.
Negotiated by `IAC SB GMCP ... IAC SE` handshake.

**Supported message types (Phase 6):**

| Module | Message | Direction | Purpose |
|---|---|---|---|
| `Core` | `Hello` | Client → Server | Client identification (name, version, GMCP support) |
| `Core` | `Supports.Set` | Server → Client | Which GMCP modules the server enables |
| `Room` | `Info` | Server → Client | Current room (name, description, exits, portals, area, players) |
| `Char` | `Info` | Server → Client | Character stats (level, HP, max HP, XP, class, race) |
| `Char` | `Skills` | Server → Client | Skill list with ranks |
| `Char` | `Inventory` | Server → Client | Inventory + equipment contents (summary) |
| `Char` | `QuestList` | Server → Client | Active quest list with progress |
| `Comm` | `Channel` | Server → Client | Channel messages for client-side tab integration |
| `MGK` | `Target` | Client → Server | Client picks a combat target (UI click) |
| `MGK` | `Spell` | Client → Server | Client casts a spell from a hotbar |
| `IRE` | `Composer.Edit` | Server → Client | Request multi-line input from client editor |

GMCP is **opt-in** per-client; the server advertises support during
telnet negotiation and only sends GMCP if the client responds with
`Core.Hello`. Upgrade path: detect MTTS (Mud Terminal Type Standard)
for automatic 256-color + GMCP negotiation.

### MXP (MUD eXtension Protocol)

MXP enriches text with clickable links and embedded tags. Negotiated
via `IAC SB MXP ...` during telnet setup.

**Tag types (Phase 6):**

| Tag | Example | Behavior |
|---|---|---|
| `<send>` | `<send \"north\">north</send>` | Clickable text that sends command |
| `<send href>` | `<send href=\"examine sword\">sword</send>` | Named link with different display |
| `<a>` | `<a href=\"look goblin\">goblin</a>` | Standard link (MUSHclient style) |
| `<img>` | `<img src=\"gauge_hp\" />` | Embedded gauge (client-side rendering) |
| `<!ENTITY>` | `<!ENTITY $player \"%s\">` | Entity substitution for repeated text |

MXP is **locked** (requires `<VERSION>` header from client) to prevent
injection. Server only sends MXP tags to clients that have locked.

### WebSocket Bridge (Phase 6)

WebSocket provides an alternative to raw Telnet for browser-based clients.

```
Endpoint:  ws://<host>:<port>/ws
Protocol:  JSON-encoded MMCC messages over WebSocket frames
           (MMCC = Minimal MUD Client Communication)
```

**MMCC frame format:**

```json
{
  "type": "command",
  "payload": { "text": "look" }
}

{
  "type": "output",
  "payload": { "text": "\"Town Square\"\n...", "html": "<span>..." }
}
```

The WebSocket connection wraps the `Connection` trait: `send()` writes
a JSON output frame, `supports()` reports `Ansi=false` (the bridge
converts ANSI to HTML/CSS), `Supports::Html = true`.

WebSocket clients MUST send a `Core.Hello`-style handshake on connect
identifying their capabilities (GMCP via JSON, MXP via embedded tags).

### REST API (Phase 6)

A lightweight REST API for companion apps (mobile, web dashboard):

| Method | Endpoint | Purpose | Auth |
|---|---|---|---|
| `GET` | `/api/who` | List online players | None |
| `GET` | `/api/characters` | List account's characters | Session token |
| `GET` | `/api/characters/:id` | Character sheet (stats, skills, equipment) | Session token |
| `GET` | `/api/characters/:id/inventory` | Character inventory | Session token |
| `POST` | `/api/characters/:id/motd` | Toggle MOTD | Session token |

Auth via session token (returned on login, valid for 24h). The REST
server runs as a separate tokio task sharing the same `World` (read
lock only for GET endpoints).

### Protocol Feature Matrix

| Feature | Phase | Requires | Impact |
|---|---|---|---|
| ANSI 16-color | 0 | — | Basic colored output |
| NAWS | 1 | Telnet | Responsive layout for client windows |
| UTF-8 | 1 | Telnet | Unicode support in names/descriptions |
| 256-color | 2 | MTTS detection | Richer color palette |
| GMCP (Room, Char) | 6 | Telnet + GMCP | Client gauges, minimap, quest log |
| MCCP (compression) | 6 | Telnet | 5-10× bandwidth reduction |
| MXP | 6 | Telnet + lock | Clickable links, gauges |
| GMCP (MGK) | 6 | GMCP | Hotbar, target UI |
| WebSocket | 6 | HTTP server | Browser client support |
| REST API | 6 | HTTP server | Companion apps |
| MSSP | 6 | Telnet | MUD listing (Mudlet, etc.) |

---

## spade — Builder TUI & MUD Client

**spade** is the terminal-based builder TUI and MUD client for the engine.
Named after the `@dig` OLC command — a spade is what you dig with.

### Philosophy

- **Single tool for all builder workflows.** World editing, validation, content
  browsing, and live in-game testing all in one terminal app.
- **Mouse-first navigation** with full keyboard fallback. Builders can click,
  scroll, and drag their way through the world tree.
- **Offline + online modes.** Edit TOML files directly (offline) or connect
  to a running game server via WebSocket/telnet and edit live (online).
- **Data-driven everything.** Help screens, keybinding lists, and sidebar
  commands are defined as data, not hardcoded layout.

### Modes

| Mode | Invocation | Description |
|---|---|---|
| Builder (offline) | `spade` (default) or `spade --mode offline` (add `--content-path <dir>`) | TOML editor, world tree, validator, file browser |
| MUD client (online) | `spade --mode online` | Full MUD client with scrollable output, ANSI rendering, clickable names |
| Split | `spade --mode split` (F9 at runtime) | Builder tools left, MUD client right (50/50) |
| Connection profile | `spade connect <host> <port>` or `spade --profile <name>` | Quick-connect to a known server with saved profile |

### Screens & Panels (Builder Mode)

Default builder layout:

```
┌─────────┬─────────────────────────────────────┬──────────┐
│  World  │  Editor / Inspector / Preview       │ Validator│
│  Tree   │                                     │  Panel   │
│         │                                     │          │
│ areas/  │  ┌────────────────────────────────┐ │ [0] ✓    │
│  ├─mid… │  │ Name: Town Square              │ │ [1] ✓    │
│  │ rooms │  │                                │ │ [2] ⚠    │
│  │  ├─sq…│  │ Description:                   │ │ [3] ✓    │
│  │  │ …  │  │ A large cobblestone square…    │ │          │
│  │  └─te…│  └────────────────────────────────┘ │          │
│  └─  …   │  ┌────────────────────────────────┐ │          │
│ mobs/    │  │ Exits:                         │ │          │
│ items/   │  │ n → temple_01  [edit] [link]   │ │          │
│          │  │ s → market_03  [edit] [link]   │ │          │
│          │  └────────────────────────────────┘ │          │
│          │                                     │          │
├─────────┴─────────────────────────────────────┴──────────┤
│  Status: OFFLINE  |  World Tree  |  midgaard/square       │
└───────────────────────────────────────────────────────────┘
```

**Available screens (switch via Ctrl+1 through Ctrl+9):**

| Key | Screen | Description |
|---|---|---|
| Ctrl+1 | World Tree | Collapsible area/room/mob/item tree |
| Ctrl+2 | Template Editor | TOML field form editor for selected entity |
| Ctrl+3 | Room Graph | ASCII room map with mouse click navigation |
| Ctrl+4 | Entity Inspector | Entity detail table with scrolling |
| Ctrl+5 | Command Palette | Fuzzy-searchable commands |
| Ctrl+6 | Live Dashboard | Server status gauges (online only) |
| Ctrl+7 | Validation Panel | Error/warning list with jump-to-source |
| Ctrl+8 | File Browser | Content directory tree |
| Ctrl+9 | Script Console | Inline Rhai REPL |

### MUD Client Mode

![MUD Client Layout]

```
┌─────────────┬──────────────────────────────────────┐
│  Sidebar    │  Output Window (scrollable, ANSI)     │
│             │                                       │
│  ─────────  │  Town Square                          │
│  Movement   │  A large cobblestone square...        │
│  ▸ goto     │                                       │
│  ▸ at       │  [Exits: n e s w]                     │
│  ▸ teleport │                                       │
│             │  A guard stands at attention.         │
│  ─────────  │  Alice is here.                       │
│  Info       │  Bob is here.                         │
│  ▸ stat     │                                       │
│  ▸ scan     │  [-- 42% --]                          │
│  ▸ where    │                                       │
│             │                                       │
│  ─────────  │  ──────────────────────────────────── │
│  Admin      │  > _                                   │
│  ▸ kick     │  [Command:  ]                          │
│  ▸ ban      │                                       │
│  ▸ freeze   │                                       │
│  ▸ purge    │                                       │
│             │                                       │
│  ─────────  │                                       │
│  Building   │                                       │
│  ▸ @dig     │                                       │
│  ▸ @link    │                                       │
│  ▸ @set     │                                       │
│  ▸ @mob     │                                       │
│             │                                       │
│  ─────────  │                                       │
│  Session    │                                       │
│  [Connected]│                                       │
│  0:02:30    │                                       │
│  8 players  │                                       │
└─────────────┴──────────────────────────────────────┘
```

**Layout breakdown:**

| Region | Width | Content |
|---|---|---|
| Sidebar | 22 cols | Immortal command palette (scrollable, mouse-clickable) |
| Output window | Remaining | Scrollable game output with rendered ANSI |
| Input bar | Full width (below output) | One-line input with command history |
| Status bar | Full width (bottom) | Mode, connection status, player count |

#### Sidebar (MUD Mode)

Collapsible sections with clickable command buttons:

```rust
struct SidebarSection {
    name: &'static str,
    commands: Vec<SidebarCommand>,
    collapsed: bool,
}

struct SidebarCommand {
    label: &'static str,
    command: &'static str,    // what to send
    takes_args: bool,         // focus input bar with prefix typed
    confirm: bool,            // show confirmation dialog first
    access: AccessLevel,      // minimum level to display
    icon: Option<char>,       // optional glyph
}
```

Default sections:

| Section | Commands | Icon |
|---|---|---|
| Movement | `goto`, `at`, `teleport` | `↗` |
| Info | `stat`, `scan`, `where`, `olocate` | `ℹ` |
| Admin | `kick`, `ban`, `freeze`, `purge`, `slay`, `restore` | `⚙` |
| Building | `@dig`, `@link`, `@set`, `@mob`, `@area save` | `🔨` |
| Session | Disconnect, Reconnect, Mode toggle, Toggle sidebar | `🔌` |

Click behavior:
1. **Takes args:** focus jumps to input bar with command prefix pre-typed (e.g. `goto `)
2. **No args:** command sent immediately
3. **Confirm dialog:** confirmation popup appears before sending

Sections collapse/expand on header click. Sidebar toggled with Ctrl+B.

#### Clickable Entity Names

Names in the output window are detected and rendered as interactive spans.

**Detection (priority order):**
1. **GMCP `Room.Info`** — server sends structured player/mob list. Client highlights all known names in output.
2. **Heuristic parsing** — regex match on capitalized words in `look` output, cross-referenced against player list and known mob template names.

**Rendering:** Names shown in `brightcyan` (players) or `yellow` (mobs) with underline. Hover shows pointer cursor (`crossterm::SetCursorStyle::SteadyBlock`).

**Click → Context menu:**

```
┌──────────────────────┐
│ Alice                │
├──────────────────────┤
│ ℹ  Stat              │
│ ↗  Goto              │
│ 🗣  Tell              │
│ ⚡  Force             │
│ ──────────────────── │
│ ⚙  Kick              │
│ 🔨  Freeze            │
│ 🚫  Ban               │
│ ──────────────────── │
│ 📋  Copy name         │
│ 📋  Copy account      │
└──────────────────────┘
```

Right-click (or left-click on a name) opens context menu at cursor position.
Actions depend on target type (player vs mob vs item), user's access level,
and current selection state. `selected_target` is stored for sidebar quick
actions (clicking `stat` sends `stat Alice` if Alice was last clicked).

#### Output Window

```rust
struct OutputWindow {
    buffer: VecDeque<OutputLine>,   // last 5000 lines
    scroll: ScrollState,
    ansi_parser: AnsiParser,
    clickable_ranges: Vec<ClickableSpan>,
}

struct OutputLine {
    segments: Vec<StyledSegment>,
    timestamp: Instant,
}

struct StyledSegment {
    text: String,
    style: Style,
    clickable: Option<EntityRef>,
}

enum EntityRef {
    Player { name: String },
    Mob { template_id: String },
    Item { template_id: String },
}
```

Features:
- ANSI escape codes → ratatui `Style` (16-color, 256-color, bold/italic/underline)
- 5000-line buffer (configurable), scroll wheel / PgUp/PgDn / ↑↓ / Home/End
- `/` to search buffer, highlight matches, jump between results
- Ctrl+Shift+C to copy selection
- Auto-scroll to bottom on new output (pauses if user scrolled up)
- Line numbers toggle (Ctrl+L)
- Timestamps toggle (Ctrl+T)

#### Command Input

```
> goto midgaard/temple_01               ← typed text
  ↑↓         — cycle command history (last 200 commands)
  Tab        — autocomplete from command trie + player names + room targets
  Ctrl+R     — reverse search through history
  Ctrl+U     — clear line
  Ctrl+A/E   — beginning/end of line
  Enter      — send
```

**History persistence:** `~/.local/share/spade/history.txt` (200 entries, shared across sessions).

### Mouse Support

Uses `crossterm::EnableMouseCapture` on entry, `DisableMouseCapture` on exit.

| Action | Behavior |
|---|---|
| Left click | Select item in tree / table / list |
| Double left click | Open / edit selected item |
| Right click | Context menu (Copy ID, Delete, Clone) |
| Scroll wheel up/down | Scroll focused pane |
| Click on tab | Switch screen |
| Click on panel border | Drag to resize split panels |
| Shift+click | Select range in list |
| Ctrl+click | Toggle selection in list |

Mouse events disabled while text input is focused (normal typing).
Status bar shows `[Mouse: On]` / `[Mouse: Off]`, toggle with Ctrl+M.

### Scroll Support

Every scrollable pane manages its own scroll state:

```rust
struct ScrollState {
    offset: usize,
    visible_lines: usize,
    total_lines: usize,
}

impl ScrollState {
    fn scroll_up(&mut self);
    fn scroll_down(&mut self);
    fn page_up(&mut self);
    fn page_down(&mut self);
    fn percent(&self) -> f32;
}
```

Scrollbar rendered on the right edge of each pane:
```
║  ▓  ║   ← dark bar = content position
║  ▓  ║
║  ▓  ║
```

Scroll percentage shown in top or bottom corner when scrolled: `-- 55% --`.
Scroll wheel, PgUp/PgDn, ↑↓, Home/End all work in the focused pane.

### Help Screen

Modal overlay triggered by `Ctrl+H` or `?`. Covers center 70% of terminal.
Dismissed by Escape.

```
╔══════════════════════════════════════════════════════════╗
║                    Help — spade v0.1                     ║
╠══════════════════════════════════════════════════════════╣
║ Navigation                                               ║
║   Tab / Shift+Tab   Cycle panels                          ║
║   ↑ ↓ ← →          Navigate / move                        ║
║   Enter             Open selected                         ║
║   /                 Search / filter                        ║
║   Ctrl+P            Command palette                       ║
║                                                          ║
║ Editing                                                   ║
║   Ctrl+S            Save (offline) / Send (online)        ║
║   Ctrl+Z            Undo                                  ║
║   Ctrl+C            Copy selection                        ║
║   Ctrl+V            Paste                                 ║
║                                                          ║
║ Views                                                     ║
║   Ctrl+1            World Tree                            ║
║   Ctrl+2            Template Editor                       ║
║   Ctrl+3            Room Graph                            ║
║   Ctrl+4            Entity Inspector                      ║
║   Ctrl+5            Command Palette                       ║
║   Ctrl+6            Live Dashboard                        ║
║   Ctrl+7            Validation Panel                      ║
║   Ctrl+8            File Browser                          ║
║   Ctrl+9            Script Console                        ║
║                                                          ║
║ Mouse                                                     ║
║   Left click        Select                                ║
║   Double click      Open                                  ║
║   Right click       Context menu                          ║
║   Scroll wheel      Scroll pane                           ║
║   Click tab         Switch view                           ║
║   Ctrl+M            Toggle mouse mode                     ║
║                                                          ║
║ General                                                   ║
║   Ctrl+H / ?        This help screen                      ║
║   Ctrl+Q / Esc      Go back / close                       ║
║   Ctrl+D            Quit spade                            ║
║   F5                Validate / refresh                    ║
║   F10               Toggle rich preview                   ║
║   Ctrl+B            Toggle sidebar (MUD mode)             ║
║   Ctrl+K            Clear output buffer                   ║
║   Ctrl+L            Toggle line numbers                   ║
║   Ctrl+T            Toggle timestamps                     ║
║   Ctrl+R            Reverse search history                ║
║   Tab               Autocomplete                          ║
║   F9                Toggle split view                     ║
║   Ctrl+Shift+M      Toggle MUD / builder mode             ║
╚════════════════════════════════════════════════════════════╝
```

Help content is data-driven — a static `Vec<HelpSection>` struct so new
keybindings can be added without layout code changes:

```rust
struct HelpSection {
    title: &'static str,
    entries: Vec<HelpEntry>,
}

struct HelpEntry {
    key: &'static str,      // "Ctrl+H" / "Tab" / "F5"
    description: &'static str,
}
```

### UI Design Principles

- **Focus-first navigation:** Tab cycles pane focus. Focused pane has a
  highlighted border (bright white / cyan). All keyboard input routes to
  the focused pane.
- **Consistent color scheme:**
  - Panel borders: `Cyan`
  - Focused border: `BrightWhite`
  - Selected item: `Yellow` background
  - Error text: `Red`
  - Warning text: `Yellow`
  - Success: `Green`
  - Keybinding hints: dim white, shown inline in pane headers
- **Status bar** (1 line, bottom): mode (OFFLINE / ONLINE), active screen,
  current file, cursor position, mouse state.
- **Breadcrumb trail** at top of content panes: `Areas > midgaard > rooms > square`
- **Empty states** with helpful prompts: `"No rooms yet. Press @dig to create one."`
- **Confirmation dialogs** before destructive actions:
  `"Delete area 'midgaard'? This cannot be undone. (y/N)"`
- **Toast notifications** (bottom-right, 3s auto-dismiss):
  `"Area created."`, `"Validation: 2 errors, 3 warnings."`

### Session Management (MUD Mode)

```rust
enum SessionState {
    Disconnected,
    Connecting { host: String, port: u16 },
    Negotiating,                       // telnet IAC handshake
    LoggingIn { attempts: u8 },
    Playing,
}

struct MudSession {
    state: SessionState,
    transport: Transport,       // Telnet | WebSocket
    connection: Box<dyn Connection>,
    output: OutputWindow,
    input_history: Vec<String>,
    last_target: Option<EntityRef>,
    known_players: HashMap<String, EntityRef>,
    gmcp_modules: HashSet<String>,
}
```

**Connection profiles** stored in `~/.config/spade/profiles.toml`:

```toml
[profiles.local]
host = "localhost"
port = 4000
mode = "telnet"
username = "admin"

[profiles.live]
host = "mud.example.com"
port = 4000
mode = "websocket"
tls = true
```

### ANSI → ratatui Mapping

| ANSI code | ratatui Style |
|---|---|
| `\x1b[0m` | Reset to default |
| `\x1b[1m` | Bold |
| `\x1b[3m` | Italic |
| `\x1b[4m` | Underline |
| `\x1b[5m` | Blink (gated by user preference) |
| `\x1b[30-37m` | Color::Black .. Color::White |
| `\x1b[38;5;Nm` | Color::Indexed(N) |
| `\x1b[90-97m` | Color::BrightBlack .. Color::BrightWhite |

### Dependencies (tui/Cargo.toml)

```toml
[package]
name = "spade"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.29"
crossterm = "0.28"
tokio = { version = "1", features = ["full"] }
tokio-tungstenite = { version = "0.24", features = ["native-tls"], optional = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
dirs = "6"
chrono = "0.4"
regex = "1"
fuzzy-matcher = "0.3"
syntect = { version = "5", optional = true }   # syntax highlighting for TOML preview
```

Optional features:

```toml
[features]
default = ["tls"]
tls = ["tokio-tungstenite/native-tls"]
syntax-highlight = ["syntect"]
```

### Keybinding Summary

| Key | Action | Scope |
|---|---|---|
| Tab / Shift+Tab | Cycle pane focus | All |
| Enter | Open selected / confirm | All |
| Escape / Ctrl+Q | Go back / close modal | All |
| Ctrl+H / ? | Toggle help screen | All |
| Ctrl+D | Quit spade | All |
| / | Search / filter focused pane | All |
| F5 | Validate / refresh | All |
| F10 | Toggle rich TOML preview | Builder |
| Ctrl+1-9 | Switch screen | Builder |
| Ctrl+S | Save (offline) / Send (online) | Builder |
| Ctrl+Z | Undo | Editor |
| Ctrl+C / V | Copy / paste | All |
| Ctrl+M | Toggle mouse mode | All |
| Ctrl+B | Toggle sidebar | MUD |
| Ctrl+K | Clear output buffer | MUD |
| Ctrl+L | Toggle line numbers | MUD |
| Ctrl+T | Toggle timestamps | MUD |
| Ctrl+R | Reverse search history | MUD Input |
| Ctrl+U | Clear input line | MUD Input |
| Ctrl+A/E | Beginning / end of line | MUD Input |
| Tab | Autocomplete | MUD Input |
| F9 | Toggle split view | All |
| Ctrl+Shift+M | Toggle MUD / builder mode | All |
| ↑↓ | Navigate list / scroll | All |
| PgUp/PgDn | Page scroll | All |
| Home/End | Jump to top / bottom | All |
| Left click | Select | All |
| Double left click | Open | All |
| Right click | Context menu | All |
| Scroll wheel | Scroll pane | All |
| Shift+click | Select range | Lists |
| Ctrl+click | Toggle selection | Lists |

---

## MCP Server — AI Agent World-Building

An MCP (Model Context Protocol) server that exposes the full content toolset
to AI agents (Claude, etc.). Agents can read, create, edit, and validate
game world data via natural language — accelerating content creation.

### Modes

| Mode | Trigger | Data Source | Write Path |
|---|---|---|---|---|
| Offline | `mcp` (default) | TOML files in `content/` (override with `--content-path`) | Atomic write (temp + rename) |
| Online | `mcp --db <path>` | SQLite DB | REST bridge to game server |

Offline mode is primary — AI agents edit TOML files directly, validation
runs locally, and a human builder reviews changes before the game loads.

### Transport

- **Primary:** stdio (MCP standard — Claude Desktop, VS Code, Cursor, etc.)
- **Future:** SSE (HTTP) for remote agent connections

### Tools

Each tool follows the MCP tool schema (name, description, input JSON schema).

| Tool | Description | Write |
|---|---|---|
| `list_areas` | List all areas (optional `query` filter) | No |
| `get_area` | Get full area details by `key` | No |
| `create_area` | Create a new area (`key`, `name`, `description`, `default_room_type`) | Yes |
| `update_area` | Update area metadata (`key`, optional fields) | Yes |
| `delete_area` | Delete area + all rooms inside (`key`, `confirm`) | Yes |
| `list_rooms` | List rooms in an area (`area_key`, optional `query`) | No |
| `get_room` | Get full room details (`area_key`, `room_key`) | No |
| `create_room` | Create a new room (`area_key`, `room_key`, `name`, `description`, `exits?`, `portals?`) | Yes |
| `update_room` | Update room fields (optional fields) | Yes |
| `delete_room` | Delete a room (`area_key`, `room_key`, `confirm`) | Yes |
| `link_rooms` | Link two rooms with an exit (`from_area`, `from_room`, `direction`, `to_area`, `to_room`) | Yes |
| `add_portal` | Add a portal exit to a room (`area_key`, `room_key`, `keyword`, `dest_area`, `dest_room`, `description`, `flags?`) | Yes |
| `remove_portal` | Remove a portal exit (`area_key`, `room_key`, `keyword`) | Yes |
| `list_mobs` | List mob templates (optional `query`, `area` filter) | No |
| `get_mob` | Get mob template details by `key` | No |
| `create_mob` | Create a mob template (`key`, `name`, `level`, attributes, equipment?, loot?, faction?) | Yes |
| `update_mob` | Update mob template fields | Yes |
| `delete_mob` | Delete mob template (`key`, `confirm`) | Yes |
| `list_items` | List item templates (optional `query`, `item_type` filter) | No |
| `get_item` | Get item template details by `key` | No |
| `create_item` | Create an item template (`key`, `name`, `item_type`, attributes, gates?, triggers?) | Yes |
| `update_item` | Update item template fields | Yes |
| `delete_item` | Delete item template (`key`, `confirm`) | Yes |
| `list_quests` | List quest templates (optional `query`, `giver`, `area` filter) | No |
| `get_quest` | Get quest details by `key` | No |
| `create_quest` | Create a quest (`key`, `title`, objectives, rewards, requirements?) | Yes |
| `update_quest` | Update quest fields | Yes |
| `delete_quest` | Delete quest (`key`, `confirm`) | Yes |
| `list_recipes` | List recipe templates (optional `query`, `station` filter) | No |
| `get_recipe` | Get recipe details by `key` | No |
| `create_recipe` | Create a recipe (`key`, `name`, `station`, materials, result) | Yes |
| `list_factions` | List faction definitions | No |
| `get_faction` | Get faction details by `key` | No |
| `create_faction` | Create a faction (`key`, `name`, `description`, relationships?) | Yes |
| `list_shops` | List shop templates | No |
| `get_shop` | Get shop details by `key` | No |
| `create_shop` | Create a shop (`key`, `name`, inventory entries?) | Yes |
| `list_skills` | List skill templates (optional `skill_type` filter) | No |
| `get_skill` | Get skill details by `key` | No |
| `get_race` | Get race template by `key` | No |
| `get_class` | Get class template by `key` | No |
| `validate` | Run content validation (`scope?`: all/area/type) | No |
| `search` | Fuzzy search all content (`query`, `type?` filter) | No |
| `get_stats` | Content summary statistics | No |

### Resources

MCP resources expose content as structured data for agent context windows:

| URI Pattern | Description | MIME |
|---|---|---|
| `content://areas/` | List all area keys | `text/plain` |
| `content://areas/{key}` | Full area TOML | `application/toml` |
| `content://areas/{key}/rooms/{room_key}` | Room TOML | `application/toml` |
| `content://mobs/{key}` | Mob template TOML | `application/toml` |
| `content://items/{key}` | Item template TOML | `application/toml` |
| `content://skills/{key}` | Skill template TOML | `application/toml` |
| `content://races/{key}` | Race template TOML | `application/toml` |
| `content://classes/{key}` | Class template TOML | `application/toml` |
| `content://quests/{key}` | Quest template TOML | `application/toml` |
| `content://recipes/{key}` | Recipe template TOML | `application/toml` |
| `content://factions/{key}` | Faction template TOML | `application/toml` |
| `content://shops/{key}` | Shop template TOML | `application/toml` |
| `content://validation/` | Current validation report | `text/plain` |
| `content://stats/` | Content statistics | `application/json` |

### Prompts (Guided Workflows)

Prompt templates that guide agents through common content creation:

| Prompt | Description |
|---|---|
| `create_area_flow` | "Create a new area." Walks through key, name, rooms, exits, mobs |
| `review_content` | "Review this content." Runs validation, checks cross-refs, suggests improvements |
| `balance_encounter` | "Balance this encounter." Analyzes mob levels vs area tier |
| `design_quest_chain` | "Design a quest chain." Guides prerequisites, objectives, branching, rewards |

### Crate Layout

```
mcp/
├── Cargo.toml
└── src/
    ├── main.rs         # Entry: --mode, --port, --content-path, start McpServer
    ├── server.rs       # McpServer: stdio transport, request dispatch
    ├── tools/          # Tool implementations by content type
    │   ├── mod.rs
    │   ├── area.rs     # create_area, update_area, delete_area, list_areas, get_area
    │   ├── room.rs     # create_room, update_room, delete_room, link_rooms
    │   ├── mob.rs      # create_mob, update_mob, delete_mob
    │   ├── item.rs     # create_item, update_item, delete_item
    │   ├── quest.rs    # create_quest, update_quest, delete_quest
    │   ├── recipe.rs   # create_recipe, delete_recipe
    │   ├── faction.rs  # create_faction, delete_faction
    │   ├── shop.rs     # create_shop, delete_shop
    │   ├── race.rs     # get_race, list_races
    │   ├── class.rs    # get_class, list_classes
    │   ├── skill.rs    # get_skill, list_skills
    │   ├── validate.rs # validate
    │   └── search.rs   # search
    ├── resources/      # Resource providers
    │   ├── mod.rs
    │   ├── templates.rs
    │   ├── validation.rs
    │   └── stats.rs
    └── prompts/        # Prompt templates
        ├── mod.rs
        ├── builder.rs
        └── reviewer.rs
```

### Dependencies

```toml
[dependencies]
rmcp = { version = "1.7", features = ["server", "macros", "schemars", "transport-io"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
tokio = { version = "1", features = ["full"] }
fuzzy-matcher = "0.3"
chrono = "0.4"
```

`rmcp` is the [official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk)
(v1.7.0, 2.5M+ downloads/month, Apache-2.0). It provides full MCP protocol
support over stdio transport. The `#[tool]` / `#[prompt]` macros handle
JSON-RPC wiring automatically — each tool function maps to a named MCP tool
with typed JSON Schema input/output derived from Rust types.

Key features used:
- **`server`** — `ServerHandler` trait, service lifecycle, peer notifications
- **`macros`** — `#[tool]`, `#[tool_router]`, `#[prompt]`, `#[prompt_router]` for declarative tool/prompt definition
- **`schemars`** — JSON Schema generation from Rust structs for tool parameter validation
- **`transport-io`** — stdio transport (read stdin, write stdout), the MCP standard

### DAG

- `mcp` depends on `core` (types) only. It loads TOML files directly for
  offline mode and connects via HTTP for online mode.
- Does NOT depend on `server`, `data`, `scripting`, or `bin`.
- Can be built and run independently of the game server.

### Flow Diagrams

**Offline mode:**
```
AI Agent (Claude)          mcp server               Filesystem
       │                       │                       │
       │── list_areas() ──────>│                       │
       │                       │── read <content_dir>/areas/ │
       │                       │<── file list ─────────│
       │<── response ──────────│                       │
       │                       │                       │
       │── create_item() ─────>│                       │
       │                       │── write items/.tmp    │
       │                       │── rename .tmp → .toml │
       │                       │<── ok ────────────────│
       │<── response ──────────│                       │
```

**Online mode:**
```
AI Agent (Claude)          mcp server               Game Server (REST)
       │                       │                       │
       │── list_rooms() ──────>│                       │
       │                       │── GET /api/areas/... ─>│
       │                       │<── JSON response ─────│
       │<── response ──────────│                       │
       │                       │                       │
       │── create_room() ─────>│                       │
       │                       │── POST /api/rooms ───>│
       │                       │<── created ───────────│
       │<── response ──────────│                       │
```

### Phasing

| Phase | Items |
|---|---|
| 5 | Crate scaffold, offline mode, area/room/mob/item tool groups, validate tool |
| 6 | Quest/recipe/faction/shop tool groups, search tool, stats resource, online mode, prompts |

---

## Development Phases

### Phase 0 — Foundation
- [x] Cargo workspace & crate skeleton
- [x] Core types (`Room`, `Exit`, `Direction`, entity management)
- [x] Tokio TCP listener with telnet negotiation (IAC stripping + WILL/WONT ECHO + SGA)
- [x] Basic ECS world with `hecs`
- [x] Raw line-in/line-out to connected players
- [x] Resource pool components (`Stamina`, `Mana`, `Energy`, `Psi`)
- [x] Unit tests (49 tests across all crates)
- [x] Encryption deployment guide (stunnel recommendation)
- [x] Void room (inescapable — `VoidRoom` marker, zero exits, blocks all relocation)
- [x] CLI config (`--port`/`--host` flags)
- [x] Graceful shutdown (SIGINT/SIGTERM via `tokio::select!`)
- [x] Player spawn — connects into void room with `Position` component

### Phase 1 — World & Movement
- [x] `ConnectionRegistry` — `HashMap<Entity, Sender<Vec<u8>>>` for room broadcasts
- [x] `say` — room broadcast (speaker: `You say, "..."`, others: `Player says, "..."`)
- [x] `look` — rooms, occupants, visible exits (`RoomExits`)
- [x] Movement commands — `n`/`s`/`e`/`w`/`u`/`d` + `ne`/`nw`/`se`/`sw` + long forms
- [x] Void room movement check — block all relocation
- [x] Auto-`look` on room entry + room enter/leave broadcasts
- [x] Player cleanup — despawn entity + registry remove on disconnect
- [x] `core::format` module — `Color`, `Modifier`, `RichText`, `RichText::render()`, `parse_tags()`
- [x] Connection feature flags — `Ansi`, `ExtendedColor`, `Blink`
- [x] ANSI color conventions (room name, exits, player name, say, etc.)
- [x] Unit tests — movement, void blocking, room broadcast, ANSI rendering

### Phase 2 — Character System
- [ ] Connection state machine (pre-Playing states)
- [ ] Account creation (username + password, argon2 hashing)
- [ ] Login flow (banner/MOTD → username → password)
- [ ] Character select screen (list existing + create new)
- [ ] Character creation wizard (name → race → class → attributes → confirm)
- [ ] Race→class filtering in creation wizard
- [ ] `characters` SQLite table + schema migration
- [ ] TOML race/class template loading
- [ ] Unified SkillDef + skill_type enum
- [ ] Expanded RaceTemplate with constraints (allowed_classes, traits, languages, racial_abilities)
- [ ] Expanded ClassTemplate with constraints (allowed_races, allowed_alignments, BAB/saves, skill caps, stances, passives)
- [ ] Races → classes → skills validation pipeline (cross-reference checks)
- [ ] Derived indices in TemplateRegistry (class_skill_index, race_class_index, prestige_index, trainer_index)
- [ ] Auto-grant racial abilities + class auto-skills on character creation
- [ ] `Attributes`, `Level`, `Experience`, `LearnedSkills` components
- [ ] Starting room spawn on character confirm
- [ ] `motd` command (toggle `show_motd` flag)

### Phase 3 — Combat & Equipment
- [ ] `Health`, `Damage` components
- [ ] Combat system (attack/damage rolls)
- [ ] Damage type system (resistance/vulnerability multipliers, stacking across sources)
- [ ] Weapon styles (two-handed 1.5× STR, dual-wield penalties, off-hand slot)
- [ ] `Equipment`, `Inventory` components
- [ ] Weapon/armor items (with class/race/alignment/skill restriction gates)
- [ ] NPC mobiles with basic AI (wander, aggro, patrol, stationary)
- [ ] Mob template system (TOML loading, loot tables, equipment, faction, skills)
- [ ] Stance subsystem (activation, deactivation, combat modifiers)
- [ ] Passive system (application on login/level-up)
- [ ] Skill cap system (max_rank formula evaluation)
- [ ] Training system (trainer NPC, cost formula, skill tree prerequisites)
- [ ] Item triggers (on_hit, on_wear, on_remove, on_use, on_kill)
- [ ] Item sets TOML + SetTracker component + runtime bonus application
- [ ] Random loot quality/affix rolling

### Phase 4 — Advanced Gameplay
- [ ] Crafting system (recipes TOML, stations, materials, quality scaling, success/failure)
- [ ] Quest system (quest TOML, objective types, event-driven progress tracking)
- [ ] Faction system (faction TOML, standing model, rank tiers, relationships, aggro)
- [ ] Prestige class system (gate validation, @prestige command)
- [ ] Multi-classing system (XP penalty, favored class, alignment restrictions, @multi_class command)
- [ ] Spell system (unified in Skill System, skill_type == Magic)
- [ ] Shop & Economy (NPC buy/sell, file-based ShopTemplate, restock)
- [ ] Resource pools (Stamina, Mana, Energy, Psi) + regeneration
- [ ] Resource cost system (skills consume resources from appropriate pool)
- [ ] Optional PvP flagging

### Phase 5 — OLC & Tooling
- [ ] Online creation commands (@dig, @link, @set, @mob, @area, @item, etc.)
- [ ] Zone/area management, area reset system
- [ ] Telnet negotiation (IAC state machine, NAWS, terminal type, keepalive)
- [ ] Schema migration system (version tracking, migration functions, startup flow)
- [ ] Hot-backup system (SQLite online backup, hourly schedule, retention)
- [ ] Rhai scripting engine integration
- [ ] Scriptable triggers & events
- [ ] Hot-reload all content types (skills, items, mobs, recipes, quests, factions, shops, scripts)
- [ ] Builder-created help files
- [ ] **spade crate scaffold** (Cargo.toml, main.rs, crossterm init, App event loop)
- [ ] **spade offline builder mode** — world tree, TOML editor, file browser
- [ ] **spade components** — ScrollState, Tree, Tabs, Table, Form, Modal, ContextMenu
- [ ] **spade help screen** — data-driven modal with all keybindings documented
- [ ] **spade mouse support** — crossterm capture, click/double-click/right-click/scroll
- [ ] **spade scroll support** — per-pane ScrollState, scrollbar, percentage indicator
- [ ] **spade validator panel** — load content, run cross-reference checks, display diagnostics
- [ ] **spade room graph** — ASCII map view of room connections
- [ ] **MCP crate scaffold** — Cargo.toml, server.rs, stdio transport
- [ ] **MCP offline mode** — area/room/mob/item CRUD via MCP tools
- [ ] **MCP validate tool** — run validator, return diagnostics to agent

### Phase 6 — spade MUD Client & Protocol Expansion
- [ ] WebSocket bridge (JSON MMCC frames, ANSI→HTML conversion)
- [ ] MCCP, GMCP (Room, Char, Comm, MGK modules), MXP tags, MSSP
- [ ] REST API endpoints (/api/who, /api/characters, /api/inventory)
- [ ] **spade MUD client mode** — output window, ANSI parser, scrollable buffer
- [ ] **spade input bar** — history, autocomplete, Ctrl+R reverse search
- [ ] **spade sidebar** — collapsible command sections, mouse-clickable
- [ ] **spade clickable names** — GMCP + heuristic detection, context menu
- [ ] **spade connection profiles** — profiles.toml, connect dialog
- [ ] **spade session management** — reconnect, state machine, login flow
- [ ] **spade split mode** — builder + MUD client side by side
- [ ] **spade live dashboard** — server status gauges (connected players, resources)
- [ ] **spade script console** — inline Rhai REPL
- [ ] **spade syntax-highlighted TOML preview** — F10 toggle
- [ ] **MCP quest/recipe/faction/shop CRUD** — remaining tool groups
- [ ] **MCP search tool** — fuzzy search across all content types
- [ ] **MCP resources** — templates.rs, validation.rs, stats.rs providers
- [ ] **MCP online mode** — REST bridge to game server for live editing
- [ ] **MCP prompts** — create_area_flow, review_content, balance_encounter, design_quest_chain
- [ ] Performance profiling & optimization
