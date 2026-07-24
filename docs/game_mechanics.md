# Game Mechanics

This document provides a reference for admins and world builders to understand the underlying mechanics, formulas, and systems that drive combat, equipment, and looting in the game engine.

---

## Combat System

Combat runs on a periodic **Combat Pulse (every 2 seconds)**. Each pulse, all engaged entities perform their attacks.

### 1. Attack Flow

For each combat round:

1. **Target Verification**: The system checks if the attacker and target exist, are alive, and are in the same room.
2. **Hit Check**: The attacker rolls a `d20` to determine if the attack lands (see hit formula below).
3. **Damage Calculation**: If the attack is a hit, the damage is calculated based on weapon/unarmed values, attributes, and styles (see damage formula below).
4. **Resistances and Buffs**: The target's resistances modify the incoming damage amount.
5. **Apply Damage**: The target's health is decremented. If health falls to 0 or below, death is triggered.
6. **Death and Rewards**: On death, XP is awarded to the attacker, a corpse is spawned containing the victim's inventory/equipment, and the victim is despawned.

### 2. To-Hit Formula

To determine if an attack hits:
\[ \text{Roll} + \text{Attacker Level} + \text{Strength Modifier} + \text{Dual Wield Penalty} \ge \text{AC} \]

- **Roll**: A random integer from 1 to 20 (`fastrand::i32(1..=20)`).
- **Attacker Level**: The level component of the attacking entity.
- **Strength Modifier**: Derived from the attacker's strength attribute (using standard D&D-style modifier: `(Strength - 10) / 2` integer division).
- **Dual Wield Penalty**: Applied only when dual-wielding (see Weapon Styles below).
- **Armor Class (AC)**: The defender's Armor Class.

#### Critical Hits & Automatic Misses

- **Natural 1**: An attack roll of 1 automatically misses, regardless of modifiers.
- **Natural 20 (Critical Hit)**: An attack roll of 20 automatically hits and deals **double damage** (total calculated damage is multiplied by 2).

### 3. Defense (Armor Class) Formula

An entity's Armor Class (AC) is calculated as:
\[ \text{AC} = 10 + \text{Level} + \text{Dexterity Modifier} + \text{Armor Rating} + \text{Shield Bonus} \]

- **Level**: The entity's current level.
- **Dexterity Modifier**: Derived from the entity's dexterity attribute (`(Dexterity - 10) / 2`).
- **Armor Rating**: The sum of base and bonus armor from all equipped items.
- **Shield Bonus**: A flat `+2` bonus to AC if a shield is equipped.

### 4. Damage Formula

The base damage of a successful hit depends on whether a weapon is equipped:

- **Armed (Melee/Ranged)**:
  \[ \text{Damage} = \text{Weapon Damage Roll} + \text{Strength Bonus} \]
  - **Weapon Damage Roll**: A roll from the weapon template's damage dice (e.g. `1d6` rolls a random value from 1 to 6).
  - **Strength Bonus**:
    - **One-Handed**: Full Strength Modifier.
    - **Two-Handed**: 1.5× Strength Modifier (rounded to nearest integer).
    - **Off-Hand (Dual-Wield)**: 0.5× Strength Modifier (rounded to nearest integer).
- **Unarmed**:
  \[ \text{Damage} = \text{1d4 Roll} + \text{Strength Modifier} + \text{Level} / 5 \]
  - **Level / 5**: Attacker's level divided by 5 (using integer division).

_Note: Final calculated damage is always clamped to a minimum of 1._

---

## Damage Types & Resistances

The game supports 10 distinct damage types. Every source of damage (weapons, spells, environment) is typed, and entities can have individual resistance multipliers per damage type.

### Damage Types

