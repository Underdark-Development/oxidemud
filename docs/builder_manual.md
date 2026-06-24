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
│       └── rooms/
│           ├── <room_id_1>.toml      # Room template 1
│           └── <room_id_2>.toml      # Room template 2
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
│   └── <set_id>.toml                # Item set definitions and tier bonuses
├── languages.toml                    # Languages granted at creation
├── socials.toml                      # Social emotes definitions
└── treasure_classes.toml             # Random loot tier configurations
```

---

## Core Template Schemas

### Areas (`AreaTemplate`)
Stored at `content/areas/<area_id>/area.toml`. Defines a geographic zone of rooms.

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

### Rooms (`RoomTemplate`)
Stored at `content/areas/<area_id>/rooms/<room_id>.toml`.

```toml
id = "temple_square"
area = "midgaard"
name = "Temple Square"
description = "A large open square paved with granite cobblestones. A grand temple stands to the north."
flags = ["portal_in"]

# Exits connect adjacent rooms
[[exits]]
direction = "north"
dest = "midgaard.temple_entrance"

[[exits]]
direction = "south"
dest = "midgaard.market_square"

# Portals represent keyword-based movement
[[portals]]
keyword = "archway"
dest = "midgaard.temple_altar"
description = "A shimmering stone archway."
flags = ["hidden"]

# Spawn configurations for characters created here
[spawn]
allowed_classes = ["cleric", "paladin"]
description = "The sanctuary spawns holy acolytes."
```

### Mobs (`MobTemplate`)
Stored under `content/mobs/`.

```toml
id = "city_guard"
name = "a city guard"
description = "A stout guard in chainmail patrols the streets."
level = 5
race = "human"
size = "medium"
faction = "midgaard_guards"

# AI behavior mode: idle, wander, patrol, aggro, combat, flee, return
ai_mode = "patrol"

# Patrol route (room IDs in area.room format) — NPC follows waypoints
patrol_route = ["midgaard.guard_post", "midgaard.temple_square", "midgaard.market_square"]

# Wander settings (active when ai_mode = "wander")
wander_rooms = ["midgaard.temple_square", "midgaard.market_square"] # Specific rooms to wander (empty = any in area)
wander_area = true                                                  # Wander anywhere in current area if true

# Friendly NPCs don't aggro players
friendly = false

