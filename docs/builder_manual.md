# Builder Manual

This manual is for world builders and game designers. It describes the structure of game content templates, online creation (OLC) command references, and validation rules.

---

## Content Directory Layout

All game content templates live in TOML files under the `content/` directory. Changing or adding files updates the game content.

```
content/
├── areas/
│   └── <area_id>/
│       ├── area.toml                 # Area metadata
│       ├── rooms/
│       │   ├── <room_id_1>.toml      # Room template 1
│       │   └── <room_id_2>.toml      # Room template 2
│       └── areas/                    # Optional nested sub-areas
│           └── <sub_area_id>/
│               ├── area.toml         # Sub-area metadata
│               └── rooms/            # Sub-area room templates
├── mobs/
│   └── <mob_id>.toml                 # NPC template files
├── items/
│   └── <item_id>.toml                # Item template files
├── races/
│   └── <race_id>.toml                # Race characteristics
├── classes/
│   └── <class_id>.toml               # Class details and skill tables
├── skills/
│   └── <skill_id>.toml               # Custom spells and combat arts
├── recipes/
│   └── <recipe_id>.toml              # Crafting blueprints
├── quests/
│   └── <quest_id>.toml               # Quests and rewards
├── factions/
│   └── <faction_id>.toml             # Faction relationships and alignments
├── shops/
│   └── <shop_id>.toml                # Shop vendor configurations
├── deities/
│   └── <deity_id>.toml               # Deities, domains, and blessings
├── help/
│   └── <help_id>.toml                # Builder-defined help files
├── scripts/
│   └── <script_id>.rhai              # Custom Rhai scripts
├── affixes/
│   └── <affix_id>.toml               # Item affix definitions (prefix/suffix)
├── sets/
│   └── <set_id>.toml                 # Item set definitions and tier bonuses
├── passives/
│   └── <passive_id>.toml             # Passive trait/ability definitions
├── stances/
│   └── <stance_id>.toml              # Combat stance definitions
├── languages.toml                    # Languages granted at creation
├── socials.toml                      # Social emotes definitions
└── weather.toml                      # Weather systems and season climate matrices
```

---

## Core Template Schemas

### Areas (`AreaTemplate`)

Stored at `content/areas/<area_id>/area.toml`. Defines a geographic zone of rooms. Areas support parent/sub-area nesting via the optional `<area_id>/areas/<sub_area_id>/` subfolder hierarchy. Sub-areas automatically resolve using dot-notated IDs (e.g., `parent_id.sub_id`).

```toml
id = "midgaard"
name = "City of Midgaard"
description = "A bustling trade hub in the center of the world."
level_range = [1, 10]
weather_zone = "temperate"
reset_interval_secs = 900
credits = "Designed by Builder Staff"
flags = ["city", "peaceful"]
```

**Weather fields:**

| Field            | Type   | Default | Description                                                     |
| ---------------- | ------ | ------- | --------------------------------------------------------------- |
| `weather_zone`   | string | —       | Reference a zone defined in `content/weather.toml`              |
| `weather_matrix` | table  | —       | Inline per-season probability matrix (overrides `weather_zone`) |
| `no_weather`     | bool   | `false` | Disable weather for entire area (all rooms inherit this)        |

**Inline weather matrix example:**

```toml
[weather_matrix.spring]
clear = 40
rain = 30
fog = 20
strong_wind = 10

[weather_matrix.summer]
clear = 50
rain = 20
storm = 20
```

### Rooms (`RoomTemplate`)

Stored at `content/areas/<area_id>/rooms/<room_id>.toml`.

```toml
id = "temple_square"
area = "midgaard"
name = "Temple Square"
description = "A large open square paved with granite cobblestones. A grand temple stands to the north."
allow_revive = true    # Ghosts can revive here without their corpse
flags = ["portal_in"]

# Exits connect adjacent rooms
[[exits]]
direction = "north"
dest = "midgaard.temple_entrance"

[[exits]]
direction = "south"
dest = "midgaard.market_square"

# Doors: add door state fields to an exit
[[exits]]
direction = "east"
dest = "midgaard.armory"
is_door = true
is_closed = true
is_locked = true
key_id = "iron_key"

# Portals represent keyword-based movement
[[portals]]
keyword = "archway"
dest = "midgaard.temple_altar"
description = "A shimmering stone archway."
flags = ["hidden"]

# Room content: mobs, items, and sets that spawn here
[content]
mobs = ["city_guard", "shopkeeper"]
items = ["torch", "bench"]
sets = [{ mob = "city_guard", items = ["steel_shortsword", "chainmail_chestpiece"] }]

# Spawn configurations for characters created here
[spawn]
allowed_classes = ["cleric", "paladin"]
description = "The sanctuary spawns holy acolytes."
```

