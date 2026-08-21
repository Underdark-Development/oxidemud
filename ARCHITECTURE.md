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
2. **Player Aliases (`Aliases`):** Per-character command shortcuts stored as `HashMap<String, String>`, resolved before static dispatch. Case-insensitive; values may carry fixed arguments (e.g. `alias gob get orb`); args typed by the player are appended. Single-level resolution, no recursion. `alias`/`unalias` names are reserved and cannot be shadowed, and entity commands take precedence so room interactions always win. Persisted per character in `components_player_aliases`.
3. **Dynamic Script Skills (`DynamicSkillRegistry`):** Custom Rhai script commands and abilities.
4. **Static Engine Commands (`CommandDispatch`):** Built-in Rust handlers (movement, combat, info, builder, admin). Command lookup is case-insensitive.

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
- **Economy & Shops:** Four-tier decimal currency (copper, silver, gold, platinum). Merc-style NPC shops (`list`, `buy`, `sell`, `value` commands) with deterministic counter-offer haggling driven by charisma and faction reputation, per-shop `buy_types` gates on sellback, runtime stock with a restock cadence, and `price_mods` scaling prices by the player's faction rank. Banking: gold-only per-character bank accounts persisted in SQLite, gated by `banker = true` NPCs in the room (`balance`, `deposit`, `withdraw` commands).
- **Deity System:** Templates in `content/deities/*.toml`. Characters adopt deities subject to class policy (`any`, `none`, `required`, `subset`), granting prayer buffs subject to cooldowns.
- **Crafting & Recipes:** Material consumption, station verification (room flags/entities), difficulty checks, and quality margin scaling.
- **Quests & Factions:** Objective tracking (kill, gather, deliver, explore, escort), reward delivery, and faction standing propagation matrices.
- **Time & Weather Systems:** Independent in-game clock (configurable minutes per game hour, 8 named time periods, seasons). Data-driven composition weather model (base + modifier, area/room overrides, season weights) in `content/weather.toml`.
- **Telnet & Communication:** Telnet IAC state machine parser (ANSI, 256-color, NAWS, Keepalive). Channels (say, tell, shout, emote, ooc, gtell, etc.) and TOML socials (`content/socials.toml`).
- **Persistence & Content Loading:** Two-tier SQLite persistence in WAL mode with dirty flushing every 5s and automated hot backups. Content is loaded from TOML files into `TemplateRegistry` with hot-reloading via `notify`; files that fail to parse are reported (never silently skipped) via startup warnings and `oxide-server --validate-content` preflight checks.
- **Zone & Area System:** Areas group rooms into directories (`content/areas/<area_id>/`). Handles doors/locks, area flags, room flags, and automated area resets.
- **spade (Builder TUI & Client):** Terminal tool providing world building and game client capabilities across four distinct operational modes:
  - **Offline Mode:** Full-screen offline content creation (tree sidebar with substring search highlighting, structured form editor, raw TOML editor, 5-column sortable validation panel, room grid, cross-category search, and script console).
  - **Online Mode:** Full-screen online builder tool connected to a live server via WebSockets for live area editing and synchronization.
  - **Client Mode:** Full-screen standalone player client terminal interface (ANSI streaming, scrollback buffer, macros, command history).
  - **Split Mode:** Dual-pane hybrid workspace combining builder tools and live client testing. Enforces a **horizontal split (top/bottom)** where builder tools occupy the top pane and the MUD client stream occupies the bottom pane, avoiding vertical side-by-side splits that cause extreme column width compression in terminal displays.
  - Features strict overlay input event isolation, floating error tooltips, and persistent notification history.

- **MCP Server:** Stdio Model Context Protocol server in `mcp/` exposing content CRUD tools, validation, search, and local gameplay simulators to AI agents.

---

## Unimplemented & Future Features

The following sections detail features that are currently planned, partially implemented, or scheduled for future phases. Because no source code exists yet for these features, these specifications remain the primary reference.

### 0. Server Configuration

The server uses a **single configurable base directory** for all paths.

- **`--base-dir` / `-B`** (default: current working directory) anchors every server path. All other paths are fixed conventions under it: `content/`, `content/server.toml`, `content/motd.txt`, `content/banner.txt`, `content/scripts/`, `data/mud.db`, `logs/`. Relative paths never resolve against the process working directory when a base is given.
- Only `--host`, `--port`, `--version`, `--help`, `--base-dir`, and `--validate-content` remain as CLI flags. `server.toml` holds non-path runtime settings (`server_name`, `max_clients`, `default_prompt`, `logging`, `api`, `websocket`, `time`); path configuration deliberately lives outside it to avoid a circular config-with-path dependency.
- **Path-handling invariants to preserve:**
  - A single `--base-dir` anchor is the only path knob; do not reintroduce per-item path flags (`--config-path`, `--content-path`, `--motd-path`, `--banner-path`, `--db-path`) or a `[content].path` section. Keep content-relative conventions (scripts dir) hardcoded under content rather than configurable.
  - Config-loading order: `Config::parse()` (CLI) and `server::config::init()` (server.toml) must both run before any code reads configurable values.
  - No runtime config modification/saving exists (`CONFIG.set` only once at startup via `config::init`). Do not add config-saving unless explicitly requested; if added, keep it in the config module.

---

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

#### WebSocket & TLS Support (Implemented)

