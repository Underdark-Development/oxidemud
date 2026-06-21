# Architecture — MUD Game Engine

## Overview

A modern DIKU-style MUD engine written in Rust. Event-driven, ECS-based, terminal-first with extensible protocol support.

**Stack:** Rust + Tokio + hecs (ECS) + rusqlite + Rhai (scripting)

**Design Philosophy:** Driver/mudlib separation (inspired by LPMud, Evennia). The engine provides networking, ECS, persistence, and scripting — game content (combat, spells, quests) lives in data files and scripts, not in engine code.

---

## State Machine Pattern

Numerous engine subsystems are formalized as explicit state machines with defined states, valid transitions, and events emitted on transition. The pattern is:

```
current_state → trigger_event → validate_transition(a, b) → emit StateChanged { entity, from, to }
```

Each state machine has a `tick()` function that takes current state + context and returns next state. Transitions emit a typed event (e.g. `AiStateChanged`, `CombatStateChanged`) that other systems can subscribe to. Transitions that fail validation are silently ignored.

---

## Cargo Workspace

Five crates under root workspace (tui/spade and mcp planned for Phase 5):

```
mud/
├── Cargo.toml              # workspace root (resolver = "2")
├── core/                   # ECS components, systems, events, resources
│   ├── components/         # hecs Component types
│   ├── systems/            # Game systems (movement, combat, regen, ai)
│   ├── resources/          # Singleton resources + resource pools (Stamina, Mana, Energy, Psi)
│   ├── format/             # Color, RichText, tag parser
│   ├── templates/          # TOML deserialization + TemplateRegistry
│   ├── dice.rs             # DiceRoll XdY+Z parser/roller
│   └── lib.rs
├── server/                 # Network layer + command dispatch
│   ├── telnet/             # IAC parser, NAWS, terminal type
│   ├── cmd/                # Linear Vec<Command> dispatch (trie planned)
│   ├── login/              # LoginFlow state machine (state.rs, handlers.rs, prompt.rs)
│   └── lib.rs
├── data/                   # Persistence layer (SQLite schema, queries, migrations)
├── scripting/              # Rhai engine setup + bindings + sandbox
└── bin/                    # Game server binary (main.rs, commands.rs, init.rs)
```

**Dependency DAG:** `core` depends on nothing. `server` depends on core + data. `data` depends on core. `scripting` depends on core. `bin` depends on core + server + data + scripting.

Content templates (TOML) live at a configurable `content/` path on disk, not in a crate.

---

## Game Loop & Scheduler

Event-driven loop using `tokio::select!` — no fixed tick. The server acquires a write lock on `World` for each branch.

Branches: `shutdown_signal` (flush + exit), `scheduler.next` (run system phase), `event_bus.recv` (dispatch event), `player_input` (execute command).

### Scheduler

Singleton resource maintaining named intervals, each producing a `Pulse` on an mpsc channel:

| Phase | Interval | Description |
|---|---|---|
| `Movement` | 100ms | Direction commands |
| `Combat` | 2s | Attack/damage round |
| `Regeneration` | 6s | HP/mana/stamina regen, effect expiry |
| `Weather` | 5m | Zone weather updates |
| `DirtyFlush` | 5s | Persist dirty entities |

Phases are independent and fire concurrently. Each iterates registered systems sorted by priority (lower runs first).

---

## Systems Architecture

Game logic is organized into systems implementing the `System` trait (run on pulse, handle_event on subscribed events, priority-sorted within phases).

### Built-in Systems

| System | Phase(s) | Pri | Responsibility |
|---|---|---|---|
| `MovementSystem` | Movement | 10 | Direction commands, update `Position`, emit `PlayerMoved` |
| `FollowSystem` | Movement | 20 | Move followers behind leader |
| `EchoSystem` | — (event) | 10 | Broadcast messages to room occupants |
| `CombatSystem` | Combat | 20 | Hit/damage/death rolls |
| `StanceSystem` | Combat | 15 | Apply stance modifiers |
| `AISystem` | Combat | 30 | NPC state machine tick |
| `FormationSystem` | Combat | 25 | Group formation bonuses |
| `RegenSystem` | Regeneration | 10 | Regen HP/mana/resource pools |
| `EffectExpirySystem` | Regeneration | 20 | Tick effect durations |
| `PassiveApplicationSystem` | Regeneration | 30 | Apply/remove passives on login/level-up |
| `WeatherSystem` | Weather | 10 | Zone weather state machine |
| `DirtyFlushSystem` | DirtyFlush | 50 | Persist dirty entities to SQLite |
| `SkillRequirementSystem` | DirtyFlush | 40 | Check skill gates on equipment |
| `GroupCleanupSystem` | DirtyFlush | 45 | Sweep stale group members |
| `CorpseSystem` | DirtyFlush | 60 | Decay corpses, transfer contents |
| `AreaResetSystem` | DirtyFlush | 70 | Area resets past interval |
| `SetBonusSystem` | — (event) | 10 | Evaluate item sets on equip/unequip |
| `QuestProgressSystem` | — (event) | 20 | Update quest objectives |
| `CraftingSystem` | — (event) | 20 | Execute crafting flow |
| `ScriptTriggerSystem` | — (event) | 100 | Run attached Rhai scripts |
| `KeepaliveSystem` | Regeneration | 5 | Detect stale connections |
| `BackupSystem` | DirtyFlush | 80 | Hot-backup SQLite |

Systems stored `PhaseMap<Vec<Box<dyn System>>>`, priority-sorted at registration.

---

## Event Bus

Events dispatch over `tokio::sync::broadcast` channel. Each event carries `EventEnvelope` with `id`, `tag`, `timestamp`, and a `GameEvent` payload.

**Event tags:** `PlayerSaid | PlayerMoved | PlayerAttacked | PlayerDied | PlayerLeveled | MobDied | MobKilled | ItemPickedUp | ItemDropped | ItemWorn | ItemRemoved | SkillUsed | SkillTrained | RoomEntered | QuestUpdated | QuestCompleted | FactionChanged | SetBonusChanged | CorpseDecayed | ContentReloaded | AiStateChanged | CombatStateChanged | PlayerDisconnected | ScriptTrigger | Pulse(Phase)`

