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

Event-driven loop using `tokio::select!` — no fixed tick. The server
acquires a write lock on `World` for each branch (pulse, event, input).

![diagram]
```
tokio::select! {
    ◄─ shutdown_signal ── flush + WAL checkpoint + exit
    ◄─ scheduler.next  ── run_system_phase(phase)
    ◄─ event_bus.recv  ── dispatch_event(event)
    ◄─ player_input    ── commands.execute(world, conn, line)
}
```

### Scheduler

Singleton resource maintaining named intervals, each producing a `Pulse`
on an mpsc channel:

| Phase | Interval | Description |
|---|---|---|
| `Movement` | 100ms | Queued movement commands |
| `Combat` | 2s | Combat round tick |
| `Regeneration` | 6s | HP/mana regen |
| `Weather` | 5m | Zone weather updates |
| `DirtyFlush` | 5s | Persist dirty entities |

Phases are independent and fire concurrently. Each iterates registered
systems sorted by priority (lower runs first) with `&mut World`.

---

## Systems Architecture

Game logic is organized into systems implementing the `System` trait
(run on pulse, handle_event on subscribed events, priority-sorted within phases).

### Built-in Systems

| System | Phase(s) | Subscribes | Pri | Responsibility |
|---|---|---|---|---|
| `MovementSystem` | Movement | — | 10 | Process direction commands, update `Position`, emit `PlayerMoved` |
| `FollowSystem` | Movement | `PlayerMoved` | 20 | Move followers behind leader |
| `EchoSystem` | — | `PlayerSaid`, `PlayerMoved`, `PlayerDied`, `ItemDropped` | 10 | Broadcast messages to room occupants |
| `CombatSystem` | Combat | `PlayerAttacked` | 20 | Combat round: hit, damage, death |
| `StanceSystem` | Combat | — | 15 | Apply stance modifiers |
| `AISystem` | Combat | — | 30 | NPC state machine (idle/wander/patrol/aggro/flee) |
| `FormationSystem` | Combat | — | 25 | Apply formation bonuses to group |
| `RegenSystem` | Regeneration | — | 10 | Regen HP/mana/resource pools |
| `EffectExpirySystem` | Regeneration | — | 20 | Tick down active effect durations |
| `PassiveApplicationSystem` | Regeneration | `PlayerLeveled` | 30 | Apply/remove class passives on login/level-up |
| `WeatherSystem` | Weather | — | 10 | Update zone weather states |
| `DirtyFlushSystem` | DirtyFlush | — | 50 | Flush dirty entities to SQLite |
| `SkillRequirementSystem` | DirtyFlush | — | 40 | Check skill gates on equipped items, auto-remove |
| `GroupCleanupSystem` | DirtyFlush | — | 45 | Sweep stale followers/disconnected members |
| `CorpseSystem` | DirtyFlush | — | 60 | Decay expired corpses, transfer contents |
| `AreaResetSystem` | DirtyFlush | — | 70 | Area resets past interval |
| `SetBonusSystem` | — | `ItemWorn`, `ItemRemoved` | 10 | Evaluate item set bonuses on equip/unequip |
| `QuestProgressSystem` | — | `MobDied`, `ItemPickedUp`, `SkillUsed` | 20 | Update quest objectives |
| `CraftingSystem` | — | `SkillUsed` (craft) | 20 | Execute crafting flow |
| `ScriptTriggerSystem` | — | `ScriptTrigger` | 100 | Evaluate attached Rhai scripts |

Systems are stored `PhaseMap<Vec<Box<dyn System>>>`, priority-sorted at
registration. All dispatch acquires a **write lock** on `World` (read-write
split deferred until profiling warrants it).

---

## Event Bus

Events dispatch over a `tokio::sync::broadcast` channel (single sender,
one receiver per subscribed system). Each event carries an `EventEnvelope`
with `id`, `tag`, `timestamp`, and a `GameEvent` payload.

### Event Tags

```
PlayerSaid | PlayerMoved | PlayerAttacked | PlayerDied | PlayerLeveled
MobDied | MobKilled | ItemPickedUp | ItemDropped | ItemWorn | ItemRemoved
SkillUsed | SkillTrained | RoomEntered | QuestUpdated | QuestCompleted
FactionChanged | SetBonusChanged | CorpseDecayed | ContentReloaded
ScriptTrigger | Pulse(Phase)
```

### Dispatch

Systems declare interest via `subscribed_events()`. Dispatched in priority
order; `handle_event()` returning `true` consumes the event (no further
dispatch). ScriptTriggers always run last (priority 100). Default is
**in-band** (synchronous under World lock); opt-in **out-of-band**
(spawned as tokio task for logging/analytics).

---

## ECS Component Design

### Spatial