- **WebSocket Bridge:** Implemented in `server/src/api.rs` via `axum` `WebSocketUpgrade` and `WsConnection` in `server/src/connection.rs`.
- **Endpoints:**
  - `/ws/play` — Direct web player connection (translates input/output frames).
  - `/ws/spade` — Spade TUI builder stream & live session synchronization.
  - `/ws/mcp` — Real-time Model Context Protocol AI agent stream.
- **TLS & Security Policy:** Supports Automatic ACME (Let's Encrypt), custom TLS certificates, and in-memory self-signed dev certs (`rcgen`). Rejects unencrypted HTTP/WS on non-loopback bindings by default unless `allow_insecure_http = true` is explicitly configured for reverse proxy deployments.

#### REST API & Status (Phase 5/6)

Implemented via `axum` in `server/src/api.rs`. Provides HTTP endpoints for character querying, simulation, and immortal administration (authenticated via bearer tokens). Refer to `server/src/api.rs` for endpoint definitions.

#### Protocol Feature Status

- **Implemented:** Telnet (line mode), ANSI 16-color, NAWS, UTF-8, 256-color (MTTS), REST API, WebSockets (WS/WSS), TLS (ACME / dev certs / reverse proxy).
- **Planned (Phase 6 Specs Below):** GMCP, MXP, MCCP, MSSP.

---

### 3. MCP Server Guidance — Imm Tools & Simulators

The MCP crate (`mcp/`) bridges AI agent operations with OxideMUD:

- **Immortal Online Tools (`mcp/src/server.rs`):** Connects to a running game server via the REST API (`server/src/api.rs`). Enforces immortal+ role authentication via API tokens. Destructive operations require an explicit `confirm: true` parameter. Refer directly to `mcp/src/server.rs` for tool registrations.
- **Gameplay Simulators (`mcp/src/simulator.rs`):** Local/offline simulation functions that hook into core game modules (loot, combat, progression, gear loadouts, AI wander, shops, crafting, skill use, prayer, group formation, death penalty) to perform balance analysis. Refer directly to `mcp/src/simulator.rs` for function signatures and simulation models.

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

### 5. Content Format Versioning & Migration (Planned)

**Problem:** The TOML template format is implicitly versioned by the engine binary that parses it. Early versions will introduce breaking changes to template fields and `server.toml` keys; today such changes surface only as parse failures at load time, with no in-file signal of which engine version a content tree targets.

**Current state (implemented):**

- The loader never silently drops content: every TOML file that fails to parse is recorded with its path, category, and the serde error, logged as a `tracing::warn!` at startup, and reported by `oxide-server --validate-content` (see below). Content that parses but fails cross-reference validation is reported by the existing `TemplateRegistry::validate()` rules.
- `oxide-server --validate-content` is a preflight mode: it parses `server.toml`, loads the entire content tree, runs all validation rules, prints a per-file and per-rule report, and exits `0` (clean) or `1` (any error) without opening the database, binding ports, or starting the game loop. Deployment pipelines run it against the staged content directory before cutover.
- The SQLite schema is versioned via the `schema_version` table with sequential, guard-checked migrations in `data/`.

**Long-term design:**

1. **`format_version` field.** Every template file gains an optional top-level `format_version` integer (absent = legacy/current). The engine declares `CURRENT_FORMAT_VERSION` per template category. Loading a file whose `format_version` exceeds the engine's is a hard validation error; loading one below it triggers the migration path rather than a parse failure.
2. **Versioned template enums.** When a category's shape changes, the previous shape is retained as a versioned deserializable struct (e.g. `ItemTemplateV1` → `ItemTemplateV2`) and upgraded via a pure `From` impl at load time. The in-memory representation is always the latest version; older files parse into their pinned struct and migrate in memory.
3. **Content migration tool.** A `oxide-mcp migrate-content` subcommand (or standalone `just migrate-content`) rewrites on-disk files from older format versions to the current one, preserving comments/ordering where TOML round-trip allows. It operates on a copy or with a `--write` flag, prints a per-file diff summary, and refuses to downgrade. This is the sanctioned path for upgrading a live content tree after a breaking release; `content.default/` remains the reference for manual comparison.
4. **Compatibility window.** Each engine release supports loading the current format plus at least one prior format version per category, giving operators a one-release grace window to run the migration tool. Removal of an old format version is itself a breaking change and called out in the changelog.
5. **Config versioning.** `server.toml` adopts the same scheme with an optional `config_version`; unknown or removed keys produce startup warnings (never silent) and missing required keys produce preflight errors from `--validate-content`.

**Interaction with deployments:** The deploy flow stages new binaries + `content.default/` without touching the live `content/` dir. Operators run `oxide-server --validate-content` against the live content with the new binary before rebooting; if a breaking format change ships, the release notes must say so and the migration tool is run during the maintenance window before cutover.

---

### 6. Development Roadmap Summary

- **Phase 0–3 (Core Engine & Content Baseline) ✓:** Cargo workspace, ECS, TCP/Telnet, Login/Char creation, combat, items, mobs, skills, races, classes, durability baseline.
- **Phase 4 (Advanced Gameplay) ✓:** Crafting, quests, factions, prestige, multi-classing, spells, economy, regeneration, time & weather.
- **Phase 5 (OLC, Tooling & REST API) ✓:** Online `@` commands, zone management, schema migrations, hot backup, Rhai hot-reload, spade offline builder, REST API server baseline, MCP server baseline with 13 simulators & online IMM tools.
- **Phase 6 (Protocol Expansion & spade MUD Client):** WebSocket, GMCP, MXP, spade MUD client mode. _(Planned)_