#### Room Weather Fields

Rooms can override, extend, or suppress the area's weather conditions.

| Field                | Type  | Default | Description                                                   |
| -------------------- | ----- | ------- | ------------------------------------------------------------- |
| `no_weather`         | bool  | `false` | Disable weather for this room (indoors, underground)          |
| `exclude_weather`    | list  | `[]`    | Remove specific conditions from the area's weather set        |
| `additional_weather` | table | `{}`    | Add conditions not in the area's matrix (probability weights) |

**Examples:**

```toml
# Dense forest — blocks wind but keeps other weather
exclude_weather = ["strong_wind"]

# Cave entrance — adds fog chance beyond what the area provides
additional_weather = { fog = 15 }

# Indoor temple — no weather at all
no_weather = true
```

Weather resolution chain: global defaults → area matrix → room exclude → room additional → roll. If `no_weather = true` on the area, all rooms inherit it.

#### Exit Door Fields

Exits support optional door state fields. When `is_door = true`, the exit behaves as a door that can be opened, closed, locked, and unlocked.

| Field         | Type   | Default | Description                                                                                                          |
| ------------- | ------ | ------- | -------------------------------------------------------------------------------------------------------------------- |
| `direction`   | String | —       | Direction name (`north`, `south`, `east`, `west`, `up`, `down`, `northeast`, `northwest`, `southeast`, `southwest`). |
| `dest`        | String | —       | Destination room in `area_id.room_id` format.                                                                        |
| `is_door`     | Bool   | `false` | Whether this exit has a door.                                                                                        |
| `is_closed`   | Bool   | `false` | Whether the door starts closed (blocks movement).                                                                    |
| `is_locked`   | Bool   | `false` | Whether the door starts locked.                                                                                      |
| `key_id`      | String | —       | Item template ID of the key that locks/unlocks this door.                                                            |
| `flags`       | Array  | `[]`    | Exit flags: `hidden` (not shown in exit list).                                                                       |
| `description` | String | —       | Extra description seen when looking in this direction.                                                               |

The `open`, `close`, `lock`, and `unlock` commands operate on door exits. Locking/unlocking requires the matching key item in the player's inventory.

#### Room Content

Rooms can define a `[content]` section for mob/item spawns and equipment sets:

```toml
[content]
mobs = ["city_guard", "shopkeeper"]       # Mob template IDs to spawn on reset
items = ["torch", "bench"]                # Item template IDs to spawn on reset
sets = [{ mob = "city_guard", items = ["steel_shortsword"] }]  # Equipment sets for mobs
```

### Mobs (`MobTemplate`)

Stored under `content/mobs/`.

```toml
id = "city_guard"
name = "a city guard"
description = "A stout guard in chainmail patrols the streets."
short_desc = "A guard is standing here."   # Displayed in room listings
level = 5
race = "human"
size = "medium"
armor = 12                                  # Base armor class (AC)
damage = "1d8+2"                            # Natural weapon damage (dice notation)
damage_type = "bludgeon"                    # Natural weapon damage type
xp_value = 250                              # XP awarded on kill

# AI behavior mode: idle, wander, patrol, aggro, combat, flee, return
ai_mode = "patrol"

# Patrol route (room IDs in area.room format) — NPC follows waypoints
patrol_route = ["midgaard.guard_post", "midgaard.temple_square", "midgaard.market_square"]

# Wander settings (active when ai_mode = "wander")
wander_rooms = ["midgaard.temple_square", "midgaard.market_square"] # Specific rooms to wander (empty = any in area)
wander_area = true                                                  # Wander anywhere in current area if true

# Friendly NPCs don't aggro players
friendly = false

# Banker NPCs offer deposit/withdraw/balance services to nearby players
banker = false

# Aggro configuration (NPCs with ai_mode = "aggro" or "patrol")
aggro_range = 5          # Distance in rooms to detect targets
aggro_players = true     # Aggro on players
aggro_mobs = false       # Aggro on other NPCs
aggro_race = []          # Only aggro specific races (empty = all)

# Faction affiliation
faction = "midgaard_guards"
faction_standing = 0     # Starting standing with faction (-100 to 100)

# Trainer configuration — types this NPC can train
trainer_types = ["attributes", "combat", "weapon"]

# Language capabilities
languages = ["common", "dwarvish"]

# Known skills
[[skills]]
id = "shield_block"
level = 3

# Script hooks
[[scripts]]
event = "on_death"
script = "guard_drops.rhai"

[health]
current = 100
max = 100

[attributes]
str = 14
dex = 12
int = 10
wis = 10
con = 14
cha = 10

# Equipment spawned on the mob
[[equipment]]
template_id = "steel_shortsword"
slot = "weapon"

[[equipment]]
template_id = "chainmail_chestpiece"
slot = "torso"

# Loot tables define items dropped on death
[[loot.entries]]
item = "copper_coin"
chance = 100

[loot.entries.count]
min = 5
max = 15

[[loot.entries]]
item = "healing_potion"
chance = 25

[loot.entries]
treasure_class = "guard_gear"  # Random roll from treasure class
chance = 15
```