- `Position { room: Entity }` — references a room entity
- `Room { name, description }` — room metadata
- `Exit { direction, dest, flags }` — room exit
- `RoomExits(Vec<Exit>)` — one per room entity
- `PortalExit { keyword, dest, description, flags }` — keyword-based portal exit
- `RoomPortals(Vec<PortalExit>)` — one per room entity
- `PortalFlags` bitmask: `PORTAL_HIDDEN`
- `RoomFlags` bitmask: `PORTAL_IN`, `PORTAL_OUT`, `NO_TELEPORT_IN`, `NO_TELEPORT_OUT`
- `VoidRoom` — marker: inescapable room, blocks all movement/recall/teleport
- `Teleportable(bool)` — targetable by player teleport spells
- `Direction`: North, South, East, West, Up, Down, NE, NW, SE, SW

The **void room** is a singleton inescapable room — the default spawn point before
character creation (Phase 2). It has the `VoidRoom` marker component and zero
exits. All movement commands, recall spells, teleport effects, and similar
relocation mechanics must check for `VoidRoom` on the origin or destination
and reject the action unless the actor has immortal bypass or it's an approved
codepath (e.g. finalizing character creation).

### Character

- `Player { account_id }` — player entity
- `Npc { template_id }` — NPC entity
- `Attributes { str, dex, int, wis, con, cha }` — 6 core stats (u8)
- `Health { current, max }` — hit points (i32)
- `Level(u8)` / `Experience(u64)` — character level and XP
- `Stamina`, `Mana`, `Energy`, `Psi` — resource pools `{ current, max }` (u16), gated by class/skill access
- `Immortal { incognito, holylight, build_mode }` — immortal status flags
- `Teleportable(bool)` — can this entity be teleported by other players?

### Combat

- `CombatTarget(Entity)` — current target
- `Damage(i32)` — damage value
- `Armor { base, bonus }` — base + bonus armor

### Items

- `Item { template_id, flags }` — item instance
- `Inventory(Vec<Entity>)` — item list on a character
- `EquipmentSlot`: Head, Neck, Torso, Arms, Hands, Finger, Legs, Feet, Weapon, Shield

### Flexible / OLC

- `Attributes(HashMap<String, String>)` — KV store for builder-defined data

### Persistence

- `Dirty` — marker: entity needs DB write
- `DbId(i64)` — maps entity to SQLite row

### Character & Progression

- `Name(String)`, `Description(String)`, `Alignment(String)`, `Wallet(u64)`
- `CombatStats { base_attack_bonus, fort_save, ref_save, will_save }`
- `ActiveStance(Option<String>)` — name of active stance
- `PassiveEffect { id, effect }` — applied passive bonus
- `LearnedSkills { skills, cooldowns }` — map of skill ID → SkillRank
- `SkillRank(u16)` — rank in a skill
- `MultiClassInfo { classes: Vec<ClassEntry> }` — multi-class tracking
- `ClassEntry { class_id, level, is_favored }`
- `FactionStanding { standings: HashMap<String, i32> }` — faction → standing
- `QuestLog { active, completed }` — active quest progress + completed quest IDs
- `QuestProgress { quest_id, objectives, started_at }`
- `ObjectiveState { objective_index, current, completed }`
- `LearnedRecipes { recipes: Vec<String> }`

### Item Progression

- `SetTracker { active_sets }` — map of active item sets
- `ActiveSet { template_id, counts, equipped, active_tiers }` — piece tracking + earned bonuses
- `ItemTriggers { on_hit, on_wear, on_remove, on_use }` — trigger effects per event
- `TriggerEffect { chance, skill_id, target }` — trigger skill execution
- `TriggerTarget`: Self, Attacker, Room, Random

---

## Combat System

### Components

- `CombatTarget(Entity)` — current target
- `Damage(i32)` — damage value
- `Armor { base, bonus }` — base + bonus armor
- `DamageType`: Slash, Pierce, Bludgeon, Fire, Cold, Lightning, Acid, Poison, Magic, True

### Attack Flow (Combat pulse every 2s)

1. Check same room (melee) or line-of-sight (ranged)
2. Hit: `d20 + level + str_mod ≥ AC` (melee) / `d20 + level + dex_mod ≥ AC` (ranged)
   — Natural 1 auto miss; Natural 20 auto crit (×2 damage)
3. Damage: `weapon_damage + str_mod + level/5`
4. Apply damage, emit `PlayerAttacked` / `MobAttacked`
5. If target dead: emit death event, grant XP (`victim.level² × 50`),
   spawn corpse, clear combat

**Defense:** `AC = 10 + level + dex_mod + armor.total()`

### NPC AI

State machine per NPC: `Idle → Combat → Flee → Idle`. Wander (random exit
every 3–5 pulses), aggro (configurable range), threat table (damage/heal/taunt
→ attacks highest threat).

### Damage Types & Resistances

Resistances are multipliers from race, class, equipment, buffs (stacked
multiplicatively). Configured via TOML: `resistances = { fire = 0.5 }`
on templates or `[effect] type = "buff" stat = "resistance"` on skills.

| Mult | Meaning |
|---|---|
| `2.0` | Vulnerable |
| `1.0` | Normal |
| `0.5` | Resistant |
| `0.0` | Immune |
| `-1.0` | Absorbed (healed) |

