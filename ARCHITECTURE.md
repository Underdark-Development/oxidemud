# Architecture — OxideMUD Engine

## Overview

OxideMUD is a modern DIKU-style MUD engine written in Rust. It is event-driven, ECS-based, terminal-first, and designed with extensible protocol support.

- **Tech Stack:** Rust, Tokio, hecs (ECS), rusqlite, Rhai (scripting)
- **Design Philosophy:** Strict driver/content separation. The engine provides core networking, ECS, persistence, and scripting runtime support. Game content (skills, spells, quests, mob AI, races, classes, items, deities) is specified in TOML templates and Rhai scripts rather than hardcoded in the engine.

> **Single Source of Truth & Documentation Guidelines:**
> The Rust source code across the workspace crates and TOML template definitions in `content/` are the **sole single source of truth** for exact struct fields, enum variants, command syntax, and formula implementations. Additional documentation files in `docs/` (e.g. [`docs/game_mechanics.md`](docs/game_mechanics.md) and [`docs/builder_manual.md`](docs/builder_manual.md)) are secondary human-written references for MUD builders and admins, and **are not a reliable source of truth** for implementation details as code evolves.
>
> - **Documentation Maintenance:** Developers and AI agents MUST update the corresponding files in `docs/` whenever implementing or modifying features to ensure human documentation remains accurate.
> - **Presentation Style:** Files in `docs/` must be written for human MUD creators—focusing conceptually on game rules, system behaviors, and TOML content formats. Avoid low-level Rust implementation details (e.g., specific Rust struct names, memory models, internal traits, or channel types) in `docs/`. Technical implementation details belong in Rust code comments and `ARCHITECTURE.md`.

---

## State Machine Pattern

Subsystems across the engine (such as `RoomState`, `CombatState`, NPC AI, and `LoginFlow`) are implemented as explicit state machines:

```
current_state → trigger_event → validate_transition(a, b) → emit StateChanged { entity, from, to }
```

- Each state machine exposes a `tick()` or transition function operating on state and context.
- Valid transitions emit typed events (e.g. `AiStateChanged`, `CombatStateChanged`) for subscribers. Invalid transitions fail silently or return error results.
- Implementations reside in [`core/src/systems/`](core/src/systems) and [`server/src/login/`](server/src/login).

---

## Cargo Workspace

The project is structured as a Rust Cargo workspace containing seven crates:

```
oxidemud/
├── Cargo.toml              # Workspace root
├── core/                   # ECS components, systems, events, resources, templates, formatting
├── server/                 # Network layer, telnet parser, command dispatch, login flow
├── data/                   # Persistence layer (SQLite schema, queries, WAL, migrations)
├── scripting/              # Rhai engine integration, sandboxing, and script bindings
├── bin/                    # Executable server entrypoint, CLI, system initialization
├── tui/                    # spade visual terminal world builder & client
└── mcp/                    # Model Context Protocol server bridge for AI agents
```

### Dependency DAG

- `core` has no workspace dependencies.
- `data` and `scripting` depend on `core`.
- `server` depends on `core` and `data`.
- `bin` depends on `core`, `server`, `data`, and `scripting`.
- `tui` (spade) depends on `core` and `scripting`.
- `mcp` depends on `core`.

Game content TOML templates and Rhai scripts live outside the crates under the configurable `content/` directory.

---

## Game Loop & Scheduler

Concurrent execution is split into two main Tokio loop layers:

1. **Connection Loop ([`server/src/server.rs`](server/src/server.rs)):** Listens for incoming TCP client connections via `tokio::select!` and spawns client handler tasks.
2. **Game Loop ([`server/src/game_loop.rs`](server/src/game_loop.rs)):** A background task running periodic system pulses via `tokio::select!` intervals:
   - **Player State (250ms):** Casting and stun timer decrements.
   - **Skill Decay (1s):** Cooldowns and temporary buff duration decrements.
   - **Combat Pulse (2s):** Hit/damage processing, stance updates, NPC AI ticks.
   - **Maintenance (5s):** Dirty stat flushing, position saving, group cleanup.
   - **Set Bonus (10s):** Re-evaluates equipped item set bonus thresholds.
   - **Weather (5min):** Rolls weather transitions per zone.
   - **Time Advance:** Advances in-game clock and emits period/season events.
   - **Big Tick (30–90s random):** HP/Mana/Stamina recovery and prompt broadcasts.

---

## Architecture & Subsystems Guidance

### ECS & Component Design

