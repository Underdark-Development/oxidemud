# Architecture — OxideMUD Engine

## Overview

OxideMUD is a modern DIKU-style MUD engine written in Rust. Event-driven, ECS-based, terminal-first with extensible protocol support.

**Stack:** Rust + Tokio + hecs (ECS) + rusqlite + Rhai (scripting)

**Design Philosophy:** Decoupled driver/content separation. The engine provides networking, ECS, persistence, and the scripting runtime — all DIKU game content (skills, spells, quests, mob AI) lives in TOML templates and Rhai scripts rather than hardcoded engine files.

---

## State Machine Pattern

Numerous engine subsystems are formalized as explicit state machines with defined states, valid transitions, and events emitted on transition. The pattern is:

```
current_state → trigger_event → validate_transition(a, b) → emit StateChanged { entity, from, to }
```

Each state machine has a `tick()` function that takes current state + context and returns next state. Transitions emit a typed event (e.g. `AiStateChanged`, `CombatStateChanged`) that other systems can subscribe to. Transitions that fail validation are silently ignored.

---

## Cargo Workspace

Seven crates under root workspace:

```
oxidemud/
├── Cargo.toml              # workspace root (resolver = "2")
├── core/                   # ECS components, systems, events, resources
│   ├── components/         # hecs Component types
│   ├── systems/            # Game systems (movement, combat, regen, ai)
│   ├── resources/          # Singleton resources + resource pools (Stamina, Mana)
│   ├── format/             # Color, RichText, tag parser
│   ├── templates/          # TOML deserialization + TemplateRegistry
│   ├── dice.rs             # DiceRoll XdY+Z parser/roller
│   └── lib.rs
├── server/                 # Network layer + command dispatch
│   ├── telnet/             # IAC parser, NAWS, terminal type
│   ├── cmd/                # Linear Vec<Command> dispatch (trie resolved)
│   ├── login/              # LoginFlow state machine (state.rs, handlers.rs, prompt.rs)
│   └── lib.rs
├── data/                   # Persistence layer (SQLite schema, queries, migrations)
├── scripting/              # Rhai engine setup + bindings + sandbox
├── bin/                    # Game server binary (main.rs, commands.rs, init.rs)
├── tui/                    # Spade visual terminal world builder
└── mcp/                    # Model Context Protocol server bridge
```

**Dependency DAG:** `core` depends on nothing. `server` depends on core + data. `data` depends on core. `scripting` depends on core. `bin` depends on core + server + data + scripting. `tui` depends on core + scripting. `mcp` depends on core.

Content templates (TOML) live at a configurable `content/` path on disk, not in a crate.

---

## Game Loop & Scheduler

The server architecture splits concurrent execution into two main loop layers using Tokio tasks:

1. **Connection Loop (`server.rs`):** A main listener loop that listens for incoming connection events via `tokio::select!`. On accept, it spawns a separate worker task to handle that client connection's inputs.
2. **Game loop (`game_loop.rs`):** A dedicated background system runner spawned on startup. It uses asynchronous tokio intervals and sleep timers to execute combat, AI, regeneration, and persistence pulses.

### Game Loop Ticks

The background loop fires independent system intervals within a central `tokio::select!` block:

| Tick         | Interval        | Phase/Function      | Description                                            |
| ------------ | --------------- | ------------------- | ------------------------------------------------------ |
| Player State | 250ms           | `player_state_tick` | Decrements player casting and stun timers              |
| Skill Decay  | 1s              | `skill_decay_tick`  | Decrements cooldowns and temporary buff durations      |
| Combat Pulse | 2s              | `combat_tick`       | Runs combat rounds, stance systems, and AI ticks       |
| Maintenance  | 5s              | `maintenance_tick`  | Flushes dirty stats, saves positions, cleans groups    |
| Set Bonus    | 10s             | `set_bonus_tick`    | Re-evaluates equipment set bonus thresholds            |
| Weather      | 5min            | `weather_tick`      | Rolls weather transitions per zone, broadcasts changes |
| Time Advance | configurable    | `time_tick`         | Advances in-game clock, emits period/season events     |
| Big Tick     | 30–90s (random) | `big_tick`          | Restores HP/MP/SP, broadcasts prompts to players       |

---

## Systems Architecture

Game logic is organized into isolated, concurrent modules executed inside the background game loop or triggered by player events.

### Built-in Pulse Systems

- **Combat System:** Checks hits, rolls damage, handles deaths, and awards XP.
- **Stance System:** Applies dynamic attribute modifiers based on active fighting stances.
- **AI System:** Moves NPCs through their AI state machine (Idle, Wander, Patrol, Combat, Flee).
- **Formation System:** Evaluates group spacing to apply shield wall or column combat bonuses.
- **Regeneration System:** Restores HP, Mana, and Stamina on the big tick based on the player's RestState.
- **Effect Expiry System:** Cleans up active temporary buffs and debuffs when their timer ends.
- **Corpse Decay System:** Decrements corpse duration timers and spills items to rooms when decayed.
- **Skill Gate System:** Continually re-evaluates gear skill requirements to auto-remove items.
- **Group Cleanup System:** Automatically removes offline or disconnected players from groups.
- **Database Backup System:** Spawns a background thread to run hot backups of the SQLite database.
- **Time System:** Advances in-game clock, emits period/season change events, tracks daylight.
- **Weather System:** Rolls weather transitions per zone on a 5-minute tick, applies gameplay effects (damage modifiers, ranged penalties, attribute changes), broadcasts severe weather to players.

### Scripting & Core System Decoupling

OxideMUD enforces strict architectural decoupling between core Rust engine systems (`combat.rs`, `regen.rs`, etc.) and game content (skills, spells, item affects):

1. **No Core Hardcoding**: Core systems **never** contain hardcoded skill names, spell IDs, or specific item strings.
2. **Data-Driven Effect Expiry (`EffectExpireCondition`)**: Active script effects register their own expiration criteria (`Timer`, `ExitCombat`, `ChangeStance`, `Custom`). State transitions in core systems (such as `transition_combat_state` moving an entity to `CombatState::NotInCombat`) trigger generic condition-driven expiration routines (`expire_effects_by_condition(world, entity, EffectExpireCondition::ExitCombat)`).
3. **Implicit Execution Context (`CURRENT_SCRIPT_CONTEXT`)**: Rhai script invocations bind context (`world`, `actor`, `self`, `target`, `room`) via RAII thread-local guards, providing a clean zero-parameter script API without leaking engine memory pointers.

---

## System Outcomes & Dispatches

System execution follows a tick outcome return-value pattern rather than an asynchronous event bus:

- **Combat System**: Returns `Vec<CombatOutcome>` to the tick loop for message generation, target switching, and corpse spawning.
- **Quest System**: Returns `Vec<String>` output messages directly to the caller/connection registry.
- **Faction System**: Returns `Vec<String>` standing adjustment feedback messages.
- **Player State Machine**: Returns `Result<PlayerState, ...>` and marks mutated entities with `Dirty` components.
- **AI System**: Ticks NPC state machines inline during system pulses and updates component state directly in `World`.

---

## ECS Component Design

### Spatial

- `Position { room: Entity }` — references a room entity
- `Room { name, description }` — room metadata
- `Exit { direction, dest, flags, key_id: Option<String> }` — direction-based exits with optional key template ID
- `RoomExits(Vec<Exit>)` — one per room
- `PortalExit { keyword, dest, description, flags }` — keyword-based portal exits
- `RoomPortals(Vec<PortalExit>)` — one per room
- `PortalFlags` bitmask: `PORTAL_HIDDEN`
- `RoomFlags` bitmask: `PORTAL_IN`, `PORTAL_OUT`, `NO_TELEPORT_IN`, `NO_TELEPORT_OUT`
- `VoidRoom` — marker: inescapable room, blocks all movement/recall/teleport
- `Teleportable(bool)` — targetable by player teleport
- `Direction`: North, South, East, West, Up, Down, NE, NW, SE, SW

### Character

- `Player { account_id, prompt, no_resurrect: bool }` — player entity, configurable prompt template, resurrection toggle flag
- `Npc { template_id }` — NPC entity
- `Attributes { str, dex, int, wis, con, cha }` — 6 core stats (u8)
- `Health { current, max }` — HP (i32)
- `Level(u8)` / `Experience(u64)` — level and XP
- `PracticePoints(u32)` — unified practice pool for training stats and skills
- `Immortal { incognito, holylight, build_mode }` — immortal status flags
- `Teleportable(bool)` — teleport target opt-out
- `RestState` — Standing, Sitting, Resting, Sleeping, Unconscious, Dead
- `PlayerState` — wraps RestState + Stunned { remaining_ms }, Casting { .. }
- `Gender { gender: String, pronouns: (String, String, String) }` — gender identity + subject/object/possessive pronouns
- `Appearance { height: u8, weight: u16, build: String, hair_color: String, hair_style: String, eye_color: String, skin_tone: String }` — structured physical description, validated against race bounds
- `Age(u16)` — character age, initial value from race template default
- `Deity(Option<String>)` — chosen deity id (none if no deity)
- `LastMessenger(Entity)` — transient component tracking last private sender
- `RecallRoom(Entity)` — component tracking character's current respawn/recall location

### In-Game Prompt

Configurable template in `Player.prompt`. Variables: `%h/%H` (HP), `%m/%M` (Mana), `%v/%V` (Stamina), `%l` (Level), `%x/%X` (XP/XP-to-next), `%n` (Name), `%g` (Gold), `%a` (Alignment), `%r` (Room name), `%e` (Exits), `%R` (Rest state), `%C` (Combat state), `%t` (Time period), `%w` (Weather description), `%c` (Newline), `%%` (literal). Rendered after every command output and on room entry, death, level-up, and combat changes. Customized via `prompt <template>`.

### Combat

- `Damage(i32)` + `DamageType`: Slash, Pierce, Bludgeon, Fire, Cold, Lightning, Acid, Poison, Magic, True

