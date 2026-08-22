# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## 0.7.0 - 2026-08-22
#### Features
- (**mcp**) switch online tools from REST to /ws/rpc JSON-RPC bridge - (cef8340) - Kevin Lanni
- (**server**) require confirm on content.delete and bound content size - (9dec106) - Kevin Lanni
- (**server**) implement JSON-RPC WS bridge at /ws/rpc with auth and RBAC - (7829776) - Kevin Lanni
- (**ws-rpc**) answer ping with pong and reject empty JSON-RPC responses - (56b0178) - Kevin Lanni
- (**ws-rpc**) add async RpcClient with request correlation and timeout - (9127245) - Kevin Lanni
- (**ws-rpc**) add oxide-ws-rpc crate with JSON-RPC 2.0 message types - (5e18cb1) - Kevin Lanni
#### Bug Fixes
- (**ci**) force remove existing containers with docker rm -f before docker compose up in restart workflow - (1391a83) - Kevin Lanni
- (**ci**) update restart workflow to start staged release whether server was running or not - (cb29519) - Kevin Lanni
- (**ci**) resolve running container ID dynamically via docker compose ps in restart workflow - (352b1a0) - Kevin Lanni
- (**ci**) use docker compose down to eliminate container name conflict during restart - (9d23c7a) - Kevin Lanni
- (**ci**) simplify restart workflow to build image and signal container with SIGTERM - (f854a42) - Kevin Lanni
- (**ci**) run docker exec as root user to write console commands to /proc/1/fd/0 and prefix broadcasts - (ac040fa) - Kevin Lanni
- (**ci**) cd to INSTALL_DIR before docker compose build in restart workflow - (0e02944) - Kevin Lanni
- (**mcp**) append /ws/rpc to legacy base URLs without a path - (904f835) - Kevin Lanni
- (**server**) harden /ws/rpc content write/delete against symlink escape, offload blocking FS, redact paths - (b9504ce) - Kevin Lanni
- (**ws-rpc**) send requests as text frames to match the server and reader - (326343a) - Kevin Lanni
- (**ws-rpc**) skip malformed inbound frames instead of tearing down the client - (64f46be) - Kevin Lanni
- (**ws-rpc**) remove pending-map entry on call timeout and send failure - (25bb4d8) - Kevin Lanni
#### Documentation
- update getting_started MCP endpoint to /ws/rpc - (2297097) - Kevin Lanni
- document /ws/rpc JSON-RPC bridge and oxide-ws-rpc crate - (5504616) - Kevin Lanni

- - -

## 0.6.1 - 2026-08-21
#### Bug Fixes
- (**install**) restore clean interactive vs non-interactive block scoping in install.sh - (36e4ef9) - Kevin Lanni

- - -

