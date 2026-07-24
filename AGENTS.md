# OxideMUD Engine — Agent Guide

## Project state

This is a Rust project with working source code across workspace crates + TOML content on disk.

> **Single Source of Truth for Architecture & Implementation Specifications:**
> All architectural specifications, tech stack choices, system designs, crate dependency DAGs, concurrency models, state machine patterns, subsystem mechanics, feature phase roadmaps, and implementation details can be found **exclusively in [`ARCHITECTURE.md`](ARCHITECTURE.md)** — read it first each session.
>
> `AGENTS.md` is strictly reserved for agent invariants, developer workflow conventions, coding standards, commit rules, and security mindset guidelines. Do NOT add architectural or feature design specifications to `AGENTS.md`.

## Workspace layout

See [`ARCHITECTURE.md`](ARCHITECTURE.md#cargo-workspace) for full Cargo workspace crate descriptions and dependency DAG.

```
core/       — ECS components, systems, events, resources
server/     — Network layer (telnet, command dispatch, connection state, REST API)
data/       — Persistence (SQLite schema, type-safe queries)
scripting/  — Rhai engine setup + Rust↔Rhai bindings
tui/        — spade: builder TUI & MUD client (ratatui + crossterm)
mcp/        — MCP server for AI agent world-building
bin/        — Server binary entrypoint (main.rs)
```

`content/` is a **disk directory** for TOML templates and Rhai scripts — NOT a Rust crate. It sits alongside the crates at the workspace root and is loaded at runtime.

## Non-Negotiable Invariants

These rules are absolute. No PR, no matter how small, may violate them.

- **Core-to-Tooling Alignment:** Any change in `core` that alters template types, fields, file format conventions, or validation rules MUST update `tui/` (spade) and `mcp/` in the same session. The core crate defines the content representation; both consumers must stay consistent.
- **State Machine Event Dispatch:** All major subsystem transitions (Combat, AI, Login, Room) must follow explicit state machine patterns with defined states and valid transitions. Transitions emit a typed `GameEvent` over `tokio::sync::broadcast`. Transitions that fail validation are silently ignored. See ARCHITECTURE.md State Machine Pattern.
- **Zero Panics in Runtime:** Prohibit `.unwrap()` and `.expect()` in runtime and packet-handling logic. All fallible operations must use `Result<T, EngineError>` with `thiserror`. ECS queries that may fail use `World::query` with pattern matching, not `World::query_one` on potentially absent entities.
- **CPU Offloading & Lock Safety:** CPU-heavy tasks (e.g., Argon2 password hashing) MUST run via `tokio::task::spawn_blocking`, outside of database locks and `Mutex` guards. Never hold a lock while executing expensive computation.
- **Zero `unsafe` Code:** No `unsafe` blocks anywhere in the codebase. No exceptions. If you believe `unsafe` is necessary, open a discussion first.
- **Treat Every Networked Byte as Hostile:** This project is a public-internet game server. Every input byte must be validated, bounded, and never trusted. Pre-auth connections are especially dangerous — enforce line length limits, read timeouts, and strike tracking before any game logic processes input.
- **Documentation Synchronization:** Whenever implementing a new feature or modifying an existing system, you MUST update the corresponding human-facing documentation in `docs/` in the same session. Keeping documentation accurate and up-to-date as implementation evolves is mandatory.
- **Scoped & Deferred Verification:** NEVER run `cargo test --workspace`. Full workspace test suite verification is handled exclusively by `lefthook` pre-commit hooks. Verification of changes should be deferred to `lefthook` whenever possible. Avoid long-running test/build commands in favor of quick verification methods (e.g., `cargo check -p <crate>` or targeted per-test execution) or deferred verification. Only run explicit tests per-crate or per-test function when strictly necessary.

## Conventions

- Follow `ARCHITECTURE.md` for component/event/command designs. If it's undefined there, default to idiomatic Rust + the crate's stated responsibility.
- When planning or expanding a feature, update `ARCHITECTURE.md` to reflect the new design before or alongside implementation.
- Each workspace crate gets `Cargo.toml`, `src/lib.rs`, and a `src/` subdirectory tree matching the design doc.
- Defer verification to `lefthook` pre-commit hooks whenever possible. When verification during development is strictly necessary, use fast scoped checks (`cargo check -p <crate>`) or run specific test functions. NEVER run `cargo test --workspace`.
- Format with `cargo fmt`; lint with `cargo clippy`.
- Pre-commit hooks via `lefthook` enforce `cargo fmt`, `cargo clippy`, and full workspace tests on every commit.
- **No code examples** — avoid inline code blocks in docs unless the pattern is genuinely non-obvious or easily misused. Prefer concise prose over example code.
- **Compact sections** — keep sections tight. Omit exhaustive enumerations when the pattern is clear (e.g. "all LoginState variants" → just name the pattern). Summarize feature completeness tables rather than listing every row.
- **Update spade and MCP when core changes** — the core crate defines template types, fields, file format conventions, and validation rules. Both the TUI builder (`tui/`) and MCP server (`mcp/`) consume these directly. When adding, removing, or modifying anything in `core` that affects content representation (new template categories, new fields, changed serialization, new validation rules, etc.), update `tui/` and `mcp/` in the same session so they stay consistent. Rely on `lefthook` for full workspace verification upon commit, or fast per-crate checks (`cargo check -p <crate>`) during development.
- **Source of Truth vs. Human Reference Docs** — Rust source code and `content/` TOML templates are the sole single source of truth for all implementation details. Files in `docs/` (e.g. `docs/game_mechanics.md`, `docs/builder_manual.md`) are secondary human reference guides for MUD builders and admins, NOT implementation specifications.
- **Human-Facing Presentation Style** — write `docs/` content specifically for human MUD creators. Explain game mechanics, builder workflows, and content template formats conceptually and accessibly. Do NOT pollute human documentation in `docs/` with low-level Rust implementation details (e.g., Rust struct/enum names, memory channels, Tokio primitives, or internal trait hierarchies). Keep code-level technical details in Rust code comments and `ARCHITECTURE.md`.

## Modular development

- **Dependency DAG** — See [`ARCHITECTURE.md`](ARCHITECTURE.md#dependency-dag) for the crate DAG and layering rules. No circular dependencies.
- **Minimal `pub`** — prefer `pub(crate)` within a crate; re-export key types at `lib.rs`.
- **Feature gates** — Cargo features for optional pieces (e.g. `mccp`), not `cfg` checks.
- **Module tree** — mirror `ARCHITECTURE.md` `src/` layout exactly; one file per component/system type.
- **No `pub use` dep wildcards** — wrap external types (e.g. hecs `Entity`, `World`) in newtypes or facade functions.
- **Thin components, fat systems** — components are data-only newtype structs with no logic. All game logic lives in stateless systems that operate on `&mut World`. Encapsulate raw external types behind newtypes.
- **Commit generated code** — if codegen exists, commit the output and document the generation command.

## Coding standards

### Language best practices

- Follow idiomatic Rust: prefer `Result`/`Option` over panics, use `thiserror` for error types, use `?` operator for propagation.
- Adhere to `cargo clippy` — all lints enabled, zero warnings. The pre-commit hook enforces this.
- Zero `unsafe` — see Non-Negotiable Invariants. No `unsafe` blocks anywhere in the codebase.
- Prefer iterator chains over explicit loops where clarity isn't sacrificed.

### ECS architecture (hecs)

- **Thin components** — components are data-only structs with no logic. Use newtypes for type safety (e.g. `struct Health(i32)`).
- **Fat systems** — all logic lives in systems (functions operating on `World`). Systems are stateless; state lives in resources or components.
- **Queries** — prefer `World::query` over `World::query_one` unless fetching a singleton. Use `With`/`Without` filters for subset queries.
- **Events** — use the event bus for cross-system communication, not direct system coupling.

### Modular design

- **Single responsibility** — each module, type, and function does one thing. If a function needs "and" in its description, split it.
- **Minimal `pub`** — start everything `pub(crate)`; make `pub` only when another crate needs it. Re-export the public API at `lib.rs`.
- **Dependency injection** — pass dependencies as function parameters, not globals. Systems receive `&mut World` and `&Resources`.
- **No circular dependencies** — the crate DAG (`core → {data, scripting, mcp} → server → bin`, with `tui` branching from `core` + `scripting`) is enforced at build time.
- **Strongly prefer code reuse over reimplementation** — when an existing command, system, or function already provides the behavior you need, call it rather than duplicating its logic in a different layer. If the architecture lacks a clean connection point (callback, event, hook), introduce the needed abstraction instead of taking a shortcut.
- **Strongly prefer clean modular boundaries** — every module, crate, and system has a defined responsibility. A change that spans layers should add a proper bridge, not blur the lines. Best practices over laziness: if doing it right requires a refactor, do the refactor.

### Maintainability

- **Readability over cleverness** — prefer straightforward code over fancy one-liners. Name things for the reader, not the writer.
- **Doc comments** — all public items get doc comments (`///`). Internal items get them when the logic is non-obvious.
- **Tests** — every system and utility function should have unit tests. Use table-driven tests for multiple cases.
- **No magic numbers** — name constants with `const` or `enum`. Use `Default` impls for sensible defaults.
- **Consistent formatting** — `cargo fmt` is non-negotiable. The pre-commit hook enforces it.
- **Keep docs updated** — when implementing features that affect game mechanics, template fields, or builder-facing capabilities, update the relevant docs in `docs/` (`builder_manual.md`, `game_mechanics.md`, etc.) alongside the code.

## Commit style

Use conventional commits (`type(scope): message`).

### Type rules

| Type       | When to use                                                                                                                                                                                                                                               | When NOT to use                                                 |
| ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| `feat`     | A new feature, command, component, system, or user-facing capability                                                                                                                                                                                      | Refactoring existing code; config/setup changes                 |
| `fix`      | A bug fix — incorrect behavior, crash, typo in logic                                                                                                                                                                                                      | Linting, formatting, or style-only changes                      |
| `refactor` | Code change that preserves behavior: renaming, restructuring, extracting, inlining. No functional difference.                                                                                                                                             | Adding tests for existing code; changing behavior               |
| `perf`     | A change that improves speed, memory, or resource use                                                                                                                                                                                                     | Non-functional restructuring                                    |
| `test`     | Adding, updating, or fixing tests (unit, integration, doc-tests)                                                                                                                                                                                          | Changes to test infrastructure/CI tooling                       |
| `docs`     | Changes to documentation files only (`*.md`, doc comments, `ARCHITECTURE.md`, `AGENTS.md`)                                                                                                                                                                | Changes that touch source code alongside docs                   |
| `style`    | Formatting-only changes: whitespace, trailing commas, import ordering. No effect on compiler output.                                                                                                                                                      | Behavioral changes that happen to include reformatting          |
| `chore`    | Build config, tooling setup, dependency management, repo metadata, editor configs, git hooks, `justfile` updates, `rust-toolchain.toml`, `Cargo.toml` workspace changes, `.gitignore` — anything that touches infrastructure, not gameplay or engine code | Adding a feature or fixing a bug                                |
| `ci`       | CI/CD config changes (GitHub Actions, etc). If a change touches both CI and code behavior, use the behavior's type instead.                                                                                                                               | Same as `chore` — use `chore` for non-CI infra                  |
| `revert`   | Reverting a previous commit (generated by `git revert`). Leave scope matching the original commit.                                                                                                                                                        | Manual undo — use the appropriate type for what the change does |

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

`feat(persistence)!: migrate from rusqlite to sqlx`

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

## Security mindset

This project exposes a networked, multi-user game server to the public internet. Treat every input byte as hostile.

### Login path

- **Read timeouts** — pre-auth connections get a 60s per-line timeout. No infinite waits.
- **Max line length** — pre-auth lines capped at 256 bytes. Prevents buffer-bloating attacks.
- **Argon2 outside DB lock** — password hashing is CPU-intensive and must never block other connections. Fetch the hash from SQLite, release the lock, then verify.
- **No `Box::leak`** — never leak memory to obtain a `&'static` lifetime for user-controlled strings. Use `Arc<str>` instead.
- **Strike tracking** — failed login attempts increment a strike counter. At 3+ strikes per state (Username, Password), the connection is dropped.

### Memory safety

- **Zero `unsafe`** — this project uses no `unsafe` code. No exceptions.
- **No heap-leaking user data** — `String` → `Box::leak()` patterns are banned. User-controlled strings live in `Arc<str>` or owned `String`s.
- **Bounded allocations** — line buffers are reused per-loop (`.clear()`), not reallocated.

### Concurrency

- **Mutex scoping** — hold `Mutex` locks as briefly as possible. Narrow scopes with blocks (`{ let g = m.lock().await; ... }`).
- **Cancel safety** — `tokio::select!` branches should handle cancellation gracefully. Prefer `tokio::time::timeout` over raw `select!` for timeouts.
- **No `tokio::spawn` on untrusted input handlers** — connection handlers are top-level tasks; spawn no further tasks for user input.
- **CPU offloading** — run Argon2 and other CPU-heavy operations via `tokio::task::spawn_blocking`, never inside a `Mutex` guard or DB transaction.

### Database

- **Parameterised queries** — never interpolate user input into SQL strings. Use `rusqlite`'s `?` / `?N` / `:name` bindings exclusively.
- **WAL mode** — enables concurrent reads during writes. Schema migration awareness required.

### Network

- **Prefix-matched command dispatch** — no eval of user input as code. Commands are pre-registered `fn` pointers.
- **Telnet negotiation first** — NAWS, terminal type, and echo negotiation happens before any login prompt. Telnet IAC bytes are parsed by a dedicated reader, not raw `read_line`.
- **Transport-agnostic commands** — the `Connection` trait abstracts telnet, WebSocket, and future transports. No transport-specific assumptions in game logic.

### Examples

```
feat(telnet): implement NAWS window negotiation
fix(combat): clamp health to [0, max] on damage
refactor(persistence)!: switch from rusqlite to sqlx
chore(meta): pin Rust to 1.85.0 in rust-toolchain.toml
chore(hooks): add lefthook, cocogitto, and justfile tasks
docs: add commit type rules and decision flowchart
```