1. **Slash**: Typical edge weapon damage (swords, axes).
2. **Pierce**: Typical pointed weapon damage (daggers, arrows, spears).
3. **Bludgeon**: Typical blunt weapon damage (maces, clubs, unarmed).
4. **Fire**: Elemental fire damage.
5. **Cold**: Elemental cold damage.
6. **Lightning**: Elemental electrical damage.
7. **Acid**: Corrosive chemical damage.
8. **Poison**: Toxic biological damage.
9. **Magic**: Non-elemental spell/arcane damage.
10. **True**: Pure damage that bypasses all standard resistances.

### Resistance Multipliers

Damage calculations are multiplied by the target's resistance profile for that type:

| Multiplier | State      | Effect on Incoming Damage          |
| ---------- | ---------- | ---------------------------------- |
| `2.0`      | Vulnerable | Double damage                      |
| `1.0`      | Normal     | Standard damage                    |
| `0.5`      | Resistant  | Halved damage (rounded)            |
| `0.0`      | Immune     | Zero damage                        |
| `-1.0`     | Absorbed   | Reverses damage to heal the target |

---

## Weapon Styles

How a character chooses to wield their weapons changes their hit and damage output:

### 1. Two-Handed Style

- Wielding a single weapon designated as two-handed.
- Grants **1.5× Strength Modifier** bonus to damage calculations.
- Prevents equipping a shield or off-hand item. Wielding automatically unequips any shield or off-hand item.
- Exposes a 1.2x weapon speed modifier.

### 2. Dual-Wield Style

- Wielding a weapon in both the primary hand and the off-hand (shield slot).
- **Hit Penalties**:
  - Primary hand attack has a `-2` penalty to hit (halved to `-1` with Ambidexterity).
  - Off-hand attack has a `-4` penalty to hit (halved to `-2` with Ambidexterity).
- **Damage Modifiers**:
  - Primary hand weapon uses full Strength Modifier.
  - Off-hand weapon gets only **0.5× Strength Modifier** bonus.
- Wielding hit penalties are mitigated by learning the **Ambidexterity** skill.

---

## Corpse & Looting

When a character or NPC dies, their body is represented as a transient `Corpse` entity in the room.

### 1. Corpse Spawn & Decay

- The corpse is populated with the victim's inventory items and equipped items.
- Corpses are transient and are not saved to the SQLite database.
- **Decay Timers**:
  - **Player Corpses**: Decay after **30 minutes** (1800 seconds).
  - **NPC Corpses**: Decay after **5 minutes** (300 seconds).
- **Expiration**: When a corpse's decay timer expires, a pulse sweep transfers all remaining items (inventory + equipment) from the corpse onto the room floor and despawns the corpse entity.

### 2. Loot Rules (Planned)

`LootRule` variants exist on the `Corpse` component but are **not enforced by the loot command** — any player can loot any corpse. Future enforcement:

- **Public**: Anyone can loot the corpse. (Default for NPC corpses).
- **GroupOnly**: Only the owner or members of the owner's party/group can loot. (Default for player corpses).
- **OwnerOnly**: Only the owner of the character can loot.
- **Faction**: Only players matching a specific faction standing can loot.

### 3. Looting Commands

In-game characters interact with corpses using standard command syntax:

- `loot <corpse>`: Transfers all **inventory** items from the corpse into the player's inventory. Equipped items are not taken by `loot`.
- `get <item> <corpse>`: Transfers a specific item from the corpse's inventory into the player's inventory (not yet wired — use `loot`).
- `get all <corpse>`: Transfers all items from the corpse's inventory into the player's inventory.

---

## Death & Ghost System

When a player's health drops to **−10 or below** (or they choose to die while unconscious via `die`), they enter **ghost mode** (`PlayerState::Dead`). NPCs die immediately when health reaches 0 or below.

### Death Thresholds

| Condition          | Entity Type          | Result                                                      |
| ------------------ | -------------------- | ----------------------------------------------------------- |
| HP ≤ 0             | NPC                  | `handle_death()` — corpse spawned, NPC despawned            |
| HP ≤ 0 (and > −10) | Player               | **Unconscious** — bleed-out begins (lose 1 HP per big tick) |
| HP ≤ −10           | Player               | **Dead** (ghost) — `handle_death()` called                  |
| `die` command      | Player (unconscious) | Voluntary death — same as reaching −10                      |