#### Mob Schema Fields

| Field              | Type    | Default    | Description                                                                                       |
| ------------------ | ------- | ---------- | ------------------------------------------------------------------------------------------------- |
| `id`               | String  | —          | Unique identifier for the mob template.                                                           |
| `name`             | String  | —          | Display name seen in room descriptions.                                                           |
| `description`      | String  | —          | Full description on `look` (supports color codes).                                                |
| `short_desc`       | String  | —          | Brief description shown in room entity list. Falls back to `name`.                                |
| `level`            | Integer | `1`        | NPC level; affects hit/damage and XP calculations.                                                |
| `race`             | String  | —          | Race template ID; grants racial traits and size defaults.                                         |
| `size`             | String  | `"medium"` | Size category: `tiny`, `small`, `medium`, `large`, `huge`.                                        |
| `armor`            | Integer | `0`        | Base armor class value.                                                                           |
| `damage`           | String  | —          | Natural weapon damage in dice notation (e.g. `1d6`, `2d4+3`).                                     |
| `damage_type`      | String  | —          | Natural weapon damage type: `slash`, `pierce`, `bludgeon`, etc.                                   |
| `xp_value`         | Integer | `0`        | XP awarded when killed.                                                                           |
| `shop`             | String  | —          | Shop template ID if this NPC is a vendor.                                                         |
| `faction`          | String  | —          | Faction template ID this NPC belongs to.                                                          |
| `faction_standing` | Integer | `0`        | Starting faction standing (–100 to 100).                                                          |
| `friendly`         | Bool    | `false`    | If true, never aggroes players.                                                                   |
| `banker`           | Bool    | `false`    | If true, this NPC offers bank services (`balance`, `deposit`, `withdraw`) to players in the room. |
| `ai_mode`          | String  | `"idle"`   | AI behavior: `idle`, `wander`, `patrol`, `aggro`.                                                 |
| `patrol_route`     | Array   | `[]`       | Room IDs for patrol waypoints (`area.room` format).                                               |
| `wander_rooms`     | Array   | `[]`       | Specific rooms to wander (empty = any in area).                                                   |
| `wander_area`      | Bool    | `false`    | If true, wanders anywhere in the current area.                                                    |
| `aggro_range`      | Integer | `0`        | Room-distance to detect targets. `0` = room only.                                                 |
| `aggro_players`    | Bool    | `false`    | If true, aggroes on player characters.                                                            |
| `aggro_mobs`       | Bool    | `false`    | If true, aggroes on other NPCs.                                                                   |
| `aggro_race`       | Array   | `[]`       | Only aggro specific races (empty = all races).                                                    |
| `trainer_types`    | Array   | `[]`       | Training categories this NPC teaches: `attributes`, `combat`, `weapon`, `magic`, `crafting`.      |
| `languages`        | Array   | `[]`       | Languages the NPC speaks/knows.                                                                   |
| `scripts`          | Array   | `[]`       | Script hooks that fire on game events.                                                            |
| `health`           | Section | —          | `{ current, max }` — starting hit points.                                                         |
| `attributes`       | Section | —          | `{ str, dex, int, wis, con, cha }` — base attributes.                                             |
| `equipment`        | Array   | `[]`       | Equipment entries: `{ template_id, slot }`.                                                       |
| `loot`             | Section | —          | Loot table: `{ entries: [{ item, chance, count?, treasure_class? }] }`.                           |
| `skills`           | Array   | `[]`       | Known skills: `{ id, level }`.                                                                    |

---

### Items (`ItemTemplate`)

Stored under `content/items/`. Items represent weapon, armor, and utility game objects.

