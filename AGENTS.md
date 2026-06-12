# MUD Game Engine — Agent Guide

## Project state

This is a **pre-initialization** Rust project. The repo has exactly two files: `.git` (fresh) and `ARCHITECTURE.md` (~7000 lines). No `Cargo.toml`, no source code, no dependencies, no CI, no tests. **Start any work session by reading `ARCHITECTURE.md`** — it is the sole design spec and must be treated as ground truth.

## Planned stack

| Layer | Choice |
|---|---|
| Language | Rust (latest stable) |
| Async runtime | Tokio |
| ECS | `hecs` |
| Database | SQLite via `rusqlite` (WAL mode) |
| Scripting | Rhai (embedded) |
| Serialization | serde (TOML content files) |
| Networking | Tokio TCP (telnet) |

## Planned workspace layout

Eight crates under a root `Cargo.toml` workspace:

```
core/       — ECS components, systems, events, resources
server/     — Network layer (telnet, command dispatch, connection state)
data/       — Persistence (SQLite schema, type-safe queries)
scripting/  — Rhai engine setup + Rust↔Rhai bindings
content/    — Game data (TOML area/mob/item templates + Rhai scripts)
tui/        — spade: builder TUI & MUD client (ratatui + crossterm)
mcp/        — MCP server for AI agent world-building
bin/        — Server binary entrypoint (main.rs)
```

## Key architecture facts

- **Event-driven game loop** — no fixed tick. Uses `tokio::select!` over player input, scheduler pulses, and event bus. Systems register for intervals (combat 2s, regen 6s, weather 5m).
- **Driver/mudlib separation** — engine provides networking, ECS, persistence, scripting. Game content (combat, spells, quests) lives in data files and scripts, not engine code.
- **Two-tier persistence** — in-memory ECS world + SQLite on disk. Dirty tracking (`Dirty` marker component), background flush every 5s, full flush + WAL checkpoint on shutdown.
- **Command dispatch** — prefix-matched trie. Commands are `fn(&mut World, &mut Connection, &str)` with access levels.
- **Connection trait** — abstracts telnet, WebSocket, REST. Transport-agnostic command layer.
- **MCP server** — exposes world-editing tools to AI agents (Claude). Works offline
  (direct TOML/DB reads) or online (REST bridge to game server). Tools cover full CRUD
  for areas, rooms, mobs, items, quests, and content validation.

## Phases

| Phase | Focus |
|---|---|---|
| 0 | Cargo workspace, core types, TCP listener, basic ECS, raw I/O |
| 1 | Room graph, movement, `look`/`say`/directional commands, ANSI color |
| 2 | Account login, SQLite persistence, character creation, attributes/levels/XP |
| 3 | Combat, damage, equipment, NPC AI |
| 4 | Crafting, quests, spells, factions, PvP |
| 5 | OLC commands, Rhai scripting, hot-reload content, **spade offline builder**, **MCP server (offline)** |
| 6 | WebSocket, MCCP/GMCP/MXP/MSSP, REST API, **spade MUD client**, **MCP (online/prompts)**, profiling |

## Conventions

- Follow `ARCHITECTURE.md` for component/event/command designs. If it's undefined there, default to idiomatic Rust + the crate's stated responsibility.
- When planning or expanding a feature, update `ARCHITECTURE.md` to reflect the new design before or alongside implementation.
- Each workspace crate gets `Cargo.toml`, `src/lib.rs`, and a `src/` subdirectory tree matching the design doc.
- Test with `cargo test` (per-crate or workspace-wide). No test framework preference specified.
- Format with `cargo fmt`; lint with `cargo clippy`.
- No CI, no pre-commit hooks, no release workflow yet.

## Modular development

- **Dependency DAG** — `core` depends on nothing else. `server`, `data`, `scripting`, `tui`, `mcp` depend on `core` only. `bin` depends on `core`, `server`, `data`, `scripting`. No circular deps.
- **Minimal `pub`** — prefer `pub(crate)` within a crate; re-export key types at `lib.rs`.
- **Feature gates** — Cargo features for optional pieces (e.g. `mccp`), not `cfg` checks.
- **Module tree** — mirror `ARCHITECTURE.md` `src/` layout exactly; one file per component/system type.
- **No `pub use` dep wildcards** — wrap external types (e.g. hecs `Entity`, `World`) in newtypes or facade functions.
- **Commit generated code** — if codegen exists, commit the output and document the generation command.

## Commit style

Use conventional commits (`type(scope): message`).

**Allowed types:** `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `style`, `chore`, `ci`, `revert`

**Scope** refers to a feature area, not a crate:

- `movement` — room graph, direction commands, Position
- `combat` — attack/damage, NPC AI, Health
- `telnet` — protocol negotiation, raw I/O, Connection trait
- `persistence` — SQLite schema, dirty tracking, flush
- `scripting` — Rhai engine, bindings, triggers
- `olc` — builder commands (@dig, @link, @set, etc.)
- `account` — login, character creation, attributes/XP
- `items` — equipment, inventory, wear/wield
- `crafting`, `quests`, `spells` — advanced gameplay
- `content` — TOML templates, mob/room/item data
- `spade` — builder TUI & MUD client (ratatui, mouse/scroll, clickable names, ANSI parsing)
- `mcp` — MCP server for AI agent world-building
- `meta` — workspace config, CI, toolchain, docs

Breaking changes: append `!` after type/scope (`feat!(movement): ...`).

Body optional. Wrap at 72 chars. Imperative mood.

Examples:
```
feat(telnet): implement NAWS window negotiation
fix(combat): clamp health to [0, max] on damage
refactor!(persistence): switch from rusqlite to sqlx
chore(meta): pin Rust to 1.85.0 in rust-toolchain.toml
```