### Death Flow (Players)

1. `handle_death(world, entity)` is called (from combat, bleed-out, or `die` command)
2. `spawn_corpse()` creates a corpse entity in the room containing the player's **inventory** and **equipment**
3. `handle_combatant_down()` reassigns attackers to new targets
4. Player's `Inventory.0` and `Equipment.slots` are cleared
5. **XP penalty** applied: lose **10% of current XP**, capped to prevent de-leveling more than 5 levels (floor = level 1)
6. Health reset to **1** (not full)
7. `PlayerState::Dead` inserted (ghost mode)
8. `LastMessenger` component removed
9. Player teleported to their **`RecallRoom`** (or stays in current room if no recall set)
10. `Dirty` marker added for persistence
11. Death broadcast sent to the room: _"<name> is dead! R.I.P."_

### Death Flow (NPCs)

1. `handle_death()` called immediately at HP ≤ 0
2. Corpse spawned with inventory and equipment
3. Attacker awarded XP: `victim_level² × 50`
4. NPC entity despawned

### Bleed-Out (Unconscious State)

When a player's HP drops to 0 or below but above −10, they enter `PlayerState::Resting(RestState::Unconscious)`:

- Lose **1 HP per big tick** (30–90s randomized)
- Most commands are blocked; only `die` is whitelisted
- `die` triggers `handle_death()` — submits to death and becomes a ghost
- Without intervention, bleed-out reaches −10 on the next big tick

### Ghost State Restrictions

While in ghost form (`PlayerState::Dead`):

| Allowed                                               | Blocked                                                      |
| ----------------------------------------------------- | ------------------------------------------------------------ |
| `look`, `say` (ghost-text), `tell`, `reply`           | `kill`, `flee`, `get`, `drop`, `wear`, `wield`, `remove`     |
| `die` (no-op while already dead), `reclaim`, `revive` | All combat, equipment, and inventory commands                |
| Movement (rooms)                                      | Rest state changes (`sit`, `rest`, `sleep`, `wake`, `stand`) |
| `score`, `commands`, `help`, `quit`                   |                                                              |

Ghost speech renders with alternating cyan/bright-blue characters via `format_ghost_text()`. Ghosts appear as _"You feel a cold shiver run down your spine."_ to normal players. Players with `detect_invisible` or `detect_undead` active effects see the ghost's name and movement: _"The ghost of <name> floats north."_

### Reclaiming Your Body

**`reclaim`** — If the player's corpse is in the same room, reclaim restores everything:

1. Equipment from the corpse is transferred back to `Equipment.slots`
2. Inventory from the corpse is transferred back to `Inventory.0`
3. Health restored to max
4. `PlayerState::Resting(RestState::Standing)` inserted
5. Corpse entity despawned
6. `Dirty` marker added
7. Room broadcast: _"<name> returns to life!"_

**`revive`** — Two paths:

1. **Corpse in same room**: Delegates to `reclaim` (same behavior)
2. **No corpse in room, in an `allow_revive` room**: Revives with health restored to max and state set to alive, but **items remain in the corpse** (must be retrieved from the corpse or the floor after it decays)

A room allows revive-from-ghost when its template has `allow_revive = true` (a boolean field on `RoomTemplate`, default `false`).

**`toggle resurrect`** / **`toggle res`** — Sets `Player.no_resurrect` flag. Stored on the player record but **not yet enforced** by reclaim/revive logic.

### XP Penalty

Applied on death in `handle_death()`:

```
loss = current_xp × 0.10
```

Capped to prevent de-leveling more than 5 levels below current level. Uses the cubic XP curve (`level³ × 100`) to compute the floor. The `Dirty` marker is added so the penalty persists to the database.

---

## Rest States

Players can adopt different rest states that gate activity and modify regen rates.

### Rest State Machine