Systems declare interest via `subscribed_events()`. Dispatched in priority order; `handle_event()` returning `true` consumes the event. Default is in-band (synchronous under World lock); out-of-band (spawned tokio task) opt-in for logging/analytics.

---

## ECS Component Design

### Spatial
- `Position { room: Entity }` — references a room entity
- `Room { name, description }` — room metadata
- `Exit { direction, dest, flags }` — direction-based exits
- `RoomExits(Vec<Exit>)` — one per room
- `PortalExit { keyword, dest, description, flags }` — keyword-based portal exits
- `RoomPortals(Vec<PortalExit>)` — one per room
- `PortalFlags` bitmask: `PORTAL_HIDDEN`
- `RoomFlags` bitmask: `PORTAL_IN`, `PORTAL_OUT`, `NO_TELEPORT_IN`, `NO_TELEPORT_OUT`
- `VoidRoom` — marker: inescapable room, blocks all movement/recall/teleport
- `Teleportable(bool)` — targetable by player teleport
- `Direction`: North, South, East, West, Up, Down, NE, NW, SE, SW

### Character
- `Player { account_id, prompt }` — player entity, configurable prompt template
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

### In-Game Prompt

The game prompt is a configurable template string stored in `Player.prompt`. Default: `"<%hhp %hm %ss> "` renders as `<450/500hp 120/200m 80/100s>`.

Sent after every command output, before the next `read_line()`. Also sent on room entry, death, level-up, and combat state changes.

| Variable | Source | Example |
|---|---|---|
| `%h` / `%H` | `Health.current` / `Health.max` | `450` / `500` |
| `%m` / `%M` | `Mana.current` / `Mana.max` | `120` / `200` |
| `%s` / `%S` | `Stamina.current` / `Stamina.max` | `80` / `100` |
| `%e` / `%E` | `Energy.current` / `Energy.max` | `50` / `100` |
| `%p` / `%P` | `Psi.current` / `Psi.max` | `30` / `60` |
| `%x` / `%X` | `Experience` / XP to next level | `5200` / `8000` |
| `%r` | `PlayerState.rest` | `Stand` / `Sit` / `Rest` / `Sleep` / `Dead` |
| `%%` | literal `%` | `%` |

Unrecognized variables render as-is (e.g. `%q` → `%q`). Unknown resources display `?` (e.g. `%m` when player has no mana pool).

**Rendering** — `prompt::render(entity, world) -> String` reads the template, walks `Health`, resource pools, `Experience`, `PlayerState` components. Run after command dispatch in the game loop.

**Customization** — `config prompt <template>` writes to `Player.prompt`, persisted to SQLite via dirty tracking. Validates template on set (syntax only, no live values).

### Combat
- `CombatState` — NotInCombat, Engaged { target, round_started, stance }, Fleeing { target, attempts }
- `Damage(i32)` + `DamageType`: Slash, Pierce, Bludgeon, Fire, Cold, Lightning, Acid, Poison, Magic, True
- `Armor { base, bonus }`

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
- `PracticePoints(u32)` — unified pool, gained per level, spent via `train`/`practice`
- `SkillRank(u16)`, `MultiClassInfo { classes: Vec<ClassEntry> }`
- `FactionStanding { standings: HashMap<String, i32> }`
- `QuestLog { active, completed }`, `QuestProgress { quest_id, objectives, started_at }`, `ObjectiveState { index, current, completed }`
- `LearnedRecipes { recipes: Vec<String> }`

### Item Progression
- `SetTracker { active_sets }` — map of active item set bonuses
- `ItemTriggers { on_hit, on_wear, on_remove, on_use }` — trigger skill executions per event
- `TriggerEffect { chance, skill_id, target }`, `TriggerTarget`: Self, Attacker, Room, Random

### Flexible / OLC
- `Attributes(HashMap<String, String>)` — KV store for builder data

### Persistence
- `Dirty` — marker needs DB write, `DbId(i64)` — maps entity to SQLite row

---

## State Machine: RoomState

```
RoomState: Normal, UnderConstruction, Locked, EventActive { event_id }, Destroyed
```

Transitions are triggered by builder commands (@set), area reset events, or scripts. Destroyed rooms block entry and show no description.

## State Machine: CombatState

```
CombatState:
  NotInCombat → { attacked } → Engaged
  Engaged     → { flee }     → Fleeing
  Engaged     → { target_dead } → NotInCombat
  Fleeing     → { success }  → NotInCombat
  Fleeing     → { failed }   → Engaged
```

`CombatSystem` dispatches on `CombatState` each pulse. `CombatStateChanged { entity, from, to }` emitted on every transition.

## State Machine: NPC AI

| From \ To | Idle | Wander | Patrol | Aggro | Combat | Flee | Return |
|---|---|---|---|---|---|---|---|
| Idle | — | timer | timer | aggro_check | — | — | — |
| Wander | timer | — | timer | aggro_check | — | — | — |
| Patrol | timer | timer | — | aggro_check | — | — | — |
| Aggro | — | — | — | — | in_range | — | — |
| Combat | — | — | — | — | — | hp<25% | target_dead |
| Flee | — | — | — | — | cornered | — | dist>safe |
| Return | — | reached_home | reached_home | — | — | — | — |

`AISystem` ticks each NPC on Combat phase. Emits `AiStateChanged { entity, from, to }` per transition. Configuration from mob template: `ai_mode`, `aggro_range`, `flee_threshold`, `patrol_route`, `return_home`.

---

## Combat System

### Attack Flow (Combat pulse every 2s)

1. Check same room (melee) or line-of-sight (ranged)
2. Hit: `d20 + level + str_mod ≥ AC` (melee) / `d20 + level + dex_mod ≥ AC` (ranged) — Natural 1 auto miss; Natural 20 auto crit (×2)
3. Damage: `weapon_damage + str_mod + level/5`
4. Apply damage, emit `PlayerAttacked` / `MobAttacked`
5. If target dead: emit death event, grant XP (`victim.level² × 50`), spawn corpse, clear combat