```toml
id = "steel_shortsword"
name = "a steel shortsword"
description = "A serviceable steel shortsword."
item_type = "weapon"
subtype = "sword"
rarity = "common"
quality = "standard"
level_requirement = 1
weight = 3.0
value = 500
flags = ["metal"]

# Restricted usage class/race limits
allowed_classes = ["warrior", "paladin"]
allowed_races = []
allowed_alignments = []

# Optional skill gate — checked continuously; auto-unequips on failure
requires_skill = { id = "two_handed", level = 1 }

# Weapon stats (only active if item_type = "weapon")
[weapon]
damage = "1d6"
damage_type = "pierce"
speed = 2.5
range = "melee"

# Equipment slot (if wearable)
[equipment]
slot = "weapon"

# Item triggers fire on game events
[[triggers]]
event = "on_hit"
chance = 100
cast = "weaken"
target = "target"

# Optional set membership
[set]
id = "leather_set"
piece_type = "gloves"

# Contextual item commands & bestowal messages
[[commands]]
name = "ignite"
script = "scripts/ignite_sword.rhai"
help = "Ignites the blade with holy fire."
requires_equipped = true
equip_message = "Equipping the sword bestows the ability 'ignite'."
unequip_message = "Unequipping the sword removes the ability 'ignite'."
examine_hint = "You feel a faint warmth radiating from the hilt."

# Permanent passive affects bestowed while equipped
[[permanent_affects]]
id = "keen_edge"
name = "+1 to Hit"
stat = "to_hit"
amount = 1
condition = "none"  # "none", set requirements, or race/class gated
affects_display = "The keen blade grants +1 to attack rolls."
```

#### Core Schema Fields

| Field                | Type         | Description                                                                                                                                         |
| -------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| `id`                 | String       | Unique identifier of the item template.                                                                                                             |
| `name`               | String       | Short display name.                                                                                                                                 |
| `description`        | String       | Description seen when looking at the item. Supports formatting/color codes.                                                                         |
| `item_type`          | String       | Broad item category: `weapon`, `armor`, `container`, `potion`, `scroll`, `wand`, `food`, `drink`, `key`, `quest`, `treasure`, `light`, `furniture`. |
| `subtype`            | String       | Sub-category details (e.g., `sword`, `shield`, `vest`).                                                                                             |
| `rarity`             | String       | Rarity tier (loot rolls, affix budget, display color): `common`, `uncommon`, `rare`, `epic`, `legendary`, `artifact`.                               |
| `quality`            | String       | Craftsmanship quality (flavor/price modifier): `poor`, `standard`, `fine`, `masterwork`. Distinct from rarity.                                      |
| `level_requirement`  | Integer      | Minimum level required to equip/use.                                                                                                                |
| `weight`             | Float        | Weight of the item in pounds.                                                                                                                       |
| `value`              | Integer      | Vendor value in copper pieces (10,000 cp = 1 gp).                                                                                                   |
| `flags`              | Array        | Tags identifying special traits (e.g., `["unique", "quest"]`).                                                                                      |
| `allowed_classes`    | Array        | Classes permitted to equip/use (empty = all).                                                                                                       |
| `allowed_races`      | Array        | Races permitted to equip/use (empty = all).                                                                                                         |
| `allowed_alignments` | Array        | Alignments permitted to equip/use (empty = all).                                                                                                    |
| `requires_skill`     | Inline Table | `{ id = "skill_id", level = N }` — Skill gate checked continuously.                                                                                 |
| `weapon`             | Section      | Defines weapon performance stats (see below).                                                                                                       |
| `equipment`          | Section      | Defines wear slot/equipment info (see below).                                                                                                       |
| `set`                | Section      | Associates the item with an item set (see below).                                                                                                   |
| `consumable`         | Section      | Defines potion, scroll, wand, food, or drink consumable properties.                                                                                 |
| `container`          | Section      | Defines portable container or stationary fluid source properties.                                                                                   |
| `durability`         | Section      | Defines maximum durability and combat decay rate.                                                                                                   |
| `triggers`           | Array        | Triggers that fire spell-like effects on events.                                                                                                    |

#### Weapon, Equipment, Consumable & Container Sections

- **`[weapon]` Section**:
  - `damage`: Fenced string in `XdY+Z` dice notation.
  - `damage_type`: String representing damage category: `slash`, `pierce`, `bludgeon`, `fire`, `cold`, `lightning`, `acid`, `poison`, `magic`, `true`.
  - `speed`: Float specifying attack delay in seconds (default is `2.5`s).
  - `range`: String specifying attack reach: `melee`, `ranged`, `reach`, `thrown`.
- **`[equipment]` Section**:
  - `slot`: String matching equipped slot location: `head`, `neck`, `torso`, `arms`, `hands`, `finger`, `legs`, `feet`, `weapon`, `shield`, `ammo`, `back`, `waist`.
