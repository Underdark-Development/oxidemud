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
├── affixes.toml                      # Item affix configurations
├── sets.toml                         # Item set sets and bonuses
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
ai_mode = "patrol"

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
slot = "weapon"
item_id = "steel_shortsword"

[[equipment]]
slot = "torso"
item_id = "chainmail_chestpiece"

# Loot tables define items dropped on death
[[loot]]
item_id = "copper_coin"
count = [5, 15]
chance = 1.0

[[loot]]
item_id = "healing_potion"
count = [1, 1]
chance = 0.25
```

---

## Online Creation (OLC) Commands

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

---

## Hot-Reloading

The engine runs a file-watcher (`notify` crate) on the `content/` directory:
- When a template TOML file is saved, the engine automatically compiles and validates it.
- If validation succeeds, the engine performs an **atomic swap** in the `TemplateRegistry` and broadcasts a `ContentReloaded` event.
- If validation fails, the change is rejected, the error is logged to staff, and the old version remains active in memory.