```
Standing ↔ Sitting ↔ Resting ↔ Sleeping
```

| State        | Movement | Combat | Regen Multiplier |
| ------------ | -------- | ------ | ---------------- |
| Standing     | Yes      | Yes    | 1.0×             |
| Sitting      | No       | No     | 1.5×             |
| Resting      | No       | No     | 2.0×             |
| Sleeping     | No       | No     | 3.0×             |
| Unconscious  | No       | No     | 0.0× (no regen)  |
| Dead (ghost) | Yes      | No     | 0.0×             |

### Commands

| Command | Action             | Transition                 |
| ------- | ------------------ | -------------------------- |
| `sit`   | Sit down           | Standing/Resting → Sitting |
| `rest`  | Lean back and rest | Any → Resting              |
| `sleep` | Go to sleep        | Any → Sleeping             |
| `wake`  | Wake from sleep    | Sleeping → Resting         |
| `stand` | Stand up           | Any → Standing             |

All rest commands broadcast to the room ("Player sits down.", "Player rests.", etc.). Combat, stun, and casting states block rest transitions with appropriate messages.

### Stun & Casting

- `PlayerState::Stunned { remaining_ms }` — blocks all actions until timer expires
- `PlayerState::Casting { .. }` — blocks movement and combat while casting

---

## Skills & Practice System

Skills represent trained abilities that gate equipment usage and provide combat modifiers.

### Skill Levels (Ranks)

Each skill has a numerical rank. Maximum rank at a given level:

```
max_rank = level * 5 + 5
```

| Level | Max Skill Rank |
| ----- | -------------- |
| 1     | 10             |
| 10    | 55             |
| 50    | 255            |

### Practice Points

Players receive practice points through leveling (amount determined by class template). Points are spent to increase skill ranks.

**`practice`** — List known skills and current ranks. Shows available practice points.

**`practice <skill>`** — Increase a skill rank by 1, costing 1 practice point. Requires a trainer NPC in the same room that can train the skill's category (combat, weapon, magic, etc.).

### Attribute Training

**`train`** — Lists attributes and training costs. Each attribute costs practice points to increase: 3 for the class's prime attribute, 5 for others. Maximum attribute value is defined by `Attributes::MAX`.

**`train <attribute>`** — Increase the named attribute by 1 point. Requires a trainer with `"attributes"` in their `trainer_types`.

### Skill Gates on Items

Items can have `requires_skill = { id = "two_handed", level = 3 }`. The system checks the wielder's `LearnedSkills` rank against the requirement each time the item is equipped (on `wear`/`wield`). If the requirement is not met, equipping is rejected.

---

## Stances

Combat stances modify hit chance, damage, armor, and skill effectiveness. Players set their stance with the **`stance`** command.

### Built-in Stances

**`stance`** (no args) — Shows current stance and available options.

**`stance <name>`** — Adopts the named stance.

| Stance       | Effect                                                   |
| ------------ | -------------------------------------------------------- |
| `normal`     | No modifiers (default).                                  |
| `defensive`  | +hit defense, −damage dealt.                             |
| `aggressive` | +hit chance, −defense.                                   |
| `berserk`    | +3 damage, −2 armor, +stun resist, −5% hit, +25% damage. |

The stance is stored in the `ActiveStance` component. The combat system reads the stance to apply modifiers during attack rolls and damage calculations. Custom stances can be defined in `content/stances/*.toml`.

---

## Doors

Exits can be configured as doors with open/close/lock/unlock states.

### Door State

Exit templates support:

- `is_door = true` — marks the exit as a door
- `is_closed = true` — door starts closed (blocks movement)
- `is_locked = true` — door starts locked
- `key_id = "iron_key"` — item template ID of the required key

### Commands

| Command              | Action                               |
| -------------------- | ------------------------------------ |
| `open <direction>`   | Opens a closed, unlocked door        |
| `close <direction>`  | Closes an open door                  |
| `lock <direction>`   | Locks a closed door (requires key)   |
| `unlock <direction>` | Unlocks a locked door (requires key) |