- **`[consumable]` Section**:
  - `kind`: String (`potion`, `scroll`, `wand`, `food`, `drink`).
  - `charges`: Initial usage charges (default `1`).
  - `max_charges`: Maximum usage charges.
  - `restore_health` / `restore_mana` / `restore_stamina`: Stat amounts restored on consumption.
  - `effect_script`: Rhai script path triggered on use/recite/zap.
  - `depleted_template`: Template ID to leave behind when empty (e.g. `"empty_vial"`).
- **`[container]` Section**:
  - `capacity_weight`: Max combined weight of items stored inside.
  - `max_items`: Maximum count of items inside.
  - `weight_reduction_pct`: Percentage weight reduction applied to contained items (0–100%).
  - `is_drink_container`: Boolean indicating if container holds liquid charges.
  - `liquid_type`: String (`water`, `ale`, `wine`).
  - `is_closed` / `is_locked`: Boolean flags for container doors/latches.
  - `key_template_id`: Template ID required to unlock.
- **`[durability]` Section**:
  - `max`: Maximum item durability integer.
  - `decay_rate`: Float specifying durability degradation multiplier.

---

### Item Triggers

Items can have trigger definitions that execute spell-like effects on game events. Defined inline on the item template:

```toml
[[triggers]]
event = "on_hit"       # Event name
chance = 50            # Percentage chance to fire (1-100)
cast = "weaken"        # Skill/template to execute
target = "target"      # Target: "self", "attacker", "room", "target"
```

| Event             | Fires When                              |
| ----------------- | --------------------------------------- |
| `on_wear`         | Item is worn or wielded                 |
| `on_remove`       | Item is removed or unwielded            |
| `on_hit`          | Attacker lands a hit in combat          |
| `on_use`          | Item is used (not yet wired)            |
| `on_kill`         | Attacker kills a target (not yet wired) |
| `on_damage_taken` | Wearer takes damage (not yet wired)     |

The trigger system scans the wielder's equipment and inventory for matching events when the event occurs. Multiple triggers on the same item all roll independently.

---

### Item Sets

Item sets provide tiered bonuses when multiple pieces of the same set are equipped. Defined in `content/sets/<set_id>.toml`.

#### Item Set Definition (`SetDef`)

```toml
id = "leather_set"
name = "Leather Armor Set"

# Tier 1 Bonus
[[bonuses]]
min_pieces = 2
effects = [{ effect_type = "stat", stat = "armor", amount = 1 }]

# Tier 2 Bonus
[[bonuses]]
min_pieces = 4
effects = [
    { effect_type = "stat", stat = "armor", amount = 3 },
    { effect_type = "stat", stat = "dexterity", amount = 1 }
]
```

- `id`: Unique identifier matching the set.
- `name`: Display name of the set.
- `bonuses`: Array of `SetBonusEntry` defining what rewards activate at specific numbers of equipped pieces.
  - `min_pieces`: The threshold count of equipped set items required.
  - `conditions`: Optional array of piece conditions (e.g. `{ piece_type = "gloves", min = 1 }`).
  - `effects`: Array of effects containing:
    - `effect_type`: Typically `stat` or `aura`.
    - `stat`: The affected attribute/stat (e.g. `strength`, `armor`).
    - `amount`: Numeric modifier (e.g. `1`, `-2`).
    - `aura_id` / `radius`: Aura parameters if `effect_type = "aura"`.

Items link to the set via their `[set]` config, which must specify the correct `piece_type` (matching the `slot` worn):

```toml
[set]
id = "leather_set"
piece_type = "gloves"
```

Set bonuses are re-evaluated on equip/remove. If a tier threshold is crossed, the active bonuses are recalculated.

---

### Combat Stances (`StanceDef`)

Stored in `content/stances/<stance_id>.toml`. Stances modify combat behavior and stats. Players switch stances with the `stance` command. Built-in stances: `normal`, `defensive`, `aggressive`, `berserk`.

```toml
id = "berserk"
name = "Berserk"
description = "Enter a raging fury, trading defense for raw damage."

# Stat bonuses applied while in this stance
stat_bonuses = { damage = 5, armor = -3, stun_resist = 10 }

# Optional: restrict to specific weapon types (empty = all)
allowed_weapons = ["axe", "bludgeon"]

# Optional: restrict to specific armor types (empty = all)
allowed_armor = []

# Skill modifiers: { skill_id = bonus_pct }
skill_modifiers = { "two_handed" = 10 }

# Combat bonus percentages: { "hit" = bonus, "damage" = bonus, "armor" = bonus }
combat_bonus = { hit = -5, damage = 25 }
```