### Resource Pools (in `core/src/resources/`)

- `Stamina { current, max }`, `Mana { current, max }`, `Energy { current, max }`, `PsiPool { current, max }` — all u16
- Characters have pools depending on class/race (warrior=stamina, mage=mana, psion=psi)

### Items & Equipment

- `Item { template_id, flags }`, `Inventory(Vec<Entity>)`, `EquipmentSlot`: Head, Neck, Torso, Arms, Hands, Finger, Legs, Feet, Weapon, Shield
- `Weapon { damage (DamageDice), speed, range }`, `Armor { ac_bonus, slot, material, skill_penalty }`
- `Container { capacity_weight, capacity_items, lock_id, is_locked }`
- `Durability { current, max, decay_rate }` — break/repair system

### Character & Progression

- `Name(String)`, `Description(String)`, `Alignment(String)`, `Wallet(u64)`
- `CombatStats { base_attack_bonus, fort_save, ref_save, will_save }`
- `ActiveStance(Option<String>)` — name of active stance
- `PassiveEffect { id, effect }`, `LearnedSkills { skills, cooldowns }`
- `SkillRank(u16)`, `MultiClassInfo { classes: Vec<ClassEntry> }`
- `FactionStanding { standings: HashMap<String, i32> }`
- `QuestLog { active, completed }`, `QuestProgress { quest_id, objectives, started_at }`, `ObjectiveState { index, current, completed }`
- `LearnedRecipes { recipes: Vec<String> }`

### Item Progression

- `SetTracker { active_sets }` — map of active item set bonuses
- `ItemTriggers { on_hit, on_wear, on_remove, on_use }` — trigger skill executions per event
- `TriggerEffect { chance, skill_id, target }`, `TriggerTarget`: Self, Attacker, Room, Random
- `EntityCommands { commands: Vec<EntityCommandDef> }` — contextual item/room/mob commands with parameter restrictions and bestowal messages
- `ActiveScriptEffects { effects: Vec<ActiveScriptEffect> }` — temporary script buffs/debuffs/auras, short desc overrides, and TTL decay
- `PermanentItemAffects { affects: Vec<PermanentAffectDef> }` — permanent passive affects bestowed by equipped items or set thresholds

### Command Resolution Order

When a player submits a command, the server resolves it in the following order:

1. **Contextual Entity Commands (`EntityCommands`):** Checks actor's current room, entities/mobs in room, and inventory/equipped items. Evaluates `CommandRestrictions` (level, class, race, deity, equipped status, script predicates).
2. **Dynamic Script Skills (`DynamicSkillRegistry`):** Checks globally registered direct script commands (e.g. `parry`).
3. **Static Server Commands (`CommandDispatch`):** Evaluates built-in Rust command handlers (e.g. `look`, `score`, `cast`).
   - Note: For `cast <spell>`, `cmd_cast` checks `DynamicSkillRegistry` for dynamic script spells before checking static template spells.

### Flexible / OLC

- `Attributes(HashMap<String, String>)` — KV store for builder data

### World State (Time & Weather)

- `GameTime { hour, minute, day, season, year }` — singleton component on a world-time entity; tracks in-game clock
- `Season` enum: Spring, Summer, Autumn, Winter
- `TimePeriod` enum: Midnight, Dawn, Morning, Noon, Afternoon, Dusk, Evening, Night (maps to hour ranges)
- `WeatherState { base: Option<String>, modifier: Option<String> }` — per-room weather; `None` = Clear
- `WeatherCondition { id, name, description, severity, condition_type, effects }` — loaded from `content/weather.toml`
- `WeatherSeverity` enum: Minor (on-request display), Severe (auto-broadcast on change)
- `ConditionType` enum: Base (primary weather), Modifier (secondary overlay like Wind/Fog)

### Persistence

- `Dirty` — marker needs DB write, `DbId(i64)` — maps entity to SQLite row

---

## State Machine: RoomState

```
RoomState: Normal, UnderConstruction, Locked, EventActive { event_id }, Destroyed
```

## State Machine: CombatState

```
CombatState:
NotInCombat → { attacked } → Engaged
Engaged → { flee } → Fleeing
Engaged → { target_dead / target_unconscious } → NotInCombat / TargetSwitch
Fleeing → { success } → NotInCombat
Fleeing → { failed } → Engaged
```

`CombatSystem` dispatches on `CombatState` each pulse. `CombatStateChanged { entity, from, to }` emitted on every transition. When an entity falls unconscious or dies, any active combatants targeting them in the same room switch targets immediately to the next conscious entity currently targeting them.

## State Machine: NPC AI

| From \ To | Idle  | Wander       | Patrol       | Aggro       | Combat   | Flee   | Return      |
| --------- | ----- | ------------ | ------------ | ----------- | -------- | ------ | ----------- |
| Idle      | —     | timer        | timer        | aggro_check | —        | —      | —           |
| Wander    | timer | —            | timer        | aggro_check | —        | —      | —           |
| Patrol    | timer | timer        | —            | aggro_check | —        | —      | —           |
| Aggro     | —     | —            | —            | —           | in_range | —      | —           |
| Combat    | —     | —            | —            | —           | —        | hp<25% | target_dead |
| Flee      | —     | —            | —            | —           | cornered | —      | dist>safe   |
| Return    | —     | reached_home | reached_home | —           | —        | —      | —           |

`AISystem` ticks each NPC on Combat phase. Emits `AiStateChanged { entity, from, to }` per transition. Configuration (such as `ai_mode`, `patrol_route`, and wander settings) is loaded from the mob template. Aggro configurations are stored on the `Npc` component.

---

## Combat System