### Weapon Styles

- **Two-handed:** 1.5× STR mod damage, 1.2× speed, no shield
- **Dual-wield:** Primary −4 hit, off-hand −8 hit + 0.5× STR; penalties halved
  with `ambidexterity` feat. Off-hand grants extra attack roll each round.

---

## Corpse & Loot

On death, a corpse entity spawns containing the victim's `Inventory` +
`Equipment`. Player corpses: `GroupOnly` loot rule, 10min decay. NPC
corpses: `Public`, 5min decay. `CorpseSystem` (DirtyFlush phase) sweeps
expired corpses, transfers remaining items to room floor, emits
`CorpseDecayed`. Corpses are transient (no SQL persistence).

```
loot <corpse>              — show contents
loot <corpse> <item>       — take item
loot <corpse> all          — take all
get <item> corpse          — alias
```

- `Corpse { owner, created_at, decay_secs, lootable_by }` — spawned on death with inventory transfer
- `LootRule`: Public, GroupOnly, OwnerOnly, Faction

---

## Group & Party

### Structure

Groups are managed by a `GroupManager` resource. Each group has a leader,
members, loot mode, and formation.

- `Group { leader, members, loot_mode (LootMode), formation (Formation) }`
- `GroupManager { groups, invites }` — resource
- `GroupInvite { inviter, target, expires_at }`
- `GroupMember { group_id, role (GroupRole) }`
- `Following { target, autofollow }`
- `LootMode`: FreeForAll, RoundRobin(next_index), MasterLooter(Entity)
- `Formation`: Default, Line, Scattered (others defined in TOML)
- `GroupRole`: Leader, Member

### Commands

```
group invite/accept/decline/leave/kick/disband/loot/status/chat/formation
follow <player> / follow stop
```

Invites expire after 30s (one pending per player). Follow uses 1-tick delay,
pauses during combat, prevents chained teleport. XP bonus: +10% per member
(max +50%). Group chat prefixed `[Group]`.

### Formations

| Formation | Effect | Min size |
|---|---|---|
| `Line` | +1 AC front / −1 AC back | 2 |
| `Scattered` | −2 AC, +10% dodge | 2 |
| `Column` | +1 damage first hit | 3 |
| `Wedge` | +2 attack, −4 AC leader | 3 |
| `Shield Wall` | +2 AC, −2 attack (shield req) | 2 |

Applied by `FormationSystem` (Combat phase) as `ActiveEffect` components.

### Group Skills & Shared Credit

Skills with `targeting = "group"` affect all members in the caster's room.
Quest kills/gathers can be shared (per-quest `share_kills` / `share_gather`).
Disconnect: leader transfers to longest-standing member; 60s grace period for
auto-rejoin; `GroupCleanupSystem` (DirtyFlush) sweeps stale entries.

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

- `SkillTypeConfig`: Combat, Magic(MagicConfig), Tech(TechConfig), Psionics(PsionicsConfig), Craft(CraftConfig), Social, General
- `MagicConfig`: school, sub_school, casting_time_secs, concentration, components, material_component
- `TechConfig`: skill_prereqs, hardware_required
- `PsionicsConfig`: discipline, power_source, risk
- `CraftConfig`: station, materials, difficulty
- `SkillDef`: id, name, skill_type, level_requirement, cooldown_secs, targeting, cost, effect, script, allowed_classes, allowed_races, requires_skill, must_train, trainer_types, use_while_fighting, use_while_sitting
- `ResourceCost`: None, Stamina(u16), Mana(u16), Energy(u16), Psi(u16), Gold(u64), Xp(u64)
- `AllowedClassEntry`: class, spell_level
- `SkillPrereq`: id, level
- `Targeting`: Self, Single(range), Room, Area(radius)

### Mana & Resource Components

Characters have resource pools tracked in ECS components. Which pool(s) a
character has depends on class/race — a warrior has `Stamina`, a mage has
`Mana`, a psion has `Psi`. Resources are depleted on skill use and
regenerated each Regeneration pulse.

Resource pool components: `Stamina { current, max }`, `Mana { current, max }`, `Energy { current, max }`, `PsiPool { current, max }` — all u16.

**Regen:** `current += max / 20` per Regeneration pulse (5% per 6s = ~100% in 2min), applied to all resource pools present on the entity.

### Learned Skills

Characters track which skills they know and when they're on cooldown:

- `LearnedSkills { skills: HashMap<skill_id, SkillRank>, cooldowns: HashMap<skill_id, Instant> }`
- `SkillRank(u16)` — proficiency level (0 = untrained)

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

- `EffectTemplate`: Damage(dice_count, dice_sides, damage_type), Heal(dice_count, dice_sides), Buff(stat, amount, duration_secs), Debuff, Teleport(target_room), Script(script_id), Spawn(mob_id, count), Aura(aura_id, radius)
- `Stat`: Strength, Dexterity, Intelligence, Wisdom, Constitution, Charisma
- `ActiveEffect { effect, remaining_secs }` — applied effect with expiry timer

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