- **Spatial:** Room entities, exits (`Exit`, `RoomExits`), keyword portals (`PortalExit`, `RoomPortals`), flags, and directions. Located in [`core/src/components/spatial.rs`](core/src/components/spatial.rs).
- **Character:** Stats, Level/XP, Practice Points, Rest/Player State, Immortal flags, Deity, Gender, Appearance, Age, Recall room. Located in [`core/src/components/character.rs`](core/src/components/character.rs).
- **Resource Pools:** Health, Mana, Stamina, Energy, Psi pools managed in [`core/src/resources/`](core/src/resources).
- **Items & Equipment:** Slot allocations, weapon/armor attributes, durability, containers, item triggers, set trackers, active script effects. Located in [`core/src/components/item.rs`](core/src/components/item.rs).
- **In-Game Prompt:** Dynamic prompt string parsing (`Player.prompt`) supporting variables for stats, time, weather, combat state, and location.

### Command Dispatch Order

When a player submits input, [`server/src/cmd/`](server/src/cmd) evaluates:

1. **Contextual Entity Commands (`EntityCommands`):** Commands attached to rooms, items, or NPCs.
2. **Dynamic Script Skills (`DynamicSkillRegistry`):** Custom Rhai script commands and abilities.
3. **Static Engine Commands (`CommandDispatch`):** Built-in Rust handlers (movement, combat, info, builder, admin).

### Scripting & Engine Decoupling

Core Rust engine systems (`combat.rs`, `regen.rs`, etc.) do **not** contain hardcoded skill names, spell IDs, or content strings:

- **Condition-Driven Expiration (`EffectExpireCondition`):** Generic triggers clean up active script effects on state transitions (e.g. exiting combat, changing stances).
- **Implicit Context (`CURRENT_SCRIPT_CONTEXT`):** Thread-local guards pass execution context (`world`, `actor`, `target`, `room`) to Rhai scripts safely.
- **Bridges:** Trait abstractions (`ScriptingBridge`, `MessageOutputBridge`) decouple `core` from physical connection packet I/O and runtime compilation.

---

## Gameplay Systems Guidance

- **Combat System:** Handles hit rolls, armor class, damage types, resistances, weapon styles (two-handed speed, dual-wield penalties, ambidexterity), and target switching in `core/src/systems/combat.rs` (with secondary overview in [`docs/game_mechanics.md`](docs/game_mechanics.md)).
- **Corpse & Looting:** Spawns corpses on death with decay timers (30 min player, 5 min mob) and configurable ownership rules (`Public`, `GroupOnly`, `OwnerOnly`).
- **Group & Party System:** Managed by `GroupManager` resource. Supports leadership, member roles, loot distribution modes, formations (Line, Column, Wedge, Shield Wall), XP bonuses, and grace periods for disconnects.
- **Unified Skill Model:** All abilities (combat, magic, tech, psionics, crafting, social) are defined as `SkillDef` templates in `content/skills/`. Supports auto-learning, trainer requirements, resource costs, cooldowns, and partial name resolution.
- **Races & Classes:** Fully data-driven via TOML templates in `content/races/` and `content/classes/`. Controls attributes, size, languages, traits, BAB/save progression, auto-skills, stances, and passives.
- **Prestige & Multi-Classing:** Multi-classing tracks per-class levels and applies non-favored class XP penalties. Prestige classes enforce gate prerequisites (`[prestige_gate]`).
- **Experience & Progression:** Cubic XP curve (`level³ × 100`). Automatic level-up awards attribute gains, practice points, BAB/save recalculations, and passive updates. Player death results in XP loss (without de-leveling), corpse creation, and transition to a Ghost state.
- **Training & Practice:** Single `PracticePoints` pool used for both stat increases (`train`) and skill rank upgrades (`practice`) at trainer NPCs.
- **Economy & Shops:** Three-tier decimal currency (copper, silver, gold). NPC shops calculate dynamic buy/sell prices based on base template values and player reputation tiers.
- **Deity System:** Templates in `content/deities/*.toml`. Characters adopt deities subject to class policy (`any`, `none`, `required`, `subset`), granting prayer buffs subject to cooldowns.
- **Crafting & Recipes:** Material consumption, station verification (room flags/entities), difficulty checks, and quality margin scaling.
- **Quests & Factions:** Objective tracking (kill, gather, deliver, explore, escort), reward delivery, and faction standing propagation matrices.
- **Time & Weather Systems:** Independent in-game clock (configurable minutes per game hour, 8 named time periods, seasons). Data-driven composition weather model (base + modifier, area/room overrides, season weights) in `content/weather.toml`.
- **Telnet & Communication:** Telnet IAC state machine parser (ANSI, 256-color, NAWS, Keepalive). Channels (say, tell, shout, emote, ooc, gtell, etc.) and TOML socials (`content/socials.toml`).
- **Persistence & Content Loading:** Two-tier SQLite persistence in WAL mode with dirty flushing every 5s and automated hot backups. Content is loaded from TOML files into `TemplateRegistry` with hot-reloading via `notify`.
- **Zone & Area System:** Areas group rooms into directories (`content/areas/<area_id>/`). Handles doors/locks, area flags, room flags, and automated area resets.
- **spade (Builder TUI & Client):** Terminal tool in `tui/` providing offline world building (tree view, TOML form editor, validation, room grid) and MUD client connection features.
- **MCP Server:** Stdio Model Context Protocol server in `mcp/` exposing content CRUD tools, validation, search, and local gameplay simulators to AI agents.