## 0.6.0 - 2026-08-21
#### Features
- (**install**) symlink binaries to system PATH with interactive confirmation and smart defaults - (db1a39c) - Kevin Lanni
#### Bug Fixes
- (**ci**) pass explicit toolchain version 1.98 to dtolnay/rust-toolchain action - (cb322c6) - Kevin Lanni
- (**ci**) honor rust-toolchain.toml pinned Rust version in workflows - (e6b78b2) - Kevin Lanni
- (**ci**) explicitly add musl target via rustup in release workflow - (0042876) - Kevin Lanni
- (**ci**) pass -y flag to install.sh in deploy workflow for automated staging - (ded326e) - Kevin Lanni
- (**ci**) add --no-cache to docker compose build in restart workflow to rebuild image with newly staged binary - (7899931) - Kevin Lanni
- (**ci**) pipe restart commands to container stdin via /proc/1/fd/0 - (9d9a16b) - Kevin Lanni
- (**ci**) allocate pseudo-TTY in restart workflow for docker attach console communication - (fb0cb75) - Kevin Lanni
#### Refactoring
- (**mcp**) modularize 3919-line server.rs monolith into 12 modules (#1) - (7e6ccd6) - Kevin Lanni

- - -

## 0.6.0 - 2026-08-21
#### Features
- (**install**) symlink binaries to system PATH with interactive confirmation and smart defaults - (db1a39c) - Kevin Lanni
#### Bug Fixes
- (**ci**) pass -y flag to install.sh in deploy workflow for automated staging - (ded326e) - Kevin Lanni
- (**ci**) add --no-cache to docker compose build in restart workflow to rebuild image with newly staged binary - (7899931) - Kevin Lanni
- (**ci**) pipe restart commands to container stdin via /proc/1/fd/0 - (9d9a16b) - Kevin Lanni
- (**ci**) allocate pseudo-TTY in restart workflow for docker attach console communication - (fb0cb75) - Kevin Lanni
#### Refactoring
- (**mcp**) modularize 3919-line server.rs monolith into 12 modules (#1) - (7e6ccd6) - Kevin Lanni

- - -

## 0.5.2 - 2026-08-21
#### Bug Fixes
- (**ci**) drop systemctl stop from deploy script - (2c586ef) - Kevin Lanni
- (**ci**) run docker deploy without sudo on VPS - (7db4432) - Kevin Lanni
- (**ci**) pass --repo to gh release download in deploy job - (b30341f) - Kevin Lanni
- (**config**) restrict = form to long options only - (fe8e317) - Kevin Lanni
- (**config**) support --flag=value, fix help/version disambiguation, drop dead Docker ENV - (f3a4405) - Kevin Lanni
- (**config**) honor [content].path in preflight and harden CLI parsing - (aa772eb) - Kevin Lanni
- (**config**) add --content-path, comprehensive config, and CLI help - (5d3e024) - Kevin Lanni
- (**install**) use --base-dir in Windows installer launch commands - (2664cd3) - Kevin Lanni
- (**tui**) resolve needless-late-init clippy lints in entity inspector - (71d3f79) - Kevin Lanni
#### Documentation
- (**server**) note banner/motd are fixed base-dir conventions - (67cf1c6) - Kevin Lanni
- (**server**) drop breaking-change note from CLI table - (3222f07) - Kevin Lanni
- (**server**) document CLI flags, content path, and config precedence - (dba22fc) - Kevin Lanni
- treat scripts as content substructure, not a server path - (eaea5d2) - Kevin Lanni
#### Refactoring
- (**config**) relocate server.toml to the base dir root - (a129275) - Kevin Lanni
- (**config**) unify all server paths under a single --base-dir anchor - (1e62fd5) - Kevin Lanni

- - -

## 0.5.1 - 2026-08-16
#### Bug Fixes
- (**ci**) dispatch deploy via workflow_dispatch - (1306a6f) - Kevin Lanni
#### Refactoring
- (**meta**) satisfy clippy for_kv_map in cooldown decay - (45effb9) - Kevin Lanni

- - -

## 0.5.0 - 2026-08-16
#### Features
- (**account**) add per-player alias command and case-insensitive dispatch - (a8b42e7) - Kevin Lanni
- (**auth**) scoped API key authentication for MCP and Spade online mode - (59254e9) - Kevin Lanni
- (**combat/skills**) Enforce active parrying stance check for parry and auto-deactivate stance when combat ends - (dad9b38) - Kevin Lanni
- (**commands**) decentralize command help with self-registering modules - (c758bfb) - Kevin Lanni
- (**communication**) add channels system with group chat, scope-based propagation, and channel shortcuts - (087953d) - Kevin Lanni
- (**content**) report content parse failures and add --validate-content preflight - (a7a920f) - Kevin Lanni
- (**economy**) add shop system with buy/sell/haggle and reputation pricing - (f2f26fa) - Kevin Lanni
- (**economy**) add banking system with banker NPCs and gold deposits - (2f672ba) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**items**) split item quality into rarity and craftsmanship quality - (98c0bff) - Kevin Lanni
- (**items**) add consumables, containers, durability, and target indexing - (1979324) - Kevin Lanni
- (**mcp**) implement remaining imm REST API handlers and MCP tools - (3c588bc) - Kevin Lanni
- (**meta**) make docker-compose paths portable and gate cloudflared behind tunnel profile - (7476999) - Kevin Lanni
- (**reports**) add player bug/idea/typo/complaint reporting and @report builder management - (cc122f4) - Kevin Lanni
- (**scripting**) Fully dynamic combat hit/damage hooks, script predicate evaluation, and builder error reporting - (969d22f) - Kevin Lanni
- (**scripting**) Complete zero-world Rhai API refactoring, EffectExpireCondition decoupling, and comprehensive documentation - (d569b17) - Kevin Lanni
- (**scripting**) Add echo_to and echo_to_except for remote room messaging - (f643c7d) - Kevin Lanni
- (**scripting**) Add echo_room_except to filter out actor/actee from room broadcasts - (0bacd4d) - Kevin Lanni
- (**server**) add websocket and tls support across workspace - (49a1152) - Kevin Lanni
- (**skill**) Scale parry deflect chance with practice level up to 60% max - (e0741f5) - Kevin Lanni
- (**socials**) add socials system with TOML-based definitions and variable interpolation - (70fa636) - Kevin Lanni
- (**spade**) add F6 live dashboard with server telemetry over WebSocket - (b154d53) - Kevin Lanni
- (**spade**) complete UI/UX roadmap with tree substring highlighting, deep auto-scroll, and notification history log - (d24b872) - Kevin Lanni
- (**spade**) isolate menu bar dropdown mouse events and bind Ctrl+Shift+P / Ctrl+P to Command Palette - (9a24622) - Kevin Lanni
- (**spade**) add cross-category search and entity duplication (Ctrl+D) - (71d59b9) - Kevin Lanni
- (**spade**) implement form editor and inspector improvements - (7e0bf5b) - Kevin Lanni
- (**spade**) enforce strict overlay input capture - (b0030ad) - Kevin Lanni
- (**spade**) improve entity validation, raw toml editor, and tui inspector UX - (b33a9f2) - Kevin Lanni
- (**weather**) implement Weather System Phase 4 (Gameplay Effects) - (c75d6ba) - Kevin Lanni
- (**weather**) implement Weather System Phase 3 (ECS Integration) - (cd527f6) - Kevin Lanni
- (**weather**) implement Weather System Core Phase 2 - (af67feb) - Kevin Lanni
- (**weather**) implement Weather & Time System Phase 1 (Time System) - (8f520aa) - Kevin Lanni
- (**weather**) implement Phase 0 — config & content types for weather/time system - (1a39962) - Kevin Lanni
- Add dynamic script skills, spells, entity commands, restriction parameters, active/permanent affects, and affects command - (9771a14) - Kevin Lanni
#### Bug Fixes
- (**ci**) install zig for cargo-zigbuild and harden bump guard - (aa392bb) - Kevin Lanni
- (**hooks**) make pre_bump.sh sed portable across GNU and BSD - (614a424) - Kevin Lanni
- (**scripting**) Support inline expressions and file paths in evaluate_script_predicate - (e131818) - Kevin Lanni
- (**scripting**) Register name method for Entity - (020e748) - Kevin Lanni
- (**scripting**) Register name getter for Entity to support dot notation in scripts - (b78ec93) - Kevin Lanni
- (**scripting**) Register attacker, target, and is_offhand field getters on HitContext - (0dd860f) - Kevin Lanni
- (**scripting**) Fix unit test imports and re-export transition_combat_state - (b908b23) - Kevin Lanni
- (**server**) Add missing closing brace in registry.rs - (60f1223) - Kevin Lanni
- (**skill**) Send parry success notifications to defender, attacker, and room occupants - (0e07553) - Kevin Lanni
- (**skill**) Update parry success message to 'You parry X's attack!' - (632a2f2) - Kevin Lanni
- (**spade**) fix search prompt cursor offset and slash double-capture in entities screen - (c566fe7) - Kevin Lanni
- (**spade**) deactivate search focus on mouse click and clean search input character handling - (eeb882d) - Kevin Lanni
- (**spade**) support wrap-around navigation with viewport auto-scrolling and mouse wheel scroll in CommandPalette - (4e6f7ac) - Kevin Lanni
- (**spade**) restore Ctrl+D as global quit shortcut and rebind entity duplication to Ctrl+Shift+D - (bfab92a) - Kevin Lanni
- (**spade**) enable CSI u keyboard enhancement and add Ctrl+R / Ctrl+Y / Ctrl+Shift+Z Redo handlers - (0bb1c74) - Kevin Lanni
- (**spade**) fix Ctrl+Shift+Z terminal keycode matching for Redo in raw editor - (7d0551a) - Kevin Lanni
- (**spade**) tighten Error Kind column width in validation panel - (f7c7586) - Kevin Lanni
- (**spade**) calculate dynamic column widths in validation panel with tightened field column - (0da7de0) - Kevin Lanni
- (**spade**) expand Error Kind column width in validation panel for sort indicator - (9799112) - Kevin Lanni
- (**tests**) Use target/ for temporary backup and script test directories - (d0ed0a1) - Kevin Lanni
#### Documentation
- (**agents**) defer architectural tasks and phase tracking to ARCHITECTURE.md - (9983eb2) - Kevin Lanni
- (**builder**) document content directory structure and sub-area hierarchy - (c8193b9) - Kevin Lanni
- (**content**) expand area directory structure in content README - (e4e88cc) - Kevin Lanni
- (**content**) update content README with full directory and server file guide - (2a1f7f7) - Kevin Lanni
- (**weather**) remove completed weather implementation tasks section - (d4aa3f7) - Kevin Lanni
- (**weather**) implement Weather System Phase 6 (Documentation) - (0efb2c6) - Kevin Lanni
- update server_admin.md for manual release and release-triggered deploy - (c8d46b1) - Kevin Lanni
- compact Spade builder architecture guidance in ARCHITECTURE.md - (8d6e62e) - Kevin Lanni
- compact architecture further - (fbd51e1) - Kevin Lanni
- update docs, and improve agents/architecture - (cfbb86d) - Kevin Lanni
- compact and update arch and agents details - (d486929) - Kevin Lanni
- Clarify DIKU MUD engine heritage and driver/content architecture - (fe965c0) - Kevin Lanni
- Update scripting_guide.md, builder_manual.md, and ARCHITECTURE.md with clean zero-world APIs and EffectExpireCondition decoupling - (47ba438) - Kevin Lanni
#### Refactoring
- (**commands**) colocate help with commands and modularize commands - (c4a85ef) - Kevin Lanni
- (**console**) rewrite help output to MUD-style formatting - (50719c4) - Kevin Lanni
- (**core**) replace glob re-exports with explicit item re-exports - (6badcd2) - Kevin Lanni
- (**core**) replace glob re-exports with explicit item re-exports - (64663a3) - Kevin Lanni
- (**core**) unify OnceLock RwLock singleton patterns across templates and scripting - (c004899) - Kevin Lanni
- (**core**) modularize templates god module - (de49630) - Kevin Lanni
- (**core**) remove dead event bus, replace with inline system outcomes - (57bfdbf) - Kevin Lanni
- (**core**) decouple script discovery and condition parsing from scripting crate - (1b1f8a8) - Kevin Lanni
- (**core/combat**) Decouple core system from skills via EffectExpireCondition enum - (ce7a075) - Kevin Lanni
- (**data**) bind component persistence save and load queries to unified column arrays - (2c63667) - Kevin Lanni
- (**examples**) Update example scripts to use built-in engine CommandRestrictions instead of hardcoded class checks - (d527c54) - Kevin Lanni
- (**scripting**) modularize scripting crate into domain submodules - (694751f) - Kevin Lanni
- (**scripting**) eliminate unsafe code and runtime panics - (1a616d6) - Kevin Lanni
- (**scripting**) Remove legacy world-parameter overloads from Rhai engine - (46e67b6) - Kevin Lanni
- (**scripting**) Infer world, actor, and room context implicitly across all Rhai script functions - (5d530c3) - Kevin Lanni
- (**server**) modularize login character creation god object - (0066000) - Kevin Lanni
- (**server**) extract combat dispatch and persistence from game_loop - (af9e42a) - Kevin Lanni
- (**spade**) replace numeric screen indices with typed ScreenId enum - (5e9a775) - Kevin Lanni
- (**weather**) make weather damage type modifiers dynamic - (1fb65cd) - Kevin Lanni
- consolidate parameter structs across data, mcp, and server - (58bba03) - Kevin Lanni
- rename category->topic - (2ec585d) - Kevin Lanni
#### Style
- apply cargo fmt and dprint formatting fixes - (1433abf) - Kevin Lanni

- - -

## 0.4.0 - 2026-07-22
#### Features
- (**console**) expand console commands, live session sync, and registry-driven help - (98d7013) - Kevin Lanni
- implement player account and character controls with dynamic width menu wrapping - (81a5189) - Kevin Lanni
#### Bug Fixes
- (**install**) copy Docker files to install directory for docker compose support - (422f7a2) - Kevin Lanni
#### Documentation
- format - (7416f62) - Kevin Lanni
- update getting started - (72d1c9e) - Kevin Lanni
- add future LPC Mudlib Importer architecture plan - (558e363) - Kevin Lanni
#### Refactoring
- client connection states to hierarchical state machine and fix login flow redirect prompt - (8aad08c) - Kevin Lanni

- - -

## 0.3.2 - 2026-07-20
#### Bug Fixes
- update scripts and docker files - (cd19f85) - Kevin Lanni

- - -

## 0.3.1 - 2026-07-17
#### Bug Fixes
- fix install script - (1244287) - Kevin Lanni

- - -

## 0.3.0 - 2026-07-17
#### Features
- (**core**) implement state transitions, update combat/AI systems, and refine documentation - (1587d21) - Kevin Lanni
- (**mcp**) complete template CRUD for all 15 content categories - (9a86b0d) - Kevin Lanni
- (**persistence**) implement online database backup system with daily/weekly retention - (b93c131) - Kevin Lanni
- (**server**) update logging pluralization, connection IDs, and past-tense logs - (c8cffe7) - Kevin Lanni
- (**telnet**) banner, MOTD, welcome stats, versioning, and hot-reloading - (14bacf6) - Kevin Lanni
- (**telnet**) implement NAWS window size negotiation and terminal type tracking - (fd25b62) - Kevin Lanni
#### Documentation
- (**meta**) fix code fences and table formatting in ARCHITECTURE.md - (8dfec82) - Kevin Lanni
- fix mcp example - (3a93f2e) - Kevin Lanni
- compactify ARCHITECTURE.md (1420 → 1063 lines) - (6cd6691) - Kevin Lanni
#### Style
- (**server**) capitalize additional console and save progress log messages - (5f1a91d) - Kevin Lanni
- (**server**) capitalize connection logging to sentence-case - (6fffb56) - Kevin Lanni
- format markdown documentation with dprint - (bb09d96) - Kevin Lanni

- - -

## 0.2.0 - 2026-07-15
#### Features
- (**meta**) make release compile for macOS, Linux, and Windows using Zig cross-compilation - (e90d125) - Kevin Lanni

- - -

## 0.1.0 - 2026-07-15
#### Features
- (**account**) complete character wizard with deity, appearance, age, and pray command - (ae868cc) - Kevin Lanni
- (**account**) implement level-up resource pool recalculation and PlayerLeveled event - (e3dd5b5) - Kevin Lanni
- (**account**) implement class progression combat stats and hit die scaling - (204581d) - Kevin Lanni
- (**account**) implement practice and train commands, enhance score - (f8fae1d) - Kevin Lanni
- (**account**) implement connection state machine and login flow - (106e314) - Kevin Lanni
- (**ai**) integrate custom scripting hook in NPC AI systems - (b283990) - Kevin Lanni
- (**ai**) implement patrol routes and wander bounds - (4819e87) - Kevin Lanni
- (**bin**) instantiate and register scripting and message bridges at startup - (5d2a883) - Kevin Lanni
- (**bin**) add width command, skill loading, wire formatted room descriptions - (273e9f5) - Kevin Lanni
- (**characters**) complete phase 2 — score, level-up, award, motd, and full ECS-DB sync - (5f88031) - Kevin Lanni
- (**characters**) implement character creation wizard, template loading, and entity persistence - (416b83d) - Kevin Lanni
- (**combat**) implement player state timer decay and no-resurrect enforcement - (3c9f5ec) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**combat**) harden gameplay mechanics, two-handed speed, slot restrictions, and loot rules - (b8fbf67) - Kevin Lanni
- (**combat**) implement scripting hooks for hit and damage calculations - (799026b) - Kevin Lanni
- (**combat**) implement die command and fix ghost combat target and recall issues - (e47d6f1) - Kevin Lanni
- (**combat**) implement player death, rest states, direct communications, and doors - (9708fcc) - Kevin Lanni
- (**combat**) auto-engage non-friendly NPCs on damage and log combat state changes - (24f49c4) - Kevin Lanni
- (**combat**) implement mechanical restoration effects for astra, kronos, vulgath, and karrgath deities - (6c8af35) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**combat**) replace CombatTarget with CombatState machine, implement flee - (a0ca5d0) - Kevin Lanni
- (**combat**) implement phase 3 — combat, equipment, NPC AI, items, stances - (77d5321) - Kevin Lanni
- (**commands**) dynamic help, player position persistence, fix query_one component checks - (380d93d) - Kevin Lanni
- (**commands**) wire trie-based object matching into target commands - (e0c5a82) - Kevin Lanni
- (**commands**) add movement commands with void check and broadcasts - (01487b0) - Kevin Lanni
- (**commands**) move handlers to commands.rs, update say and look - (5744645) - Kevin Lanni
- (**content**) update descriptions - (df5289b) - Kevin Lanni
- (**content**) add watchtower quest area, guard captain NPC, and quest items - (30de798) - Kevin Lanni
- (**content**) add astra, kronos, vulgath, and karrgath deity templates and validation tests - (a8d407a) - Kevin Lanni
- (**content**) spawn trainer mob and attach Trainer component - (208f9e2) - Kevin Lanni
- (**content**) update mob templates with friendly and short_desc fields - (a22eb75) - Kevin Lanni
- (**content**) centralize template loading in core - (16b8308) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**content**) restructure areas into per-room files with nested sub-area support - (036a1f5) - Kevin Lanni
- (**content**) add orc race, 8 skill templates, update mage/warrior classes - (3eaa538) - Kevin Lanni
- (**content**) add race/class template types, TOML loader, and LearnedSkills component - (be4e791) - Kevin Lanni
- (**content,look**) extract NPC descriptions into entities, add mob listing to look - (8a7696a) - Kevin Lanni
- (**core**) add PracticePoints component - (219a8e0) - Kevin Lanni
- (**core**) add ShortDesc/Friendly components and aggro_mobs AI flag - (2f415c4) - Kevin Lanni
- (**core**) add word-wrap support to RichText (render_wrapped) - (a1cc5b2) - Kevin Lanni
- (**core**) extend template system with skill resolution, SkillResolveError, WalletAmount - (291019f) - Kevin Lanni
- (**core**) add Alignment, Wallet components and screen_width to Player - (38452ad) - Kevin Lanni
- (**core**) add Name component - (fc14b25) - Kevin Lanni
- (**core**) add format module with ANSI color support - (e2f2c9b) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**crafting**) implement Phase 4 crafting system, recipe templates, and persistence - (d27ad41) - Kevin Lanni
- (**creation**) implement gender selection in character creation - (e8605e2) - Kevin Lanni
- (**data**) extend schema with skills, alignment, description, golds, screen_width - (b782434) - Kevin Lanni
- (**equipment**) implement skill gates with equip-time check and continuous pulse - (fc524df) - Kevin Lanni
- (**experience**) implement DIKU-style practice points system - (ea943a0) - Kevin Lanni
- (**format**) simplify mob and item look preview output - (bab1654) - Kevin Lanni
- (**format**) add 256-color fallback, blink gating, and spec tag syntax - (f3adb39) - Kevin Lanni
- (**items**) implement random loot drops, affix stat modifiers, and examine display - (5dd90df) - Kevin Lanni
- (**items**) wire item trigger processing for wear/remove/combat events - (9730212) - Kevin Lanni
- (**items**) wire item set membership spawning and set bonus change feedback - (d436a96) - Kevin Lanni
- (**login**) extract standalone LoginFlow, add admin console CLI - (22cb74d) - Kevin Lanni
- (**mcp**) support database character state overrides in simulation tools - (5172f79) - Kevin Lanni
- (**mcp**) implement new game mechanics simulation tools - (6106725) - Kevin Lanni
- (**mcp**) implement REST API, apikey management, and online/offline MCP character creation simulation - (a463c38) - Kevin Lanni
- (**mcp**) add offline simulation tools and fix quality/affix rolling - (94912c5) - Kevin Lanni
- (**mcp**) simulate common scenarios - (c35e42f) - Kevin Lanni
- (**mcp**) add MCP server crate with content CRUD tools - (1e60200) - Kevin Lanni
- (**meta**) add bash pre-bump hook to synchronize Cargo.toml version - (16b73a1) - Kevin Lanni
- (**meta**) update Ansible play description to reflect precompiled deployment - (cb82bfa) - Kevin Lanni
- (**meta**) package Ansible playbook and update playbook source paths - (d061e77) - Kevin Lanni
- (**meta**) add Docker choice prompt to Ansible deployment playbook - (92b90ae) - Kevin Lanni
- (**meta**) add pure Ansible playbook for VPS deployment - (d27d0a3) - Kevin Lanni
- (**meta**) add pre-upgrade DB backups and deployment host warnings - (abb468e) - Kevin Lanni
- (**meta**) mount log volume and document Docker console attachment - (54e05db) - Kevin Lanni
- (**meta**) add Docker Compose support and packaging - (fe7fd4d) - Kevin Lanni
- (**meta**) add release packaging, versioning, and deployment pipeline - (3e8c2bc) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**meta**) implement multiclassing, groups, factions, quests, crafting, and staff commands - (47296c6) - Kevin Lanni
- (**meta**) migrate to modern module layout, add engine systems and spawn-based login - (04150e0) - Kevin Lanni
- (**movement**) implement entities_in_room utility - (080d24f) - Kevin Lanni
- (**olc**) implement in-memory CRUD for area, mob, and item templates with template validation - (a80d8f3) - Kevin Lanni
- (**olc**) implement help categories refactor and staff commands - (b0374b2) - Kevin Lanni
- (**persistence**) migrate characters table to support recall_room_id persistence - (9cd4d06) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**persistence**) dedicated practice points table and migration - (4f9d8ec) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**persistence**) full player state persistence with dirty tracking - (06690e9) - Kevin Lanni
- (**prompt**) newline on broadcast - (ffc2711) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**scripting**) implement dynamic TOML parameter mapping for scripts across all templates - (ec21a35) - Kevin Lanni
- (**scripting**) register exit controls, rand helper, and write content scripts - (f9bb515) - Kevin Lanni
- (**scripting**) implement say hooks, follower movement, use command, and room script attribute - (dec3c97) - Kevin Lanni
- (**scripting**) implement ScriptingBridge and register Rhai wrappers in ScriptEngine - (ff5eb55) - Kevin Lanni
- (**scripting**) define scripting bridges and core scripting models - (7d59cdc) - Kevin Lanni
- (**server**) implement and export ServerMessageBridge for core messaging triggers - (fc38f05) - Kevin Lanni
- (**server**) combat messages, partial command matching, help details, fix quit and password echo - (1644a09) - Kevin Lanni
- (**server**) refactor login handlers into modules, expand character creation - (63cb631) - Kevin Lanni
- (**server**) add screen_width to Connection trait and TelnetConnection - (a23f282) - Kevin Lanni
- (**server**) show banner and login prompt immediately, add stats and command prompt - (d993d57) - Kevin Lanni
- (**server**) clean up player on disconnect - (679061b) - Kevin Lanni
- (**server**) add ConnectionRegistry for room broadcasts - (eb00685) - Kevin Lanni
- (**server**) add connection feature flags - (c79f241) - Kevin Lanni
- (**spade**) support editing fields and arrays in entity inspector screens - (afb35ff) - Kevin Lanni
- (**spade**) realign scripting tests and server logging - (e0a2c2d) - Kevin Lanni
- (**spade**) migrate screen switching to F1..F6 and script running to F9 - (0d46211) - Kevin Lanni
- (**spade**) rename room graph to room grid and make boxes taller - (d82c37c) - Kevin Lanni
- (**spade**) widen room graph boxes and migrate screen switching to alt hotkeys - (4d49a4a) - Kevin Lanni
- (**spade**) redesign room graph to neighbor room grid with double-click to dig - (6325da3) - Kevin Lanni
- (**spade**) implement command palette overlay - (ccb4879) - Kevin Lanni
- (**spade**) implement live dashboard screen - (0d19994) - Kevin Lanni
- (**spade**) implement script console and testing screen - (778607c) - Kevin Lanni
- (**spade**) implement file browser screen - (182b03c) - Kevin Lanni
- (**spade**) implement room graph screen and building tools - (550581b) - Kevin Lanni
- (**spade**) shared dropdown component, hover bleed-through fix, mode label style - (09620be) - Kevin Lanni
- (**spade**) dialog widget, flat actions section, unsaved tracking, hover cleanup - (26b5585) - Kevin Lanni
- (**spade**) menu bar, command sidebar, entities screen, entity inspector - (3097e28) - Kevin Lanni
- (**spade**) add search/filter to WorldTreeScreen - (2c736f2) - Kevin Lanni
- (**spade**) add create/delete entity workflows - (c5a6d5f) - Kevin Lanni
- (**spade**) persist inline edits to disk via save_to_disk - (4b2327f) - Kevin Lanni
- (**spade**) implement inline editing in EntityInspectorScreen - (83e6043) - Kevin Lanni
- (**spade**) wire entity inspector into world tree navigation - (ee0c88f) - Kevin Lanni
- (**spade**) scaffold TUI crate with screen framework and content tree - (30c8e82) - Kevin Lanni
- (**spawn**) spawn mobs from room content templates - (f76fbe4) - Kevin Lanni
- (**templates**) add shops, cross-area exit validation, and new mob fields - (8cba0b4) - Kevin Lanni
- (**trie**) generic prefix/word/index object matching - (a625f88) - Kevin Lanni
- (**tui**) add entity inspector screen - (03b68f3) - Kevin Lanni
- (**tui**) add mouse support, split layout, and keyboard shortcuts - (b2bff52) - Kevin Lanni
- (**tui**) add ValidationPanelScreen with template validation - (019c0b2) - Kevin Lanni
- channge town guard to idle - (fbf46e3) - Kevin Lanni
- adjust mobs - (8f3e0b6) - Kevin Lanni
- prompt system with mana/stamina persistence - (7153451) - Kevin Lanni
- add character schemas and persistence queries - (33011c5) - Kevin Lanni
- motd, content path config - (d1f58d4) - Kevin Lanni
- update initial scaffold - (aa33ecc) - Kevin Lanni
- phase 0 implementation - (ed4e2d7) - Kevin Lanni
#### Bug Fixes
- (**account**) resolve character creation loop bugs, UX issues, and safety fallbacks - (91d6d8b) - Kevin Lanni
- (**account**) check trainer existence via get() and add tests - (aa1cea1) - Kevin Lanni
- (**combat**) fix double role indicator in group status command - (f29fd78) - Kevin Lanni
- (**combat**) rebalance mob HP, XP, and merchant level - (888a9d6) - Kevin Lanni
- (**combat**) stop equipped items from dropping on NPC death - (efd2141) - Kevin Lanni
- (**combat**) resolve unused variable warning in AI test - (d951ed6) - Kevin Lanni
- (**combat**) broadcast player unconsciousness and deaths to room occupants - (1265be4) - Kevin Lanni
- (**combat**) player recall upon death, ghost restrictions, and death messages - (d57d314) - Kevin Lanni
- (**combat**) send incapacitated and mortally wounded messages to players - (76a9408) - Kevin Lanni
- (**combat**) fix npc death at zero health by correctly checking player component - (40dc6d4) - Kevin Lanni
- (**combat**) remove verbose NPC AI state transition logging - (fb34cb8) - Kevin Lanni
- (**content**) use cyan for exit directions - (61e5665) - Kevin Lanni
- (**format**) align color conventions with architecture spec - (c984ea6) - Kevin Lanni
- (**items**) fix set condition piece_type counting and add mapping warning - (8065d38) - Kevin Lanni
- (**login**) remove redundant character score on spawn, fix who alias - (1aeb1a9) - Kevin Lanni
- (**mcp**) merge duplicate impl blocks to register all tools under tool_router - (3d9d5d9) - Kevin Lanni
- (**mcp**) round-trip CRUD operations through struct types for valid TOML output - (a49c002) - Kevin Lanni
- (**mcp**) load areas from subdirectories and convert content to flat format - (9a7c345) - Kevin Lanni
- (**movement**) block player movement commands while in combat - (e378266) - Kevin Lanni
- (**movement**) block active commands when player is unconscious - (e842109) - Kevin Lanni
- (**persistence**) validate spawn_key on character load to heal legacy corrupted recall points - (9c3cbbf) - Kevin Lanni
- (**server**) tighten read timeout to pre-auth only via is_pre_auth() - (2d5292a) - Kevin Lanni
- (**telnet**) prepend newline to new output when prompt was last sent - (6aa2aa9) - Kevin Lanni
- (**telnet**) fix prompt formatting and delivery frequency - (527cc0b) - Kevin Lanni
- (**telnet**) send WONT ECHO initially, toggle WILL/WONT on password state - (203000a) - Kevin Lanni
#### Documentation
- (**agents**) update agent guidelines - (825a273) - Kevin Lanni
- (**spade**) update screen ordering and keybindings in ARCHITECTURE.md - (9627654) - Kevin Lanni
- update - (ef43e9d) - Kevin Lanni
- update docs - (b079e20) - Kevin Lanni
- update scripting architecture plan - (66f615b) - Kevin Lanni
- update documentation - (ad2817c) - Kevin Lanni
- add administrator and builder manuals - (d0689ba) - Kevin Lanni
- compact completed character and training phases in ARCHITECTURE.md - (59318da) - Kevin Lanni
- mark phase 4 tasks as complete in ARCHITECTURE.md - (35aecd6) - Kevin Lanni
- mark phase 2 and phase 3 tasks as complete in ARCHITECTURE.md - (71ce7be) - Kevin Lanni
- add note to keep spade/MCP in sync with core changes - (68af3b8) - Kevin Lanni
- update ARCHITECTURE.md with prompt, skills, and character creation details - (d74fbe5) - Kevin Lanni
- simplify arch plan - remove code - (2486276) - Kevin Lanni
- fix commit example - (6bb08b3) - Kevin Lanni
- reconcile format spec with implementation and tick Phase 1 checklist - (5a68db2) - Kevin Lanni
- add commit type rules and decision flowchart - (03e9620) - Kevin Lanni
- update architecture plan - (6078b4a) - Kevin Lanni
#### Refactoring
- (**ai**) formalize NPC AI state machine with enum transitions - (3ddaf01) - Kevin Lanni
- (**bin**) delegate mob spawning in init.rs to MobTemplate::spawn - (04842f3) - Kevin Lanni
- (**combat**) remove combat state transition logging - (50e21e5) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**content**) extract rooms into per-file structure with spawn data and example fields - (28f7ed5) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**format**) rename RichText/Segment and split module per spec - (49d8495) - Kevin Lanni
- (**look**) strip headers from mob/player listings - (98f8f7e) - Kevin Lanni
- (**meta**) rename compiled server binary from oxide-bin to oxide-server - (30361a1) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**meta**) rename engine crates and code imports to oxide - (6be84eb) - Kevin Lanni
- (**meta**) rename tinytin.tin to mud.tin and add reconnect alias - (0de9a47) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**olc**) replace temple name-substring check with allow_revive flag - (19d06f3) - Kevin Lanni
- <span style="background-color: #d73a49; color: white; padding: 2px 6px; border-radius: 3px; font-weight: bold; font-size: 0.85em;">BREAKING</span>(**persistence**) migrate character room persistence to RoomKey and fix starting item spawning - (04972ed) - Kevin Lanni
- (**persistence**) remove save_player_component debug logging - (1e39436) - Kevin Lanni
- (**spade**) modularize entity inspector and entities screen tree builder - (c3e6a01) - Kevin Lanni
- (**spade**) fix table constraint solver and standardize CLI with clap - (0882974) - Kevin Lanni
#### Style
- (**meta**) simplify deploy playbook play name - (8ff7130) - Kevin Lanni
- (**meta**) format all markdown files and integrate dprint - (487f36d) - Kevin Lanni

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).