### Data Structures

- `RaceTemplate { id, name, description, attributes, size, speed, allowed_classes, languages, hometown, traits, racial_abilities, familiarity, alignment_tendencies }`
- `RaceTraitValue`: Bool(bool), Int(i32)
- `LanguageDef { id, name, script, speakers }`

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

- `ClassProgression`: base_attack_bonus (f32 × level, floored), fort_save, ref_save, will_save (SaveProgression)
- `SaveProgression`: Good (+2 + level × 0.5), Poor (level × 0.33)

Computed at level-up and stored on `CombatStats { base_attack_bonus, fort_save, ref_save, will_save }`.

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
stance deactivates the previous one). Tracked via `ActiveStance(Option<String>)`.

Activation: `stance <name>` command. Deactivation: `stance default`.
Stances cannot be changed while in combat unless the stance explicitly
allows it.

### Passives

Passives are always-on class bonuses. They are applied as components
on level-up (or on class grant for multi-classed characters) and
removed if the class is lost. Passives can reference scripts for
complex effects. Stored as `PassiveEffect { id, effect }`.

A `PassiveApplicationSystem` runs at login and level-up: it queries all
character class components, resolves the class template's `[passives]` list,
and ensures the matching `PassiveEffect` components exist on the entity.

### Data Structures

- `ClassTemplate`: id, name, description, attribute_mods, hit_die, base_attack_bonus, skill_ranks_per_level, fort/ref/will save, prestige(bool + gate), allowed_races, allowed_alignments, class/cross_class/exclusive_skills, auto_skills, auto_spells, stances, passives, multi_classing
- `SkillAccessEntry`: id, max_rank (formula string like "level+3")
- `AutoSkillEntry`: id, level
- `StanceDef`: id, name, ac_bonus, attack_penalty, damage_bonus, ac_penalty, min_level
- `PassiveDef`: id, name, description, effect, min_level
- `MultiClassingConfig`: favored

---

## Portal, Teleport & Transport Mechanics

Portal and teleport skills use the generic `MagicConfig` / effect system
and check room flags for permission. Room flags are defined in the Room
Components section. Portals create a `TempPortal` component on the room
that expires after a skill-defined duration; usable via `enter` command.
Teleport checks `ROOM_NO_TELEPORT_OUT` (source) and `ROOM_NO_TELEPORT_IN`
(destination), targets one-hop adjacent rooms, and checks the target's
`Teleportable` component. `Teleportable(false)` opts out.

### Skill Caps & Training

Every class template defines three skill access categories determining
max rank: `class_skills` (level+3), `cross_class_skills` ((level+3)/2),
`exclusive_skills` (per-class max). Skills unlisted in any category are
unavailable. Training cost from `mud.toml`:

```
total_gold = (base_cost + level * cost_per_level + current_rank * cost_per_rank) * cost_multiplier
```

Skills can require prerequisites (`requires_skill` DAG, validated for
circular deps at load). Trainer NPCs match via `trainer_types`
intersection — the `train` command shows a filtered menu.

### Stances & Passives

**Stances** are toggleable combat modes defined in class templates
(trade-off bonuses/penalties). One active at a time, applied by
`StanceSystem` (Combat phase). **Passives** are always-on class bonuses
applied by `PassiveApplicationSystem` on login/level-up (checks
`min_level`, idempotent, stacks across classes unless `stackable = false`).
Both defined in the Classes section.

---

## Prestige & Multi-Classing

### Multi-Classing

Characters track multiple classes via `MultiClassInfo { classes: Vec<ClassEntry> }`.
Each `ClassEntry` has `class_id`, `level`, `is_favored`. Total character level
is the sum of all class entry levels.

**XP penalty:** `(non_favored_classes - 1) × 20%` (capped at 80%, configurable
in `mud.toml`). Favored class (from class template `[multi_classing] favored = true`)
does not count toward penalty. One favored class at a time.

**Adding a class:** `@multi_class <class>` — checks race/alignment gates, no
duplicates, inserts as level 1. **Leveling:** player chooses which class to
advance each level-up; that class's hit die, skill points, and auto-learns apply.

**Alignment violations:** Warning on change, no new levels until restored.
Features remain active (script can strip them at admin discretion).

### Prestige Classes

Class templates with `prestige = true` + `[prestige_gate]` defining
requirements (base class, level, skills, race, alignment, quest, faction
standing). Granted via `@prestige <class>` or Rhai `grant_prestige()`.

```
@prestige <class> → validate all gates → add class entry, apply auto_skills/passives/stances
```

Multiple prestige classes allowed, levels count toward total, stacking with
base class grants. If requirements lost, keep existing levels but cannot gain more.