**Defense:** `AC = 10 + level + dex_mod + armor.total()`

### Damage Types & Resistances

Resistances are multipliers from race, class, equipment, buffs (stacked multiplicatively). Configured via TOML.

| Mult | Meaning |
|---|---|
| `2.0` | Vulnerable |
| `1.0` | Normal |
| `0.5` | Resistant |
| `0.0` | Immune |
| `-1.0` | Absorbed (healed) |

### Weapon Styles

- **Two-handed:** 1.5× STR mod damage, 1.2× speed, no shield
- **Dual-wield:** Primary −4 hit, off-hand −8 hit + 0.5× STR; penalties halved with `ambidexterity`

---

## Corpse & Loot

On death, a corpse entity spawns containing the victim's Inventory + Equipment. Player corpses: `GroupOnly` loot rule, 10min decay. NPC corpses: `Public`, 5min decay. CorpseSystem sweeps expired corpses, transfers items to room floor, emits `CorpseDecayed`. Corpses are transient (no SQL persistence). Lootable via `loot <corpse>` / `get <item> corpse`.

- `Corpse { owner, created_at, decay_secs, lootable_by }`, `LootRule`: Public, GroupOnly, OwnerOnly, Faction

---

## Group & Party

Groups managed by `GroupManager` resource. Each group has a leader, members, loot mode, and formation.

- `Group { leader, members, loot_mode, formation }` — resource entry
- `GroupMember { group_id, role }`, `Following { target, autofollow }`
- `LootMode`: FreeForAll, RoundRobin, MasterLooter; `Formation`: Default, Line, Scattered + others
- Invites expire 30s. XP bonus: +10%/member (max +50%). Group chat prefixed `[Group]`.

### Formations

| Formation | Effect | Min |
|---|---|---|
| `Line` | +1 AC front / −1 AC back | 2 |
| `Scattered` | −2 AC, +10% dodge | 2 |
| `Column` | +1 damage first hit | 3 |
| `Wedge` | +2 attack, −4 AC leader | 3 |
| `Shield Wall` | +2 AC, −2 attack (shield req) | 2 |

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

| Type | Resource | Extra checks |
|---|---|---|
| combat | Stamina | Must be fighting or `use_while_fighting` |
| magic | Mana | Concentration check, component check |
| tech | Energy | Hardware focus in inventory/equipped |
| psionics | Psi | Risk roll (backlash chance) |
| craft | Stamina | Station in room, materials consumed |
| social | None | — |
| general | None/Stamina | — |

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

| Source | Formula |
|---|---|
| Kill mob | `victim.level² × 50 × xp_multiplier` |
| Quest reward | Per-quest definition |
| Explore room | `5 × room.level` (first discovery) |
| Craft item | `item.level² × 10` |
| Group bonus | +10%/member (max +50%) |
| Danger bonus | +25% for aggro mobs |

### Level-Up

When `current_xp >= xp_for_next_level()`: automatic immediate level-up with:

| Effect | Detail |
|---|---|
| HP gain | `hit_die + CON_mod` (min 1), max increases |
| Full heal | HP set to new max |
| Resource pools | Mana/Stamina recalculated via `from_formula(level, int, wis)` / `(level, str, dex)`. Current clamped to new max. |
| BAB & saves | Recalculated per class progression: `bab` (full/medium/poor), saves (good/poor) |
| Practice points | `(2 + WIS_mod + INT_mod).max(1)` added to `PracticePoints` pool |
| Passives | Re-applied via `apply_all_passives()` |
| Event | `PlayerLeveled { entity, old_level, new_level }` emitted |

Multiple levels gained at once if XP far exceeds threshold; the loop processes each increment sequentially. Level-up messages returned to the caller for delivery to the player's connection.

### Death Penalty

`xp_loss = 10%` (configurable), capped at 5 levels' worth. Never de-levels.

---

## Training & Practice System

DIKU-style unified practice pool: one resource funds both stat training and skill practice, giving players a meaningful choice between improving attributes or broadening abilities.

### Practice Points

`PracticePoints(u32)` is a per-character component. Gained on each level-up:

```
gain = (2 + WIS_mod + INT_mod).max(1)
```

Where `WIS_mod = (wisdom - 10) / 2` and `INT_mod = (intelligence - 10) / 2`. Minimum 1 point per level regardless of stats.

### Trainer NPCs

Both `train` and `practice` commands require a trainer NPC in the same room. Trainer NPCs have a `Trainer` component with a `trainer_types` field:

- **Empty list** — general trainer, can train anything
- **Specific types** — restricted to matching categories (e.g. `["attributes"]`, `["combat"]`, `["magic"]`)

Mob templates define `trainer_types` in their TOML; the component is attached at spawn time via `bin/src/init.rs`.

### `train <stat>` — Attribute Training

Increases one of the six core attributes (strength, dexterity, intelligence, wisdom, constitution, charisma).

| Condition | Failure message |
|---|---|
| Trainer with `"attributes"` type in room? | "You can't do that here. Seek out a trainer." |
| PracticePoints >= cost? | "You don't have enough practice points." |
| Stat < MAX (50)? | "Your strength is already at its maximum." |