---

## Unimplemented & Future Features

The following sections detail features that are currently planned, partially implemented, or scheduled for future phases. Because no source code exists yet for these features, these specifications remain the primary reference.

### 1. Item Durability & Repair (Planned)

- Weapons lose durability on hitting; armor loses durability on being hit.
- Items reaching `current == 0` durability are marked broken and yield no stat bonuses or armor protection.
- Items can be repaired at blacksmith NPCs or via repair skills/recipes.
- Future item features include container entity hierarchies and consumable commands (`quaff`, `recite`, `eat`, `drink`).

---

### 2. Protocol Expansion Path (Phase 6 Specs)

#### GMCP (Phase 6)

Structured JSON over telnet subnegotiation (IAC SB GMCP ... IAC SE).

- **Modules:** Core (hello/supports), Room (Info), Char (Info/Skills/Inventory/QuestList), Comm (Channel), MGK (Target/Spell).
- Opt-in per client during negotiation.

#### MXP (Phase 6)

Clickable links, entities, and status gauges formatted as XML tags (`<send>`, `<a>`, `<img>`, `<!ENTITY>`).

- Locked protocol requiring initial `<VERSION>` exchange before activation.

#### WebSocket Bridge (Phase 6)

- JSON MMCC frames over WebSocket: `{ type: "command"|"output", payload: { text, html } }`.
- Interfaces via `Connection` trait with server-side ANSI-to-HTML conversion.

#### REST API (Phase 6)

Lightweight HTTP management endpoints:

- `GET /api/who`, `GET /api/characters`, `GET /api/characters/:id`, `GET /api/characters/:id/inventory`.
- Authenticated via session tokens or API keys.

#### Protocol Feature Matrix

| Feature           | Phase | Requires      | Status      |
| ----------------- | ----- | ------------- | ----------- |
| ANSI 16-color     | 0     | —             | Implemented |
| NAWS              | 1     | Telnet        | Implemented |
| UTF-8             | 1     | Telnet        | Implemented |
| 256-color         | 2     | MTTS          | Implemented |
| GMCP (Room, Char) | 6     | Telnet        | Planned     |
| MCCP              | 6     | Telnet        | Planned     |
| MXP               | 6     | Telnet + lock | Planned     |
| WebSocket         | 6     | HTTP server   | Planned     |
| REST API          | 6     | HTTP server   | Planned     |
| MSSP              | 6     | Telnet        | Planned     |

---

### 3. MCP Server — Planned Imm Tools & Simulators (Phase 6)

#### Planned Imm Online Tools (Requires `--url` + `--key`)

Requires immortal+ access API key and REST connection to running server. Destructive actions require an explicit `confirm: true` parameter.

| Tool                | REST Endpoint                 | Description                                                           |
| ------------------- | ----------------------------- | --------------------------------------------------------------------- |
| `imm_set_stat`      | `POST /api/imm/set_stat`      | Modify attributes/HP/mana/stamina/level/XP (explicit params per stat) |
| `imm_load_mob`      | `POST /api/imm/load_mob`      | Spawn mob from template into room                                     |
| `imm_load_item`     | `POST /api/imm/load_item`     | Spawn item into room                                                  |
| `imm_gecho`         | `POST /api/imm/gecho`         | Global echo to all players                                            |
| `imm_advance`       | `POST /api/imm/advance`       | Level up a player                                                     |
| `imm_stat`          | `POST /api/imm/stat`          | Inspect ECS components on target                                      |
| `imm_heal`          | `POST /api/imm/heal`          | Full heal (HP/mana/stamina)                                           |
| `imm_damage`        | `POST /api/imm/damage`        | Deal damage to target                                                 |
| `imm_kill`          | `POST /api/imm/kill`          | Instantly kill target (requires confirmation)                         |
| `imm_revive`        | `POST /api/imm/revive`        | Revive dead/ghost player                                              |
| `imm_set_alignment` | `POST /api/imm/set_alignment` | Change alignment                                                      |
| `imm_set_faction`   | `POST /api/imm/set_faction`   | Adjust faction standing                                               |
| `imm_purge_room`    | `POST /api/imm/purge_room`    | Remove all NPCs/items from room (requires confirmation)               |
| `imm_reboot`        | `POST /api/imm/reboot`        | Graceful server reboot (requires confirmation)                        |