| Field             | Type   | Description                                                      |
| ----------------- | ------ | ---------------------------------------------------------------- |
| `id`              | String | Unique stance identifier.                                        |
| `name`            | String | Display name shown to the player.                                |
| `description`     | String | Description seen when examining the stance.                      |
| `stat_bonuses`    | Table  | Attribute modifications: `{ damage, armor, stun_resist, etc. }`. |
| `allowed_weapons` | Array  | Weapon subtypes allowed (empty = all).                           |
| `allowed_armor`   | Array  | Armor subtypes allowed (empty = all).                            |
| `skill_modifiers` | Table  | Skill rank bonuses: `{ skill_id = bonus_pct }`.                  |

---

### Skills & Spells (`SkillDef`)

Stored in `content/skills/<skill_id>.toml`. Defines combat abilities, spells, and utilities. Skills can be restricted via engine `[restrictions]` and linked to Rhai script triggers.

```toml
id = "parry"
name = "Parry"
command = "parry"
script = "skills/parry.rhai"
description = "Deflect incoming melee attacks. Success rate scales with practice level up to 60%."

# Built-in Engine Restrictions (automatically evaluated prior to script invocation)
[restrictions]
allowed_classes = ["warrior"]
allowed_races = []
min_level = 1
in_combat_only = false
```

#### Restriction Fields (`CommandRestrictions`)

| Field                | Type    | Description                                                                                                                |
| :------------------- | :------ | :------------------------------------------------------------------------------------------------------------------------- |
| `allowed_classes`    | Array   | List of permitted classes (e.g. `["warrior"]` or `["mage", "wizard"]`). Empty = all classes.                               |
| `allowed_races`      | Array   | List of permitted races (e.g. `["elf"]`). Empty = all races.                                                               |
| `min_level`          | Integer | Minimum character level required. If under-leveled, engine prints: _"You are not experienced enough to use that ability."_ |
| `min_skill_ranks`    | Table   | Required practice ranks: `{ "two_handed" = 50 }`.                                                                          |
| `allowed_stances`    | Array   | Required fighting stances: `["defensive"]`.                                                                                |
| `in_combat_only`     | Bool    | If true, skill can only be used while in combat.                                                                           |
| `out_of_combat_only` | Bool    | If true, skill can only be used outside combat.                                                                            |

---

### Passive Traits (`PassiveDef`)

Stored in `content/passives/<passive_id>.toml`. Passives are always-active traits granted by race, class, equipment, or spells.

```toml
id = "toughness"
name = "Toughness"
description = "Increases maximum hit points."

# Effect type: "stat_bonus", "grant_skill", "trigger"
effect_type = "stat_bonus"

# Stat modification (for effect_type = "stat_bonus")
stat = "constitution"
amount = 2

# Skill granted (for effect_type = "grant_skill")
grant_skill = "shield_block"

# Trigger parameters (for effect_type = "trigger")
trigger_chance = 20
duration = 30
cooldown = 60
```

| Field            | Type    | Default | Description                                        |
| ---------------- | ------- | ------- | -------------------------------------------------- |
| `id`             | String  | —       | Unique passive identifier.                         |
| `name`           | String  | —       | Display name.                                      |
| `description`    | String  | —       | Description of the passive effect.                 |
| `effect_type`    | String  | —       | `stat_bonus`, `grant_skill`, or `trigger`.         |
| `stat`           | String  | —       | Attribute/skill name to modify (for `stat_bonus`). |
| `amount`         | Integer | `0`     | Modifier value.                                    |
| `grant_skill`    | String  | —       | Skill ID granted by this passive.                  |
| `trigger_chance` | Integer | `0`     | Percentage chance to fire (for `trigger` type).    |
| `duration`       | Integer | `0`     | Effect duration in seconds (0 = permanent).        |
| `cooldown`       | Integer | `0`     | Cooldown between triggers in seconds.              |

---

### Quests (`QuestDef`)

Quests define narrative tasks that players can perform for rewards (experience, gold, item templates, faction standings).