**Cost:** 5 points (3 for the class's prime attribute — one with highest class modifier).

**On success:** deduct cost, increment stat by 1, set Dirty, persist.

### `practice <skill>` — Skill Practice

Increases a learned skill's rank by 1.

| Condition | Failure message |
|---|---|
| Trainer with matching skill type in room? | "You can't practice that here." |
| PracticePoints >= 1? | "You don't have enough practice points." |
| Skill in LearnedSkills? | "You don't know that skill." |
| Rank < SkillCap.for_level(level)? | "You have mastered that skill for your level." |

**Cost:** 1 point per rank attempt.

**On success:** spend 1 point, rank += 1, set Dirty, persist.

### Skill Caps

Skill ranks bounded by character level:

```
cap = SkillCap.base_cap + SkillCap.per_level × level
```

Default: `base_cap = 5`, `per_level = 5` → `5 + 5 × level`. Class templates define three categories: `class_skills` (full cap), `cross_class_skills` (half cap), `exclusive_skills` (per-class).

### Database

`PracticePoints` persisted in `components_practice_points` table. Migration from legacy `unspent_skill_points` computes retroactive points for existing characters:

```
retro = level × MAX(1, 2 + (wis - 10)/2 + (int - 10)/2) + existing_unspent
```

---

## Item System

Items defined in `content/items/*.toml`. Each template: id, name, description, item_type, subtype, quality, level_requirement, weight, value, flags, gates, weapon/equipment data, triggers, affixes, loot, set membership.

### Item Types

weapon, armor, container, potion, scroll, wand, food, drink, key, quest, treasure, light, furniture.

### Quality Tiers

poor (0.75×), common (1.0×), magic (1.5×), rare (2.0×), legendary (3.0×). Affixed items roll from `content/affixes.toml`.

### Item Restrictions

Gates: allowed_classes, allowed_races, allowed_alignments, requires_skill. Checked on equip/wield + continuously by SkillRequirementSystem (auto-removes on violation).

### Item Triggers

Events: on_hit, on_wear, on_remove, on_use, on_kill, on_damage_taken. Each with chance roll and targeted skill execution.

### Durability & Repair

Weapons lose on hit, armor on being hit. `current == 0` = broken (no stats). Repaired at NPC blacksmith or via repair skill.

### Item Sets

Defined in `content/sets.toml`. Tiers with min_pieces, conditions, effects. `SetTracker` component tracks active sets. Evaluated on equip/unequip by SetBonusSystem.

### Random Loot

Loot entries roll quality between min/max tiers, roll affixes by slot+level. Weighted selection in loot tables.

---

## Mob Templates

NPCs defined in `content/mobs/*.toml`. Zero baked-in knowledge.

**MobTemplate:** id, name, description, level, attributes, health, armor, damage, race, size, equipment, xp_value, loot, ai_mode, aggro config, faction, trainer_types, languages, skills, scripts

- AI modes: idle, wander, patrol, stationary
- Loot tables: entries with item, count range, chance, optional treasure_class references
- Spawns via area template `[mobs.<area>.<room>]` with count + respawn timer
- AreaResetSystem populates/respawns mobs on area reset

---

## Shop & Economy

**Currency:** Three-tier decimal in copper pieces. Wallet: `{ copper, banked_copper }` — both u64.

| Coin | Value |
|---|---|
| Copper (cp) | 1 |
| Silver (sp) | 100 cp |
| Gold (gp) | 10,000 cp (100 sp) |

**NPC Shops:** `Shop { name, buy_rate, sell_rate, inventory, currency, restock_secs }`. Shop templates in `content/shops/*.toml` instantiated on NPC entities at server start.

**Pricing:** `buy_price = base × sell_rate × reputation_mult`, `sell_price = base × buy_rate × reputation_mult`. Reputation tiers: adored (0.80), friendly (0.90), neutral (1.00), unfriendly (1.25), hostile (1.50).

---

## Deity System

Deities are template entities defined in `content/deities/*.toml`. A character may adopt a deity during creation or in-game. Deities grant optional effects when prayed to, subject to a cooldown.

### DeityTemplate

```
DeityTemplate {
    id: String,
    name: String,
    description: String,
    alignment: Option<String>,       // deity's own alignment
    symbol: String,
    favored_weapon: Option<String>,
    tenets: Vec<String>,
    domains: Vec<String>,            // War, Nature, Trickery, Knowledge, Life, etc.
    allowed_races: Vec<String>,      // empty = all races
    allowed_classes: Vec<String>,    // empty = all classes
    allowed_alignments: Vec<String>, // empty = all alignments
    prayer_effect: Option<PrayerEffect>,
}
```

### PrayerEffect

```
PrayerEffect {
    buff_id: String,        // references a PassiveDef or ActiveEffect template
    duration_secs: u64,     // how long the effect lasts
    cooldown_secs: u64,     // minimum time between prayers
    description: String,    // flavor text on pray
}
```

### Class Deity Policy

Class templates define a `deity_policy` field controlling character creation behavior:

| Policy | Meaning |
|---|---|
| `any` | Player may choose any deity or none (default) |
| `none` | Player may not have a deity |
| `required` | Player must choose a deity |
| `subset([ids])` | Player must choose from this explicit list |

### Pray Command

```
pray              — pray to your deity (if you have one)
pray <deity>      — pray to a specific deity (shrine required in room)
pray <target>     — cleric/paladin heal ally via deity channel
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

**FactionDef:** id, name, description, starting_standing, min/max, ranks, relationships, aggro. Standing changes propagate to related factions via multiplier.

| Source | Delta |
|---|---|
| Kill aggro member | +5 |
| Kill ally member | −10 |
| Quest completion | Per quest |
| Attack faction member | −50 to −200 |

Rank resolved by highest threshold. Aggro checked on room entry.

---

## Command System

Commands are stored as a flat `Vec<Command>` with linear prefix matching (trie planned). Each command:

- `Command { name, aliases, access (AccessLevel), handler: fn(&mut World, &mut Connection, args) -> CommandResult }`

### Player Commands

`look/l`, directional (`n/s/e/w/u/d/ne/nw/se/sw`), `enter`, `say`, `tell/whisper`, `reply/r`, `shout`, `emote/:`, `channels`/`channel`, `kill`, `get/drop`, `put/in`, `give/to`, `inventory/i`, `equipment/eq`, `wear/wield/remove`, `examine/exam`, `use/cast`, `score/stats`, `train`, `practice`, `craft`, `recipes`, `repair`, `stance`, `group`, `follow`, `quests/quest/quest abandon`, `factions/faction`, `sit/rest/sleep/wake/stand`, `loot`, `time`, `weather`, `motd`, `help/?`, `who`, `config`, `@prestige`, `@multi_class`

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

## Time & Weather

Game time is independent of real time (default 1:60 ratio). Persisted in SQLite. Seasons affect daylight, weather tables, temperature, and mob spawns.

Weather tracked per `weather_zone`. Updated by `WeatherSystem` on Weather phase. Conditions: Rain/Storm (−2 fire, +2 lightning), Fog (−25% ranged), Snow/Blizzard (−1 DEX), Strong wind (−2 ranged attacks).

### Weather Probability Matrix

| Current \ Next | Clear | Cloudy | Rain | Storm | Fog | Snow |
|---|---|---|---|---|---|---|
| Clear | 60% | 30% | 5% | 0% | 5% | 0% |
| Cloudy | 20% | 40% | 25% | 5% | 10% | 0% |
| Rain | 10% | 20% | 40% | 20% | 5% | 5% |
| Storm | 5% | 10% | 30% | 30% | 5% | 20% |
| Fog | 30% | 30% | 10% | 0% | 30% | 0% |
| Snow | 10% | 10% | 5% | 5% | 5% | 65% |

Season modifiers shift probabilities (more rain/storm in spring, more snow in winter).

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

**Schema:** SQLite tables mirror component types: `entities` table + `components_*` tables per component type. Startup: load entities, populate components, delete stale.

**WriteBatch:** `{ entity_id, entity_type, components: Vec<ComponentRow> }`. Type-safe queries in `data/src/queries.rs`. Migrations via `PRAGMA user_version`.

**WAL config:** `PRAGMA journal_mode = WAL, foreign_keys = ON, busy_timeout = 5000, synchronous = NORMAL`. Connection in `Arc<parking_lot::Mutex<Connection>>`.

**Backup:** Hot backup via SQLite online backup API. Scheduled hourly by `BackupSystem`. Stored in `data/backups/`, retain 7 daily + 4 weekly.

---

## Content Loading & Hot-Reload

All game content in TOML under `content/` (configurable path). Scanned at startup, deserialized via serde, cross-referenced, built into `TemplateRegistry` (behind `Arc<RwLock<...>>`).

**Directory layout:** `content/{areas, mobs, items, races, classes, skills, scripts, recipes, quests, factions, shops, help, deities}/` + `affixes.toml`, `sets.toml`, `languages.toml`, `socials.toml`, `treasure_classes.toml`. Rooms live in individual files under `content/areas/<area_id>/rooms/<room_id>.toml`.

Hot-reload uses `notify` crate. On change: re-parse, validate, atomic-swap in registry, emit `ContentReloaded`.

---

## Zone & Area System

Areas group rooms into named, managed zones. Each area is a directory under `content/areas/<area_id>/` containing an `area.toml` (metadata) and a `rooms/` subdirectory with one TOML file per room.

**AreaTemplate:** id, name, description, level_range, flags, weather_zone, reset_interval_secs, credits. **RoomTemplate:** id, area, name, description, exits, portals, flags, content, spawn (optional — label, description, allowed_classes for character creation).

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

`mud.toml` (override with `--config` flag or `MUD_CONTENT` env var). Sections: server (host, port, max_players), database (path), game (name, motd, start_room, max_level, content_dir), combat, training, multi_classing, item_sets, logging.

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
help            — topic index
help <topic>    — show topic
help <partial>  — keyword search
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

| Step | Prompt | Validation |
|---|---|---|
| **Name** | "Enter your character's name:" | 3–16 letters, hyphens, apostrophes; unique |
| **Race** | Pick from `content/races/*.toml` | Valid key |
| **Class** | Pick from filtered list | Race ∈ class.allowed_races ∩ class ∈ race.allowed_classes |
| **Gender** | "Choose gender:" (male/female/neutral/other) | Gender ∈ race.allowed_genders; if `other`, prompt for pronouns |
| **Attributes** | Point-buy (27pts), standard array (15/14/13/12/10/8), or roll 4d6 drop lowest | Clamped [3, 25]; race base + class mod applied final |
| **Alignment** | Pick from 3×3 lawful–chaotic × good–evil grid | Race/class may restrict |
| **Deity** | Pick from `content/deities/*.toml` (or none) | Class deity policy: required/optional/prohibited/subset; alignment gate |
| **Skills** | Pick from class skill pool | Prefix match against id/name |
| **Appearance** | Height, weight, build, hair, eyes, skin | Bounded by race appearance_bounds |
| **Description** | Multi-line free text (type `.` to finish) | — |
| **Spawn** | Choose starting location | Area spawn entries filtered by race/class/alignment |
| **Confirm** | Full summary → accept? | Save to DB + spawn entity |

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

### Script Lifecycle

Startup: scan `scripts/` dir, parse `.rhai` into AST, cache. Register event bindings (template `[[scripts]]` entries → event tag → AST). Runtime: event fires → `ScriptTriggerSystem` looks up binding → fresh engine + `ScriptCtx` → execute AST inline under World lock.

`ScriptCtx`: entity, actor, target, world_ref. EntityHandle methods: name(), id(), room(), health(), level(), is_player(), say(), etc. RoomHandle: name(), echo(), entities(), exits(). WorldHandle: spawn_mob/remove_entity/grant_xp/grant_recipe/etc.

### Security Model

| Concern | Protection |
|---|---|
| Infinite loops | max_operations = 50k, max_call_levels = 32 |
| Memory | max_string_size = 10k, max_arrays = 100, max_map = 50 |
| File access | Resolver reads only `<content_dir>/scripts/` |
| Network | No socket bindings |
| CPU | Inline but ~1ms at 50k ops |

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

| Feature | Phase | Requires |
|---|---|---|
| ANSI 16-color | 0 | — |
| NAWS | 1 | Telnet |
| UTF-8 | 1 | Telnet |
| 256-color | 2 | MTTS |
| GMCP (Room, Char) | 6 | Telnet |
| MCCP | 6 | Telnet |
| MXP | 6 | Telnet + lock |
| WebSocket | 6 | HTTP server |
| REST API | 6 | HTTP server |
| MSSP | 6 | Telnet |

---

## spade — Builder TUI & MUD Client

**spade** is the terminal-based builder TUI and MUD client. Named after the `@dig` OLC command — a spade is what you dig with.

### Philosophy

- **Single tool** for all builder workflows: world editing, validation, content browsing, live testing
- **Mouse-first navigation** with full keyboard fallback
- **Offline + online modes:** edit TOML files directly or connect to running server
- **Data-driven** help screens, keybindings, sidebar commands

### Modes

| Mode | Invocation | Description |
|---|---|---|
| Builder (offline) | `spade` or `spade --mode offline` | TOML editor, world tree, validator, file browser |
| MUD client (online) | `spade --mode online` | Full client with scrollable output, ANSI, clickable names |
| Split | `spade --mode split` (F9) | Builder 50% | MUD client 50% |
| Connection profile | `spade connect <host> <port>` | Quick-connect with saved profile |

### Screens & Panels (Builder Mode)

| Key | Screen | Description |
|---|---|---|
| Ctrl+1 | World Tree | Collapsible area/room/mob/item tree |
| Ctrl+2 | Template Editor | TOML field form editor |
| Ctrl+3 | Room Graph | ASCII room map with mouse navigation |
| Ctrl+4 | Entity Inspector | Detail table with scrolling |
| Ctrl+5 | Command Palette | Fuzzy-searchable commands |
| Ctrl+6 | Live Dashboard | Server status gauges (online only) |
| Ctrl+7 | Validation Panel | Error/warning list with jump-to-source |
| Ctrl+8 | File Browser | Content directory tree |
| Ctrl+9 | Script Console | Inline Rhai REPL |

Layout: World Tree (left), Editor/Inspector/Preview (center), Validator Panel (right). Status bar (bottom): mode, active screen, current file, mouse state.

### UI Design Principles

- **Focus-first navigation:** Tab cycles pane focus. Focused pane has highlighted border (bright white/cyan).
- **Consistent colors:** Panel borders Cyan, focused BrightWhite, selected Yellow bg, errors Red, warnings Yellow, success Green.
- **Breadcrumb trail:** `Areas > midgaard > rooms > square`
- **Empty states:** Helpful prompts like "No rooms yet. Press @dig to create one."
- **Confirmation dialogs** before destructive actions.
- **Toast notifications** (bottom-right, 3s auto-dismiss).

### MUD Client Mode

Regions: Sidebar (22 cols, collapsible commands), Output window (scrollable ANSI), Input bar (full width, history/autocomplete), Status bar (mode, connection, player count).

**Sidebar:** Collapsible sections with mouse-clickable command buttons. Sections: Movement, Info, Admin, Building, Session. Click behavior: if takes args → pre-type command prefix; if no args → send immediately; confirm dialog for destructive actions.

**Clickable Names:** Detection via GMCP Room.Info (structured data) or heuristic regex on look output. Players shown in brightcyan, mobs in yellow with underline. Right-click opens context menu (Stat, Tell, Goto, Force, Kick, Freeze, Ban, Copy name/account).

**Output Window:** `OutputWindow { buffer: VecDeque<OutputLine>(5000), scroll, ansi_parser, clickable_ranges }`. ANSI escape codes → ratatui Style. 5000-line buffer, scroll wheel, search (/), line numbers toggle (Ctrl+L), timestamps toggle (Ctrl+T), auto-scroll (pauses on manual scroll).

**Input Bar:** Command history (↑↓, 200 entries, persisted), Tab autocomplete, Ctrl+R reverse search, Ctrl+U clear, Ctrl+A/E line bounds.

### Mouse Support

Left click (select), double left click (open), right click (context menu), scroll wheel (scroll), click tab (switch screen), click panel border (resize split). Disabled while input focused. Toggle with Ctrl+M.

### Scroll Support

Each pane has `ScrollState { offset, visible_lines, total_lines }`. Scrollbar rendered right edge. Scroll percentage shown when scrolled (`-- 55% --`). Wheel, PgUp/PgDn, ↑↓, Home/End all work in focused pane.

### Help Screen

Modal overlay (Ctrl+H / ?), center 70% of terminal. Data-driven sections. Dismissed by Escape.

### Session Management (MUD Mode)

`SessionState`: Disconnected, Connecting, Negotiating, LoggingIn(n attempts), Playing. Connection profiles in `~/.config/spade/profiles.toml` (host, port, mode=telnet/websocket, username, tls).

### Keybinding Summary

| Key | Action | Scope |
|---|---|---|
| Tab / Shift+Tab | Cycle pane focus | All |
| Enter | Open / confirm | All |
| Escape / Ctrl+Q | Go back / close modal | All |
| Ctrl+H / ? | Toggle help screen | All |
| Ctrl+D | Quit spade | All |
| / | Search / filter | All |
| F5 | Validate / refresh | All |
| F10 | Toggle rich TOML preview | Builder |
| Ctrl+1-9 | Switch screen | Builder |
| Ctrl+S | Save (offline) / Send (online) | Builder |
| Ctrl+Z | Undo | Editor |
| Ctrl+C/V | Copy / paste | All |
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
| ↑↓ | Navigate / scroll | All |
| PgUp/PgDn | Page scroll | All |
| Home/End | Jump to top / bottom | All |
| Left click | Select | All |
| Double left click | Open | All |
| Right click | Context menu | All |
| Scroll wheel | Scroll | All |
| Shift+click | Select range | Lists |
| Ctrl+click | Toggle selection | Lists |

---

## MCP Server — AI Agent World-Building

MCP server exposing the full content toolset to AI agents (Claude, etc.). Agents read, create, edit, and validate game world data via natural language.

### Modes

| Mode | Trigger | Data Source | Write |
|---|---|---|---|
| Offline | `mcp` (default) | TOML files | Atomic write (temp + rename) |
| Online | `mcp --db <path>` | SQLite DB | REST bridge to game server |

Offline mode is primary — AI agents edit TOML files, validation runs locally, human reviews before game loads.

### Transport

Primary: stdio (MCP standard). Future: SSE (HTTP) for remote connections.

### Tools

| Tool | Write | Description |
|---|---|---|
| `list_areas` / `get_area` | No | List areas / get details |
| `create_area` / `update_area` / `delete_area` | Yes | Area CRUD |
| `list_rooms` / `get_room` | No | Room listing / detail |
| `create_room` / `update_room` / `delete_room` | Yes | Room CRUD |
| `link_rooms` / `add_portal` / `remove_portal` | Yes | Room connections |
| `list_mobs` / `get_mob` | No | Mob listing / detail |
| `create_mob` / `update_mob` / `delete_mob` | Yes | Mob template CRUD |
| `list_items` / `get_item` | No | Item listing / detail |
| `create_item` / `update_item` / `delete_item` | Yes | Item template CRUD |
| `list_quests` / `get_quest` | No | Quest listing / detail |
| `create_quest` / `update_quest` / `delete_quest` | Yes | Quest CRUD |
| `list_recipes` / `get_recipe` | No | Recipe listing / detail |
| `create_recipe` | Yes | Recipe creation |
| `list_factions` / `get_faction` | No | Faction listing / detail |
| `create_faction` | Yes | Faction creation |
| `list_shops` / `get_shop` | No | Shop listing / detail |
| `create_shop` | Yes | Shop creation |
| `list_skills` / `get_skill` | No | Skill listing / detail |
| `get_race` / `get_class` | No | Race/class templates |
| `validate` | No | Content validation (scope: all/area/type) |
| `search` | No | Fuzzy search all content |
| `get_stats` | No | Content summary statistics |

Each tool follows MCP schema (name, description, input JSON Schema). Uses `rmcp` (Rust MCP SDK v1.7) with `#[tool]` macros for declarative definition.

### Resources

MCP content URIs: `content://areas/`, `content://areas/{key}`, `content://areas/{key}/rooms/{room_key}`, `content://mobs/{key}`, `content://items/{key}`, `content://skills/{key}`, `content://races/{key}`, `content://classes/{key}`, `content://quests/{key}`, `content://recipes/{key}`, `content://factions/{key}`, `content://shops/{key}`, `content://validation/`, `content://stats/`.

### Prompts (Guided Workflows)

`create_area_flow` (guided area creation), `review_content` (validation + suggestions), `balance_encounter` (mob vs area tier analysis), `design_quest_chain` (prerequisites, objectives, branching, rewards).

### Crate Layout

```
mcp/
├── Cargo.toml
└── src/
    ├── main.rs         # Entry: --mode, --port, --content-path
    ├── server.rs       # McpServer: stdio transport, dispatch
    ├── tools/          # Tool implementations by content type
    ├── resources/      # Resource providers (templates, validation, stats)
    └── prompts/        # Prompt templates (builder, reviewer)
```

### DAG

`mcp` depends on `core` (types) only. Loads TOML files directly for offline mode, HTTP for online. No dependency on server/data/scripting/bin.

---

## Development Phases

### Phase 0 — Foundation
- [x] Cargo workspace & crate skeleton (5 crates)
- [x] Core types (Room, Exit, Direction, entity management)
- [x] Tokio TCP listener with telnet negotiation
- [x] Basic ECS world with hecs
- [x] Raw line-in/line-out to connected players
- [x] Resource pools (Stamina, Mana, Energy, Psi) in core/src/resources/
- [x] Unit tests (49 across all crates)
- [x] Encryption deployment guide (stunnel)
- [x] Void room (inescapable — VoidRoom marker)
- [x] CLI config (--port/--host flags)
- [x] Graceful shutdown (SIGINT/SIGTERM)
- [x] Player spawn — connects into void room with Position

### Phase 1 — World & Movement
- [x] ConnectionRegistry — HashMap<Entity, Sender> for room broadcasts
- [x] say — room broadcast
- [x] look — rooms, occupants, visible exits
- [x] Movement commands — n/s/e/w/u/d + ne/nw/se/sw + long forms
- [x] Void room movement check
- [x] Auto-look on room entry + enter/leave broadcasts
- [x] Player cleanup — despawn + registry remove on disconnect
- [x] core::format module — Color, Modifier, RichText, parse_tags()
- [x] Connection feature flags — Ansi, ExtendedColor, Blink
- [x] ANSI color conventions
- [x] Unit tests — movement, void blocking, room broadcast, ANSI

### Phase 2 — Character System
- [x] Connection state machine (LoginFlow in server/src/login/)
- [x] Account creation (username + password, argon2 hashing)
- [x] Login flow (banner/MOTD → username → password)
- [x] Character select screen (list existing + create new)
- [x] Character creation wizard (name → race → class → gender → attributes → alignment → deity → skills → appearance → description → spawn → confirm)
- [x] Race→class filtering in creation wizard
- [x] characters SQLite table + schema migration
- [x] TOML race/class template loading
- [x] Unified SkillDef + skill_type enum
- [x] Expanded RaceTemplate with constraints (allowed_genders, appearance_bounds, age defaults)
- [x] Expanded ClassTemplate with constraints (+ deity_policy)
- [x] Cross-reference validation pipeline
- [x] Derived indices in TemplateRegistry
- [x] Auto-grant racial abilities + class auto-skills
- [x] Starting room spawn on character confirm
- [x] motd command
- [ ] Gender component + DB column + creation step
- [ ] Appearance component + DB table + creation step + race-bounded validation
- [ ] Age component
- [ ] Deity component + template system + creation step + in-game adopt
- [ ] DeityTemplate loader + validation
- [ ] pray command (basic: own deity + optional target heal)
- [ ] Class deity_policy enforcement in creation

### Phase 3 — Combat & Equipment
- [x] Health, Damage components
- [x] Combat system (attack/damage rolls)
- [x] Damage type system (resistance/vulnerability)
- [x] Weapon styles (two-handed, dual-wield)
- [x] Equipment, Inventory components
- [x] Weapon/armor items with restriction gates
- [x] NPC mobiles with basic AI
- [x] Mob template system
- [x] Stance subsystem
- [ ] Passive system (login/level-up application)
- [ ] Skill cap system
- [ ] Training system
  - [x] PracticePoints component + unified pool
  - [ ] Trainer NPC system (spawn attachment, room check)
  - [ ] train command (attributes)
  - [ ] practice command (skills)
  - [ ] score/stats command
  - [ ] Class progression (BAB, saves) on level-up
  - [ ] Resource pool recalculation on level-up
  - [ ] PlayerLeveled event emission
  - [ ] Migration: retroactive practice points
- [ ] Item triggers
- [ ] Item sets TOML + SetTracker
- [ ] Random loot quality/affix rolling

### Phase 4 — Advanced Gameplay
- [ ] Crafting system
- [ ] Quest system
- [ ] Faction system
- [ ] Prestige class system
- [ ] Multi-classing system
- [ ] Spell system (unified in Skill System)
- [ ] Shop & Economy
- [x] Resource pools + regeneration
- [ ] Resource cost system
- [ ] Optional PvP flagging

### Phase 5 — OLC & Tooling
- [ ] Online creation commands (@dig, @link, @set, @mob, @area, @item)
- [ ] Zone/area management, area reset system
- [ ] Telnet negotiation (IAC state machine, NAWS, terminal type)
- [ ] Schema migration system
- [ ] Hot-backup system
- [ ] Rhai scripting engine integration
- [ ] Scriptable triggers & events
- [ ] Hot-reload all content types
- [ ] Builder-created help files
- [ ] **spade crate scaffold** (Cargo.toml, main.rs, crossterm init)
- [ ] **spade offline builder mode** — world tree, TOML editor, file browser
- [ ] **spade components** — ScrollState, Tree, Tabs, Table, Form, Modal, ContextMenu
- [ ] **spade help screen** — data-driven modal
- [ ] **spade mouse support** — click/double-click/right-click/scroll
- [ ] **spade scroll support** — per-pane ScrollState, scrollbar
- [ ] **spade validator panel** — cross-reference diagnostics
- [ ] **spade room graph** — ASCII map view
- [ ] **MCP crate scaffold** — Cargo.toml, server.rs, stdio transport
- [ ] **MCP offline mode** — area/room/mob/item CRUD
- [ ] **MCP validate tool** — run validator, return diagnostics

### Phase 6 — spade MUD Client & Protocol Expansion
- [ ] WebSocket bridge (JSON MMCC frames, ANSI→HTML)
- [ ] MCCP, GMCP (Room, Char, Comm, MGK), MXP, MSSP
- [ ] REST API endpoints
- [ ] **spade MUD client mode** — output window, ANSI parser, scroll buffer
- [ ] **spade input bar** — history, autocomplete, Ctrl+R
- [ ] **spade sidebar** — collapsible commands, mouse-clickable
- [ ] **spade clickable names** — GMCP + heuristic, context menu
- [ ] **spade connection profiles** — profiles.toml, connect dialog
- [ ] **spade session management** — reconnect, state machine, login flow
- [ ] **spade split mode** — builder + client side by side
- [ ] **spade live dashboard** — server status gauges
- [ ] **spade script console** — inline Rhai REPL
- [ ] **spade syntax-highlighted TOML preview** — F10
- [ ] **MCP quest/recipe/faction/shop CRUD**
- [ ] **MCP search tool** — fuzzy search
- [ ] **MCP resources** — templates, validation, stats
- [ ] **MCP online mode** — REST bridge to game server
- [ ] **MCP prompts** — guided workflows
- [ ] Performance profiling & optimization

---

## Implementation Plan: Level-Up & Training

### Phase 1 — Core PracticePoints Component
- [x] Add `PracticePoints(pub u32)` component to `core/src/components/character.rs`
- [x] Grant points in `award_xp()`: `(2 + WIS_mod + INT_mod).max(1)` per level, replacing old `skill_points` formula
- [x] Remove `unspent_points` field from `LearnedSkills`
- [x] Return level-up messages from `award_xp()` as `Vec<String>`, caller delivers
- [x] Update `save_player_progress()` to persist/load `PracticePoints` instead of `unspent_skill_points`

### Phase 2 — Database Migration
- [x] Bump `VERSION` to 15, add `components_practice_points` table
- [x] Migration 15: for each existing character, compute retroactive points:
      `level × MAX(1, 2 + (wis-10)/2 + (int-10)/2) + existing unspent_skill_points`
- [x] Add `save_practice_points()` and `load_practice_points()` queries
- [x] Update `save_player_progress()` to save new component

### Phase 3 — Room Entity Finder
- [x] Add `entities_in_room(world, room) -> Vec<Entity>` utility in `core/src/`

### Phase 4 — Trainer NPC System
- [x] Attach `Trainer` component in mob spawn code (`bin/src/init.rs`) when `trainer_types` is non-empty
- [x] Create trainer mob template (`content/mobs/trainer.toml`, friendly, idle, `trainer_types = ["attributes"]`)

### Phase 5 — Commands
- [x] `train` command: validates trainer proximity, PracticePoints, stat bounds; deducts cost, increments stat
- [x] `practice` command: validates trainer proximity, PracticePoints, skill known, skill cap; deducts 1, increments rank
- [x] `score/stats` command: character sheet (level, XP, HP, pools, all 6 attrs, BAB, saves, PracticePoints)
- [x] Register all three commands in CommandDispatch

### Phase 6 — Class Progression
- [x] Add `bab`, `fort_save`, `ref_save`, `will_save` fields to `ClassTemplate`
- [x] Recalculate `CombatStats` on each level-up using class progression formulas
- [x] Fix `get_hit_die()` to use entity's actual class via `TemplateRegistry` lookup
- [x] Update class TOML files (warrior: full/mage: poor BAB, matching saves)

### Phase 7 — Enhanced Level-Up Effects
- [x] Recalculate Mana/Stamina pools via `from_formula()` on each level-up
- [x] Emit `PlayerLeveled` event from `award_xp()`

### Phase 8 — Tests
- [x] Unit: practice points grant formula
- [x] Unit: `train` command (success + all failure cases)
- [x] Unit: `practice` command (success + all failure cases)
- [x] Unit: `entities_in_room()` utility
- [x] Unit: class progression (BAB/saves at various levels)
- [x] Unit: migration retroactive calculation
- [x] Integration: full level-up cycle