# Aggro configuration (NPCs with ai_mode = "aggro" or "patrol")
aggro_range = 5          # Distance in rooms to detect targets
aggro_players = true     # Aggro on players
aggro_mobs = false       # Aggro on other NPCs
aggro_race = []          # Only aggro specific races (empty = all)

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
```

---

### Items (`ItemTemplate`)
Stored under `content/items/`. Items represent weapon, armor, and utility game objects.

```toml
id = "steel_shortsword"
name = "a steel shortsword"
description = "A serviceable steel shortsword."
item_type = "weapon"
subtype = "sword"
quality = "common"
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
```

#### Core Schema Fields

| Field | Type | Description |
|---|---|---|
| `id` | String | Unique identifier of the item template. |
| `name` | String | Short display name. |
| `description` | String | Description seen when looking at the item. Supports formatting/color codes. |
| `item_type` | String | Broad item category: `weapon`, `armor`, `container`, `potion`, `scroll`, `wand`, `food`, `drink`, `key`, `quest`, `treasure`, `light`, `furniture`. |
| `subtype` | String | Sub-category details (e.g., `sword`, `shield`, `vest`). |
| `quality` | String | Base quality tier: `poor`, `common`, `magic`, `rare`, `epic`, `legendary`. |
| `level_requirement`| Integer| Minimum level required to equip/use. |
| `weight` | Float | Weight of the item in pounds. |
| `value` | Integer | Vendor value in copper pieces (10,000 cp = 1 gp). |
| `flags` | Array | Tags identifying special traits (e.g., `["unique", "quest"]`). |
| `allowed_classes` | Array | Classes permitted to equip/use (empty = all). |
| `allowed_races` | Array | Races permitted to equip/use (empty = all). |
| `allowed_alignments`| Array | Alignments permitted to equip/use (empty = all). |
| `requires_skill` | Inline Table | `{ id = "skill_id", level = N }` — Skill gate checked continuously. |
| `weapon` | Section | Defines weapon performance stats (see below). |
| `equipment` | Section | Defines wear slot/equipment info (see below). |
| `set` | Section | Associates the item with an item set (see below). |
| `triggers` | Array | Triggers that fire spell-like effects on events. |

#### Weapon & Equipment Sections

- **`[weapon]` Section**:
  - `damage`: Fenced string in `XdY+Z` dice notation.
  - `damage_type`: String representing damage category: `slash`, `pierce`, `bludgeon`, `fire`, `cold`, `lightning`, `acid`, `poison`, `magic`, `true`.
  - `speed`: Float specifying attack delay in seconds (default is `2.5`s).
  - `range`: String specifying attack reach: `melee`, `ranged`, `reach`, `thrown`.
- **`[equipment]` Section**:
  - `slot`: String matching equipped slot location: `head`, `neck`, `torso`, `arms`, `hands`, `finger`, `legs`, `feet`, `weapon`, `shield`, `ammo`, `back`, `waist`.

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

| Event | Fires When |
|---|---|
| `on_wear` | Item is worn or wielded |
| `on_remove` | Item is removed or unwielded |
| `on_hit` | Attacker lands a hit in combat |
| `on_use` | Item is used (not yet wired) |
| `on_kill` | Attacker kills a target (not yet wired) |
| `on_damage_taken` | Wearer takes damage (not yet wired) |

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

### Item Quality & Affixes

Items have a quality tier that determines how many affixes they can roll when spawned as loot.

#### Quality Tiers & Affix Counts

| Tier | Max Affixes | Color |
|---|---|---|
| Common | 0 | White |
| Uncommon | 1 | Green |
| Rare | 2 | Blue |
| Epic | 3 | Purple |
| Legendary | 4 | Orange |

#### Affix Templates (`AffixDef`)
Stored in `content/affixes/<affix_id>.toml`.

```toml
id = "of_might"
name = "Might"
description = "+2 Strength"
type = "suffix"                # prefix or suffix
stat = "strength"              # target attribute to modify
amount = "2"                   # modifier value
quality_min = "uncommon"       # minimum item quality required to roll this affix
slot = ["weapon", "torso"]     # item slots eligible for this affix
weight = 5                     # relative selection weight
```

When loot drops, the system selects random affixes compatible with the item's slot and rolls them up. Prefix names prepend to the item name, and suffix names append (prefixed with "of"). For example, a "steel shortsword" might roll "Keen steel shortsword of Might". Affix modifiers are applied directly to the item's stat profile.

Builders can modify the world dynamically using in-game OLC commands.

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

---

## Validation and Integrity Checks

To prevent crashes and gameplay glitches, the engine runs a **Cross-Reference Validation Pipeline** during startup, in Spade, and on demand via the `@validate` command or MCP.

Validation checks verify:
1. **Broken Links**: Exits and portals must point to valid rooms (`area_id.room_id`).
2. **Missing Templates**: Mobs and items must reference valid race, class, and set definitions.
3. **Attribute Bounds**: Attributes, ages, heights, and weights must lie within constraints set by the target race template.
4. **Prerequisites Integrity**: Quests, recipes, and skills prerequisites cannot form circular dependencies.
5. **Deity Policies**: Deity requirements on classes and alignments must be valid.
6. **Skill Gates**: Item and weapon `requires_skill` definitions must reference valid skill IDs in the skills registry.

---

## Hot-Reloading

The engine runs a file-watcher (`notify` crate) on the `content/` directory:
- When a template TOML file is saved, the engine automatically compiles and validates it.
- If validation succeeds, the engine performs an **atomic swap** in the `TemplateRegistry` and broadcasts a `ContentReloaded` event.
- If validation fails, the change is rejected, the error is logged to staff, and the old version remains active in memory.