Door operations broadcast to both sides of the exit ("Player opens the door north." in current room, "The door opens." in destination room). The key item must be in the player's inventory.

---

## Regen & Game Loop Ticks

The game loop runs multiple tick intervals rather than a fixed tick rate.

### Tick Intervals

| Tick         | Interval            | Description                                 |
| ------------ | ------------------- | ------------------------------------------- |
| Player State | 250ms               | Processes decay of player stun & cast times |
| Skill Decay  | 1s                  | Decrements cooldowns & buff durations       |
| Combat Pulse | 2s                  | Runs combat rounds, AI ticks, & stances     |
| Maintenance  | 5s                  | Flushes dirty stats, saves positions        |
| Set Bonus    | 10s                 | Re-evaluates equipment set bonuses          |
| Big Tick     | 30–90s (randomized) | HP/MP/SP regen + prompt broadcast           |

### Regen (Big Tick)

The big tick fires every 30–90 seconds (uniform random). Each entity with `Health`, `Mana`, or `Stamina` components regens a portion of their max value:

```
regen_amount = max_value / 10 * rest_multiplier
```

| Rest State       | Multiplier |
| ---------------- | ---------- |
| Standing         | 1.0×       |
| Sitting          | 1.5×       |
| Resting          | 2.0×       |
| Sleeping         | 3.0×       |
| Unconscious/Dead | 0.0×       |

After regen, `send_player_prompt` is called for all connected players, broadcasting their updated prompt line.

---

## Prompt System

The prompt is the status line displayed after command output. It's configurable per-player.

### Prompt Variables

The following variables can be used in your prompt template and will be replaced by the system at runtime:

| Variable | Description                                                    |
| -------- | -------------------------------------------------------------- |
| `%h`     | Current HP                                                     |
| `%H`     | Max HP                                                         |
| `%m`     | Current Mana                                                   |
| `%M`     | Max Mana                                                       |
| `%v`     | Current Stamina                                                |
| `%V`     | Max Stamina                                                    |
| `%l`     | Current Level                                                  |
| `%x`     | Current Experience (XP)                                        |
| `%X`     | Experience (XP) remaining to next level                        |
| `%n`     | Character Name                                                 |
| `%g`     | Wallet Gold (total copper)                                     |
| `%a`     | Character Alignment                                            |
| `%r`     | Current Room Name                                              |
| `%e`     | Available Exits (e.g. `n s e w`)                               |
| `%s`     | Strength Attribute                                             |
| `%d`     | Dexterity Attribute                                            |
| `%i`     | Intelligence Attribute                                         |
| `%w`     | Wisdom Attribute                                               |
| `%o`     | Constitution Attribute                                         |
| `%u`     | Charisma Attribute                                             |
| `%R`     | Rest State (e.g. `Standing`, `Sitting`, `Resting`, `Sleeping`) |
| `%C`     | Combat State (e.g. `In Combat`, `Not In Combat`)               |
| `%c`     | Carriage return / newline (`\r\n`)                             |
| `%%`     | A literal `%` character                                        |

### Commands

**`prompt`** — Shows current prompt template. `prompt <template>` — Sets a custom prompt. `prompt reset` — Reverts to server default.

Default prompt is configurable via `server.toml` (`default_prompt` field).

**`width <columns>`** — Sets screen width for text wrapping. `0` = unlimited. Persisted to the player's DB record.

**`colors`** — Toggles ANSI color output (persisted to player DB).

**`toggle`** — Toggles player settings. Currently only `toggle resurrect` is wired (prevents resurrection in `allow_revive` rooms).

---

## Communication Channels

| Command                      | Scope  | Description                                             |
| ---------------------------- | ------ | ------------------------------------------------------- |
| `say <message>`              | Room   | Speaks aloud in the current room                        |
| `tell <player> <message>`    | Global | Sends a private message to any online player            |
| `reply <message>`            | Global | Replies to the last player who messaged you (`r` alias) |
| `shout <message>`            | Area   | Shouts to all players in the same area                  |
| `whisper <player> <message>` | Room   | Whispers to a player in the same room                   |