#### Planned Simulators (Phase 6)

| Tool                      | Core Hook                               | Description                                                             |
| ------------------------- | --------------------------------------- | ----------------------------------------------------------------------- |
| `simulate_regen`          | `systems::regen`                        | HP/mana/stamina regen per tick across rest states                       |
| `simulate_level_up`       | `award_xp` logic                        | Detailed level-up breakdown (HP die, skill points, mana/stamina recalc) |
| `simulate_faction_change` | `systems::faction::handle_faction_kill` | Faction standing changes from killing a mob                             |
| `simulate_quest_rewards`  | `QuestDef.rewards`                      | Quest reward breakdown                                                  |
| `simulate_practice`       | `cmd_train`/`cmd_practice`              | Skill training costs and practice point allocation                      |
| `simulate_xp_curve`       | `Experience::for_level`                 | XP thresholds across all levels                                         |

---

### 4. LPC Mudlib Importer Specification (Post-1.0, Low Priority)

Outline for an offline transpiler and runner architecture (`lpc-to-oxide`) to convert legacy LPC mudlibs (rooms, items, NPCs, base objects) into TOML data templates and Rhai scripts.

#### Architecture Specifications:

1. **Stateful Object State Mapping:**
   - Add an in-memory `DynamicState` component (`HashMap<String, Dynamic>`) to the ECS.
   - When modified by scripts, flag the entity as `Dirty` for flush during the 5-second `DirtyFlush` maintenance tick.
   - Expose `self.get_state("key")` and `self.set_state("key", value)` to Rhai.

2. **Dynamic Commands (`add_action`):**
   - Add an `ActiveCommands` component to characters.
   - When entering rooms or equipping items, register dynamic command verbs.
   - Match verbs in `server/src/cmd/` using prefix matching, with engine built-in commands taking precedence.

3. **Pre-Movement Interception (`on_before_exit`):**
   - Implement `on_before_exit` hook run by `systems::movement` on room entities and NPCs.
   - Returning `false` or calling `cancel_move` aborts room transitions.

4. **Code Reuse & Inheritance:**
   - Flatten inheritance hierarchies during transpilation or generate Rhai `import` statements (e.g. `import "std/room" as room;`).

5. **Delayed Events & Heartbeats (`call_out`):**
   - Implement `ScriptTimerManager` resource in the ECS world to track delayed/recurring tasks on the 250ms Player State tick.

6. **Transpiler Pipeline:**
   - **Phase 1 (Preprocessing):** C-preprocessor pass with `--include-dir` to resolve macros and `#ifdef` conditionals.
   - **Phase 2 (Static Extraction):** Parse static setter calls (e.g. `set_short`, `set_long`) to generate TOML templates.
   - **Phase 3 (Behavior Transpilation):** Convert LPC callbacks (`init`, `hit_callback`) and efuns (`write`, `say`, `destruct`) into Rhai script functions.

---

### 5. Development Roadmap Summary

- **Phase 0–3 (Core Engine & Content Baseline) ✓:** Cargo workspace, ECS, TCP/Telnet, Login/Char creation, combat, items, mobs, skills, races, classes, durability baseline.
- **Phase 4 (Advanced Gameplay):** Crafting, quests, factions, prestige, multi-classing, spells, economy, regeneration, time & weather. _(In progress / partially complete)_
- **Phase 5 (OLC & Tooling):** Online `@` commands, zone management, schema migrations, hot backup, Rhai hot-reload, spade offline builder, MCP server baseline. _(In progress / partially complete)_
- **Phase 6 (Protocol Expansion & spade MUD Client):** WebSocket, GMCP, MXP, REST API expansions, spade MUD client mode, advanced MCP imm tools & simulators. _(Planned)_