- `PrestigeGate`: requires_class, requires_skills (list of SkillPrereq), requires_race, requires_alignment, requires_quest, requires_faction, requires_level
    requires_class: Vec<String>,
    requires_level: u8,
    requires_skills: Vec<PrerequisiteSkill>,
    requires_race: Vec<String>,
    requires_alignment: Vec<String>,
    requires_quest: Vec<String>,
    requires_faction: Vec<PrerequisiteFaction>,
}
struct PrerequisiteSkill { id: String, rank: u8 }
struct PrerequisiteFaction { id: String, standing: i32 }
```

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

- `Experience(u64)` — cumulative XP. Formula: `for_level(level) = level³ × 100`. Method `to_next_level()` returns XP remaining.
- `LevelUpReward`: new_level, hp_gain, mana_gain, unlocked_skills, attribute_points

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

- `Durability { current, max, decay_rate }` — u16 current/max, f32 decay rate

Weapons lose durability on hit. Armor loses durability on being hit.
At `current == 0`, the item is **broken** (no stats) until repaired.
Repair via NPC blacksmith (`repair <item>`) or the `repair` skill.

### Extra Components

- `Weapon { damage (DamageDice), speed (f32), range (WeaponRange) }`
- `WeaponRange`: Melee, Ranged, Reach, Thrown
- `Armor { ac_bonus, slot, material, skill_penalty }`
- `Container { capacity_weight, capacity_items, lock_id, is_locked }`
- `Material`: Cloth, Leather, Metal, Mithril, Adamantium, Dragonhide, Wood

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

- `SetTracker { active_sets: HashMap<set_id, ActiveSet> }`
- `ActiveSet { template_id, counts (piece_type → count), equipped (template IDs), active_tiers }`

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

### Data Structures

- `ItemTemplate`: id, name, description, item_type, subtype, quality, level_requirement, weight, value, flags, attributes, class/race/alignment/skill gates, weapon, equipment, triggers, affixes, loot, set
- `SkillRequirement`: id, level
- `WeaponDef`: damage (DamageDice), speed, range
- `DamageDice`: count, sides, damage_type
- `EquipmentDef`: slot
- `TriggerDef`: event, chance, cast, target
- `AffixRef`: affix_type, element, stat, amount
- `LootParams`: min/max_quality, min/max_affixes, weight
- `SetMembership`: id, piece_type
- `AffixDef`: id, name, description, affix_type, element, amount, quality_min, slot, weight
- `SetDef`: id, name, bonuses (Vec<SetBonus>)
- `SetBonus`: min_pieces, conditions, effects
- `SetCondition`: piece_type, min
- `SetEffect`: effect_type, stat, amount, aura_id, radius

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

### Data Structures

- `MobTemplate`: id, name, description, level, attributes, health (HealthBounds), armor, damage (DamageDice), race, size, equipment, xp_value, loot, ai_mode, aggro_range, aggro_players, aggro_race, faction, faction_standing, trainer_types, languages, skills, scripts
- `HealthBounds { current, max }`
- `MobEquipment { template_id, slot }`
- `LootTable { entries: Vec<LootEntry> }`
- `LootEntry { item, count (CountRange), chance, loot_params }`
- `CountRange { min, max }`
- `AiMode`: Idle, Wander, Patrol, Stationary
- `MobSkill { id, level }`
- `ScriptHook { event, script }`

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

`Wallet { copper, banked_copper }` — both u64.

### NPC Shops

Shops are entities with a `Shop` component:

- `Shop { name, buy_rate, sell_rate, inventory (Vec<ShopItem>), currency, restock_secs }`
- `ShopItem { template_id, count, price_override, unlimited }`

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

- `ShopTemplate { id, name, npc, buy_rate, sell_rate, currency, restock_secs, inventory: Vec<ShopInventoryEntry> }`
- `ShopInventoryEntry { item, count, price_override, unlimited }`

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

### Data Structures

- `RecipeDef { id, name, station, skill (RecipeSkill), difficulty, materials, result (CraftResult), success_chance, quality_scaling, script }`
- `RecipeSkill { id, level }`
- `CraftMaterial { item, count }`
- `CraftResult { item, count, quality }`
- `QualityScaling { margin_per_point, max_quality }`

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

Stations are room flags (`room_flags = ["station:anvil"]`) or entities with `Station { station_type, quality_bonus }`.

### Known Recipes

Tracked via `LearnedRecipes { recipes: Vec<recipe_id> }`.

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

### Data Structures

- `QuestDef { id, name, description, level_requirement, repeatable, auto_complete, giver_npc, turn_in_npc, prerequisites (Vec<QuestPrerequisite>), objectives (Vec<QuestObjective>), rewards (Vec<QuestReward>), scripts }`
- `QuestPrerequisite`: Level(u8), Quest(String), Faction(id, standing), Skill(id, rank), Item(id, count)
- `QuestObjective`: Kill(mob, count, room_area), Gather(item, count), Deliver(item, target_npc), Explore(room), Talk(npc), Escort(npc, destination), Craft(item, count, station), Use(skill, count)
- `QuestReward`: Xp(u64), Gold(u64), Item(id, count), Faction(id, standing), Skill(id, rank), Recipe(String)

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

- `QuestLog { active: HashMap<quest_id, QuestProgress>, completed: Vec<quest_id> }`
- `QuestProgress { quest_id, objectives: Vec<ObjectiveState>, started_at }`
- `ObjectiveState { objective_index, current, completed }`

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

### Data Structures

- `FactionDef { id, name, description, starting_standing, min/max_standing, ranks, relationships, aggro }`
- `FactionAggro { threshold, members }`
- `FactionStanding { standings: HashMap<faction_id, i32> }`

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

- `Command { name, aliases, access (AccessLevel), handler: fn(&mut World, &mut Connection, args) -> CommandResult }`

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

Named communication streams, each with name, color, min level, min access,
and history. Built-in channels: `say` (room), `tell`/`reply` (player),
`whisper` (room-private), `shout` (zone), `yell` (area), `emote` (room),
`gossip`/`auction`/`ooc` (global), `gtell` (immortal), `admin` (admin).

```
tell/reply/whisper/emote/; /channels /channel <name> <on|off>
```

### Socials & Emotions

TOML-defined in `content/socials.toml` with three message forms (self, target, room).
Built-in: `smile`, `wave`, `nod`, `glare`, `poke`, `hug`, `frown`, `grin`, `wince`,
`cough`, `sigh`, `laugh`, `bow`, `curtsey`, `shrug`, `applaud`, `sniff`, `salute`, `shiver`.

### Resting States

`RestState`: Standing, Sitting, Resting, Sleeping, Unconscious, Dead
| State | Regen | Dodge | Input |
|---|---|---|---|
| Standing | 0% | 0% | Full |
| Sitting | 0% | -20% | Full (no combat) |
| Resting | +50% | -40% | Chat |
| Sleeping | +100% | — | Tell/say wakes |
| Unconscious | +50% | — | 0 HP, auto-wake |

Commands: `sit`, `rest`, `sleep`, `wake`, `stand`.

---

## Time & Weather

Game time is independent of real time (default 1:60 ratio — 1 min = 1 game hour).
Persisted in SQLite via `game_time` table; on startup, fast-forwards from stored
`raw_seconds`. Seasons (`Spring/Summer/Autumn/Winter`) affect daylight hours,
weather tables, temperature, visuals, and mob spawns.

Weather tracked per `weather_zone` (from area definition). Updated by
`WeatherSystem` on each Weather phase pulse.

| Condition | Effect |
|---|---|
| Rain/Storm | −2 fire damage, +2 lightning damage |
| Fog | −25% ranged visibility |
| Snow/Blizzard | −1 DEX |
| Strong wind | −2 ranged attacks |

Commands: `time` (show game time), `weather` (show zone weather).

---

## Telnet Protocol

IAC byte parser (`0xFF` escape) with state machine: `Data → IAC → Will/Wont/Do/Dont/Subneg`.
Each connection gets a `TelnetConnection` wrapping `TcpStream`, implementing the
`Connection` trait (transport-agnostic — `WsConnection` for WebSocket dispatches the same way).

### Feature Detection

`Feature`: Ansi, ExtendedColor, Naws, Mccp, Gmcp, Mxp, Mssp, Blink, Html, Utf8

Negotiation sequence: `WILL ECHO` + `DO NAWS` + `DO TERMINAL-TYPE` →
client replies → capability set built. 256-color/GMCP/MXP negotiated only
if terminal type supports it (MTTS detection).

### Keepalive

`IAC NOP` every 60s; 120s inactivity = disconnect. Detected by
`KeepaliveSystem` (DirtyFlush phase). Emits `PlayerDisconnected`.

### Connection Registry

`Arc<Mutex<HashMap<Entity, mpsc::UnboundedSender<Vec<u8>>>>>` — temporary
broadcast mechanism for room messages before event bus is fully wired.

---

## Text Formatting & Color

Color types: 16 ANSI + `Indexed(u8)` (256-color). `Modifier(u8)` bitmask
(BOLD|DIM|ITALIC|UNDERLINE|BLINK|REVERSE|HIDDEN|STRIKE). `RichText(Vec<Segment>)`
with `Segment { text, fg, bg, modifiers }`. `Color::Default` replaces `Option<Color>`.

Render respects client capability (16-color fallback for `Indexed`) and user
preference (`blink_enabled` per-account in `accounts.blink_enabled`).

Tag syntax for content files:
```
{red}text{/}  {brightblue}item{/}  {yellow bold}critical!{/}
{bg:color} sets background, {/modifier} clears one, {{ emits literal brace.
```

Parser at `core/src/format/tag.rs::parse_tags(&str) -> RichText`.

Conventions table — see inline in this section for color-by-context mapping
(room name=brightwhite, player=yellow, mob=red, say=default, etc.).

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

Components that mutate during gameplay mark the entity `Dirty` on write.
`DirtyFlushSystem` queries `(&Dirty, ..)`, serializes components to SQLite,
removes `Dirty`. Runs every 5s.

### Full Flush on Shutdown

1. Steal `Scheduler` (prevent new pulses), disconnect players, flush all dirty
2. `PRAGMA wal_checkpoint(TRUNCATE)`, close DB

### Schema Migrations

`data` crate manages version via `PRAGMA user_version` + `migrations` table.
Startup: iterate `current..SCHEMA_VERSION`, apply migration SQL in transaction.

### Entity Serialization

`WriteBatch { entity_id, entity_type, components: Vec<ComponentRow> }` mirrors
component types. SQL tables per component (e.g. `components_position`,
`components_health`, `components_json` catch-all). On startup, the `data` crate
rebuilds ECS: `SELECT id, type FROM entities` → create entities, then populate
components. Stale entities (no rows in any component table) are deleted.

### WAL Configuration

`PRAGMA journal_mode = WAL, foreign_keys = ON, busy_timeout = 5000, synchronous = NORMAL`.
Connection in `Arc<parking_lot::Mutex<Connection>>` shared across systems.

### Type-Safe Query Wrappers

`data/src/queries.rs` exposes typed functions: `get_account`, `create_account`,
`save_components`, `load_all_entities`, etc.

### Backup Strategy

Hot backup via SQLite online backup API. Scheduled hourly by `BackupSystem`.
Stored in `data/backups/`, retain 7 daily + 4 weekly (configurable).

---

## Content Loading & Hot-Reload

All game content defined in TOML under `content/` (configurable path). The
`content` crate scans the tree at startup, loads every `.toml`,
deserializes via serde, cross-references (room exits point to valid rooms,
mob/item refs exist, script paths are real, etc.), builds derived indices
(class→skills, race→classes, trainer→skills, set→items, etc.), and inserts
into `TemplateRegistry` (behind `Arc<RwLock<...>>`).

Hot-reload uses `notify` crate (platform-native file watcher). On change:
re-parse, validate, atomic-swap entry in registry, emit `ContentReloaded`
event. Startup blocks; hot-reload is non-blocking.

```
content/{areas, mobs, items, races, classes, skills, scripts, recipes,
         quests, factions, shops, help} + affixes.toml, sets.toml,
         languages.toml, socials.toml, treasure_classes.toml
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

### Data Structures

- `AreaTemplate { id, name, description, level_range (LevelRange), flags, weather_zone, reset_interval_secs, credits }`
- `LevelRange { min, max }`
- `RoomTemplate { id, area, name, description, exits (Vec<ExitDef>), portals (Vec<PortalDef>), flags, flags_to, heal_rate, mana_rate, teleport_dest, extra_descriptions, content }`
- `ExitDef { direction, target, flags, key_id, door_name }`
- `PortalDef { keyword, target, description, flags }`
- `ExtraDesc { keyword, text }`
- `RoomContent {}`
- `MobSpawnDef { mob, count, respawn_secs }`

---

## Startup & Shutdown Flow

### Startup Phases

```
CliParse → ConfigLoad → LoggingInit → ContentLoad → Validation →
DatabaseOpen → WorldCreate → StateSeed → SystemRegister →
ScriptingInit → EventBusInit → CommandTrie → ListenerBind →
BackgroundTasks → Ready
```

Wire diagram:

```
bin::main()
  ├── clap::parse()                        → CliArgs
  ├── mud_core::Config::from_file(args)    → Config resource
  ├── tracing_subscriber init
  ├── mud_content::Loader::load()          → TemplateRegistry
  ├── mud_content::Validator::validate()   → Vec<Diagnostic>
  ├── mud_data::Database::open() → Connection; migrate
  ├── mud_core::World::new() + insert all resources
  ├── mud_data::loader::load_world()       → load persistent entities
  ├── mud_core::systems::register_all()
  ├── mud_scripting::Engine::init()
  ├── mud_server::cmd::CommandTrie::register_all()
  ├── mud_server::Listener::bind()
  ├── tokio::spawn(flush_daemon)
  ├── tokio::spawn(hot_reload_watcher)
  ├── tokio::spawn(area_reset_timer)
  └── MainLoop::run().await
```

### Shutdown Sequence

| Step | Action | Timeout |
|---|---|---|
| 1 | Close listener | Immediate |
| 2 | Notify players | Immediate |
| 3 | Drain in-flight commands | 200ms |
| 4 | Flush all dirty entities | ∞ |
| 5 | WAL checkpoint (FULL) | 5s |
| 6 | Disconnect all players | Immediate |

Triggers: SIGTERM, SIGINT, `shutdown` command, fatal error.

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

- `Config { server, database, game, combat, training, multi_classing, item_sets, logging }` — implements `Resource`

Systems access config via `world.get_resource::<Config>()`.

---

## Error Handling & Logging

### Error Types

Each crate defines its own error enum with `Display + std::error::Error`
(using `thiserror`) and a crate-level `Result` alias:

- `core::Error`: EntityNotFound(Entity), ComponentMissing(name), InvalidDirection
- `server::Error`: Io(std::io), Telnet(String), CommandNotFound, InsufficientAccess
- `data::Error`: Db(rusqlite), EntityNotFound(i64), MigrationFailed
- `scripting::Error`: Rhai(rhai), BindingNotFound, ScriptNotFound

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

All destructive admin actions are logged with `tracing::warn!` (target: "audit",
action, executor, target, timestamp auto-added by tracing-subscriber).

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

### Data Structure

- `HelpEntry { id, aliases, title, text }`

---

## Admin & Immortal System

### Access Levels

Five tiers, stored at the account level, consulted per-command:

```
Player < Builder < Immortal < God < Admin
```

Account-level permission means any character on an admin account inherits that tier. Permission is cached on the `Connection` at login.

### Immortal Component

- `Immortal { incognito, holylight, build_mode }` — added to entity if account's access_level > Player. Per-session flags (default false on reconnect).

Added to a character entity at spawn if the account's `access_level > Player`. Flags are per-session (default false on reconnect).

### Connection Changes

- `Connection` trait additions: `access_level()`, `set_access_level()`, `has_immortal()`, `immortal_flag()` — gate commands via `conn.access_level() < cmd.access`

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

`ConnectionState`: Connected, Negotiating, Banner, Username, Password, CharacterSelect, CharacterCreate{Name,Race,Class,Attributes,Confirm}, Playing

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

- `state()`, `set_state()`, `create_buffer()` methods on Connection trait
- `CharacterCreateBuffer { name, race, class, attributes }` — holds in-progress creation state

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

- `ScriptEngine { engine, module_resolver }` — wraps rhai::Engine. Sandbox limits: 8 modules, 32 call levels, 50k operations, 10k string size, 100 dynamic arrays, 50 maps.

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

- `ScriptCtx { entity (EntityHandle), actor (Option<EntityHandle>), target (Option<EntityHandle>), world (WorldGuard) }` — context passed to each script invocation. EntityHandle is a thin wrapper over `hecs::Entity`, invalidated after script returns.

### Rhai Type Bindings — EntityHandle

Methods exposed to Rhai on the attached entity: `name()`, `id()`, `room()`, `has_flag(f)`, `has_comp(name)`, `health()`, `max_health()`, `level()`, `race()`, `classes()`, `is_player()`, `is_npc()`, `say(msg)`, `emote(msg)`, `echo(msg)`, `get_attr(name)`, `set_attr(k, v)`.

### Rhai Type Bindings — RoomHandle

`room().name()`, `.echo(msg)`, `.entities()`, `.players()`, `.exits()`, `.has_mob(id)`, `.has_item(id)`.

### Rhai Type Bindings — WorldHandle

`ctx.world.spawn_mob()`, `.spawn_item()`, `.remove_entity()`, `.echo_room()`, `.echo_zone()`, `.echo_world()`, `.grant_xp()`, `.grant_recipe()`, `.grant_quest()`, `.advance_quest()`, `.set_faction()`, `.mod_faction()`, `.has_entity()`.

### Event Registration

Scripts use the `on()` function to register handlers. The runtime
maps event names to `EventTag` values:

Scripts use the `on()` function to register handlers. Built-in events:
`death`, `enter`, `leave`, `hit`, `kill`, `say`, `use`, `damage`, `tick`,
`spawn`, `reset`, `quest_done`, `custom`.

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

A library module exports utility functions (e.g. `calculate_damage(level, base)`).
A per-mob script imports it and registers event handlers via `on("hit", |ctx| ...)`,
with access to `ctx.entity` methods and string interpolation.

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

- `rng(min, max)` → uniform Int
- `clamp(val, min, max)` → numeric clamp
- `roll(dice, sides)` → dice roll (e.g. 3d6)
- `capitalize(s)` → String
- `template(id, type)`, `item_template(id)`, `mob_template(id)`, `skill_template(id)` → lookup from TemplateRegistry
- `echo_room(room_id, msg)`, `echo_entity(entity, msg)` — messaging via ctx.world

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

- `SidebarSection { name, commands: Vec<SidebarCommand>, collapsed }`
- `SidebarCommand { label, command, takes_args, confirm, access, icon }`

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

- `OutputWindow { buffer: VecDeque<OutputLine>(5000 lines), scroll, ansi_parser, clickable_ranges }`
- `OutputLine { segments: Vec<StyledSegment>, timestamp }`
- `StyledSegment { text, style, clickable: Option<EntityRef> }`
- `EntityRef`: Player(name), Mob(template_id), Item(template_id)

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

Every scrollable pane manages its own scroll state: `ScrollState { offset, visible_lines, total_lines }` with methods `scroll_up()`, `scroll_down()`, `page_up()`, `page_down()`, `percent()`.

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

Help content is data-driven — a static struct so new keybindings can be added without layout code changes: `HelpSection { title, entries: Vec<HelpEntry> }`, `HelpEntry { key, description }`.

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

- `SessionState`: Disconnected, Connecting(host, port), Negotiating, LoggingIn(attempts), Playing
- `MudSession { state, transport, connection, output, input_history, last_target, known_players, gmcp_modules }`

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