Ghost speech renders in a distinctive alternating color pattern. `tell` and `reply` track the last messenger via the `LastMessenger` component.

---

## Inventory & Equipment Commands

| Command                   | Description                                 |
| ------------------------- | ------------------------------------------- |
| `inventory` / `inv` / `i` | List items you are carrying                 |
| `equipment` / `eq`        | Show what you are wearing and wielding      |
| `get <item>`              | Pick up an item from the floor              |
| `drop <item>`             | Drop a carried item to the floor            |
| `put <item> <container>`  | Place item in a container (not yet wired)   |
| `give <item> <player>`    | Give item to another player (not yet wired) |
| `loot <corpse>`           | Take all items from a corpse                |
| `wear <item>`             | Wear a piece of armor                       |
| `wield <item>`            | Wield a weapon                              |
| `remove <slot>`           | Unequip an item (removes to inventory)      |
| `examine` / `exa`         | Inspect an item's detailed properties       |

### Equipment Slots

Available equipment slots: `head`, `neck`, `torso`, `arms`, `hands`, `finger`, `legs`, `feet`, `weapon`, `shield`, `back`, `waist`.

Items with `requires_skill` are checked on `wear`/`wield`. If the player lacks the required skill rank, equipping is rejected with "You lack the skill to use that."

---

## Time & Weather

### Time System

- **Clock Model**: 1 real-world minute equals 12 in-game minutes (1 in-game hour = 5 real minutes; 1 in-game day = 2 real hours).
- **Time Periods**: 24-hour cycle split into `Night`, `Midnight`, `Dawn`, `Morning`, `Noon`, `Afternoon`, `Dusk`, and `Evening`.
- **Seasons & Calendar**: 4 seasons (`Spring`, `Summer`, `Autumn`, `Winter`) of 30 days each (120-day in-game year).
- **Prompt Integration**: `%t` prompt variable renders current active time period (e.g. `"Dawn"`).

### Weather System

- **Composition**: Per-room `WeatherState` component tracking active base condition (e.g. `"Rain"`) and modifier condition (e.g. `"Fog"`).
- **Resolution Chain**: Evaluated per room via a 5-tier fallback chain: Room `additional_weather` $\rightarrow$ Area `weather_matrix` $\rightarrow$ Zone matrix $\rightarrow$ Season availability $\rightarrow$ Base fallback (`"clear"`).
- **Gameplay Effects**: Active weather condition effects dynamically apply to gameplay:
  - **Damage Modifiers**: Dynamic `damage_<type>` modifiers (e.g., `damage_fire`, `damage_lightning`, `damage_cold`) modify damage rolls of matching damage types.
  - **Ranged Combat**: `ranged_accuracy` (flat modifier) and `ranged_accuracy_pct` (percentage modifier) modify ranged hit rolls; `ranged_attack` modifies ranged damage rolls.
  - **Attributes**: `dexterity` modifier applies to character DEX attribute evaluations.
- **Flavor & Looking**: Active weather flavor text automatically appends to `look` room descriptions and room transition outputs.
- **Prompt Integration**: `%w` prompt variable renders active room weather conditions (e.g. `"Rain, Fog"`).

| Command   | Description                                                                                                            |
| --------- | ---------------------------------------------------------------------------------------------------------------------- |
| `time`    | Shows current in-game time period, day, season, and year (e.g. "It is Dawn on the 14th day of Spring, Year 1.")        |
| `weather` | Shows current weather conditions in your area (e.g. "Rain falls from grey clouds. Strong winds gust across the land.") |

For full architectural details, persistence schemas, and mathematical resolution formulas, see [ARCHITECTURE.md](../ARCHITECTURE.md#time-system) and [ARCHITECTURE.md](../ARCHITECTURE.md#weather-system).