The core combat formulas, including the attack flow, to-hit checks, defense/Armor Class, damage calculation, damage types, resistances, and weapon styles are documented in [game_mechanics.md](file:///Users/therealklanni/Projects/mud/docs/game_mechanics.md).

### Weapon Styles

The following weapon style mechanics are implemented:

- **Two-handed weapon speed**: Two-handed weapons have a 1.2x speed modifier (exposed via `effective_speed()`). Wielding one automatically unequips the shield/off-hand slot, and equipping to the shield slot is blocked.
- **Ambidexterity skill**: The ambidexterity skill halves the dual-wield hit penalties (halving the primary hand -2 and off-hand -4 penalties to -1 and -2, respectively).

---

## Corpse & Loot

For detailed loot rules, decay timers, and looting commands, see [game_mechanics.md](file:///Users/therealklanni/Projects/mud/docs/game_mechanics.md).

### Components

- `Corpse { owner: Option<Entity>, created_at: Instant, decay_secs: u64, lootable_by: LootRule }` — attached to the transient corpse entity spawned in the room on death.
- `LootRule` enum: `Public`, `GroupOnly`, `OwnerOnly`, `Faction`.

---

## Group & Party

Groups managed by `GroupManager` resource. Each group has a leader, members, loot mode, and formation.

- `Group { leader, members, loot_mode, formation }` — resource entry
- `GroupMember { group_id, role }`, `Following { target, autofollow }`
- `LootMode`: FreeForAll, RoundRobin, MasterLooter; `Formation`: Default, Line, Scattered + others
- Invites expire 30s. XP bonus: +10%/member (max +50%). Group chat prefixed `[Group]`.

### Formations

| Formation     | Effect                        | Min |
| ------------- | ----------------------------- | --- |
| `Line`        | +1 AC front / −1 AC back      | 2   |
| `Scattered`   | −2 AC, +10% dodge             | 2   |
| `Column`      | +1 damage first hit           | 3   |
| `Wedge`       | +2 attack, −4 AC leader       | 3   |
| `Shield Wall` | +2 AC, −2 attack (shield req) | 2   |

Applied by `FormationSystem` as `ActiveEffect` components. Disconnect: leader transfers to longest-standing member; 60s grace; GroupCleanupSystem sweeps stale.

---

## Skill System

### Unified Skill Model

All abilities — combat, spells, racial powers, crafting, psionics — are a single `SkillDef` discriminated by `skill_type`. Everything lives in `content/skills/` subdirectories, one registry `TemplateRegistry.skills`, one command (`use`). `cast` is syntactic sugar gated on `skill_type == Magic`.

- `SkillDef`: id, name, skill_type, level_requirement, cooldown_secs, targeting, cost, effect, script, allowed_classes, allowed_races, requires_skill, must_train, trainer_types, use_while_fighting, use_while_sitting
- `SkillTypeConfig`: Combat, Magic(MagicConfig), Tech(TechConfig), Psionics(PsionicsConfig), Craft(CraftConfig), Social, General
- `ResourceCost`: None, Stamina(u16), Mana(u16), Energy(u16), Psi(u16), Gold(u64), Xp(u64)
- `Targeting`: Self, Single(range), Room, Area(radius)

### Resource Cost & Regen

Skills consume from appropriate pool. `RegenSystem`: `current += max / 20` per Regeneration pulse (5% per 6s = ~100% in 2min).

### Learned Skills

`LearnedSkills { skills: HashMap<skill_id, SkillRank>, cooldowns }`. Auto-learn on level-up from class `auto_skills`. Skill training via `practice <skill>` at a trainer NPC, costing 1 practice point per rank.

### Partial Name Resolution

All skill name inputs (character creation skill selection, `train` command,
future `use`/`cast`) are resolved against both `SkillDef.id` and
`SkillDef.name` via `TemplateRegistry::resolve_skill(input, pool)`.

Resolution priority:

1. **Exact match** on id or name — immediate return
2. **Unique prefix match** on id — return the matching skill
3. **Unique prefix match** on name — return the matching skill
4. **Multiple matches** — produce a disambiguation prompt
   (`"Which skill did you mean? skill_a (Name A), skill_b (Name B)"`)
5. **No match** — error with `SkillResolveError::NotFound`

The optional `pool` parameter restricts search to a subset of skills (used
during character creation where only the class's `skill_pool` is valid).
When no pool is provided, the full `TemplateRegistry.skills` is searched.

Resolution is case-insensitive. If the template registry is unavailable
(e.g. during early startup), callers fall back to exact match on raw input.

### Effect System

`EffectTemplate`: Damage(dice), Heal(dice), Buff(stat, amount, duration), Debuff, Teleport(room), Script(id), Spawn(mob_id, count), Aura(aura_id, radius). Runtime: `ActiveEffect { effect, remaining_secs }` — ticked by `EffectExpirySystem`.

### Type-Specific Behavior

| Type     | Resource     | Extra checks                             |
| -------- | ------------ | ---------------------------------------- |
| combat   | Stamina      | Must be fighting or `use_while_fighting` |
| magic    | Mana         | Concentration check, component check     |
| tech     | Energy       | Hardware focus in inventory/equipped     |
| psionics | Psi          | Risk roll (backlash chance)              |
| craft    | Stamina      | Station in room, materials consumed      |
| social   | None         | —                                        |
| general  | None/Stamina | —                                        |

### Skill Caps & Training

Class templates define three categories: `class_skills` (full SkillCap), `cross_class_skills` (half SkillCap), `exclusive_skills` (per-class). Skills require prerequisites (DAG, validated at load). Training uses the unified practice system — see [Training & Practice System](#training--practice-system).

### Stances & Passives

**Stances:** Toggleable combat modes defined in class templates (trade-off bonuses/penalties). One active at a time (`ActiveStance`). Applied by `StanceSystem`. **Passives:** Always-on class bonuses applied by `PassiveApplicationSystem` on login/level-up (idempotent, stacks unless `stackable = false`).

---

## Races

Races are template definitions, not enums. All behavior in `content/races/*.toml`. Removing a file removes the race without recompiling.

**RaceTemplate:** id, name, description, attributes, size, speed, allowed_classes, languages, hometown, traits, racial_abilities, familiarity, alignment_tendencies, allowed_genders (HashMap<String, GenderDef>), appearance_bounds, age_default, age_max

- Size effects: small (+1 AC, -1 damage, ×0.75 carry), medium (baseline), large (-1 AC, +1 damage, ×1.5 carry)
- Traits are boolean-or-numeric always-on passives (e.g. `infravision = 60`), checked at query time
- Racial abilities are `SkillDef` entries in `content/skills/racial/` with `allowed_races` gates
- Languages defined in `content/languages.toml`, auto-granted on creation
- Familiarity gives bonus on checks involving listed races
- **GenderDef** maps a gender id to its pronoun tuple: `male → (he, him, his)`, `female → (she, her, hers)`, `neutral → (they, them, their)`. Custom genders require explicit pronoun definitions.
- **AppearanceBounds** defines `height_min/max` (inches), `weight_min/max` (lbs), `allowed_builds`, `allowed_hair_colors`, `allowed_eye_colors`, `allowed_skin_tones` — enforced during character creation
- **Age default** is the starting age; **age max** is the upper bound for natural aging

---

## Classes

Classes are template definitions, not enums. All behavior in `content/classes/*.toml`.

**ClassTemplate:** id, name, description, attribute_mods, hit_die, bab, fort_save, ref_save, will_save, skill_ranks_per_level, prestige, allowed_races, allowed_alignments, class/cross_class/exclusive_skills, auto_skills, stances, passives, multi_classing

- BAB & saves: `ClassProgression` computed at level-up, stored in `CombatStats`
- `bab`: `"full"` (+1/level), `"medium"` (+3/4), `"poor"` (+1/2) — controls `CombatStats.base_attack_bonus` recalculation on level-up
- Save fields (`fort_save`, `ref_save`, `will_save`): `"good"` (+½×(level+2)) or `"poor"` (+⅓×level) — controls `CombatStats.*_save` recalculation
- Alignment gates on creation; violations warn during gameplay
- Stances: toggleable combat modes with trade-offs; one active at a time
- Passives: always-on bonuses applied by `PassiveApplicationSystem`

---

## Prestige & Multi-Classing

### Multi-Classing

`MultiClassInfo { classes: Vec<ClassEntry> }` — each class entry has id, level, is_favored. Total level = sum of all classes.

**XP penalty:** `(non_favored_classes - 1) × 20%` (max 80%). Favored class waives penalty. Adding: `@multi_class <class>` — checks race/alignment gates. Leveling: player chooses which class to advance.

### Prestige Classes

Template with `prestige = true` + `[prestige_gate]`. Gates: requires_class, requires_skills, requires_race, requires_alignment, requires_quest, requires_faction, requires_level. Granted via `@prestige` or Rhai `grant_prestige()`. Multiple allowed.

---

## Experience & Leveling

### XP Curve

`XP(level) = level³ × 100`. Level cap configurable (default 100).

### XP Sources

| Source       | Formula                              |
| ------------ | ------------------------------------ |
| Kill mob     | `victim.level² × 50 × xp_multiplier` |
| Quest reward | Per-quest definition                 |
| Explore room | `5 × room.level` (first discovery)   |
| Craft item   | `item.level² × 10`                   |
| Group bonus  | +10%/member (max +50%)               |
| Danger bonus | +25% for aggro mobs                  |

### Level-Up

When `current_xp >= xp_for_next_level()`: automatic immediate level-up with:

| Effect          | Detail                                                                                                           |
| --------------- | ---------------------------------------------------------------------------------------------------------------- |
| HP gain         | `hit_die + CON_mod` (min 1), max increases                                                                       |
| Full heal       | HP set to new max                                                                                                |
| Resource pools  | Mana/Stamina recalculated via `from_formula(level, int, wis)` / `(level, str, dex)`. Current clamped to new max. |
| BAB & saves     | Recalculated per class progression: `bab` (full/medium/poor), saves (good/poor)                                  |
| Practice points | `(2 + WIS_mod + INT_mod).max(1)` added to `PracticePoints` pool                                                  |
| Passives        | Re-applied via `apply_all_passives()`                                                                            |
| Event           | `PlayerLeveled { entity, old_level, new_level }` emitted                                                         |

Multiple levels gained at once if XP far exceeds threshold; the loop processes each increment sequentially. Level-up messages returned to the caller for delivery to the player's connection.

### Death Penalty

`xp_loss = 10%` (configurable), capped at 5 levels' worth. Never de-levels.

### Player Death & Ghost Loop

On death (health drops to `-10` or lower), players are not despawned. Instead:

- A player corpse is spawned in the room containing all inventory and equipped items. Player corpses decay in 30 minutes (mobs decay in 5 minutes) and are owned by the player.
- Player's inventory and equipment are cleared.
- Player's health is set to `1` HP, and they are teleported to their `RecallRoom`.
- Player enters `PlayerState::Dead` (Ghost state).
- In the Ghost state, they cannot speak normally (whispers are colored in alternating cyan/blue characters), cannot engage in combat (ignored by aggro), and cannot pick up or wear items.
- Ghosts revive by walking to their corpse and using `reclaim` or `revive` (restores all gear), or — if in a room flagged with `allow_revive` — they can use `revive` alone (naked revive, gear stays in the corpse).

---

## Training & Practice System

DIKU-style unified practice pool: one resource funds both stat training and skill practice, giving players a meaningful choice between improving attributes or broadening abilities.

### Practice Points

`PracticePoints(u32)` is a per-character component. Gained on each level-up: `gain = (2 + WIS_mod + INT_mod).max(1)` where `WIS_mod = (wisdom - 10) / 2`, `INT_mod = (intelligence - 10) / 2`. Minimum 1 per level.

### Trainer NPCs

Both `train` and `practice` commands require a trainer NPC in the same room. Trainer NPCs have a `Trainer` component with `trainer_types`: empty list = general (can train anything), specific types restrict to matching categories (e.g. `["attributes"]`, `["combat"]`). Mob templates define `trainer_types`; component attached at spawn via `bin/src/init.rs`.

### `train <stat>`

Increases one of the six core attributes. Requires trainer with `"attributes"` type in room, `PracticePoints >= cost`, and stat < 50. **Cost:** 5 points (3 for the class's prime attribute — the one with highest class modifier). On success: deduct cost, increment stat by 1, set Dirty.

### `practice <skill>`

Increases a learned skill's rank by 1. Requires matching trainer type, `PracticePoints >= 1`, skill in LearnedSkills, and rank < SkillCap. **Cost:** 1 point per rank. On success: spend 1 point, rank += 1, set Dirty.

### Skill Caps

`cap = SkillCap.base_cap + SkillCap.per_level × level`. Default: `base_cap = 5`, `per_level = 5` → `5 + 5 × level`. Class templates define: `class_skills` (full cap), `cross_class_skills` (half cap), `exclusive_skills` (per-class). See [Skill System](#skill-system).

### Database

`PracticePoints` persisted in `components_practice_points` table.

---

## Item System

Items defined in TOML templates under `content/items/`. For the full TOML schema, item types, quality tiers, trigger events, sets, and affixes, refer to [builder_manual.md](file:///Users/therealklanni/Projects/mud/docs/builder_manual.md).

### Item Restrictions & Gates

Equip restrictions (such as `allowed_classes`, `allowed_races`, `allowed_alignments`, and `requires_skill` gates) are verified at equip time and continuously checked by `SkillRequirementSystem`, which automatically removes the item if prerequisites are violated.

### Durability & Repair (Planned)

Weapons lose durability on hitting, and armor on being hit. An item with `current == 0` durability is broken and yields no stat bonuses. Items can be repaired at black-smith NPCs or via a repair skill.

### Planned Systems

- **Container items**: Items capable of holding other item entities.
- **Usable commands**: Commands like `use`, `drink`, `eat`, `quaff`, `recite` for potions, scrolls, wands, food, and drink.
- **Rhai scripting for triggers**: Execution of custom Rhai scripts when item triggers fire.

---

## Mob Templates

NPCs defined in TOML templates under `content/mobs/`. For the full TOML schema and AI configurations, refer to [builder_manual.md](file:///Users/therealklanni/Projects/mud/docs/builder_manual.md).

- **MobTemplate** schema defines identifiers, level, attributes, health, armor, custom size, faction standing, equipment, loot, AI configurations, languages, and scripts.
- Spawns via area template `[mobs.<area>.<room>]` with count + respawn timer.
- `AreaResetSystem` populates/respawns mobs on area reset.

---

## Shop & Economy

**Currency:** Three-tier decimal in copper pieces. Wallet: `{ copper, banked_copper }` — both u64.

| Coin        | Value              |
| ----------- | ------------------ |
| Copper (cp) | 1                  |
| Silver (sp) | 100 cp             |
| Gold (gp)   | 10,000 cp (100 sp) |

**NPC Shops:** `Shop { name, buy_rate, sell_rate, inventory, currency, restock_secs }`. Shop templates in `content/shops/*.toml` instantiated on NPC entities at server start.

**Pricing:** `buy_price = base × sell_rate × reputation_mult`, `sell_price = base × buy_rate × reputation_mult`. Reputation tiers: adored (0.80), friendly (0.90), neutral (1.00), unfriendly (1.25), hostile (1.50).

---

## Deity System

Deities are template entities defined in `content/deities/*.toml`. A character may adopt a deity during creation or in-game. Deities grant optional effects when prayed to, subject to a cooldown.

**DeityTemplate:** id, name, description, alignment, symbol, favored_weapon, tenets, domains (War/Nature/Trickery/Knowledge/Life/etc), allowed_races/classes/alignments, prayer_effect.

**PrayerEffect:** buff_id (references PassiveDef or ActiveEffect), duration_secs, cooldown_secs, description (flavor text).

### Class Deity Policy

Class templates define a `deity_policy` field controlling character creation behavior:

| Policy          | Meaning                                       |
| --------------- | --------------------------------------------- |
| `any`           | Player may choose any deity or none (default) |
| `none`          | Player may not have a deity                   |
| `required`      | Player must choose a deity                    |
| `subset([ids])` | Player must choose from this explicit list    |

### Pray Command

```
pray — pray to your deity (if you have one)
pray <deity> — pray to a specific deity (shrine required in room)
pray <target> — cleric/paladin heal ally via deity channel
```

Each prayer applies the deity's `prayer_effect` as an `ActiveEffect` component with the specified duration. On cooldown: display remaining time message. Cooldown tracked as a simple `Instant` timestamp on the player entity.

### Domains

Domains are thematic groupings used for:

- Quest gating (domain-restricted quests)
- Faction relationships (domain-aligned factions)
- Class requirements (domain-restricted prestige classes)
- Item restrictions (domain-aligned items)

### ECS Integration

- `Deity(Option<String>)` component on player entity (optional)
- `PrayerCooldown { last_prayed: Instant }` component or resource tracks cooldown per entity
- `PrayState` state machine (future) — Idle, Praying, OnCooldown

### Content Directory

```
content/deities/*.toml
```

---

Recipes in `content/recipes/*.toml`. `RecipeDef`: id, name, station, skill requirement, difficulty, materials, result, success_chance, quality_scaling, script.

**Flow:** check recipe known → station present → skill rank → materials → roll success. Success: roll quality upgrade (skill margin / margin_per_point). Failure: consume 50% materials. Critical failure (natural 1): all materials consumed.

Stations are room flags (`room_flags = ["station:anvil"]`) or entities with `Station` component. Recipes learned via auto-grant, trainer, scroll drops, or Rhai `grant_recipe()`.

---

## Quest System

### Quest State Machine

```
Inactive → { accepted } → Active → { all_objectives_complete } → ReadyToTurnIn
ReadyToTurnIn → { turn_in } → Completed
Active → { abandon } → Abandoned (repeatable: → Inactive)
```

Emits `QuestUpdated` on progress, `QuestCompleted` on turn-in.

### Quest Template

`QuestDef`: id, name, description, level_requirement, repeatable, auto_complete, giver_npc, turn_in_npc, prerequisites, objectives, rewards, scripts.

**Objective Types:** Kill(mob, count), Gather(item, count), Deliver(item, npc), Explore(room), Talk(npc), Escort(npc, dest), Craft(item, count), Use(skill, count).

**Reward Types:** Xp, Gold, Item, Faction, Skill, Recipe. Auto-updated via event subscriptions.

---

## Faction System

Factions track numeric standing that gates access, affects prices, triggers aggro, and gates quests/prestige.

**FactionDef:** id, name, description, starting_standing, min/max, ranks, relationships, aggro.

Standing changes propagate to related factions via multiplier.

| Source                | Delta       |
| --------------------- | ----------- |
| Kill aggro member     | +5          |
| Kill ally member      | −10         |
| Quest completion      | Per quest   |
| Attack faction member | −50 to −200 |

Rank resolved by highest threshold. Aggro checked on room entry.

---

## Command System

Commands are stored as a flat `Vec<Command>` with linear prefix matching (trie planned). Each command:

- `Command { name, aliases, access (AccessLevel), handler: fn(&mut World, &mut Connection, args) -> CommandResult }`

### Player Commands

`look/l`, directional (`n/s/e/w/u/d/ne/nw/se/sw`), `enter`, `say`, `tell/whisper`, `reply/r`, `shout`, `emote/:`, `channels`/`channel`, `kill`, `get/drop`, `put/in`, `give/to`, `inventory/i`, `equipment/eq`, `wear/wield/remove`, `examine/exam`, `open/close/lock/unlock`, `use/cast`, `score/stats`, `train`, `practice`, `craft`, `recipes`, `repair`, `stance`, `group`, `follow`, `quests/quest/quest abandon`, `factions/faction`, `sit/rest/sleep/wake/stand`, `reclaim/revive`, `toggle`, `loot`, `time`, `weather`, `motd`, `help/?`, `who`, `config`, `@prestige`, `@multi_class`

### Builder Commands

`@area` (create/list/edit/delete/reset/save), `@dig`, `@link/@unlink`, `@set`, `@desc`, `@room delete`, `@portal` (add/remove/hide), `@mob` (add/remove/edit), `@item` (create/edit/delete), `@load`

### Immortal Commands

`goto`, `at`, `force`, `stat`, `owhere`/`olocate`, `gecho`, `gtell`, `wizwho`, `wizin`, `holylight`, `@teleport`, `switch`, `return`

### God Commands

`@purge`, `@slay`, `@restore`, `@clone`, `ban/unban`, `freeze/unfreeze`, `load`

### Admin Commands

`shutdown`, `restart`, `wizlock`, `config`, `version`, `audit`

### Permission Checking

Command dispatch gates on `conn.access_level() < cmd.access`. Five tiers: `Player < Builder < Immortal < God < Admin`. Account-level permission. `Immortal` component added on spawn if account's access > Player.

### Incognito Mode

`wizin` toggles incognito. Who list skips incognito. Look shows "You sense a presence here." to mortals. God+ with holylight see through incognito.

### Safety Invariants

Combat skips damage on Immortal components. `switch` refuses player targets. `force` blocked if target's access >= executor's. `purge` refuses Immortal entities. All destructive actions logged via `tracing::warn!`.

---

## Communication & Socials

**Channels:** say (room), tell/reply (player), whisper (room-private), shout (zone), yell (area), emote (room), gossip/auction/ooc (global), gtell (immortal), admin (admin). Each with name, color, min level, min access, history.

**Socials:** TOML-defined in `content/socials.toml` with three message forms (self, target, room). Built-in: smile, wave, nod, glare, poke, hug, frown, grin, wince, cough, sigh, laugh, bow, curtsey, shrug, applaud, sniff, salute, shiver.

---

## Time System

Game time advances independently of real-world time. The scale is configurable in `content/server.toml` under `[time]`:

| Field                        | Default    | Description                         |
| ---------------------------- | ---------- | ----------------------------------- |
| `real_minutes_per_game_hour` | `24`       | Real-world minutes per in-game hour |
| `days_per_season`            | `30`       | Game days per season                |
| `start_season`               | `"spring"` | Season on first boot                |
| `start_hour`                 | `6`        | Hour on first boot (0–23)           |

### Clock Model

- 1 in-game hour = `real_minutes_per_game_hour` real minutes
- 1 in-game day = 24 game hours (default: 9.6 real hours)
- 1 season = `days_per_season` game days (default: 30)
- 4 seasons = 1 year (120 game days, default: ≈48 real hours)

### Time Periods

Eight named periods map to hour ranges:

| Period    | Hours | Description               |
| --------- | ----- | ------------------------- |
| Midnight  | 1–5   | Deep night, few abroad    |
| Dawn      | 5–7   | First light, birdsong     |
| Morning   | 7–10  | Bright, market bustle     |
| Noon      | 10–14 | Sun at its peak           |
| Afternoon | 14–17 | Warm, shadows lengthen    |
| Dusk      | 17–19 | Fading light, fires lit   |
| Evening   | 19–22 | Dark, taverns busy        |
| Night     | 22–1  | Full dark, danger outside |

### Commands

- `time` — shows period, day, season, year (e.g. "It is Dawn on the 14th day of Spring, Year 1.")

### Prompt Variables

- `%t` — current time period name (e.g. "Dawn")

### Persistence

Game time is stored in SQLite `world_time` table. Saved on shutdown (lazy persist). On startup, time loads from DB; if missing, uses `[time]` config defaults.

### Events

- `TimeChanged` — emitted every game hour advance
- `PeriodChanged` — emitted when the named period changes (e.g. Dawn → Morning)
- `SeasonChanged` — emitted when the season rolls over

---

## Weather System

Weather is data-driven, per-zone, with probability-based transitions. Defined in `content/weather.toml`.

### Architecture

Weather operates on a **base + modifier** model:

- **Base condition**: Clear, Rain, Snow, Storm, etc. (one active at a time)
- **Modifier**: Strong Wind, Fog, etc. (optional secondary condition)

If no weather is active, the room is **Clear** (no effects).

### Composition Model

Weather conditions are resolved per-room by merging three layers:

1. **Global defaults** — season availability from `weather.toml [seasons]`
2. **Area override** — `weather_zone` references a zone matrix, OR `weather_matrix` defines inline per-season weights. Areas can set `no_weather = true` to disable weather for the entire area.
3. **Room override** — additive/subtractive:
   - `no_weather = true` → no weather (indoors, underground)
   - `exclude_weather = ["strong_wind"]` → removes specific conditions (forest blocks wind)
   - `additional_weather = { fog = 15 }` → adds conditions (cave entrance gets fog)

If an area has `no_weather = true`, all rooms in that area inherit it (room overrides ignored).

### Resolution Chain

```
area.no_weather = true?  →  Clear (done)
         ↓ no
global season conditions
         ↓
area.weather_zone or area.weather_matrix (if present)
         ↓
room.exclude_weather (remove entries)
         ↓
room.additional_weather (add/merge entries)
         ↓
room.no_weather = true?  →  Clear (done)
         ↓ no
resolved weights → roll_weather()
```

### Weather Conditions

Defined in `content/weather.toml [conditions]`:

```toml
[conditions.rain]
name = "Rain"
description = "Rain falls from grey clouds."
severity = "minor"          # "minor" = on-request, "severe" = auto-broadcast
[conditions.rain.effects]
damage_fire = -2            # Flat damage modifier
damage_lightning = 2
```

Severity controls player notifications:

- **Severe** (Storm, Blizzard) → auto-broadcast to players in affected rooms
- **Minor** (Rain, Fog, Wind) → shown on `weather` command or room entry

### Season Availability

```toml
[seasons.spring]
available = ["clear", "rain", "fog", "strong_wind"]
```

Only conditions listed for the current season can roll.

### Zone Probability Matrices

```toml
[zones.temperate.spring]
clear = 40       # Weight (system normalizes to probability)
rain = 30
fog = 20
strong_wind = 10
```

Weights are arbitrary values — the system normalizes them internally. If all weights resolve to zero or empty, the result is **Clear**.

### Weather Tick

Fires every 5 minutes (300s). For each active weather zone:

1. Resolve the room's weather set (area + room overrides)
2. Roll base condition from resolved weights
3. Roll modifier separately (same logic, only modifier-type conditions)
4. If weather changed → update `WeatherState` component on room entity
5. If new condition is **severe** → broadcast to all players in room

### Gameplay Effects

Weather effects are defined per-condition in `weather.toml` and applied during combat, skill use, and movement:

| Effect Key            | Applies To              | Description                                 |
| --------------------- | ----------------------- | ------------------------------------------- |
| `damage_fire`         | Fire damage rolls       | Flat modifier to fire-type damage           |
| `damage_lightning`    | Lightning damage rolls  | Flat modifier to lightning damage           |
| `ranged_accuracy`     | Ranged attack hit rolls | Flat penalty to ranged attacks              |
| `ranged_accuracy_pct` | Ranged attack hit rolls | Percentage penalty to ranged accuracy       |
| `ranged_attack`       | Ranged attack rolls     | Flat attack penalty                         |
| `dexterity`           | DEX attribute           | Temporary DEX modifier while weather active |

### Room Descriptions

Weather flavor text is appended to room descriptions on `look`:

- `"Rain falls steadily here."`
- `"A thick fog obscures the exits."`

### Persistence

Weather states stored in SQLite `weather_states` table (zone_id → base, modifier). Saved on shutdown (lazy persist). On startup, loads from DB; if missing, rolls initial weather from zone matrix.

### Content Structure

```toml
# content/weather.toml

[conditions.clear]
name = "Clear"
description = "The sky is clear and pleasant."
severity = "minor"

[conditions.rain]
name = "Rain"
description = "Rain falls from grey clouds."
severity = "minor"
[conditions.rain.effects]
damage_fire = -2
damage_lightning = 2

[conditions.storm]
name = "Storm"
description = "Thunder rolls overhead as rain lashes down."
severity = "severe"
[conditions.storm.effects]
damage_fire = -2
damage_lightning = 2
ranged_accuracy = -2

[conditions.fog]
name = "Fog"
description = "A thick fog rolls in, obscuring your vision."
severity = "minor"
[conditions.fog.effects]
ranged_accuracy_pct = -25

[conditions.snow]
name = "Snow"
description = "Snow falls gently from the sky."
severity = "minor"
[conditions.snow.effects]
dexterity = -1

[conditions.blizzard]
name = "Blizzard"
description = "A howling blizzard lashes at you with ice and wind."
severity = "severe"
[conditions.blizzard.effects]
dexterity = -2
ranged_accuracy = -3

[conditions.strong_wind]
name = "Strong Wind"
description = "Strong winds gust across the land."
severity = "minor"
condition_type = "modifier"
[conditions.strong_wind.effects]
ranged_attack = -2

# Season availability
[seasons.spring]
available = ["clear", "rain", "fog", "strong_wind"]

[seasons.summer]
available = ["clear", "rain", "storm", "strong_wind"]

[seasons.autumn]
available = ["clear", "rain", "fog", "strong_wind"]

[seasons.winter]
available = ["clear", "snow", "blizzard", "strong_wind"]

# Zone probability matrices
[zones.temperate.spring]
clear = 40
rain = 30
fog = 20
strong_wind = 10

[zones.temperate.summer]
clear = 50
rain = 20
storm = 20
strong_wind = 10

[zones.temperate.autumn]
clear = 35
rain = 30
fog = 25
strong_wind = 10

[zones.temperate.winter]
clear = 30
snow = 35
blizzard = 20
strong_wind = 15
```

### Room/Area TOML Fields

**AreaTemplate additions:**

- `no_weather: bool` — disables weather for entire area
- `weather_zone: String` — reference a zone in weather.toml
- `weather_matrix: { season → { condition → weight } }` — inline zone definition

**RoomTemplate additions:**

- `no_weather: bool` — disables weather for this room
- `exclude_weather: [String]` — conditions to remove from resolved set
- `additional_weather: { condition → weight }` — conditions to add to resolved set

---

## Telnet Protocol

IAC byte parser with state machine: `Data → IAC → Will/Wont/Do/Dont/Subneg`. Each connection gets a `TelnetConnection` wrapping `TcpStream`, implementing the `Connection` trait.

**Feature Detection:** `Feature`: Ansi, ExtendedColor, Naws, Mccp, Gmcp, Mxp, Mssp, Blink, Html, Utf8. Negotiation: WILL ECHO + DO NAWS + DO TERMINAL-TYPE → client replies → capability set. 256-color/GMCP/MXP negotiated if terminal type supports (MTTS detection).

**Keepalive:** `IAC NOP` every 60s; 120s inactivity = disconnect. Detected by KeepaliveSystem. Emits `PlayerDisconnected`.

**Connection Registry:** `Arc<Mutex<HashMap<Entity, mpsc::UnboundedSender<Vec<u8>>>>>` — broadcast mechanism for room messages.

---

## Text Formatting & Color

Color types: 16 ANSI + `Indexed(u8)`. `Modifier(u8)` bitmask (BOLD|DIM|ITALIC|UNDERLINE|BLINK|REVERSE|HIDDEN|STRIKE). `RichText(Vec<Segment>)` with `Segment { text, fg, bg, modifiers }`.

Tag syntax: `{red}text{/}`, `{brightblue}item{/}`, `{yellow bold}critical!{/}`, `{bg:color}`, `{/modifier}`, `{{` emits literal brace. Parser at `core/src/format/tag.rs::parse_tags()`.

---

## Persistence

**Two-tier:** In-memory ECS world ↔ SQLite on disk (WAL mode). Dirty tracking via `Dirty` marker component. Flush every 5s (DirtyFlush phase). Full flush + WAL checkpoint on shutdown.

**Schema:** SQLite tables mirror component types: `entities` table + `components_*` tables per component type, plus `world_time` (game clock) and `weather_states` (per-zone weather). Startup: load entities, populate components, delete stale.

**WriteBatch:** `{ entity_id, entity_type, components: Vec<ComponentRow> }`. Type-safe queries in `data/src/queries.rs`. Migrations via `PRAGMA user_version`.

**WAL config:** `PRAGMA journal_mode = WAL, foreign_keys = ON, busy_timeout = 5000, synchronous = NORMAL`. Connection in `Arc<parking_lot::Mutex<Connection>>`.

**Backup:** Hot backup via SQLite online backup API. Scheduled hourly by `BackupSystem`. Stored in `data/backups/`, retain 7 daily + 4 weekly.

---

## Content Loading & Hot-Reload

All game content in TOML under `content/` (configurable path). Scanned at startup, deserialized via serde, cross-referenced, built into `TemplateRegistry` (behind `Arc<RwLock<...>>`).

**Directory layout:** `content/{areas, mobs, items, races, classes, skills, scripts, recipes, quests, factions, shops, help, deities, affixes, sets}/` + `weather.toml`, `languages.toml`, `socials.toml`, `treasure_classes.toml`. Rooms live in individual files under `content/areas/<area_id>/rooms/<room_id>.toml`.

Hot-reload uses `notify` crate. On change: re-parse, validate, atomic-swap in registry, emit `ContentReloaded`.

---

## Zone & Area System

Areas group rooms into named, managed zones. Each area is a directory under `content/areas/<area_id>/` containing an `area.toml` (metadata) and a `rooms/` subdirectory with one TOML file per room.

**AreaTemplate:** id, name, description, level_range, flags, weather_zone, reset_interval_secs, credits. **RoomTemplate:** id, area, name, description, exits, portals, flags, content, spawn (optional — label, description, allowed_classes for character creation).

### Room Exits & Doors

Exits in `RoomTemplate` are specified as either a destination room ID string or a detailed table:

- Simple: `north = "area_id:room_id"`
- Detailed: `north = { dest = "area_id:room_id", door = true, closed = true, locked = false, key_id = "key_bronze" }`

### Area Flags

city (safe rest), peaceful (no combat), no_pk, no_magic, no_summon, no_flee, underground (always dark), water (breathing check), air (flying only), hell (no recall).

### Room Flags

portal_in/out (opt-in teleport targeting), no_teleport_in/out (opt-out teleport blocking). Immortal commands bypass all flags.

### Area Reset

Each area has a reset interval. On reset: respawn dead mobs, re-equip NPCs, reset room flags/doors/containers, clean up expired corpses and old items. Staggered via `last_reset` timestamp.

### Builder Commands

`@area` (create/list/edit/delete/reset/save), `@mob add/remove`, `@dig`, `@link/@unlink`, `@set`, `@desc`, `@portal`

---

## Startup & Shutdown Flow

**Startup:** CliParse → ConfigLoad → LoggingInit → ContentLoad → Validation → DatabaseOpen → WorldCreate → StateSeed → SystemRegister → ScriptingInit → EventBusInit → CommandRegister → ListenerBind → BackgroundTasks → Ready

**Shutdown:** Close listener → notify players → drain in-flight commands (200ms) → flush all dirty → WAL checkpoint (5s) → disconnect all players. Triggers: SIGTERM, SIGINT, `shutdown` command, fatal error.

---

## Configuration

`server.toml` (override with `--config` flag or `OXIDE_CONTENT` env var). Sections: server (host, port, max_players), database (path), game (name, motd, start_room, max_level, content_dir), combat, training, multi_classing, item_sets, logging.

**Precedence:** CLI flags > env vars > config file > built-in defaults. Runtime overrides via `config` command, persisted to SQLite. Server section changes require restart.

`Config` resource in ECS world, accessible to all systems.

---

## Error Handling & Logging

Crate-level `Error` enums with `thiserror` + `Result` aliases. Errors composed (not boxed) via `From` impls.

**Logging** via `tracing`. Levels: `error!` (unrecoverable), `warn!` (admin actions, audit), `info!` (lifecycle), `debug!` (dev diagnostics), `trace!` (per-pulse). Admin actions logged with target: "audit".

---

## Help System

Help topics in `content/help/*.toml`. `HelpEntry`: id, aliases, title, text. Loaded into TemplateRegistry at startup. Index maps id + all aliases.

```
help — topic index
help <topic> — show topic
help <partial> — keyword search
```

Builder-created help stored in SQLite `help` table, merged with file-based topics (DB overrides on collision).

---

## Login & Character System

### Connection State Machine

Each connection traverses a state machine between telnet negotiation and gameplay:

```
Connected → Negotiating → Banner → Username → Password → CharacterSelect → Playing
↘ CharacterCreate* ↗
```

`ConnectionState`: Connected, Negotiating, Banner, Username, Password, CharacterSelect, CharacterCreate{Name,Race,Class,Gender,Attributes,Alignment,Deity,Skills,Appearance,Description,Spawn,Confirm}, Playing.

Login state machine lives in `server/src/login/` as a standalone `LoginFlow` struct with separate modules: `state.rs` (enum + transitions), `handlers.rs` (per-state input handling), `prompt.rs` (prompt rendering). Failed input counts toward strike limit (3 → disconnect).

### Login Flow

1. Telnet negotiation completes → send banner from `content/banner.txt` + MOTD
2. Username prompt → case-insensitive lookup in `accounts` table
   - Found → transition to Password
   - Not found → offer account creation (y/n)
3. Password → argon2 verify against hash in DB (hash fetched outside DB lock to avoid blocking)
   - Match → cache access_level on connection → CharacterSelect
   - No match → retry (3 total strikes)
4. Character select → show list + "Create new" → pick existing or create

### Character Creation Wizard

Steps: Name (3–16 chars, unique) → Race (from TOML templates) → Class (race/class cross-filtered) → Gender (race-allowed, custom pronouns if other) → Attributes (point-buy 27pts / standard array / 4d6-drop-lowest; clamped [3,25]) → Alignment (3×3 grid, race/class restricted) → Deity (class policy: any/none/required/subset) → Skills (class pool, prefix match) → Appearance (race-bounded height/weight/build/hair/eyes/skin) → Description (multi-line, `.` to finish) → Spawn (race/class/alignment filtered) → Confirm → save + spawn entity.

### Attribute Calculation

`final = race_base + class_mod + bonus_points`. Three methods:

- **Point-buy:** Each stat starts at 8, 27 points, progressive costs (8→0, 18→19)
- **Standard array:** 15, 14, 13, 12, 10, 8 — assign freely
- **Roll:** 4d6 drop lowest × 6, assign freely, up to 3 rerolls

On confirm: insert `characters` row → create ECS entity with `Position` (starting room), `Player`, `Gender`, `Attributes`, `Health`, `Level`, `Experience`, `LearnedSkills`, `Alignment`, `Description`, `Deity`, `Appearance`, `Age` → state = Playing.

---

## Server Console

The `server` crate exposes a `Console` struct with static accessor methods for runtime diagnostics and debug commands:

- `Console::broadcast(msg)` — send message to all connected players
- `Console::shutdown()` — initiate graceful shutdown
- `Console::player_count()` — current online count
- `Console::list_players()` — names of connected players
- `Console::world_status()` — entity counts, system timing stats
- `Console::flush_db()` — force dirty flush

Implemented via a `OnceLock<Arc<ConsoleState>>` global set during server initialization. Used by admin commands and the shutdown signal handler.

---

## Scripting & OLC

### Design Principle

Rhai scripts drive all dynamic behavior not captured by TOML: NPC logic, item procs, quest triggers, stance/passive effects, OLC automation, custom skills.

### Engine Setup

Sandboxed `rhai::Engine` per execution. Limits: 8 modules, 32 call levels, 50k operations, 10k string size, 100 dynamic arrays, 50 maps.

### Script Lifecycle & Bridges

To maintain modular boundaries and respect the dependency DAG, scripting uses dependency injection:

- `core` defines the `ScriptingBridge` and `MessageOutputBridge` traits.
- `scripting` implements `ScriptingBridge`, running the sandboxed Rhai engine and caching compiled ASTs.
- `server` implements `MessageOutputBridge` (handling physical connection packet I/O).
- `bin` registers these implementations in global cells at startup.

Rhai scripts run synchronously under the World lock. Scripts receive a `ScriptWorld` wrapper around a raw pointer to the ECS `World` (safe for synchronous execution duration), exposing operations like mob spawning, movement, custom damage, and skill rank modifications.

### Integration Hooks

- **Combat Swing Hooks**: `execute_combat_hit_hook` executes on attacker/defender skills, passing a mutable `HitContext`. Scripts can add/subtract hit chance modifiers, override the hit outcome, or abort the attack roll entirely with a custom message (returning `HitResult::Aborted` to silence the default miss outputs). `execute_combat_damage_hook` allows modifying final damage amounts and types.
- **Mob AI Hooks**: `execute_mob_ai` triggers on custom AI ticks, allowing scripts to override default mob behaviors (say, move, use skills, follow leaders via the `Following` component).
- **Say Triggers**: `execute_say_hook` executes when a player speaks in a room, broadcasting to say triggers registered on the room itself, floor items, or carried equipment (enabling secret door words, magical talking swords, etc.).
- **Active Skills**: A player-facing `use`/`cast` command resolves targeted spell/skill scripts in parallel with the main game loop, validating player skill ranks and executing the script.

### Security Model

| Concern        | Protection                                            |
| -------------- | ----------------------------------------------------- |
| Infinite loops | max_operations = 50k, max_call_levels = 32            |
| Memory         | max_string_size = 10k, max_arrays = 100, max_map = 50 |
| File access    | Resolver reads only `<content_dir>/scripts/`          |
| Network        | No socket bindings                                    |
| CPU            | Inline but ~1ms at 50k ops                            |

### Hot-Reload

`notify` watcher monitors `scripts/`. File change: re-parse AST, update binding map. Parse failure keeps old AST.

### OLC Commands

All OLC edits are transactional — modify in-memory `TemplateRegistry` immediately, persisted on `@area save` or DirtyFlush auto-save.

**Builder workflow:** `@dig` creates room → `@link` connects → `@set` modifies → `@mob add` spawns → `@area save` writes to disk. Edits stored in `builder_edits` overlay (HashMap of diffs) applied on top of file templates. TOML files remain source of truth.

---

## Protocol Expansion Path

### Phase 0 — Telnet (line mode)

Protocol: TCP (telnet). Features: local echo off, ANSI 16-color. Encryption via stunnel (recommended) — no native TLS.

### GMCP (Phase 6)

Structured JSON over telnet subnegotiation. Modules: Core (hello/supports), Room (Info), Char (Info/Skills/Inventory/QuestList), Comm (Channel), MGK (Target/Spell). Opt-in per client.

### MXP (Phase 6)

Clickable links and gauges. Tags: `<send>`, `<a>`, `<img>`, `<!ENTITY>`. Locked protocol (requires `<VERSION>` header). Server only sends to locked clients.

### WebSocket Bridge (Phase 6)

JSON MMCC frames over WebSocket: `{ type: "command"|"output", payload: { text, html } }`. Wraps Connection trait; ANSI→HTML conversion for browsers.

### REST API (Phase 6)

Lightweight endpoints: GET `/api/who`, `/api/characters`, `/api/characters/:id`, `/api/characters/:id/inventory`. Auth via session token.

### Protocol Feature Matrix

| Feature           | Phase | Requires      |
| ----------------- | ----- | ------------- |
| ANSI 16-color     | 0     | —             |
| NAWS              | 1     | Telnet        |
| UTF-8             | 1     | Telnet        |
| 256-color         | 2     | MTTS          |
| GMCP (Room, Char) | 6     | Telnet        |
| MCCP              | 6     | Telnet        |
| MXP               | 6     | Telnet + lock |
| WebSocket         | 6     | HTTP server   |
| REST API          | 6     | HTTP server   |
| MSSP              | 6     | Telnet        |

---

## spade — Builder TUI & MUD Client

**spade** is the terminal-based builder TUI and MUD client. Named after the `@dig` OLC command — a spade is what you dig with.

### Philosophy

- **Single tool** for all builder workflows: world editing, validation, content browsing, live testing
- **Mouse-first navigation** with full keyboard fallback
- **Offline + online modes:** edit TOML files directly or connect to running server
- **Data-driven** help screens, keybindings, sidebar commands

### Modes

| Mode                | Invocation                        | Description                                               |
| ------------------- | --------------------------------- | --------------------------------------------------------- |
| Builder (offline)   | `spade` or `spade --mode offline` | TOML editor, world tree, validator, file browser          |
| MUD client (online) | `spade --mode online`             | Full client with scrollable output, ANSI, clickable names |
| Split               | `spade --mode split` (F9)         | Builder 50%                                               |
| Connection profile  | `spade connect <host> <port>`     | Quick-connect with saved profile                          |

### Online Mode Authentication

When connecting to a remote server, spade supports two authentication methods:

- **Player mode** — standard username/password login (no config needed)
- **Builder/imm mode** — API key authentication via `api_key` field in `~/.config/spade/config.toml` under `[connection]`. When set, spade sends `@apikey <key>` as the username, bypassing the password prompt. The key must have the `spade` scope and be associated with an account that has builder or imm access.

```toml
[connection]
host = "mud.example.com"
port = 4000
api_key = "your-api-key-here"
```

### Screens (Builder Mode)

F1 Entities Editor (world tree + template form), F2 Room Grid (ASCII map + exit commands), F3 Validation Panel (error list with jump-to-source), F4 File Browser (content tree + TOML/Rhai preview), F5 Script Console (Rhai editor + test runner), F6 Live Dashboard (gauges + log tail). Command Palette via `Ctrl+P`. Layout: left tree, center form/map, right details. Status bar at bottom.

### MUD Client Mode

Sidebar (22 cols, collapsible, mouse-clickable command sections: Movement, Info, Admin, Building, Session). Output window (5000-line scrollable ANSI buffer with search, line numbers, timestamps). Input bar (history, autocomplete, Ctrl+R reverse search). Clickable names via GMCP or heuristic regex — players in brightcyan, mobs in yellow underline, right-click context menu.

### Mouse & Scroll

Left click (select), double click (open), right click (context menu), scroll wheel, Tab (switch screen). Per-pane `ScrollState` with scrollbar. Toggle mouse mode with Ctrl+M.

### Help & Session

Help screen: modal overlay (Ctrl+H), data-driven, dismiss with Escape. `SessionState`: Disconnected → Connecting → Negotiating → LoggingIn → Playing. Connection profiles in `~/.config/spade/profiles.toml`.

### Keybindings

Tab/Shift+Tab (focus cycle), Enter (open/confirm), Escape/Ctrl+Q (back), Ctrl+H (help), Ctrl+D (quit), Ctrl+P (command palette), Ctrl+S (save/send), Ctrl+M (mouse toggle), Ctrl+B (sidebar), Ctrl+K (clear output), Ctrl+L/T (line numbers/timestamps), F9 (split), F12 (toggle mode), F10 (TOML preview). Arrow keys, PgUp/PgDn, Home/End for navigation.

---

## MCP Server — AI Agent World-Building

MCP server exposing the full content toolset to AI agents (Claude, etc.). Agents read, create, edit, and validate game world data via natural language.

### Modes

| Mode    | Trigger                       | Data Source        | Write                        |
| ------- | ----------------------------- | ------------------ | ---------------------------- |
| Offline | `mcp` (default)               | TOML files         | Atomic write (temp + rename) |
| Online  | `mcp --url <url> --key <key>` | SQLite DB via REST | REST bridge to game server   |

Offline mode is primary — AI agents edit TOML files, validation runs locally, human reviews before game loads.

### Transport

Primary: stdio (MCP standard). Future: SSE (HTTP) for remote connections.

### Tools

#### Template CRUD (offline)

Full CRUD for all 15 content categories. Each category gets `list_*`, `get_*`, `create_*`, `delete_*` tools. The `update_template` and `update_room` tools handle field-level patches for all types via JSON round-trip (parse TOML → patch JSON → validate via typed struct → serialize back).

| Category | list | get | create | delete | update          |
| -------- | ---- | --- | ------ | ------ | --------------- |
| areas    | ✓    | ✓   | ✓      | ✓      | update_room     |
| rooms    | ✓    | ✓   | ✓      | ✓      | update_room     |
| mobs     | ✓    | ✓   | ✓      | ✓      | update_template |
| items    | ✓    | ✓   | ✓      | ✓      | update_template |
| races    | ✓    | ✓   | ✓      | ✓      | update_template |
| classes  | ✓    | ✓   | ✓      | ✓      | update_template |
| skills   | ✓    | ✓   | ✓      | ✓      | update_template |
| quests   | ✓    | ✓   | ✓      | ✓      | update_template |
| factions | ✓    | ✓   | ✓      | ✓      | update_template |
| recipes  | ✓    | ✓   | ✓      | ✓      | update_template |
| shops    | ✓    | ✓   | ✓      | ✓      | update_template |
| deities  | ✓    | ✓   | ✓      | ✓      | update_template |
| stances  | ✓    | ✓   | ✓      | ✓      | update_template |
| sets     | ✓    | ✓   | ✓      | ✓      | update_template |
| affixes  | ✓    | ✓   | ✓      | ✓      | update_template |
| passives | ✓    | ✓   | ✓      | ✓      | update_template |

Plus room-specific tools: `link_rooms`, `add_portal`, `remove_portal`.

#### Validation & Search

- `validate` — cross-reference validation across all templates
- `get_stats` — aggregate content counts
- `search` — fuzzy case-insensitive search across names/descriptions
- `validate_content_dag` — circular dependency detection in skill prerequisites

#### Simulators (offline + online)

Simulation tools that hook into core game systems for balance analysis:

| Tool                            | Core Hook                                | Description                          |
| ------------------------------- | ---------------------------------------- | ------------------------------------ |
| `simulate_loot`                 | `systems::loot::roll_loot`               | N corpse loot rolls with drop rates  |
| `simulate_combat`               | `systems::combat` damage formulas        | Hit/miss/crit rates, average damage  |
| `simulate_progression`          | `Experience::for_level`, class tables    | Level-by-level stat progression      |
| `simulate_gear_loadout`         | `systems::passive`, `systems::set_bonus` | Final stats with sets + passives     |
| `simulate_ai_wander`            | `systems::ai` movement                   | Random walk path + room frequency    |
| `simulate_shop_transaction`     | `ShopTemplate` buy/sell rates            | Pricing across reputation levels     |
| `simulate_crafting`             | `systems::crafting`                      | Recipe success/failure/quality rolls |
| `simulate_skill_use`            | `systems::skill_use`                     | Spell/ability effect resolution      |
| `simulate_prayer`               | Deity adoption + prayer buffs            | Eligibility + buff effects           |
| `simulate_prestige_eligibility` | `systems::multi_class`                   | Prestige class gate check            |
| `simulate_group_formation`      | `systems::group` formation               | Party layout stat modifiers          |
| `simulate_death_penalty`        | XP loss, corpse decay, ghost             | Death event calculations             |
| `simulate_character_creation`   | `login::compute_final_attributes`        | Full char creation simulation        |

#### Imm / Online Tools (require `--url` + `--key`)

REST bridge to running game server. Imm tools require immortal+ API key.

| Tool                     | REST Endpoint                 | Description                                                |
| ------------------------ | ----------------------------- | ---------------------------------------------------------- |
| `list_connected_players` | `GET /api/players`            | List online players                                        |
| `imm_put_item`           | `POST /api/imm/put_item`      | Add item to player inventory                               |
| `imm_teleport`           | `POST /api/imm/teleport`      | Teleport player to room                                    |
| `imm_force_command`      | `POST /api/imm/force_command` | Force player to execute command (requires `confirm: true`) |

#### Planned Imm Tools (Phase 6)

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

Destructive tools (`imm_kill`, `imm_purge_room`, `imm_reboot`) require a `confirm: true` parameter.

#### Planned Simulators (Phase 6)

| Tool                      | Core Hook                               | Description                                                             |
| ------------------------- | --------------------------------------- | ----------------------------------------------------------------------- |
| `simulate_regen`          | `systems::regen`                        | HP/mana/stamina regen per tick across rest states                       |
| `simulate_level_up`       | `award_xp` logic                        | Detailed level-up breakdown (HP die, skill points, mana/stamina recalc) |
| `simulate_faction_change` | `systems::faction::handle_faction_kill` | Faction standing changes from killing a mob                             |
| `simulate_quest_rewards`  | `QuestDef.rewards`                      | Quest reward breakdown                                                  |
| `simulate_practice`       | `cmd_train`/`cmd_practice`              | Skill training costs and practice point allocation                      |
| `simulate_xp_curve`       | `Experience::for_level`                 | XP thresholds across all levels                                         |

### Resources

MCP content URIs: `content://areas/`, `content://areas/{key}`, `content://areas/{key}/rooms/{room_key}`, `content://mobs/{key}`, `content://items/{key}`, `content://skills/{key}`, `content://races/{key}`, `content://classes/{key}`, `content://quests/{key}`, `content://recipes/{key}`, `content://factions/{key}`, `content://shops/{key}`, `content://deities/{key}`, `content://stances/{key}`, `content://sets/{key}`, `content://affixes/{key}`, `content://passives/{key}`, `content://validation/`, `content://stats/`.

### Prompts (Guided Workflows)

`create_area_flow` (guided area creation), `review_content` (validation + suggestions), `balance_encounter` (mob vs area tier analysis), `design_quest_chain` (prerequisites, objectives, branching, rewards).

### Crate Layout

```
mcp/
├── Cargo.toml
└── src/
    ├── main.rs          # Entry: --content-path, --url, --key
    ├── lib.rs           # Re-exports OxideMcpServer
    ├── server.rs        # All tool definitions (#[tool] macros)
    ├── content.rs       # Re-exports from oxide_core::content
    └── simulator.rs     # Simulation logic + unit tests
```

### DAG

`mcp` depends on `core` (types) only. Loads TOML files directly for offline mode, HTTP for online. No dependency on server/data/scripting/bin.

---

## Future Feature: LPC Mudlib Importer (Post-1.0, Low Priority)

This outline details the planned (post-1.0, low priority) offline transpiler and runner architecture (`lpc-to-oxide`) to migrate legacy LPC mudlibs (rooms, items, NPCs, and base objects) into OxideMUD compatible TOML data templates and Rhai scripts.

### 1. Stateful Object State Mapping

LPC objects store local variables that persist over their active lifetime. Oxide's Rhai scripts are stateless.

- **Solution:** Introduce an in-memory `DynamicState` component (`HashMap<String, Dynamic>`) to the ECS. When modified by scripts, the entity is flagged as `Dirty`. State will be flushed to the SQLite database during the standard 5-second `DirtyFlush` maintenance tick, preventing database write bottlenecks. Expose `self.get_state("key")` and `self.set_state("key", value)` to Rhai.

### 2. Dynamic Commands (`add_action`)

LPC mudlibs register actions dynamically on players in proximity.

- **Solution:** Add an `ActiveCommands` component to characters. When a player enters a room or equips an item, the engine registers their custom command verbs. The command dispatcher (`server/src/cmd/`) matches input verbs using the same prefix/abbreviation matching rules as built-in commands. Built-in engine commands always take precedence.

### 3. Pre-Movement Interception (`on_before_exit`)

LPC scripts block player movement based on custom conditions (e.g., guard NPCs blocking exits).

- **Solution:** Implement an `on_before_exit` hook run by `systems::movement` on the current room and all NPCs inside it. Returning `false` or invoking `cancel_move` aborts the move.

### 4. Code Reuse and Inheritance

LPC heavily relies on class inheritance (e.g., inheriting `/std/room.c`).

- **Solution:** The transpiler will flatten the inheritance hierarchy during compilation or generate Rhai `import` statements (e.g., `import "std/room" as room;`).

### 5. Delayed Events & Heartbeats (`call_out`)

LPC allows scheduling future functions and recurring ticks.

- **Solution:** Implement a central `ScriptTimerManager` resource in the ECS world. It will register delayed/recurring task profiles and trigger executions on the 250ms Player State tick.

### 6. Transpiler Pipeline

The offline tool converts LPC assets into Oxide templates and scripts:

- **Phase 1: Preprocessing & Macro Resolution:** Integrate a dedicated C-preprocessor pass accepting a mudlib `--include-dir` to resolve all macros, conditional compilation (`#ifdef`), and header expansions.
- **Phase 2: Static Structure Extraction:** Parse static configurators (e.g., `set_short`, `set_long`) and output clean TOML templates.
- **Phase 3: Behavior Transpilation:** Map LPC functions (`init`, `hit_callback`) and standard efuns (`write`, `say`, `destruct`) into Rhai equivalents.

---

## Development Phases

### Phase 0 — Foundation ✓

Cargo workspace (5 crates), core types, TCP listener with telnet, basic ECS (hecs), resource pools (Stamina/Mana/Energy/Psi), void room, CLI config, graceful shutdown, 49 unit tests.

### Phase 1 — World & Movement ✓

ConnectionRegistry for room broadcasts, say/look/movement commands (10 directions), void room blocking, auto-look on entry, ANSI color module, connection feature flags, player cleanup on disconnect.

### Phase 2 — Character System ✓

LoginFlow state machine, account creation (argon2), character creation wizard (12 steps: name→race→class→gender→attributes→alignment→deity→skills→appearance→description→spawn→confirm), TOML race/class templates, unified SkillDef, cross-reference validation, auto-grant racial abilities + class auto-skills, Gender/Appearance/Age/Deity components, pray command, deity_policy enforcement.

### Phase 3 — Combat & Equipment ✓

Health/Damage components, combat system (attack/damage rolls), damage types with resistance/vulnerability, weapon styles (two-handed, dual-wield), equipment/inventory, weapon/armor restriction gates, NPC mobiles with AI, mob templates, stances, passives, skill caps, Training & Practice system, item triggers, item sets + SetTracker, random loot quality/affix rolling.

### Phase 4 — Advanced Gameplay

Crafting, quests, factions, prestige classes, multi-classing, spells, shop & economy, resource pools + regeneration ✓, resource cost system, optional PvP flagging.

### Phase 5 — OLC & Tooling

Online creation commands (@dig/@link/@set/@mob/@area/@item), zone/area management + area reset, telnet negotiation ✓, schema migration ✓, hot-backup ✓, Rhai scripting + triggers + hot-reload ✓, builder help files, **spade**: offline builder (world tree, TOML editor, file browser, mouse/scroll, validator, room grid) ✓, **MCP**: full CRUD for all 15 content categories, simulators (12 tools), validation, search, imm tools (4 online tools) ✓.

### Phase 6 — spade MUD Client & Protocol Expansion

WebSocket bridge, MCCP/GMCP/MXP/MSSP, REST API expansion (14 new imm endpoints), **spade MUD client mode** (output window, ANSI, scroll, input bar, sidebar, clickable names, connection profiles, session management, split mode, dashboard, script console, TOML preview), **MCP**: imm tools (set_stat, load, gecho, advance, stat, heal, damage, kill, revive, set_alignment, set_faction, purge_room, reboot), advanced simulators (regen, level-up, faction change, quest rewards, practice, XP curve), prompts/guided workflows, MCP resources.

---

## Weather & Time System — Implementation Tasks

### Phase 0 — Config & Content Types ✓

- [x] Create `core/src/templates/weather.rs` — `WeatherConfig`, `WeatherConditionDef`, `WeatherEffects`, severity/type enums
- [x] Extend `RoomTemplate` with `no_weather`, `exclude_weather`, `additional_weather`
- [x] Extend `AreaTemplate` with `no_weather`, `weather_matrix`
- [x] Add `weather: Option<WeatherConfig>` to `TemplateRegistry`
- [x] Update `core/src/content.rs` to load standalone `weather.toml`
- [x] Create `content/weather.toml` — conditions, seasons, zone matrices
- [x] Add `TimeConfig` to `core/src/systems/time.rs` and `ServerConfig`
- [x] Add `[time]` section to `content/server.toml`
- [x] Update all TUI/MCP/bin construction sites for new fields

### Phase 1 — Time System (pending)

- [ ] `GameTime` component (`hour`, `minute`, `day`, `season`, `year`)
- [ ] `Season` and `TimePeriod` enums with hour-range mapping
- [ ] `core/src/systems/time.rs` — `advance_time()`, `period_from_hour()`, `TimeEvent` enum
- [ ] Time tick interval in `server/src/game_loop.rs` (configurable via `real_minutes_per_game_hour`)
- [ ] Spawn `GameTime` on startup from DB or config defaults
- [ ] `cmd_time()` — query `GameTime`, format period/day/season/year
- [ ] SQLite `world_time` table + save/load queries
- [ ] `%t` prompt variable in `core/src/prompt.rs`

### Phase 2 — Weather System Core (pending)

- [ ] `WeatherState` component (`base: Option<String>`, `modifier: Option<String>`)
- [ ] `core/src/systems/weather.rs` — `resolve_weather_weights()`, `roll_weather()`, `roll_modifier()`
- [ ] Resolution chain: global season → area zone/matrix → room exclude/additional → roll
- [ ] Weather tick (300s) in `game_loop.rs` — per-zone roll, update `WeatherState`, broadcast severe
- [ ] SQLite `weather_states` table + save/load queries

### Phase 3 — ECS Integration (pending)

- [ ] Spawn `WeatherState` on room entities during world load
- [ ] `%w` prompt variable — query room's `WeatherState`, format description
- [ ] `cmd_weather()` — query `WeatherState`, look up condition descriptions
- [ ] Append weather flavor text to `look` room descriptions on movement

### Phase 4 — Gameplay Effects (pending)

- [ ] Combat: apply `damage_fire`, `damage_lightning`, `ranged_accuracy`, `ranged_attack` modifiers from room weather
- [ ] Attributes: apply `dexterity` modifier from weather on DEX-based checks
- [ ] Weather condition descriptions in room `look` output

### Phase 5 — Tests (pending)

- [ ] `time.rs` — `period_from_hour`, `advance_time` day/season/year rollover, edge cases
- [ ] `weather.rs` — resolution chain, `no_weather` short-circuit, weight normalization, modifier rolling
- [ ] `weather.rs` deserialization — `content/weather.toml` parses cleanly
- [ ] `prompt.rs` — `%t` and `%w` render correctly

### Phase 6 — Documentation (pending)

- [ ] `docs/game_mechanics.md` lines 488-495 — replace stub with reference to ARCHITECTURE.md sections
- [ ] `ARCHITECTURE.md` Development Phases — mark time/weather as implemented
- [ ] `docs/builder_manual.md` — verify weather field docs match implementation
- [ ] `AGENTS.md` — update Phases table

---

## Architectural Debt — P0

Items discovered during architectural review. Each violates an existing invariant, contradicts the
stated architecture, or represents a shallow module where deepening would yield significant leverage.

### Medium: Glob Re-exports and Flat Public API

**Problem:** `core/src/lib.rs` uses `#![allow(ambiguous_glob_reexports)]` to suppress name
collisions from three glob re-exports (`pub use components::*`, `pub use events::*`,
`pub use resources::*`) plus 30+ selective re-exports. `components.rs` adds 11 more glob
re-exports from submodules.

The public API surface of `oxide_core` is the union of every `pub` item across 11 component files,
17 system files, 4 resource files, plus templates, events, scripting, util, and content — hundreds
of types exported flat. Consumer crates cannot tell which submodule a type came from without reading
source. Name collisions exist but are silenced by the `allow` annotation.

**Fix:** Replace glob re-exports with explicit re-exports. Remove
`#![allow(ambiguous_glob_reexports)]`. Each submodule should re-export only the types its
consumers actually need.
