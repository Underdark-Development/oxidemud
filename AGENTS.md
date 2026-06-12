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

### Type rules

| Type | When to use | When NOT to use |
|---|---|---|
| `feat` | A new feature, command, component, system, or user-facing capability | Refactoring existing code; config/setup changes |
| `fix` | A bug fix — incorrect behavior, crash, typo in logic | Linting, formatting, or style-only changes |
| `refactor` | Code change that preserves behavior: renaming, restructuring, extracting, inlining. No functional difference. | Adding tests for existing code; changing behavior |
| `perf` | A change that improves speed, memory, or resource use | Non-functional restructuring |
| `test` | Adding, updating, or fixing tests (unit, integration, doc-tests) | Changes to test infrastructure/CI tooling |
| `docs` | Changes to documentation files only (`*.md`, doc comments, `ARCHITECTURE.md`, `AGENTS.md`) | Changes that touch source code alongside docs |
| `style` | Formatting-only changes: whitespace, trailing commas, import ordering. No effect on compiler output. | Behavioral changes that happen to include reformatting |
| `chore` | Build config, tooling setup, dependency management, repo metadata, editor configs, git hooks, `justfile` updates, `rust-toolchain.toml`, `Cargo.toml` workspace changes, `.gitignore` — anything that touches infrastructure, not gameplay or engine code | Adding a feature or fixing a bug |
| `ci` | CI/CD config changes (GitHub Actions, etc). If a change touches both CI and code behavior, use the behavior's type instead. | Same as `chore` — use `chore` for non-CI infra |
| `revert` | Reverting a previous commit (generated by `git revert`). Leave scope matching the original commit. | Manual undo — use the appropriate type for what the change does |

### Decision flowchart

1. Does this change alter compiled binary behavior?
   - **No** → is it documentation? → `docs`
   - **No** → is it formatting/style? → `style`
   - **No** → is it infra/tooling/config? → `chore` or `ci`
   - **Yes** → continue to step 2
2. Does it fix incorrect existing behavior? → `fix`
3. Does it add new capability? → `feat`
4. Does it preserve behavior but improve structure? → `refactor`
5. Does it improve speed with no other change? → `perf`
6. Does it only touch tests? → `test`

### Breaking changes

Append `!` after type/scope when the commit introduces a SemVer-breaking change:
- API signature changes (public function args/return types, component field changes)
- Behavior changes that break existing consumers
- Database schema migrations that drop/rename columns
- Rhai binding changes that require script updates

`feat!(persistence): migrate from rusqlite to sqlx`

### Scope

**Scope** refers to a feature area, not a crate. One scope per commit. If a change spans multiple feature areas, pick the dominant one. If truly cross-cutting (e.g. workspace config), use `meta`.

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
- `hooks` — git hooks, lefthook, cocogitto, just tasks for conventional commits
- `meta` — workspace config, toolchain, CI, docs, anything not covered above

### Body

Optional. Wrap at 72 chars. Imperative mood.

### Examples

```
feat(telnet): implement NAWS window negotiation
fix(combat): clamp health to [0, max] on damage
refactor!(persistence): switch from rusqlite to sqlx
chore(meta): pin Rust to 1.85.0 in rust-toolchain.toml
chore(hooks): add lefthook, cocogitto, and justfile tasks
docs: add commit type rules and decision flowchart
```