| Field               | Type    | Description                                                                                             |
| ------------------- | ------- | ------------------------------------------------------------------------------------------------------- |
| `id`                | String  | Unique quest identifier (e.g. `save_the_sheep`).                                                        |
| `name`              | String  | User-facing quest title.                                                                                |
| `description`       | String  | In-game quest journal description.                                                                      |
| `level_requirement` | Integer | Minimum player level required to start the quest. Default `0`.                                          |
| `repeatable`        | Boolean | If true, the player can repeat the quest after completion. Default `false`.                             |
| `auto_complete`     | Boolean | If true, completes instantly when all objectives are met without turning in to an NPC. Default `false`. |
| `giver_npc`         | String  | Optional template ID of the NPC who offers this quest.                                                  |
| `turn_in_npc`       | String  | Optional template ID of the NPC to whom this quest must be turned in.                                   |
| `prerequisites`     | Array   | List of quest IDs that must be completed before accepting this quest.                                   |
| `objectives`        | Array   | List of quest objectives (e.g. `Kill`, `Gather`, `Deliver`, `Explore`, `Talk`).                         |
| `rewards`           | Section | Rewards paid out on completion (XP, Gold, Items, Faction adjustments).                                  |
| `scripts`           | Section | Path to scripts triggered on `on_accept`, `on_update`, and `on_complete`.                               |

#### Objective Types

- **Kill**: `{ type = "Kill", mob = "wolf", count = 3 }`
- **Gather**: `{ type = "Gather", item = "wolf_pelt", count = 2 }`
- **Deliver**: `{ type = "Deliver", item = "old_letter", npc = "mayor" }`
- **Explore**: `{ type = "Explore", room = "midgaard.temple_altar" }`
- **Talk**: `{ type = "Talk", npc = "old_hermit" }`

---

### Factions (`FactionDef`)

Factions track players' reputations with in-game organizations or groups. Faction standings affect NPC aggressiveness and merchant prices.

| Field               | Type    | Description                                                                           |
| ------------------- | ------- | ------------------------------------------------------------------------------------- |
| `id`                | String  | Unique faction identifier (e.g. `town_guard`).                                        |
| `name`              | String  | User-facing faction name.                                                             |
| `description`       | String  | Description of the faction's purpose/lore.                                            |
| `starting_standing` | Integer | Default reputation score for players. Default `0`.                                    |
| `min_standing`      | Integer | Minimum possible reputation score. Default `-1000`.                                   |
| `max_standing`      | Integer | Maximum possible reputation score. Default `1000`.                                    |
| `ranks`             | Array   | List of ranks mapped to standings thresholds (e.g. `Neutral`, `Friendly`, `Hostile`). |
| `relationships`     | Map     | Inter-faction propagation multipliers.                                                |
| `aggro_below`       | Integer | Reputation threshold below which faction members automatically attack the player.     |

---

### Shops (`ShopTemplate`)

Stored under `content/shops/`. A shop is attached to an NPC through the mob's `shop` field. The mob's `faction` determines which reputation tier applies to shoppers (see `price_mods` below).

```toml
id = "elias_wares"
name = "Elias's General Store"

buy_rate = 0.5    # Fraction of value the shop pays when buying from players
sell_rate = 1.5   # Multiplier on value applied to prices players pay
restock_secs = 300  # Seconds between automatic restocks of stock counts

# Item types/subtypes the shop will buy back from players (empty = buys nothing)
buy_types = ["potion"]

# Price multiplier per faction rank (keys must match the faction's `ranks`)
[price_mods]
Adored = 0.85
Friendly = 0.95
Neutral = 1.0
Hostile = 1.5

# Haggling knobs (string values)
[params]
haggle_floor = "0.75"          # Vendor accepts nothing below this fraction of asking
insult_floor = "0.60"          # Offers below this are refused as insulting
haggle_rounds = "4"            # Counter-offer rounds before the vendor gives up
haggle_cooldown_secs = "300"   # Cooldown between haggling attempts per item

[[inventory]]
item = "healing_salve"   # Item template ID
count = 10               # Restock target count
price = 30               # Asking price in copper (overrides value × sell_rate)

[[inventory]]
item = "torch"
count = 25
price = 3
```

| Field          | Type    | Default | Description                                                                                                     |
| -------------- | ------- | ------- | --------------------------------------------------------------------------------------------------------------- |
| `id`           | String  | —       | Unique shop identifier.                                                                                         |
| `name`         | String  | —       | Shop display name.                                                                                              |
| `buy_rate`     | Number  | —       | Fraction of base value the shop pays when buying from players.                                                  |
| `sell_rate`    | Number  | —       | Multiplier on base value applied to prices players pay to buy.                                                  |
| `restock_secs` | Integer | —       | Seconds between automatic restocks of stock counts.                                                             |
| `inventory`    | Array   | `[]`    | Stock entries: `item` (template ID), `count` (restock target), `price` (copper; defaults to value × sell_rate). |
| `buy_types`    | Array   | `[]`    | Item types/subtypes the shop buys back from players. Empty means the shop buys nothing.                         |
| `price_mods`   | Map     | `{}`    | Price multiplier per faction rank, keyed by rank name (e.g. `Neutral`).                                         |
| `params`       | Map     | `{}`    | Haggling knobs: `haggle_floor`, `insult_floor`, `haggle_rounds`, `haggle_cooldown_secs`.                        |

If no `price_mods` entry matches a player's current rank, Neutral pricing (1.0) is assumed.

---

### Item Rarity, Quality & Affixes

Items have two independent tier axes:

- **Rarity** (`rarity`) — how unusual the item is: `common`, `uncommon`, `rare`, `epic`, `legendary`, `artifact`. Rarity determines how many affixes an item can roll when spawned as loot, and its display color.
- **Quality** (`quality`) — craftsmanship of the individual item: `poor`, `standard`, `fine`, `masterwork`. A purely descriptive/price modifier; a masterwork common item and a poor rare item are both valid.

#### Rarity Tiers & Affix Counts

| Tier      | Max Affixes | Color  |
| --------- | ----------- | ------ |
| Common    | 0           | White  |
| Uncommon  | 1           | Green  |
| Rare      | 2           | Blue   |
| Epic      | 3           | Purple |
| Legendary | 4           | Orange |

#### Affix Templates (`AffixDef`)

Stored in `content/affixes/<affix_id>.toml`.

```toml
id = "of_might"
name = "Might"
description = "+2 Strength"
type = "suffix"                # prefix or suffix
stat = "strength"              # target attribute to modify
amount = "2"                   # modifier value
quality_min = "uncommon"       # minimum item rarity required to roll this affix
slot = ["weapon", "torso"]     # item slots eligible for this affix
weight = 5                     # relative selection weight
```

When loot drops, the system selects random affixes compatible with the item's slot and rolls them up. Prefix names prepend to the item name, and suffix names append (prefixed with "of"). For example, a "steel shortsword" might roll "Keen steel shortsword of Might". Affix modifiers are applied directly to the item's stat profile.

Builders can modify the world dynamically using in-game OLC commands.

> **Note:** Full OLC commands are planned. The following are partially or not yet implemented in code — documented here as the target API.

- `@area create <id> <name>` — Creates a new zone.
- `@area reset <id>` — Forces an immediate area reset.
- `@area save <id>` — Writes all current in-memory edits for a zone back to its TOML files.
- `@dig <direction> <dest_room_id> [name]` — Digs an exit from the current room to a new room. If the target room doesn't exist, it creates it.
- `@link <direction> <dest_room_id>` — Creates a one-way exit from the current room in the specified direction.
- `@unlink <direction>` — Removes an exit in the specified direction.
- `@set <field> <value>` — Modifies properties of the current room, target mob, or item (e.g., `@set name The Grand Altar`, `@set flag dark`).
- `@desc` — Opens a multi-line text editor to set the current room description.
- `@portal add <keyword> <dest_room_id>` — Adds a portal connection.
- `@portal remove <keyword>` — Deletes a portal.
- `@mob add <mob_id>` — Spawns a mobile template into the current room.
- `@mob remove <entity_id>` — Despawns a mobile.
- `@item load <item_id>` — Loads a copy of an item template into your inventory.
- `@validate [area_id]` — Runs full cross-reference validation on all templates, or a single area.
- `@award <xp_amount>` — Grants XP to the current player (for testing).

---

## Validation and Integrity Checks

To prevent crashes and gameplay glitches, the engine runs a **Cross-Reference Validation Pipeline** during startup via the `validate_all` MCP tool, and per-area via `validate_area`.

Validation checks verify:

1. **Broken Links**: Exits and portals must point to valid rooms (`area_id.room_id`).
2. **Missing Templates**: Mobs and items must reference valid race, class, and set definitions.
3. **Attribute Bounds**: Attributes, ages, heights, and weights must lie within constraints set by the target race template.
4. **Prerequisites Integrity**: Skills prerequisites are checked for circular dependencies via `validate_content_dag`.
5. **Deity Policies**: Deity requirements on classes and alignments must be valid.
6. **Skill Gates**: Item and weapon `requires_skill` definitions must reference valid skill IDs in the skills registry.

---

## Hot-Reloading

Hot-reloading is planned. Currently, content templates are loaded at server startup. To apply template changes:

- **MCP offline mode**: The MCP server reads/writes TOML files directly and validates them. Changes are visible on next game server restart.
- **MCP online mode** (planned): A REST bridge will push validated template changes to a running game server without restart.
- **`notify` file-watcher** (planned): Automatic reload of templates and scripts on file save.

For now, edit TOML files and restart the server to pick up changes.
